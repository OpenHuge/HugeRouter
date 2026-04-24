use core_domain::{
    AdmissionResult, ConfigSnapshot, ExcludedTarget, HealthState, ProvenanceClass,
    ProviderResource, ProviderResourceStatus, RoutePolicy, ScoreBreakdown,
};
use provider_traits::{ProviderTargetKind, TransitProviderMetadata};
use tracing::info;

use crate::{
    ActiveGatewayConfig, NormalizedChatRequest, RequestContext, normalize_image_model_alias,
};

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

pub fn evaluate_route(
    active_config: &ActiveGatewayConfig,
    request: &NormalizedChatRequest,
    context: &RequestContext,
) -> RouteEvaluation {
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
                reason_code: format!(
                    "health_{}",
                    match target.resource.health_state {
                        HealthState::Healthy => "healthy",
                        HealthState::Degraded => "degraded",
                        HealthState::Quarantined => "quarantined",
                        HealthState::Draining => "draining",
                        HealthState::Disabled => "disabled",
                    }
                ),
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

    info!(
        request_id = context.request_id,
        trace_id = context.trace_id,
        candidate_count = ranked_targets.len(),
        excluded_count = excluded_targets.len(),
        "route evaluation completed"
    );

    RouteEvaluation {
        config_snapshot: active_config.config_snapshot.clone(),
        admission_result,
        excluded_targets,
        ranked_targets,
    }
}

fn target_supports_protocol_family(target: &ProviderTargetRuntime, protocol_family: &str) -> bool {
    target
        .resource
        .supported_protocol_families
        .iter()
        .any(|supported| supported == protocol_family)
}

fn protocol_family_exclusion(
    target: &ProviderTargetRuntime,
    protocol_family: &str,
) -> ExcludedTarget {
    ExcludedTarget {
        provider_resource_id: target.resource.provider_resource_id.clone(),
        reason_code: "protocol_family_unsupported".to_string(),
        reason: format!(
            "provider target does not declare support for protocol family `{protocol_family}`"
        ),
    }
}

fn model_alias_matches(protocol_family: &str, route_alias: &str, request_alias: &str) -> bool {
    if protocol_family == "openai_images" {
        normalize_image_model_alias(route_alias) == normalize_image_model_alias(request_alias)
    } else {
        route_alias == request_alias
    }
}

fn route_capabilities_supported(
    route_policy: &RoutePolicy,
    target: &ProviderTargetRuntime,
) -> bool {
    route_policy
        .required_capabilities
        .iter()
        .all(|capability| match capability.as_str() {
            "streaming" => target.resource.capabilities.supports_streaming,
            "tool_calling" | "tool_related" => target.resource.capabilities.supports_tool_calling,
            "json_mode" => target.resource.capabilities.supports_json_mode,
            "chat_completions" | "image_generation" => true,
            "realtime" => target.resource.capabilities.supports_realtime,
            "response_model_metadata" => {
                target
                    .resource
                    .capabilities
                    .supports_response_model_metadata
            }
            "transit_gateway" => target.target_kind == ProviderTargetKind::TransitGateway,
            "native_provider" => target.target_kind == ProviderTargetKind::Native,
            _ => false,
        })
}

fn score_target(
    active_config: &ActiveGatewayConfig,
    target: &ProviderTargetRuntime,
) -> ScoreBreakdown {
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
