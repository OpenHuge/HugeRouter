use core_domain::{
    ConfigSnapshot, LedgerEntry, ServiceName, UsageEvent,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolFamily {
    BootstrapPlaceholder,
    OpenAiChat,
    OpenAiResponses,
    McpStreamableHttp,
    RealtimeWebRtc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    UsageEventRecorded,
    LedgerEntryCreated,
    BudgetThresholdExceeded,
    ProviderResourceQuarantined,
    ProviderResourceDegraded,
    AuditEventCreated,
    ConfigSnapshotActivated,
    ReplayCapsuleBuildRequested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub protocol_family: ProtocolFamily,
    pub source_service: ServiceName,
    pub request_id: String,
    pub trace_id: String,
}

impl RequestEnvelope {
    #[must_use]
    pub fn bootstrap() -> Self {
        Self {
            protocol_family: ProtocolFamily::BootstrapPlaceholder,
            source_service: ServiceName::from("gateway-api"),
            request_id: "req_bootstrap".to_string(),
            trace_id: "trace_bootstrap".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub message_id: String,
    pub message_type: MessageType,
    pub schema_version: u16,
    pub occurred_at: String,
    pub producer: ServiceName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub idempotency_key: String,
    pub payload: T,
}

impl<T> MessageEnvelope<T> {
    #[must_use]
    pub fn new(
        message_id: impl Into<String>,
        message_type: MessageType,
        occurred_at: impl Into<String>,
        producer: ServiceName,
        idempotency_key: impl Into<String>,
        payload: T,
    ) -> Self {
        Self {
            message_id: message_id.into(),
            message_type,
            schema_version: 1,
            occurred_at: occurred_at.into(),
            producer,
            trace_id: None,
            request_id: None,
            idempotency_key: idempotency_key.into(),
            payload,
        }
    }

    #[must_use]
    pub fn with_request_context(
        mut self,
        trace_id: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        self.trace_id = Some(trace_id.into());
        self.request_id = Some(request_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEventRecorded {
    pub usage_event: UsageEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntryCreated {
    pub ledger_entry: LedgerEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSnapshotActivated {
    pub config_snapshot: ConfigSnapshot,
}

#[cfg(test)]
mod tests {
    use super::{
        ConfigSnapshotActivated, MessageEnvelope, MessageType, ProtocolFamily, RequestEnvelope,
    };
    use core_domain::ConfigSnapshot;

    #[test]
    fn request_envelope_bootstrap_uses_documented_defaults() {
        let envelope = RequestEnvelope::bootstrap();

        assert_eq!(envelope.protocol_family, ProtocolFamily::BootstrapPlaceholder);
        assert_eq!(envelope.source_service.0, "gateway-api");
        assert_eq!(envelope.request_id, "req_bootstrap");
    }

    #[test]
    fn message_envelope_includes_schema_version_and_request_context() {
        let payload = ConfigSnapshotActivated {
            config_snapshot: ConfigSnapshot {
                config_snapshot_id: core_domain::ConfigSnapshotId::parse("cfgsnap_123").unwrap(),
                activated_at: "2026-04-20T00:00:00Z".to_string(),
                revision: 1,
            },
        };
        let envelope = MessageEnvelope::new(
            "msg_123",
            MessageType::ConfigSnapshotActivated,
            "2026-04-20T00:00:00Z",
            core_domain::ServiceName::from("control-plane-api"),
            "cfgsnap_123:activated",
            payload,
        )
        .with_request_context("trace_123", "req_123");

        let json = serde_json::to_value(&envelope).unwrap();

        assert_eq!(json["message_type"], "config_snapshot_activated");
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["trace_id"], "trace_123");
        assert_eq!(json["payload"]["config_snapshot"]["config_snapshot_id"], "cfgsnap_123");
    }
}
