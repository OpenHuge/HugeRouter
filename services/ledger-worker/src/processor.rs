#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::format_collect,
    clippy::missing_const_for_fn,
    clippy::needless_raw_string_hashes,
    clippy::trivially_copy_pass_by_ref,
    clippy::useless_conversion
)]

use anyhow::{Context, Result};
use core_domain::{UsageEventId, UsagePhase};
use metering::{default_budget_micros, quote_usage, threshold_status};
use protocol_ir::UsageEventRecordedMessage;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionOutcome {
    Inserted,
    Duplicate,
}

#[derive(Debug, Clone)]
pub struct BudgetThresholdEventRecord {
    pub budget_threshold_event_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub currency: String,
    pub threshold_status: String,
    pub billable_cost_micros: i64,
    pub configured_budget_micros: i64,
}

#[derive(Debug, Clone)]
pub struct UsagePersistenceOutcome {
    pub ingestion: IngestionOutcome,
    pub budget_threshold_event: Option<BudgetThresholdEventRecord>,
}

#[derive(Debug, Clone)]
pub struct PendingLedgerEntry {
    pub ledger_entry_id: String,
    pub usage_event_id: UsageEventId,
    pub grant_id: Option<String>,
    pub ledger_entry_type: String,
    pub amount_micros: i64,
    pub provider_cost_micros: i64,
    pub billable_cost_micros: i64,
    pub currency: String,
    pub recorded_at: String,
    pub tenant_id: String,
    pub project_id: String,
    pub route_receipt_id: String,
    pub provider_resource_id: String,
    pub provider_id: String,
    pub model_alias: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
    pub idempotency_key: String,
    pub source_message_id: String,
    pub source_producer: String,
    pub source_request_id: Option<String>,
    pub source_trace_id: Option<String>,
    pub usage_phase: String,
}

pub async fn ensure_ledger_table(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ledger_entries (
            ledger_entry_id TEXT PRIMARY KEY,
            usage_event_id TEXT NOT NULL,
            grant_id TEXT NULL,
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

pub async fn ensure_projection_tables(pool: &PgPool) -> Result<()> {
    for statement in [
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS provider_cost_micros BIGINT"#,
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS grant_id TEXT"#,
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS billable_cost_micros BIGINT"#,
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS provider_id TEXT"#,
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS model_alias TEXT"#,
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS input_tokens BIGINT"#,
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS output_tokens BIGINT"#,
        r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS cached_input_tokens BIGINT"#,
        r#"
        CREATE TABLE IF NOT EXISTS usage_daily_projections (
            tenant_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            usage_date DATE NOT NULL,
            provider_id TEXT NOT NULL,
            model_alias TEXT NOT NULL,
            currency TEXT NOT NULL,
            input_tokens BIGINT NOT NULL,
            output_tokens BIGINT NOT NULL,
            cached_input_tokens BIGINT NOT NULL,
            provider_cost_micros BIGINT NOT NULL,
            billable_cost_micros BIGINT NOT NULL,
            event_count BIGINT NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (tenant_id, project_id, usage_date, provider_id, model_alias)
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS balance_projections (
            tenant_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            currency TEXT NOT NULL,
            provider_cost_micros BIGINT NOT NULL,
            billable_cost_micros BIGINT NOT NULL,
            configured_budget_micros BIGINT NOT NULL,
            remaining_budget_micros BIGINT NOT NULL,
            threshold_status TEXT NOT NULL,
            threshold_crossed_at TIMESTAMPTZ NULL,
            last_projected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (tenant_id, project_id, currency)
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS budget_threshold_events (
            budget_threshold_event_id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            currency TEXT NOT NULL,
            threshold_status TEXT NOT NULL,
            billable_cost_micros BIGINT NOT NULL,
            configured_budget_micros BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .context("creating ledger projection tables failed")?;
    }

    Ok(())
}

pub fn usage_phase_as_str(phase: &UsagePhase) -> &'static str {
    match phase {
        UsagePhase::Reserve => "reserve",
        UsagePhase::Partial => "partial",
        UsagePhase::Final => "final",
        UsagePhase::Release => "release",
    }
}

#[allow(dead_code)]
pub fn parse_estimated_cost_micros(amount: &str) -> Result<i64> {
    let raw = amount.trim();
    if raw.is_empty() {
        anyhow::bail!("cost amount is empty");
    }

    let mut parts = raw.split('.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next();

    if parts.next().is_some() {
        anyhow::bail!("cost amount has more than one decimal point: {raw}");
    }
    if int_part.is_empty() || !int_part.chars().all(|digit| digit.is_ascii_digit()) {
        anyhow::bail!("cost amount integer part is invalid: {raw}");
    }

    let mut integer = int_part
        .parse::<i128>()
        .context("invalid cost integer part")?;
    if integer < 0 {
        anyhow::bail!("cost amount cannot be negative: {raw}");
    }

    let mut fractional = 0i128;
    let mut digits = 0u32;
    if let Some(frac) = frac_part {
        if !frac.chars().all(|digit| digit.is_ascii_digit()) {
            anyhow::bail!("cost amount fractional part is invalid: {raw}");
        }
        for ch in frac.chars() {
            if digits >= 6 {
                break;
            }
            let digit = u32::from(ch.to_digit(10).unwrap_or_default());
            fractional = fractional * 10 + i128::from(digit);
            digits += 1;
        }
    }

    for _ in digits..6 {
        fractional *= 10;
    }

    integer *= 1_000_000;
    let micros = integer + fractional;
    if micros > i64::MAX as i128 {
        anyhow::bail!("cost amount is too large: {raw}");
    }

    Ok(micros as i64)
}

pub fn ledger_entry_id(idempotency_key: &str) -> String {
    let digest = Sha256::digest(idempotency_key.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("ledger_{hex}")
}

async fn resolve_provider_id(pool: &PgPool, provider_resource_id: &str) -> Result<String> {
    let row =
        sqlx::query("SELECT provider_id FROM provider_resources WHERE provider_resource_id = $1")
            .bind(provider_resource_id)
            .fetch_optional(pool)
            .await
            .context("loading provider resource for pricing failed")?;

    Ok(row
        .and_then(|row| row.try_get::<String, _>("provider_id").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "default".to_string()))
}

pub async fn build_pending_entry(
    pool: &PgPool,
    event: &UsageEventRecordedMessage,
) -> Result<PendingLedgerEntry> {
    if event.idempotency_key != event.payload.usage_event.idempotency_key {
        anyhow::bail!("envelope idempotency key does not match usage event idempotency key");
    }

    let provider_id = resolve_provider_id(
        pool,
        event.payload.usage_event.provider_resource_id.as_str(),
    )
    .await?;
    let quote = quote_usage(&provider_id, &event.payload.usage_event.usage);

    Ok(PendingLedgerEntry {
        ledger_entry_id: ledger_entry_id(&event.idempotency_key),
        usage_event_id: event.payload.usage_event.usage_event_id.clone(),
        grant_id: event.payload.usage_event.grant_id.clone(),
        ledger_entry_type: "usage_debit".to_string(),
        amount_micros: quote.billable_cost_micros,
        provider_cost_micros: quote.provider_cost_micros,
        billable_cost_micros: quote.billable_cost_micros,
        currency: quote.currency,
        recorded_at: event.payload.usage_event.recorded_at.clone(),
        tenant_id: event.payload.usage_event.tenant_id.to_string(),
        project_id: event.payload.usage_event.project_id.to_string(),
        route_receipt_id: event.payload.usage_event.route_receipt_id.to_string(),
        provider_resource_id: event.payload.usage_event.provider_resource_id.to_string(),
        provider_id,
        model_alias: event.payload.usage_event.model_alias.clone(),
        input_tokens: event.payload.usage_event.usage.input_tokens,
        output_tokens: event.payload.usage_event.usage.output_tokens,
        cached_input_tokens: event.payload.usage_event.usage.cached_input_tokens,
        idempotency_key: event.idempotency_key.clone(),
        source_message_id: event.message_id.clone(),
        source_producer: event.producer.0.clone(),
        source_request_id: event.request_id.clone(),
        source_trace_id: event.trace_id.clone(),
        usage_phase: usage_phase_as_str(&event.payload.usage_event.phase).to_string(),
    })
}

async fn update_usage_daily_projection(
    tx: &mut Transaction<'_, Postgres>,
    entry: &PendingLedgerEntry,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO usage_daily_projections (
            tenant_id,
            project_id,
            usage_date,
            provider_id,
            model_alias,
            currency,
            input_tokens,
            output_tokens,
            cached_input_tokens,
            provider_cost_micros,
            billable_cost_micros,
            event_count
        )
        VALUES (
            $1,
            $2,
            DATE($3::timestamptz),
            $4,
            $5,
            $6,
            $7,
            $8,
            $9,
            $10,
            $11,
            1
        )
        ON CONFLICT (tenant_id, project_id, usage_date, provider_id, model_alias)
        DO UPDATE SET
            input_tokens = usage_daily_projections.input_tokens + EXCLUDED.input_tokens,
            output_tokens = usage_daily_projections.output_tokens + EXCLUDED.output_tokens,
            cached_input_tokens = usage_daily_projections.cached_input_tokens + EXCLUDED.cached_input_tokens,
            provider_cost_micros = usage_daily_projections.provider_cost_micros + EXCLUDED.provider_cost_micros,
            billable_cost_micros = usage_daily_projections.billable_cost_micros + EXCLUDED.billable_cost_micros,
            event_count = usage_daily_projections.event_count + 1,
            updated_at = NOW()
        "#,
    )
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(&entry.recorded_at)
    .bind(&entry.provider_id)
    .bind(&entry.model_alias)
    .bind(&entry.currency)
    .bind(i64::from(entry.input_tokens))
    .bind(i64::from(entry.output_tokens))
    .bind(i64::from(entry.cached_input_tokens))
    .bind(entry.provider_cost_micros)
    .bind(entry.billable_cost_micros)
    .execute(&mut **tx)
    .await
    .context("updating usage daily projection failed")?;

    Ok(())
}

async fn maybe_record_budget_threshold_event(
    tx: &mut Transaction<'_, Postgres>,
    entry: &PendingLedgerEntry,
    previous_status: Option<&str>,
    next_status: &str,
    new_billable_cost_micros: i64,
    configured_budget_micros: i64,
) -> Result<Option<BudgetThresholdEventRecord>> {
    let crossed =
        previous_status != Some(next_status) && matches!(next_status, "warning" | "exceeded");

    if !crossed {
        return Ok(None);
    }

    let event_id = format!(
        "budgetevt_{}_{}_{}",
        entry.tenant_id, entry.project_id, next_status
    );

    sqlx::query(
        r#"
        INSERT INTO budget_threshold_events (
            budget_threshold_event_id,
            tenant_id,
            project_id,
            currency,
            threshold_status,
            billable_cost_micros,
            configured_budget_micros
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (budget_threshold_event_id) DO NOTHING
        "#,
    )
    .bind(&event_id)
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(&entry.currency)
    .bind(next_status)
    .bind(new_billable_cost_micros)
    .bind(configured_budget_micros)
    .execute(&mut **tx)
    .await
    .context("recording budget threshold event failed")?;

    Ok(Some(BudgetThresholdEventRecord {
        budget_threshold_event_id: event_id,
        tenant_id: entry.tenant_id.clone(),
        project_id: entry.project_id.clone(),
        currency: entry.currency.clone(),
        threshold_status: next_status.to_string(),
        billable_cost_micros: new_billable_cost_micros,
        configured_budget_micros,
    }))
}

async fn update_balance_projection(
    tx: &mut Transaction<'_, Postgres>,
    entry: &PendingLedgerEntry,
) -> Result<Option<BudgetThresholdEventRecord>> {
    let current = sqlx::query(
        r#"
        SELECT provider_cost_micros, billable_cost_micros, threshold_status
        FROM balance_projections
        WHERE tenant_id = $1 AND project_id = $2 AND currency = $3
        FOR UPDATE
        "#,
    )
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(&entry.currency)
    .fetch_optional(&mut **tx)
    .await
    .context("loading current balance projection failed")?;

    let previous_provider_cost_micros = current
        .as_ref()
        .and_then(|row| row.try_get::<i64, _>("provider_cost_micros").ok())
        .unwrap_or_default();
    let previous_billable_cost_micros = current
        .as_ref()
        .and_then(|row| row.try_get::<i64, _>("billable_cost_micros").ok())
        .unwrap_or_default();
    let previous_threshold_status = current
        .as_ref()
        .and_then(|row| row.try_get::<String, _>("threshold_status").ok());

    let new_provider_cost_micros = previous_provider_cost_micros + entry.provider_cost_micros;
    let new_billable_cost_micros = previous_billable_cost_micros + entry.billable_cost_micros;
    let configured_budget_micros = default_budget_micros(&entry.tenant_id, &entry.project_id);
    let remaining_budget_micros = configured_budget_micros - new_billable_cost_micros;
    let next_threshold_status =
        threshold_status(new_billable_cost_micros, configured_budget_micros).to_string();

    sqlx::query(
        r#"
        INSERT INTO balance_projections (
            tenant_id,
            project_id,
            currency,
            provider_cost_micros,
            billable_cost_micros,
            configured_budget_micros,
            remaining_budget_micros,
            threshold_status,
            threshold_crossed_at,
            last_projected_at
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            CASE WHEN $8 IN ('warning', 'exceeded') THEN NOW() ELSE NULL END,
            NOW()
        )
        ON CONFLICT (tenant_id, project_id, currency)
        DO UPDATE SET
            provider_cost_micros = EXCLUDED.provider_cost_micros,
            billable_cost_micros = EXCLUDED.billable_cost_micros,
            configured_budget_micros = EXCLUDED.configured_budget_micros,
            remaining_budget_micros = EXCLUDED.remaining_budget_micros,
            threshold_status = EXCLUDED.threshold_status,
            threshold_crossed_at = CASE
                WHEN balance_projections.threshold_status <> EXCLUDED.threshold_status
                 AND EXCLUDED.threshold_status IN ('warning', 'exceeded')
                THEN NOW()
                ELSE balance_projections.threshold_crossed_at
            END,
            last_projected_at = NOW()
        "#,
    )
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(&entry.currency)
    .bind(new_provider_cost_micros)
    .bind(new_billable_cost_micros)
    .bind(configured_budget_micros)
    .bind(remaining_budget_micros)
    .bind(&next_threshold_status)
    .execute(&mut **tx)
    .await
    .context("updating balance projection failed")?;

    maybe_record_budget_threshold_event(
        tx,
        entry,
        previous_threshold_status.as_deref(),
        &next_threshold_status,
        new_billable_cost_micros,
        configured_budget_micros,
    )
    .await
}

#[allow(dead_code)]
pub async fn rebuild_projections(pool: &PgPool) -> Result<()> {
    sqlx::query("TRUNCATE usage_daily_projections, balance_projections, budget_threshold_events")
        .execute(pool)
        .await
        .context("truncating projection tables failed")?;

    let rows = sqlx::query(
        r#"
        SELECT
            ledger_entry_id,
            usage_event_id,
            grant_id,
            ledger_entry_type,
            amount_micros,
            provider_cost_micros,
            billable_cost_micros,
            currency,
            recorded_at::text AS recorded_at,
            tenant_id,
            project_id,
            route_receipt_id,
            provider_resource_id,
            provider_id,
            model_alias,
            input_tokens,
            output_tokens,
            cached_input_tokens,
            idempotency_key,
            source_message_id,
            source_producer,
            source_request_id,
            source_trace_id,
            usage_phase
        FROM ledger_entries
        ORDER BY recorded_at ASC, created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("loading ledger entries for projection rebuild failed")?;

    for row in rows {
        let entry = PendingLedgerEntry {
            ledger_entry_id: row.try_get("ledger_entry_id")?,
            usage_event_id: UsageEventId::parse(row.try_get::<String, _>("usage_event_id")?)?,
            grant_id: row.try_get("grant_id")?,
            ledger_entry_type: row.try_get("ledger_entry_type")?,
            amount_micros: row.try_get("amount_micros")?,
            provider_cost_micros: row
                .try_get::<Option<i64>, _>("provider_cost_micros")?
                .unwrap_or_default(),
            billable_cost_micros: row
                .try_get::<Option<i64>, _>("billable_cost_micros")?
                .unwrap_or_else(|| row.try_get("amount_micros").unwrap_or_default()),
            currency: row.try_get("currency")?,
            recorded_at: row.try_get("recorded_at")?,
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            route_receipt_id: row.try_get("route_receipt_id")?,
            provider_resource_id: row.try_get("provider_resource_id")?,
            provider_id: row
                .try_get::<Option<String>, _>("provider_id")?
                .unwrap_or_else(|| "default".to_string()),
            model_alias: row
                .try_get::<Option<String>, _>("model_alias")?
                .unwrap_or_else(|| "unknown".to_string()),
            input_tokens: u32::try_from(
                row.try_get::<Option<i64>, _>("input_tokens")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            output_tokens: u32::try_from(
                row.try_get::<Option<i64>, _>("output_tokens")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            cached_input_tokens: u32::try_from(
                row.try_get::<Option<i64>, _>("cached_input_tokens")?
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            idempotency_key: row.try_get("idempotency_key")?,
            source_message_id: row.try_get("source_message_id")?,
            source_producer: row.try_get("source_producer")?,
            source_request_id: row.try_get("source_request_id")?,
            source_trace_id: row.try_get("source_trace_id")?,
            usage_phase: row.try_get("usage_phase")?,
        };

        let mut tx = pool
            .begin()
            .await
            .context("opening projection rebuild transaction failed")?;
        update_usage_daily_projection(&mut tx, &entry).await?;
        update_balance_projection(&mut tx, &entry).await?;
        tx.commit()
            .await
            .context("committing projection rebuild transaction failed")?;
    }

    Ok(())
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
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19::timestamptz,$20,$21,$22,$23,$24
        ) ON CONFLICT (idempotency_key) DO NOTHING
        "#,
    )
    .bind(&entry.ledger_entry_id)
    .bind(entry.usage_event_id.to_string())
    .bind(&entry.grant_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_amount_micros_supports_fractional_digits_and_padding() {
        assert_eq!(
            parse_estimated_cost_micros("0.1420").expect("parse"),
            142_000
        );
        assert_eq!(parse_estimated_cost_micros("2").expect("parse"), 2_000_000);
        assert_eq!(
            parse_estimated_cost_micros("1.5").expect("parse"),
            1_500_000
        );
        assert_eq!(
            parse_estimated_cost_micros("1.000001").expect("parse"),
            1_000_001
        );
        assert_eq!(
            parse_estimated_cost_micros("1.0000019").expect("parse"),
            1_000_001
        );
    }

    #[test]
    fn parse_amount_micros_rejects_invalid_values() {
        assert!(parse_estimated_cost_micros("abc").is_err());
        assert!(parse_estimated_cost_micros("1.2.3").is_err());
        assert!(parse_estimated_cost_micros("").is_err());
        assert!(parse_estimated_cost_micros("-1").is_err());
    }

    #[test]
    fn generated_ledger_entry_id_is_deterministic() {
        let first = ledger_entry_id("usageevt_123:final");
        let second = ledger_entry_id("usageevt_123:final");
        assert_eq!(first, second);
        assert!(first.starts_with("ledger_"));
    }

    #[test]
    fn threshold_status_tracks_budget_crossings() {
        assert_eq!(threshold_status(10_000_000, 75_000_000), "ok");
        assert_eq!(threshold_status(60_000_000, 75_000_000), "warning");
        assert_eq!(threshold_status(80_000_000, 75_000_000), "exceeded");
    }
}
