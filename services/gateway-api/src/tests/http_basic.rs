use super::*;

#[tokio::test]
async fn healthz_returns_ok() {
    let app = app_with_state(test_state(
        Arc::new(MockAdapter {
            outcomes: BTreeMap::new(),
        }),
        vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )],
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn internal_provider_adapters_exposes_runtime_manifests() {
    let app = app_with_state(test_state(
        Arc::new(MockAdapter {
            outcomes: BTreeMap::new(),
        }),
        vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )],
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/internal/provider-adapters")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["adapters"][0]["adapter_id"], "mock-openai");
    assert_eq!(payload["adapters"][0]["lifecycle_family"], "inference");
    assert_eq!(
        payload["adapters"][0]["supported_protocol_families"][1],
        "openai_responses"
    );
}

#[tokio::test]
async fn rejects_missing_auth_with_normalized_error() {
    let app = app_with_state(test_state(
        Arc::new(MockAdapter {
            outcomes: BTreeMap::new(),
        }),
        vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )],
    ));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "auth_invalid");
}

#[tokio::test]
async fn returns_validation_failure_for_bad_request_shape() {
    let app = app_with_state(test_state(
        Arc::new(MockAdapter {
            outcomes: BTreeMap::new(),
        }),
        vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )],
    ));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "",
                        "messages": [],
                        "stream": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "request_validation_failed");
}

#[tokio::test]
async fn rejects_request_when_budget_is_exceeded_before_upstream_call() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("resp_should_not_happen".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "unexpected".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let app = app_with_state(Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: BalanceProjectionResponse {
                data: protocol_ir::BalanceProjection {
                    threshold_status: "exceeded".to_string(),
                    remaining_budget: MonetaryAmount {
                        currency: "USD".to_string(),
                        amount: "-1.00".to_string(),
                    },
                    ..ok_budget_projection().data
                },
            },
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    }));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_responses_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "budget_exceeded");
}

#[tokio::test]
async fn responses_route_returns_response_api_shape() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("resp_test_123".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "hello from responses".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 12,
                    output_tokens: 8,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let mut config = build_config(vec![build_target(
        "prvrsrc_openai_primary",
        "us-east-1",
        0.9,
        0.6,
        HealthState::Healthy,
    )]);
    config.route_policy.protocol_family = "openai_responses".to_string();
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let app = app_with_state(Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(config)),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    }));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_responses_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["object"], "response");
    assert_eq!(payload["output_text"], "hello from responses");
    assert_eq!(payload["output"][0]["type"], "message");
    assert_eq!(payload["output"][0]["content"][0]["type"], "output_text");
}

#[tokio::test]
async fn gateway_target_routes_successfully_through_provider_chain() {
    let transport = Arc::new(MockGatewayTransport {
        requests: Arc::new(Mutex::new(Vec::new())),
        response: Mutex::new(Ok(GatewayHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: Some(
                serde_json::json!({
                    "id": "chatcmpl_gateway_ok",
                    "model": "gpt-4.1-mini",
                    "choices": [{
                        "message": {"content": "transit success"},
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 10,
                        "completion_tokens": 6,
                        "prompt_tokens_details": {"cached_tokens": 1}
                    }
                })
                .to_string(),
            ),
        })),
    });
    let app = app_with_state(test_state(
        Arc::new(GatewayAdapter::new(transport.clone())),
        vec![build_gateway_target("prvrsrc_gateway_primary")],
    ));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .header("x-forwarded-proto", "https")
                .header("host", "router.example.com")
                .header("idempotency-key", "idem-route-1")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["choices"][0]["message"]["content"],
        "transit success"
    );

    let outbound_headers = {
        let outbound_requests = transport.requests.lock().await;
        outbound_requests[0]
            .headers
            .iter()
            .cloned()
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(
        outbound_headers.get("idempotency-key"),
        Some(&"idem-route-1".to_string())
    );
    assert_eq!(
        outbound_headers.get("x-hugerouter-transit-via"),
        Some(&"gateway-api".to_string())
    );
}

#[tokio::test]
async fn transit_loop_prevention_returns_normalized_error() {
    let transport = Arc::new(MockGatewayTransport {
        requests: Arc::new(Mutex::new(Vec::new())),
        response: Mutex::new(Ok(GatewayHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: Some("{}".to_string()),
        })),
    });
    let app = app_with_state(test_state(
        Arc::new(GatewayAdapter::new(transport.clone())),
        vec![build_gateway_target("prvrsrc_gateway_primary")],
    ));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .header("x-forwarded-proto", "https")
                .header("host", "router.example.com")
                .header("x-hugerouter-transit-hop", "1")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "transit_loop_detected");
    assert_eq!(
        payload["error"]["details"]["loop_guard"],
        "transit_header_present"
    );
    assert!(transport.requests.lock().await.is_empty());
}
