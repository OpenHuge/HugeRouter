use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::{PgPool, types::Json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentSeverity {
    Info,
    Warning,
    Critical,
}

impl IncidentSeverity {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationEnvelope {
    pub message_id: String,
    pub message_type: String,
    pub occurred_at: Option<String>,
    pub producer: String,
    pub provider_resource_id: Option<String>,
    pub severity: IncidentSeverity,
    pub incident_key: String,
    pub summary: String,
    pub raw_payload: Value,
}

#[derive(Debug, Clone)]
pub struct NotificationHandleOutcome {
    pub envelope: NotificationEnvelope,
    pub persisted: bool,
    pub suppressed: bool,
}

fn payload_string(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn infer_severity(message_type: &str, payload: &Value) -> IncidentSeverity {
    if let Some(explicit) = payload
        .get("severity")
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
    {
        return match explicit.as_str() {
            "critical" | "high" => IncidentSeverity::Critical,
            "warning" | "medium" => IncidentSeverity::Warning,
            _ => IncidentSeverity::Info,
        };
    }

    match message_type {
        "provider_resource.quarantined" | "budget_threshold.exceeded" => {
            IncidentSeverity::Critical
        }
        "provider_resource.degraded" => IncidentSeverity::Warning,
        _ => IncidentSeverity::Info,
    }
}

fn incident_summary(
    message_type: &str,
    provider_resource_id: Option<&str>,
    payload: &Value,
) -> String {
    let reason = payload_string(payload, "reason");
    match (message_type, provider_resource_id) {
        ("provider_resource.quarantined", Some(provider_resource_id)) => format!(
            "Provider {provider_resource_id} was quarantined{}",
            reason
                .as_deref()
                .map(|value| format!(": {value}"))
                .unwrap_or_default()
        ),
        ("provider_resource.degraded", Some(provider_resource_id)) => format!(
            "Provider {provider_resource_id} degraded{}",
            reason
                .as_deref()
                .map(|value| format!(": {value}"))
                .unwrap_or_default()
        ),
        _ => reason.unwrap_or_else(|| format!("Received incident {message_type}")),
    }
}

pub fn parse_notification_envelope(payload: &[u8]) -> Result<NotificationEnvelope> {
    let raw_payload: Value =
        serde_json::from_slice(payload).context("invalid notification worker message payload")?;
    let message_type = raw_payload
        .get("message_type")
        .and_then(Value::as_str)
        .context("missing field message_type")?
        .to_string();
    let message_id = raw_payload
        .get("message_id")
        .and_then(Value::as_str)
        .context("missing field message_id")?
        .to_string();
    let producer = raw_payload
        .get("producer")
        .and_then(Value::as_str)
        .context("missing field producer")?
        .to_string();
    let occurred_at = raw_payload
        .get("occurred_at")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let nested_payload = raw_payload
        .get("payload")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    let provider_resource_id = payload_string(&nested_payload, "provider_resource_id");
    let severity = infer_severity(&message_type, &nested_payload);
    let incident_key = format!(
        "{}:{}",
        message_type,
        provider_resource_id.as_deref().unwrap_or("global")
    );

    Ok(NotificationEnvelope {
        message_id,
        message_type: message_type.clone(),
        occurred_at,
        producer,
        provider_resource_id: provider_resource_id.clone(),
        severity,
        incident_key,
        summary: incident_summary(&message_type, provider_resource_id.as_deref(), &nested_payload),
        raw_payload,
    })
}

pub async fn ensure_notification_tables(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notification_events (
            message_id TEXT PRIMARY KEY,
            incident_key TEXT NOT NULL,
            message_type TEXT NOT NULL,
            severity TEXT NOT NULL,
            producer TEXT NOT NULL,
            provider_resource_id TEXT NULL,
            summary TEXT NOT NULL,
            occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            suppressed BOOLEAN NOT NULL DEFAULT FALSE,
            raw_payload JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .context("creating notification_events table failed")?;

    Ok(())
}

async fn recently_notified(
    pool: &PgPool,
    incident_key: &str,
    rate_limit_seconds: u64,
) -> Result<bool> {
    let rate_limit_seconds =
        i64::try_from(rate_limit_seconds).context("rate limit seconds exceeds i64")?;
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM notification_events
        WHERE incident_key = $1
          AND suppressed = FALSE
          AND created_at >= NOW() - make_interval(secs => $2)
        "#,
    )
    .bind(incident_key)
    .bind(rate_limit_seconds)
    .fetch_one(pool)
    .await
    .context("querying notification rate limit window failed")?;

    Ok(count > 0)
}

pub async fn persist_notification_event(
    pool: &PgPool,
    envelope: &NotificationEnvelope,
    suppressed: bool,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO notification_events (
            message_id,
            incident_key,
            message_type,
            severity,
            producer,
            provider_resource_id,
            summary,
            occurred_at,
            suppressed,
            raw_payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8::timestamptz, NOW()), $9, $10)
        ON CONFLICT (message_id) DO NOTHING
        "#,
    )
    .bind(&envelope.message_id)
    .bind(&envelope.incident_key)
    .bind(&envelope.message_type)
    .bind(envelope.severity.as_str())
    .bind(&envelope.producer)
    .bind(&envelope.provider_resource_id)
    .bind(&envelope.summary)
    .bind(envelope.occurred_at.as_deref())
    .bind(suppressed)
    .bind(Json(envelope.raw_payload.clone()))
    .execute(pool)
    .await
    .context("writing notification event failed")?;

    Ok(result.rows_affected() > 0)
}

pub async fn handle_notification_envelope(
    pool: &PgPool,
    payload: &[u8],
    rate_limit_seconds: u64,
) -> Result<NotificationHandleOutcome> {
    let envelope = parse_notification_envelope(payload)?;
    let suppressed = recently_notified(pool, &envelope.incident_key, rate_limit_seconds).await?;
    let persisted = persist_notification_event(pool, &envelope, suppressed).await?;

    Ok(NotificationHandleOutcome {
        envelope,
        persisted,
        suppressed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_quarantine_messages_as_critical() {
        let payload = br#"{
          "message_id":"msg_1",
          "message_type":"provider_resource.quarantined",
          "producer":"routing-worker",
          "payload":{"provider_resource_id":"prvrsrc_openai_primary","reason":"failure budget exhausted"}
        }"#;

        let envelope = parse_notification_envelope(payload).expect("parse");
        assert_eq!(envelope.severity, IncidentSeverity::Critical);
        assert_eq!(
            envelope.summary,
            "Provider prvrsrc_openai_primary was quarantined: failure budget exhausted"
        );
    }

    #[test]
    fn honors_explicit_severity_override() {
        let payload = br#"{
          "message_id":"msg_2",
          "message_type":"provider_resource.degraded",
          "producer":"routing-worker",
          "payload":{"provider_resource_id":"prvrsrc_openai_primary","severity":"info"}
        }"#;

        let envelope = parse_notification_envelope(payload).expect("parse");
        assert_eq!(envelope.severity, IncidentSeverity::Info);
        assert_eq!(
            envelope.incident_key,
            "provider_resource.degraded:prvrsrc_openai_primary"
        );
    }

    #[test]
    fn rejects_missing_required_metadata() {
        let payload = br#"{"message_type":"provider_resource.degraded"}"#;
        assert!(parse_notification_envelope(payload).is_err());
    }
}
