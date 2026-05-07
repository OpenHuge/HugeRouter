use anyhow::{Context, Result};
use core_domain::{DEFAULT_OWNER_ACCOUNT_ID, UsageEventId};
use metering::{default_budget_micros_for_scope, threshold_status};
use sqlx::{PgPool, Postgres, Row, Transaction};

use super::entry::{BudgetThresholdEventRecord, PendingLedgerEntry};

const PROJECTION_SCHEMA_STATEMENTS: &[&str] = &[
    r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS provider_cost_micros BIGINT"#,
    r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS grant_id TEXT"#,
    r#"ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS owner_account_id TEXT"#,
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
        owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default',
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
        PRIMARY KEY (tenant_id, project_id, owner_account_id, usage_date, provider_id, model_alias)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS balance_projections (
        tenant_id TEXT NOT NULL,
        project_id TEXT NOT NULL,
        owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default',
        currency TEXT NOT NULL,
        provider_cost_micros BIGINT NOT NULL,
        billable_cost_micros BIGINT NOT NULL,
        configured_budget_micros BIGINT NOT NULL,
        remaining_budget_micros BIGINT NOT NULL,
        threshold_status TEXT NOT NULL,
        threshold_crossed_at TIMESTAMPTZ NULL,
        last_projected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (tenant_id, project_id, owner_account_id, currency)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS budget_threshold_events (
        budget_threshold_event_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        project_id TEXT NOT NULL,
        owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default',
        currency TEXT NOT NULL,
        threshold_status TEXT NOT NULL,
        billable_cost_micros BIGINT NOT NULL,
        configured_budget_micros BIGINT NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"ALTER TABLE usage_daily_projections ADD COLUMN IF NOT EXISTS owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default'"#,
    r#"ALTER TABLE balance_projections ADD COLUMN IF NOT EXISTS owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default'"#,
    r#"ALTER TABLE budget_threshold_events ADD COLUMN IF NOT EXISTS owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default'"#,
    r#"
    DO $$
    BEGIN
        IF EXISTS (
            SELECT 1
            FROM pg_constraint
            WHERE conrelid = 'usage_daily_projections'::regclass
              AND conname = 'usage_daily_projections_pkey'
              AND pg_get_constraintdef(oid) NOT LIKE '%owner_account_id%'
        ) THEN
            ALTER TABLE usage_daily_projections DROP CONSTRAINT usage_daily_projections_pkey;
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM pg_constraint
            WHERE conrelid = 'usage_daily_projections'::regclass
              AND conname = 'usage_daily_projections_pkey'
        ) THEN
            ALTER TABLE usage_daily_projections
                ADD CONSTRAINT usage_daily_projections_pkey
                PRIMARY KEY (tenant_id, project_id, owner_account_id, usage_date, provider_id, model_alias);
        END IF;
    END $$;
    "#,
    r#"
    DO $$
    BEGIN
        IF EXISTS (
            SELECT 1
            FROM pg_constraint
            WHERE conrelid = 'balance_projections'::regclass
              AND conname = 'balance_projections_pkey'
              AND pg_get_constraintdef(oid) NOT LIKE '%owner_account_id%'
        ) THEN
            ALTER TABLE balance_projections DROP CONSTRAINT balance_projections_pkey;
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM pg_constraint
            WHERE conrelid = 'balance_projections'::regclass
              AND conname = 'balance_projections_pkey'
        ) THEN
            ALTER TABLE balance_projections
                ADD CONSTRAINT balance_projections_pkey
                PRIMARY KEY (tenant_id, project_id, owner_account_id, currency);
        END IF;
    END $$;
    "#,
];

pub async fn ensure_projection_tables(pool: &PgPool) -> Result<()> {
    for statement in PROJECTION_SCHEMA_STATEMENTS {
        sqlx::query(statement)
            .execute(pool)
            .await
            .context("creating ledger projection tables failed")?;
    }

    Ok(())
}

pub(super) fn projection_owner_account_id(entry: &PendingLedgerEntry) -> &str {
    entry
        .owner_account_id
        .as_deref()
        .unwrap_or(DEFAULT_OWNER_ACCOUNT_ID)
}

pub(super) fn budget_threshold_event_id(entry: &PendingLedgerEntry, next_status: &str) -> String {
    format!(
        "budgetevt_{}_{}_{}_{}",
        entry.tenant_id,
        entry.project_id,
        projection_owner_account_id(entry),
        next_status
    )
}

pub(super) async fn update_usage_daily_projection(
    tx: &mut Transaction<'_, Postgres>,
    entry: &PendingLedgerEntry,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO usage_daily_projections (
            tenant_id,
            project_id,
            owner_account_id,
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
            $3,
            DATE($4::timestamptz),
            $5,
            $6,
            $7,
            $8,
            $9,
            $10,
            $11,
            $12,
            1
        )
        ON CONFLICT (tenant_id, project_id, owner_account_id, usage_date, provider_id, model_alias)
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
    .bind(projection_owner_account_id(entry))
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

    let event_id = budget_threshold_event_id(entry, next_status);

    sqlx::query(
        r#"
        INSERT INTO budget_threshold_events (
            budget_threshold_event_id,
            tenant_id,
            project_id,
            owner_account_id,
            currency,
            threshold_status,
            billable_cost_micros,
            configured_budget_micros
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (budget_threshold_event_id) DO NOTHING
        "#,
    )
    .bind(&event_id)
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(projection_owner_account_id(entry))
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
        owner_account_id: projection_owner_account_id(entry).to_string(),
        currency: entry.currency.clone(),
        threshold_status: next_status.to_string(),
        billable_cost_micros: new_billable_cost_micros,
        configured_budget_micros,
    }))
}

struct CurrentBalanceProjection {
    provider_cost_micros: i64,
    billable_cost_micros: i64,
    threshold_status: Option<String>,
}

struct BalanceProjectionUpdate {
    provider_cost_micros: i64,
    billable_cost_micros: i64,
    configured_budget_micros: i64,
    remaining_budget_micros: i64,
    threshold_status: String,
}

impl BalanceProjectionUpdate {
    fn from_entry(current: &CurrentBalanceProjection, entry: &PendingLedgerEntry) -> Self {
        let provider_cost_micros = current.provider_cost_micros + entry.provider_cost_micros;
        let billable_cost_micros = current.billable_cost_micros + entry.billable_cost_micros;
        let configured_budget_micros = default_budget_micros_for_scope(
            &entry.tenant_id,
            &entry.project_id,
            Some(projection_owner_account_id(entry)),
        );
        let remaining_budget_micros = configured_budget_micros - billable_cost_micros;
        let threshold_status =
            threshold_status(billable_cost_micros, configured_budget_micros).to_string();

        Self {
            provider_cost_micros,
            billable_cost_micros,
            configured_budget_micros,
            remaining_budget_micros,
            threshold_status,
        }
    }
}

async fn load_current_balance_projection(
    tx: &mut Transaction<'_, Postgres>,
    entry: &PendingLedgerEntry,
) -> Result<CurrentBalanceProjection> {
    let current = sqlx::query(
        r#"
        SELECT provider_cost_micros, billable_cost_micros, threshold_status
        FROM balance_projections
        WHERE tenant_id = $1 AND project_id = $2 AND owner_account_id = $3 AND currency = $4
        FOR UPDATE
        "#,
    )
    .bind(&entry.tenant_id)
    .bind(&entry.project_id)
    .bind(projection_owner_account_id(entry))
    .bind(&entry.currency)
    .fetch_optional(&mut **tx)
    .await
    .context("loading current balance projection failed")?;

    Ok(CurrentBalanceProjection {
        provider_cost_micros: current
            .as_ref()
            .and_then(|row| row.try_get::<i64, _>("provider_cost_micros").ok())
            .unwrap_or_default(),
        billable_cost_micros: current
            .as_ref()
            .and_then(|row| row.try_get::<i64, _>("billable_cost_micros").ok())
            .unwrap_or_default(),
        threshold_status: current
            .as_ref()
            .and_then(|row| row.try_get::<String, _>("threshold_status").ok()),
    })
}

async fn upsert_balance_projection(
    tx: &mut Transaction<'_, Postgres>,
    entry: &PendingLedgerEntry,
    update: &BalanceProjectionUpdate,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO balance_projections (
            tenant_id,
            project_id,
            owner_account_id,
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
            $9,
            CASE WHEN $9 IN ('warning', 'exceeded') THEN NOW() ELSE NULL END,
            NOW()
        )
        ON CONFLICT (tenant_id, project_id, owner_account_id, currency)
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
    .bind(projection_owner_account_id(entry))
    .bind(&entry.currency)
    .bind(update.provider_cost_micros)
    .bind(update.billable_cost_micros)
    .bind(update.configured_budget_micros)
    .bind(update.remaining_budget_micros)
    .bind(&update.threshold_status)
    .execute(&mut **tx)
    .await
    .context("updating balance projection failed")?;

    Ok(())
}

pub(super) async fn update_balance_projection(
    tx: &mut Transaction<'_, Postgres>,
    entry: &PendingLedgerEntry,
) -> Result<Option<BudgetThresholdEventRecord>> {
    let current = load_current_balance_projection(tx, entry).await?;
    let update = BalanceProjectionUpdate::from_entry(&current, entry);

    upsert_balance_projection(tx, entry, &update).await?;

    maybe_record_budget_threshold_event(
        tx,
        entry,
        current.threshold_status.as_deref(),
        &update.threshold_status,
        update.billable_cost_micros,
        update.configured_budget_micros,
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
            owner_account_id,
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
            owner_account_id: row.try_get("owner_account_id")?,
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
