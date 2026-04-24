use super::{
    HttpRequest, HttpResponse, HttpTransport, HttpTransportError, HttpTransportErrorKind,
    OpenAiAdapter, OpenAiWireApi,
};
use async_trait::async_trait;
use provider_traits::{
    ProviderAdapter, ProviderEndpoint, ProviderErrorKind, ProviderExecutionContext,
    ProviderImageRequest, ProviderMessage, ProviderRequest,
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
    async fn post_json(&self, request: HttpRequest) -> Result<HttpResponse, HttpTransportError> {
        self.requests.lock().unwrap().push(request);
        self.responses.lock().unwrap().remove(0)
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
            provider_resource_id: "prvrsrc_openai_primary".to_string(),
            endpoint_base_url: "https://api.openai.example/v1".to_string(),
            api_key: "secret".to_string(),
            region: None,
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

fn image_request() -> ProviderImageRequest {
    ProviderImageRequest {
        model: "chatgpt-image-2".to_string(),
        prompt: "draw a router".to_string(),
        n: Some(1),
        size: Some("1024x1024".to_string()),
        quality: Some("auto".to_string()),
        response_format: Some("b64_json".to_string()),
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
    let adapter =
        OpenAiAdapter::with_wire_api(Arc::new(transport.clone()), OpenAiWireApi::ChatCompletions);

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
async fn image_generation_uses_images_endpoint_and_chatgpt_image_alias() {
    let transport = MockTransport::new(vec![Ok(HttpResponse {
        status: 200,
        body: Some(
            json!({
                "id": "img_123",
                "model": "chatgpt-image-latest",
                "created": 1_777_000_000_u64,
                "data": [{
                    "b64_json": "aW1hZ2U=",
                    "revised_prompt": "Draw a precise network router"
                }],
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 40,
                    "input_tokens_details": {
                        "cached_tokens": 2
                    }
                }
            })
            .to_string(),
        ),
    })]);
    let adapter = OpenAiAdapter::new(Arc::new(transport.clone()));

    let response = adapter
        .execute_image_generation(&image_request(), &context())
        .await
        .unwrap();

    assert_eq!(response.response_id.as_deref(), Some("img_123"));
    assert_eq!(response.model, "chatgpt-image-latest");
    assert_eq!(response.images[0].b64_json.as_deref(), Some("aW1hZ2U="));
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 40);
    assert_eq!(response.usage.cached_input_tokens, 2);

    let (request_url, model, prompt, response_format) = {
        let recorded_requests = transport.requests.lock().unwrap();
        (
            recorded_requests[0].url.clone(),
            recorded_requests[0].body["model"].clone(),
            recorded_requests[0].body["prompt"].clone(),
            recorded_requests[0].body["response_format"].clone(),
        )
    };
    assert_eq!(
        request_url,
        "https://api.openai.example/v1/images/generations"
    );
    assert_eq!(model, "chatgpt-image-latest");
    assert_eq!(prompt, "draw a router");
    assert_eq!(response_format, "b64_json");
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
