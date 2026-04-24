use anyhow::{Context, Result};
use protocol_ir::{RouteReceiptRecordedMessage, RouteReceiptRecordedMessageType};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction, types::Json};
#[cfg(test)]
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionOutcome {
    Inserted,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRouteReceiptRecord {
    pub route_receipt_id: String,
    pub route_receipt_payload: Value,
    pub decision_timeline: Value,
    pub policy_checks: Value,
    pub provider_attempts: Value,
    pub source_message_id: String,
    pub source_producer: String,
    pub source_request_id: Option<String>,
    pub source_trace_id: Option<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredRouteReceiptRecord {
    route_receipt_payload: Value,
    decision_timeline: Value,
    policy_checks: Value,
    provider_attempts: Value,
}

#[cfg(test)]
#[derive(Debug, Default)]
struct InMemoryRouteReceiptStore {
    route_receipts: BTreeMap<String, StoredRouteReceiptRecord>,
}

pub async fn ensure_route_receipt_tables(pool: &PgPool) -> Result<()> {
    for statement in [
        r"
        CREATE TABLE IF NOT EXISTS route_receipts (
            route_receipt_id TEXT PRIMARY KEY,
            payload JSONB NOT NULL
        )
        ",
        r"
        CREATE TABLE IF NOT EXISTS route_receipt_diagnostics (
            route_receipt_id TEXT PRIMARY KEY,
            decision_timeline JSONB NOT NULL,
            policy_checks JSONB NOT NULL,
            provider_attempts JSONB NOT NULL,
            source_message_id TEXT NOT NULL,
            source_producer TEXT NOT NULL,
            source_request_id TEXT NULL,
            source_trace_id TEXT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        ",
    ] {
        match sqlx::query(statement).execute(pool).await {
            Ok(_) => {}
            Err(error) if is_concurrent_create_table_race(&error) => {}
            Err(error) => return Err(error).context("creating route receipt tables failed"),
        }
    }

    Ok(())
}

fn is_concurrent_create_table_race(error: &sqlx::Error) -> bool {
    let Some(database_error) = error.as_database_error() else {
        return false;
    };
    database_error.code().is_some_and(|code| code == "23505")
        && database_error
            .message()
            .contains("pg_type_typname_nsp_index")
}

pub fn parse_route_receipt_recorded(payload: &[u8]) -> Result<RouteReceiptRecordedMessage> {
    let event: RouteReceiptRecordedMessage =
        serde_json::from_slice(payload).context("invalid route receipt recorded payload")?;
    if event.message_type != RouteReceiptRecordedMessageType::RouteReceiptRecorded {
        anyhow::bail!(
            "unexpected message type `{}`",
            serde_json::to_string(&event.message_type)?
        );
    }

    Ok(event)
}

pub fn build_pending_route_receipt_record(
    event: &RouteReceiptRecordedMessage,
) -> Result<PendingRouteReceiptRecord> {
    let route_receipt_id = event.payload.route_receipt.route_receipt_id.to_string();
    let expected_idempotency_key = format!("{route_receipt_id}:recorded");
    if event.idempotency_key != expected_idempotency_key {
        anyhow::bail!(
            "envelope idempotency key `{}` does not match expected `{expected_idempotency_key}`",
            event.idempotency_key
        );
    }

    Ok(PendingRouteReceiptRecord {
        route_receipt_id,
        route_receipt_payload: serde_json::to_value(&event.payload.route_receipt)
            .context("serialize route receipt payload")?,
        decision_timeline: serde_json::to_value(&event.payload.decision_timeline)
            .context("serialize decision timeline")?,
        policy_checks: serde_json::to_value(&event.payload.policy_checks)
            .context("serialize policy checks")?,
        provider_attempts: serde_json::to_value(&event.payload.provider_attempts)
            .context("serialize provider attempts")?,
        source_message_id: event.message_id.clone(),
        source_producer: event.producer.0.clone(),
        source_request_id: event.request_id.clone(),
        source_trace_id: event.trace_id.clone(),
    })
}

#[cfg(test)]
fn persist_pending_route_receipt_in_memory(
    store: &mut InMemoryRouteReceiptStore,
    pending: &PendingRouteReceiptRecord,
) -> IngestionOutcome {
    if store.route_receipts.contains_key(&pending.route_receipt_id) {
        return IngestionOutcome::Duplicate;
    }

    store.route_receipts.insert(
        pending.route_receipt_id.clone(),
        StoredRouteReceiptRecord {
            route_receipt_payload: pending.route_receipt_payload.clone(),
            decision_timeline: pending.decision_timeline.clone(),
            policy_checks: pending.policy_checks.clone(),
            provider_attempts: pending.provider_attempts.clone(),
        },
    );

    IngestionOutcome::Inserted
}

async fn persist_pending_route_receipt(
    tx: &mut Transaction<'_, Postgres>,
    pending: &PendingRouteReceiptRecord,
) -> Result<IngestionOutcome> {
    let result = sqlx::query(
        r"
        INSERT INTO route_receipts (
            route_receipt_id,
            payload
        )
        VALUES ($1, $2)
        ON CONFLICT (route_receipt_id) DO NOTHING
        ",
    )
    .bind(&pending.route_receipt_id)
    .bind(Json(pending.route_receipt_payload.clone()))
    .execute(&mut **tx)
    .await
    .context("writing route receipt failed")?;

    if result.rows_affected() == 0 {
        return Ok(IngestionOutcome::Duplicate);
    }

    sqlx::query(
        r"
        INSERT INTO route_receipt_diagnostics (
            route_receipt_id,
            decision_timeline,
            policy_checks,
            provider_attempts,
            source_message_id,
            source_producer,
            source_request_id,
            source_trace_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (route_receipt_id) DO NOTHING
        ",
    )
    .bind(&pending.route_receipt_id)
    .bind(Json(pending.decision_timeline.clone()))
    .bind(Json(pending.policy_checks.clone()))
    .bind(Json(pending.provider_attempts.clone()))
    .bind(&pending.source_message_id)
    .bind(&pending.source_producer)
    .bind(&pending.source_request_id)
    .bind(&pending.source_trace_id)
    .execute(&mut **tx)
    .await
    .context("writing route receipt diagnostics failed")?;

    Ok(IngestionOutcome::Inserted)
}

pub async fn handle_route_receipt_recorded(
    pool: &PgPool,
    payload: &[u8],
) -> Result<IngestionOutcome> {
    let event = parse_route_receipt_recorded(payload)?;
    let pending = build_pending_route_receipt_record(&event)?;
    let mut tx = pool
        .begin()
        .await
        .context("begin route receipt transaction")?;
    let outcome = persist_pending_route_receipt(&mut tx, &pending).await?;
    tx.commit()
        .await
        .context("commit route receipt transaction")?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_ir::{
        RouteReceiptDecisionTraceStep, RouteReceiptPolicyCheck, RouteReceiptProviderAttempt,
        RouteReceiptRecorded, RouteReceiptRecordedMessageType,
    };

    #[allow(clippy::too_many_lines)]
    fn sample_route_receipt_recorded_message(
        normalized_error_code: Option<&str>,
    ) -> RouteReceiptRecordedMessage {
        let route_receipt = core_domain::RouteReceipt {
            route_receipt_id: core_domain::RouteReceiptId::parse("routercpt_123").unwrap(),
            tenant_id: core_domain::TenantId::parse("tenant_acme").unwrap(),
            project_id: core_domain::ProjectId::parse("proj_core").unwrap(),
            route_policy_id: core_domain::RoutePolicyId::parse("routepol_default").unwrap(),
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
            config_snapshot_id: core_domain::ConfigSnapshotId::parse("cfgsnap_123").unwrap(),
            admission_result: core_domain::AdmissionResult::Admitted,
            selected_target: Some(
                core_domain::ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            ),
            excluded_targets: vec![core_domain::ExcludedTarget {
                provider_resource_id: core_domain::ProviderResourceId::parse("prvrsrc_backup")
                    .unwrap(),
                reason_code: "rejected_health_state".to_string(),
                reason: "rejected_health_state".to_string(),
            }],
            score_breakdown: core_domain::ScoreBreakdown {
                latency: 0.92,
                cost: 0.74,
                health: 0.88,
                trust: 1.0,
            },
            fallback_transitions: vec![core_domain::FallbackTransition {
                from_provider_resource_id: core_domain::ProviderResourceId::parse(
                    "prvrsrc_openai_primary",
                )
                .unwrap(),
                to_provider_resource_id: core_domain::ProviderResourceId::parse(
                    "prvrsrc_openai_backup",
                )
                .unwrap(),
                reason: "retrying next ranked candidate".to_string(),
            }],
            normalized_error: normalized_error_code.map(|code| core_domain::NormalizedError {
                code: code.to_string(),
                message: "provider down".to_string(),
                request_id: "req_123".to_string(),
                retryable: false,
                upstream_code: None,
                upstream_status_code: Some(503),
                validation_issues: Vec::new(),
                details: BTreeMap::from([(
                    "provider_resource_id".to_string(),
                    "prvrsrc_openai_primary".to_string(),
                )]),
            }),
            failure_reason: normalized_error_code.map(|_| "provider down".to_string()),
            created_at: "2026-04-23T10:00:00Z".to_string(),
        };
        let payload = RouteReceiptRecorded {
            route_receipt,
            decision_timeline: vec![RouteReceiptDecisionTraceStep {
                stage: "admission".to_string(),
                status: "passed".to_string(),
                message: "Request admitted by active route policy".to_string(),
                score: Some(1.0),
                notes: vec!["protocol_family=openai_chat".to_string()],
            }],
            policy_checks: vec![RouteReceiptPolicyCheck {
                policy_id: core_domain::RoutePolicyId::parse("routepol_default").unwrap(),
                status: "passed".to_string(),
                reason: None,
            }],
            provider_attempts: vec![RouteReceiptProviderAttempt {
                provider_resource_id: core_domain::ProviderResourceId::parse(
                    "prvrsrc_openai_primary",
                )
                .unwrap(),
                attempt: 1,
                status: if normalized_error_code.is_some() {
                    "non_retryable_failure".to_string()
                } else {
                    "success".to_string()
                },
                reason_code: if normalized_error_code.is_some() {
                    "provider_unavailable".to_string()
                } else {
                    "provider_success".to_string()
                },
                retryable: false,
                fallback_target: None,
                started_at: "2026-04-23T10:00:00Z".to_string(),
                finished_at: "2026-04-23T10:00:01Z".to_string(),
                latency_ms: 1000,
                reason: if normalized_error_code.is_some() {
                    "provider down".to_string()
                } else {
                    "provider returned output".to_string()
                },
            }],
        };
        RouteReceiptRecordedMessage {
            message_id: "msg_routercpt_123".to_string(),
            message_type: RouteReceiptRecordedMessageType::RouteReceiptRecorded,
            schema_version: 1,
            occurred_at: "2026-04-23T10:00:00Z".to_string(),
            producer: core_domain::ServiceName::parse("gateway-api").unwrap(),
            trace_id: Some("trace_123".to_string()),
            request_id: Some("req_123".to_string()),
            idempotency_key: "routercpt_123:recorded".to_string(),
            payload,
        }
    }

    #[test]
    fn parses_route_receipt_recorded_message() {
        let event = sample_route_receipt_recorded_message(None);
        let payload = serde_json::to_vec(&event).unwrap();
        let reparsed = parse_route_receipt_recorded(&payload).unwrap();

        assert_eq!(reparsed, event);
    }

    #[test]
    fn build_pending_record_preserves_success_path_payload() {
        let event = sample_route_receipt_recorded_message(None);
        let pending = build_pending_route_receipt_record(&event).unwrap();

        assert_eq!(pending.route_receipt_id, "routercpt_123");
        assert_eq!(
            pending.route_receipt_payload["selected_target"],
            "prvrsrc_openai_primary"
        );
        assert_eq!(pending.provider_attempts[0]["status"], "success");
        assert_eq!(
            pending.provider_attempts[0]["reason_code"],
            "provider_success"
        );
        assert_eq!(pending.source_message_id, "msg_routercpt_123");
    }

    #[test]
    fn build_pending_record_preserves_failure_path_payload() {
        let event = sample_route_receipt_recorded_message(Some("provider_unavailable"));
        let pending = build_pending_route_receipt_record(&event).unwrap();

        assert_eq!(
            pending.route_receipt_payload["normalized_error"]["code"],
            "provider_unavailable"
        );
        assert_eq!(
            pending.provider_attempts[0]["status"],
            "non_retryable_failure"
        );
        assert_eq!(
            pending.provider_attempts[0]["reason_code"],
            "provider_unavailable"
        );
    }

    #[test]
    fn rejects_mismatched_idempotency_key() {
        let mut event = sample_route_receipt_recorded_message(None);
        event.idempotency_key = "routercpt_123:wrong".to_string();

        assert!(build_pending_route_receipt_record(&event).is_err());
    }

    #[test]
    fn deduplicates_duplicate_route_receipt_records() {
        let event = sample_route_receipt_recorded_message(None);
        let pending = build_pending_route_receipt_record(&event).unwrap();
        let mut store = InMemoryRouteReceiptStore::default();

        let first = persist_pending_route_receipt_in_memory(&mut store, &pending);
        let second = persist_pending_route_receipt_in_memory(&mut store, &pending);

        assert_eq!(first, IngestionOutcome::Inserted);
        assert_eq!(second, IngestionOutcome::Duplicate);
        assert_eq!(store.route_receipts.len(), 1);
    }

    #[test]
    fn persists_route_receipt_and_diagnostics_together_in_memory() {
        let event = sample_route_receipt_recorded_message(Some("provider_unavailable"));
        let pending = build_pending_route_receipt_record(&event).unwrap();
        let mut store = InMemoryRouteReceiptStore::default();

        let outcome = persist_pending_route_receipt_in_memory(&mut store, &pending);
        let stored = store.route_receipts.get("routercpt_123").unwrap();

        assert_eq!(outcome, IngestionOutcome::Inserted);
        assert_eq!(
            stored.route_receipt_payload["normalized_error"]["code"],
            "provider_unavailable"
        );
        assert_eq!(stored.decision_timeline[0]["stage"], "admission");
        assert_eq!(stored.policy_checks[0]["status"], "passed");
        assert_eq!(
            stored.provider_attempts[0]["status"],
            "non_retryable_failure"
        );
    }
}
