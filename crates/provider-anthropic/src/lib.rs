use async_trait::async_trait;
use protocol_anthropic::AnthropicMessageResponse;
use provider_traits::{
    AdapterLifecycleFamily, AdapterManifest, AdapterStability,
    CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION, ProviderAdapter, ProviderError, ProviderErrorKind,
    ProviderExecutionContext, ProviderRequest, ProviderResponse, ProviderUsage, StreamingSupport,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 1024;

#[derive(Clone)]
pub struct AnthropicAdapter {
    transport: Arc<dyn HttpTransport>,
    anthropic_version: String,
}

impl AnthropicAdapter {
    #[must_use]
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            anthropic_version: DEFAULT_ANTHROPIC_VERSION.to_string(),
        }
    }

    #[must_use]
    pub fn with_version(
        transport: Arc<dyn HttpTransport>,
        anthropic_version: impl Into<String>,
    ) -> Self {
        Self {
            transport,
            anthropic_version: anthropic_version.into(),
        }
    }
}

impl Default for AnthropicAdapter {
    fn default() -> Self {
        Self::new(Arc::new(ReqwestTransport::default()))
    }
}

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
            adapter_id: "anthropic-messages-v1",
            provider_kind: "anthropic",
            display_name: "Anthropic Messages",
            protocol_family: "anthropic_messages",
            supported_protocol_families: &["anthropic_messages"],
            lifecycle_family: AdapterLifecycleFamily::Inference,
            stability: AdapterStability::Stable,
            streaming_support: StreamingSupport::Unsupported,
            configuration_schema_ref: Some("env:GATEWAY_ANTHROPIC_*"),
        }
    }

    async fn execute_chat(
        &self,
        request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        if request.stream {
            return Err(ProviderError::new(
                ProviderErrorKind::InvalidRequest,
                "stream=true is not supported in non-streaming adapter mode",
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
            ));
        }

        let http_request = HttpRequest {
            url: format!(
                "{}/messages",
                context.endpoint.endpoint_base_url.trim_end_matches('/')
            ),
            headers: vec![
                ("x-api-key".to_string(), context.endpoint.api_key.clone()),
                (
                    "anthropic-version".to_string(),
                    self.anthropic_version.clone(),
                ),
                ("content-type".to_string(), "application/json".to_string()),
            ],
            body: build_upstream_request(request),
        };

        let response = self
            .transport
            .post_json(http_request)
            .await
            .map_err(|error| {
                let kind = match error.kind {
                    HttpTransportErrorKind::Timeout => ProviderErrorKind::Timeout,
                    HttpTransportErrorKind::Network => ProviderErrorKind::Unavailable,
                };

                ProviderError::new(kind, error.message, error.retryable).with_detail(
                    "provider_resource_id",
                    &context.endpoint.provider_resource_id,
                )
            })?;

        if response.status >= 400 {
            return Err(map_error_response(
                response.status,
                response.body.as_deref(),
                &context.endpoint.provider_resource_id,
            ));
        }

        let payload = serde_json::from_str::<AnthropicMessageResponse>(
            response.body.as_deref().ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorKind::Protocol,
                    "Anthropic returned an empty response body",
                    false,
                )
                .with_detail(
                    "provider_resource_id",
                    &context.endpoint.provider_resource_id,
                )
            })?,
        )
        .map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::Protocol,
                format!("failed to decode Anthropic response: {error}"),
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
            )
        })?;

        let output_text = protocol_anthropic::response_text(&payload).map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::Protocol,
                format!("invalid Anthropic response content: {error}"),
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
            )
        })?;

        let finish_reason = payload
            .stop_reason
            .unwrap_or_else(|| "end_turn".to_string());
        let usage = payload.usage.unwrap_or_default();

        Ok(ProviderResponse {
            response_id: payload.id,
            model: payload.model.unwrap_or_else(|| request.model.clone()),
            output_text,
            finish_reason,
            usage: ProviderUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cached_input_tokens: 0,
            },
        })
    }
}

fn build_upstream_request(request: &ProviderRequest) -> Value {
    let messages = request
        .messages
        .iter()
        .map(|message| {
            json!({
                "role": &message.role,
                "content": [{
                    "type": "text",
                    "text": &message.content,
                }],
            })
        })
        .collect::<Vec<_>>();

    json!({
        "model": request.model,
        "max_tokens": DEFAULT_MAX_TOKENS,
        "messages": messages,
    })
}

fn map_error_response(
    status: u16,
    body: Option<&str>,
    provider_resource_id: &str,
) -> ProviderError {
    let parsed_error =
        body.and_then(|raw| serde_json::from_str::<AnthropicErrorResponse>(raw).ok());
    let upstream_code = parsed_error
        .as_ref()
        .and_then(|error| error.error.as_ref().and_then(|value| value.code.clone()));
    let message = parsed_error
        .as_ref()
        .and_then(AnthropicErrorResponse::message)
        .unwrap_or_else(|| format!("Anthropic returned HTTP {status}"));
    let (kind, retryable) = match status {
        401 | 403 => (ProviderErrorKind::Auth, false),
        400 | 402 | 404 => (ProviderErrorKind::InvalidRequest, false),
        408 | 504 => (ProviderErrorKind::Timeout, true),
        429 => (ProviderErrorKind::RateLimited, true),
        500..=599 => (ProviderErrorKind::Unavailable, true),
        _ => (ProviderErrorKind::Protocol, false),
    };

    ProviderError::new(kind, message, retryable)
        .with_upstream_status(Some(status))
        .with_upstream_code(upstream_code)
        .with_detail("provider_resource_id", provider_resource_id)
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpTransportErrorKind {
    Timeout,
    Network,
}

#[derive(Debug, Clone)]
pub struct HttpTransportError {
    pub kind: HttpTransportErrorKind,
    pub message: String,
    pub retryable: bool,
}

#[async_trait]
pub trait HttpTransport: Send + Sync {
    async fn post_json(&self, request: HttpRequest) -> Result<HttpResponse, HttpTransportError>;
}

#[derive(Default)]
struct ReqwestTransport {
    client: reqwest::Client,
}

#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn post_json(&self, request: HttpRequest) -> Result<HttpResponse, HttpTransportError> {
        let mut builder = self.client.post(&request.url).json(&request.body);
        let mut headers = BTreeMap::new();
        for (name, value) in request.headers {
            headers.insert(name, value);
        }
        for (name, value) in headers {
            builder = builder.header(name, value);
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
        let body = response.text().await.map_err(|error| HttpTransportError {
            kind: HttpTransportErrorKind::Network,
            message: error.to_string(),
            retryable: false,
        })?;

        Ok(HttpResponse {
            status,
            body: Some(body),
        })
    }
}

#[derive(Debug, serde::Deserialize)]
struct AnthropicErrorResponse {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<AnthropicApiError>,
}

#[derive(Debug, serde::Deserialize)]
struct AnthropicApiError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(rename = "type", default)]
    _error_type: Option<String>,
}

impl AnthropicErrorResponse {
    fn message(&self) -> Option<String> {
        self.message
            .clone()
            .or_else(|| self.error.as_ref().and_then(|error| error.message.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use provider_traits::{
        ProviderEndpoint, ProviderErrorKind, ProviderExecutionContext, ProviderMessage,
        ProviderRequest,
    };
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct MockTransport {
        responses: Arc<Mutex<Vec<Result<HttpResponse, HttpTransportError>>>>,
        requests: Arc<Mutex<Vec<HttpRequest>>>,
    }

    impl MockTransport {
        fn new(responses: Vec<Result<HttpResponse, HttpTransportError>>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl HttpTransport for MockTransport {
        async fn post_json(
            &self,
            request: HttpRequest,
        ) -> Result<HttpResponse, HttpTransportError> {
            self.requests.lock().unwrap().push(request);
            self.responses.lock().unwrap().pop().unwrap_or_else(|| {
                Err(HttpTransportError {
                    kind: HttpTransportErrorKind::Network,
                    message: "no response prepared".to_string(),
                    retryable: true,
                })
            })
        }
    }

    fn context() -> ProviderExecutionContext {
        ProviderExecutionContext {
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            gateway_service_name: "gateway-api".to_string(),
            gateway_origin: Some("https://router.example.com/v1".to_string()),
            request_headers: BTreeMap::new(),
            endpoint: ProviderEndpoint {
                provider_resource_id: "prvrsrc_anthropic_primary".to_string(),
                endpoint_base_url: "https://api.anthropic.example/v1".to_string(),
                api_key: "secret".to_string(),
                region: None,
            },
        }
    }

    fn request() -> ProviderRequest {
        ProviderRequest {
            model: "claude-opus-4-0".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "Hello Anthropic".to_string(),
            }],
            stream: false,
        }
    }

    #[tokio::test]
    async fn sends_anthropic_messages_request_and_extracts_usage_and_output() {
        let transport = MockTransport::new(vec![Ok(HttpResponse {
            status: 200,
            body: Some(
                json!({
                    "id": "msg_123",
                    "type": "message",
                    "model": "claude-opus-4-0",
                    "role": "assistant",
                    "content": [{
                        "type": "text",
                        "text": "Hello from Anthropic"
                    }],
                    "stop_reason": "end_turn",
                    "usage": {
                        "input_tokens": 13,
                        "output_tokens": 9
                    }
                })
                .to_string(),
            ),
        })]);
        let adapter = AnthropicAdapter::new(Arc::new(transport.clone()));

        let response = adapter.execute_chat(&request(), &context()).await.unwrap();
        let first_request = {
            let recorded_requests = transport.requests.lock().unwrap();
            assert_eq!(recorded_requests.len(), 1);
            recorded_requests[0].clone()
        };

        assert_eq!(
            first_request.url,
            "https://api.anthropic.example/v1/messages"
        );
        assert_eq!(response.response_id.as_deref(), Some("msg_123"));
        assert_eq!(response.output_text, "Hello from Anthropic");
        assert_eq!(response.usage.input_tokens, 13);
        assert_eq!(response.usage.output_tokens, 9);
    }

    #[tokio::test]
    async fn maps_rate_limit_error_to_retryable_provider_error() {
        let adapter = AnthropicAdapter::new(Arc::new(MockTransport::new(vec![Ok(HttpResponse {
            status: 429,
            body: Some(
                json!({
                    "error": {
                        "type": "rate_limit_error",
                        "message": "rate limit exceeded"
                    }
                })
                .to_string(),
            ),
        })])));
        let error = adapter
            .execute_chat(&request(), &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::RateLimited);
        assert!(error.retryable);
        assert_eq!(error.upstream_status_code, Some(429));
    }

    #[tokio::test]
    async fn maps_transport_timeout_to_timeout_error() {
        let adapter = AnthropicAdapter::new(Arc::new(MockTransport::new(vec![Err(
            HttpTransportError {
                kind: HttpTransportErrorKind::Timeout,
                message: "timed out".to_string(),
                retryable: true,
            },
        )])));
        let error = adapter
            .execute_chat(&request(), &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Timeout);
        assert!(error.retryable);
    }

    #[test]
    fn maps_provider_request_messages_to_anthropic_messages_payload() {
        let body = build_upstream_request(&request());
        let messages = body["messages"]
            .as_array()
            .expect("messages should be array");
        assert_eq!(body["model"], "claude-opus-4-0");
        assert_eq!(body["max_tokens"], 1024);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"][0]["type"], "text");
        assert_eq!(messages[0]["content"][0]["text"], "Hello Anthropic");
    }
}
