use axum::{
    extract::State,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use core_domain::{
    AdmissionResult, ConfigSnapshot, ConfigSnapshotId, ExcludedTarget, FallbackTransition,
    ProviderResourceId, RouteReceipt, RouteReceiptId, ScoreBreakdown, ServiceName, UsageEvent,
    UsageEventId, UsagePhase,
};
use protocol_ir::{MessageEnvelope, MessageType, UsageEventRecorded};
use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;

const BOOTSTRAP_CONFIG_SNAPSHOT_ID: &str = "cfgsnap_bootstrap_v1";
const BOOTSTRAP_OCCURRED_AT: &str = "2026-04-20T00:00:00Z";
const GATEWAY_SERVICE_NAME: &str = "gateway-api";
const SELECTED_PROVIDER_RESOURCE_ID: &str = "prvrsrc_openai_primary";

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1_000);

#[derive(Clone, Debug, Default)]
pub struct GatewayState;

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize)]
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
    pub finish_reason: &'static str,
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
    pub config_snapshot: ConfigSnapshot,
    pub route_receipt: RouteReceipt,
    pub usage_event: UsageEvent,
    pub usage_event_message: MessageEnvelope<UsageEventRecorded>,
    pub response: ChatCompletionResponse,
}

#[derive(Debug, Clone)]
pub struct GatewayError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub request_id: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct GatewayErrorEnvelope {
    error: GatewayErrorBody,
}

#[derive(Debug, Clone, Serialize)]
struct GatewayErrorBody {
    code: &'static str,
    message: String,
    request_id: String,
    trace_id: String,
}

#[derive(Debug, Clone)]
struct RequestContext {
    request_id: String,
    trace_id: String,
    sequence: u64,
}

pub fn app() -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(GatewayState)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: GATEWAY_SERVICE_NAME,
        status: "ok",
    })
}

async fn chat_completions(
    State(_state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Response {
    match process_chat_completion(headers.get(AUTHORIZATION), request) {
        Ok(success) => success.into_response(),
        Err(error) => error.into_response(),
    }
}

fn process_chat_completion(
    authorization_header: Option<&HeaderValue>,
    request: ChatCompletionRequest,
) -> Result<GatewaySuccess, GatewayError> {
    let context = next_request_context();

    ensure_bearer_auth(authorization_header, &context)?;
    validate_request(&request, &context)?;

    let config_snapshot = ConfigSnapshot {
        config_snapshot_id: ConfigSnapshotId::parse(BOOTSTRAP_CONFIG_SNAPSHOT_ID.to_string())
            .expect("bootstrap config snapshot id should be valid"),
        activated_at: BOOTSTRAP_OCCURRED_AT.to_string(),
        revision: 1,
    };

    let selected_target =
        ProviderResourceId::parse(SELECTED_PROVIDER_RESOURCE_ID.to_string()).expect("valid id");
    let route_receipt = RouteReceipt {
        route_receipt_id: RouteReceiptId::parse(format!("routercpt_{}", context.sequence))
            .expect("valid route receipt id"),
        request_id: context.request_id.clone(),
        trace_id: context.trace_id.clone(),
        config_snapshot_id: config_snapshot.config_snapshot_id.clone(),
        admission_result: AdmissionResult::Admitted,
        selected_target: Some(selected_target.clone()),
        excluded_targets: vec![ExcludedTarget {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_shadow_candidate".to_string())
                .expect("valid excluded target id"),
            reason: "bootstrap slice keeps one active candidate and one documented exclusion"
                .to_string(),
        }],
        score_breakdown: ScoreBreakdown {
            latency: 0.82,
            cost: 0.66,
            health: 0.97,
            trust: 1.0,
        },
        fallback_transitions: vec![FallbackTransition {
            from_provider_resource_id: selected_target.clone(),
            to_provider_resource_id: selected_target.clone(),
            reason: "no fallback transition required in bootstrap flow".to_string(),
        }],
    };

    let usage_event = UsageEvent {
        usage_event_id: UsageEventId::parse(format!("usageevt_{}", context.sequence))
            .expect("valid usage event id"),
        route_receipt_id: route_receipt.route_receipt_id.clone(),
        phase: UsagePhase::Final,
        idempotency_key: format!("{}:final", route_receipt.route_receipt_id),
    };

    let usage_event_message = MessageEnvelope::new(
        format!("msg_{}", context.sequence),
        MessageType::UsageEventRecorded,
        BOOTSTRAP_OCCURRED_AT,
        ServiceName::from(GATEWAY_SERVICE_NAME),
        usage_event.idempotency_key.clone(),
        UsageEventRecorded {
            usage_event: usage_event.clone(),
        },
    )
    .with_request_context(context.trace_id.clone(), context.request_id.clone());

    info!(
        request_id = context.request_id,
        trace_id = context.trace_id,
        config_snapshot_id = %config_snapshot.config_snapshot_id,
        route_receipt = %serde_json::to_string(&route_receipt).expect("route receipt serialization should succeed"),
        usage_event_message = %serde_json::to_string(&usage_event_message)
            .expect("usage event serialization should succeed"),
        "gateway bootstrap flow completed"
    );

    let prompt_tokens = estimate_prompt_tokens(&request.messages);
    let completion_tokens = if request.stream { 36 } else { 24 };
    let response = ChatCompletionResponse {
        id: format!("chatcmpl_{}", context.sequence),
        object: "chat.completion",
        created: unix_timestamp_seconds(),
        model: request.model,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: AssistantMessage {
                role: "assistant",
                content: format!(
                    "HugeRouter bootstrap placeholder admitted the request, selected {selected_target}, and emitted route receipt {}.",
                    route_receipt.route_receipt_id
                ),
            },
            finish_reason: "stop",
        }],
        usage: UsageSummary {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        },
    };

    Ok(GatewaySuccess {
        request_id: context.request_id,
        trace_id: context.trace_id,
        config_snapshot,
        route_receipt,
        usage_event,
        usage_event_message,
        response,
    })
}

fn ensure_bearer_auth(
    authorization_header: Option<&HeaderValue>,
    context: &RequestContext,
) -> Result<(), GatewayError> {
    let Some(value) = authorization_header else {
        return Err(GatewayError {
            status: StatusCode::UNAUTHORIZED,
            code: "auth_invalid",
            message: "missing Authorization header".to_string(),
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
        });
    };

    let is_bearer = value
        .to_str()
        .ok()
        .is_some_and(|header| header.starts_with("Bearer "));

    if is_bearer {
        Ok(())
    } else {
        Err(GatewayError {
            status: StatusCode::UNAUTHORIZED,
            code: "auth_invalid",
            message: "Authorization header must use Bearer credentials".to_string(),
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
        })
    }
}

fn validate_request(
    request: &ChatCompletionRequest,
    context: &RequestContext,
) -> Result<(), GatewayError> {
    if request.model.trim().is_empty() {
        return Err(GatewayError {
            status: StatusCode::BAD_REQUEST,
            code: "request_validation_failed",
            message: "model must not be empty".to_string(),
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
        });
    }

    if request.messages.is_empty()
        || request
            .messages
            .iter()
            .any(|message| message.role.trim().is_empty() || message.content.trim().is_empty())
    {
        return Err(GatewayError {
            status: StatusCode::BAD_REQUEST,
            code: "request_validation_failed",
            message: "messages must include at least one non-empty role/content pair".to_string(),
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
        });
    }

    Ok(())
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
            HeaderValue::from_str(self.config_snapshot.config_snapshot_id.as_str())
                .expect("config snapshot id should be valid header"),
        );

        response
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let payload = GatewayErrorEnvelope {
            error: GatewayErrorBody {
                code: self.code,
                message: self.message,
                request_id: self.request_id.clone(),
                trace_id: self.trace_id.clone(),
            },
        };
        let mut response = (self.status, Json(payload)).into_response();
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

        response
    }
}

#[cfg(test)]
mod tests {
    use super::{process_chat_completion, ChatCompletionRequest, ChatMessage};
    use axum::http::HeaderValue;
    use core_domain::AdmissionResult;
    use protocol_ir::MessageType;

    fn valid_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "reasoning-fast".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "hello router".to_string(),
            }],
            stream: false,
        }
    }

    #[test]
    fn rejects_missing_auth_with_normalized_error() {
        let error = process_chat_completion(None, valid_request()).unwrap_err();

        assert_eq!(error.status, axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(error.code, "auth_invalid");
        assert!(error.request_id.starts_with("req_"));
        assert!(error.trace_id.starts_with("trace_"));
    }

    #[test]
    fn emits_route_receipt_and_usage_event_on_success() {
        let success = process_chat_completion(
            Some(&HeaderValue::from_static("Bearer bootstrap-token")),
            valid_request(),
        )
        .unwrap();

        assert_eq!(success.route_receipt.admission_result, AdmissionResult::Admitted);
        assert!(success.request_id.starts_with("req_"));
        assert!(success.trace_id.starts_with("trace_"));
        assert_eq!(success.config_snapshot.config_snapshot_id.as_str(), "cfgsnap_bootstrap_v1");
        assert!(success.route_receipt.route_receipt_id.as_str().starts_with("routercpt_"));
        assert!(success.usage_event.usage_event_id.as_str().starts_with("usageevt_"));
        assert_eq!(
            success.usage_event_message.message_type,
            MessageType::UsageEventRecorded
        );
        assert_eq!(
            success.response.choices[0].message.role,
            "assistant"
        );
    }
}
