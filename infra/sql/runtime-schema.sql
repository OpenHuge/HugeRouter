BEGIN;

CREATE TABLE IF NOT EXISTS tenants (
    tenant_id TEXT PRIMARY KEY,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_resources (
    provider_resource_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NULL,
    provider_id TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS route_policies (
    route_policy_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS disabled_route_policies (
    route_policy_id TEXT PRIMARY KEY,
    disabled_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_keys (
    api_key_id TEXT PRIMARY KEY,
    provider_resource_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    project_id TEXT NULL,
    display_name TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    hash TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL,
    version BIGINT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS config_snapshots (
    config_snapshot_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS active_config_pointers (
    pointer_key TEXT PRIMARY KEY,
    config_snapshot_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    user_id TEXT PRIMARY KEY,
    primary_email TEXT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS tenant_memberships (
    membership_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS auth_provider_links (
    link_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    email TEXT NULL,
    can_unlink BOOLEAN NOT NULL,
    payload JSONB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS auth_provider_links_subject_key
    ON auth_provider_links (provider, provider_subject);

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    state TEXT NOT NULL,
    active_tenant_id TEXT NULL,
    authenticated_by TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS login_flows (
    flow_id TEXT PRIMARY KEY,
    flow_kind TEXT NOT NULL,
    email TEXT NULL,
    provider TEXT NULL,
    workspace_slug TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS route_receipts (
    route_receipt_id TEXT PRIMARY KEY,
    payload JSONB NOT NULL
);

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
);

CREATE TABLE IF NOT EXISTS billing_export_jobs (
    export_job_id TEXT PRIMARY KEY,
    tenant_id TEXT NULL,
    project_id TEXT NULL,
    window_start TEXT NOT NULL,
    window_end TEXT NOT NULL,
    format TEXT NOT NULL,
    status TEXT NOT NULL,
    requested_at TEXT NOT NULL,
    completed_at TEXT NULL,
    error_message TEXT NULL,
    export_content TEXT NULL,
    content_type TEXT NULL
);

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
);

ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS provider_cost_micros BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS billable_cost_micros BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS provider_id TEXT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS model_alias TEXT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS input_tokens BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS output_tokens BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS cached_input_tokens BIGINT;

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
);

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
);

CREATE TABLE IF NOT EXISTS budget_threshold_events (
    budget_threshold_event_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    currency TEXT NOT NULL,
    threshold_status TEXT NOT NULL,
    billable_cost_micros BIGINT NOT NULL,
    configured_budget_micros BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMIT;
