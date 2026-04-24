use axum::http::StatusCode;
use core_domain::{
    AdmissionResult, FallbackTransition, MonetaryAmount, NormalizedError, RouteReceipt,
    RouteReceiptId, ScoreBreakdown, UsageEvent, UsageEventId, UsageMetrics, UsagePhase,
};
use protocol_ir::{
    RouteReceiptDecisionTraceStep, RouteReceiptPolicyCheck, RouteReceiptProviderAttempt,
    RouteReceiptRecorded,
};
use provider_traits::{ProviderImageResponse, ProviderResponse};

use crate::route_evaluation::{ProviderTargetRuntime, RankedTarget, RouteEvaluation};
use crate::{
    GatewayDebugHeaders, GatewayError, GatewayState, NormalizedChatRequest, RequestContext,
    image_usd_per_generation_for_target, normalized_error, now_rfc3339,
};

pub fn build_route_receipt(
    route: &RouteEvaluation,
    context: &RequestContext,
    request: &NormalizedChatRequest,
    ranked_target: Option<&RankedTarget>,
    normalized_error: Option<NormalizedError>,
    fallback_transitions: Vec<FallbackTransition>,
) -> RouteReceipt {
    let failure_reason = normalized_error.as_ref().map(|error| error.message.clone());

    RouteReceipt {
        route_receipt_id: RouteReceiptId::parse(format!("routercpt_{}", context.sequence))
            .expect("route receipt id should be valid"),
        tenant_id: route.config_snapshot.tenant_id.clone(),
        project_id: route.config_snapshot.project_id.clone(),
        route_policy_id: route.config_snapshot.route_policy_id.clone(),
        request_id: context.request_id.clone(),
        trace_id: context.trace_id.clone(),
        protocol_family: request.protocol_family.clone(),
        model_alias: request.model_alias.clone(),
        config_snapshot_id: route.config_snapshot.config_snapshot_id.clone(),
        admission_result: route.admission_result,
        selected_target: ranked_target
            .map(|ranked| ranked.target.resource.provider_resource_id.clone()),
        excluded_targets: route.excluded_targets.clone(),
        score_breakdown: ranked_target.map_or(
            ScoreBreakdown {
                latency: 0.0,
                cost: 0.0,
                health: 0.0,
                trust: 0.0,
            },
            |ranked| ranked.score_breakdown.clone(),
        ),
        fallback_transitions,
        normalized_error,
        failure_reason,
        created_at: now_rfc3339(),
    }
}

pub fn build_usage_event(
    route_receipt: &RouteReceipt,
    target: &ProviderTargetRuntime,
    request: &NormalizedChatRequest,
    response: &ProviderResponse,
) -> UsageEvent {
    let usage = UsageMetrics {
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
        cached_input_tokens: response.usage.cached_input_tokens,
    };
    let total_tokens = f64::from(usage.input_tokens + usage.output_tokens);
    let estimated_cost = (total_tokens / 1_000.0) * target.usd_per_1k_tokens;

    UsageEvent {
        usage_event_id: UsageEventId::parse(format!(
            "usageevt_{}",
            route_receipt
                .route_receipt_id
                .as_str()
                .trim_start_matches("routercpt_")
        ))
        .expect("usage event id should be valid"),
        route_receipt_id: route_receipt.route_receipt_id.clone(),
        tenant_id: route_receipt.tenant_id.clone(),
        project_id: route_receipt.project_id.clone(),
        provider_resource_id: target.resource.provider_resource_id.clone(),
        model_alias: request.model_alias.clone(),
        phase: UsagePhase::Final,
        idempotency_key: format!("{}:final", route_receipt.route_receipt_id),
        usage,
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: format!("{estimated_cost:.6}"),
        },
        recorded_at: now_rfc3339(),
    }
}

pub fn build_image_usage_event(
    route_receipt: &RouteReceipt,
    target: &ProviderTargetRuntime,
    request: &NormalizedChatRequest,
    response: &ProviderImageResponse,
) -> UsageEvent {
    let usage = UsageMetrics {
        input_tokens: response
            .usage
            .input_tokens
            .max(request.estimated_prompt_tokens),
        output_tokens: response.usage.output_tokens,
        cached_input_tokens: response.usage.cached_input_tokens,
    };
    let total_tokens = f64::from(usage.input_tokens + usage.output_tokens);
    let image_units = f64::from(u32::try_from(response.images.len().max(1)).unwrap_or(u32::MAX));
    let estimated_cost = (total_tokens / 1_000.0).mul_add(
        target.usd_per_1k_tokens,
        image_units * image_usd_per_generation_for_target(target),
    );

    UsageEvent {
        usage_event_id: UsageEventId::parse(format!(
            "usageevt_{}",
            route_receipt
                .route_receipt_id
                .as_str()
                .trim_start_matches("routercpt_")
        ))
        .expect("usage event id should be valid"),
        route_receipt_id: route_receipt.route_receipt_id.clone(),
        tenant_id: route_receipt.tenant_id.clone(),
        project_id: route_receipt.project_id.clone(),
        provider_resource_id: target.resource.provider_resource_id.clone(),
        model_alias: request.model_alias.clone(),
        phase: UsagePhase::Final,
        idempotency_key: format!("{}:final", route_receipt.route_receipt_id),
        usage,
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: format!("{estimated_cost:.6}"),
        },
        recorded_at: now_rfc3339(),
    }
}

pub fn build_route_receipt_policy_checks(
    route: &RouteEvaluation,
    route_receipt: &RouteReceipt,
) -> Vec<RouteReceiptPolicyCheck> {
    vec![RouteReceiptPolicyCheck {
        policy_id: route.config_snapshot.route_policy_id.clone(),
        status: match route.admission_result {
            AdmissionResult::Admitted => "passed".to_string(),
            _ => "failed".to_string(),
        },
        reason: match route.admission_result {
            AdmissionResult::Admitted => None,
            _ => Some(route_receipt.normalized_error.as_ref().map_or_else(
                || "request rejected before provider execution".to_string(),
                |error| error.message.clone(),
            )),
        },
    }]
}

pub fn build_route_receipt_decision_timeline(
    route: &RouteEvaluation,
    route_receipt: &RouteReceipt,
    provider_attempts: &[RouteReceiptProviderAttempt],
) -> Vec<RouteReceiptDecisionTraceStep> {
    let mut timeline = vec![RouteReceiptDecisionTraceStep {
        stage: "admission".to_string(),
        status: match route.admission_result {
            AdmissionResult::Admitted => "passed".to_string(),
            _ => "failed".to_string(),
        },
        message: match route.admission_result {
            AdmissionResult::Admitted => "Request admitted by active route policy".to_string(),
            _ => "Request rejected before provider execution".to_string(),
        },
        score: Some(match route.admission_result {
            AdmissionResult::Admitted => 1.0,
            _ => 0.0,
        }),
        notes: vec![
            format!("protocol_family={}", route_receipt.protocol_family),
            format!("model_alias={}", route_receipt.model_alias),
        ],
    }];

    timeline.push(RouteReceiptDecisionTraceStep {
        stage: "candidate_selection".to_string(),
        status: if route_receipt.selected_target.is_some() {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        message: route_receipt.selected_target.as_ref().map_or_else(
            || {
                if route.ranked_targets.is_empty() {
                    "No eligible provider target satisfied the request".to_string()
                } else {
                    "Candidate selection completed but no provider attempt succeeded".to_string()
                }
            },
            |selected_target| {
                format!(
                    "Selected {selected_target} from {} ranked candidates",
                    route.ranked_targets.len()
                )
            },
        ),
        score: route_receipt.selected_target.as_ref().map(|_| {
            route_receipt.score_breakdown.latency
                + route_receipt.score_breakdown.cost
                + route_receipt.score_breakdown.health
                + route_receipt.score_breakdown.trust
        }),
        notes: vec![
            format!("excluded_targets={}", route_receipt.excluded_targets.len()),
            format!(
                "fallback_transitions={}",
                route_receipt.fallback_transitions.len()
            ),
        ],
    });

    if let Some(last_attempt) = provider_attempts.last() {
        timeline.push(RouteReceiptDecisionTraceStep {
            stage: "provider_execution".to_string(),
            status: last_attempt.status.clone(),
            message: if route_receipt.normalized_error.is_some() {
                format!(
                    "Provider execution ended with {} after {} attempts",
                    last_attempt.status,
                    provider_attempts.len()
                )
            } else {
                format!(
                    "Provider execution completed via {} on attempt {}",
                    last_attempt.provider_resource_id, last_attempt.attempt
                )
            },
            score: None,
            notes: vec![format!("provider_attempts={}", provider_attempts.len())],
        });
    }

    timeline
}

pub fn build_route_receipt_recorded(
    route: &RouteEvaluation,
    route_receipt: &RouteReceipt,
    provider_attempts: Vec<RouteReceiptProviderAttempt>,
) -> RouteReceiptRecorded {
    RouteReceiptRecorded {
        route_receipt: route_receipt.clone(),
        decision_timeline: build_route_receipt_decision_timeline(
            route,
            route_receipt,
            &provider_attempts,
        ),
        policy_checks: build_route_receipt_policy_checks(route, route_receipt),
        provider_attempts,
    }
}

pub fn provider_attempt_record(
    provider_resource_id: core_domain::ProviderResourceId,
    attempt: usize,
    status: impl Into<String>,
    started_at: String,
    finished_at: String,
    latency_ms: u32,
    reason: impl Into<String>,
) -> RouteReceiptProviderAttempt {
    RouteReceiptProviderAttempt {
        provider_resource_id,
        attempt: u8::try_from(attempt).unwrap_or(u8::MAX),
        status: status.into(),
        started_at,
        finished_at,
        latency_ms,
        reason: reason.into(),
    }
}

pub async fn publish_route_receipt_or_error(
    state: &GatewayState,
    route: &RouteEvaluation,
    route_receipt: &RouteReceipt,
    provider_attempts: Vec<RouteReceiptProviderAttempt>,
    context: &RequestContext,
    debug_headers: Option<GatewayDebugHeaders>,
) -> Result<(), GatewayError> {
    let payload = build_route_receipt_recorded(route, route_receipt, provider_attempts);
    state
        .event_sink
        .publish_route_receipt(route_receipt, &payload, context)
        .await
        .map_err(|message| {
            GatewayError::with_route_receipt(
                StatusCode::SERVICE_UNAVAILABLE,
                route_receipt.clone(),
                normalized_error("route_receipt_publish_failed", message, context, true).error,
                context,
                Some(route.config_snapshot.config_snapshot_id.clone()),
                debug_headers,
            )
        })
}
