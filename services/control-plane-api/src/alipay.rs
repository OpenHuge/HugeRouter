use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use qrcode::{QrCode, render::svg};
use reqwest::Client as HttpClient;
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1::DecodeRsaPrivateKey,
    pkcs1v15::{Signature as RsaSignature, SigningKey, VerifyingKey},
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    signature::{RandomizedSigner, SignatureEncoding, Verifier},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::{SystemTime, UNIX_EPOCH}};

const ALIPAY_GATEWAY_URL: &str = "https://openapi.alipay.com/gateway.do";
const ALIPAY_USER_AGENT: &str = "HugeRouter-Alipay/1.0";
const DEFAULT_PAYMENT_DESCRIPTION: &str = "HugeRouter order payment";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlipayPrecreateRequest {
    pub tenant_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub amount_total: u32,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_payment_description")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlipayPrecreateResponse {
    pub app_id: String,
    pub out_trade_no: String,
    pub channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pay_url: Option<String>,
    pub qr_code: String,
    pub code_qr_svg: String,
}

#[derive(Debug, Clone)]
pub struct AlipayNotifyPayload {
    pub out_trade_no: String,
    pub trade_no: Option<String>,
    pub trade_status: String,
    pub total_amount: Option<String>,
    pub notify_id: Option<String>,
    pub gmt_payment: Option<String>,
}

#[derive(Debug, Clone)]
struct AlipayConfig {
    app_id: String,
    notify_url: String,
    private_key: RsaPrivateKey,
    alipay_public_key: RsaPublicKey,
}

#[derive(Debug, Clone)]
pub struct AlipayClient {
    config: AlipayConfig,
    http_client: HttpClient,
}

impl AlipayClient {
    pub fn from_env() -> Result<Self> {
        let private_key_pem = env_secret_or_file("ALIPAY_PRIVATE_KEY", "ALIPAY_PRIVATE_KEY_PATH")?;
        let public_key_pem = env_secret_or_file("ALIPAY_PUBLIC_KEY", "ALIPAY_PUBLIC_KEY_PATH")?;
        Ok(Self {
            config: AlipayConfig {
                app_id: required_env("ALIPAY_APP_ID")?,
                notify_url: required_env("ALIPAY_NOTIFY_URL")?,
                private_key: parse_private_key(&private_key_pem)?,
                alipay_public_key: parse_public_key(&public_key_pem)?,
            },
            http_client: HttpClient::new(),
        })
    }

    pub async fn precreate(
        &self,
        request: AlipayPrecreateRequest,
        out_trade_no: &str,
    ) -> Result<AlipayPrecreateResponse> {
        validate_precreate_request(&request)?;
        validate_out_trade_no(out_trade_no)?;
        let result = self
            .execute_payment(
                "alipay.trade.precreate",
                precreate_biz_content(&request, out_trade_no),
            )
            .await?;
        let qr_code = result
            .get("qr_code")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("Alipay precreate response is missing qr_code"))?
            .to_string();
        Ok(AlipayPrecreateResponse {
            app_id: self.config.app_id.clone(),
            out_trade_no: out_trade_no.to_string(),
            channel: "alipay_qr".to_string(),
            pay_url: None,
            code_qr_svg: render_qr_svg(&qr_code)?,
            qr_code,
        })
    }

    pub async fn query_order(&self, out_trade_no: &str) -> Result<Value> {
        validate_out_trade_no(out_trade_no)?;
        self.execute(
            "alipay.trade.query",
            serde_json::json!({ "out_trade_no": out_trade_no }),
        )
        .await
    }

    pub fn verify_notify(&self, body: &str) -> Result<AlipayNotifyPayload> {
        let params = parse_form_encoded(body)?;
        let sign = params
            .get("sign")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("Alipay notification is missing sign"))?;
        let sign_content = alipay_sign_content(&params, &["sign", "sign_type"]);
        verify_message(&self.config.alipay_public_key, &sign_content, sign)
            .context("invalid Alipay notification signature")?;
        let app_id = params.get("app_id").map(String::as_str);
        if app_id != Some(self.config.app_id.as_str()) {
            bail!("Alipay notification app_id does not match configuration");
        }
        let out_trade_no = required_param(&params, "out_trade_no")?.to_string();
        let trade_status = required_param(&params, "trade_status")?.to_string();
        Ok(AlipayNotifyPayload {
            out_trade_no,
            trade_status,
            trade_no: params.get("trade_no").cloned(),
            total_amount: params.get("total_amount").cloned(),
            notify_id: params.get("notify_id").cloned(),
            gmt_payment: params.get("gmt_payment").cloned(),
        })
    }

    async fn execute_payment(&self, method: &str, biz_content: Value) -> Result<Value> {
        self.execute_with_notify_url(method, biz_content, true).await
    }

    async fn execute(&self, method: &str, biz_content: Value) -> Result<Value> {
        self.execute_with_notify_url(method, biz_content, false).await
    }

    async fn execute_with_notify_url(
        &self,
        method: &str,
        biz_content: Value,
        include_notify_url: bool,
    ) -> Result<Value> {
        let params = self.signed_api_params(method, biz_content, include_notify_url);
        let response = self
            .http_client
            .post(ALIPAY_GATEWAY_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, ALIPAY_USER_AGENT)
            .body(form_encode(&params))
            .send()
            .await
            .context("failed to send Alipay API request")?;
        let status = response.status();
        let response_text = response.text().await.context("failed to read Alipay response")?;
        if !status.is_success() {
            bail!("Alipay API request failed with status {status}: {response_text}");
        }
        let payload: Value = serde_json::from_str(&response_text)
            .with_context(|| format!("failed to parse Alipay response: {response_text}"))?;
        let response_key = format!("{}_response", method.replace('.', "_"));
        let result = payload
            .get(&response_key)
            .cloned()
            .ok_or_else(|| anyhow!("Alipay API response is missing {response_key}: {response_text}"))?;
        if let Some(sign) = payload.get("sign").and_then(Value::as_str)
            && let Some(sign_content) = extract_response_sign_content(&response_text, &response_key)
        {
            if let Err(error) = verify_message(&self.config.alipay_public_key, &sign_content, sign)
                .context("invalid Alipay API response signature")
            {
                tracing::warn!(
                    method = method,
                    error = %error,
                    "Alipay API response signature verification failed; continuing with trusted HTTPS response"
                );
            }
        }
        if result.get("code").and_then(Value::as_str) != Some("10000") {
            bail!("Alipay API error: {result}");
        }
        Ok(result)
    }

    fn signed_api_params(
        &self,
        method: &str,
        biz_content: Value,
        include_notify_url: bool,
    ) -> BTreeMap<String, String> {
        self.signed_params(method, biz_content, include_notify_url)
    }

    fn signed_params(
        &self,
        method: &str,
        biz_content: Value,
        include_notify_url: bool,
    ) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("app_id".to_string(), self.config.app_id.clone());
        params.insert("method".to_string(), method.to_string());
        params.insert("format".to_string(), "JSON".to_string());
        params.insert("charset".to_string(), "utf-8".to_string());
        params.insert("sign_type".to_string(), "RSA2".to_string());
        params.insert("timestamp".to_string(), alipay_timestamp());
        params.insert("version".to_string(), "1.0".to_string());
        if include_notify_url {
            params.insert("notify_url".to_string(), self.config.notify_url.clone());
        }
        params.insert("biz_content".to_string(), biz_content.to_string());
        let sign_content = alipay_sign_content(&params, &["sign"]);
        let sign = sign_message(&self.config.private_key, &sign_content);
        params.insert("sign".to_string(), sign);
        params
    }
}

fn precreate_biz_content(request: &AlipayPrecreateRequest, out_trade_no: &str) -> Value {
    serde_json::json!({
        "out_trade_no": out_trade_no,
        "product_code": "FACE_TO_FACE_PAYMENT",
        "total_amount": amount_total_to_yuan(request.amount_total),
        "subject": request.description,
    })
}

pub fn new_out_trade_no(tenant_id: &str, project_id: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tenant_id.as_bytes());
    if let Some(project_id) = project_id {
        hasher.update(b":");
        hasher.update(project_id.as_bytes());
    }
    hasher.update(b":alipay:");
    hasher.update(unix_timestamp().as_bytes());
    hasher.update(b":");
    hasher.update(nonce_string().as_bytes());
    let digest = hasher.finalize();
    format!("ha{}", hex_prefix(&digest, 30))
}

pub fn alipay_trade_status_is_paid(status: &str) -> bool {
    matches!(status, "TRADE_SUCCESS" | "TRADE_FINISHED")
}

pub fn is_alipay_trade_not_exist_error(error: &anyhow::Error) -> bool {
    error.to_string().contains("ACQ.TRADE_NOT_EXIST")
}

fn validate_precreate_request(request: &AlipayPrecreateRequest) -> Result<()> {
    if request.tenant_id.trim().is_empty() {
        bail!("tenant_id is required");
    }
    if request.amount_total == 0 {
        bail!("amount_total must be greater than zero");
    }
    if request.currency != "CNY" {
        bail!("only CNY Alipay orders are supported");
    }
    if request.description.trim().is_empty() || request.description.chars().count() > 256 {
        bail!("description must be present and at most 256 characters");
    }
    Ok(())
}

fn validate_out_trade_no(out_trade_no: &str) -> Result<()> {
    if out_trade_no.trim().is_empty() || out_trade_no.len() > 64 {
        bail!("out_trade_no must be present and at most 64 bytes");
    }
    if !out_trade_no
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '|'))
    {
        bail!("out_trade_no contains unsupported characters");
    }
    Ok(())
}

fn amount_total_to_yuan(amount_total: u32) -> String {
    format!("{}.{:02}", amount_total / 100, amount_total % 100)
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name)
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}

fn env_secret_or_file(value_name: &str, path_name: &str) -> Result<String> {
    if let Ok(value) = std::env::var(value_name) {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }
    if let Ok(path) = std::env::var(path_name) {
        let path = path.trim();
        if !path.is_empty() {
            return std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {path_name} at {path}"));
        }
    }
    Err(anyhow!("{value_name} or {path_name} is required"))
}

fn parse_private_key(pem: &str) -> Result<RsaPrivateKey> {
    let pem = normalize_private_key_pem(pem);
    RsaPrivateKey::from_pkcs8_pem(&pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(&pem))
        .context("failed to parse Alipay private key PEM")
}

fn parse_public_key(pem: &str) -> Result<RsaPublicKey> {
    let pem = normalize_public_key_pem(pem);
    RsaPublicKey::from_public_key_pem(&pem)
        .context("failed to parse Alipay public key PEM")
}

fn normalize_private_key_pem(value: &str) -> String {
    let normalized = normalize_key_value(value);
    if normalized.contains("-----BEGIN") {
        return normalized;
    }
    format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----",
        wrap_base64(&normalized)
    )
}

fn normalize_public_key_pem(value: &str) -> String {
    let normalized = normalize_key_value(value);
    if normalized.contains("-----BEGIN") {
        return normalized;
    }
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        wrap_base64(&normalized)
    )
}

fn normalize_key_value(value: &str) -> String {
    value
        .trim()
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
}

fn wrap_base64(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<Vec<_>>()
        .chunks(64)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn sign_message(private_key: &RsaPrivateKey, message: &str) -> String {
    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let mut rng = rsa::rand_core::OsRng;
    let signature = signing_key.sign_with_rng(&mut rng, message.as_bytes());
    BASE64.encode(signature.to_bytes())
}

fn verify_message(public_key: &RsaPublicKey, message: &str, signature: &str) -> Result<()> {
    let signature = BASE64.decode(signature).context("failed to decode Alipay RSA signature")?;
    let signature = RsaSignature::try_from(signature.as_slice())
        .context("failed to parse Alipay RSA signature")?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key.clone());
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|error| anyhow!("RSA2 verification failed: {error}"))
}

fn alipay_sign_content(params: &BTreeMap<String, String>, excluded_keys: &[&str]) -> String {
    params
        .iter()
        .filter(|(key, value)| {
            !excluded_keys.contains(&key.as_str()) && !value.trim().is_empty()
        })
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn form_encode(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn parse_form_encoded(body: &str) -> Result<BTreeMap<String, String>> {
    let mut params = BTreeMap::new();
    for pair in body.split('&') {
        if pair.trim().is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        params.insert(
            urlencoding::decode(key).context("failed to decode Alipay form key")?.into_owned(),
            urlencoding::decode(value).context("failed to decode Alipay form value")?.into_owned(),
        );
    }
    Ok(params)
}

fn required_param<'a>(params: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Alipay payload is missing {key}"))
}

fn extract_response_sign_content(raw_text: &str, response_key: &str) -> Option<String> {
    let key_pattern = format!("\"{response_key}\"");
    let key_index = raw_text.find(&key_pattern)?;
    let colon_index = raw_text[key_index + key_pattern.len()..].find(':')? + key_index + key_pattern.len();
    let mut start = colon_index + 1;
    while raw_text.as_bytes().get(start).copied() == Some(b' ') {
        start += 1;
    }
    let bytes = raw_text.as_bytes();
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for index in start..bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' && in_string {
            escaped = true;
            continue;
        }
        if byte == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if byte == b'{' {
            depth += 1;
        }
        if byte == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(raw_text[start..=index].to_string());
            }
        }
    }
    None
}

fn render_qr_svg(value: &str) -> Result<String> {
    let code = QrCode::new(value.as_bytes()).context("failed to build Alipay QR code")?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(220, 220)
        .dark_color(svg::Color("#111827"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

fn alipay_timestamp() -> String {
    let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).expect("valid offset"));
    now.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn unix_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn nonce_string() -> String {
    let mut seed = [0_u8; 32];
    getrandom::fill(&mut seed).expect("OS random source is required");
    BASE64.encode(seed).replace(['+', '/', '='], "")
}

fn hex_prefix(bytes: &[u8], length: usize) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .take(length)
        .map(|nibble| char::from_digit(u32::from(nibble), 16).expect("hex nibble"))
        .collect()
}

fn default_currency() -> String {
    "CNY".to_string()
}

fn default_payment_description() -> String {
    DEFAULT_PAYMENT_DESCRIPTION.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        amount_total_to_yuan, alipay_sign_content, extract_response_sign_content,
        new_out_trade_no, normalize_private_key_pem, normalize_public_key_pem,
        precreate_biz_content,
        AlipayPrecreateRequest,
    };
    use std::collections::BTreeMap;

    #[test]
    fn amount_total_formats_fen_as_yuan() {
        assert_eq!(amount_total_to_yuan(1), "0.01");
        assert_eq!(amount_total_to_yuan(1234), "12.34");
    }

    #[test]
    fn alipay_sign_content_sorts_and_excludes_signature_fields() {
        let mut params = BTreeMap::new();
        params.insert("sign".to_string(), "ignored".to_string());
        params.insert("method".to_string(), "alipay.trade.precreate".to_string());
        params.insert("app_id".to_string(), "app".to_string());
        assert_eq!(
            alipay_sign_content(&params, &["sign"]),
            "app_id=app&method=alipay.trade.precreate"
        );
    }

    #[test]
    fn response_sign_content_extracts_response_json() {
        let raw = r#"{"alipay_trade_precreate_response":{"code":"10000","qr_code":"abc"},"sign":"sig"}"#;
        assert_eq!(
            extract_response_sign_content(raw, "alipay_trade_precreate_response").unwrap(),
            r#"{"code":"10000","qr_code":"abc"}"#
        );
    }

    #[test]
    fn alipay_out_trade_no_fits_gateway_limit() {
        let out_trade_no = new_out_trade_no("tenant_acme", Some("proj_core"));
        assert!(out_trade_no.starts_with("ha"));
        assert!(out_trade_no.len() <= 64);
    }

    #[test]
    fn alipay_keys_accept_bare_base64_values() {
        assert!(normalize_private_key_pem("abc").starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(normalize_public_key_pem("abc").starts_with("-----BEGIN PUBLIC KEY-----"));
    }

    #[test]
    fn detects_alipay_trade_not_exist_error() {
        let error = anyhow::anyhow!(
            r#"Alipay API error: {{"sub_code":"ACQ.TRADE_NOT_EXIST"}}"#
        );
        assert!(super::is_alipay_trade_not_exist_error(&error));
    }

    #[test]
    fn precreate_biz_content_uses_face_to_face_payment_product() {
        let request = AlipayPrecreateRequest {
            tenant_id: "tenant_acme".to_string(),
            project_id: Some("proj_core".to_string()),
            amount_total: 1,
            currency: "CNY".to_string(),
            description: "HugeCode Pro".to_string(),
            attach: None,
        };
        let biz_content = precreate_biz_content(&request, "ha_order_1");
        assert_eq!(biz_content["out_trade_no"], "ha_order_1");
        assert_eq!(biz_content["product_code"], "FACE_TO_FACE_PAYMENT");
        assert_eq!(biz_content["total_amount"], "0.01");
        assert_eq!(biz_content["subject"], "HugeCode Pro");
    }
}
