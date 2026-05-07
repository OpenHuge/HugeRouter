use super::*;

fn opening_grant_success_adapter() -> Arc<MockAdapter> {
    Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_opening_grant".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "ok".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 11,
                    output_tokens: 7,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    })
}

fn opening_grant_scope() -> GatewayApiKeyScope {
    GatewayApiKeyScope {
        credential_id: "cred_opening_grant_customer".to_string(),
        grant_id: Some("grant_acme_customer".to_string()),
        owner_account_id: Some("acct_acme_owner".to_string()),
        tenant_id: "tenant_acme".to_string(),
        project_id: Some("proj_core".to_string()),
        status: "active".to_string(),
        config_snapshot_id: Some("cfgsnap_test".to_string()),
        route_policy_id: Some("routepol_default".to_string()),
        scopes: vec![
            "route:codex".to_string(),
            "provider:hugerouter-commercial".to_string(),
            "protocol:openai_chat".to_string(),
            "model:reasoning-fast".to_string(),
        ],
    }
}

fn opening_grant_state_with_sink(sink: Arc<RecordingRuntimeEventSink>) -> GatewayState {
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(opening_grant_success_adapter()).unwrap();
    Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(vec![build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        )]))),
        auth_store: Arc::new(StaticApiKeyScopeStore {
            scope: opening_grant_scope(),
        }),
        route_token_store: Arc::new(super::InMemoryRouteTokenStore::default()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: sink,
    })
}

async fn single_route_receipt_snapshot(sink: &RecordingRuntimeEventSink) -> RouteReceiptRecorded {
    let receipts = { sink.published_route_receipts.lock().await.clone() };
    assert_eq!(receipts.len(), 1);
    receipts.into_iter().next().unwrap()
}

async fn single_usage_event_snapshot(sink: &RecordingRuntimeEventSink) -> UsageEvent {
    let usage_events = { sink.published_usage_events.lock().await.clone() };
    assert_eq!(usage_events.len(), 1);
    usage_events.into_iter().next().unwrap()
}

#[tokio::test]
async fn opening_grant_scope_can_call_chat_and_publish_usage_witness() {
    let sink = Arc::new(RecordingRuntimeEventSink::default());
    let app = app_with_state(opening_grant_state_with_sink(sink.clone()));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer akp_customer_opening")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let route_receipt_id = response
        .headers()
        .get("x-route-receipt-id")
        .and_then(|value| value.to_str().ok())
        .expect("routed customer request should expose route receipt id")
        .to_string();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["choices"][0]["message"]["content"], "ok");

    let receipt = single_route_receipt_snapshot(sink.as_ref()).await;
    assert_eq!(
        receipt
            .route_receipt
            .selected_target
            .as_ref()
            .unwrap()
            .as_str(),
        "prvrsrc_openai_primary"
    );
    let recorded_receipt_id = receipt.route_receipt.route_receipt_id.to_string();
    assert_eq!(recorded_receipt_id, route_receipt_id);

    let usage_event = single_usage_event_snapshot(sink.as_ref()).await;
    assert_eq!(usage_event.grant_id.as_deref(), Some("grant_acme_customer"));
    assert_eq!(
        usage_event.owner_account_id.as_deref(),
        Some("acct_acme_owner")
    );
    assert_eq!(usage_event.usage.output_tokens, 7);
    let usage_route_receipt_id = usage_event.route_receipt_id.to_string();
    assert_eq!(usage_route_receipt_id, route_receipt_id);
    let route_receipt_suffix = route_receipt_id
        .strip_prefix("routercpt_")
        .expect("route receipt id should use routercpt prefix");
    assert_eq!(
        usage_event.usage_event_id.to_string(),
        format!("usageevt_{route_receipt_suffix}")
    );
    assert_eq!(
        usage_event.idempotency_key,
        format!("{route_receipt_id}:final")
    );
}

#[tokio::test]
async fn opening_grant_scope_uses_owner_budget_scope() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_opening_budget".to_string()),
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
    let budget_scopes = Arc::new(Mutex::new(Vec::new()));
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
                credential_id: "cred_opening_grant_budget".to_string(),
                grant_id: Some("grant_acme_budget".to_string()),
                owner_account_id: Some("acct_acme_owner".to_string()),
                tenant_id: "tenant_acme".to_string(),
                project_id: Some("proj_core".to_string()),
                status: "active".to_string(),
                config_snapshot_id: Some("cfgsnap_test".to_string()),
                route_policy_id: Some("routepol_default".to_string()),
                scopes: vec![
                    "route:codex".to_string(),
                    "provider:hugerouter-commercial".to_string(),
                ],
            },
        }),
        route_token_store: Arc::new(super::InMemoryRouteTokenStore::default()),
        budget_store: Arc::new(RecordingBudgetProjectionStore {
            response: ok_budget_projection(),
            scopes: budget_scopes.clone(),
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
                .header("authorization", "Bearer akp_customer_opening")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let scopes = { budget_scopes.lock().await.clone() };
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].tenant, "tenant_acme");
    assert_eq!(scopes[0].project.as_deref(), Some("proj_core"));
    assert_eq!(scopes[0].owner_account.as_deref(), Some("acct_acme_owner"));
}

#[tokio::test]
async fn project_api_key_scope_uses_default_budget_owner_scope() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_primary".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_project_budget".to_string()),
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
    let budget_scopes = Arc::new(Mutex::new(Vec::new()));
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
                credential_id: "cred_project_default_budget".to_string(),
                grant_id: None,
                owner_account_id: None,
                tenant_id: "tenant_acme".to_string(),
                project_id: Some("proj_core".to_string()),
                status: "active".to_string(),
                config_snapshot_id: None,
                route_policy_id: None,
                scopes: Vec::new(),
            },
        }),
        route_token_store: Arc::new(super::InMemoryRouteTokenStore::default()),
        budget_store: Arc::new(RecordingBudgetProjectionStore {
            response: ok_budget_projection(),
            scopes: budget_scopes.clone(),
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
                .header("authorization", "Bearer akp_project_default_budget")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&valid_http_request()).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let scopes = { budget_scopes.lock().await.clone() };
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].tenant, "tenant_acme");
    assert_eq!(scopes[0].project.as_deref(), Some("proj_core"));
    assert_eq!(
        scopes[0].owner_account.as_deref(),
        Some(DEFAULT_OWNER_ACCOUNT_ID)
    );
}

#[tokio::test]
async fn opening_grant_scope_rejects_inactive_config_snapshot() {
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
                credential_id: "cred_opening_grant_stale".to_string(),
                grant_id: Some("grant_acme_stale".to_string()),
                owner_account_id: Some("acct_acme_owner".to_string()),
                tenant_id: "tenant_acme".to_string(),
                project_id: Some("proj_core".to_string()),
                status: "active".to_string(),
                config_snapshot_id: Some("cfgsnap_superseded".to_string()),
                route_policy_id: Some("routepol_default".to_string()),
                scopes: vec![
                    "route:codex".to_string(),
                    "provider:hugerouter-commercial".to_string(),
                ],
            },
        }),
        route_token_store: Arc::new(super::InMemoryRouteTokenStore::default()),
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
                .header("authorization", "Bearer akp_customer_opening")
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
