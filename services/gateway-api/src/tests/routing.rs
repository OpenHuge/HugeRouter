use super::*;

#[test]
fn rejects_streaming_requests_during_normalization() {
    let error = normalize_request(
        ChatCompletionRequest {
            stream: true,
            ..valid_http_request()
        },
        &request_context(),
    )
    .unwrap_err();

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.envelope.error.validation_issues[0].field, "stream");
}

#[test]
fn default_registry_registers_bedrock_adapter() {
    let registry = composition::provider_registry_from_config(
        &composition::ProviderRegistryConfig::all_enabled(),
    )
    .expect("default provider catalog should compose");

    assert!(registry.resolve("bedrock").is_some());
}

#[test]
fn route_scoring_prefers_preferred_region() {
    let config = build_config(vec![
        build_target("prvrsrc_west", "us-west-2", 0.95, 0.8, HealthState::Healthy),
        build_target(
            "prvrsrc_east",
            "us-east-1",
            0.85,
            0.75,
            HealthState::Healthy,
        ),
    ]);
    let route = evaluate_route(
        &config,
        &normalize_request(valid_http_request(), &request_context()).unwrap(),
        &request_context(),
    );

    assert_eq!(
        route.ranked_targets[0]
            .target
            .resource
            .provider_resource_id
            .as_str(),
        "prvrsrc_east"
    );
}

#[test]
fn route_scoring_can_target_transit_provider() {
    let mut config = build_config(vec![
        build_target(
            "prvrsrc_openai_native",
            "us-east-1",
            0.88,
            0.45,
            HealthState::Healthy,
        ),
        build_gateway_target("prvrsrc_gateway_primary"),
    ]);
    config.route_policy.required_capabilities =
        vec!["json_mode".to_string(), "transit_gateway".to_string()];

    let route = evaluate_route(
        &config,
        &normalize_request(valid_http_request(), &request_context()).unwrap(),
        &request_context(),
    );

    assert_eq!(route.ranked_targets.len(), 1);
    assert_eq!(
        route.ranked_targets[0]
            .target
            .resource
            .provider_resource_id
            .as_str(),
        "prvrsrc_gateway_primary"
    );
    assert!(route.ranked_targets[0].target.transit_metadata.is_some());
}
