use super::*;

#[tokio::test]
async fn falls_back_to_second_target_when_first_is_retryable_failure() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([
            (
                "prvrsrc_openai_primary".to_string(),
                Err(ProviderError::new(
                    ProviderErrorKind::Unavailable,
                    "primary is degraded",
                    true,
                )
                .with_upstream_status(Some(503))),
            ),
            (
                "prvrsrc_openai_backup".to_string(),
                Ok(ProviderResponse {
                    response_id: Some("chatcmpl_123".to_string()),
                    model: "gpt-4.1-mini".to_string(),
                    output_text: "fallback success".to_string(),
                    finish_reason: "stop".to_string(),
                    usage: ProviderUsage {
                        input_tokens: 12,
                        output_tokens: 9,
                        cached_input_tokens: 0,
                    },
                }),
            ),
        ]),
    });
    let app = app_with_state(test_state(
        adapter,
        vec![
            build_target(
                "prvrsrc_openai_primary",
                "us-east-1",
                0.95,
                0.8,
                HealthState::Healthy,
            ),
            build_target(
                "prvrsrc_openai_backup",
                "us-east-1",
                0.85,
                0.75,
                HealthState::Healthy,
            ),
        ],
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

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("x-route-receipt-id").is_some());
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["choices"][0]["message"]["content"],
        "fallback success"
    );
}

#[tokio::test]
async fn falls_back_when_first_target_has_no_registered_adapter() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([(
            "prvrsrc_openai_backup".to_string(),
            Ok(ProviderResponse {
                response_id: Some("chatcmpl_456".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "adapter fallback success".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 11,
                    output_tokens: 7,
                    cached_input_tokens: 0,
                },
            }),
        )]),
    });
    let app = app_with_state(test_state(
        adapter,
        vec![
            build_target_with_provider(
                "prvrsrc_anthropic_primary",
                "anthropic",
                "us-east-1",
                0.95,
                0.8,
                HealthState::Healthy,
            ),
            build_target(
                "prvrsrc_openai_backup",
                "us-east-1",
                0.85,
                0.75,
                HealthState::Healthy,
            ),
        ],
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

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["choices"][0]["message"]["content"],
        "adapter fallback success"
    );
}

#[tokio::test]
async fn emits_documented_debug_headers_when_enabled() {
    let adapter = Arc::new(MockAdapter {
        outcomes: BTreeMap::from([
            (
                "prvrsrc_openai_primary".to_string(),
                Err(ProviderError::new(
                    ProviderErrorKind::Unavailable,
                    "primary is degraded",
                    true,
                )),
            ),
            (
                "prvrsrc_openai_backup".to_string(),
                Ok(ProviderResponse {
                    response_id: Some("chatcmpl_789".to_string()),
                    model: "gpt-4.1-mini".to_string(),
                    output_text: "debug headers success".to_string(),
                    finish_reason: "stop".to_string(),
                    usage: ProviderUsage {
                        input_tokens: 12,
                        output_tokens: 8,
                        cached_input_tokens: 0,
                    },
                }),
            ),
        ]),
    });
    let app = app_with_state(test_state_with_debug(
        adapter,
        vec![
            build_target(
                "prvrsrc_openai_primary",
                "us-east-1",
                0.95,
                0.8,
                HealthState::Healthy,
            ),
            build_target(
                "prvrsrc_openai_backup",
                "us-east-1",
                0.85,
                0.75,
                HealthState::Healthy,
            ),
        ],
        true,
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

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-debug-selected-target").unwrap(),
        "prvrsrc_openai_backup"
    );
    assert_eq!(
        response.headers().get("x-debug-route-policy-id").unwrap(),
        "routepol_default"
    );
    assert_eq!(
        response.headers().get("x-debug-fallback-count").unwrap(),
        "1"
    );
    assert_eq!(
        response.headers().get("x-debug-admission-result").unwrap(),
        "admitted"
    );
}

#[tokio::test]
async fn control_plane_config_store_fetches_and_maps_active_config() {
    let (base_url, request_count, handle) = spawn_control_plane_server(false).await;
    let store = ControlPlaneConfigStore::new(
        base_url,
        "active",
        None,
        Duration::from_secs(60),
        reqwest::Client::new(),
    );

    let config = store.load().await.unwrap();

    assert_eq!(
        config.config_snapshot.config_snapshot_id.as_str(),
        "cfgsnap_test"
    );
    assert_eq!(
        config.route_policy.route_policy_id.as_str(),
        "routepol_default"
    );
    assert_eq!(config.provider_targets.len(), 2);
    assert_eq!(request_count.load(AtomicOrdering::Relaxed), 1);

    handle.abort();
}

#[tokio::test]
async fn control_plane_config_store_uses_ttl_cache() {
    let (base_url, request_count, handle) = spawn_control_plane_server(false).await;
    let store = ControlPlaneConfigStore::new(
        base_url,
        "active",
        None,
        Duration::from_secs(60),
        reqwest::Client::new(),
    );

    let _ = store.load().await.unwrap();
    let _ = store.load().await.unwrap();

    assert_eq!(request_count.load(AtomicOrdering::Relaxed), 1);
    handle.abort();
}

#[tokio::test]
async fn control_plane_config_store_reports_unavailable_backend() {
    let (base_url, _request_count, handle) = spawn_control_plane_server(true).await;
    let store = ControlPlaneConfigStore::new(
        base_url,
        "active",
        None,
        Duration::from_millis(1),
        reqwest::Client::new(),
    );

    let error = store.load().await.unwrap_err();

    assert!(error.contains("HTTP 503"));
    handle.abort();
}

#[tokio::test]
async fn control_plane_api_key_store_resolves_and_caches_scope() {
    let (base_url, request_count, handle) = spawn_control_plane_server(false).await;
    let store = ControlPlaneApiKeyStore::new(
        base_url,
        "/internal/gateway/api-keys/resolve",
        Some("dev-internal-token".to_string()),
        reqwest::Client::new(),
    );

    let first = store.resolve("test").await.unwrap();
    let second = store.resolve("test").await.unwrap();

    assert_eq!(first.credential_id, "cred_gateway_test");
    assert_eq!(second.project_id.as_deref(), Some("proj_core"));
    assert_eq!(request_count.load(AtomicOrdering::Relaxed), 2);

    handle.abort();
}

#[tokio::test]
async fn control_plane_api_key_store_fails_when_internal_token_is_missing() {
    let (base_url, _request_count, handle) = spawn_control_plane_server(false).await;
    let store = ControlPlaneApiKeyStore::new(
        base_url,
        "/internal/gateway/api-keys/resolve",
        None,
        reqwest::Client::new(),
    );

    let error = store.resolve("test").await.unwrap_err();

    assert!(error.contains("CONTROL_PLANE_INTERNAL_TOKEN must be configured"));
    handle.abort();
}
