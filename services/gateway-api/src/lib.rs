mod composition;
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
    ProvenanceClass, ProviderResource, ProviderResourceStatus, RoutePolicy, RoutePolicyId,
    RouteReceipt, RouteReceiptId, ScoreBreakdown, ServiceName, UsageEvent, UsageEventId,
    UsageMetrics, UsagePhase, ValidationIssue,
};
use protocol_anthropic::{
    AnthropicMessageRequest, AnthropicMessageResponse, AnthropicResponseContentBlock,
    AnthropicUsage, MappingError as AnthropicMappingError,
};
use protocol_gemini::{
    CompatibilityError as GeminiCompatibilityError, GenerateContentRequest,
    GenerateContentResponse, from_provider_response as gemini_from_provider_response,
};
use protocol_ir::{
    BalanceProjectionResponse, MessageEnvelope, MessageType, RouteReceiptDecisionTraceStep,
    RouteReceiptPolicyCheck, RouteReceiptProviderAttempt, RouteReceiptRecorded,
    RouteReceiptRecordedMessage, RouteReceiptRecordedMessageType, UsageEventRecorded,
};
use provider_traits::{
    AdapterLifecycleFamily, AdapterManifest, AdapterStability, ProviderAdapterRegistry,
    ProviderEndpoint, ProviderError, ProviderErrorKind, ProviderExecutionContext, ProviderMessage,
    ProviderRequest, ProviderResponse, ProviderTargetKind, StreamingSupport, TransitGatewayKind,
    TransitProviderMetadata,
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
    budget_store: Arc<dyn BudgetProjectionStore>,
    adapter_registry: ProviderAdapterRegistry,
    debug_headers_enabled: bool,
    event_sink: Arc<dyn RuntimeEventSink>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponsesApiRequest {
    pub model: String,
    pub input: Vec<ResponsesApiInputMessage>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponsesApiInputMessage {
    pub role: String,
    pub content: Vec<ResponsesApiInputContent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponsesApiInputContent {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
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
pub struct ResponsesApiResponse {
    pub id: String,
    pub object: &'static str,
    pub model: String,
    pub output: Vec<ResponsesApiOutputItem>,
    pub output_text: String,
    pub usage: UsageSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponsesApiOutputItem {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub role: &'static str,
    pub content: Vec<ResponsesApiOutputContent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponsesApiOutputContent {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub text: String,
    pub annotations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAdapterManifestsResponse {
    pub adapters: Vec<ProviderAdapterManifestDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAdapterManifestDto {
    pub manifest_schema_version: u16,
    pub adapter_id: &'static str,
    pub provider_kind: &'static str,
    pub display_name: &'static str,
    pub protocol_family: &'static str,
    pub supported_protocol_families: Vec<&'static str>,
    pub lifecycle_family: &'static str,
    pub stability: &'static str,
    pub streaming_support: &'static str,
    pub configuration_schema_ref: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAdapterManifestLookupError {
    pub code: &'static str,
    pub message: String,
    pub provider_kind: String,
    pub available_provider_kinds: Vec<String>,
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
    target_kind: ProviderTargetKind,
    transit_metadata: Option<TransitProviderMetadata>,
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
        .route("/internal/provider-adapters", get(provider_adapters))
        .route(
            "/internal/provider-adapters/{provider_kind}",
            get(provider_adapter),
        )
        .route("/v1/responses", post(responses))
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
    let budget_store = Arc::new(ControlPlaneBudgetStore::from_env());
    let adapter_registry = composition::default_provider_registry()
        .expect("provider adapter composition should succeed");

    Arc::new(AppState {
        config_store,
        auth_store,
        budget_store,
        adapter_registry,
        debug_headers_enabled: debug_headers_enabled_from_env(),
        event_sink: Arc::new(NatsEventSink::from_env()),
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: GATEWAY_SERVICE_NAME,
        status: "ok",
    })
}

async fn provider_adapters(
    State(state): State<GatewayState>,
) -> Json<ProviderAdapterManifestsResponse> {
    Json(provider_adapter_manifests_response(&state.adapter_registry))
}

async fn provider_adapter(
    State(state): State<GatewayState>,
    Path(provider_kind): Path<String>,
) -> Response {
    let provider_kind = provider_kind.trim().to_ascii_lowercase();

    state.adapter_registry.resolve(&provider_kind).map_or_else(
        || {
            let available_provider_kinds = state
                .adapter_registry
                .provider_kinds()
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            (
                StatusCode::NOT_FOUND,
                Json(ProviderAdapterManifestLookupError {
                    code: "provider_adapter_not_found",
                    message: format!("provider adapter `{provider_kind}` is not loaded"),
                    provider_kind,
                    available_provider_kinds,
                }),
            )
                .into_response()
        },
        |adapter| {
            let manifest = adapter.manifest();
            Json(provider_adapter_manifest_dto(&manifest)).into_response()
        },
    )
}

fn provider_adapter_manifests_response(
    registry: &ProviderAdapterRegistry,
) -> ProviderAdapterManifestsResponse {
    let manifests = registry.manifests();

    ProviderAdapterManifestsResponse {
        adapters: manifests
            .iter()
            .map(provider_adapter_manifest_dto)
            .collect(),
    }
}

fn provider_adapter_manifest_dto(manifest: &AdapterManifest) -> ProviderAdapterManifestDto {
    ProviderAdapterManifestDto {
        manifest_schema_version: manifest.manifest_schema_version,
        adapter_id: manifest.adapter_id,
        provider_kind: manifest.provider_kind,
        display_name: manifest.display_name,
        protocol_family: manifest.protocol_family,
        supported_protocol_families: manifest.supported_protocol_families.to_vec(),
        lifecycle_family: lifecycle_family_slug(manifest.lifecycle_family),
        stability: adapter_stability_slug(manifest.stability),
        streaming_support: streaming_support_slug(manifest.streaming_support),
        configuration_schema_ref: manifest.configuration_schema_ref,
    }
}

const fn lifecycle_family_slug(lifecycle_family: AdapterLifecycleFamily) -> &'static str {
    match lifecycle_family {
        AdapterLifecycleFamily::Inference => "inference",
        AdapterLifecycleFamily::TransitGateway => "transit_gateway",
        AdapterLifecycleFamily::Realtime => "realtime",
        AdapterLifecycleFamily::Tool => "tool",
        AdapterLifecycleFamily::Agent => "agent",
    }
}

const fn adapter_stability_slug(stability: AdapterStability) -> &'static str {
    match stability {
        AdapterStability::Stable => "stable",
        AdapterStability::Beta => "beta",
        AdapterStability::Experimental => "experimental",
    }
}

const fn streaming_support_slug(streaming_support: StreamingSupport) -> &'static str {
    match streaming_support {
        StreamingSupport::Unsupported => "unsupported",
        StreamingSupport::ServerSentEvents => "server_sent_events",
    }
}

async fn chat_completions(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Response {
    match process_chat_completion(state, &headers, request).await {
        Ok(success) => success.into_response(),
        Err(error) => error.into_response(),
    }
}

async fn responses(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ResponsesApiRequest>,
) -> Response {
    match process_responses_request(state, &headers, request).await {
        Ok(success) => responses_success_response(&success),
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

    match process_normalized_request(state, &headers, normalized_request).await {
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

    match process_normalized_request(state, &headers, normalized_request).await {
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
    headers: &HeaderMap,
    request: ChatCompletionRequest,
) -> Result<GatewaySuccess, GatewayError> {
    let normalized_request = normalize_request(request, &next_request_context())?;
    let success = process_normalized_request(state, headers, normalized_request.clone()).await?;

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

async fn process_responses_request(
    state: GatewayState,
    headers: &HeaderMap,
    request: ResponsesApiRequest,
) -> Result<ExecutionSuccess, GatewayError> {
    let normalized_request = normalize_responses_request(request, &next_request_context())?;
    process_normalized_request(state, headers, normalized_request).await
}

#[allow(clippy::too_many_lines)]
async fn process_normalized_request(
    state: GatewayState,
    headers: &HeaderMap,
    normalized_request: NormalizedChatRequest,
) -> Result<ExecutionSuccess, GatewayError> {
    let context = next_request_context();
    let bearer_token = extract_bearer_token(headers.get(AUTHORIZATION), &context)?;
    let request_headers = normalize_forward_headers(headers);
    let gateway_origin = infer_gateway_origin(headers);
    let api_key_scope = state
        .auth_store
        .resolve(&bearer_token)
        .await
        .map_err(|message| {
            publish_audit_best_effort(
                &state,
                "gateway.request.rejected",
                "auth_invalid",
                &context,
                BTreeMap::from([("reason".to_string(), message.clone())]),
            );
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
    ensure_budget_allows_request(
        &state,
        &api_key_scope,
        &active_config,
        &normalized_request,
        &context,
    )
    .await?;
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
        let debug_headers = maybe_debug_headers(
            state.debug_headers_enabled,
            &active_config.route_policy.route_policy_id,
            None,
            AdmissionResult::RejectedNoCandidate,
            0,
        );
        publish_route_receipt_or_error(
            &state,
            &route,
            &route_receipt,
            Vec::new(),
            &context,
            debug_headers.clone(),
        )
        .await?;

        return Err(GatewayError::with_route_receipt(
            StatusCode::SERVICE_UNAVAILABLE,
            route_receipt,
            normalized,
            &context,
            Some(active_config.config_snapshot.config_snapshot_id.clone()),
            debug_headers,
        ));
    }

    let result = execute_route(
        state.clone(),
        route,
        normalized_request,
        context.clone(),
        request_headers,
        gateway_origin,
    )
    .await;
    match &result {
        Ok(success) => {
            publish_audit_best_effort(
                &state,
                "gateway.request.succeeded",
                "admitted",
                &context,
                BTreeMap::from([
                    (
                        "route_receipt_id".to_string(),
                        success.route_receipt.route_receipt_id.to_string(),
                    ),
                    (
                        "provider_resource_id".to_string(),
                        success
                            .route_receipt
                            .selected_target
                            .as_ref()
                            .map_or_else(String::new, ToString::to_string),
                    ),
                ]),
            );
        }
        Err(error) => {
            publish_audit_best_effort(
                &state,
                "gateway.request.failed",
                &error.envelope.error.code,
                &context,
                BTreeMap::from([
                    ("message".to_string(), error.envelope.error.message.clone()),
                    (
                        "route_receipt_id".to_string(),
                        error
                            .route_receipt_id
                            .as_ref()
                            .map_or_else(String::new, ToString::to_string),
                    ),
                ]),
            );
        }
    }

    result
}

fn normalize_forward_headers(headers: &HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_ascii_lowercase(), value.to_string()))
        })
        .collect()
}

fn infer_gateway_origin(headers: &HeaderMap) -> Option<String> {
    std::env::var("GATEWAY_PUBLIC_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            let scheme = headers
                .get("x-forwarded-proto")
                .and_then(|value| value.to_str().ok())?;
            let host = headers.get("host").and_then(|value| value.to_str().ok())?;
            Some(format!("{}://{}/v1", scheme.trim(), host.trim()))
        })
}

#[allow(clippy::too_many_lines)]
async fn execute_route(
    state: GatewayState,
    route: RouteEvaluation,
    request: NormalizedChatRequest,
    context: RequestContext,
    request_headers: BTreeMap<String, String>,
    gateway_origin: Option<String>,
) -> Result<ExecutionSuccess, GatewayError> {
    let mut fallback_transitions = Vec::new();
    let mut provider_attempts = Vec::new();
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
            let occurred_at = now_rfc3339();
            provider_attempts.push(provider_attempt_record(
                target.resource.provider_resource_id.clone(),
                index + 1,
                "skipped",
                occurred_at.clone(),
                occurred_at,
                0,
                provider_error.message.clone(),
            ));

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
        let adapter_manifest = adapter.manifest();
        if !adapter_manifest.supports_protocol_family(&request.protocol_family) {
            let provider_error = ProviderError::new(
                ProviderErrorKind::Unavailable,
                format!(
                    "provider adapter `{}` does not support protocol `{}`",
                    adapter_manifest.adapter_id, request.protocol_family
                ),
                false,
            )
            .with_detail(
                "provider_resource_id",
                target.resource.provider_resource_id.as_str(),
            )
            .with_detail("adapter_id", adapter_manifest.adapter_id)
            .with_detail("provider_kind", adapter_manifest.provider_kind)
            .with_detail("protocol_family", request.protocol_family.as_str())
            .with_detail("manifest_boundary", "protocol_family_unsupported");
            let occurred_at = now_rfc3339();
            provider_attempts.push(provider_attempt_record(
                target.resource.provider_resource_id.clone(),
                index + 1,
                "skipped",
                occurred_at.clone(),
                occurred_at,
                0,
                provider_error.message.clone(),
            ));

            if let Some(next_target) = route.ranked_targets.get(index + 1) {
                fallback_transitions.push(FallbackTransition {
                    from_provider_resource_id: target.resource.provider_resource_id.clone(),
                    to_provider_resource_id: next_target
                        .target
                        .resource
                        .provider_resource_id
                        .clone(),
                    reason: format!("{}; retrying next ranked candidate", provider_error.message),
                });
            }

            last_error = Some((ranked_target.clone(), provider_error));
            if index + 1 < route.ranked_targets.len() {
                continue;
            }

            break;
        }

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
            gateway_service_name: GATEWAY_SERVICE_NAME.to_string(),
            gateway_origin: gateway_origin.clone(),
            request_headers: request_headers.clone(),
            endpoint: ProviderEndpoint {
                provider_resource_id: target.resource.provider_resource_id.as_str().to_string(),
                endpoint_base_url: target.resource.endpoint_base_url.clone(),
                api_key: target.api_key.clone(),
                region: Some(target.resource.region.clone()),
            },
        };
        let attempt_started_at = now_rfc3339();
        let attempt_started = Instant::now();

        match adapter
            .execute_chat(&provider_request, &provider_context)
            .await
        {
            Ok(provider_response) => {
                provider_attempts.push(provider_attempt_record(
                    target.resource.provider_resource_id.clone(),
                    index + 1,
                    "succeeded",
                    attempt_started_at,
                    now_rfc3339(),
                    u32::try_from(attempt_started.elapsed().as_millis()).unwrap_or(u32::MAX),
                    "provider returned output".to_string(),
                ));
                let fallback_count = fallback_transitions.len();
                let route_receipt = build_route_receipt(
                    &route,
                    &context,
                    &request,
                    Some(ranked_target),
                    None,
                    fallback_transitions,
                );
                let debug_headers = maybe_debug_headers(
                    state.debug_headers_enabled,
                    &route.config_snapshot.route_policy_id,
                    Some(target.resource.provider_resource_id.as_str()),
                    AdmissionResult::Admitted,
                    fallback_count,
                );
                publish_route_receipt_or_error(
                    &state,
                    &route,
                    &route_receipt,
                    provider_attempts.clone(),
                    &context,
                    debug_headers.clone(),
                )
                .await?;
                let usage_event =
                    build_usage_event(&route_receipt, target, &request, &provider_response);
                state
                    .event_sink
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
                            debug_headers.clone(),
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
                    debug_headers,
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
                provider_attempts.push(provider_attempt_record(
                    target.resource.provider_resource_id.clone(),
                    index + 1,
                    if error.retryable && has_more_candidates {
                        "retryable_failure"
                    } else {
                        "failed"
                    },
                    attempt_started_at,
                    now_rfc3339(),
                    u32::try_from(attempt_started.elapsed().as_millis()).unwrap_or(u32::MAX),
                    error.message.clone(),
                ));
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
                    let debug_headers = maybe_debug_headers(
                        state.debug_headers_enabled,
                        &route.config_snapshot.route_policy_id,
                        Some(ranked_target.target.resource.provider_resource_id.as_str()),
                        route.admission_result,
                        fallback_count,
                    );
                    publish_route_receipt_or_error(
                        &state,
                        &route,
                        &route_receipt,
                        provider_attempts.clone(),
                        &context,
                        debug_headers.clone(),
                    )
                    .await?;

                    return Err(GatewayError::with_route_receipt(
                        status_for_error_code(&normalized.error.code),
                        route_receipt,
                        normalized.error,
                        &context,
                        Some(route.config_snapshot.config_snapshot_id.clone()),
                        debug_headers,
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
    let debug_headers = maybe_debug_headers(
        state.debug_headers_enabled,
        &route.config_snapshot.route_policy_id,
        Some(ranked_target.target.resource.provider_resource_id.as_str()),
        route.admission_result,
        fallback_count,
    );
    publish_route_receipt_or_error(
        &state,
        &route,
        &route_receipt,
        provider_attempts,
        &context,
        debug_headers.clone(),
    )
    .await?;

    Err(GatewayError::with_route_receipt(
        status_for_error_code(&normalized.error.code),
        route_receipt,
        normalized.error,
        &context,
        Some(route.config_snapshot.config_snapshot_id.clone()),
        debug_headers,
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
fn normalize_responses_request(
    request: ResponsesApiRequest,
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

    if request.input.is_empty() {
        validation_issues.push(ValidationIssue {
            field: "input".to_string(),
            message: "input must include at least one message".to_string(),
        });
    }

    let mut messages = Vec::with_capacity(request.input.len());
    for (message_index, message) in request.input.iter().enumerate() {
        if message.role.trim().is_empty() {
            validation_issues.push(ValidationIssue {
                field: format!("input[{message_index}].role"),
                message: "role must not be empty".to_string(),
            });
        }
        let text = message
            .content
            .iter()
            .filter(|content| content.kind == "input_text")
            .map(|content| content.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if text.is_empty() {
            validation_issues.push(ValidationIssue {
                field: format!("input[{message_index}].content"),
                message:
                    "responses input content must include at least one non-empty input_text item"
                        .to_string(),
            });
            continue;
        }
        messages.push(ProviderMessage {
            role: message.role.clone(),
            content: text,
        });
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
        protocol_family: "openai_responses".to_string(),
        estimated_prompt_tokens: estimate_provider_messages_tokens(&messages),
        messages,
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
            "chat_completions" => true,
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

fn build_route_receipt(
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

fn build_route_receipt_policy_checks(
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

fn build_route_receipt_decision_timeline(
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

fn build_route_receipt_recorded(
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

fn provider_attempt_record(
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

async fn publish_route_receipt_or_error(
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

fn map_provider_response_to_responses(
    context: &RequestContext,
    request: &NormalizedChatRequest,
    response: &ProviderResponse,
) -> ResponsesApiResponse {
    let prompt_tokens = response
        .usage
        .input_tokens
        .max(request.estimated_prompt_tokens);
    let completion_tokens = response.usage.output_tokens;
    let response_id = response
        .response_id
        .clone()
        .unwrap_or_else(|| format!("resp_{}", context.sequence));

    ResponsesApiResponse {
        id: response_id,
        object: "response",
        model: response.model.clone(),
        output_text: response.output_text.clone(),
        output: vec![ResponsesApiOutputItem {
            kind: "message",
            role: "assistant",
            content: vec![ResponsesApiOutputContent {
                kind: "output_text",
                text: response.output_text.clone(),
                annotations: Vec::new(),
            }],
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
        ProviderErrorKind::InvalidRequest => {
            if error.details.contains_key("loop_guard") {
                "transit_loop_detected"
            } else {
                "request_validation_failed"
            }
        }
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

async fn ensure_budget_allows_request(
    state: &GatewayState,
    api_key_scope: &GatewayApiKeyScope,
    active_config: &ActiveGatewayConfig,
    request: &NormalizedChatRequest,
    context: &RequestContext,
) -> Result<(), GatewayError> {
    let projection = state
        .budget_store
        .load_budget(&BudgetProjectionScope {
            tenant_id: api_key_scope.tenant_id.clone(),
            project_id: api_key_scope.project_id.clone(),
        })
        .await
        .map_err(|error| {
            GatewayError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                normalized_error(
                    "budget_projection_unavailable",
                    format!("budget projection is unavailable: {error}"),
                    context,
                    true,
                ),
                context,
            )
        })?;

    if projection.data.threshold_status != "exceeded" {
        return Ok(());
    }

    let route = RouteEvaluation {
        config_snapshot: active_config.config_snapshot.clone(),
        admission_result: AdmissionResult::RejectedBudget,
        excluded_targets: Vec::new(),
        ranked_targets: Vec::new(),
    };
    let reason = format!(
        "budget exhausted for tenant `{}` project `{}`; remaining budget {} {}",
        projection.data.tenant_id,
        projection
            .data
            .project_id
            .as_ref()
            .map_or_else(|| "global".to_string(), ToString::to_string),
        projection.data.remaining_budget.amount,
        projection.data.remaining_budget.currency
    );
    let normalized = normalized_error("budget_exceeded", reason, context, false).error;
    let route_receipt = build_route_receipt(
        &route,
        context,
        request,
        None,
        Some(normalized.clone()),
        Vec::new(),
    );
    let debug_headers = maybe_debug_headers(
        state.debug_headers_enabled,
        &active_config.route_policy.route_policy_id,
        None,
        AdmissionResult::RejectedBudget,
        0,
    );
    publish_route_receipt_or_error(
        state,
        &route,
        &route_receipt,
        Vec::new(),
        context,
        debug_headers.clone(),
    )
    .await?;
    publish_audit_best_effort(
        state,
        "gateway.request.rejected",
        "budget_exceeded",
        context,
        BTreeMap::from([
            (
                "configured_budget".to_string(),
                projection.data.configured_budget.amount.clone(),
            ),
            (
                "remaining_budget".to_string(),
                projection.data.remaining_budget.amount.clone(),
            ),
        ]),
    );

    Err(GatewayError::with_route_receipt(
        StatusCode::FORBIDDEN,
        route_receipt,
        normalized,
        context,
        Some(active_config.config_snapshot.config_snapshot_id.clone()),
        debug_headers,
    ))
}

fn unix_timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn publish_audit_best_effort(
    state: &GatewayState,
    action: &str,
    outcome: &str,
    context: &RequestContext,
    details: BTreeMap<String, String>,
) {
    let sink = Arc::clone(&state.event_sink);
    let context = context.clone();
    let action = action.to_string();
    let outcome = outcome.to_string();
    tokio::spawn(async move {
        if let Err(error) = sink
            .publish_audit(&action, &outcome, &context, details)
            .await
        {
            warn!(
                request_id = context.request_id,
                trace_id = context.trace_id,
                action,
                outcome,
                error,
                "failed to publish gateway audit event"
            );
        }
    });
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
        "request_validation_failed" | "transit_loop_detected" => StatusCode::BAD_REQUEST,
        "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
        "upstream_timeout" => StatusCode::GATEWAY_TIMEOUT,
        "upstream_protocol_error" => StatusCode::BAD_GATEWAY,
        "route_not_available"
        | "provider_unavailable"
        | "route_receipt_publish_failed"
        | "usage_event_publish_failed" => StatusCode::SERVICE_UNAVAILABLE,
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

fn responses_success_response(success: &ExecutionSuccess) -> Response {
    let context = RequestContext {
        request_id: success.request_id.clone(),
        trace_id: success.trace_id.clone(),
        sequence: success.sequence,
    };
    let request = NormalizedChatRequest {
        model_alias: success.provider_response.model.clone(),
        protocol_family: "openai_responses".to_string(),
        messages: Vec::new(),
        estimated_prompt_tokens: success.provider_response.usage.input_tokens,
    };
    let payload =
        map_provider_response_to_responses(&context, &request, &success.provider_response);
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
trait BudgetProjectionStore: Send + Sync {
    async fn load_budget(
        &self,
        scope: &BudgetProjectionScope,
    ) -> Result<BalanceProjectionResponse, String>;
}

#[async_trait]
trait ApiKeyScopeStore: Send + Sync {
    async fn resolve(&self, api_key: &str) -> Result<GatewayApiKeyScope, String>;
}

#[async_trait]
trait RuntimeEventSink: Send + Sync {
    async fn publish_route_receipt(
        &self,
        route_receipt: &RouteReceipt,
        payload: &RouteReceiptRecorded,
        context: &RequestContext,
    ) -> Result<(), String>;

    async fn publish(
        &self,
        route_receipt: &RouteReceipt,
        usage_event: &UsageEvent,
        context: &RequestContext,
    ) -> Result<(), String>;

    async fn publish_audit(
        &self,
        action: &str,
        outcome: &str,
        context: &RequestContext,
        details: BTreeMap<String, String>,
    ) -> Result<(), String>;
}

#[cfg(test)]
#[derive(Debug)]
struct StaticConfigStore {
    config: ActiveGatewayConfig,
}

#[cfg(test)]
#[derive(Debug)]
struct StaticBudgetProjectionStore {
    response: BalanceProjectionResponse,
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

#[cfg(test)]
#[async_trait]
impl BudgetProjectionStore for StaticBudgetProjectionStore {
    async fn load_budget(
        &self,
        _scope: &BudgetProjectionScope,
    ) -> Result<BalanceProjectionResponse, String> {
        Ok(self.response.clone())
    }
}

#[derive(Debug, Clone)]
struct ControlPlaneConfigStore {
    base_url: String,
    cache: Arc<Mutex<Option<CachedActiveConfig>>>,
    cache_ttl: Duration,
    client: reqwest::Client,
    internal_token: Option<String>,
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

#[derive(Debug, Clone)]
struct ControlPlaneBudgetStore {
    base_url: String,
    client: reqwest::Client,
    internal_token: Option<String>,
    projection_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GatewayApiKeyResolveRequest {
    api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GatewayApiKeyResolveResponse {
    credential_id: String,
    project_id: Option<String>,
    status: String,
    tenant_id: String,
}

#[derive(Debug, Clone)]
struct BudgetProjectionScope {
    tenant_id: String,
    project_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct InternalGatewayConfigResponse {
    config_snapshot: ConfigSnapshot,
    route_policy: RoutePolicy,
    provider_resources: Vec<ProviderResource>,
}

#[derive(Debug)]
struct NatsEventSink {
    client: OnceCell<async_nats::Client>,
    audit_subject: String,
    route_receipt_subject: String,
    usage_event_subject: String,
    url: String,
}

impl ControlPlaneConfigStore {
    fn from_env() -> Self {
        let base_url = std::env::var("CONTROL_PLANE_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_CONTROL_PLANE_BASE_URL.to_string());
        let internal_token = std::env::var("CONTROL_PLANE_INTERNAL_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let snapshot_ref = std::env::var("GATEWAY_CONTROL_PLANE_SNAPSHOT_REF")
            .unwrap_or_else(|_| DEFAULT_CONTROL_PLANE_SNAPSHOT_REF.to_string());
        let cache_ttl = std::env::var("GATEWAY_CONFIG_CACHE_TTL_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map_or_else(|| Duration::from_secs(5), Duration::from_millis);

        Self::new(
            base_url,
            snapshot_ref,
            internal_token,
            cache_ttl,
            reqwest::Client::new(),
        )
    }

    fn new(
        base_url: impl Into<String>,
        snapshot_ref: impl Into<String>,
        internal_token: Option<String>,
        cache_ttl: Duration,
        client: reqwest::Client,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            cache: Arc::new(Mutex::new(None)),
            cache_ttl,
            client,
            internal_token,
            snapshot_ref: snapshot_ref.into(),
        }
    }

    async fn fetch_active_config(&self) -> Result<ActiveGatewayConfig, String> {
        let payload = self
            .get_json::<InternalGatewayConfigResponse>("/internal/gateway/config/current")
            .await?;

        if self.snapshot_ref != DEFAULT_CONTROL_PLANE_SNAPSHOT_REF
            && payload.config_snapshot.config_snapshot_id.as_str() != self.snapshot_ref
        {
            return Err(format!(
                "control plane returned config snapshot `{}` but gateway requested `{}`",
                payload.config_snapshot.config_snapshot_id, self.snapshot_ref
            ));
        }

        let provider_targets = payload
            .config_snapshot
            .provider_resource_ids
            .iter()
            .enumerate()
            .map(|(index, provider_resource_id)| {
                let resource = payload
                    .provider_resources
                    .iter()
                    .find(|candidate| &candidate.provider_resource_id == provider_resource_id)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "provider resource {provider_resource_id} is missing from control plane"
                        )
                    })?;

                Ok(ProviderTargetRuntime {
                    target_kind: provider_target_kind(&resource.provider_id),
                    transit_metadata: transit_metadata_for_target(&resource, &payload.route_policy),
                    priority: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    upstream_model: upstream_model_for_target(&resource),
                    api_key: api_key_for_target(&resource),
                    static_latency_score: static_latency_score_for_region(
                        &payload.route_policy.preferred_regions,
                        &resource.region,
                    ),
                    static_cost_score: static_cost_score_for_target(
                        &payload.route_policy,
                        &resource,
                    ),
                    usd_per_1k_tokens: usd_per_1k_tokens_for_target(
                        &payload.route_policy,
                        &resource,
                    ),
                    resource,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(ActiveGatewayConfig {
            config_snapshot: payload.config_snapshot,
            route_policy: payload.route_policy,
            provider_targets,
        })
    }

    async fn get_json<T>(&self, path: &str) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let base = self.base_url.trim_end_matches('/');
        let url = format!("{base}{path}");
        let mut request = self.client.get(&url);
        if let Some(internal_token) = self.internal_token.as_ref() {
            request = request.header(AUTHORIZATION, format!("Bearer {internal_token}"));
        }
        let response = request
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
            .map(|payload| GatewayApiKeyScope {
                credential_id: payload.credential_id,
                tenant_id: payload.tenant_id,
                project_id: payload.project_id,
                status: payload.status,
            })
            .map_err(|error| format!("failed to decode API key resolution payload: {error}"))
    }
}

impl ControlPlaneBudgetStore {
    fn from_env() -> Self {
        let base_url = std::env::var("CONTROL_PLANE_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_CONTROL_PLANE_BASE_URL.to_string());
        let internal_token = std::env::var("CONTROL_PLANE_INTERNAL_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let projection_path = std::env::var("GATEWAY_BILLING_PROJECTION_PATH")
            .unwrap_or_else(|_| "/internal/gateway/billing-projection".to_string());

        Self::new(
            base_url,
            projection_path,
            internal_token,
            reqwest::Client::new(),
        )
    }

    fn new(
        base_url: impl Into<String>,
        projection_path: impl Into<String>,
        internal_token: Option<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            client,
            internal_token,
            projection_path: projection_path.into(),
        }
    }

    async fn fetch_budget(
        &self,
        scope: &BudgetProjectionScope,
    ) -> Result<BalanceProjectionResponse, String> {
        let internal_token = self.internal_token.as_ref().ok_or_else(|| {
            "CONTROL_PLANE_INTERNAL_TOKEN must be configured for gateway budget resolution"
                .to_string()
        })?;
        let base = self.base_url.trim_end_matches('/');
        let path = if self.projection_path.starts_with('/') {
            self.projection_path.clone()
        } else {
            format!("/{}", self.projection_path)
        };
        let url = format!("{base}{path}");
        let mut request = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {internal_token}"))
            .query(&[("tenant_id", scope.tenant_id.as_str())]);
        if let Some(project_id) = scope.project_id.as_deref() {
            request = request.query(&[("project_id", project_id)]);
        }
        let response = request.send().await.map_err(|error| {
            format!("failed to fetch budget projection via control plane: {error}")
        })?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read response body".to_string());
            return Err(format!(
                "control plane rejected budget projection query with HTTP {status}: {body}"
            ));
        }

        response
            .json::<BalanceProjectionResponse>()
            .await
            .map_err(|error| format!("failed to decode balance projection payload: {error}"))
    }
}

#[async_trait]
impl ApiKeyScopeStore for ControlPlaneApiKeyStore {
    async fn resolve(&self, api_key: &str) -> Result<GatewayApiKeyScope, String> {
        self.fetch_scope(api_key).await
    }
}

#[async_trait]
impl BudgetProjectionStore for ControlPlaneBudgetStore {
    async fn load_budget(
        &self,
        scope: &BudgetProjectionScope,
    ) -> Result<BalanceProjectionResponse, String> {
        self.fetch_budget(scope).await
    }
}

impl NatsEventSink {
    fn from_env() -> Self {
        let url = std::env::var("GATEWAY_NATS_URL")
            .or_else(|_| std::env::var("NATS_URL"))
            .unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
        let audit_subject = std::env::var("GATEWAY_AUDIT_EVENT_SUBJECT")
            .unwrap_or_else(|_| "events.audit_event.created".to_string());
        let route_receipt_subject = std::env::var("GATEWAY_ROUTE_RECEIPT_SUBJECT")
            .unwrap_or_else(|_| "events.route_receipt.recorded".to_string());
        let usage_event_subject = std::env::var("GATEWAY_USAGE_EVENT_SUBJECT")
            .unwrap_or_else(|_| "events.usage_event.recorded".to_string());

        Self::new(
            url,
            audit_subject,
            route_receipt_subject,
            usage_event_subject,
        )
    }

    fn new(
        url: impl Into<String>,
        audit_subject: impl Into<String>,
        route_receipt_subject: impl Into<String>,
        usage_event_subject: impl Into<String>,
    ) -> Self {
        Self {
            client: OnceCell::new(),
            audit_subject: audit_subject.into(),
            route_receipt_subject: route_receipt_subject.into(),
            usage_event_subject: usage_event_subject.into(),
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
impl RuntimeEventSink for NatsEventSink {
    async fn publish_route_receipt(
        &self,
        route_receipt: &RouteReceipt,
        payload: &RouteReceiptRecorded,
        context: &RequestContext,
    ) -> Result<(), String> {
        let envelope = RouteReceiptRecordedMessage {
            message_id: format!("msg_{}", route_receipt.route_receipt_id),
            message_type: RouteReceiptRecordedMessageType::RouteReceiptRecorded,
            schema_version: 1,
            occurred_at: route_receipt.created_at.clone(),
            producer: ServiceName::parse(GATEWAY_SERVICE_NAME)
                .expect("gateway service name should be valid"),
            trace_id: Some(context.trace_id.clone()),
            request_id: Some(context.request_id.clone()),
            idempotency_key: format!("{}:recorded", route_receipt.route_receipt_id),
            payload: payload.clone(),
        };
        let payload = serde_json::to_vec(&envelope)
            .map_err(|error| format!("failed to serialize route receipt envelope: {error}"))?;
        let client = self.client().await?;

        client
            .publish(self.route_receipt_subject.clone(), payload.into())
            .await
            .map_err(|error| format!("failed to publish route receipt: {error}"))?;
        client
            .flush()
            .await
            .map_err(|error| format!("failed to flush route receipt publish: {error}"))?;
        Ok(())
    }

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
            .publish(self.usage_event_subject.clone(), payload.into())
            .await
            .map_err(|error| format!("failed to publish usage event: {error}"))?;
        client
            .flush()
            .await
            .map_err(|error| format!("failed to flush usage event publish: {error}"))?;
        Ok(())
    }

    async fn publish_audit(
        &self,
        action: &str,
        outcome: &str,
        context: &RequestContext,
        details: BTreeMap<String, String>,
    ) -> Result<(), String> {
        let audit_event = core_domain::AuditEvent {
            audit_event_id: format!("auditevt_{}", context.sequence),
            actor: GATEWAY_SERVICE_NAME.to_string(),
            action: action.to_string(),
            request_id: Some(context.request_id.clone()),
            trace_id: context.trace_id.clone(),
            recorded_at: now_rfc3339(),
        };
        let payload = serde_json::json!({
            "audit_event": audit_event,
            "outcome": outcome,
            "details": details,
        });
        let envelope = MessageEnvelope::new(
            format!("msg_audit_{}", context.sequence),
            MessageType::AuditEventCreated,
            now_rfc3339(),
            ServiceName::parse(GATEWAY_SERVICE_NAME).expect("gateway service name should be valid"),
            format!("audit:{}:{}", action, context.request_id),
            payload,
        )
        .with_request_context(context.trace_id.clone(), context.request_id.clone());
        let body = serde_json::to_vec(&envelope)
            .map_err(|error| format!("failed to serialize audit event envelope: {error}"))?;
        let client = self.client().await?;

        client
            .publish(self.audit_subject.clone(), body.into())
            .await
            .map_err(|error| format!("failed to publish audit event: {error}"))?;
        client
            .flush()
            .await
            .map_err(|error| format!("failed to flush audit event publish: {error}"))?;
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

fn provider_resource_env_prefix(provider_resource_id: &str) -> String {
    provider_resource_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn provider_target_kind(provider_id: &str) -> ProviderTargetKind {
    if provider_id == "gateway" {
        ProviderTargetKind::TransitGateway
    } else {
        ProviderTargetKind::Native
    }
}

fn transit_metadata_for_target(
    resource: &ProviderResource,
    route_policy: &RoutePolicy,
) -> Option<TransitProviderMetadata> {
    (resource.provider_id == "gateway").then(|| TransitProviderMetadata {
        gateway_kind: TransitGatewayKind::OpenAiCompatible,
        gateway_name: reqwest::Url::parse(&resource.endpoint_base_url)
            .ok()
            .and_then(|url| url.host_str().map(ToString::to_string))
            .unwrap_or_else(|| "openai-compatible-gateway".to_string()),
        route_cost_scope: route_policy.protocol_family.clone(),
        transit_hops: 1,
        preserves_error_diagnostics: true,
    })
}

fn api_key_for_target(resource: &ProviderResource) -> String {
    let resource_prefix = provider_resource_env_prefix(resource.provider_resource_id.as_str());
    std::env::var(format!("PROVIDER_RESOURCE_{resource_prefix}_API_KEY"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| match resource.provider_id.as_str() {
            "openai" => std::env::var("GATEWAY_OPENAI_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .unwrap_or_default(),
            "anthropic" => std::env::var("GATEWAY_ANTHROPIC_API_KEY")
                .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
                .unwrap_or_default(),
            "gateway" => std::env::var("GATEWAY_TRANSIT_API_KEY").unwrap_or_default(),
            "gemini" => std::env::var("GATEWAY_GEMINI_API_KEY")
                .or_else(|_| std::env::var("GEMINI_API_KEY"))
                .or_else(|_| std::env::var("GOOGLE_API_KEY"))
                .unwrap_or_default(),
            _ => String::new(),
        })
}

fn upstream_model_for_target(resource: &ProviderResource) -> Option<String> {
    let resource_prefix = provider_resource_env_prefix(resource.provider_resource_id.as_str());
    if let Ok(model) = std::env::var(format!("PROVIDER_RESOURCE_{resource_prefix}_MODEL")) {
        return Some(model);
    }

    match resource.provider_id.as_str() {
        "openai" => Some(
            std::env::var("GATEWAY_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string()),
        ),
        "anthropic" => Some(
            std::env::var("GATEWAY_ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-5-sonnet-latest".to_string()),
        ),
        "bedrock" => std::env::var("GATEWAY_BEDROCK_MODEL").ok(),
        "gateway" => std::env::var("GATEWAY_TRANSIT_MODEL").ok(),
        "gemini" => Some(
            std::env::var("GATEWAY_GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-1.5-flash-latest".to_string()),
        ),
        _ => None,
    }
}

fn transit_usd_per_1k_tokens_for_route(route_policy: &RoutePolicy) -> f64 {
    match route_policy.protocol_family.as_str() {
        "anthropic_messages" => std::env::var("GATEWAY_TRANSIT_ANTHROPIC_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.018),
        "gemini_generate_content" => std::env::var("GATEWAY_TRANSIT_GEMINI_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.011),
        _ => std::env::var("GATEWAY_TRANSIT_OPENAI_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.014),
    }
}

fn usd_per_1k_tokens_for_target(route_policy: &RoutePolicy, resource: &ProviderResource) -> f64 {
    if resource.provider_id == "gateway" {
        return transit_usd_per_1k_tokens_for_route(route_policy);
    }

    match resource.provider_id.as_str() {
        "openai" => std::env::var("GATEWAY_OPENAI_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.01),
        "anthropic" => std::env::var("GATEWAY_ANTHROPIC_USD_PER_1K_TOKENS")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.012),
        "bedrock" => std::env::var("GATEWAY_BEDROCK_USD_PER_1K_TOKENS")
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

#[allow(clippy::cast_possible_truncation)]
fn static_cost_score_for_target(route_policy: &RoutePolicy, resource: &ProviderResource) -> f32 {
    let rate = usd_per_1k_tokens_for_target(route_policy, resource);
    let bounded_rate = (rate / 0.05).clamp(0.0, 1.0) as f32;
    let price_score = (1.0 - bounded_rate).max(0.1);
    ((price_score * 0.7) + (static_cost_score_for_scope(resource.deployment_scope) * 0.3)).max(0.1)
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
mod tests;
