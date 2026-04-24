use std::sync::{Arc, RwLock};

use core_domain::{
    AuthKind, BudgetPolicyId, ConfigSnapshot, ConfigSnapshotId, ConfigSnapshotStatus, CredentialId,
    CredentialOwnerType, DeploymentScope, HealthState, ProjectId, ProvenanceClass,
    ProviderCapabilities, ProviderResource, ProviderResourceId, ProviderResourceStatus,
    RoutePolicy, RoutePolicyId, TenantId,
};
use protocol_ir::{ProtocolFamily, RouteSimulationRequest};

use super::{
    MemoryStore, RoutingConfig, RoutingRequest, SeedData, StoreMode, protocol_family_slug,
    provider_resource_to_route_target,
};

fn parity_provider_resource(
    provider_resource_id: &str,
    health_state: HealthState,
    supports_tool_calling: bool,
    supported_protocol_families: Vec<&str>,
    is_transit_gateway: bool,
) -> ProviderResource {
    ProviderResource {
        provider_resource_id: ProviderResourceId::parse(provider_resource_id).unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: Some(ProjectId::parse("proj_core").unwrap()),
        provider_id: if is_transit_gateway {
            "huge-router".to_string()
        } else {
            "openai".to_string()
        },
        name: provider_resource_id.to_string(),
        status: ProviderResourceStatus::Active,
        provenance_class: ProvenanceClass::OfficialApi,
        credential_owner_type: CredentialOwnerType::Platform,
        deployment_scope: DeploymentScope::Shared,
        region: "us-east-1".to_string(),
        endpoint_base_url: "https://api.openai.com/v1".to_string(),
        auth_kind: AuthKind::ApiKey,
        health_state,
        health_message: Some("parity fixture health".to_string()),
        quarantine_reason: None,
        budget_policy_id: None,
        capabilities: ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling,
            supports_json_mode: true,
            supports_realtime: false,
            supports_response_model_metadata: true,
        },
        supported_protocol_families: supported_protocol_families
            .into_iter()
            .map(str::to_string)
            .collect(),
        is_transit_gateway,
        version: 1,
        created_at: "2026-04-22T00:00:00Z".to_string(),
        updated_at: "2026-04-22T00:00:00Z".to_string(),
    }
}

fn parity_route_policy() -> RoutePolicy {
    RoutePolicy {
        route_policy_id: RoutePolicyId::parse("routepol_cp_parity").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        display_name: "Control-plane parity policy".to_string(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: vec![
            "json_mode".to_string(),
            "tool_calling".to_string(),
            "transit_gateway".to_string(),
        ],
        preferred_regions: vec!["us-east-1".to_string()],
        version: 1,
        created_at: "2026-04-22T00:00:00Z".to_string(),
        updated_at: "2026-04-22T00:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn route_simulation_matches_shared_engine_for_explainability_fixture() {
    let route_policy = parity_route_policy();
    let provider_resources = vec![
        parity_provider_resource(
            "prvrsrc_transit_selected",
            HealthState::Healthy,
            true,
            vec!["openai_chat"],
            true,
        ),
        parity_provider_resource(
            "prvrsrc_native_capability_gap",
            HealthState::Healthy,
            true,
            vec!["openai_chat"],
            false,
        ),
        parity_provider_resource(
            "prvrsrc_no_tooling",
            HealthState::Healthy,
            false,
            vec!["openai_chat"],
            true,
        ),
        parity_provider_resource(
            "prvrsrc_quarantined",
            HealthState::Quarantined,
            true,
            vec!["openai_chat"],
            true,
        ),
        parity_provider_resource(
            "prvrsrc_draining",
            HealthState::Draining,
            true,
            vec!["openai_chat"],
            true,
        ),
        parity_provider_resource(
            "prvrsrc_disabled",
            HealthState::Disabled,
            true,
            vec!["openai_chat"],
            true,
        ),
        parity_provider_resource(
            "prvrsrc_responses_only",
            HealthState::Healthy,
            true,
            vec!["openai_responses"],
            true,
        ),
    ];
    let active_snapshot = ConfigSnapshot {
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_cp_parity").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        revision: 1,
        status: ConfigSnapshotStatus::Active,
        activated_at: Some("2026-04-22T00:00:00Z".to_string()),
        provider_resource_ids: provider_resources
            .iter()
            .map(|resource| resource.provider_resource_id.clone())
            .collect(),
        route_policy_id: route_policy.route_policy_id.clone(),
        budget_policy_id: BudgetPolicyId::parse("budgetpol_default").unwrap(),
    };
    let mut seed = SeedData::bootstrap();
    seed.provider_resources = provider_resources.clone();
    seed.route_policies = vec![route_policy.clone()];
    seed.config_snapshots = vec![active_snapshot.clone()];
    seed.active_config_snapshot_id = active_snapshot.config_snapshot_id.to_string();
    let store = StoreMode::Memory(Arc::new(RwLock::new(MemoryStore::from_seed(seed))));
    let request = RouteSimulationRequest {
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        credential_scope: CredentialId::parse("cred_demo").unwrap(),
        protocol_family: ProtocolFamily::OpenAiChat,
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: route_policy.required_capabilities.clone(),
        region: "us-east-1".to_string(),
        expected_prompt_tokens: 64,
        expected_max_output_tokens: 128,
        traffic_class: "interactive".to_string(),
    };

    let simulation = store.simulate_route(request.clone()).await.unwrap();
    let direct_route = routing_engine::evaluate_route(
        &RoutingConfig {
            config_snapshot: active_snapshot,
            route_policy,
            provider_targets: provider_resources
                .iter()
                .enumerate()
                .map(|(index, resource)| provider_resource_to_route_target(index, resource))
                .collect(),
        },
        &RoutingRequest {
            protocol_family: protocol_family_slug(&request.protocol_family).to_string(),
            model_alias: request.model_alias,
        },
    );

    assert_eq!(simulation.admission_result, direct_route.admission_result);
    assert_eq!(
        simulation.selected_target.as_ref().unwrap().as_str(),
        direct_route.ranked_targets[0]
            .target
            .resource
            .provider_resource_id
            .as_str()
    );
    assert_eq!(
        simulation
            .eligible_candidates
            .iter()
            .map(|candidate| (
                candidate.provider_resource_id.as_str().to_string(),
                candidate.score_breakdown.clone(),
            ))
            .collect::<Vec<_>>(),
        direct_route
            .ranked_targets
            .iter()
            .map(|candidate| (
                candidate
                    .target
                    .resource
                    .provider_resource_id
                    .as_str()
                    .to_string(),
                candidate.score_breakdown.clone(),
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        simulation
            .excluded_candidates
            .iter()
            .map(|candidate| (
                candidate.provider_resource_id.as_str().to_string(),
                candidate.reason_code.clone(),
            ))
            .collect::<Vec<_>>(),
        direct_route
            .excluded_targets
            .iter()
            .map(|candidate| (
                candidate.provider_resource_id.as_str().to_string(),
                candidate.reason_code.clone(),
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        simulation
            .excluded_candidates
            .iter()
            .map(|candidate| candidate.reason_code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "capability_gap",
            "capability_gap",
            "health_quarantined",
            "health_draining",
            "health_disabled",
            "protocol_family_unsupported",
        ]
    );
}
