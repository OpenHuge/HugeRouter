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
    wire_api: OpenAiWireApi,
}

impl OpenAiAdapter {
    #[must_use]
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self::with_wire_api(transport, OpenAiWireApi::from_env())
    }

    #[must_use]
    fn with_wire_api(transport: Arc<dyn HttpTransport>, wire_api: OpenAiWireApi) -> Self {
        Self {
            transport,
            wire_api,
        }
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
            adapter_id: match self.wire_api {
                OpenAiWireApi::ChatCompletions => "openai-chat-completions-v1",
                OpenAiWireApi::Responses => "openai-responses-v1",
            },
            provider_kind: "openai",
            display_name: match self.wire_api {
                OpenAiWireApi::ChatCompletions => "OpenAI Chat Completions",
                OpenAiWireApi::Responses => "OpenAI Responses",
            },
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
        let http_request = match self.wire_api {
            OpenAiWireApi::ChatCompletions => HttpRequest {
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
            },
            OpenAiWireApi::Responses => HttpRequest {
                url: format!(
                    "{}/responses",
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
                    "input": request.messages.iter().map(OpenAiResponseInputMessage::from).collect::<Vec<_>>(),
                    "stream": false,
                }),
            },
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

        let body = response.body.as_deref().ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::Protocol,
                "OpenAI returned an empty response body",
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
            )
        })?;

        match self.wire_api {
            OpenAiWireApi::ChatCompletions => parse_chat_completion_response(
                body,
                request,
                &context.endpoint.provider_resource_id,
            ),
            OpenAiWireApi::Responses => {
                parse_responses_api_response(body, request, &context.endpoint.provider_resource_id)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenAiWireApi {
    ChatCompletions,
    Responses,
}

impl OpenAiWireApi {
    fn from_env() -> Self {
        match std::env::var("GATEWAY_OPENAI_WIRE_API")
            .or_else(|_| std::env::var("OPENAI_WIRE_API"))
            .ok()
            .as_deref()
        {
            Some("responses") => Self::Responses,
            _ => Self::ChatCompletions,
        }
    }
}

fn parse_chat_completion_response(
    body: &str,
    request: &ProviderRequest,
    provider_resource_id: &str,
) -> Result<ProviderResponse, ProviderError> {
    let payload: OpenAiChatCompletionResponse = serde_json::from_str(body).map_err(|error| {
        ProviderError::new(
            ProviderErrorKind::Protocol,
            format!("failed to decode OpenAI response: {error}"),
            false,
        )
        .with_detail("provider_resource_id", provider_resource_id)
    })?;

    let choice = payload.choices.into_iter().next().ok_or_else(|| {
        ProviderError::new(
            ProviderErrorKind::Protocol,
            "OpenAI response did not include any choices",
            false,
        )
        .with_detail("provider_resource_id", provider_resource_id)
    })?;

    let output_text = choice.message.content.ok_or_else(|| {
        ProviderError::new(
            ProviderErrorKind::Protocol,
            "OpenAI response did not include assistant content",
            false,
        )
        .with_detail("provider_resource_id", provider_resource_id)
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

fn parse_responses_api_response(
    body: &str,
    request: &ProviderRequest,
    provider_resource_id: &str,
) -> Result<ProviderResponse, ProviderError> {
    let payload: OpenAiResponsesApiResponse = serde_json::from_str(body).map_err(|error| {
        ProviderError::new(
            ProviderErrorKind::Protocol,
            format!("failed to decode OpenAI response: {error}"),
            false,
        )
        .with_detail("provider_resource_id", provider_resource_id)
    })?;
    let output_text = payload.output_text().ok_or_else(|| {
        ProviderError::new(
            ProviderErrorKind::Protocol,
            "OpenAI responses payload did not include assistant text",
            false,
        )
        .with_detail("provider_resource_id", provider_resource_id)
    })?;
    let finish_reason = payload.finish_reason();
    let model = payload
        .model
        .clone()
        .unwrap_or_else(|| request.model.clone());

    Ok(ProviderResponse {
        response_id: payload.id,
        model,
        output_text,
        finish_reason,
        usage: ProviderUsage {
            input_tokens: payload.usage.input_tokens,
            output_tokens: payload.usage.output_tokens,
            cached_input_tokens: payload
                .usage
                .input_tokens_details
                .as_ref()
                .and_then(|details| details.cached_tokens)
                .unwrap_or_default(),
        },
    })
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

struct ReqwestTransport {
    client: reqwest::Client,
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .use_native_tls()
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

#[derive(Debug, Clone, Serialize)]
struct OpenAiResponseInputMessage {
    role: String,
    content: Vec<OpenAiResponseInputContent>,
}

impl From<&provider_traits::ProviderMessage> for OpenAiResponseInputMessage {
    fn from(message: &provider_traits::ProviderMessage) -> Self {
        Self {
            role: message.role.clone(),
            content: vec![OpenAiResponseInputContent {
                content_type: "input_text",
                text: message.content.clone(),
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct OpenAiResponseInputContent {
    #[serde(rename = "type")]
    content_type: &'static str,
    text: String,
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

#[derive(Debug, Default, Deserialize)]
struct OpenAiResponsesApiUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    input_tokens_details: Option<OpenAiPromptTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesApiResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    output: Vec<OpenAiResponsesOutputItem>,
    #[serde(default)]
    usage: OpenAiResponsesApiUsage,
    #[serde(default)]
    status: Option<String>,
}

impl OpenAiResponsesApiResponse {
    fn output_text(&self) -> Option<String> {
        let text = self
            .output
            .iter()
            .filter_map(OpenAiResponsesOutputItem::text)
            .collect::<String>();
        if text.is_empty() { None } else { Some(text) }
    }

    fn finish_reason(&self) -> String {
        match self.status.as_deref() {
            Some("completed") | None => "stop".to_string(),
            Some(status) => status.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesOutputItem {
    #[serde(default)]
    content: Vec<OpenAiResponsesOutputContent>,
}

impl OpenAiResponsesOutputItem {
    fn text(&self) -> Option<String> {
        let text = self
            .content
            .iter()
            .filter_map(OpenAiResponsesOutputContent::text)
            .collect::<String>();
        if text.is_empty() { None } else { Some(text) }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesOutputContent {
    #[serde(rename = "type", default)]
    content_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

impl OpenAiResponsesOutputContent {
    fn text(&self) -> Option<String> {
        match self.content_type.as_deref() {
            Some("output_text" | "text") | None => self.text.clone(),
            Some(_) => None,
        }
    }
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
        OpenAiAdapter, OpenAiWireApi,
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
        let adapter = OpenAiAdapter::with_wire_api(
            Arc::new(transport.clone()),
            OpenAiWireApi::ChatCompletions,
        );

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
    async fn responses_api_maps_text_and_usage() {
        let transport = MockTransport::new(vec![Ok(HttpResponse {
            status: 200,
            body: Some(
                json!({
                    "id": "resp_123",
                    "model": "gpt-4.1-mini",
                    "status": "completed",
                    "output": [{
                        "content": [
                            { "type": "output_text", "text": "world" }
                        ]
                    }],
                    "usage": {
                        "input_tokens": 11,
                        "output_tokens": 7,
                        "input_tokens_details": {
                            "cached_tokens": 2
                        }
                    }
                })
                .to_string(),
            ),
        })]);
        let adapter =
            OpenAiAdapter::with_wire_api(Arc::new(transport.clone()), OpenAiWireApi::Responses);

        let response = adapter.execute_chat(&request(), &context()).await.unwrap();

        assert_eq!(response.output_text, "world");
        assert_eq!(response.finish_reason, "stop");
        assert_eq!(response.usage.input_tokens, 11);
        assert_eq!(response.usage.output_tokens, 7);
        assert_eq!(response.usage.cached_input_tokens, 2);

        let (request_url, request_role, content_type) = {
            let recorded_requests = transport.requests.lock().unwrap();
            (
                recorded_requests[0].url.clone(),
                recorded_requests[0].body["input"][0]["role"].clone(),
                recorded_requests[0].body["input"][0]["content"][0]["type"].clone(),
            )
        };
        assert_eq!(request_url, "https://api.openai.example/v1/responses");
        assert_eq!(request_role, "user");
        assert_eq!(content_type, "input_text");
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
