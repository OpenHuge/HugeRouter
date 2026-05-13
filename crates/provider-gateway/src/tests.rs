use super::{
    ChatGptWebAdapter, GatewayAdapter, HttpRequest, HttpResponse, HttpTransport,
    HttpTransportError, HttpTransportErrorKind, TRANSIT_HEADER_HOP, TRANSIT_HEADER_ORIGIN,
    TRANSIT_HEADER_PROVIDER, TRANSIT_HEADER_VIA, prepare_transit_headers,
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
    async fn post_json(&self, request: HttpRequest) -> Result<HttpResponse, HttpTransportError> {
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
            region: None,
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

#[test]
fn chatgpt_web_adapter_declares_openai_compatible_image_transit() {
    let adapter = ChatGptWebAdapter::new(Arc::new(MockTransport::default()));
    let manifest = adapter.manifest();

    assert_eq!(manifest.provider_kind, "chatgpt_web");
    assert_eq!(manifest.protocol_family, "openai_images");
    assert!(manifest.supports_protocol_family("openai_images"));
    assert!(manifest.supports_protocol_family("openai_chat"));
}
