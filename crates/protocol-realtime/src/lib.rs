use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const REALTIME_CONNECT_SCOPE: &str = "realtime.connect";
pub const EPHEMERAL_TOKEN_PREFIX: &str = "hrt1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeTransport {
    Websocket,
    Webrtc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLifecyclePhase {
    Connect,
    Accepted,
    Rejected,
    Closed,
    UpstreamFailed,
}

impl SessionLifecyclePhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Closed => "closed",
            Self::UpstreamFailed => "upstream-failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCloseReason {
    ClientClosed,
    IdleTimeout,
    ServerShutdown,
    ProtocolError,
    UpstreamFailed,
}

impl SessionCloseReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClientClosed => "client_closed",
            Self::IdleTimeout => "idle_timeout",
            Self::ServerShutdown => "server_shutdown",
            Self::ProtocolError => "protocol_error",
            Self::UpstreamFailed => "upstream_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowedSessionParams {
    pub model_aliases: Vec<String>,
    pub max_duration_seconds: u64,
    pub max_concurrency: u16,
}

impl Default for AllowedSessionParams {
    fn default() -> Self {
        Self {
            model_aliases: vec!["realtime-default".to_string()],
            max_duration_seconds: 900,
            max_concurrency: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EphemeralSessionTokenClaims {
    pub subject: String,
    pub tenant_id: String,
    pub project_id: String,
    pub scopes: Vec<String>,
    pub transport: RealtimeTransport,
    pub expires_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_before: Option<i64>,
    #[serde(default)]
    pub allowed_session_params: AllowedSessionParams,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
}

impl EphemeralSessionTokenClaims {
    #[must_use]
    pub fn expires_at_rfc3339(&self) -> String {
        timestamp_to_rfc3339(self.expires_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HandshakeRequest {
    pub model: String,
    #[serde(default)]
    pub session_max_duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcceptedRealtimeSession {
    pub subject: String,
    pub tenant_id: String,
    pub project_id: String,
    pub selected_model: String,
    pub transport: RealtimeTransport,
    pub expires_at: i64,
    pub max_duration_seconds: u64,
    pub max_concurrency: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
}

impl AcceptedRealtimeSession {
    #[must_use]
    pub fn expires_at_rfc3339(&self) -> String {
        timestamp_to_rfc3339(self.expires_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEnvelope {
    pub id: String,
    pub status: String,
    pub model: String,
    pub tenant_id: String,
    pub project_id: String,
    pub subject: String,
    pub request_id: String,
    pub trace_id: String,
    pub expires_at: String,
    pub max_duration_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseCreateInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "ping")]
    Ping {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
    },
    #[serde(rename = "response.create")]
    ResponseCreate { response: ResponseCreateInput },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub id: String,
    pub status: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "session.created")]
    SessionCreated {
        event_id: String,
        session: SessionEnvelope,
    },
    #[serde(rename = "pong")]
    Pong { event_id: String, timestamp: String },
    #[serde(rename = "response.created")]
    ResponseCreated {
        event_id: String,
        response: ResponseEnvelope,
    },
    #[serde(rename = "response.output_text.delta")]
    ResponseOutputTextDelta {
        event_id: String,
        response_id: String,
        delta: String,
    },
    #[serde(rename = "response.completed")]
    ResponseCompleted {
        event_id: String,
        response: ResponseEnvelope,
    },
    #[serde(rename = "error")]
    Error {
        event_id: String,
        error: ErrorEnvelope,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    #[error("signing secret must not be empty")]
    EmptySecret,
    #[error("ephemeral token must not be empty")]
    EmptyToken,
    #[error("invalid token format")]
    InvalidFormat,
    #[error("invalid token prefix")]
    InvalidPrefix,
    #[error("invalid token signature")]
    InvalidSignature,
    #[error("invalid token payload: {0}")]
    InvalidPayload(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionValidationError {
    #[error("token expired at {0}")]
    Expired(String),
    #[error("token is not valid yet")]
    NotYetValid,
    #[error("token missing required scope realtime.connect")]
    MissingConnectScope,
    #[error("token transport does not allow {requested:?}")]
    TransportMismatch { requested: RealtimeTransport },
    #[error("model must not be empty")]
    EmptyModel,
    #[error("requested session duration must be greater than zero")]
    InvalidDuration,
    #[error("requested model `{requested}` is not allowed by token scope")]
    ModelNotAllowed { requested: String },
    #[error("requested session duration {requested} exceeds token cap {allowed}")]
    DurationExceeded { requested: u64, allowed: u64 },
    #[error("token must declare at least one allowed model alias")]
    MissingAllowedModels,
}

#[derive(Debug, Clone)]
pub struct EphemeralTokenSigner {
    secret: Vec<u8>,
}

impl EphemeralTokenSigner {
    /// # Errors
    ///
    /// Returns an error when the signing secret is empty.
    pub fn new(secret: impl Into<Vec<u8>>) -> Result<Self, TokenError> {
        let secret = secret.into();
        if secret.is_empty() {
            return Err(TokenError::EmptySecret);
        }
        Ok(Self { secret })
    }

    /// # Errors
    ///
    /// Returns an error when the claims cannot be serialized.
    pub fn sign(&self, claims: &EphemeralSessionTokenClaims) -> Result<String, TokenError> {
        let payload = serde_json::to_vec(claims)
            .map_err(|error| TokenError::InvalidPayload(error.to_string()))?;
        let payload = URL_SAFE_NO_PAD.encode(payload);

        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| TokenError::EmptySecret)?;
        mac.update(payload.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        Ok(format!("{EPHEMERAL_TOKEN_PREFIX}.{payload}.{signature}"))
    }

    /// # Errors
    ///
    /// Returns an error when the token format, signature, or payload is invalid.
    pub fn parse(&self, token: &str) -> Result<EphemeralSessionTokenClaims, TokenError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(TokenError::EmptyToken);
        }

        let mut parts = token.split('.');
        let Some(prefix) = parts.next() else {
            return Err(TokenError::InvalidFormat);
        };
        let Some(payload) = parts.next() else {
            return Err(TokenError::InvalidFormat);
        };
        let Some(signature) = parts.next() else {
            return Err(TokenError::InvalidFormat);
        };
        if parts.next().is_some() {
            return Err(TokenError::InvalidFormat);
        }
        if prefix != EPHEMERAL_TOKEN_PREFIX {
            return Err(TokenError::InvalidPrefix);
        }

        let supplied_signature = URL_SAFE_NO_PAD
            .decode(signature.as_bytes())
            .map_err(|_| TokenError::InvalidSignature)?;

        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| TokenError::EmptySecret)?;
        mac.update(payload.as_bytes());
        mac.verify_slice(&supplied_signature)
            .map_err(|_| TokenError::InvalidSignature)?;

        let payload = URL_SAFE_NO_PAD
            .decode(payload.as_bytes())
            .map_err(|error| TokenError::InvalidPayload(error.to_string()))?;

        serde_json::from_slice(&payload)
            .map_err(|error| TokenError::InvalidPayload(error.to_string()))
    }
}

/// # Errors
///
/// Returns an error when token scope, timing, transport, or requested
/// parameters violate the allowed realtime session contract.
pub fn validate_handshake(
    claims: &EphemeralSessionTokenClaims,
    request: &HandshakeRequest,
    transport: RealtimeTransport,
    now_epoch_seconds: i64,
) -> Result<AcceptedRealtimeSession, SessionValidationError> {
    if now_epoch_seconds >= claims.expires_at {
        return Err(SessionValidationError::Expired(claims.expires_at_rfc3339()));
    }

    if let Some(not_before) = claims.not_before
        && now_epoch_seconds < not_before
    {
        return Err(SessionValidationError::NotYetValid);
    }

    if !claims
        .scopes
        .iter()
        .any(|scope| scope == REALTIME_CONNECT_SCOPE)
    {
        return Err(SessionValidationError::MissingConnectScope);
    }

    if claims.transport != transport {
        return Err(SessionValidationError::TransportMismatch {
            requested: transport,
        });
    }

    let requested_model = request.model.trim();
    if requested_model.is_empty() {
        return Err(SessionValidationError::EmptyModel);
    }

    if claims.allowed_session_params.model_aliases.is_empty() {
        return Err(SessionValidationError::MissingAllowedModels);
    }

    let model_allowed = claims
        .allowed_session_params
        .model_aliases
        .iter()
        .any(|allowed| allowed == "*" || allowed == requested_model);
    if !model_allowed {
        return Err(SessionValidationError::ModelNotAllowed {
            requested: requested_model.to_string(),
        });
    }

    let requested_duration = request
        .session_max_duration_seconds
        .unwrap_or(claims.allowed_session_params.max_duration_seconds);
    if requested_duration == 0 {
        return Err(SessionValidationError::InvalidDuration);
    }
    if requested_duration > claims.allowed_session_params.max_duration_seconds {
        return Err(SessionValidationError::DurationExceeded {
            requested: requested_duration,
            allowed: claims.allowed_session_params.max_duration_seconds,
        });
    }

    Ok(AcceptedRealtimeSession {
        subject: claims.subject.clone(),
        tenant_id: claims.tenant_id.clone(),
        project_id: claims.project_id.clone(),
        selected_model: requested_model.to_string(),
        transport,
        expires_at: claims.expires_at,
        max_duration_seconds: requested_duration,
        max_concurrency: claims.allowed_session_params.max_concurrency,
        token_id: claims.token_id.clone(),
    })
}

#[must_use]
pub fn now_epoch_seconds() -> i64 {
    Utc::now().timestamp()
}

#[must_use]
pub fn timestamp_to_rfc3339(timestamp: i64) -> String {
    chrono::DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map_or_else(
            || "1970-01-01T00:00:00Z".to_string(),
            |date_time| date_time.to_rfc3339_opts(SecondsFormat::Secs, true),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        AllowedSessionParams, EPHEMERAL_TOKEN_PREFIX, EphemeralSessionTokenClaims,
        EphemeralTokenSigner, HandshakeRequest, REALTIME_CONNECT_SCOPE, RealtimeTransport,
        SessionValidationError, validate_handshake,
    };

    fn claims() -> EphemeralSessionTokenClaims {
        EphemeralSessionTokenClaims {
            subject: "browser-client".to_string(),
            tenant_id: "tenant-test".to_string(),
            project_id: "project-test".to_string(),
            scopes: vec![REALTIME_CONNECT_SCOPE.to_string()],
            transport: RealtimeTransport::Websocket,
            expires_at: 1_900_000_000,
            not_before: Some(1_800_000_000),
            allowed_session_params: AllowedSessionParams {
                model_aliases: vec!["realtime-default".to_string()],
                max_duration_seconds: 60,
                max_concurrency: 1,
            },
            token_id: Some("tok_123".to_string()),
        }
    }

    #[test]
    fn signer_round_trips_claims() {
        let signer = EphemeralTokenSigner::new("secret").expect("secret should be accepted");
        let token = signer.sign(&claims()).expect("claims should sign");
        assert!(token.starts_with(EPHEMERAL_TOKEN_PREFIX));

        let parsed = signer.parse(&token).expect("token should parse");
        assert_eq!(parsed, claims());
    }

    #[test]
    fn handshake_validation_enforces_scope_and_limits() {
        let request = HandshakeRequest {
            model: "realtime-default".to_string(),
            session_max_duration_seconds: Some(30),
        };

        let accepted = validate_handshake(
            &claims(),
            &request,
            RealtimeTransport::Websocket,
            1_850_000_000,
        )
        .expect("handshake should be accepted");
        assert_eq!(accepted.selected_model, "realtime-default");
        assert_eq!(accepted.max_duration_seconds, 30);
    }

    #[test]
    fn handshake_validation_rejects_model_outside_scope() {
        let request = HandshakeRequest {
            model: "gpt-4o-realtime".to_string(),
            session_max_duration_seconds: None,
        };

        let error = validate_handshake(
            &claims(),
            &request,
            RealtimeTransport::Websocket,
            1_850_000_000,
        )
        .expect_err("handshake should reject model");
        assert_eq!(
            error,
            SessionValidationError::ModelNotAllowed {
                requested: "gpt-4o-realtime".to_string()
            }
        );
    }
}
