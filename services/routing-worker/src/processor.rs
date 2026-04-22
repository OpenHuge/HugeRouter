use anyhow::{Context, Result};
use core_domain::{HealthState, ProviderResource, ProviderResourceStatus};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Row, types::Json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedProbeStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProbeObservationPayload {
    pub provider_resource_id: String,
    pub observed_status: ObservedProbeStatus,
    #[serde(default)]
    pub latency_ms: Option<u32>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub target_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProbeObservationEnvelope {
    pub message_id: String,
    pub message_type: String,
    #[serde(default)]
    pub occurred_at: Option<String>,
    pub producer: String,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    pub payload: ProbeObservationPayload,
}

#[derive(Debug, Clone)]
pub struct RoutingIncidentEvent {
    pub message_type: &'static str,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub enum RouteHealthUpdate {
    Duplicate {
        provider_resource_id: String,
    },
    Updated {
        provider_resource_id: String,
        previous_health_state: HealthState,
        next_health_state: HealthState,
        emitted_event: Option<RoutingIncidentEvent>,
    },
}

pub async fn ensure_probe_tables(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS provider_probe_events (
            message_id TEXT PRIMARY KEY,
            provider_resource_id TEXT NOT NULL,
            observed_status TEXT NOT NULL,
            latency_ms INTEGER NULL,
            reason TEXT NULL,
            target_url TEXT NULL,
            producer TEXT NOT NULL,
            trace_id TEXT NULL,
            request_id TEXT NULL,
            occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            raw_payload JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .context("creating provider_probe_events table failed")?;

    Ok(())
}

pub fn parse_probe_observation(payload: &[u8]) -> Result<ProbeObservationEnvelope> {
    let envelope: ProbeObservationEnvelope =
        serde_json::from_slice(payload).context("invalid provider probe observation payload")?;
    if envelope.message_type != "provider_probe.observed" {
        anyhow::bail!("unexpected message type `{}`", envelope.message_type);
    }

    Ok(envelope)
}

async fn persist_probe_event(pool: &PgPool, envelope: &ProbeObservationEnvelope) -> Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO provider_probe_events (
            message_id,
            provider_resource_id,
            observed_status,
            latency_ms,
            reason,
            target_url,
            producer,
            trace_id,
            request_id,
            occurred_at,
            raw_payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, COALESCE($10::timestamptz, NOW()), $11)
        ON CONFLICT (message_id) DO NOTHING
        "#,
    )
    .bind(&envelope.message_id)
    .bind(&envelope.payload.provider_resource_id)
    .bind(observed_status_slug(envelope.payload.observed_status))
    .bind(envelope.payload.latency_ms.map(i32::try_from).transpose()?)
    .bind(&envelope.payload.reason)
    .bind(&envelope.payload.target_url)
    .bind(&envelope.producer)
    .bind(&envelope.trace_id)
    .bind(&envelope.request_id)
    .bind(envelope.occurred_at.as_deref())
    .bind(Json(serde_json::to_value(envelope)?))
    .execute(pool)
    .await
    .context("writing provider probe event failed")?;

    Ok(result.rows_affected() > 0)
}

fn observed_status_slug(status: ObservedProbeStatus) -> &'static str {
    match status {
        ObservedProbeStatus::Healthy => "healthy",
        ObservedProbeStatus::Degraded => "degraded",
        ObservedProbeStatus::Unhealthy => "unhealthy",
    }
}

async fn recent_unhealthy_streak(pool: &PgPool, provider_resource_id: &str) -> Result<u32> {
    let rows = sqlx::query(
        r#"
        SELECT observed_status
        FROM provider_probe_events
        WHERE provider_resource_id = $1
        ORDER BY occurred_at DESC, created_at DESC
        LIMIT 10
        "#,
    )
    .bind(provider_resource_id)
    .fetch_all(pool)
    .await
    .context("loading recent provider probe events failed")?;

    let mut streak = 0u32;
    for row in rows {
        let observed_status: String = row.try_get("observed_status")?;
        if observed_status == "unhealthy" {
            streak += 1;
            continue;
        }

        break;
    }

    Ok(streak)
}

fn resolve_next_states(
    provider_resource: &ProviderResource,
    observed_status: ObservedProbeStatus,
    unhealthy_streak: u32,
    quarantine_threshold: u32,
) -> (HealthState, ProviderResourceStatus) {
    if provider_resource.status == ProviderResourceStatus::Quarantined
        || provider_resource.health_state == HealthState::Quarantined
    {
        return (
            HealthState::Quarantined,
            ProviderResourceStatus::Quarantined,
        );
    }

    let next_health_state = match observed_status {
        ObservedProbeStatus::Healthy => HealthState::Healthy,
        ObservedProbeStatus::Degraded => HealthState::Degraded,
        ObservedProbeStatus::Unhealthy if unhealthy_streak >= quarantine_threshold => {
            HealthState::Quarantined
        }
        ObservedProbeStatus::Unhealthy => HealthState::Degraded,
    };

    let next_status = match next_health_state {
        HealthState::Quarantined => ProviderResourceStatus::Quarantined,
        _ => ProviderResourceStatus::Active,
    };

    (next_health_state, next_status)
}

fn incident_event(
    provider_resource_id: &str,
    observed_status: ObservedProbeStatus,
    previous_health_state: HealthState,
    next_health_state: HealthState,
    unhealthy_streak: u32,
    reason: Option<&str>,
    latency_ms: Option<u32>,
) -> Option<RoutingIncidentEvent> {
    if previous_health_state == next_health_state {
        return None;
    }

    let (message_type, severity) = match next_health_state {
        HealthState::Quarantined => ("provider_resource.quarantined", "critical"),
        HealthState::Degraded => ("provider_resource.degraded", "warning"),
        _ => return None,
    };

    Some(RoutingIncidentEvent {
        message_type,
        payload: json!({
            "provider_resource_id": provider_resource_id,
            "severity": severity,
            "reason": reason.unwrap_or(match observed_status {
                ObservedProbeStatus::Healthy => "probe recovered",
                ObservedProbeStatus::Degraded => "probe exceeded degraded latency budget",
                ObservedProbeStatus::Unhealthy => "probe failed",
            }),
            "previous_health_state": format!("{previous_health_state:?}").to_ascii_lowercase(),
            "next_health_state": format!("{next_health_state:?}").to_ascii_lowercase(),
            "consecutive_unhealthy_observations": unhealthy_streak,
            "latency_ms": latency_ms
        }),
    })
}

pub async fn handle_probe_observation(
    pool: &PgPool,
    payload: &[u8],
    quarantine_threshold: u32,
) -> Result<RouteHealthUpdate> {
    let envelope = parse_probe_observation(payload)?;
    let inserted = persist_probe_event(pool, &envelope).await?;

    if !inserted {
        return Ok(RouteHealthUpdate::Duplicate {
            provider_resource_id: envelope.payload.provider_resource_id,
        });
    }

    let row = sqlx::query("SELECT payload FROM provider_resources WHERE provider_resource_id = $1")
        .bind(&envelope.payload.provider_resource_id)
        .fetch_optional(pool)
        .await
        .context("loading provider resource for health update failed")?
        .context("provider resource not found for probe observation")?;
    let payload: Value = row.try_get("payload")?;
    let mut provider_resource: ProviderResource =
        serde_json::from_value(payload).context("invalid provider resource payload in postgres")?;

    let previous_health_state = provider_resource.health_state;
    let unhealthy_streak =
        recent_unhealthy_streak(pool, &provider_resource.provider_resource_id.to_string()).await?;
    let (next_health_state, next_status) = resolve_next_states(
        &provider_resource,
        envelope.payload.observed_status,
        unhealthy_streak,
        quarantine_threshold,
    );

    provider_resource.health_state = next_health_state;
    provider_resource.status = next_status;

    sqlx::query("UPDATE provider_resources SET payload = $2 WHERE provider_resource_id = $1")
        .bind(provider_resource.provider_resource_id.to_string())
        .bind(Json(provider_resource.clone()))
        .execute(pool)
        .await
        .context("updating provider resource health state failed")?;

    Ok(RouteHealthUpdate::Updated {
        provider_resource_id: provider_resource.provider_resource_id.to_string(),
        previous_health_state,
        next_health_state,
        emitted_event: incident_event(
            provider_resource.provider_resource_id.as_str(),
            envelope.payload.observed_status,
            previous_health_state,
            next_health_state,
            unhealthy_streak,
            envelope.payload.reason.as_deref(),
            envelope.payload.latency_ms,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provider_resource() -> ProviderResource {
        serde_json::from_value(json!({
            "provider_resource_id": "prvrsrc_openai_primary",
            "tenant_id": "tenant_acme",
            "project_id": "proj_core",
            "provider_id": "openai",
            "name": "OpenAI Primary",
            "status": "active",
            "provenance_class": "official_api",
            "credential_owner_type": "platform",
            "deployment_scope": "shared",
            "region": "us-east-1",
            "endpoint_base_url": "https://api.openai.com/v1",
            "auth_kind": "api_key",
            "health_state": "healthy",
            "budget_policy_id": null,
            "capabilities": {
              "supports_streaming": true,
              "supports_tool_calling": true,
              "supports_json_mode": true
            },
            "version": 1,
            "created_at": "2026-04-23T00:00:00Z",
            "updated_at": "2026-04-23T00:00:00Z"
        }))
        .expect("provider resource fixture")
    }

    #[test]
    fn resolves_quarantine_when_failure_budget_is_exhausted() {
        let provider_resource = sample_provider_resource();
        let (next_health, next_status) =
            resolve_next_states(&provider_resource, ObservedProbeStatus::Unhealthy, 3, 3);

        assert_eq!(next_health, HealthState::Quarantined);
        assert_eq!(next_status, ProviderResourceStatus::Quarantined);
    }

    #[test]
    fn preserves_manual_quarantine_until_operator_override() {
        let mut provider_resource = sample_provider_resource();
        provider_resource.status = ProviderResourceStatus::Quarantined;
        provider_resource.health_state = HealthState::Quarantined;

        let (next_health, next_status) =
            resolve_next_states(&provider_resource, ObservedProbeStatus::Healthy, 0, 3);

        assert_eq!(next_health, HealthState::Quarantined);
        assert_eq!(next_status, ProviderResourceStatus::Quarantined);
    }

    #[test]
    fn parses_expected_probe_message_shape() {
        let payload = br#"{
          "message_id":"msg_probe_1",
          "message_type":"provider_probe.observed",
          "producer":"edge-probe",
          "payload":{
            "provider_resource_id":"prvrsrc_openai_primary",
            "observed_status":"degraded",
            "latency_ms":1900,
            "reason":"latency budget exceeded"
          }
        }"#;

        let envelope = parse_probe_observation(payload).expect("parse");
        assert_eq!(
            envelope.payload.provider_resource_id,
            "prvrsrc_openai_primary"
        );
        assert_eq!(
            envelope.payload.observed_status,
            ObservedProbeStatus::Degraded
        );
    }
}
