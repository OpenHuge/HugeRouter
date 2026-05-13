use crate::ids::new_id;
use crate::upstream::{UpstreamConnector, UpstreamError};
use axum::extract::ws::{CloseFrame, Message, WebSocket, close_code};
use futures_util::StreamExt;
use protocol_realtime::{
    AcceptedRealtimeSession, ClientEvent, ErrorEnvelope, ResponseCreateInput, ResponseEnvelope,
    ServerEvent, SessionCloseReason, SessionEnvelope, SessionLifecyclePhase, timestamp_to_rfc3339,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct SessionContext {
    pub request_id: String,
    pub trace_id: String,
    pub session_id: String,
}

#[allow(clippy::too_many_lines)]
pub async fn run_session(
    mut socket: WebSocket,
    context: SessionContext,
    session: AcceptedRealtimeSession,
    idle_timeout: Duration,
    upstream: Arc<dyn UpstreamConnector + Send + Sync>,
) {
    let session_created = ServerEvent::SessionCreated {
        event_id: new_id("evt"),
        session: SessionEnvelope {
            id: context.session_id.clone(),
            status: "accepted".to_string(),
            model: session.selected_model.clone(),
            tenant_id: session.tenant_id.clone(),
            project_id: session.project_id.clone(),
            subject: session.subject.clone(),
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
            expires_at: session.expires_at_rfc3339(),
            max_duration_seconds: session.max_duration_seconds,
        },
    };

    if let Err(error) = send_event(&mut socket, &session_created).await {
        warn!(
            lifecycle = SessionLifecyclePhase::Closed.as_str(),
            request_id = context.request_id.as_str(),
            trace_id = context.trace_id.as_str(),
            session_id = context.session_id.as_str(),
            close_reason = SessionCloseReason::ProtocolError.as_str(),
            error = %error,
            "failed to send session.created event"
        );
        return;
    }

    loop {
        let next_frame = tokio::time::timeout(idle_timeout, socket.next()).await;
        match next_frame {
            Err(_) => {
                info!(
                    lifecycle = SessionLifecyclePhase::Closed.as_str(),
                    request_id = context.request_id.as_str(),
                    trace_id = context.trace_id.as_str(),
                    session_id = context.session_id.as_str(),
                    close_reason = SessionCloseReason::IdleTimeout.as_str(),
                    "closing idle realtime session"
                );
                let _ = socket
                    .send(Message::Close(Some(CloseFrame {
                        code: close_code::POLICY,
                        reason: "idle timeout".into(),
                    })))
                    .await;
                break;
            }
            Ok(None) => {
                info!(
                    lifecycle = SessionLifecyclePhase::Closed.as_str(),
                    request_id = context.request_id.as_str(),
                    trace_id = context.trace_id.as_str(),
                    session_id = context.session_id.as_str(),
                    close_reason = SessionCloseReason::ClientClosed.as_str(),
                    "client disconnected realtime session"
                );
                break;
            }
            Ok(Some(Err(error))) => {
                warn!(
                    lifecycle = SessionLifecyclePhase::Closed.as_str(),
                    request_id = context.request_id.as_str(),
                    trace_id = context.trace_id.as_str(),
                    session_id = context.session_id.as_str(),
                    close_reason = SessionCloseReason::ProtocolError.as_str(),
                    error = %error,
                    "websocket transport error"
                );
                break;
            }
            Ok(Some(Ok(Message::Close(frame)))) => {
                info!(
                    lifecycle = SessionLifecyclePhase::Closed.as_str(),
                    request_id = context.request_id.as_str(),
                    trace_id = context.trace_id.as_str(),
                    session_id = context.session_id.as_str(),
                    close_reason = SessionCloseReason::ClientClosed.as_str(),
                    close_code = frame.as_ref().map_or(1000, |value| value.code),
                    "client closed realtime session"
                );
                break;
            }
            Ok(Some(Ok(Message::Ping(payload)))) => {
                let _ = socket.send(Message::Pong(payload)).await;
            }
            Ok(Some(Ok(Message::Pong(_)))) => {}
            Ok(Some(Ok(Message::Binary(_)))) => {
                let error_event = ServerEvent::Error {
                    event_id: new_id("evt"),
                    error: ErrorEnvelope {
                        code: "request_validation_failed".to_string(),
                        message: "binary websocket frames are not supported by this MVP"
                            .to_string(),
                        retryable: false,
                    },
                };
                let _ = send_event(&mut socket, &error_event).await;
            }
            Ok(Some(Ok(Message::Text(payload)))) => {
                if handle_text_message(&mut socket, &context, &session, &payload, upstream.as_ref())
                    .await
                {
                    break;
                }
            }
        }
    }
}

async fn handle_text_message(
    socket: &mut WebSocket,
    context: &SessionContext,
    session: &AcceptedRealtimeSession,
    payload: &str,
    upstream: &(dyn UpstreamConnector + Send + Sync),
) -> bool {
    let event = match serde_json::from_str::<ClientEvent>(payload) {
        Ok(event) => event,
        Err(error) => {
            let error_event = ServerEvent::Error {
                event_id: new_id("evt"),
                error: ErrorEnvelope {
                    code: "request_validation_failed".to_string(),
                    message: format!("invalid client event payload: {error}"),
                    retryable: false,
                },
            };
            let _ = send_event(socket, &error_event).await;
            return false;
        }
    };

    match event {
        ClientEvent::Ping { .. } => {
            let pong = ServerEvent::Pong {
                event_id: new_id("evt"),
                timestamp: timestamp_to_rfc3339(chrono::Utc::now().timestamp()),
            };
            let _ = send_event(socket, &pong).await;
            false
        }
        ClientEvent::ResponseCreate { response } => {
            handle_response_create(socket, context, session, response, upstream).await
        }
    }
}

async fn handle_response_create(
    socket: &mut WebSocket,
    context: &SessionContext,
    session: &AcceptedRealtimeSession,
    response: ResponseCreateInput,
    upstream: &(dyn UpstreamConnector + Send + Sync),
) -> bool {
    let response_id = new_id("resp");
    let created = ServerEvent::ResponseCreated {
        event_id: new_id("evt"),
        response: ResponseEnvelope {
            id: response_id.clone(),
            status: "in_progress".to_string(),
            model: session.selected_model.clone(),
        },
    };

    if send_event(socket, &created).await.is_err() {
        return true;
    }

    match upstream.create_response(session, &response).await {
        Ok(output) => {
            let delta = ServerEvent::ResponseOutputTextDelta {
                event_id: new_id("evt"),
                response_id: response_id.clone(),
                delta: output.output_text,
            };
            let completed = ServerEvent::ResponseCompleted {
                event_id: new_id("evt"),
                response: ResponseEnvelope {
                    id: response_id,
                    status: "completed".to_string(),
                    model: session.selected_model.clone(),
                },
            };

            if send_event(socket, &delta).await.is_err() {
                return true;
            }
            let _ = send_event(socket, &completed).await;
            false
        }
        Err(error) => close_after_upstream_failure(socket, context, error).await,
    }
}

async fn close_after_upstream_failure(
    socket: &mut WebSocket,
    context: &SessionContext,
    error: UpstreamError,
) -> bool {
    error!(
        lifecycle = SessionLifecyclePhase::UpstreamFailed.as_str(),
        request_id = context.request_id.as_str(),
        trace_id = context.trace_id.as_str(),
        session_id = context.session_id.as_str(),
        error = %error,
        "realtime upstream execution failed"
    );

    let error_event = ServerEvent::Error {
        event_id: new_id("evt"),
        error: ErrorEnvelope {
            code: "provider_unavailable".to_string(),
            message: error.to_string(),
            retryable: true,
        },
    };
    let _ = send_event(socket, &error_event).await;
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: close_code::ERROR,
            reason: "upstream failure".into(),
        })))
        .await;

    info!(
        lifecycle = SessionLifecyclePhase::Closed.as_str(),
        request_id = context.request_id.as_str(),
        trace_id = context.trace_id.as_str(),
        session_id = context.session_id.as_str(),
        close_reason = SessionCloseReason::UpstreamFailed.as_str(),
        "closed realtime session after upstream failure"
    );
    true
}

async fn send_event(socket: &mut WebSocket, event: &ServerEvent) -> Result<(), axum::Error> {
    let payload = serde_json::to_string(event).expect("server events should serialize");
    socket.send(Message::Text(payload.into())).await
}
