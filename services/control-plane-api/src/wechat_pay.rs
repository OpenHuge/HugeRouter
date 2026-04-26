use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Payload},
};
use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use qrcode::{QrCode, render::svg};
use reqwest::{Client as HttpClient, Method};
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    pkcs1v15::{Signature as RsaSignature, SigningKey, VerifyingKey},
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    signature::{SignatureEncoding, Signer, Verifier},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

const WECHAT_PAY_API_BASE_URL: &str = "https://api.mch.weixin.qq.com";
const DEFAULT_PAYMENT_DESCRIPTION: &str = "HugeRouter balance recharge";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WechatPayChannel {
    Native,
    Jsapi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPayPrepayRequest {
    pub tenant_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub amount_total: u32,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_payment_description")]
    pub description: String,
    pub channel: WechatPayChannel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payer_openid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPayPrepayResponse {
    pub app_id: String,
    pub mchid: String,
    pub out_trade_no: String,
    pub channel: WechatPayChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_qr_svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepay_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsapi_params: Option<WechatPayJsapiParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPayOrderQueryResponse {
    pub transaction: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatPayJsapiParams {
    pub app_id: String,
    pub time_stamp: String,
    pub nonce_str: String,
    pub package: String,
    pub sign_type: String,
    pub pay_sign: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPayNotificationResponse {
    pub id: String,
    pub create_time: String,
    pub event_type: String,
    pub resource_type: String,
    pub summary: String,
    pub transaction: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WechatPayNotification {
    pub id: String,
    pub create_time: String,
    pub event_type: String,
    pub resource_type: String,
    pub summary: String,
    pub resource: WechatPayEncryptedResource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WechatPayEncryptedResource {
    pub algorithm: String,
    pub ciphertext: String,
    pub associated_data: String,
    pub nonce: String,
}

#[derive(Debug, Clone)]
pub struct WechatPayHeaders {
    pub timestamp: String,
    pub nonce: String,
    pub signature: String,
    pub serial: String,
}

#[derive(Debug, Clone)]
struct WechatPayConfig {
    app_id: String,
    mchid: String,
    merchant_serial_no: String,
    notify_url: String,
    private_key: RsaPrivateKey,
    api_v3_key: String,
    platform_public_key: RsaPublicKey,
}

#[derive(Debug, Clone)]
pub struct WechatPayClient {
    config: WechatPayConfig,
    http_client: HttpClient,
}

impl WechatPayClient {
    #[must_use]
    pub fn app_id(&self) -> &str {
        &self.config.app_id
    }

    #[must_use]
    pub fn mchid(&self) -> &str {
        &self.config.mchid
    }

    /// # Errors
    ///
    /// Returns an error when any required `WeChat Pay` environment variable is
    /// missing or the configured private/public keys cannot be parsed.
    pub fn from_env() -> Result<Self> {
        let private_key_pem = env_secret_or_file(
            "WECHAT_PAY_MERCHANT_PRIVATE_KEY",
            "WECHAT_PAY_MERCHANT_PRIVATE_KEY_PATH",
        )?;
        let platform_public_key = env_secret_or_file(
            "WECHAT_PAY_PLATFORM_PUBLIC_KEY",
            "WECHAT_PAY_PLATFORM_PUBLIC_KEY_PATH",
        )
        .and_then(|pem| parse_public_key(&pem))?;

        Ok(Self {
            config: WechatPayConfig {
                app_id: required_env("WECHAT_PAY_APP_ID")?,
                mchid: required_env("WECHAT_PAY_MCH_ID")?,
                merchant_serial_no: required_env("WECHAT_PAY_MERCHANT_SERIAL_NO")?,
                notify_url: required_env("WECHAT_PAY_NOTIFY_URL")?,
                private_key: parse_private_key(&private_key_pem)?,
                api_v3_key: required_env("WECHAT_PAY_API_V3_KEY")?,
                platform_public_key,
            },
            http_client: HttpClient::new(),
        })
    }

    /// # Errors
    ///
    /// Returns an error when `WeChat` rejects the prepay request, response
    /// verification fails, or the response payload is invalid.
    pub async fn create_prepay(
        &self,
        request: WechatPayPrepayRequest,
        out_trade_no: &str,
    ) -> Result<WechatPayPrepayResponse> {
        validate_prepay_request(&request)?;

        let path = match request.channel {
            WechatPayChannel::Native => "/v3/pay/transactions/native",
            WechatPayChannel::Jsapi => "/v3/pay/transactions/jsapi",
        };
        let url = format!("{WECHAT_PAY_API_BASE_URL}{path}");
        let body = build_prepay_body(&self.config, &request, out_trade_no);
        let body_text = serde_json::to_string(&body)?;
        let authorization = self.authorization_header(Method::POST.as_str(), path, &body_text);
        let response = self
            .http_client
            .post(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", authorization)
            .body(body_text)
            .send()
            .await
            .context("failed to send WeChat Pay prepay request")?;

        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response
            .text()
            .await
            .context("failed to read WeChat Pay prepay response")?;

        if !status.is_success() {
            bail!("WeChat Pay prepay failed with status {status}: {response_text}");
        }

        self.verify_response(Method::POST.as_str(), path, &headers, &response_text)?;

        match request.channel {
            WechatPayChannel::Native => {
                let response: NativePrepayResponse = serde_json::from_str(&response_text)
                    .context("failed to decode WeChat Native prepay response")?;
                Ok(WechatPayPrepayResponse {
                    app_id: self.config.app_id.clone(),
                    mchid: self.config.mchid.clone(),
                    out_trade_no: out_trade_no.to_string(),
                    channel: WechatPayChannel::Native,
                    code_qr_svg: Some(render_qr_svg(&response.code_url)?),
                    code_url: Some(response.code_url),
                    prepay_id: None,
                    jsapi_params: None,
                })
            }
            WechatPayChannel::Jsapi => {
                let response: JsapiPrepayResponse = serde_json::from_str(&response_text)
                    .context("failed to decode WeChat JSAPI prepay response")?;
                let jsapi_params = self.jsapi_params(&response.prepay_id);
                Ok(WechatPayPrepayResponse {
                    app_id: self.config.app_id.clone(),
                    mchid: self.config.mchid.clone(),
                    out_trade_no: out_trade_no.to_string(),
                    channel: WechatPayChannel::Jsapi,
                    code_qr_svg: None,
                    code_url: None,
                    prepay_id: Some(response.prepay_id),
                    jsapi_params: Some(jsapi_params),
                })
            }
        }
    }

    /// # Errors
    ///
    /// Returns an error when the query request fails, response verification
    /// fails, or the returned payload is not valid JSON.
    pub async fn query_order(&self, out_trade_no: &str) -> Result<WechatPayOrderQueryResponse> {
        validate_out_trade_no(out_trade_no)?;
        let encoded_trade_no = urlencoding::encode(out_trade_no);
        let path = format!(
            "/v3/pay/transactions/out-trade-no/{encoded_trade_no}?mchid={}",
            self.config.mchid
        );
        let url = format!("{WECHAT_PAY_API_BASE_URL}{path}");
        let authorization = self.authorization_header(Method::GET.as_str(), &path, "");
        let response = self
            .http_client
            .get(url)
            .header("Accept", "application/json")
            .header("Authorization", authorization)
            .send()
            .await
            .context("failed to query WeChat Pay order")?;
        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response
            .text()
            .await
            .context("failed to read WeChat Pay order query response")?;

        if !status.is_success() {
            bail!("WeChat Pay order query failed with status {status}: {response_text}");
        }

        self.verify_response(Method::GET.as_str(), &path, &headers, &response_text)?;
        Ok(WechatPayOrderQueryResponse {
            transaction: serde_json::from_str(&response_text)
                .context("failed to decode WeChat Pay order query response")?,
        })
    }

    /// # Errors
    ///
    /// Returns an error when notification signature verification or AES-GCM
    /// resource decryption fails.
    pub fn decode_notification(
        &self,
        headers: &WechatPayHeaders,
        body: &[u8],
    ) -> Result<WechatPayNotificationResponse> {
        self.verify_notification(headers, body)?;
        let notification: WechatPayNotification =
            serde_json::from_slice(body).context("failed to decode WeChat Pay notification")?;
        let transaction = self.decrypt_resource(&notification.resource)?;

        Ok(WechatPayNotificationResponse {
            id: notification.id,
            create_time: notification.create_time,
            event_type: notification.event_type,
            resource_type: notification.resource_type,
            summary: notification.summary,
            transaction,
        })
    }

    fn authorization_header(&self, method: &str, path: &str, body: &str) -> String {
        let timestamp = unix_timestamp();
        let nonce = nonce_string();
        let message = signature_message(method, path, &timestamp, &nonce, body);
        let signature = sign_message(&self.config.private_key, &message);

        format!(
            "WECHATPAY2-SHA256-RSA2048 mchid=\"{}\",nonce_str=\"{}\",timestamp=\"{}\",serial_no=\"{}\",signature=\"{}\"",
            self.config.mchid, nonce, timestamp, self.config.merchant_serial_no, signature
        )
    }

    fn jsapi_params(&self, prepay_id: &str) -> WechatPayJsapiParams {
        let time_stamp = unix_timestamp();
        let nonce_str = nonce_string();
        let package = format!("prepay_id={prepay_id}");
        let message = format!(
            "{}\n{}\n{}\n{}\n",
            self.config.app_id, time_stamp, nonce_str, package
        );
        let pay_sign = sign_message(&self.config.private_key, &message);

        WechatPayJsapiParams {
            app_id: self.config.app_id.clone(),
            time_stamp,
            nonce_str,
            package,
            sign_type: "RSA".to_string(),
            pay_sign,
        }
    }

    fn verify_response(
        &self,
        method: &str,
        path: &str,
        headers: &reqwest::header::HeaderMap,
        body: &str,
    ) -> Result<()> {
        let timestamp = header_value(headers, "Wechatpay-Timestamp")?;
        let nonce = header_value(headers, "Wechatpay-Nonce")?;
        let signature = header_value(headers, "Wechatpay-Signature")?;
        let _serial = header_value(headers, "Wechatpay-Serial")?;
        let message = signature_message(method, path, timestamp, nonce, body);

        verify_message(&self.config.platform_public_key, &message, signature)
            .context("invalid WeChat Pay response signature")
    }

    fn verify_notification(&self, headers: &WechatPayHeaders, body: &[u8]) -> Result<()> {
        let body_text =
            std::str::from_utf8(body).context("WeChat Pay notification body is not UTF-8")?;
        let message = format!("{}\n{}\n{}\n", headers.timestamp, headers.nonce, body_text);
        verify_message(
            &self.config.platform_public_key,
            &message,
            &headers.signature,
        )
        .with_context(|| {
            format!(
                "invalid WeChat Pay notification signature for platform serial {}",
                headers.serial
            )
        })
    }

    fn decrypt_resource(&self, resource: &WechatPayEncryptedResource) -> Result<Value> {
        if resource.algorithm != "AEAD_AES_256_GCM" {
            bail!(
                "unsupported WeChat Pay resource algorithm `{}`",
                resource.algorithm
            );
        }
        let key = self.config.api_v3_key.as_bytes();
        if key.len() != 32 {
            bail!("WECHAT_PAY_API_V3_KEY must be exactly 32 bytes");
        }

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| anyhow!("failed to initialize WeChat Pay AES-GCM cipher"))?;
        let ciphertext = BASE64
            .decode(&resource.ciphertext)
            .context("failed to decode WeChat Pay resource ciphertext")?;
        let plaintext = cipher
            .decrypt(
                resource.nonce.as_bytes().into(),
                Payload {
                    msg: &ciphertext,
                    aad: resource.associated_data.as_bytes(),
                },
            )
            .map_err(|_| anyhow!("failed to decrypt WeChat Pay notification resource"))?;

        serde_json::from_slice(&plaintext)
            .context("failed to decode decrypted WeChat Pay transaction payload")
    }
}

#[derive(Debug, Deserialize)]
struct NativePrepayResponse {
    code_url: String,
}

#[derive(Debug, Deserialize)]
struct JsapiPrepayResponse {
    prepay_id: String,
}

fn default_currency() -> String {
    "CNY".to_string()
}

fn default_payment_description() -> String {
    DEFAULT_PAYMENT_DESCRIPTION.to_string()
}

fn validate_prepay_request(request: &WechatPayPrepayRequest) -> Result<()> {
    if request.tenant_id.trim().is_empty() {
        bail!("tenant_id is required");
    }
    if request.amount_total == 0 {
        bail!("amount_total must be greater than zero");
    }
    if request.currency != "CNY" {
        bail!("only CNY WeChat Pay orders are supported");
    }
    if matches!(request.channel, WechatPayChannel::Jsapi) && request.payer_openid.is_none() {
        bail!("payer_openid is required for JSAPI prepay");
    }

    Ok(())
}

pub fn new_out_trade_no(tenant_id: &str, project_id: Option<&str>) -> String {
    build_out_trade_no(tenant_id, project_id)
}

fn validate_out_trade_no(out_trade_no: &str) -> Result<()> {
    if out_trade_no.trim().is_empty() || out_trade_no.len() > 32 {
        bail!("out_trade_no must be present and at most 32 bytes");
    }
    if !out_trade_no
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '|'))
    {
        bail!("out_trade_no contains unsupported characters");
    }

    Ok(())
}

fn build_prepay_body(
    config: &WechatPayConfig,
    request: &WechatPayPrepayRequest,
    out_trade_no: &str,
) -> Value {
    let mut body = serde_json::json!({
        "appid": config.app_id,
        "mchid": config.mchid,
        "description": request.description,
        "out_trade_no": out_trade_no,
        "notify_url": config.notify_url,
        "amount": {
            "total": request.amount_total,
            "currency": request.currency
        }
    });

    if let Some(attach) = request.attach.as_ref() {
        body["attach"] = Value::String(attach.clone());
    }

    if matches!(request.channel, WechatPayChannel::Jsapi) {
        body["payer"] = serde_json::json!({
            "openid": request.payer_openid.as_ref().expect("validated payer_openid")
        });
    }

    body
}

fn build_out_trade_no(tenant_id: &str, project_id: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tenant_id.as_bytes());
    if let Some(project_id) = project_id {
        hasher.update(b":");
        hasher.update(project_id.as_bytes());
    }
    hasher.update(b":");
    hasher.update(unix_timestamp().as_bytes());
    hasher.update(b":");
    hasher.update(nonce_string().as_bytes());
    let digest = hasher.finalize();
    format!("hr{}", hex_prefix(&digest, 30))
}

fn hex_prefix(bytes: &[u8], length: usize) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .take(length)
        .map(|nibble| char::from_digit(u32::from(nibble), 16).expect("hex nibble"))
        .collect()
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

fn signature_message(method: &str, path: &str, timestamp: &str, nonce: &str, body: &str) -> String {
    format!("{method}\n{path}\n{timestamp}\n{nonce}\n{body}\n")
}

fn sign_message(private_key: &RsaPrivateKey, message: &str) -> String {
    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let signature = signing_key.sign(message.as_bytes());
    BASE64.encode(signature.to_bytes())
}

fn render_qr_svg(value: &str) -> Result<String> {
    let code = QrCode::new(value.as_bytes()).context("failed to build payment QR code")?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(220, 220)
        .dark_color(svg::Color("#111827"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

fn verify_message(public_key: &RsaPublicKey, message: &str, signature: &str) -> Result<()> {
    let signature = BASE64
        .decode(signature)
        .context("failed to decode WeChat Pay RSA signature")?;
    let signature = RsaSignature::try_from(signature.as_slice())
        .context("failed to parse WeChat Pay RSA signature")?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key.clone());
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|error| anyhow!("RSA-SHA256 verification failed: {error}"))
}

fn parse_private_key(pem: &str) -> Result<RsaPrivateKey> {
    RsaPrivateKey::from_pkcs8_pem(pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem))
        .context("failed to parse WeChat Pay merchant private key PEM")
}

fn parse_public_key(pem: &str) -> Result<RsaPublicKey> {
    RsaPublicKey::from_public_key_pem(pem)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(pem))
        .context("failed to parse WeChat Pay platform public key PEM")
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name)
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}

fn env_secret_or_file(value_name: &str, path_name: &str) -> Result<String> {
    optional_env_secret_or_file(value_name, path_name)?
        .ok_or_else(|| anyhow!("{value_name} or {path_name} is required"))
}

fn optional_env_secret_or_file(value_name: &str, path_name: &str) -> Result<Option<String>> {
    if let Ok(value) = std::env::var(value_name) {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return Ok(Some(value));
        }
    }

    if let Ok(path) = std::env::var(path_name) {
        let path = path.trim();
        if !path.is_empty() {
            return std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {path_name} at {path}"))
                .map(Some);
        }
    }

    Ok(None)
}

fn header_value<'a>(headers: &'a reqwest::header::HeaderMap, name: &str) -> Result<&'a str> {
    headers
        .get(name)
        .ok_or_else(|| anyhow!("missing WeChat Pay response header {name}"))?
        .to_str()
        .with_context(|| format!("invalid WeChat Pay response header {name}"))
}

#[cfg(test)]
mod tests {
    use super::{
        WechatPayChannel, WechatPayPrepayRequest, build_out_trade_no, validate_out_trade_no,
        validate_prepay_request,
    };

    #[test]
    fn prepay_validation_requires_positive_amount() {
        let request = WechatPayPrepayRequest {
            tenant_id: "tenant_acme".to_string(),
            project_id: None,
            amount_total: 0,
            currency: "CNY".to_string(),
            description: "Recharge".to_string(),
            channel: WechatPayChannel::Native,
            payer_openid: None,
            attach: None,
        };

        assert!(validate_prepay_request(&request).is_err());
    }

    #[test]
    fn jsapi_prepay_requires_openid() {
        let request = WechatPayPrepayRequest {
            tenant_id: "tenant_acme".to_string(),
            project_id: None,
            amount_total: 1,
            currency: "CNY".to_string(),
            description: "Recharge".to_string(),
            channel: WechatPayChannel::Jsapi,
            payer_openid: None,
            attach: None,
        };

        assert!(validate_prepay_request(&request).is_err());
    }

    #[test]
    fn out_trade_no_fits_wechat_pay_length_limit() {
        let out_trade_no = build_out_trade_no("tenant_acme", Some("proj_core"));

        assert!(out_trade_no.starts_with("hr"));
        assert!(out_trade_no.len() <= 32);
        assert!(validate_out_trade_no(&out_trade_no).is_ok());
    }
}
