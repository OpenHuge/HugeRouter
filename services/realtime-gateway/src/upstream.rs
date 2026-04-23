use crate::config::{RealtimeGatewayConfig, UpstreamMode};
use async_trait::async_trait;
use protocol_realtime::{AcceptedRealtimeSession, ResponseCreateInput};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamResponse {
    pub output_text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UpstreamError {
    #[error("no realtime upstream is configured for this gateway slice")]
    NoRealtimeUpstream,
}

#[async_trait]
pub trait UpstreamConnector: Send + Sync {
    async fn create_response(
        &self,
        session: &AcceptedRealtimeSession,
        request: &ResponseCreateInput,
    ) -> Result<UpstreamResponse, UpstreamError>;
}

pub fn build_upstream_connector(
    config: &RealtimeGatewayConfig,
) -> Arc<dyn UpstreamConnector + Send + Sync> {
    match config.upstream_mode {
        UpstreamMode::Echo => Arc::new(EchoUpstream {
            default_text: config.stub_response_text.clone(),
        }),
        UpstreamMode::FailAllResponses => Arc::new(FailingUpstream),
    }
}

struct EchoUpstream {
    default_text: String,
}

#[async_trait]
impl UpstreamConnector for EchoUpstream {
    async fn create_response(
        &self,
        session: &AcceptedRealtimeSession,
        request: &ResponseCreateInput,
    ) -> Result<UpstreamResponse, UpstreamError> {
        let output_text = request
            .input_text
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("echo:{}:{value}", session.selected_model))
            .or_else(|| {
                request
                    .instructions
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| self.default_text.clone());

        Ok(UpstreamResponse { output_text })
    }
}

struct FailingUpstream;

#[async_trait]
impl UpstreamConnector for FailingUpstream {
    async fn create_response(
        &self,
        _session: &AcceptedRealtimeSession,
        _request: &ResponseCreateInput,
    ) -> Result<UpstreamResponse, UpstreamError> {
        Err(UpstreamError::NoRealtimeUpstream)
    }
}
