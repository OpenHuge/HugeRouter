use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::{PgPool, types::Json};

#[derive(Debug, Clone)]
pub struct AuditMessageEnvelope {
    pub message_type: String,
    pub message_id: String,
    pub occurred_at: Option<String>,
    pub producer: String,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub raw_payload: Value,
}

#[derive(Debug, Clone)]
pub struct AuditHandleOutcome {
    pub envelope: AuditMessageEnvelope,
    pub persisted: bool,
    pub pruned_rows: u64,
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
    let occurred_at = value
        .get("occurred_at")
        .and_then(Value::as_str)
        .map(ToString::to_string);

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
        occurred_at,
        producer,
        trace_id,
        request_id,
        raw_payload: value,
    })
}

pub async fn ensure_audit_table(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS audit_events (
            message_id TEXT PRIMARY KEY,
            message_type TEXT NOT NULL,
            producer TEXT NOT NULL,
            occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            trace_id TEXT NULL,
            request_id TEXT NULL,
            raw_payload JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        ",
    )
    .execute(pool)
    .await
    .context("creating audit_events table failed")?;

    Ok(())
}

pub async fn prune_audit_events(pool: &PgPool, retention_days: u64) -> Result<u64> {
    let retention_days = i32::try_from(retention_days).context("retention days exceeds i32")?;
    let result = sqlx::query(
        r"
        DELETE FROM audit_events
        WHERE occurred_at < NOW() - make_interval(days => $1)
        ",
    )
    .bind(retention_days)
    .execute(pool)
    .await
    .context("pruning expired audit events failed")?;

    Ok(result.rows_affected())
}

pub async fn persist_audit_envelope(
    pool: &PgPool,
    envelope: &AuditMessageEnvelope,
) -> Result<bool> {
    let result = sqlx::query(
        r"
        INSERT INTO audit_events (
            message_id,
            message_type,
            producer,
            occurred_at,
            trace_id,
            request_id,
            raw_payload
        )
        VALUES ($1, $2, $3, COALESCE($4::timestamptz, NOW()), $5, $6, $7)
        ON CONFLICT (message_id) DO NOTHING
        ",
    )
    .bind(&envelope.message_id)
    .bind(&envelope.message_type)
    .bind(&envelope.producer)
    .bind(envelope.occurred_at.as_deref())
    .bind(&envelope.trace_id)
    .bind(&envelope.request_id)
    .bind(Json(envelope.raw_payload.clone()))
    .execute(pool)
    .await
    .context("writing audit event failed")?;

    Ok(result.rows_affected() > 0)
}

pub async fn handle_audit_envelope(
    pool: &PgPool,
    payload: &[u8],
    retention_days: u64,
) -> Result<AuditHandleOutcome> {
    let envelope = parse_audit_envelope(payload)?;
    let pruned_rows = prune_audit_events(pool, retention_days).await?;
    let persisted = persist_audit_envelope(pool, &envelope).await?;

    Ok(AuditHandleOutcome {
        envelope,
        persisted,
        pruned_rows,
    })
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
        assert_eq!(envelope.occurred_at, None);
    }

    #[test]
    fn rejects_payload_without_required_fields() {
        let payload = br#"{"message_type":"audit_event.created"}"#;
        assert!(parse_audit_envelope(payload).is_err());
    }

    #[test]
    fn preserves_optional_request_metadata() {
        let payload = br#"{
            "message_type":"audit_event.created",
            "message_id":"msg_2",
            "producer":"control-plane-api",
            "occurred_at":"2026-04-23T00:00:00Z",
            "trace_id":"trace_123",
            "request_id":"req_123"
        }"#;

        let envelope = parse_audit_envelope(payload).expect("parse");
        assert_eq!(
            envelope.occurred_at.as_deref(),
            Some("2026-04-23T00:00:00Z")
        );
        assert_eq!(envelope.trace_id.as_deref(), Some("trace_123"));
        assert_eq!(envelope.request_id.as_deref(), Some("req_123"));
    }
}
