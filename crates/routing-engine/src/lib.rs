use core_domain::{
    AdmissionResult, ConfigSnapshot, ExcludedTarget, HealthState, ProvenanceClass,
    ProviderCapabilities, ProviderResource, ProviderResourceStatus, RoutePolicy, ScoreBreakdown,
};
pub use provider_traits::{ProviderTargetKind, TransitProviderMetadata};

#[derive(Debug, Clone)]
pub struct ProviderTargetRuntime {
    pub resource: ProviderResource,
    pub target_kind: ProviderTargetKind,
    pub transit_metadata: Option<TransitProviderMetadata>,
    pub priority: u32,
    pub upstream_model: Option<String>,
    pub api_key: String,
    pub static_latency_score: f32,
    pub static_cost_score: f32,
    pub usd_per_1k_tokens: f64,
}

#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub config_snapshot: ConfigSnapshot,
    pub route_policy: RoutePolicy,
    pub provider_targets: Vec<ProviderTargetRuntime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingRequest {
    pub protocol_family: String,
    pub model_alias: String,
}

#[derive(Debug, Clone)]
pub struct RankedTarget {
    pub target: ProviderTargetRuntime,
    pub total_score: f32,
    pub score_breakdown: ScoreBreakdown,
}

#[derive(Debug, Clone)]
pub struct RouteEvaluation {
    pub config_snapshot: ConfigSnapshot,
    pub admission_result: AdmissionResult,
    pub excluded_targets: Vec<ExcludedTarget>,
    pub ranked_targets: Vec<RankedTarget>,
}

#[must_use]
pub fn evaluate_route(active_config: &RoutingConfig, request: &RoutingRequest) -> RouteEvaluation {
    let mut excluded_targets = Vec::new();
    let mut ranked_targets = Vec::new();

    if active_config.route_policy.protocol_family != request.protocol_family
        || !model_alias_matches(
            &active_config.route_policy.protocol_family,
            &active_config.route_policy.model_alias,
            &request.model_alias,
        )
    {
        return RouteEvaluation {
            config_snapshot: active_config.config_snapshot.clone(),
            admission_result: AdmissionResult::RejectedNoCandidate,
            excluded_targets,
            ranked_targets,
        };
    }

    let configured_ids = active_config
        .config_snapshot
        .provider_resource_ids
        .iter()
        .collect::<Vec<_>>();

    for target in &active_config.provider_targets {
        if !configured_ids.iter().any(|provider_resource_id| {
            provider_resource_id == &&target.resource.provider_resource_id
        }) {
            continue;
        }

        if target.resource.status != ProviderResourceStatus::Active {
            excluded_targets.push(ExcludedTarget {
                provider_resource_id: target.resource.provider_resource_id.clone(),
                reason_code: "provider_inactive".to_string(),
                reason: format!("provider status is {:?}", target.resource.status),
            });
            continue;
        }

        if matches!(
            target.resource.health_state,
            HealthState::Quarantined | HealthState::Disabled | HealthState::Draining
        ) {
            excluded_targets.push(ExcludedTarget {
                provider_resource_id: target.resource.provider_resource_id.clone(),
                reason_code: format!("health_{}", health_state_slug(target.resource.health_state)),
                reason: format!("provider health is {:?}", target.resource.health_state),
            });
            continue;
        }

        if !target_supports_protocol_family(target, &request.protocol_family) {
            excluded_targets.push(protocol_family_exclusion(target, &request.protocol_family));
            continue;
        }

        if !route_capabilities_supported(&active_config.route_policy, target) {
            excluded_targets.push(ExcludedTarget {
                provider_resource_id: target.resource.provider_resource_id.clone(),
                reason_code: "capability_gap".to_string(),
                reason: "required capabilities are not satisfied by the target".to_string(),
            });
            continue;
        }

        let score_breakdown = score_target(active_config, target);
        let total_score = weighted_score(&score_breakdown, target.priority);

        ranked_targets.push(RankedTarget {
            target: target.clone(),
            total_score,
            score_breakdown,
        });
    }

    ranked_targets.sort_by(|left, right| {
        right
            .total_score
            .partial_cmp(&left.total_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.target.priority.cmp(&right.target.priority))
    });

    let admission_result = if ranked_targets.is_empty() {
        AdmissionResult::RejectedNoCandidate
    } else {
        AdmissionResult::Admitted
    };

    RouteEvaluation {
        config_snapshot: active_config.config_snapshot.clone(),
        admission_result,
        excluded_targets,
        ranked_targets,
    }
}

#[must_use]
pub fn route_capability_supported_by_provider_capabilities(capability: &str) -> bool {
    matches!(
        capability,
        "streaming"
            | "tool_calling"
            | "tool_related"
            | "json_mode"
            | "chat_completions"
            | "image_generation"
            | "realtime"
            | "response_model_metadata"
            | "transit_gateway"
            | "native_provider"
    )
}

#[must_use]
pub fn provider_supports_protocol_family(
    provider_resource: &ProviderResource,
    protocol_family: &str,
) -> bool {
    provider_resource
        .supported_protocol_families
        .iter()
        .any(|candidate| candidate == protocol_family)
}

#[must_use]
pub fn provider_supports_capability(
    provider_resource: &ProviderResource,
    capability: &str,
) -> bool {
    match capability {
        "transit_gateway" => provider_resource.is_transit_gateway,
        "native_provider" => !provider_resource.is_transit_gateway,
        _ => route_capability_supported(capability, &provider_resource.capabilities),
    }
}

#[must_use]
pub fn provider_capability_gaps(
    provider_resource: &ProviderResource,
    required_capabilities: &[String],
) -> Vec<String> {
    required_capabilities
        .iter()
        .filter(|capability| !provider_supports_capability(provider_resource, capability))
        .cloned()
        .collect()
}

#[must_use]
pub const fn is_health_blocked(health_state: HealthState) -> bool {
    matches!(
        health_state,
        HealthState::Quarantined | HealthState::Draining | HealthState::Disabled
    )
}

#[must_use]
pub const fn health_state_slug(health_state: HealthState) -> &'static str {
    match health_state {
        HealthState::Healthy => "healthy",
        HealthState::Degraded => "degraded",
        HealthState::Quarantined => "quarantined",
        HealthState::Draining => "draining",
        HealthState::Disabled => "disabled",
    }
}

fn target_supports_protocol_family(target: &ProviderTargetRuntime, protocol_family: &str) -> bool {
    provider_supports_protocol_family(&target.resource, protocol_family)
}

fn protocol_family_exclusion(
    target: &ProviderTargetRuntime,
    protocol_family: &str,
) -> ExcludedTarget {
    ExcludedTarget {
        provider_resource_id: target.resource.provider_resource_id.clone(),
        reason_code: "protocol_family_unsupported".to_string(),
        reason: format!("provider does not advertise protocol family `{protocol_family}`"),
    }
}

fn model_alias_matches(protocol_family: &str, route_alias: &str, request_alias: &str) -> bool {
    if protocol_family == "openai_images" {
        normalize_image_model_alias(route_alias) == normalize_image_model_alias(request_alias)
    } else {
        route_alias == request_alias
    }
}

fn normalize_image_model_alias(model: &str) -> String {
    match model.trim().to_ascii_lowercase().as_str() {
        "chatgpt image 2" | "chatgpt-image-2" | "chatgpt_image_2" => {
            "chatgpt-image-latest".to_string()
        }
        _ => model.trim().to_string(),
    }
}

fn route_capabilities_supported(
    route_policy: &RoutePolicy,
    target: &ProviderTargetRuntime,
) -> bool {
    route_policy
        .required_capabilities
        .iter()
        .all(|capability| route_capability_supported_for_target(capability.as_str(), target))
}

fn route_capability_supported_for_target(capability: &str, target: &ProviderTargetRuntime) -> bool {
    match capability {
        "transit_gateway" => target.target_kind == ProviderTargetKind::TransitGateway,
        "native_provider" => target.target_kind == ProviderTargetKind::Native,
        _ => route_capability_supported(capability, &target.resource.capabilities),
    }
}

fn route_capability_supported(capability: &str, target: &ProviderCapabilities) -> bool {
    match capability {
        "streaming" => target.supports_streaming,
        "tool_calling" | "tool_related" => target.supports_tool_calling,
        "json_mode" => target.supports_json_mode,
        "chat_completions" | "image_generation" => true,
        "realtime" => target.supports_realtime,
        "response_model_metadata" => target.supports_response_model_metadata,
        _ => false,
    }
}

fn score_target(active_config: &RoutingConfig, target: &ProviderTargetRuntime) -> ScoreBreakdown {
    let preferred_region = active_config
        .route_policy
        .preferred_regions
        .iter()
        .any(|region| region == &target.resource.region);
    let latency = if preferred_region {
        target.static_latency_score
    } else {
        (target.static_latency_score * 0.7).max(0.1)
    };
    let latency = target
        .transit_metadata
        .as_ref()
        .map_or(latency, |metadata| {
            f32::from(metadata.transit_hops)
                .mul_add(-0.05, latency)
                .max(0.1)
        });
    let health = match target.resource.health_state {
        HealthState::Healthy => 1.0,
        HealthState::Degraded => 0.55,
        HealthState::Quarantined | HealthState::Draining | HealthState::Disabled => 0.0,
    };
    let trust: f32 = match target.resource.provenance_class {
        ProvenanceClass::OfficialApi => 1.0,
        ProvenanceClass::OfficialGateway => 0.95,
        ProvenanceClass::DedicatedManagedAccount => 0.85,
        ProvenanceClass::ByoCustomerCredential => 0.8,
        ProvenanceClass::SharedBrokeredPool => 0.6,
        ProvenanceClass::UnofficialClientChannel => 0.2,
    };
    let trust = target.transit_metadata.as_ref().map_or(trust, |metadata| {
        let penalty = if metadata.preserves_error_diagnostics {
            0.92
        } else {
            0.85
        };
        (trust * penalty).max(0.1_f32)
    });

    ScoreBreakdown {
        latency,
        cost: target.static_cost_score,
        health,
        trust,
    }
}

#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
fn weighted_score(score_breakdown: &ScoreBreakdown, priority: u32) -> f32 {
    let base = score_breakdown.latency
        + score_breakdown.cost
        + score_breakdown.health
        + score_breakdown.trust;
    let priority_bonus = 1.0 - (priority as f32 * 0.01);
    (base / 4.0) * priority_bonus
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::{
        AuthKind, BudgetPolicyId, ConfigSnapshotId, CredentialOwnerType, DeploymentScope,
        ProjectId, ProviderResourceId, RoutePolicyId, TenantId,
    };

    #[test]
    fn excludes_target_without_protocol_support() {
        let mut target = target("prvrsrc_one", "us-east-1", HealthState::Healthy);
        target.resource.supported_protocol_families = vec!["openai_chat".to_string()];
        let mut config = config(vec![target]);
        config.route_policy.protocol_family = "openai_responses".to_string();
        let request = RoutingRequest {
            protocol_family: "openai_responses".to_string(),
            model_alias: "reasoning-fast".to_string(),
        };

        let route = evaluate_route(&config, &request);

        assert_eq!(route.admission_result, AdmissionResult::RejectedNoCandidate);
        assert_eq!(
            route.excluded_targets[0].reason_code,
            "protocol_family_unsupported"
        );
    }

    #[test]
    fn ranks_preferred_region_first() {
        let config = config(vec![
            target("prvrsrc_west", "us-west-2", HealthState::Healthy),
            target("prvrsrc_east", "us-east-1", HealthState::Healthy),
        ]);
        let request = RoutingRequest {
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
        };

        let route = evaluate_route(&config, &request);

        assert_eq!(
            route.ranked_targets[0]
                .target
                .resource
                .provider_resource_id
                .as_str(),
            "prvrsrc_east"
        );
    }

    fn config(provider_targets: Vec<ProviderTargetRuntime>) -> RoutingConfig {
        RoutingConfig {
            config_snapshot: ConfigSnapshot {
                config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_test").unwrap(),
                tenant_id: TenantId::parse("tenant_test").unwrap(),
                project_id: ProjectId::parse("proj_test").unwrap(),
                revision: 1,
                budget_policy_id: BudgetPolicyId::parse("budgetpol_test").unwrap(),
                route_policy_id: RoutePolicyId::parse("routepol_test").unwrap(),
                provider_resource_ids: provider_targets
                    .iter()
                    .map(|target| target.resource.provider_resource_id.clone())
                    .collect(),
                status: core_domain::ConfigSnapshotStatus::Active,
                activated_at: Some("2026-04-24T00:00:00Z".to_string()),
            },
            route_policy: RoutePolicy {
                route_policy_id: RoutePolicyId::parse("routepol_test").unwrap(),
                tenant_id: TenantId::parse("tenant_test").unwrap(),
                display_name: "Default".to_string(),
                protocol_family: "openai_chat".to_string(),
                model_alias: "reasoning-fast".to_string(),
                required_capabilities: vec!["json_mode".to_string()],
                preferred_regions: vec!["us-east-1".to_string()],
                version: 1,
                created_at: "2026-04-24T00:00:00Z".to_string(),
                updated_at: "2026-04-24T00:00:00Z".to_string(),
            },
            provider_targets,
        }
    }

    fn target(id: &str, region: &str, health_state: HealthState) -> ProviderTargetRuntime {
        ProviderTargetRuntime {
            resource: ProviderResource {
                provider_resource_id: ProviderResourceId::parse(id.to_string()).unwrap(),
                tenant_id: TenantId::parse("tenant_test").unwrap(),
                project_id: Some(ProjectId::parse("proj_test").unwrap()),
                provider_id: "openai".to_string(),
                name: id.to_string(),
                status: ProviderResourceStatus::Active,
                provenance_class: ProvenanceClass::OfficialApi,
                credential_owner_type: CredentialOwnerType::Platform,
                deployment_scope: DeploymentScope::Shared,
                region: region.to_string(),
                endpoint_base_url: "https://api.example.test".to_string(),
                auth_kind: AuthKind::ApiKey,
                health_state,
                health_message: None,
                quarantine_reason: None,
                budget_policy_id: None,
                capabilities: ProviderCapabilities {
                    supports_streaming: false,
                    supports_tool_calling: true,
                    supports_json_mode: true,
                    supports_realtime: false,
                    supports_response_model_metadata: true,
                },
                supported_protocol_families: vec!["openai_chat".to_string()],
                is_transit_gateway: false,
                version: 1,
                created_at: "2026-04-24T00:00:00Z".to_string(),
                updated_at: "2026-04-24T00:00:00Z".to_string(),
            },
            target_kind: ProviderTargetKind::Native,
            transit_metadata: None,
            priority: 0,
            upstream_model: None,
            api_key: "test".to_string(),
            static_latency_score: 0.95,
            static_cost_score: 0.8,
            usd_per_1k_tokens: 0.01,
        }
    }
}
