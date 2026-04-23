use async_trait::async_trait;
use provider_traits::{
    AdapterManifest, ProviderAdapter, ProviderError, ProviderErrorKind, ProviderExecutionContext,
    ProviderRequest, ProviderResponse, ProviderUsage, StreamingSupport,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

const TRANSIT_HEADER_HOP: &str = "x-hugerouter-transit-hop";
const TRANSIT_HEADER_VIA: &str = "x-hugerouter-transit-via";
const TRANSIT_HEADER_ORIGIN: &str = "x-hugerouter-transit-origin";
const TRANSIT_HEADER_PROVIDER: &str = "x-hugerouter-transit-provider";
const DEFAULT_TRANSIT_TIMEOUT_MS: u64 = 30_000;

const PASSTHROUGH_HEADER_ALLOWLIST: &[&str] = &[
    "accept",
    "accept-encoding",
    "accept-language",
    "idempotency-key",
    "openai-organization",
    "openai-project",
    "user-agent",
    "x-correlation-id",
    "x-request-id",
    "x-trace-id",
];

const STRIP_HEADER_DENYLIST: &[&str] = &[
    "authorization",
    "connection",
    "content-length",
    "forwarded",
    "host",
    "proxy-authorization",
    "proxy-connection",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "via",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "x-portkey-forward-headers",
    TRANSIT_HEADER_HOP,
    TRANSIT_HEADER_ORIGIN,
    TRANSIT_HEADER_PROVIDER,
    TRANSIT_HEADER_VIA,
];

#[derive(Clone)]
pub struct GatewayAdapter {
    transport: Arc<dyn HttpTransport>,
}

impl GatewayAdapter {
    #[must_use]
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self { transport }
    }
}

impl Default for GatewayAdapter {
    fn default() -> Self {
        Self::new(Arc::new(ReqwestTransport::from_env()))
    }
}

#[async_trait]
impl ProviderAdapter for GatewayAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            adapter_id: "gateway-openai-compatible-v1",
            provider_kind: "gateway",
            display_name: "OpenAI-Compatible Transit Gateway",
            protocol_family: "openai_chat",
            streaming_support: StreamingSupport::Unsupported,
        }
    }

    async fn execute_chat(
        &self,
        request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        if request.stream {
            return Err(base_error(
                ProviderErrorKind::InvalidRequest,
                "stream=true is not supported for transit gateway targets",
                false,
                context,
            ));
        }

        let headers = build_transit_headers(context)?;
        let response = self
            .transport
            .post_json(HttpRequest {
                url: format!(
                    "{}/chat/completions",
                    context.endpoint.endpoint_base_url.trim_end_matches('/')
                ),
                headers,
                body: json!({
                    "model": request.model,
                    "messages": request.messages.iter().map(OpenAiChatMessage::from).collect::<Vec<_>>(),
                    "stream": false,
                }),
            })
            .await
            .map_err(|error| {
                base_error(
                    match error.kind {
                        HttpTransportErrorKind::Timeout => ProviderErrorKind::Timeout,
                        HttpTransportErrorKind::Network => ProviderErrorKind::Unavailable,
                    },
                    error.message,
                    error.retryable,
                    context,
                )
                .with_detail("target_kind", "transit_gateway")
                .with_detail("transit_gateway", "openai_compatible")
            })?;

        if response.status >= 400 {
            return Err(map_error_response(&response, context));
        }

        let body = response.body.as_deref().ok_or_else(|| {
            base_error(
                ProviderErrorKind::Protocol,
                "transit gateway returned an empty response body",
                false,
                context,
            )
        })?;

        parse_chat_completion_response(body, request, context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderPolicyReport {
    pub forwarded_headers: Vec<String>,
    pub stripped_headers: Vec<String>,
    pub rewritten_headers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTransitHeaders {
    pub headers: Vec<(String, String)>,
    pub report: HeaderPolicyReport,
}

/// Prepares the outbound header set for a transit-gateway provider request.
///
/// # Errors
///
/// Returns [`ProviderError`] when the incoming request already carries
/// transit-loop headers and forwarding would recurse through another gateway hop.
pub fn prepare_transit_headers(
    context: &ProviderExecutionContext,
) -> Result<PreparedTransitHeaders, ProviderError> {
    ensure_no_transit_loop(context)?;

    let allowlist = PASSTHROUGH_HEADER_ALLOWLIST
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let denylist = STRIP_HEADER_DENYLIST
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut headers = BTreeMap::new();
    let mut forwarded_headers = Vec::new();
    let mut stripped_headers = Vec::new();

    for (name, value) in &context.request_headers {
        if denylist.contains(name.as_str()) {
            stripped_headers.push(name.clone());
            continue;
        }

        if allowlist.contains(name.as_str()) {
            headers.insert(name.clone(), value.clone());
            forwarded_headers.push(name.clone());
        }
    }

    headers.insert(
        AUTHORIZATION.as_str().to_string(),
        format!("Bearer {}", context.endpoint.api_key),
    );
    headers.insert(
        CONTENT_TYPE.as_str().to_string(),
        "application/json".to_string(),
    );
    headers.insert(TRANSIT_HEADER_HOP.to_string(), "1".to_string());
    headers.insert(
        TRANSIT_HEADER_VIA.to_string(),
        context.gateway_service_name.clone(),
    );
    headers.insert(
        TRANSIT_HEADER_ORIGIN.to_string(),
        context
            .gateway_origin
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    );
    headers.insert(
        TRANSIT_HEADER_PROVIDER.to_string(),
        context.endpoint.provider_resource_id.clone(),
    );
    headers.insert("x-request-id".to_string(), context.request_id.clone());
    headers.insert("x-trace-id".to_string(), context.trace_id.clone());

    let report = HeaderPolicyReport {
        forwarded_headers,
        stripped_headers,
        rewritten_headers: vec![
            AUTHORIZATION.as_str().to_string(),
            CONTENT_TYPE.as_str().to_string(),
            TRANSIT_HEADER_HOP.to_string(),
            TRANSIT_HEADER_ORIGIN.to_string(),
            TRANSIT_HEADER_PROVIDER.to_string(),
            TRANSIT_HEADER_VIA.to_string(),
            "x-request-id".to_string(),
            "x-trace-id".to_string(),
        ],
    };

    Ok(PreparedTransitHeaders {
        headers: headers.into_iter().collect(),
        report,
    })
}

fn build_transit_headers(
    context: &ProviderExecutionContext,
) -> Result<Vec<(String, String)>, ProviderError> {
    Ok(prepare_transit_headers(context)?.headers)
}

fn ensure_no_transit_loop(context: &ProviderExecutionContext) -> Result<(), ProviderError> {
    for header in [
        TRANSIT_HEADER_HOP,
        TRANSIT_HEADER_ORIGIN,
        TRANSIT_HEADER_PROVIDER,
        TRANSIT_HEADER_VIA,
    ] {
        if context.request_headers.contains_key(header) {
            return Err(base_error(
                ProviderErrorKind::InvalidRequest,
                format!("transit loop prevention rejected header `{header}`"),
                false,
                context,
            )
            .with_detail("loop_guard", "transit_header_present")
            .with_detail("rejected_header", header));
        }
    }

    if let Some(origin) = context.gateway_origin.as_deref()
        && same_origin(origin, &context.endpoint.endpoint_base_url)
    {
        return Err(base_error(
            ProviderErrorKind::InvalidRequest,
            "transit loop prevention rejected a self-targeted gateway endpoint",
            false,
            context,
        )
        .with_detail("loop_guard", "self_target")
        .with_detail("gateway_origin", origin)
        .with_detail("endpoint_base_url", &context.endpoint.endpoint_base_url));
    }

    Ok(())
}

fn same_origin(left: &str, right: &str) -> bool {
    let normalized_left = left.trim_end_matches('/').to_ascii_lowercase();
    let normalized_right = right.trim_end_matches('/').to_ascii_lowercase();
    normalized_left == normalized_right
}

fn parse_chat_completion_response(
    body: &str,
    request: &ProviderRequest,
    context: &ProviderExecutionContext,
) -> Result<ProviderResponse, ProviderError> {
    let payload: OpenAiChatCompletionResponse = serde_json::from_str(body).map_err(|error| {
        base_error(
            ProviderErrorKind::Protocol,
            format!("failed to decode transit gateway response: {error}"),
            false,
            context,
        )
    })?;

    let choice = payload.choices.into_iter().next().ok_or_else(|| {
        base_error(
            ProviderErrorKind::Protocol,
            "transit gateway response did not include any choices",
            false,
            context,
        )
    })?;
    let output_text = choice.message.content.ok_or_else(|| {
        base_error(
            ProviderErrorKind::Protocol,
            "transit gateway response did not include assistant content",
            false,
            context,
        )
    })?;

    let cached_input_tokens = payload
        .usage
        .as_ref()
        .and_then(|usage| usage.prompt_tokens_details.as_ref())
        .and_then(|details| details.cached_tokens)
        .unwrap_or_default();
    let usage = payload.usage.unwrap_or_default();

    Ok(ProviderResponse {
        response_id: payload.id,
        model: payload.model.unwrap_or_else(|| request.model.clone()),
        output_text,
        finish_reason: choice.finish_reason.unwrap_or_else(|| "stop".to_string()),
        usage: ProviderUsage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cached_input_tokens,
        },
    })
}

fn map_error_response(
    response: &HttpResponse,
    context: &ProviderExecutionContext,
) -> ProviderError {
    let parsed_error = response
        .body
        .as_deref()
        .and_then(|raw| serde_json::from_str::<OpenAiErrorResponse>(raw).ok())
        .and_then(|payload| payload.error);

    let status = response.status;
    let message = parsed_error.as_ref().map_or_else(
        || format!("transit gateway returned HTTP {status}"),
        |error| error.message.clone(),
    );
    let upstream_code = parsed_error
        .as_ref()
        .and_then(|error| error.code.clone().or_else(|| error.error_type.clone()));
    let (kind, retryable) = match status {
        401 | 403 => (ProviderErrorKind::Auth, false),
        400 | 402 | 404 | 409 => (ProviderErrorKind::InvalidRequest, false),
        408 | 504 => (ProviderErrorKind::Timeout, true),
        429 => (ProviderErrorKind::RateLimited, true),
        500..=599 => (ProviderErrorKind::Unavailable, true),
        _ => (ProviderErrorKind::Protocol, false),
    };

    let mut error = base_error(kind, message, retryable, context)
        .with_upstream_status(Some(status))
        .with_upstream_code(upstream_code)
        .with_detail("target_kind", "transit_gateway")
        .with_detail("transit_gateway", "openai_compatible")
        .with_detail("endpoint_base_url", &context.endpoint.endpoint_base_url);

    if let Some(request_id) = first_response_header(response, &["x-request-id", "request-id"]) {
        error = error.with_detail("upstream_request_id", request_id);
    }

    error
}

fn first_response_header<'a>(response: &'a HttpResponse, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| response.headers.get(*name).map(String::as_str))
}

fn base_error(
    kind: ProviderErrorKind,
    message: impl Into<String>,
    retryable: bool,
    context: &ProviderExecutionContext,
) -> ProviderError {
    ProviderError::new(kind, message, retryable).with_detail(
        "provider_resource_id",
        &context.endpoint.provider_resource_id,
    )
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpTransportErrorKind {
    Timeout,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpTransportError {
    pub kind: HttpTransportErrorKind,
    pub message: String,
    pub retryable: bool,
}

#[async_trait]
pub trait HttpTransport: Send + Sync {
    async fn post_json(&self, request: HttpRequest) -> Result<HttpResponse, HttpTransportError>;
}

struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    fn from_env() -> Self {
        let timeout = std::env::var("GATEWAY_TRANSIT_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map_or_else(
                || Duration::from_millis(DEFAULT_TRANSIT_TIMEOUT_MS),
                Duration::from_millis,
            );
        let client = reqwest::Client::builder()
            .use_native_tls()
            .timeout(timeout)
            .build()
            .expect("native-tls reqwest client should build");
        Self { client }
    }
}

#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn post_json(&self, request: HttpRequest) -> Result<HttpResponse, HttpTransportError> {
        let mut builder = self.client.post(&request.url).json(&request.body);
        for (key, value) in &request.headers {
            builder = builder.header(key, value);
        }

        let response = builder.send().await.map_err(|error| HttpTransportError {
            kind: if error.is_timeout() {
                HttpTransportErrorKind::Timeout
            } else {
                HttpTransportErrorKind::Network
            },
            message: error.to_string(),
            retryable: error.is_timeout() || error.is_connect(),
        })?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.to_string(), value.to_string()))
            })
            .collect::<BTreeMap<_, _>>();
        let body = response.text().await.map_err(|error| HttpTransportError {
            kind: HttpTransportErrorKind::Network,
            message: error.to_string(),
            retryable: false,
        })?;

        Ok(HttpResponse {
            status,
            headers,
            body: Some(body),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiErrorResponse {
    error: Option<OpenAiError>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiError {
    message: String,
    code: Option<String>,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiChatCompletionResponse {
    id: Option<String>,
    model: Option<String>,
    #[serde(default)]
    choices: Vec<OpenAiChatCompletionChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiChatCompletionChoice {
    message: OpenAiChatCompletionMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiChatCompletionMessage {
    content: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    prompt_tokens_details: Option<OpenAiPromptTokenDetails>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiPromptTokenDetails {
    cached_tokens: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OpenAiChatMessage {
    role: String,
    content: String,
}

impl From<&provider_traits::ProviderMessage> for OpenAiChatMessage {
    fn from(message: &provider_traits::ProviderMessage) -> Self {
        Self {
            role: message.role.clone(),
            content: message.content.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GatewayAdapter, HttpRequest, HttpResponse, HttpTransport, HttpTransportError,
        HttpTransportErrorKind, TRANSIT_HEADER_HOP, TRANSIT_HEADER_ORIGIN, TRANSIT_HEADER_PROVIDER,
        TRANSIT_HEADER_VIA, prepare_transit_headers,
    };
    use provider_traits::{
        ProviderAdapter, ProviderEndpoint, ProviderErrorKind, ProviderExecutionContext,
        ProviderMessage, ProviderRequest,
    };
    use std::{collections::BTreeMap, sync::Arc};
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct MockTransport {
        captured: Arc<Mutex<Vec<HttpRequest>>>,
        response: Mutex<Option<Result<HttpResponse, HttpTransportError>>>,
    }

    impl MockTransport {
        fn success(response: HttpResponse) -> Arc<Self> {
            Arc::new(Self {
                captured: Arc::new(Mutex::new(Vec::new())),
                response: Mutex::new(Some(Ok(response))),
            })
        }

        fn failure(error: HttpTransportError) -> Arc<Self> {
            Arc::new(Self {
                captured: Arc::new(Mutex::new(Vec::new())),
                response: Mutex::new(Some(Err(error))),
            })
        }
    }

    #[async_trait::async_trait]
    impl HttpTransport for MockTransport {
        async fn post_json(
            &self,
            request: HttpRequest,
        ) -> Result<HttpResponse, HttpTransportError> {
            self.captured.lock().await.push(request);
            self.response
                .lock()
                .await
                .take()
                .expect("transport response should exist")
        }
    }

    fn context() -> ProviderExecutionContext {
        ProviderExecutionContext {
            request_id: "req_test".to_string(),
            trace_id: "trace_test".to_string(),
            gateway_service_name: "gateway-api".to_string(),
            gateway_origin: Some("https://router.example.com/v1".to_string()),
            request_headers: BTreeMap::from([
                ("accept".to_string(), "application/json".to_string()),
                ("authorization".to_string(), "Bearer caller".to_string()),
                ("idempotency-key".to_string(), "idem-123".to_string()),
                ("x-request-id".to_string(), "client-req".to_string()),
                ("x-forwarded-for".to_string(), "203.0.113.1".to_string()),
            ]),
            endpoint: ProviderEndpoint {
                provider_resource_id: "prvrsrc_gateway_primary".to_string(),
                endpoint_base_url: "https://gateway.example.com/v1".to_string(),
                api_key: "gateway-secret".to_string(),
            },
        }
    }

    fn request() -> ProviderRequest {
        ProviderRequest {
            model: "gpt-4.1-mini".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "hello transit".to_string(),
            }],
            stream: false,
        }
    }

    #[tokio::test]
    async fn forwards_successful_request_with_header_policy() {
        let transport = MockTransport::success(HttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: Some(
                serde_json::json!({
                    "id": "chatcmpl_gateway",
                    "model": "gpt-4.1-mini",
                    "choices": [{
                        "message": {"content": "forwarded ok"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 12,
                        "completion_tokens": 7,
                        "prompt_tokens_details": {"cached_tokens": 2}
                    }
                })
                .to_string(),
            ),
        });
        let adapter = GatewayAdapter::new(transport.clone());

        let response = adapter.execute_chat(&request(), &context()).await.unwrap();

        assert_eq!(response.output_text, "forwarded ok");
        assert_eq!(response.usage.cached_input_tokens, 2);

        let headers = {
            let captured = transport.captured.lock().await;
            captured[0]
                .headers
                .iter()
                .cloned()
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(
            headers.get("authorization"),
            Some(&"Bearer gateway-secret".to_string())
        );
        assert_eq!(
            headers.get("idempotency-key"),
            Some(&"idem-123".to_string())
        );
        assert_eq!(headers.get(TRANSIT_HEADER_HOP), Some(&"1".to_string()));
        assert_eq!(
            headers.get(TRANSIT_HEADER_VIA),
            Some(&"gateway-api".to_string())
        );
        assert_eq!(
            headers.get(TRANSIT_HEADER_PROVIDER),
            Some(&"prvrsrc_gateway_primary".to_string())
        );
        assert_eq!(
            headers.get(TRANSIT_HEADER_ORIGIN),
            Some(&"https://router.example.com/v1".to_string())
        );
        assert!(!headers.contains_key("x-forwarded-for"));
    }

    #[tokio::test]
    async fn preserves_upstream_4xx_diagnostics() {
        let adapter = GatewayAdapter::new(MockTransport::success(HttpResponse {
            status: 429,
            headers: BTreeMap::from([("x-request-id".to_string(), "upstream-req-1".to_string())]),
            body: Some(
                serde_json::json!({
                    "error": {
                        "message": "too many requests",
                        "code": "rate_limit_exceeded",
                        "type": "rate_limit_error"
                    }
                })
                .to_string(),
            ),
        }));

        let error = adapter
            .execute_chat(&request(), &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::RateLimited);
        assert_eq!(error.upstream_status_code, Some(429));
        assert_eq!(error.upstream_code.as_deref(), Some("rate_limit_exceeded"));
        assert_eq!(
            error.details.get("upstream_request_id"),
            Some(&"upstream-req-1".to_string())
        );
    }

    #[tokio::test]
    async fn preserves_upstream_5xx_diagnostics() {
        let adapter = GatewayAdapter::new(MockTransport::success(HttpResponse {
            status: 503,
            headers: BTreeMap::from([("request-id".to_string(), "gw-503".to_string())]),
            body: Some(
                serde_json::json!({
                    "error": {
                        "message": "upstream overloaded",
                        "code": "gateway_overloaded"
                    }
                })
                .to_string(),
            ),
        }));

        let error = adapter
            .execute_chat(&request(), &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Unavailable);
        assert!(error.retryable);
        assert_eq!(
            error.details.get("upstream_request_id"),
            Some(&"gw-503".to_string())
        );
        assert_eq!(
            error.details.get("target_kind"),
            Some(&"transit_gateway".to_string())
        );
    }

    #[tokio::test]
    async fn maps_transport_timeouts_to_retryable_timeout() {
        let adapter = GatewayAdapter::new(MockTransport::failure(HttpTransportError {
            kind: HttpTransportErrorKind::Timeout,
            message: "deadline exceeded".to_string(),
            retryable: true,
        }));

        let error = adapter
            .execute_chat(&request(), &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Timeout);
        assert!(error.retryable);
        assert_eq!(
            error.details.get("provider_resource_id"),
            Some(&"prvrsrc_gateway_primary".to_string())
        );
    }

    #[test]
    fn rejects_existing_transit_headers_to_prevent_loops() {
        let mut context = context();
        context
            .request_headers
            .insert(TRANSIT_HEADER_HOP.to_string(), "3".to_string());

        let error = prepare_transit_headers(&context).unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::InvalidRequest);
        assert_eq!(
            error.details.get("loop_guard"),
            Some(&"transit_header_present".to_string())
        );
    }

    #[test]
    fn rejects_self_targeted_gateway_origin() {
        let mut context = context();
        context.gateway_origin = Some("https://gateway.example.com/v1".to_string());

        let error = prepare_transit_headers(&context).unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::InvalidRequest);
        assert_eq!(
            error.details.get("loop_guard"),
            Some(&"self_target".to_string())
        );
    }

    #[test]
    fn header_policy_reports_forwarded_and_stripped_headers() {
        let prepared = prepare_transit_headers(&context()).unwrap();

        assert!(
            prepared
                .report
                .forwarded_headers
                .contains(&"accept".to_string())
        );
        assert!(
            prepared
                .report
                .stripped_headers
                .contains(&"authorization".to_string())
        );
        assert!(
            prepared
                .report
                .rewritten_headers
                .contains(&TRANSIT_HEADER_PROVIDER.to_string())
        );
    }
}
