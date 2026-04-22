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
use protocol_ir::UsageEventRecordedMessage;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionOutcome {
    Inserted,
    Duplicate,
}

#[derive(Debug, Clone)]
pub struct PendingLedgerEntry {
    pub ledger_entry_id: String,
    pub usage_event_id: UsageEventId,
    pub ledger_entry_type: String,
    pub amount_micros: i64,
    pub currency: String,
    pub recorded_at: String,
    pub tenant_id: String,
    pub project_id: String,
    pub route_receipt_id: String,
    pub provider_resource_id: String,
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

pub fn usage_phase_as_str(phase: &UsagePhase) -> &'static str {
    match phase {
        UsagePhase::Reserve => "reserve",
        UsagePhase::Partial => "partial",
        UsagePhase::Final => "final",
        UsagePhase::Release => "release",
    }
}

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

pub fn build_pending_entry(event: &UsageEventRecordedMessage) -> Result<PendingLedgerEntry> {
    if event.idempotency_key != event.payload.usage_event.idempotency_key {
        anyhow::bail!("envelope idempotency key does not match usage event idempotency key");
    }

    let amount_micros =
        parse_estimated_cost_micros(&event.payload.usage_event.estimated_cost.amount)?;

    Ok(PendingLedgerEntry {
        ledger_entry_id: ledger_entry_id(&event.idempotency_key),
        usage_event_id: event.payload.usage_event.usage_event_id.clone(),
        ledger_entry_type: "usage_debit".to_string(),
        amount_micros,
        currency: event.payload.usage_event.estimated_cost.currency.clone(),
        recorded_at: event.payload.usage_event.recorded_at.clone(),
        tenant_id: event.payload.usage_event.tenant_id.to_string(),
        project_id: event.payload.usage_event.project_id.to_string(),
        route_receipt_id: event.payload.usage_event.route_receipt_id.to_string(),
        provider_resource_id: event.payload.usage_event.provider_resource_id.to_string(),
        idempotency_key: event.idempotency_key.clone(),
        source_message_id: event.message_id.clone(),
        source_producer: event.producer.0.clone(),
        source_request_id: event.request_id.clone(),
        source_trace_id: event.trace_id.clone(),
        usage_phase: usage_phase_as_str(&event.payload.usage_event.phase).to_string(),
    })
}

pub async fn persist_usage_event(
    pool: &PgPool,
    event: &UsageEventRecordedMessage,
) -> Result<IngestionOutcome> {
    let entry = build_pending_entry(event)?;
    let result = sqlx::query(
        r#"
        INSERT INTO ledger_entries (
            ledger_entry_id,
            usage_event_id,
            usage_phase,
            tenant_id,
            project_id,
            route_receipt_id,
            provider_resource_id,
            ledger_entry_type,
            amount_micros,
            currency,
            recorded_at,
            idempotency_key,
            source_message_id,
            source_producer,
            source_request_id,
            source_trace_id
        ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16
        ) ON CONFLICT (idempotency_key) DO NOTHING
        "#,
    )
    .bind(&entry.ledger_entry_id)
    .bind(entry.usage_event_id.to_string())
    .bind(&entry.usage_phase)
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(&entry.route_receipt_id)
    .bind(&entry.provider_resource_id)
    .bind(&entry.ledger_entry_type)
    .bind(entry.amount_micros)
    .bind(&entry.currency)
    .bind(&entry.recorded_at)
    .bind(&entry.idempotency_key)
    .bind(&entry.source_message_id)
    .bind(&entry.source_producer)
    .bind(&entry.source_request_id)
    .bind(&entry.source_trace_id)
    .execute(pool)
    .await
    .context("writing ledger entry failed")?;

    if result.rows_affected() == 0 {
        debug!(
            idempotency_key = entry.idempotency_key,
            "skipping duplicate usage event ledger entry"
        );
        Ok(IngestionOutcome::Duplicate)
    } else {
        info!(
            idempotency_key = entry.idempotency_key,
            usage_event_id = %entry.usage_event_id,
            ledger_entry_id = %entry.ledger_entry_id,
            amount_micros = entry.amount_micros,
            "inserted ledger entry"
        );
        Ok(IngestionOutcome::Inserted)
    }
}

pub async fn handle_usage_event_recorded(
    pool: &PgPool,
    payload: &[u8],
) -> Result<IngestionOutcome> {
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
}
