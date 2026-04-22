use async_trait::async_trait;
use provider_traits::{
    AdapterManifest, ProviderAdapter, ProviderError, ProviderErrorKind, ProviderExecutionContext,
    ProviderRequest, ProviderResponse, ProviderUsage, StreamingSupport,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub struct OpenAiAdapter {
    transport: Arc<dyn HttpTransport>,
}

impl OpenAiAdapter {
    #[must_use]
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self { transport }
    }
}

impl Default for OpenAiAdapter {
    fn default() -> Self {
        Self::new(Arc::new(ReqwestTransport::default()))
    }
}

#[async_trait]
impl ProviderAdapter for OpenAiAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            adapter_id: "openai-chat-completions-v1",
            provider_kind: "openai",
            display_name: "OpenAI Chat Completions",
            protocol_family: "openai_chat",
            streaming_support: StreamingSupport::ServerSentEvents,
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn execute_chat(
        &self,
        request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        let http_request = HttpRequest {
            url: format!(
                "{}/chat/completions",
                context.endpoint.endpoint_base_url.trim_end_matches('/')
            ),
            headers: vec![
                (
                    AUTHORIZATION.as_str().to_string(),
                    format!("Bearer {}", context.endpoint.api_key),
                ),
                (
                    CONTENT_TYPE.as_str().to_string(),
                    "application/json".to_string(),
                ),
            ],
            body: json!({
                "model": request.model,
                "messages": request.messages.iter().map(OpenAiChatMessage::from).collect::<Vec<_>>(),
                "stream": false,
            }),
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

        let payload: OpenAiChatCompletionResponse =
            serde_json::from_str(response.body.as_deref().ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorKind::Protocol,
                    "OpenAI returned an empty response body",
                    false,
                )
                .with_detail(
                    "provider_resource_id",
                    &context.endpoint.provider_resource_id,
                )
            })?)
            .map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::Protocol,
                    format!("failed to decode OpenAI response: {error}"),
                    false,
                )
                .with_detail(
                    "provider_resource_id",
                    &context.endpoint.provider_resource_id,
                )
            })?;

        let choice = payload.choices.into_iter().next().ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::Protocol,
                "OpenAI response did not include any choices",
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
            )
        })?;

        let output_text = choice.message.content.ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::Protocol,
                "OpenAI response did not include assistant content",
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
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
}

fn map_error_response(
    status: u16,
    body: Option<&str>,
    provider_resource_id: &str,
) -> ProviderError {
    let parsed_error = body
        .and_then(|raw| serde_json::from_str::<OpenAiErrorResponse>(raw).ok())
        .and_then(|response| response.error);

    let upstream_code = parsed_error
        .as_ref()
        .and_then(|error| error.code.clone().or_else(|| error.error_type.clone()));
    let message = parsed_error.as_ref().map_or_else(
        || format!("OpenAI returned HTTP {status}"),
        |error| error.message.clone(),
    );
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
        let _headers: BTreeMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.to_string(), value.to_string()))
            })
            .collect();
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiAssistantMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiAssistantMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<OpenAiPromptTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct OpenAiPromptTokenDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorResponse {
    #[serde(default)]
    error: Option<OpenAiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorBody {
    message: String,
    #[serde(rename = "type", default)]
    error_type: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        HttpRequest, HttpResponse, HttpTransport, HttpTransportError, HttpTransportErrorKind,
        OpenAiAdapter,
    };
    use async_trait::async_trait;
    use provider_traits::{
        ProviderAdapter, ProviderEndpoint, ProviderErrorKind, ProviderExecutionContext,
        ProviderMessage, ProviderRequest,
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
            self.responses.lock().unwrap().remove(0)
        }
    }

    fn context() -> ProviderExecutionContext {
        ProviderExecutionContext {
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            endpoint: ProviderEndpoint {
                provider_resource_id: "prvrsrc_openai_primary".to_string(),
                endpoint_base_url: "https://api.openai.example/v1".to_string(),
                api_key: "secret".to_string(),
            },
        }
    }

    fn request() -> ProviderRequest {
        ProviderRequest {
            model: "gpt-4.1-mini".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }],
            stream: false,
        }
    }

    #[tokio::test]
    async fn extracts_usage_from_success_response() {
        let transport = MockTransport::new(vec![Ok(HttpResponse {
            status: 200,
            body: Some(
                json!({
                    "id": "chatcmpl_123",
                    "model": "gpt-4.1-mini",
                    "choices": [{
                        "message": { "content": "world" },
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 12,
                        "completion_tokens": 8,
                        "prompt_tokens_details": {
                            "cached_tokens": 3
                        }
                    }
                })
                .to_string(),
            ),
        })]);
        let adapter = OpenAiAdapter::new(Arc::new(transport.clone()));

        let response = adapter.execute_chat(&request(), &context()).await.unwrap();

        assert_eq!(response.output_text, "world");
        assert_eq!(response.usage.input_tokens, 12);
        assert_eq!(response.usage.output_tokens, 8);
        assert_eq!(response.usage.cached_input_tokens, 3);

        let first_request_url = {
            let recorded_requests = transport.requests.lock().unwrap();
            assert_eq!(recorded_requests.len(), 1);
            recorded_requests[0].url.clone()
        };
        assert_eq!(
            first_request_url,
            "https://api.openai.example/v1/chat/completions"
        );
    }

    #[tokio::test]
    async fn maps_rate_limit_error_to_retryable_provider_error() {
        let adapter = OpenAiAdapter::new(Arc::new(MockTransport::new(vec![Ok(HttpResponse {
            status: 429,
            body: Some(
                json!({
                    "error": {
                        "message": "too many requests",
                        "type": "rate_limit_error",
                        "code": "rate_limit_exceeded"
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
        assert_eq!(error.upstream_code.as_deref(), Some("rate_limit_exceeded"));
    }

    #[tokio::test]
    async fn maps_transport_timeout_to_timeout_error() {
        let adapter = OpenAiAdapter::new(Arc::new(MockTransport::new(vec![Err(
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
}
