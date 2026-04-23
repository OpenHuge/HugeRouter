mod config;
mod ids;
mod session;
mod upstream;

use crate::ids::new_id;
use crate::session::{SessionContext, run_session};
use crate::upstream::build_upstream_connector;
use anyhow::Result;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router, extract::State};
use protocol_realtime::{
    AcceptedRealtimeSession, EphemeralTokenSigner, HandshakeRequest, SessionLifecyclePhase,
    TokenError, validate_handshake,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

pub use config::{RealtimeGatewayConfig, UpstreamMode};

#[derive(Clone)]
struct AppState {
    config: RealtimeGatewayConfig,
    signer: Arc<EphemeralTokenSigner>,
    upstream: Arc<dyn upstream::UpstreamConnector + Send + Sync>,
}

#[derive(Debug, Serialize)]
struct HealthResponse<'a> {
    service: &'a str,
    status: &'a str,
}

#[derive(Debug, Serialize)]
struct ErrorResponse<'a> {
    request_id: &'a str,
    trace_id: &'a str,
    error: ErrorBody<'a>,
}

#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: String,
}

#[derive(Debug, Clone)]
struct RequestContext {
    request_id: String,
    trace_id: String,
}

#[derive(Debug)]
struct HandshakeRejection {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl HandshakeRejection {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

/// # Errors
///
/// Returns an error when runtime configuration is invalid.
pub fn app(config: RealtimeGatewayConfig) -> Result<Router> {
    let signer = Arc::new(EphemeralTokenSigner::new(
        config.signing_secret.clone().into_bytes(),
    )?);
    let upstream = build_upstream_connector(&config);

    Ok(Router::new()
        .route("/healthz", get(health))
        .route("/v1/realtime", get(realtime_websocket))
        .with_state(AppState {
            config,
            signer,
            upstream,
        }))
}

/// # Errors
///
/// Returns an error when the server cannot start or exits unexpectedly.
pub async fn serve(listener: TcpListener, config: RealtimeGatewayConfig) -> Result<()> {
    let router = app(config)?;
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        service: "realtime-gateway",
        status: "ok",
    })
}

async fn realtime_websocket(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let context = RequestContext {
        request_id: new_id("req"),
        trace_id: extract_trace_id(&headers),
    };

    let parsed = parse_handshake_request(&uri).and_then(|request| {
        authenticate_and_validate(&state, &headers, &request).map(|session| (request, session))
    });

    match parsed {
        Ok((request, session)) => {
            info!(
                lifecycle = SessionLifecyclePhase::Connect.as_str(),
                request_id = context.request_id.as_str(),
                trace_id = context.trace_id.as_str(),
                model = request.model.as_str(),
                "received realtime websocket connect attempt"
            );

            let session_id = new_id("sess");
            info!(
                lifecycle = SessionLifecyclePhase::Accepted.as_str(),
                request_id = context.request_id.as_str(),
                trace_id = context.trace_id.as_str(),
                session_id = session_id.as_str(),
                tenant_id = session.tenant_id.as_str(),
                project_id = session.project_id.as_str(),
                subject = session.subject.as_str(),
                model = session.selected_model.as_str(),
                expires_at = session.expires_at_rfc3339(),
                max_duration_seconds = session.max_duration_seconds,
                billing_coupled = false,
                usage_boundary = "post_handshake_only",
                "accepted realtime websocket session"
            );

            let idle_timeout = state
                .config
                .idle_timeout
                .min(state.config.session_duration_cap);
            let upstream = state.upstream.clone();
            let RequestContext {
                request_id,
                trace_id,
            } = context;
            let response_request_id = request_id.clone();
            let response_trace_id = trace_id.clone();
            let response = ws.on_upgrade(move |socket| async move {
                run_session(
                    socket,
                    SessionContext {
                        request_id,
                        trace_id,
                        session_id,
                    },
                    session,
                    idle_timeout,
                    upstream,
                )
                .await;
            });

            with_context_headers(response, &response_request_id, &response_trace_id)
        }
        Err(rejection) => {
            warn!(
                lifecycle = SessionLifecyclePhase::Rejected.as_str(),
                request_id = context.request_id.as_str(),
                trace_id = context.trace_id.as_str(),
                code = rejection.code,
                status = rejection.status.as_u16(),
                message = rejection.message.as_str(),
                "rejected realtime websocket connect attempt"
            );

            let response = (
                rejection.status,
                Json(ErrorResponse {
                    request_id: &context.request_id,
                    trace_id: &context.trace_id,
                    error: ErrorBody {
                        code: rejection.code,
                        message: rejection.message,
                    },
                }),
            )
                .into_response();

            with_context_headers(response, &context.request_id, &context.trace_id)
        }
    }
}

fn authenticate_and_validate(
    state: &AppState,
    headers: &HeaderMap,
    request: &HandshakeRequest,
) -> Result<AcceptedRealtimeSession, HandshakeRejection> {
    let token = extract_bearer_token(headers)?;
    let claims = state
        .signer
        .parse(token)
        .map_err(|error| map_token_error_to_rejection(&error))?;
    let session = validate_handshake(
        &claims,
        request,
        protocol_realtime::RealtimeTransport::Websocket,
        protocol_realtime::now_epoch_seconds(),
    )
    .map_err(|error| map_validation_error_to_rejection(&error))?;

    if session.max_duration_seconds > state.config.session_duration_cap.as_secs() {
        return Err(HandshakeRejection::new(
            StatusCode::BAD_REQUEST,
            "request_validation_failed",
            format!(
                "requested session duration {} exceeds gateway cap {}",
                session.max_duration_seconds,
                state.config.session_duration_cap.as_secs()
            ),
        ));
    }

    Ok(session)
}

fn parse_handshake_request(uri: &Uri) -> Result<HandshakeRequest, HandshakeRejection> {
    let Some(query) = uri.query() else {
        return Err(HandshakeRejection::new(
            StatusCode::BAD_REQUEST,
            "request_validation_failed",
            "query parameter `model` is required",
        ));
    };

    serde_urlencoded::from_str::<HandshakeRequest>(query).map_err(|error| {
        HandshakeRejection::new(
            StatusCode::BAD_REQUEST,
            "request_validation_failed",
            format!("invalid realtime handshake query: {error}"),
        )
    })
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, HandshakeRejection> {
    let header_value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| {
            HandshakeRejection::new(
                StatusCode::UNAUTHORIZED,
                "auth_invalid",
                "missing Authorization bearer token",
            )
        })?
        .to_str()
        .map_err(|_| {
            HandshakeRejection::new(
                StatusCode::UNAUTHORIZED,
                "auth_invalid",
                "Authorization header must be valid ASCII",
            )
        })?;

    header_value
        .strip_prefix("Bearer ")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            HandshakeRejection::new(
                StatusCode::UNAUTHORIZED,
                "auth_invalid",
                "Authorization header must use Bearer token syntax",
            )
        })
}

fn extract_trace_id(headers: &HeaderMap) -> String {
    headers
        .get("x-trace-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| new_id("trace"), ToString::to_string)
}

fn map_token_error_to_rejection(error: &TokenError) -> HandshakeRejection {
    HandshakeRejection::new(StatusCode::UNAUTHORIZED, "auth_invalid", error.to_string())
}

fn map_validation_error_to_rejection(
    error: &protocol_realtime::SessionValidationError,
) -> HandshakeRejection {
    match error {
        protocol_realtime::SessionValidationError::MissingConnectScope
        | protocol_realtime::SessionValidationError::TransportMismatch { .. } => {
            HandshakeRejection::new(
                StatusCode::FORBIDDEN,
                "auth_insufficient_scope",
                error.to_string(),
            )
        }
        protocol_realtime::SessionValidationError::EmptyModel
        | protocol_realtime::SessionValidationError::InvalidDuration
        | protocol_realtime::SessionValidationError::ModelNotAllowed { .. }
        | protocol_realtime::SessionValidationError::DurationExceeded { .. } => {
            HandshakeRejection::new(
                StatusCode::BAD_REQUEST,
                "request_validation_failed",
                error.to_string(),
            )
        }
        protocol_realtime::SessionValidationError::Expired(_)
        | protocol_realtime::SessionValidationError::NotYetValid
        | protocol_realtime::SessionValidationError::MissingAllowedModels => {
            HandshakeRejection::new(StatusCode::UNAUTHORIZED, "auth_invalid", error.to_string())
        }
    }
}

fn with_context_headers(mut response: Response, request_id: &str, trace_id: &str) -> Response {
    if let Ok(header_value) = HeaderValue::from_str(request_id) {
        response.headers_mut().insert("x-request-id", header_value);
    }
    if let Ok(header_value) = HeaderValue::from_str(trace_id) {
        response.headers_mut().insert("x-trace-id", header_value);
    }
    response
}
