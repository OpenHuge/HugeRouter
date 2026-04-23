#![allow(clippy::too_many_lines)]

use async_trait::async_trait;
use protocol_gemini::{
    GenerationConfig, to_provider_response, to_upstream_generate_content_request,
};
use provider_traits::{
    AdapterManifest, ProviderAdapter, ProviderEndpoint, ProviderError, ProviderErrorKind,
    ProviderExecutionContext, ProviderRequest, ProviderResponse, StreamingSupport,
};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 1024;
const DEFAULT_TEMPERATURE: f32 = 1.0;

#[derive(Clone)]
pub struct GeminiAdapter {
    transport: Arc<dyn HttpTransport>,
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new(Arc::new(ReqwestTransport::default()))
    }
}

impl GeminiAdapter {
    #[must_use]
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl ProviderAdapter for GeminiAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            adapter_id: "gemini-native-v1",
            provider_kind: "gemini",
            display_name: "Gemini Generate Content",
            protocol_family: "gemini_generate_content",
            streaming_support: StreamingSupport::Unsupported,
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

        let mut upstream_request =
            to_upstream_generate_content_request(request).map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidRequest,
                    format!("invalid Gemini request: {error}"),
                    false,
                )
                .with_detail(
                    "provider_resource_id",
                    &context.endpoint.provider_resource_id,
                )
            })?;
        upstream_request.generation_config = Some(GenerationConfig {
            max_output_tokens: Some(DEFAULT_MAX_OUTPUT_TOKENS),
            temperature: Some(DEFAULT_TEMPERATURE),
        });

        let request_url = gemini_request_url(&request.model, &context.endpoint);
        let http_request = HttpRequest {
            url: request_url,
            headers: vec![
                (
                    "x-goog-api-key".to_string(),
                    context.endpoint.api_key.clone(),
                ),
                ("content-type".to_string(), "application/json".to_string()),
            ],
            body: serde_json::to_value(&upstream_request).map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::Protocol,
                    format!("failed to serialize Gemini request: {error}"),
                    false,
                )
                .with_detail(
                    "provider_resource_id",
                    &context.endpoint.provider_resource_id,
                )
            })?,
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

        let response_body = response.body.ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::Protocol,
                "Gemini returned an empty response body",
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
            )
        })?;
        let upstream_response: protocol_gemini::GenerateContentResponse =
            serde_json::from_str(&response_body).map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::Protocol,
                    format!("failed to decode Gemini response: {error}"),
                    false,
                )
                .with_detail(
                    "provider_resource_id",
                    &context.endpoint.provider_resource_id,
                )
            })?;

        to_provider_response(&upstream_response, &request.model).ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::Protocol,
                "Gemini response does not contain text content",
                false,
            )
            .with_detail(
                "provider_resource_id",
                &context.endpoint.provider_resource_id,
            )
        })
    }
}

fn gemini_request_url(model: &str, endpoint: &ProviderEndpoint) -> String {
    let base = endpoint.endpoint_base_url.trim_end_matches('/');
    let model = urlencoding(model);
    if base.ends_with("/v1beta/models") {
        format!("{base}/{model}:generateContent")
    } else if base.ends_with("/v1beta") {
        format!("{base}/models/{model}:generateContent")
    } else {
        format!("{base}/v1beta/models/{model}:generateContent")
    }
}

fn urlencoding(model: &str) -> String {
    let mut encoded = String::with_capacity(model.len());
    for byte in model.chars() {
        match byte {
            ' ' => encoded.push_str("%20"),
            '/' => encoded.push_str("%2F"),
            ':' => encoded.push_str("%3A"),
            '?' => encoded.push_str("%3F"),
            '&' => encoded.push_str("%26"),
            '=' => encoded.push_str("%3D"),
            '+' => encoded.push_str("%2B"),
            '%' => encoded.push_str("%25"),
            _ => encoded.push(byte),
        }
    }
    encoded
}

fn map_error_response(
    status: u16,
    body: Option<&str>,
    provider_resource_id: &str,
) -> ProviderError {
    let parsed = body.and_then(|raw| serde_json::from_str::<GeminiErrorResponse>(raw).ok());
    let upstream_code = parsed
        .as_ref()
        .and_then(|error| error.error.as_ref().and_then(|value| value.code.clone()));
    let message = parsed
        .as_ref()
        .and_then(|error| {
            error
                .message
                .as_ref()
                .or_else(|| error.error.as_ref().and_then(|e| e.message.as_ref()))
        })
        .cloned()
        .unwrap_or_else(|| format!("Gemini returned HTTP {status}"));

    let (kind, retryable) = match status {
        401 | 403 => (ProviderErrorKind::Auth, false),
        400 | 404 => (ProviderErrorKind::InvalidRequest, false),
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
struct GeminiErrorResponse {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<GeminiErrorDetail>,
}

#[derive(Debug, serde::Deserialize)]
struct GeminiErrorDetail {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use provider_traits::{ProviderErrorKind, ProviderExecutionContext, ProviderMessage};
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
            request_headers: std::collections::BTreeMap::new(),
            endpoint: ProviderEndpoint {
                provider_resource_id: "prvrsrc_gemini_primary".to_string(),
                endpoint_base_url: "https://generativelanguage.googleapis.com/".to_string(),
                api_key: "secret".to_string(),
            },
        }
    }

    fn request() -> ProviderRequest {
        ProviderRequest {
            model: "gemini-1.5-flash-latest".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "Hello Gemini".to_string(),
            }],
            stream: false,
        }
    }

    #[tokio::test]
    async fn sends_gemini_request_and_extracts_output_and_usage() {
        let transport = MockTransport::new(vec![Ok(HttpResponse {
            status: 200,
            body: Some(
                json!({
                    "responseId": "resp_123",
                    "candidates": [{
                        "content": {
                            "role": "model",
                            "parts": [{ "text": "Hello from Gemini" }],
                        },
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 5,
                        "candidatesTokenCount": 9,
                        "totalTokenCount": 14,
                        "cachedContentTokenCount": 0
                    },
                    "modelVersion": "gemini-1.5-flash-latest"
                })
                .to_string(),
            ),
        })]);
        let adapter = GeminiAdapter::new(Arc::new(transport.clone()));

        let response = adapter.execute_chat(&request(), &context()).await.unwrap();
        let recorded_request = {
            let requests = transport.requests.lock().unwrap();
            assert_eq!(requests.len(), 1);
            requests[0].clone()
        };

        assert_eq!(
            recorded_request.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent"
        );
        assert_eq!(response.response_id.as_deref(), Some("resp_123"));
        assert_eq!(response.output_text, "Hello from Gemini");
        assert_eq!(response.usage.input_tokens, 5);
        assert_eq!(response.usage.output_tokens, 9);
        assert!(
            recorded_request
                .headers
                .iter()
                .any(|(name, value)| name == "x-goog-api-key" && value == "secret")
        );
    }

    #[tokio::test]
    async fn maps_empty_response_text_to_protocol_error() {
        let transport = MockTransport::new(vec![Ok(HttpResponse {
            status: 200,
            body: Some(
                json!({
                    "responseId": "resp_123",
                    "candidates": [{
                        "content": {
                            "role": "model",
                            "parts": [{ "text": "" }],
                        },
                        "finishReason": "STOP"
                    }]
                })
                .to_string(),
            ),
        })]);
        let adapter = GeminiAdapter::new(Arc::new(transport));

        let error = adapter
            .execute_chat(&request(), &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::Protocol);
        assert!(error.message.contains("does not contain text content"));
    }

    #[tokio::test]
    async fn maps_rate_limit_error_to_provider_error() {
        let adapter = GeminiAdapter::new(Arc::new(MockTransport::new(vec![Ok(HttpResponse {
            status: 429,
            body: Some(
                json!({
                    "error": {
                        "message": "rate limit exceeded",
                        "code": "RATE_LIMIT"
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
        let adapter = GeminiAdapter::new(Arc::new(MockTransport::new(vec![Err(
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

    #[tokio::test]
    async fn rejects_streaming_request() {
        let mut request = request();
        request.stream = true;

        let adapter = GeminiAdapter::new(Arc::new(MockTransport::new(Vec::new())));

        let error = adapter
            .execute_chat(&request, &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::InvalidRequest);
    }
}
