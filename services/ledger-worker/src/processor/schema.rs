use anyhow::{Context, Result};
use sqlx::PgPool;

pub async fn ensure_ledger_table(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ledger_entries (
            ledger_entry_id TEXT PRIMARY KEY,
            usage_event_id TEXT NOT NULL,
            grant_id TEXT NULL,
            owner_account_id TEXT NULL,
            usage_phase TEXT NOT NULL,
            tenant_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            route_receipt_id TEXT NOT NULL,
            provider_resource_id TEXT NOT NULL,
            ledger_entry_type TEXT NOT NULL,
            amount_micros BIGINT NOT NULL,
            currency TEXT NOT NULL,
            recorded_at TIMESTAMPTZ NOT NULL,
            idempotency_key TEXT NOT NULL UNIQUE,
            source_message_id TEXT NOT NULL,
            source_producer TEXT NOT NULL,
            source_request_id TEXT NULL,
            source_trace_id TEXT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .context("creating ledger_entries table failed")?;

    Ok(())
}
