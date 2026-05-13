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

#[test]
fn route_evaluation_excludes_targets_that_do_not_support_request_protocol() {
    let mut unsupported_target = build_target(
        "prvrsrc_chat_only",
        "us-east-1",
        0.95,
        0.8,
        HealthState::Healthy,
    );
    unsupported_target.resource.supported_protocol_families = vec!["openai_chat".to_string()];
    let mut config = build_config(vec![unsupported_target]);
    config.route_policy.protocol_family = "openai_responses".to_string();
    config.route_policy.required_capabilities = vec![];
    let request = normalize_responses_request(valid_responses_request(), &request_context())
        .expect("responses request should normalize");

    let route = evaluate_route(&config, &request, &request_context());

    assert!(route.ranked_targets.is_empty());
    assert_eq!(route.excluded_targets.len(), 1);
    assert_eq!(
        route.excluded_targets[0].reason_code,
        "protocol_family_unsupported"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn gateway_route_evaluation_matches_shared_engine_for_explainability_fixture() {
    let mut healthy_target = build_target(
        "prvrsrc_healthy_json",
        "us-east-1",
        0.91,
        0.68,
        HealthState::Healthy,
    );
    healthy_target.priority = 1;
    let mut degraded_target = build_target(
        "prvrsrc_degraded_tooling",
        "us-west-2",
        0.86,
        0.72,
        HealthState::Degraded,
    );
    degraded_target.priority = 2;
    let quarantined_target = build_target(
        "prvrsrc_quarantined",
        "us-east-1",
        0.99,
        0.99,
        HealthState::Quarantined,
    );
    let draining_target = build_target(
        "prvrsrc_draining",
        "us-east-1",
        0.88,
        0.88,
        HealthState::Draining,
    );
    let disabled_target = build_target(
        "prvrsrc_disabled",
        "us-east-1",
        0.88,
        0.88,
        HealthState::Disabled,
    );
    let mut capability_gap_target = build_target(
        "prvrsrc_no_tools",
        "us-east-1",
        0.9,
        0.7,
        HealthState::Healthy,
    );
    capability_gap_target
        .resource
        .capabilities
        .supports_tool_calling = false;
    let mut protocol_gap_target = build_target(
        "prvrsrc_responses_only",
        "us-east-1",
        0.9,
        0.7,
        HealthState::Healthy,
    );
    protocol_gap_target.resource.supported_protocol_families = vec!["openai_responses".to_string()];

    let mut config = build_config(vec![
        healthy_target,
        degraded_target,
        quarantined_target,
        draining_target,
        disabled_target,
        capability_gap_target,
        protocol_gap_target,
    ]);
    config.route_policy.required_capabilities =
        vec!["json_mode".to_string(), "tool_calling".to_string()];

    let request = normalize_request(valid_http_request(), &request_context()).unwrap();
    let gateway_route = evaluate_route(&config, &request, &request_context());
    let shared_route = routing_engine::evaluate_route(
        &routing_engine::RoutingConfig {
            config_snapshot: config.config_snapshot.clone(),
            route_policy: config.route_policy.clone(),
            provider_targets: config.provider_targets,
        },
        &routing_engine::RoutingRequest {
            protocol_family: request.protocol_family,
            model_alias: request.model_alias,
        },
    );

    assert_eq!(
        gateway_route.admission_result,
        shared_route.admission_result
    );
    assert_eq!(
        gateway_route
            .ranked_targets
            .iter()
            .map(|target| (
                target
                    .target
                    .resource
                    .provider_resource_id
                    .as_str()
                    .to_string(),
                target.score_breakdown.clone(),
            ))
            .collect::<Vec<_>>(),
        shared_route
            .ranked_targets
            .iter()
            .map(|target| (
                target
                    .target
                    .resource
                    .provider_resource_id
                    .as_str()
                    .to_string(),
                target.score_breakdown.clone(),
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        gateway_route
            .excluded_targets
            .iter()
            .map(|target| (
                target.provider_resource_id.as_str().to_string(),
                target.reason_code.clone(),
            ))
            .collect::<Vec<_>>(),
        shared_route
            .excluded_targets
            .iter()
            .map(|target| (
                target.provider_resource_id.as_str().to_string(),
                target.reason_code.clone(),
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        gateway_route.ranked_targets[0]
            .target
            .resource
            .provider_resource_id
            .as_str(),
        "prvrsrc_healthy_json"
    );
    assert_eq!(
        gateway_route
            .excluded_targets
            .iter()
            .map(|target| target.reason_code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "health_quarantined",
            "health_draining",
            "health_disabled",
            "capability_gap",
            "protocol_family_unsupported",
        ]
    );
}
