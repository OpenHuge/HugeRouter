use anyhow::{Context, Result};
use protocol_ir::UsageEventRecordedMessage;
use sqlx::PgPool;
use tracing::{debug, info};

use super::entry::{BudgetThresholdEventRecord, build_pending_entry};
use super::projections::{update_balance_projection, update_usage_daily_projection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionOutcome {
    Inserted,
    Duplicate,
}

#[derive(Debug, Clone)]
pub struct UsagePersistenceOutcome {
    pub ingestion: IngestionOutcome,
    pub budget_threshold_event: Option<BudgetThresholdEventRecord>,
}

pub async fn persist_usage_event(
    pool: &PgPool,
    event: &UsageEventRecordedMessage,
) -> Result<UsagePersistenceOutcome> {
    let entry = build_pending_entry(pool, event).await?;
    let mut tx = pool
        .begin()
        .await
        .context("opening ledger transaction failed")?;
    let result = sqlx::query(
        r#"
        INSERT INTO ledger_entries (
            ledger_entry_id,
            usage_event_id,
            grant_id,
            owner_account_id,
            usage_phase,
            tenant_id,
            project_id,
            route_receipt_id,
            provider_resource_id,
            provider_id,
            model_alias,
            ledger_entry_type,
            amount_micros,
            provider_cost_micros,
            billable_cost_micros,
            input_tokens,
            output_tokens,
            cached_input_tokens,
            currency,
            recorded_at,
            idempotency_key,
            source_message_id,
            source_producer,
            source_request_id,
            source_trace_id
        ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20::timestamptz,$21,$22,$23,$24,$25
        ) ON CONFLICT (idempotency_key) DO NOTHING
        "#,
    )
    .bind(&entry.ledger_entry_id)
    .bind(entry.usage_event_id.to_string())
    .bind(&entry.grant_id)
    .bind(&entry.owner_account_id)
    .bind(&entry.usage_phase)
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(&entry.route_receipt_id)
    .bind(&entry.provider_resource_id)
    .bind(&entry.provider_id)
    .bind(&entry.model_alias)
    .bind(&entry.ledger_entry_type)
    .bind(entry.amount_micros)
    .bind(entry.provider_cost_micros)
    .bind(entry.billable_cost_micros)
    .bind(i64::from(entry.input_tokens))
    .bind(i64::from(entry.output_tokens))
    .bind(i64::from(entry.cached_input_tokens))
    .bind(&entry.currency)
    .bind(&entry.recorded_at)
    .bind(&entry.idempotency_key)
    .bind(&entry.source_message_id)
    .bind(&entry.source_producer)
    .bind(&entry.source_request_id)
    .bind(&entry.source_trace_id)
    .execute(&mut *tx)
    .await
    .context("writing ledger entry failed")?;

    if result.rows_affected() == 0 {
        tx.rollback()
            .await
            .context("rolling back duplicate ledger transaction failed")?;
        debug!(
            idempotency_key = entry.idempotency_key,
            "skipping duplicate usage event ledger entry"
        );
        Ok(UsagePersistenceOutcome {
            ingestion: IngestionOutcome::Duplicate,
            budget_threshold_event: None,
        })
    } else {
        update_usage_daily_projection(&mut tx, &entry).await?;
        let budget_threshold_event = update_balance_projection(&mut tx, &entry).await?;
        tx.commit()
            .await
            .context("committing ledger transaction failed")?;

        info!(
            idempotency_key = entry.idempotency_key,
            usage_event_id = %entry.usage_event_id,
            ledger_entry_id = %entry.ledger_entry_id,
            amount_micros = entry.amount_micros,
            "inserted ledger entry"
        );
        Ok(UsagePersistenceOutcome {
            ingestion: IngestionOutcome::Inserted,
            budget_threshold_event,
        })
    }
}

pub async fn handle_usage_event_recorded(
    pool: &PgPool,
    payload: &[u8],
) -> Result<UsagePersistenceOutcome> {
    let envelope: UsageEventRecordedMessage = serde_json::from_slice(payload)
        .context("failed to deserialize UsageEventRecordedMessage")?;
    persist_usage_event(pool, &envelope).await
}
