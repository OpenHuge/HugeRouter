use anyhow::{Context, Result};
use std::{env, net::SocketAddr, time::Duration};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8081";
const DEFAULT_IDLE_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_SESSION_DURATION_SECONDS: u64 = 900;
const DEFAULT_SIGNING_SECRET: &str = "dev-only-signing-secret-change-me";
const DEFAULT_STUB_RESPONSE_TEXT: &str =
    "HugeRouter realtime ingress MVP accepted the request, but no upstream model is wired yet.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamMode {
    Echo,
    FailAllResponses,
}

impl UpstreamMode {
    fn from_env(value: Option<&str>) -> Self {
        match value {
            Some("fail_all_responses") => Self::FailAllResponses,
            _ => Self::Echo,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RealtimeGatewayConfig {
    pub bind_addr: SocketAddr,
    pub signing_secret: String,
    pub idle_timeout: Duration,
    pub session_duration_cap: Duration,
    pub upstream_mode: UpstreamMode,
    pub stub_response_text: String,
    pub uses_insecure_default_secret: bool,
}

impl RealtimeGatewayConfig {
    /// # Errors
    ///
    /// Returns an error when environment configuration cannot be parsed.
    pub fn from_env() -> Result<Self> {
        let bind_addr = env::var("REALTIME_GATEWAY_BIND_ADDR")
            .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string())
            .parse::<SocketAddr>()
            .with_context(|| format!("invalid REALTIME_GATEWAY_BIND_ADDR: {DEFAULT_BIND_ADDR}"))?;

        let signing_secret = env::var("REALTIME_GATEWAY_SIGNING_SECRET")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_SIGNING_SECRET.to_string());
        let uses_insecure_default_secret = signing_secret == DEFAULT_SIGNING_SECRET;

        let idle_timeout_seconds = parse_u64_env(
            "REALTIME_GATEWAY_IDLE_TIMEOUT_SECONDS",
            DEFAULT_IDLE_TIMEOUT_SECONDS,
        )?;
        let session_duration_cap_seconds = parse_u64_env(
            "REALTIME_GATEWAY_MAX_SESSION_DURATION_SECONDS",
            DEFAULT_MAX_SESSION_DURATION_SECONDS,
        )?;

        Ok(Self {
            bind_addr,
            signing_secret,
            idle_timeout: Duration::from_secs(idle_timeout_seconds),
            session_duration_cap: Duration::from_secs(session_duration_cap_seconds),
            upstream_mode: UpstreamMode::from_env(
                env::var("REALTIME_GATEWAY_UPSTREAM_MODE").ok().as_deref(),
            ),
            stub_response_text: env::var("REALTIME_GATEWAY_STUB_RESPONSE_TEXT")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_STUB_RESPONSE_TEXT.to_string()),
            uses_insecure_default_secret,
        })
    }
}

fn parse_u64_env(key: &str, fallback: u64) -> Result<u64> {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            value
                .parse::<u64>()
                .with_context(|| format!("invalid {key}: {value}"))
        })
        .transpose()
        .map(|value| value.unwrap_or(fallback))
}
