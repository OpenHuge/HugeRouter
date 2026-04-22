use anyhow::{Context, Result};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AuditMessageEnvelope {
    pub message_type: String,
    pub message_id: String,
    pub producer: String,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
}

pub fn parse_audit_envelope(payload: &[u8]) -> Result<AuditMessageEnvelope> {
    let value: Value =
        serde_json::from_slice(payload).context("invalid audit worker message payload")?;
    let message_type = value
        .get("message_type")
        .and_then(Value::as_str)
        .context("missing field message_type")?
        .to_string();
    let message_id = value
        .get("message_id")
        .and_then(Value::as_str)
        .context("missing field message_id")?
        .to_string();
    let producer = value
        .get("producer")
        .and_then(Value::as_str)
        .context("missing field producer")?
        .to_string();

    let trace_id = value
        .get("trace_id")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let request_id = value
        .get("request_id")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    Ok(AuditMessageEnvelope {
        message_type,
        message_id,
        producer,
        trace_id,
        request_id,
    })
}

pub fn handle_audit_envelope(payload: &[u8]) -> Result<AuditMessageEnvelope> {
    parse_audit_envelope(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_envelope_metadata() {
        let payload = br#"{"message_type":"audit_event.created","message_id":"msg_1","producer":"gateway-api"}"#;
        let envelope = parse_audit_envelope(payload).expect("parse");
        assert_eq!(envelope.message_type, "audit_event.created");
        assert_eq!(envelope.message_id, "msg_1");
        assert_eq!(envelope.producer, "gateway-api");
    }

    #[test]
    fn rejects_payload_without_required_fields() {
        let payload = br#"{"message_type":"audit_event.created"}"#;
        assert!(parse_audit_envelope(payload).is_err());
    }
}
