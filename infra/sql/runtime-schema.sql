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

CREATE TABLE IF NOT EXISTS opening_grants (
    grant_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default',
    config_snapshot_id TEXT NOT NULL,
    grantee_kind TEXT NOT NULL,
    grantee_id TEXT NOT NULL,
    status TEXT NOT NULL,
    credential_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS opening_grants_owner_active_idx
    ON opening_grants (tenant_id, project_id, owner_account_id, status, expires_at);

CREATE TABLE IF NOT EXISTS opening_grant_owner_locks (
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    owner_account_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (tenant_id, project_id, owner_account_id)
);

CREATE TABLE IF NOT EXISTS deliveries (
    delivery_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default',
    redemption_batch_id TEXT NULL,
    provider TEXT NOT NULL,
    status TEXT NOT NULL,
    operator_id TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS deliveries_tenant_project_idx
    ON deliveries (tenant_id, project_id, created_at);

CREATE INDEX IF NOT EXISTS deliveries_owner_idx
    ON deliveries (tenant_id, project_id, owner_account_id, created_at);

CREATE TABLE IF NOT EXISTS delivery_redemption_owner_locks (
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    owner_account_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (tenant_id, project_id, owner_account_id)
);

CREATE TABLE IF NOT EXISTS delivery_codes (
    code_id TEXT PRIMARY KEY,
    delivery_id TEXT NOT NULL,
    code_type TEXT NOT NULL,
    code_hash TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS delivery_codes_delivery_idx
    ON delivery_codes (delivery_id, code_type);

CREATE TABLE IF NOT EXISTS delivery_secret_plaintexts (
    delivery_id TEXT NOT NULL,
    code_type TEXT NOT NULL,
    secret_plaintext TEXT NOT NULL,
    created_at TEXT NOT NULL,
    used_at TEXT NULL,
    PRIMARY KEY (delivery_id, code_type)
);

CREATE TABLE IF NOT EXISTS delivery_entitlements (
    entitlement_id TEXT PRIMARY KEY,
    delivery_id TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,
    ends_at TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS delivery_artifacts (
    artifact_id TEXT PRIMARY KEY,
    delivery_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    version BIGINT NOT NULL,
    sha256 TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_backend TEXT NOT NULL,
    storage_ref TEXT NOT NULL,
    ciphertext BYTEA NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS delivery_artifacts_delivery_status_idx
    ON delivery_artifacts (delivery_id, status, version);

CREATE INDEX IF NOT EXISTS delivery_artifacts_tenant_project_idx
    ON delivery_artifacts (tenant_id, project_id, created_at);

CREATE TABLE IF NOT EXISTS delivery_upload_batches (
    batch_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    status TEXT NOT NULL,
    idempotency_key TEXT NULL,
    source_file_name TEXT NOT NULL,
    source_file_sha256 TEXT NOT NULL,
    total_count BIGINT NOT NULL,
    success_count BIGINT NOT NULL,
    failed_count BIGINT NOT NULL,
    duplicate_count BIGINT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS delivery_upload_batches_source_sha_uidx
    ON delivery_upload_batches (tenant_id, project_id, source_file_sha256);

CREATE UNIQUE INDEX IF NOT EXISTS delivery_upload_batches_idempotency_uidx
    ON delivery_upload_batches (tenant_id, project_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE INDEX IF NOT EXISTS delivery_upload_batches_scope_status_idx
    ON delivery_upload_batches (tenant_id, project_id, status, created_at);

CREATE TABLE IF NOT EXISTS delivery_upload_batch_items (
    item_id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    delivery_id TEXT NOT NULL,
    artifact_id TEXT NULL,
    status TEXT NOT NULL,
    row_index BIGINT NOT NULL,
    payload_sha256 TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    ciphertext BYTEA NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS delivery_upload_batch_items_batch_delivery_payload_uidx
    ON delivery_upload_batch_items (batch_id, delivery_id, payload_sha256);

CREATE INDEX IF NOT EXISTS delivery_upload_batch_items_batch_idx
    ON delivery_upload_batch_items (batch_id, row_index);

CREATE INDEX IF NOT EXISTS delivery_upload_batch_items_scope_status_idx
    ON delivery_upload_batch_items (tenant_id, project_id, status, created_at);

CREATE TABLE IF NOT EXISTS delivery_activations (
    activation_id TEXT PRIMARY KEY,
    delivery_id TEXT NOT NULL,
    code_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    entitlement_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    activated_at TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS delivery_activations_code_id_uidx
    ON delivery_activations (code_id);

CREATE INDEX IF NOT EXISTS delivery_activations_delivery_idx
    ON delivery_activations (delivery_id, activated_at);

CREATE INDEX IF NOT EXISTS delivery_activations_tenant_project_idx
    ON delivery_activations (tenant_id, project_id, activated_at);

CREATE TABLE IF NOT EXISTS delivery_download_grants (
    grant_id TEXT PRIMARY KEY,
    activation_id TEXT NOT NULL,
    delivery_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    entitlement_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    use_count BIGINT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS delivery_download_grants_activation_idx
    ON delivery_download_grants (activation_id, created_at);

CREATE INDEX IF NOT EXISTS delivery_download_grants_tenant_project_idx
    ON delivery_download_grants (tenant_id, project_id, created_at);

CREATE INDEX IF NOT EXISTS delivery_download_grants_status_expires_idx
    ON delivery_download_grants (status, expires_at);

CREATE TABLE IF NOT EXISTS delivery_service_segments (
    segment_id TEXT PRIMARY KEY,
    entitlement_id TEXT NOT NULL,
    activation_id TEXT NOT NULL,
    delivery_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    effective_from TEXT NOT NULL,
    effective_until TEXT NOT NULL,
    carrier_valid_until TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS delivery_service_segments_entitlement_idx
    ON delivery_service_segments (entitlement_id, effective_from);

CREATE INDEX IF NOT EXISTS delivery_service_segments_artifact_idx
    ON delivery_service_segments (artifact_id, status);

CREATE TABLE IF NOT EXISTS delivery_lifecycle_events (
    event_id TEXT PRIMARY KEY,
    entitlement_id TEXT NOT NULL,
    segment_id TEXT NULL,
    event_type TEXT NOT NULL,
    status TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS delivery_lifecycle_events_entitlement_idx
    ON delivery_lifecycle_events (entitlement_id, created_at);

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

CREATE TABLE IF NOT EXISTS merchant_shops (
    merchant_shop_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    slug TEXT NOT NULL,
    payload JSONB NOT NULL,
    UNIQUE (tenant_id, slug)
);

CREATE TABLE IF NOT EXISTS card_products (
    card_product_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    merchant_shop_id TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS merchant_product_experience_configs (
    card_product_id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    experience_kind TEXT NOT NULL,
    duration_minutes BIGINT NOT NULL,
    requires_phone BOOLEAN NOT NULL,
    is_active BOOLEAN NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS merchant_product_inventory (
    inventory_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    merchant_shop_id TEXT NOT NULL,
    card_product_id TEXT NOT NULL,
    delivery_id TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,
    reserved_order_id TEXT NULL,
    sold_order_id TEXT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS merchant_product_inventory_product_status_idx
    ON merchant_product_inventory (card_product_id, status, updated_at);

CREATE TABLE IF NOT EXISTS merchant_product_orders (
    order_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    merchant_shop_id TEXT NOT NULL,
    card_product_id TEXT NOT NULL,
    inventory_id TEXT NOT NULL,
    inventory_delivery_id TEXT NOT NULL,
    buyer_user_id TEXT NOT NULL,
    out_trade_no TEXT NULL UNIQUE,
    amount_total BIGINT NOT NULL,
    currency TEXT NOT NULL,
    channel TEXT NULL,
    status TEXT NOT NULL,
    pickup_token_hash TEXT NULL UNIQUE,
    activation_id TEXT NULL,
    download_grant_id TEXT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS merchant_product_orders_buyer_idx
    ON merchant_product_orders (buyer_user_id, created_at);

CREATE TABLE IF NOT EXISTS promotion_claims (
    claim_id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    phone_hash TEXT NOT NULL,
    buyer_user_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    status TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS promotion_claims_campaign_phone_uidx
    ON promotion_claims (campaign_id, phone_hash);

CREATE TABLE IF NOT EXISTS wechat_user_openids (
    user_id TEXT PRIMARY KEY,
    openid TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS trial_connections (
    trial_connection_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS relay_evaluations (
    relay_evaluation_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    trial_connection_id TEXT NOT NULL,
    replay_capsule_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS replay_capsules (
    replay_capsule_id TEXT PRIMARY KEY,
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

CREATE TABLE IF NOT EXISTS wechat_payment_orders (
    out_trade_no TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NULL,
    amount_total BIGINT NOT NULL,
    currency TEXT NOT NULL,
    channel TEXT NOT NULL,
    status TEXT NOT NULL,
    trade_state TEXT NULL,
    code_url TEXT NULL,
    prepay_id TEXT NULL,
    transaction_id TEXT NULL,
    notification_id TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    paid_at TEXT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS billing_renewal_intents (
    renewal_intent_id TEXT PRIMARY KEY,
    out_trade_no TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pricing_catalog_entries (
    catalog_id TEXT NOT NULL,
    catalog_version INTEGER NOT NULL,
    currency TEXT NOT NULL,
    dimension TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_alias TEXT NULL,
    model_alias_key TEXT GENERATED ALWAYS AS (COALESCE(model_alias, '')) STORED,
    region TEXT NULL,
    region_key TEXT GENERATED ALWAYS AS (COALESCE(region, '')) STORED,
    micros_per_unit BIGINT NOT NULL,
    billable_micros_per_unit BIGINT NOT NULL,
    unit_denominator BIGINT NOT NULL,
    source TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (
        catalog_id,
        catalog_version,
        dimension,
        provider_id,
        model_alias_key,
        region_key
    )
);

CREATE TABLE IF NOT EXISTS codex_auth_accounts (
    codex_account_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    project_id TEXT NULL,
    provider_resource_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    status TEXT NOT NULL,
    auth_json_sha256 TEXT NOT NULL,
    encrypted_auth_json JSONB NOT NULL,
    leased_until TEXT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS oauth_sharing_leases (
    lease_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    pool_id TEXT NOT NULL,
    borrower_workspace_id TEXT NOT NULL,
    status TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS oauth_carpools (
    carpool_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS oauth_pool_runtime_leases (
    runtime_lease_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    pool_id TEXT NULL,
    lease_id TEXT NULL,
    carpool_id TEXT NULL,
    workspace_id TEXT NULL,
    session_key TEXT NULL,
    holder_id TEXT NULL,
    operation_id TEXT NULL,
    status TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    heartbeat_at TEXT NOT NULL,
    released_at TEXT NULL,
    fencing_token BIGINT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS oauth_pool_runtime_leases_active_account_idx
    ON oauth_pool_runtime_leases (account_id, status, expires_at);

CREATE TABLE IF NOT EXISTS oauth_pool_session_bindings (
    binding_id TEXT PRIMARY KEY,
    session_key TEXT NOT NULL,
    provider TEXT NOT NULL,
    pool_id TEXT NULL,
    account_id TEXT NOT NULL,
    workspace_id TEXT NULL,
    model_id TEXT NULL,
    binding_policy TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    rebind_count BIGINT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (session_key, provider)
);

CREATE TABLE IF NOT EXISTS oauth_sharing_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    provider TEXT NOT NULL,
    lease_id TEXT NULL,
    carpool_id TEXT NULL,
    workspace_id TEXT NULL,
    account_id TEXT NULL,
    pool_id TEXT NULL,
    payload JSONB NOT NULL,
    created_at TEXT NOT NULL
);

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
);

ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS provider_cost_micros BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS grant_id TEXT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS owner_account_id TEXT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS billable_cost_micros BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS provider_id TEXT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS model_alias TEXT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS input_tokens BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS output_tokens BIGINT;
ALTER TABLE ledger_entries ADD COLUMN IF NOT EXISTS cached_input_tokens BIGINT;

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
);

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
);

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
);

ALTER TABLE usage_daily_projections ADD COLUMN IF NOT EXISTS owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default';
ALTER TABLE balance_projections ADD COLUMN IF NOT EXISTS owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default';
ALTER TABLE budget_threshold_events ADD COLUMN IF NOT EXISTS owner_account_id TEXT NOT NULL DEFAULT 'tenant_project_default';

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

COMMIT;
