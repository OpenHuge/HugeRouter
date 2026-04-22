mod openai;

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use core_domain::{
    AdmissionResult, ConfigSnapshot, ConfigSnapshotId, DeploymentScope, ErrorEnvelope,
    ExcludedTarget, FallbackTransition, HealthState, MonetaryAmount, NormalizedError,
    ProvenanceClass, ProviderCapabilities, ProviderResource, ProviderResourceStatus, RoutePolicy,
    RoutePolicyId, RouteReceipt, RouteReceiptId, ScoreBreakdown, ServiceName, UsageEvent,
    UsageEventId, UsageMetrics, UsagePhase, ValidationIssue,
};
use openai::OpenAiAdapter;
use protocol_anthropic::{
    AnthropicMessageRequest, AnthropicMessageResponse, AnthropicResponseContentBlock,
    AnthropicUsage, MappingError as AnthropicMappingError,
};
use protocol_gemini::{
    CompatibilityError as GeminiCompatibilityError, GenerateContentRequest,
    GenerateContentResponse, from_provider_response as gemini_from_provider_response,
};
use protocol_ir::{
    ConfigSnapshotResponse, MessageEnvelope, MessageType, ProviderResourcesResponse,
    RoutePoliciesResponse, UsageEventRecorded,
};
use provider_anthropic::AnthropicAdapter;
use provider_gemini::GeminiAdapter;
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
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::sync::{Mutex, OnceCell};
use tracing::{info, warn};

const GATEWAY_SERVICE_NAME: &str = "gateway-api";
const DEFAULT_CONTROL_PLANE_BASE_URL: &str = "http://127.0.0.1:8081";
const DEFAULT_CONTROL_PLANE_SNAPSHOT_REF: &str = "active";
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1_000);

pub type GatewayState = Arc<AppState>;

pub struct AppState {
    config_store: Arc<dyn ActiveConfigStore>,
    auth_store: Arc<dyn ApiKeyScopeStore>,
    adapter_registry: ProviderAdapterRegistry,
    debug_headers_enabled: bool,
    usage_event_sink: Arc<dyn UsageEventSink>,
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
struct ExecutionSuccess {
    sequence: u64,
    request_id: String,
    trace_id: String,
    config_snapshot_id: ConfigSnapshotId,
    route_receipt: RouteReceipt,
    usage_event: UsageEvent,
    provider_response: ProviderResponse,
    debug_headers: Option<GatewayDebugHeaders>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GatewayApiKeyScope {
    credential_id: String,
    tenant_id: String,
    project_id: Option<String>,
    status: String,
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
        .route("/v1/messages", post(anthropic_messages))
        .route(
            "/v1beta/models/{*model_action}",
            post(gemini_generate_content),
        )
        .with_state(state)
}

fn default_state() -> GatewayState {
    let config_store = Arc::new(ControlPlaneConfigStore::from_env());
    let auth_store = Arc::new(ControlPlaneApiKeyStore::from_env());
    let mut adapter_registry = ProviderAdapterRegistry::new();
    adapter_registry
        .register(Arc::new(OpenAiAdapter::default()))
        .expect("openai adapter registration should succeed");
    adapter_registry
        .register(Arc::new(AnthropicAdapter::default()))
        .expect("anthropic adapter registration should succeed");
    adapter_registry
        .register(Arc::new(GeminiAdapter::default()))
        .expect("gemini adapter registration should succeed");

    Arc::new(AppState {
        config_store,
        auth_store,
        adapter_registry,
        debug_headers_enabled: debug_headers_enabled_from_env(),
        usage_event_sink: Arc::new(NatsUsageEventSink::from_env()),
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

async fn anthropic_messages(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<AnthropicMessageRequest>,
) -> Response {
    let context = next_request_context();
    let normalized_request = match normalize_anthropic_request(request, &context) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };

    match process_normalized_request(state, headers.get(AUTHORIZATION), normalized_request).await {
        Ok(success) => anthropic_success_response(&success),
        Err(error) => error.into_response(),
    }
}

async fn gemini_generate_content(
    State(state): State<GatewayState>,
    Path(model_action): Path<String>,
    headers: HeaderMap,
    Json(mut request): Json<GenerateContentRequest>,
) -> Response {
    let context = next_request_context();
    let model = match parse_gemini_model_action(&model_action, &context) {
        Ok(model) => model,
        Err(error) => return error.into_response(),
    };
    request.model = model;
    let normalized_request = match normalize_gemini_request(request, &context) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };

    match process_normalized_request(state, headers.get(AUTHORIZATION), normalized_request).await {
        Ok(success) => gemini_success_response(&success),
        Err(error) => error.into_response(),
    }
}

#[allow(clippy::result_large_err)]
fn parse_gemini_model_action(
    model_action: &str,
    context: &RequestContext,
) -> Result<String, GatewayError> {
    let Some(model) = model_action.strip_suffix(":generateContent") else {
        return Err(GatewayError::new(
            StatusCode::BAD_REQUEST,
            normalized_error(
                "request_validation_failed",
                "Gemini path must end with :generateContent".to_string(),
                context,
                false,
            ),
            context,
        ));
    };

    if model.trim().is_empty() {
        return Err(GatewayError::new(
            StatusCode::BAD_REQUEST,
            normalized_error(
                "request_validation_failed",
                "Gemini path model must not be empty".to_string(),
                context,
                false,
            ),
            context,
        ));
    }

    Ok(model.to_string())
}

async fn process_chat_completion(
    state: GatewayState,
    authorization_header: Option<&HeaderValue>,
    request: ChatCompletionRequest,
) -> Result<GatewaySuccess, GatewayError> {
    let normalized_request = normalize_request(request, &next_request_context())?;
    let success =
        process_normalized_request(state, authorization_header, normalized_request.clone()).await?;

    let context = RequestContext {
        request_id: success.request_id.clone(),
        trace_id: success.trace_id.clone(),
        sequence: success.sequence,
    };

    Ok(GatewaySuccess {
        request_id: success.request_id,
        trace_id: success.trace_id,
        config_snapshot_id: success.config_snapshot_id,
        route_receipt: success.route_receipt,
        usage_event: success.usage_event,
        response: map_provider_response(&context, &normalized_request, &success.provider_response),
        debug_headers: success.debug_headers,
    })
}

async fn process_normalized_request(
    state: GatewayState,
    authorization_header: Option<&HeaderValue>,
    normalized_request: NormalizedChatRequest,
) -> Result<ExecutionSuccess, GatewayError> {
    let context = next_request_context();
    let bearer_token = extract_bearer_token(authorization_header, &context)?;
    let api_key_scope = state
        .auth_store
        .resolve(&bearer_token)
        .await
        .map_err(|message| {
            GatewayError::new(
                StatusCode::UNAUTHORIZED,
                normalized_error("auth_invalid", message, &context, false),
                &context,
            )
        })?;

    let active_config = state.config_store.load().await.map_err(|error| {
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
    ensure_scope_matches_config(&api_key_scope, &active_config, &context)?;
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
) -> Result<ExecutionSuccess, GatewayError> {
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
                state
                    .usage_event_sink
                    .publish(&route_receipt, &usage_event, &context)
                    .await
                    .map_err(|message| {
                        GatewayError::with_route_receipt(
                            StatusCode::SERVICE_UNAVAILABLE,
                            route_receipt.clone(),
                            normalized_error("usage_event_publish_failed", message, &context, true)
                                .error,
                            &context,
                            Some(route.config_snapshot.config_snapshot_id.clone()),
                            maybe_debug_headers(
                                state.debug_headers_enabled,
                                &route.config_snapshot.route_policy_id,
                                Some(target.resource.provider_resource_id.as_str()),
                                AdmissionResult::Admitted,
                                fallback_count,
                            ),
                        )
                    })?;

                info!(
                    request_id = context.request_id,
                    trace_id = context.trace_id,
                    route_receipt_id = %route_receipt.route_receipt_id,
                    provider_resource_id = %target.resource.provider_resource_id,
                    "gateway request routed successfully"
                );

                return Ok(ExecutionSuccess {
                    sequence: context.sequence,
                    request_id: context.request_id.clone(),
                    trace_id: context.trace_id.clone(),
                    config_snapshot_id: route.config_snapshot.config_snapshot_id.clone(),
                    route_receipt,
                    usage_event,
                    provider_response,
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
fn extract_bearer_token(
    authorization_header: Option<&HeaderValue>,
    context: &RequestContext,
) -> Result<String, GatewayError> {
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

    let header = value.to_str().map_err(|_| {
        GatewayError::new(
            StatusCode::UNAUTHORIZED,
            normalized_error(
                "auth_invalid",
                "Authorization header must be valid UTF-8".to_string(),
                context,
                false,
            ),
            context,
        )
    })?;

    let Some(bearer_token) = header.strip_prefix("Bearer ") else {
        return Err(GatewayError::new(
            StatusCode::UNAUTHORIZED,
            normalized_error(
                "auth_invalid",
                "Authorization header must use Bearer credentials".to_string(),
                context,
                false,
            ),
            context,
        ));
    };

    if bearer_token.trim().is_empty() {
        return Err(GatewayError::new(
            StatusCode::UNAUTHORIZED,
            normalized_error(
                "auth_invalid",
                "Bearer token must not be empty".to_string(),
                context,
                false,
            ),
            context,
        ));
    }

    Ok(bearer_token.to_string())
}

#[allow(clippy::result_large_err)]
fn ensure_scope_matches_config(
    api_key_scope: &GatewayApiKeyScope,
    active_config: &ActiveGatewayConfig,
    context: &RequestContext,
) -> Result<(), GatewayError> {
    if api_key_scope.status != "active" {
        return Err(GatewayError::new(
            StatusCode::UNAUTHORIZED,
            normalized_error(
                "auth_invalid",
                "API key is not active".to_string(),
                context,
                false,
            ),
            context,
        ));
    }

    if api_key_scope.tenant_id != active_config.config_snapshot.tenant_id.as_str() {
        return Err(GatewayError::new(
            StatusCode::FORBIDDEN,
            normalized_error(
                "auth_forbidden",
                "API key tenant scope does not match the active gateway configuration".to_string(),
                context,
                false,
            ),
            context,
        ));
    }

    if let Some(project_id) = api_key_scope.project_id.as_deref()
        && project_id != active_config.config_snapshot.project_id.as_str()
    {
        return Err(GatewayError::new(
            StatusCode::FORBIDDEN,
            normalized_error(
                "auth_forbidden",
                "API key project scope does not match the active gateway configuration".to_string(),
                context,
                false,
            ),
            context,
        ));
    }

    Ok(())
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

#[allow(clippy::result_large_err)]
fn normalize_anthropic_request(
    request: AnthropicMessageRequest,
    context: &RequestContext,
) -> Result<NormalizedChatRequest, GatewayError> {
    let normalized = protocol_anthropic::normalize_request(request)
        .map_err(|error| anthropic_mapping_error(&error, context))?;
    let mut messages = Vec::with_capacity(
        normalized.messages.len() + usize::from(normalized.system_prompt.is_some()),
    );

    if let Some(system_prompt) = normalized.system_prompt.as_ref() {
        messages.push(ProviderMessage {
            role: "system".to_string(),
            content: system_prompt.clone(),
        });
    }

    messages.extend(normalized.messages.iter().map(|message| ProviderMessage {
        role: message.role.clone(),
        content: message.content.clone(),
    }));

    Ok(NormalizedChatRequest {
        model_alias: normalized.model_alias.clone(),
        protocol_family: "anthropic_messages".to_string(),
        estimated_prompt_tokens: protocol_anthropic::estimate_prompt_tokens(&normalized),
        messages,
    })
}

#[allow(clippy::result_large_err, clippy::needless_pass_by_value)]
fn normalize_gemini_request(
    request: GenerateContentRequest,
    context: &RequestContext,
) -> Result<NormalizedChatRequest, GatewayError> {
    let provider_request = protocol_gemini::to_provider_request(&request)
        .map_err(|error| gemini_mapping_error(error, context))?;

    Ok(NormalizedChatRequest {
        model_alias: provider_request.model,
        protocol_family: "gemini_generate_content".to_string(),
        estimated_prompt_tokens: estimate_provider_messages_tokens(&provider_request.messages),
        messages: provider_request.messages,
    })
}

fn anthropic_mapping_error(
    error: &AnthropicMappingError,
    context: &RequestContext,
) -> GatewayError {
    GatewayError::new(
        StatusCode::BAD_REQUEST,
        validation_error(
            "anthropic request validation failed",
            vec![ValidationIssue {
                field: anthropic_mapping_error_field(error),
                message: error.to_string(),
            }],
            context,
            false,
        ),
        context,
    )
}

fn anthropic_mapping_error_field(error: &AnthropicMappingError) -> String {
    match error {
        AnthropicMappingError::EmptyModel => "model".to_string(),
        AnthropicMappingError::EmptyMessages => "messages".to_string(),
        AnthropicMappingError::StreamingNotSupported => "stream".to_string(),
        AnthropicMappingError::EmptyRole { index }
        | AnthropicMappingError::UnsupportedRole { index, .. } => {
            format!("messages[{index}].role")
        }
        AnthropicMappingError::EmptyContent { index }
        | AnthropicMappingError::UnsupportedContentBlock { index }
        | AnthropicMappingError::UnsupportedContentTextStructure { index } => {
            format!("messages[{index}].content")
        }
        AnthropicMappingError::ResponseHasNoText => "content".to_string(),
    }
}

fn gemini_mapping_error(error: GeminiCompatibilityError, context: &RequestContext) -> GatewayError {
    GatewayError::new(
        StatusCode::BAD_REQUEST,
        validation_error(
            "gemini request validation failed",
            vec![ValidationIssue {
                field: error.field.to_string(),
                message: error.detail,
            }],
            context,
            false,
        ),
        context,
    )
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

fn estimate_provider_messages_tokens(messages: &[ProviderMessage]) -> u32 {
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
        "auth_forbidden" => StatusCode::FORBIDDEN,
        "request_validation_failed" => StatusCode::BAD_REQUEST,
        "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
        "upstream_timeout" => StatusCode::GATEWAY_TIMEOUT,
        "upstream_protocol_error" => StatusCode::BAD_GATEWAY,
        "route_not_available" | "provider_unavailable" | "usage_event_publish_failed" => {
            StatusCode::SERVICE_UNAVAILABLE
        }
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

fn anthropic_success_response(success: &ExecutionSuccess) -> Response {
    let payload = AnthropicMessageResponse {
        id: success
            .provider_response
            .response_id
            .clone()
            .or_else(|| Some(format!("msg_{}", success.sequence))),
        model: Some(success.provider_response.model.clone()),
        content: vec![AnthropicResponseContentBlock::Text {
            text: success.provider_response.output_text.clone(),
        }],
        stop_reason: Some(success.provider_response.finish_reason.clone()),
        usage: Some(AnthropicUsage {
            input_tokens: success.provider_response.usage.input_tokens,
            output_tokens: success.provider_response.usage.output_tokens,
        }),
    };
    let mut response = Json(payload).into_response();
    insert_success_headers(
        &mut response,
        &success.request_id,
        &success.trace_id,
        &success.route_receipt,
        &success.config_snapshot_id,
        success.debug_headers.as_ref(),
    );
    response
}

fn gemini_success_response(success: &ExecutionSuccess) -> Response {
    let payload: GenerateContentResponse =
        gemini_from_provider_response(&success.provider_response);
    let mut response = Json(payload).into_response();
    insert_success_headers(
        &mut response,
        &success.request_id,
        &success.trace_id,
        &success.route_receipt,
        &success.config_snapshot_id,
        success.debug_headers.as_ref(),
    );
    response
}

fn insert_success_headers(
    response: &mut Response,
    request_id: &str,
    trace_id: &str,
    route_receipt: &RouteReceipt,
    config_snapshot_id: &ConfigSnapshotId,
    debug_headers: Option<&GatewayDebugHeaders>,
) {
    let headers = response.headers_mut();

    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("request id should be valid header"),
    );
    headers.insert(
        "x-trace-id",
        HeaderValue::from_str(trace_id).expect("trace id should be valid header"),
    );
    headers.insert(
        "x-route-receipt-id",
        HeaderValue::from_str(route_receipt.route_receipt_id.as_str())
            .expect("route receipt id should be valid header"),
    );
    headers.insert(
        "x-config-snapshot-id",
        HeaderValue::from_str(config_snapshot_id.as_str())
            .expect("config snapshot id should be valid header"),
    );

    if let Some(debug_headers) = debug_headers {
        if let Some(selected_target) = &debug_headers.selected_target {
            headers.insert(
                "x-debug-selected-target",
                HeaderValue::from_str(selected_target)
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
}

#[async_trait]
trait ActiveConfigStore: Send + Sync {
    async fn load(&self) -> Result<ActiveGatewayConfig, String>;
}

#[async_trait]
trait ApiKeyScopeStore: Send + Sync {
    async fn resolve(&self, api_key: &str) -> Result<GatewayApiKeyScope, String>;
}

#[async_trait]
trait UsageEventSink: Send + Sync {
    async fn publish(
        &self,
        route_receipt: &RouteReceipt,
        usage_event: &UsageEvent,
        context: &RequestContext,
    ) -> Result<(), String>;
}

#[cfg(test)]
#[derive(Debug)]
struct StaticConfigStore {
    config: ActiveGatewayConfig,
}

#[cfg(test)]
impl StaticConfigStore {
    const fn new(config: ActiveGatewayConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
#[async_trait]
impl ActiveConfigStore for StaticConfigStore {
    async fn load(&self) -> Result<ActiveGatewayConfig, String> {
        Ok(self.config.clone())
    }
}

#[derive(Debug, Clone)]
struct ControlPlaneConfigStore {
    base_url: String,
    cache: Arc<Mutex<Option<CachedActiveConfig>>>,
    cache_ttl: Duration,
    client: reqwest::Client,
    snapshot_ref: String,
}

#[derive(Debug, Clone)]
struct CachedActiveConfig {
    config: ActiveGatewayConfig,
    fetched_at: Instant,
}

#[derive(Debug, Clone)]
struct ControlPlaneApiKeyStore {
    base_url: String,
    client: reqwest::Client,
    internal_token: Option<String>,
    resolve_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GatewayApiKeyResolveRequest {
    api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GatewayApiKeyResolveResponse {
    api_key: GatewayApiKeyScope,
}

#[derive(Debug)]
struct NatsUsageEventSink {
    client: OnceCell<async_nats::Client>,
    subject: String,
    url: String,
}

impl ControlPlaneConfigStore {
    fn from_env() -> Self {
        let base_url = std::env::var("CONTROL_PLANE_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_CONTROL_PLANE_BASE_URL.to_string());
        let snapshot_ref = std::env::var("GATEWAY_CONTROL_PLANE_SNAPSHOT_REF")
            .unwrap_or_else(|_| DEFAULT_CONTROL_PLANE_SNAPSHOT_REF.to_string());
        let cache_ttl = std::env::var("GATEWAY_CONFIG_CACHE_TTL_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map_or_else(|| Duration::from_secs(5), Duration::from_millis);

        Self::new(base_url, snapshot_ref, cache_ttl, reqwest::Client::new())
    }

    fn new(
        base_url: impl Into<String>,
        snapshot_ref: impl Into<String>,
        cache_ttl: Duration,
        client: reqwest::Client,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            cache: Arc::new(Mutex::new(None)),
            cache_ttl,
            client,
            snapshot_ref: snapshot_ref.into(),
        }
    }

    async fn fetch_active_config(&self) -> Result<ActiveGatewayConfig, String> {
        let snapshot = self
            .get_json::<ConfigSnapshotResponse>(&format!(
                "/v1/config-snapshots/{}",
                self.snapshot_ref
            ))
            .await?
            .config_snapshot;
        let route_policies = self
            .get_json::<RoutePoliciesResponse>("/v1/route-policies")
            .await?
            .data;
        let provider_resources = self
            .get_json::<ProviderResourcesResponse>("/v1/provider-resources")
            .await?
            .data;

        let route_policy = route_policies
            .into_iter()
            .find(|policy| policy.route_policy_id == snapshot.route_policy_id)
            .ok_or_else(|| {
                format!(
                    "route policy {route_policy_id} is missing from control plane",
                    route_policy_id = snapshot.route_policy_id
                )
            })?;

        let provider_targets = snapshot
            .provider_resource_ids
            .iter()
            .enumerate()
            .map(|(index, provider_resource_id)| {
                let resource = provider_resources
                    .iter()
                    .find(|candidate| &candidate.provider_resource_id == provider_resource_id)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "provider resource {provider_resource_id} is missing from control plane"
                        )
                    })?;

                Ok(ProviderTargetRuntime {
                    priority: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    upstream_model: upstream_model_for_provider(&resource.provider_id),
                    api_key: api_key_for_provider(&resource.provider_id),
                    static_latency_score: static_latency_score_for_region(
                        &route_policy.preferred_regions,
                        &resource.region,
                    ),
                    static_cost_score: static_cost_score_for_scope(resource.deployment_scope),
                    usd_per_1k_tokens: usd_per_1k_tokens_for_provider(&resource.provider_id),
                    resource,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(ActiveGatewayConfig {
            config_snapshot: snapshot,
            route_policy,
            provider_targets,
        })
    }

    async fn get_json<T>(&self, path: &str) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let base = self.base_url.trim_end_matches('/');
        let url = format!("{base}{path}");
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|error| format!("failed to fetch {url}: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read response body".to_string());
            return Err(format!(
                "control plane returned HTTP {status} for {url}: {body}"
            ));
        }
        response
            .json::<T>()
            .await
            .map_err(|error| format!("failed to decode control-plane payload from {url}: {error}"))
    }
}

impl ControlPlaneApiKeyStore {
    fn from_env() -> Self {
        let base_url = std::env::var("CONTROL_PLANE_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_CONTROL_PLANE_BASE_URL.to_string());
        let internal_token = std::env::var("CONTROL_PLANE_INTERNAL_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let resolve_path = std::env::var("GATEWAY_API_KEY_RESOLVE_PATH")
            .unwrap_or_else(|_| "/internal/gateway/api-keys/resolve".to_string());

        Self::new(
            base_url,
            resolve_path,
            internal_token,
            reqwest::Client::new(),
        )
    }

    fn new(
        base_url: impl Into<String>,
        resolve_path: impl Into<String>,
        internal_token: Option<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            client,
            internal_token,
            resolve_path: resolve_path.into(),
        }
    }

    async fn fetch_scope(&self, api_key: &str) -> Result<GatewayApiKeyScope, String> {
        let internal_token = self.internal_token.as_ref().ok_or_else(|| {
            "CONTROL_PLANE_INTERNAL_TOKEN must be configured for gateway auth resolution"
                .to_string()
        })?;
        let base = self.base_url.trim_end_matches('/');
        let path = if self.resolve_path.starts_with('/') {
            self.resolve_path.clone()
        } else {
            format!("/{}", self.resolve_path)
        };
        let url = format!("{base}{path}");
        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {internal_token}"))
            .json(&GatewayApiKeyResolveRequest {
                api_key: api_key.to_string(),
            })
            .send()
            .await
            .map_err(|error| format!("failed to resolve API key via control plane: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read response body".to_string());
            return Err(format!(
                "control plane rejected API key with HTTP {status}: {body}"
            ));
        }

        response
            .json::<GatewayApiKeyResolveResponse>()
            .await
            .map(|payload| payload.api_key)
            .map_err(|error| format!("failed to decode API key resolution payload: {error}"))
    }
}

#[async_trait]
impl ApiKeyScopeStore for ControlPlaneApiKeyStore {
    async fn resolve(&self, api_key: &str) -> Result<GatewayApiKeyScope, String> {
        self.fetch_scope(api_key).await
    }
}

impl NatsUsageEventSink {
    fn from_env() -> Self {
        let url = std::env::var("GATEWAY_NATS_URL")
            .or_else(|_| std::env::var("NATS_URL"))
            .unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
        let subject = std::env::var("GATEWAY_USAGE_EVENT_SUBJECT")
            .unwrap_or_else(|_| "events.usage_event.recorded".to_string());

        Self::new(url, subject)
    }

    fn new(url: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            client: OnceCell::new(),
            subject: subject.into(),
            url: url.into(),
        }
    }

    async fn client(&self) -> Result<&async_nats::Client, String> {
        self.client
            .get_or_try_init(|| async {
                async_nats::connect(self.url.as_str())
                    .await
                    .map_err(|error| format!("failed to connect to NATS at {}: {error}", self.url))
            })
            .await
    }
}

#[async_trait]
impl UsageEventSink for NatsUsageEventSink {
    async fn publish(
        &self,
        _route_receipt: &RouteReceipt,
        usage_event: &UsageEvent,
        context: &RequestContext,
    ) -> Result<(), String> {
        let envelope = MessageEnvelope::new(
            format!("msg_{}", usage_event.usage_event_id),
            MessageType::UsageEventRecorded,
            usage_event.recorded_at.clone(),
            ServiceName::parse(GATEWAY_SERVICE_NAME).expect("gateway service name should be valid"),
            usage_event.idempotency_key.clone(),
            UsageEventRecorded {
                usage_event: usage_event.clone(),
            },
        )
        .with_request_context(context.trace_id.clone(), context.request_id.clone());
        let payload = serde_json::to_vec(&envelope)
            .map_err(|error| format!("failed to serialize usage event envelope: {error}"))?;
        let client = self.client().await?;

        client
            .publish(self.subject.clone(), payload.into())
            .await
            .map_err(|error| format!("failed to publish usage event: {error}"))?;
        client
            .flush()
            .await
            .map_err(|error| format!("failed to flush usage event publish: {error}"))?;
        Ok(())
    }
}

#[async_trait]
impl ActiveConfigStore for ControlPlaneConfigStore {
    async fn load(&self) -> Result<ActiveGatewayConfig, String> {
        {
            let cache = self.cache.lock().await;
            if let Some(entry) = cache.as_ref()
                && entry.fetched_at.elapsed() <= self.cache_ttl
            {
                return Ok(entry.config.clone());
            }
        }

        let fetched = self.fetch_active_config().await?;
        *self.cache.lock().await = Some(CachedActiveConfig {
            config: fetched.clone(),
            fetched_at: Instant::now(),
        });
        Ok(fetched)
    }
}

fn api_key_for_provider(provider_id: &str) -> String {
    match provider_id {
        "openai" => std::env::var("GATEWAY_OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .unwrap_or_default(),
        "anthropic" => std::env::var("GATEWAY_ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .unwrap_or_default(),
        "gemini" => std::env::var("GATEWAY_GEMINI_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn upstream_model_for_provider(provider_id: &str) -> Option<String> {
    match provider_id {
        "openai" => Some(
            std::env::var("GATEWAY_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string()),
        ),
        "anthropic" => Some(
            std::env::var("GATEWAY_ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-5-sonnet-latest".to_string()),
        ),
        "gemini" => Some(
            std::env::var("GATEWAY_GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-1.5-flash-latest".to_string()),
        ),
        _ => None,
    }
}

fn usd_per_1k_tokens_for_provider(provider_id: &str) -> f64 {
    match provider_id {
        "openai" => std::env::var("GATEWAY_OPENAI_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.01),
        "anthropic" => std::env::var("GATEWAY_ANTHROPIC_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.012),
        "gemini" => std::env::var("GATEWAY_GEMINI_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.008),
        _ => 0.02,
    }
}

fn static_latency_score_for_region(preferred_regions: &[String], region: &str) -> f32 {
    if preferred_regions
        .iter()
        .any(|preferred| preferred == region)
    {
        0.95
    } else {
        0.8
    }
}

const fn static_cost_score_for_scope(scope: DeploymentScope) -> f32 {
    match scope {
        DeploymentScope::Shared => 0.7,
        DeploymentScope::TenantDedicated => 0.6,
        DeploymentScope::ProjectDedicated => 0.5,
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
        insert_success_headers(
            &mut response,
            &self.request_id,
            &self.trace_id,
            &self.route_receipt,
            &self.config_snapshot_id,
            self.debug_headers.as_ref(),
        );

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
        ActiveConfigStore, ActiveGatewayConfig, ApiKeyScopeStore, AppState, ChatCompletionRequest,
        ChatMessage, ControlPlaneApiKeyStore, ControlPlaneConfigStore, GatewayApiKeyResolveRequest,
        GatewayApiKeyResolveResponse, GatewayApiKeyScope, GatewayState, ProviderTargetRuntime,
        RequestContext, StaticConfigStore, UsageEventSink, app_with_state, evaluate_route,
        normalize_request,
    };
    use axum::{
        Json, Router,
        body::{Body, to_bytes},
        extract::Json as ExtractJson,
        extract::State,
        http::{Request, StatusCode},
        routing::{get, post},
    };
    use core_domain::{
        AuthKind, BudgetPolicyId, ConfigSnapshot, ConfigSnapshotId, ConfigSnapshotStatus,
        CredentialOwnerType, DeploymentScope, HealthState, ProjectId, ProvenanceClass,
        ProviderCapabilities, ProviderResource, ProviderResourceId, ProviderResourceStatus,
        RoutePolicy, RoutePolicyId, RouteReceipt, TenantId, UsageEvent,
    };
    use protocol_ir::{ConfigSnapshotResponse, ProviderResourcesResponse, RoutePoliciesResponse};
    use provider_traits::{
        AdapterManifest, ProviderAdapter, ProviderAdapterRegistry, ProviderError,
        ProviderErrorKind, ProviderExecutionContext, ProviderRequest, ProviderResponse,
        ProviderUsage, StreamingSupport,
    };
    use std::{
        collections::BTreeMap,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering as AtomicOrdering},
        },
        time::Duration,
    };
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
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

    #[derive(Debug)]
    struct StaticApiKeyScopeStore {
        scope: GatewayApiKeyScope,
    }

    impl StaticApiKeyScopeStore {
        fn matching_config() -> Self {
            Self {
                scope: GatewayApiKeyScope {
                    credential_id: "cred_gateway_test".to_string(),
                    tenant_id: "tenant_acme".to_string(),
                    project_id: Some("proj_core".to_string()),
                    status: "active".to_string(),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl ApiKeyScopeStore for StaticApiKeyScopeStore {
        async fn resolve(&self, _api_key: &str) -> Result<GatewayApiKeyScope, String> {
            Ok(self.scope.clone())
        }
    }

    #[derive(Default)]
    struct RecordingUsageEventSink {
        publish_error: Option<String>,
        published_events: Arc<Mutex<Vec<UsageEvent>>>,
    }

    #[async_trait::async_trait]
    impl UsageEventSink for RecordingUsageEventSink {
        async fn publish(
            &self,
            _route_receipt: &RouteReceipt,
            usage_event: &UsageEvent,
            _context: &RequestContext,
        ) -> Result<(), String> {
            if let Some(error) = &self.publish_error {
                return Err(error.clone());
            }

            self.published_events.lock().await.push(usage_event.clone());
            Ok(())
        }
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
            auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
            adapter_registry: registry,
            debug_headers_enabled,
            usage_event_sink: Arc::new(RecordingUsageEventSink::default()),
        })
    }

    struct MockAdapter {
        outcomes: BTreeMap<String, Result<ProviderResponse, ProviderError>>,
    }

    struct ProtocolMockAdapter {
        provider_kind: &'static str,
        protocol_family: &'static str,
        outcomes: BTreeMap<String, Result<ProviderResponse, ProviderError>>,
    }

    #[derive(Clone)]
    struct ControlPlaneFixture {
        config_snapshot: ConfigSnapshotResponse,
        provider_resources: ProviderResourcesResponse,
        route_policies: RoutePoliciesResponse,
    }

    #[derive(Clone)]
    struct FixtureState {
        fail: bool,
        fixture: ControlPlaneFixture,
        request_count: Arc<AtomicUsize>,
    }

    async fn control_plane_api_key_resolution(
        State(state): State<FixtureState>,
        ExtractJson(request): ExtractJson<GatewayApiKeyResolveRequest>,
    ) -> Result<Json<GatewayApiKeyResolveResponse>, StatusCode> {
        state.request_count.fetch_add(1, AtomicOrdering::Relaxed);
        if state.fail || request.api_key != "test" {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(Json(GatewayApiKeyResolveResponse {
            api_key: GatewayApiKeyScope {
                credential_id: "cred_gateway_test".to_string(),
                tenant_id: "tenant_acme".to_string(),
                project_id: Some("proj_core".to_string()),
                status: "active".to_string(),
            },
        }))
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

    #[async_trait::async_trait]
    impl ProviderAdapter for ProtocolMockAdapter {
        fn manifest(&self) -> AdapterManifest {
            AdapterManifest {
                adapter_id: "mock-protocol",
                provider_kind: self.provider_kind,
                display_name: "Mock Protocol Adapter",
                protocol_family: self.protocol_family,
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

    fn control_plane_fixture() -> ControlPlaneFixture {
        let config = build_config(vec![
            build_target(
                "prvrsrc_openai_primary",
                "us-east-1",
                0.9,
                0.6,
                HealthState::Healthy,
            ),
            build_target(
                "prvrsrc_openai_backup",
                "us-west-2",
                0.85,
                0.7,
                HealthState::Healthy,
            ),
        ]);

        ControlPlaneFixture {
            config_snapshot: ConfigSnapshotResponse {
                config_snapshot: config.config_snapshot,
            },
            provider_resources: ProviderResourcesResponse {
                data: config
                    .provider_targets
                    .iter()
                    .map(|target| target.resource.clone())
                    .collect(),
            },
            route_policies: RoutePoliciesResponse {
                data: vec![config.route_policy],
            },
        }
    }

    async fn control_plane_snapshot(
        State(state): State<FixtureState>,
    ) -> Result<Json<ConfigSnapshotResponse>, StatusCode> {
        state.request_count.fetch_add(1, AtomicOrdering::Relaxed);
        if state.fail {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        Ok(Json(state.fixture.config_snapshot))
    }

    async fn control_plane_route_policies(
        State(state): State<FixtureState>,
    ) -> Result<Json<RoutePoliciesResponse>, StatusCode> {
        state.request_count.fetch_add(1, AtomicOrdering::Relaxed);
        if state.fail {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        Ok(Json(state.fixture.route_policies))
    }

    async fn control_plane_provider_resources(
        State(state): State<FixtureState>,
    ) -> Result<Json<ProviderResourcesResponse>, StatusCode> {
        state.request_count.fetch_add(1, AtomicOrdering::Relaxed);
        if state.fail {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        Ok(Json(state.fixture.provider_resources))
    }

    async fn spawn_control_plane_server(
        fail: bool,
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let request_count = Arc::new(AtomicUsize::new(0));
        let app = Router::new()
            .route("/v1/config-snapshots/active", get(control_plane_snapshot))
            .route("/v1/route-policies", get(control_plane_route_policies))
            .route(
                "/v1/provider-resources",
                get(control_plane_provider_resources),
            )
            .route(
                "/internal/gateway/api-keys/resolve",
                post(control_plane_api_key_resolution),
            )
            .with_state(FixtureState {
                fail,
                fixture: control_plane_fixture(),
                request_count: request_count.clone(),
            });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{address}"), request_count, handle)
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
    async fn control_plane_config_store_fetches_and_maps_active_config() {
        let (base_url, request_count, handle) = spawn_control_plane_server(false).await;
        let store = ControlPlaneConfigStore::new(
            base_url,
            "active",
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
        assert_eq!(request_count.load(AtomicOrdering::Relaxed), 3);

        handle.abort();
    }

    #[tokio::test]
    async fn control_plane_config_store_uses_ttl_cache() {
        let (base_url, request_count, handle) = spawn_control_plane_server(false).await;
        let store = ControlPlaneConfigStore::new(
            base_url,
            "active",
            Duration::from_secs(60),
            reqwest::Client::new(),
        );

        let _ = store.load().await.unwrap();
        let _ = store.load().await.unwrap();

        assert_eq!(request_count.load(AtomicOrdering::Relaxed), 3);
        handle.abort();
    }

    #[tokio::test]
    async fn control_plane_config_store_reports_unavailable_backend() {
        let (base_url, _request_count, handle) = spawn_control_plane_server(true).await;
        let store = ControlPlaneConfigStore::new(
            base_url,
            "active",
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
            adapter_registry: registry,
            debug_headers_enabled: false,
            usage_event_sink: Arc::new(RecordingUsageEventSink::default()),
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
            adapter_registry: registry,
            debug_headers_enabled: false,
            usage_event_sink: Arc::new(RecordingUsageEventSink::default()),
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
            adapter_registry: registry,
            debug_headers_enabled: false,
            usage_event_sink: Arc::new(RecordingUsageEventSink {
                publish_error: Some("nats unavailable".to_string()),
                published_events: Arc::new(Mutex::new(Vec::new())),
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
            adapter_registry: registry,
            debug_headers_enabled: false,
            usage_event_sink: Arc::new(RecordingUsageEventSink::default()),
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
            adapter_registry: registry,
            debug_headers_enabled: false,
            usage_event_sink: Arc::new(RecordingUsageEventSink::default()),
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
