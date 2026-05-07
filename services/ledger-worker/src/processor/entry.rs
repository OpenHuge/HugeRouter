use anyhow::{Context, Result};
use core_domain::{UsageEventId, UsagePhase};
use metering::quote_usage;
use protocol_ir::UsageEventRecordedMessage;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

#[derive(Debug, Clone)]
pub struct BudgetThresholdEventRecord {
    pub budget_threshold_event_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub owner_account_id: String,
    pub currency: String,
    pub threshold_status: String,
    pub billable_cost_micros: i64,
    pub configured_budget_micros: i64,
}

#[derive(Debug, Clone)]
pub struct PendingLedgerEntry {
    pub ledger_entry_id: String,
    pub usage_event_id: UsageEventId,
    pub grant_id: Option<String>,
    pub owner_account_id: Option<String>,
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

pub(super) fn build_pending_entry_with_provider_id(
    event: &UsageEventRecordedMessage,
    provider_id: String,
) -> Result<PendingLedgerEntry> {
    if event.idempotency_key != event.payload.usage_event.idempotency_key {
        anyhow::bail!("envelope idempotency key does not match usage event idempotency key");
    }

    let quote = quote_usage(&provider_id, &event.payload.usage_event.usage);

    Ok(PendingLedgerEntry {
        ledger_entry_id: ledger_entry_id(&event.idempotency_key),
        usage_event_id: event.payload.usage_event.usage_event_id.clone(),
        grant_id: event.payload.usage_event.grant_id.clone(),
        owner_account_id: event.payload.usage_event.owner_account_id.clone(),
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

pub async fn build_pending_entry(
    pool: &PgPool,
    event: &UsageEventRecordedMessage,
) -> Result<PendingLedgerEntry> {
    let provider_id = resolve_provider_id(
        pool,
        event.payload.usage_event.provider_resource_id.as_str(),
    )
    .await?;

    build_pending_entry_with_provider_id(event, provider_id)
}
