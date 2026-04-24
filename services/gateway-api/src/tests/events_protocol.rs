use super::*;

#[tokio::test]
async fn returns_forbidden_when_api_key_scope_does_not_match_active_config() {
    let mut registry = ProviderAdapterRegistry::new();
    registry
        .register(Arc::new(MockAdapter {
            outcomes: BTreeMap::new(),
        }))
        .unwrap();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore {
            scope: GatewayApiKeyScope {
                credential_id: "cred_other".to_string(),
                tenant_id: "tenant_platform".to_string(),
                project_id: Some("proj_core".to_string()),
                status: "active".to_string(),
            },
        }),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "auth_forbidden");
}

#[tokio::test]
async fn allows_tenant_scoped_api_keys_without_project_scope() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_tenant_scoped".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "ok".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 6,
                    output_tokens: 4,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore {
            scope: GatewayApiKeyScope {
                credential_id: "cred_tenant_shared".to_string(),
                tenant_id: "tenant_acme".to_string(),
                project_id: None,
                status: "active".to_string(),
            },
        }),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn publishes_route_receipt_event_for_successful_request() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_success".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "ok".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 9,
                    output_tokens: 6,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let sink = Arc::new(RecordingRuntimeEventSink::default());
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: sink.clone(),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let (receipt_count, selected_target, provider_attempt_count, first_attempt_status, first_stage) = {
        let receipts = sink.published_route_receipts.lock().await;
        (
            receipts.len(),
            receipts[0]
                .route_receipt
                .selected_target
                .as_ref()
                .unwrap()
                .as_str()
                .to_string(),
            receipts[0].provider_attempts.len(),
            receipts[0].provider_attempts[0].status.clone(),
            receipts[0].decision_timeline[0].stage.clone(),
        )
    };
    assert_eq!(receipt_count, 1);
    assert_eq!(selected_target, "prvrsrc_openai_primary");
    assert_eq!(provider_attempt_count, 1);
    assert_eq!(first_attempt_status, "succeeded");
    assert_eq!(first_stage, "admission");

    let usage_event_count = {
        let usage_events = sink.published_usage_events.lock().await;
        usage_events.len()
    };
    assert_eq!(usage_event_count, 1);
}

#[tokio::test]
async fn publishes_route_receipt_event_for_failed_request() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Err(ProviderError::new(
                ProviderErrorKind::Unavailable,
                "provider down",
                false,
            )),
        )]),
    });
    let sink = Arc::new(RecordingRuntimeEventSink::default());
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: sink.clone(),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let (receipt_count, normalized_error_code, provider_attempt_count, first_attempt_status) = {
        let receipts = sink.published_route_receipts.lock().await;
        (
            receipts.len(),
            receipts[0]
                .route_receipt
                .normalized_error
                .as_ref()
                .unwrap()
                .code
                .clone(),
            receipts[0].provider_attempts.len(),
            receipts[0].provider_attempts[0].status.clone(),
        )
    };
    assert_eq!(receipt_count, 1);
    assert_eq!(normalized_error_code, "provider_unavailable");
    assert_eq!(provider_attempt_count, 1);
    assert_eq!(first_attempt_status, "failed");
}

#[tokio::test]
async fn returns_service_unavailable_when_route_receipt_publish_fails() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_123".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "ok".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 8,
                    output_tokens: 5,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink {
            route_receipt_publish_error: Some("nats unavailable".to_string()),
            ..RecordingRuntimeEventSink::default()
        }),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "route_receipt_publish_failed");
}

#[tokio::test]
async fn returns_service_unavailable_when_usage_event_publish_fails() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_123".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "ok".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 8,
                    output_tokens: 5,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink {
            usage_event_publish_error: Some("nats unavailable".to_string()),
            published_usage_events: Arc::new(Mutex::new(Vec::new())),
            ..RecordingRuntimeEventSink::default()
        }),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "usage_event_publish_failed");
}

#[tokio::test]
async fn anthropic_messages_route_returns_vendor_shaped_response() {
    let adapter = Arc::new(ProtocolMockAdapter {
        provider_kind: "anthropic",
        protocol_family: "anthropic_messages",
        outcomes: BTreeMap::from([(
            "prvrsrc_anthropic_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("msg_123".to_string()),
                model: "claude-3-opus".to_string(),
                output_text: "anthropic summary".to_string(),
                finish_reason: "end_turn".to_string(),
                usage: ProviderUsage {
                    input_tokens: 10,
                    output_tokens: 6,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let mut config = build_config(vec![build_target_with_provider(
        "prvrsrc_anthropic_primary",
        "anthropic",
        "us-east-1",
        0.9,
        0.6,
        HealthState::Healthy,
    )]);
    config.route_policy.protocol_family = "anthropic_messages".to_string();
    config.route_policy.model_alias = "claude-3-opus".to_string();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(config)),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/messages")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "claude-3-opus",
                        "messages": [{
                            "role": "user",
                            "content": [{"type": "text", "text": "hello"}]
                        }],
                        "max_tokens": 256,
                        "stream": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["id"], "msg_123");
    assert_eq!(payload["content"][0]["text"], "anthropic summary");
    assert_eq!(payload["usage"]["input_tokens"], 10);
}

#[tokio::test]
async fn gemini_generate_content_route_returns_vendor_shaped_response() {
    let adapter = Arc::new(ProtocolMockAdapter {
        provider_kind: "gemini",
        protocol_family: "gemini_generate_content",
        outcomes: BTreeMap::from([(
            "prvrsrc_gemini_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("resp_gemini_123".to_string()),
                model: "gemini-1.5-pro-latest".to_string(),
                output_text: "gemini summary".to_string(),
                finish_reason: "STOP".to_string(),
                usage: ProviderUsage {
                    input_tokens: 14,
                    output_tokens: 8,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();
    let mut config = build_config(vec![build_target_with_provider(
        "prvrsrc_gemini_primary",
        "gemini",
        "us-east-1",
        0.9,
        0.6,
        HealthState::Healthy,
    )]);
    config.route_policy.protocol_family = "gemini_generate_content".to_string();
    config.route_policy.model_alias = "gemini-1.5-pro".to_string();
    let state = Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(config)),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    });
    let app = app_with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1beta/models/gemini-1.5-pro:generateContent")
                .header("authorization", "Bearer test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "placeholder",
                        "contents": [{
                            "role": "user",
                            "parts": [{"text": "hello"}]
                        }],
                        "stream": false,
                        "tools": []
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["responseId"], "resp_gemini_123");
    assert_eq!(
        payload["candidates"][0]["content"]["parts"][0]["text"],
        "gemini summary"
    );
    assert_eq!(payload["usageMetadata"]["totalTokenCount"], 22);
}

#[tokio::test]
async fn returns_provider_failure_when_last_candidate_fails() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Err(
                ProviderError::new(ProviderErrorKind::Unavailable, "openai unavailable", false)
                    .with_upstream_status(Some(503))
                    .with_upstream_code(Some("server_error".to_string())),
            ),
        )]),
    });
    let app = app_with_state(test_state(
        adapter,
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
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "provider_unavailable");
    assert_eq!(payload["error"]["upstream_status_code"], 503);
}
