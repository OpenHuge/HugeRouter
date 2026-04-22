mod openai;

use axum::{
    Json, Router,
    extract::State,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use core_domain::{
    AdmissionResult, AuthKind, ConfigSnapshot, ConfigSnapshotId, ConfigSnapshotStatus,
    CredentialOwnerType, DeploymentScope, ErrorEnvelope, ExcludedTarget, FallbackTransition,
    HealthState, MonetaryAmount, NormalizedError, ProjectId, ProvenanceClass, ProviderCapabilities,
    ProviderResource, ProviderResourceId, ProviderResourceStatus, RoutePolicy, RoutePolicyId,
    RouteReceipt, RouteReceiptId, ScoreBreakdown, TenantId, UsageEvent, UsageEventId, UsageMetrics,
    UsagePhase, ValidationIssue,
};
use openai::OpenAiAdapter;
use provider_traits::{
    ProviderAdapterRegistry, ProviderEndpoint, ProviderError, ProviderErrorKind,
    ProviderExecutionContext, ProviderMessage, ProviderRequest, ProviderResponse,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::{info, warn};

const GATEWAY_SERVICE_NAME: &str = "gateway-api";
const DEFAULT_TENANT_ID: &str = "tenant_acme";
const DEFAULT_PROJECT_ID: &str = "proj_core";
const DEFAULT_CONFIG_SNAPSHOT_ID: &str = "cfgsnap_gateway_v1";
const DEFAULT_ROUTE_POLICY_ID: &str = "routepol_openai_chat_default";
const DEFAULT_PROVIDER_RESOURCE_ID: &str = "prvrsrc_openai_primary";

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1_000);

pub type GatewayState = Arc<AppState>;

pub struct AppState {
    config_store: Arc<dyn ActiveConfigStore>,
    adapter_registry: ProviderAdapterRegistry,
    debug_headers_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: UsageSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: AssistantMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssistantMessage {
    pub role: &'static str,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone)]
pub struct GatewaySuccess {
    pub request_id: String,
    pub trace_id: String,
    pub config_snapshot_id: ConfigSnapshotId,
    pub route_receipt: RouteReceipt,
    pub usage_event: UsageEvent,
    pub response: ChatCompletionResponse,
    pub debug_headers: Option<GatewayDebugHeaders>,
}

#[derive(Debug, Clone)]
pub struct GatewayError {
    pub status: StatusCode,
    pub request_id: String,
    pub trace_id: String,
    pub route_receipt_id: Option<RouteReceiptId>,
    pub config_snapshot_id: Option<ConfigSnapshotId>,
    pub envelope: ErrorEnvelope,
    pub debug_headers: Option<GatewayDebugHeaders>,
}

#[derive(Debug, Clone)]
pub struct GatewayDebugHeaders {
    pub selected_target: Option<String>,
    pub route_policy_id: String,
    pub fallback_count: usize,
    pub admission_result: &'static str,
}

#[derive(Debug, Clone)]
struct RequestContext {
    request_id: String,
    trace_id: String,
    sequence: u64,
}

#[derive(Debug, Clone)]
struct NormalizedChatRequest {
    model_alias: String,
    protocol_family: String,
    messages: Vec<ProviderMessage>,
    estimated_prompt_tokens: u32,
}

#[derive(Debug, Clone)]
struct ActiveGatewayConfig {
    config_snapshot: ConfigSnapshot,
    route_policy: RoutePolicy,
    provider_targets: Vec<ProviderTargetRuntime>,
}

#[derive(Debug, Clone)]
struct ProviderTargetRuntime {
    resource: ProviderResource,
    priority: u32,
    upstream_model: Option<String>,
    api_key: String,
    static_latency_score: f32,
    static_cost_score: f32,
    usd_per_1k_tokens: f64,
}

#[derive(Debug, Clone)]
struct RankedTarget {
    target: ProviderTargetRuntime,
    total_score: f32,
    score_breakdown: ScoreBreakdown,
}

#[derive(Debug, Clone)]
struct RouteEvaluation {
    config_snapshot: ConfigSnapshot,
    admission_result: AdmissionResult,
    excluded_targets: Vec<ExcludedTarget>,
    ranked_targets: Vec<RankedTarget>,
}

pub fn app() -> Router {
    app_with_state(default_state())
}

pub fn app_with_state(state: GatewayState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state)
}

fn default_state() -> GatewayState {
    let config_store = Arc::new(StaticConfigStore::new(default_active_config()));
    let mut adapter_registry = ProviderAdapterRegistry::new();
    adapter_registry
        .register(Arc::new(OpenAiAdapter::default()))
        .expect("openai adapter registration should succeed");

    Arc::new(AppState {
        config_store,
        adapter_registry,
        debug_headers_enabled: debug_headers_enabled_from_env(),
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: GATEWAY_SERVICE_NAME,
        status: "ok",
    })
}

async fn chat_completions(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Response {
    match process_chat_completion(state, headers.get(AUTHORIZATION), request).await {
        Ok(success) => success.into_response(),
        Err(error) => error.into_response(),
    }
}

async fn process_chat_completion(
    state: GatewayState,
    authorization_header: Option<&HeaderValue>,
    request: ChatCompletionRequest,
) -> Result<GatewaySuccess, GatewayError> {
    let context = next_request_context();

    ensure_bearer_auth(authorization_header, &context)?;
    let normalized_request = normalize_request(request, &context)?;

    let active_config = state.config_store.load().map_err(|error| {
        GatewayError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            normalized_error(
                "route_not_available",
                format!("active configuration is unavailable: {error}"),
                &context,
                true,
            ),
            &context,
        )
    })?;
    let route = evaluate_route(&active_config, &normalized_request, &context);

    if route.ranked_targets.is_empty() {
        let normalized = normalized_error(
            "route_not_available",
            "no active provider targets can satisfy the request".to_string(),
            &context,
            true,
        )
        .error;
        let route_receipt = build_route_receipt(
            &route,
            &context,
            &normalized_request,
            None,
            Some(normalized.clone()),
            Vec::new(),
        );

        return Err(GatewayError::with_route_receipt(
            StatusCode::SERVICE_UNAVAILABLE,
            route_receipt,
            normalized,
            &context,
            Some(active_config.config_snapshot.config_snapshot_id.clone()),
            maybe_debug_headers(
                state.debug_headers_enabled,
                &active_config.route_policy.route_policy_id,
                None,
                AdmissionResult::RejectedNoCandidate,
                0,
            ),
        ));
    }

    execute_route(state, route, normalized_request, context).await
}

#[allow(clippy::too_many_lines)]
async fn execute_route(
    state: GatewayState,
    route: RouteEvaluation,
    request: NormalizedChatRequest,
    context: RequestContext,
) -> Result<GatewaySuccess, GatewayError> {
    let mut fallback_transitions = Vec::new();
    let mut last_error = None;

    for (index, ranked_target) in route.ranked_targets.iter().enumerate() {
        let target = &ranked_target.target;
        let Some(adapter) = state.adapter_registry.resolve(&target.resource.provider_id) else {
            let provider_error = ProviderError::new(
                ProviderErrorKind::Unavailable,
                format!(
                    "no provider adapter registered for `{}`",
                    target.resource.provider_id
                ),
                false,
            )
            .with_detail(
                "provider_resource_id",
                target.resource.provider_resource_id.as_str(),
            );

            if let Some(next_target) = route.ranked_targets.get(index + 1) {
                fallback_transitions.push(FallbackTransition {
                    from_provider_resource_id: target.resource.provider_resource_id.clone(),
                    to_provider_resource_id: next_target
                        .target
                        .resource
                        .provider_resource_id
                        .clone(),
                    reason: format!(
                        "no provider adapter registered for `{}`; retrying next ranked candidate",
                        target.resource.provider_id
                    ),
                });
            }

            last_error = Some((ranked_target.clone(), provider_error));
            if index + 1 < route.ranked_targets.len() {
                continue;
            }

            break;
        };

        let provider_request = ProviderRequest {
            model: target
                .upstream_model
                .clone()
                .unwrap_or_else(|| request.model_alias.clone()),
            messages: request.messages.clone(),
            stream: false,
        };
        let provider_context = ProviderExecutionContext {
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
            endpoint: ProviderEndpoint {
                provider_resource_id: target.resource.provider_resource_id.as_str().to_string(),
                endpoint_base_url: target.resource.endpoint_base_url.clone(),
                api_key: target.api_key.clone(),
            },
        };

        match adapter
            .execute_chat(&provider_request, &provider_context)
            .await
        {
            Ok(provider_response) => {
                let fallback_count = fallback_transitions.len();
                let route_receipt = build_route_receipt(
                    &route,
                    &context,
                    &request,
                    Some(ranked_target),
                    None,
                    fallback_transitions,
                );
                let usage_event =
                    build_usage_event(&route_receipt, target, &request, &provider_response);

                info!(
                    request_id = context.request_id,
                    trace_id = context.trace_id,
                    route_receipt_id = %route_receipt.route_receipt_id,
                    provider_resource_id = %target.resource.provider_resource_id,
                    "gateway request routed successfully"
                );

                return Ok(GatewaySuccess {
                    request_id: context.request_id.clone(),
                    trace_id: context.trace_id.clone(),
                    config_snapshot_id: route.config_snapshot.config_snapshot_id.clone(),
                    route_receipt,
                    usage_event,
                    response: map_provider_response(&context, &request, &provider_response),
                    debug_headers: maybe_debug_headers(
                        state.debug_headers_enabled,
                        &route.config_snapshot.route_policy_id,
                        Some(target.resource.provider_resource_id.as_str()),
                        AdmissionResult::Admitted,
                        fallback_count,
                    ),
                });
            }
            Err(error) => {
                warn!(
                    request_id = context.request_id,
                    trace_id = context.trace_id,
                    provider_resource_id = %target.resource.provider_resource_id,
                    message = error.message,
                    "provider execution failed"
                );

                if let Some(next_target) = route.ranked_targets.get(index + 1)
                    && error.retryable
                {
                    fallback_transitions.push(FallbackTransition {
                        from_provider_resource_id: target.resource.provider_resource_id.clone(),
                        to_provider_resource_id: next_target
                            .target
                            .resource
                            .provider_resource_id
                            .clone(),
                        reason: format!("{}; retrying next ranked candidate", error.message),
                    });
                }

                last_error = Some((ranked_target.clone(), error.clone()));

                let has_more_candidates = index + 1 < route.ranked_targets.len();
                if !error.retryable || !has_more_candidates {
                    let normalized = map_provider_error(&error, &context);
                    let fallback_count = fallback_transitions.len();
                    let route_receipt = build_route_receipt(
                        &route,
                        &context,
                        &request,
                        Some(ranked_target),
                        Some(normalized.error.clone()),
                        fallback_transitions,
                    );

                    return Err(GatewayError::with_route_receipt(
                        status_for_error_code(&normalized.error.code),
                        route_receipt,
                        normalized.error,
                        &context,
                        Some(route.config_snapshot.config_snapshot_id.clone()),
                        maybe_debug_headers(
                            state.debug_headers_enabled,
                            &route.config_snapshot.route_policy_id,
                            Some(ranked_target.target.resource.provider_resource_id.as_str()),
                            route.admission_result,
                            fallback_count,
                        ),
                    ));
                }
            }
        }
    }

    let (ranked_target, provider_error) = last_error.expect("at least one target was evaluated");
    let normalized = map_provider_error(&provider_error, &context);
    let fallback_count = fallback_transitions.len();
    let route_receipt = build_route_receipt(
        &route,
        &context,
        &request,
        Some(&ranked_target),
        Some(normalized.error.clone()),
        fallback_transitions,
    );

    Err(GatewayError::with_route_receipt(
        status_for_error_code(&normalized.error.code),
        route_receipt,
        normalized.error,
        &context,
        Some(route.config_snapshot.config_snapshot_id.clone()),
        maybe_debug_headers(
            state.debug_headers_enabled,
            &route.config_snapshot.route_policy_id,
            Some(ranked_target.target.resource.provider_resource_id.as_str()),
            route.admission_result,
            fallback_count,
        ),
    ))
}

#[allow(clippy::result_large_err)]
fn ensure_bearer_auth(
    authorization_header: Option<&HeaderValue>,
    context: &RequestContext,
) -> Result<(), GatewayError> {
    let Some(value) = authorization_header else {
        return Err(GatewayError::new(
            StatusCode::UNAUTHORIZED,
            normalized_error(
                "auth_invalid",
                "missing Authorization header".to_string(),
                context,
                false,
            ),
            context,
        ));
    };

    let is_bearer = value
        .to_str()
        .ok()
        .is_some_and(|header| header.starts_with("Bearer "));

    if is_bearer {
        Ok(())
    } else {
        Err(GatewayError::new(
            StatusCode::UNAUTHORIZED,
            normalized_error(
                "auth_invalid",
                "Authorization header must use Bearer credentials".to_string(),
                context,
                false,
            ),
            context,
        ))
    }
}

#[allow(clippy::result_large_err)]
fn normalize_request(
    request: ChatCompletionRequest,
    context: &RequestContext,
) -> Result<NormalizedChatRequest, GatewayError> {
    let mut validation_issues = Vec::new();

    if request.model.trim().is_empty() {
        validation_issues.push(ValidationIssue {
            field: "model".to_string(),
            message: "model must not be empty".to_string(),
        });
    }

    if request.stream {
        validation_issues.push(ValidationIssue {
            field: "stream".to_string(),
            message: "stream=true is intentionally deferred for this slice".to_string(),
        });
    }

    if request.messages.is_empty() {
        validation_issues.push(ValidationIssue {
            field: "messages".to_string(),
            message: "messages must include at least one item".to_string(),
        });
    }

    for (index, message) in request.messages.iter().enumerate() {
        if message.role.trim().is_empty() {
            validation_issues.push(ValidationIssue {
                field: format!("messages[{index}].role"),
                message: "role must not be empty".to_string(),
            });
        }

        if message.content.trim().is_empty() {
            validation_issues.push(ValidationIssue {
                field: format!("messages[{index}].content"),
                message: "content must not be empty".to_string(),
            });
        }
    }

    if !validation_issues.is_empty() {
        return Err(GatewayError::new(
            StatusCode::BAD_REQUEST,
            validation_error(
                "request validation failed",
                validation_issues,
                context,
                false,
            ),
            context,
        ));
    }

    Ok(NormalizedChatRequest {
        model_alias: request.model,
        protocol_family: "openai_chat".to_string(),
        estimated_prompt_tokens: estimate_prompt_tokens(&request.messages),
        messages: request
            .messages
            .into_iter()
            .map(|message| ProviderMessage {
                role: message.role,
                content: message.content,
            })
            .collect(),
    })
}

fn evaluate_route(
    active_config: &ActiveGatewayConfig,
    request: &NormalizedChatRequest,
    context: &RequestContext,
) -> RouteEvaluation {
    let mut excluded_targets = Vec::new();
    let mut ranked_targets = Vec::new();

    if active_config.route_policy.protocol_family != request.protocol_family
        || active_config.route_policy.model_alias != request.model_alias
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
                reason: format!("provider health is {:?}", target.resource.health_state),
            });
            continue;
        }

        if !route_capabilities_supported(&active_config.route_policy, &target.resource.capabilities)
        {
            excluded_targets.push(ExcludedTarget {
                provider_resource_id: target.resource.provider_resource_id.clone(),
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

fn route_capabilities_supported(
    route_policy: &RoutePolicy,
    capabilities: &ProviderCapabilities,
) -> bool {
    route_policy
        .required_capabilities
        .iter()
        .all(|capability| match capability.as_str() {
            "streaming" => capabilities.supports_streaming,
            "tool_calling" => capabilities.supports_tool_calling,
            "json_mode" => capabilities.supports_json_mode,
            "chat_completions" => true,
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
    let health = match target.resource.health_state {
        HealthState::Healthy => 1.0,
        HealthState::Degraded => 0.55,
        HealthState::Quarantined | HealthState::Draining | HealthState::Disabled => 0.0,
    };
    let trust = match target.resource.provenance_class {
        ProvenanceClass::OfficialApi => 1.0,
        ProvenanceClass::OfficialGateway => 0.95,
        ProvenanceClass::DedicatedManagedAccount => 0.85,
        ProvenanceClass::ByoCustomerCredential => 0.8,
        ProvenanceClass::SharedBrokeredPool => 0.6,
        ProvenanceClass::UnofficialClientChannel => 0.2,
    };

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

fn build_route_receipt(
    route: &RouteEvaluation,
    context: &RequestContext,
    request: &NormalizedChatRequest,
    ranked_target: Option<&RankedTarget>,
    normalized_error: Option<NormalizedError>,
    fallback_transitions: Vec<FallbackTransition>,
) -> RouteReceipt {
    RouteReceipt {
        route_receipt_id: RouteReceiptId::parse(format!("routercpt_{}", context.sequence))
            .expect("route receipt id should be valid"),
        tenant_id: route.config_snapshot.tenant_id.clone(),
        project_id: route.config_snapshot.project_id.clone(),
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
        created_at: now_rfc3339(),
    }
}

fn build_usage_event(
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

fn map_provider_response(
    context: &RequestContext,
    request: &NormalizedChatRequest,
    response: &ProviderResponse,
) -> ChatCompletionResponse {
    let prompt_tokens = response
        .usage
        .input_tokens
        .max(request.estimated_prompt_tokens);
    let completion_tokens = response.usage.output_tokens;

    ChatCompletionResponse {
        id: response
            .response_id
            .clone()
            .unwrap_or_else(|| format!("chatcmpl_{}", context.sequence)),
        object: "chat.completion",
        created: unix_timestamp_seconds(),
        model: response.model.clone(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: AssistantMessage {
                role: "assistant",
                content: response.output_text.clone(),
            },
            finish_reason: response.finish_reason.clone(),
        }],
        usage: UsageSummary {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        },
    }
}

fn map_provider_error(error: &ProviderError, context: &RequestContext) -> ErrorEnvelope {
    let code = match error.kind {
        ProviderErrorKind::Auth | ProviderErrorKind::Unavailable => "provider_unavailable",
        ProviderErrorKind::InvalidRequest => "request_validation_failed",
        ProviderErrorKind::RateLimited => "rate_limited",
        ProviderErrorKind::Timeout => "upstream_timeout",
        ProviderErrorKind::Protocol => "upstream_protocol_error",
    };

    ErrorEnvelope {
        error: NormalizedError {
            code: code.to_string(),
            message: error.message.clone(),
            request_id: context.request_id.clone(),
            retryable: error.retryable,
            upstream_code: error.upstream_code.clone(),
            upstream_status_code: error.upstream_status_code,
            validation_issues: Vec::new(),
            details: error.details.clone(),
        },
    }
}

fn normalized_error(
    code: &str,
    message: String,
    context: &RequestContext,
    retryable: bool,
) -> ErrorEnvelope {
    ErrorEnvelope {
        error: NormalizedError {
            code: code.to_string(),
            message,
            request_id: context.request_id.clone(),
            retryable,
            upstream_code: None,
            upstream_status_code: None,
            validation_issues: Vec::new(),
            details: BTreeMap::new(),
        },
    }
}

fn validation_error(
    message: &str,
    validation_issues: Vec<ValidationIssue>,
    context: &RequestContext,
    retryable: bool,
) -> ErrorEnvelope {
    ErrorEnvelope {
        error: NormalizedError {
            code: "request_validation_failed".to_string(),
            message: message.to_string(),
            request_id: context.request_id.clone(),
            retryable,
            upstream_code: None,
            upstream_status_code: None,
            validation_issues,
            details: BTreeMap::new(),
        },
    }
}

fn next_request_context() -> RequestContext {
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);

    RequestContext {
        request_id: format!("req_{sequence}"),
        trace_id: format!("trace_{sequence}"),
        sequence,
    }
}

fn estimate_prompt_tokens(messages: &[ChatMessage]) -> u32 {
    messages
        .iter()
        .map(|message| {
            u32::try_from(message.content.split_whitespace().count())
                .unwrap_or(u32::MAX)
                .max(1)
                + 4
        })
        .sum()
}

fn unix_timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn status_for_error_code(code: &str) -> StatusCode {
    match code {
        "auth_invalid" => StatusCode::UNAUTHORIZED,
        "request_validation_failed" => StatusCode::BAD_REQUEST,
        "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
        "upstream_timeout" => StatusCode::GATEWAY_TIMEOUT,
        "upstream_protocol_error" => StatusCode::BAD_GATEWAY,
        "route_not_available" | "provider_unavailable" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn debug_headers_enabled_from_env() -> bool {
    std::env::var("GATEWAY_ENABLE_DEBUG_HEADERS")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
}

const fn admission_result_header_value(admission_result: AdmissionResult) -> &'static str {
    match admission_result {
        AdmissionResult::Admitted => "admitted",
        AdmissionResult::RejectedBudget => "rejected_budget",
        AdmissionResult::RejectedRateLimit => "rejected_rate_limit",
        AdmissionResult::RejectedConcurrency => "rejected_concurrency",
        AdmissionResult::RejectedPolicy => "rejected_policy",
        AdmissionResult::RejectedNoCandidate => "rejected_no_candidate",
    }
}

fn maybe_debug_headers(
    enabled: bool,
    route_policy_id: &RoutePolicyId,
    selected_target: Option<&str>,
    admission_result: AdmissionResult,
    fallback_count: usize,
) -> Option<GatewayDebugHeaders> {
    if !enabled {
        return None;
    }

    Some(GatewayDebugHeaders {
        selected_target: selected_target.map(ToString::to_string),
        route_policy_id: route_policy_id.as_str().to_string(),
        fallback_count,
        admission_result: admission_result_header_value(admission_result),
    })
}

trait ActiveConfigStore: Send + Sync {
    fn load(&self) -> Result<ActiveGatewayConfig, String>;
}

#[derive(Debug)]
struct StaticConfigStore {
    config: ActiveGatewayConfig,
}

impl StaticConfigStore {
    const fn new(config: ActiveGatewayConfig) -> Self {
        Self { config }
    }
}

impl ActiveConfigStore for StaticConfigStore {
    fn load(&self) -> Result<ActiveGatewayConfig, String> {
        Ok(self.config.clone())
    }
}

fn default_active_config() -> ActiveGatewayConfig {
    let tenant_id = TenantId::parse(DEFAULT_TENANT_ID).expect("tenant id should be valid");
    let project_id = ProjectId::parse(DEFAULT_PROJECT_ID).expect("project id should be valid");
    let provider_resource_id = ProviderResourceId::parse(DEFAULT_PROVIDER_RESOURCE_ID)
        .expect("provider resource id should be valid");

    let route_policy = RoutePolicy {
        route_policy_id: RoutePolicyId::parse(DEFAULT_ROUTE_POLICY_ID)
            .expect("route policy id should be valid"),
        tenant_id: tenant_id.clone(),
        display_name: "OpenAI chat default".to_string(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: vec!["json_mode".to_string()],
        preferred_regions: vec!["us-east-1".to_string()],
        version: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };

    let provider_resource = ProviderResource {
        provider_resource_id: provider_resource_id.clone(),
        tenant_id: tenant_id.clone(),
        project_id: Some(project_id.clone()),
        provider_id: "openai".to_string(),
        name: "openai-primary".to_string(),
        status: ProviderResourceStatus::Active,
        provenance_class: ProvenanceClass::OfficialApi,
        credential_owner_type: CredentialOwnerType::Platform,
        deployment_scope: DeploymentScope::Shared,
        region: std::env::var("GATEWAY_OPENAI_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        endpoint_base_url: std::env::var("GATEWAY_OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        auth_kind: AuthKind::ApiKey,
        health_state: HealthState::Healthy,
        budget_policy_id: None,
        capabilities: ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_json_mode: true,
        },
        version: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };

    ActiveGatewayConfig {
        config_snapshot: ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse(DEFAULT_CONFIG_SNAPSHOT_ID)
                .expect("config snapshot id should be valid"),
            tenant_id,
            project_id,
            revision: 1,
            status: ConfigSnapshotStatus::Active,
            activated_at: Some(now_rfc3339()),
            provider_resource_ids: vec![provider_resource_id],
            route_policy_id: route_policy.route_policy_id.clone(),
            budget_policy_id: core_domain::BudgetPolicyId::parse("budgetpol_default")
                .expect("budget policy id should be valid"),
        },
        route_policy,
        provider_targets: vec![ProviderTargetRuntime {
            resource: provider_resource,
            priority: 1,
            upstream_model: Some(
                std::env::var("GATEWAY_OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4.1-mini".to_string()),
            ),
            api_key: std::env::var("GATEWAY_OPENAI_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .unwrap_or_default(),
            static_latency_score: 0.9,
            static_cost_score: 0.65,
            usd_per_1k_tokens: 0.01,
        }],
    }
}

impl GatewayError {
    fn new(status: StatusCode, envelope: ErrorEnvelope, context: &RequestContext) -> Self {
        Self {
            status,
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
            route_receipt_id: None,
            config_snapshot_id: None,
            envelope,
            debug_headers: None,
        }
    }

    fn with_route_receipt(
        status: StatusCode,
        route_receipt: RouteReceipt,
        error: NormalizedError,
        context: &RequestContext,
        config_snapshot_id: Option<ConfigSnapshotId>,
        debug_headers: Option<GatewayDebugHeaders>,
    ) -> Self {
        Self {
            status,
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
            route_receipt_id: Some(route_receipt.route_receipt_id),
            config_snapshot_id,
            envelope: ErrorEnvelope { error },
            debug_headers,
        }
    }
}

impl IntoResponse for GatewaySuccess {
    fn into_response(self) -> Response {
        let mut response = Json(self.response).into_response();
        let headers = response.headers_mut();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        headers.insert(
            "x-request-id",
            HeaderValue::from_str(&self.request_id).expect("request id should be valid header"),
        );
        headers.insert(
            "x-trace-id",
            HeaderValue::from_str(&self.trace_id).expect("trace id should be valid header"),
        );
        headers.insert(
            "x-route-receipt-id",
            HeaderValue::from_str(self.route_receipt.route_receipt_id.as_str())
                .expect("route receipt id should be valid header"),
        );
        headers.insert(
            "x-config-snapshot-id",
            HeaderValue::from_str(self.config_snapshot_id.as_str())
                .expect("config snapshot id should be valid header"),
        );

        if let Some(debug_headers) = self.debug_headers {
            if let Some(selected_target) = debug_headers.selected_target {
                headers.insert(
                    "x-debug-selected-target",
                    HeaderValue::from_str(&selected_target)
                        .expect("selected target should be a valid header"),
                );
            }
            headers.insert(
                "x-debug-route-policy-id",
                HeaderValue::from_str(&debug_headers.route_policy_id)
                    .expect("route policy id should be a valid header"),
            );
            headers.insert(
                "x-debug-fallback-count",
                HeaderValue::from_str(&debug_headers.fallback_count.to_string())
                    .expect("fallback count should be a valid header"),
            );
            headers.insert(
                "x-debug-admission-result",
                HeaderValue::from_str(debug_headers.admission_result)
                    .expect("admission result should be a valid header"),
            );
        }

        response
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(self.envelope)).into_response();
        let headers = response.headers_mut();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        headers.insert(
            "x-request-id",
            HeaderValue::from_str(&self.request_id).expect("request id should be valid header"),
        );
        headers.insert(
            "x-trace-id",
            HeaderValue::from_str(&self.trace_id).expect("trace id should be valid header"),
        );

        if let Some(route_receipt_id) = self.route_receipt_id {
            headers.insert(
                "x-route-receipt-id",
                HeaderValue::from_str(route_receipt_id.as_str())
                    .expect("route receipt id should be valid header"),
            );
        }

        if let Some(config_snapshot_id) = self.config_snapshot_id {
            headers.insert(
                "x-config-snapshot-id",
                HeaderValue::from_str(config_snapshot_id.as_str())
                    .expect("config snapshot id should be valid header"),
            );
        }

        if let Some(debug_headers) = self.debug_headers {
            if let Some(selected_target) = debug_headers.selected_target {
                headers.insert(
                    "x-debug-selected-target",
                    HeaderValue::from_str(&selected_target)
                        .expect("selected target should be a valid header"),
                );
            }
            headers.insert(
                "x-debug-route-policy-id",
                HeaderValue::from_str(&debug_headers.route_policy_id)
                    .expect("route policy id should be a valid header"),
            );
            headers.insert(
                "x-debug-fallback-count",
                HeaderValue::from_str(&debug_headers.fallback_count.to_string())
                    .expect("fallback count should be a valid header"),
            );
            headers.insert(
                "x-debug-admission-result",
                HeaderValue::from_str(debug_headers.admission_result)
                    .expect("admission result should be a valid header"),
            );
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActiveGatewayConfig, AppState, ChatCompletionRequest, ChatMessage, GatewayState,
        ProviderTargetRuntime, StaticConfigStore, app_with_state, evaluate_route,
        normalize_request,
    };
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use core_domain::{
        AuthKind, BudgetPolicyId, ConfigSnapshot, ConfigSnapshotId, ConfigSnapshotStatus,
        CredentialOwnerType, DeploymentScope, HealthState, ProjectId, ProvenanceClass,
        ProviderCapabilities, ProviderResource, ProviderResourceId, ProviderResourceStatus,
        RoutePolicy, RoutePolicyId, TenantId,
    };
    use provider_traits::{
        AdapterManifest, ProviderAdapter, ProviderAdapterRegistry, ProviderError,
        ProviderErrorKind, ProviderExecutionContext, ProviderRequest, ProviderResponse,
        ProviderUsage, StreamingSupport,
    };
    use std::{collections::BTreeMap, sync::Arc};
    use tower::ServiceExt;

    fn valid_http_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "reasoning-fast".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "hello router".to_string(),
            }],
            stream: false,
        }
    }

    fn request_context() -> super::RequestContext {
        super::RequestContext {
            request_id: "req_test".to_string(),
            trace_id: "trace_test".to_string(),
            sequence: 42,
        }
    }

    fn build_config(targets: Vec<ProviderTargetRuntime>) -> ActiveGatewayConfig {
        let tenant_id = TenantId::parse("tenant_acme").unwrap();
        let project_id = ProjectId::parse("proj_core").unwrap();
        let route_policy = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_default").unwrap(),
            tenant_id: tenant_id.clone(),
            display_name: "default".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-east-1".to_string()],
            version: 1,
            created_at: "2026-04-20T00:00:00Z".to_string(),
            updated_at: "2026-04-20T00:00:00Z".to_string(),
        };

        ActiveGatewayConfig {
            config_snapshot: ConfigSnapshot {
                config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_test").unwrap(),
                tenant_id,
                project_id,
                revision: 1,
                status: ConfigSnapshotStatus::Active,
                activated_at: Some("2026-04-20T00:00:00Z".to_string()),
                provider_resource_ids: targets
                    .iter()
                    .map(|target| target.resource.provider_resource_id.clone())
                    .collect(),
                route_policy_id: route_policy.route_policy_id.clone(),
                budget_policy_id: BudgetPolicyId::parse("budgetpol_test").unwrap(),
            },
            route_policy,
            provider_targets: targets,
        }
    }

    fn build_target(
        provider_resource_id: &str,
        region: &str,
        latency: f32,
        cost: f32,
        health_state: HealthState,
    ) -> ProviderTargetRuntime {
        ProviderTargetRuntime {
            resource: ProviderResource {
                provider_resource_id: ProviderResourceId::parse(provider_resource_id).unwrap(),
                tenant_id: TenantId::parse("tenant_acme").unwrap(),
                project_id: Some(ProjectId::parse("proj_core").unwrap()),
                provider_id: "openai".to_string(),
                name: provider_resource_id.to_string(),
                status: ProviderResourceStatus::Active,
                provenance_class: ProvenanceClass::OfficialApi,
                credential_owner_type: CredentialOwnerType::Platform,
                deployment_scope: DeploymentScope::Shared,
                region: region.to_string(),
                endpoint_base_url: "https://api.openai.example/v1".to_string(),
                auth_kind: AuthKind::ApiKey,
                health_state,
                budget_policy_id: None,
                capabilities: ProviderCapabilities {
                    supports_streaming: true,
                    supports_tool_calling: true,
                    supports_json_mode: true,
                },
                version: 1,
                created_at: "2026-04-20T00:00:00Z".to_string(),
                updated_at: "2026-04-20T00:00:00Z".to_string(),
            },
            priority: 1,
            upstream_model: Some("gpt-4.1-mini".to_string()),
            api_key: "secret".to_string(),
            static_latency_score: latency,
            static_cost_score: cost,
            usd_per_1k_tokens: 0.01,
        }
    }

    fn build_target_with_provider(
        provider_resource_id: &str,
        provider_id: &str,
        region: &str,
        latency: f32,
        cost: f32,
        health_state: HealthState,
    ) -> ProviderTargetRuntime {
        let mut target = build_target(provider_resource_id, region, latency, cost, health_state);
        target.resource.provider_id = provider_id.to_string();
        target
    }

    fn test_state(
        adapter: Arc<dyn ProviderAdapter>,
        targets: Vec<ProviderTargetRuntime>,
    ) -> GatewayState {
        test_state_with_debug(adapter, targets, false)
    }

    fn test_state_with_debug(
        adapter: Arc<dyn ProviderAdapter>,
        targets: Vec<ProviderTargetRuntime>,
        debug_headers_enabled: bool,
    ) -> GatewayState {
        let mut registry = ProviderAdapterRegistry::new();
        registry.register(adapter).unwrap();

        Arc::new(AppState {
            config_store: Arc::new(StaticConfigStore::new(build_config(targets))),
            adapter_registry: registry,
            debug_headers_enabled,
        })
    }

    struct MockAdapter {
        outcomes: BTreeMap<String, Result<ProviderResponse, ProviderError>>,
    }

    #[async_trait::async_trait]
    impl ProviderAdapter for MockAdapter {
        fn manifest(&self) -> AdapterManifest {
            AdapterManifest {
                adapter_id: "mock-openai",
                provider_kind: "openai",
                display_name: "Mock OpenAI",
                protocol_family: "openai_chat",
                streaming_support: StreamingSupport::Unsupported,
            }
        }

        async fn execute_chat(
            &self,
            _request: &ProviderRequest,
            context: &ProviderExecutionContext,
        ) -> Result<ProviderResponse, ProviderError> {
            self.outcomes
                .get(&context.endpoint.provider_resource_id)
                .cloned()
                .expect("test outcome should exist")
        }
    }

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
    async fn returns_provider_failure_when_last_candidate_fails() {
        let adapter = Arc::new(MockAdapter {
            outcomes: BTreeMap::from([(
                "prvrsrc_openai_primary".to_string(),
                Err(ProviderError::new(
                    ProviderErrorKind::Unavailable,
                    "openai unavailable",
                    false,
                )
                .with_upstream_status(Some(503))
                .with_upstream_code(Some("server_error".to_string()))),
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
}
