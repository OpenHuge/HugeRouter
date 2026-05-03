pub const REQUIRED_TABLES: &[&str] = &[
    "tenants",
    "projects",
    "provider_resources",
    "route_policies",
    "api_keys",
    "opening_grants",
    "config_snapshots",
    "active_config_pointers",
    "users",
    "tenant_memberships",
    "auth_provider_links",
    "sessions",
    "login_flows",
    "merchant_shops",
    "card_products",
    "trial_connections",
    "relay_evaluations",
    "replay_capsules",
    "route_receipts",
    "route_receipt_diagnostics",
    "billing_export_jobs",
    "wechat_payment_orders",
    "billing_renewal_intents",
    "pricing_catalog_entries",
    "codex_auth_accounts",
    "oauth_sharing_leases",
    "oauth_carpools",
    "oauth_pool_runtime_leases",
    "oauth_pool_session_bindings",
    "oauth_sharing_audit_events",
];

pub const MIGRATIONS: &[&str] = &[
    r"CREATE TABLE IF NOT EXISTS tenants (
        tenant_id TEXT PRIMARY KEY,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS projects (
        project_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS provider_resources (
        provider_resource_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        project_id TEXT NULL,
        provider_id TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS route_policies (
        route_policy_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS disabled_route_policies (
        route_policy_id TEXT PRIMARY KEY,
        disabled_at TEXT NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS api_keys (
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
    )",
    r"CREATE TABLE IF NOT EXISTS opening_grants (
        grant_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        project_id TEXT NOT NULL,
        config_snapshot_id TEXT NOT NULL,
        grantee_kind TEXT NOT NULL,
        grantee_id TEXT NOT NULL,
        status TEXT NOT NULL,
        credential_hash TEXT NOT NULL UNIQUE,
        payload JSONB NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS config_snapshots (
        config_snapshot_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        project_id TEXT NOT NULL,
        status TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS active_config_pointers (
        pointer_key TEXT PRIMARY KEY,
        config_snapshot_id TEXT NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS users (
        user_id TEXT PRIMARY KEY,
        primary_email TEXT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS tenant_memberships (
        membership_id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        tenant_id TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS auth_provider_links (
        link_id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        provider TEXT NOT NULL,
        provider_subject TEXT NOT NULL,
        email TEXT NULL,
        can_unlink BOOLEAN NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE UNIQUE INDEX IF NOT EXISTS auth_provider_links_subject_key
       ON auth_provider_links (provider, provider_subject)",
    r"CREATE TABLE IF NOT EXISTS sessions (
        session_id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        state TEXT NOT NULL,
        active_tenant_id TEXT NULL,
        authenticated_by TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS login_flows (
        flow_id TEXT PRIMARY KEY,
        flow_kind TEXT NOT NULL,
        email TEXT NULL,
        provider TEXT NULL,
        workspace_slug TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS merchant_shops (
        merchant_shop_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        slug TEXT NOT NULL,
        payload JSONB NOT NULL,
        UNIQUE (tenant_id, slug)
    )",
    r"CREATE TABLE IF NOT EXISTS card_products (
        card_product_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        merchant_shop_id TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS trial_connections (
        trial_connection_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS relay_evaluations (
        relay_evaluation_id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        trial_connection_id TEXT NOT NULL,
        replay_capsule_id TEXT NOT NULL,
        created_at TEXT NOT NULL,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS replay_capsules (
        replay_capsule_id TEXT PRIMARY KEY,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS route_receipts (
        route_receipt_id TEXT PRIMARY KEY,
        payload JSONB NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS route_receipt_diagnostics (
        route_receipt_id TEXT PRIMARY KEY,
        decision_timeline JSONB NOT NULL,
        policy_checks JSONB NOT NULL,
        provider_attempts JSONB NOT NULL,
        source_message_id TEXT NOT NULL,
        source_producer TEXT NOT NULL,
        source_request_id TEXT NULL,
        source_trace_id TEXT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )",
    r"CREATE TABLE IF NOT EXISTS billing_export_jobs (
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
    )",
    r"CREATE TABLE IF NOT EXISTS wechat_payment_orders (
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
    )",
    r"CREATE TABLE IF NOT EXISTS billing_renewal_intents (
        renewal_intent_id TEXT PRIMARY KEY,
        out_trade_no TEXT NOT NULL,
        grant_id TEXT NOT NULL,
        tenant_id TEXT NOT NULL,
        project_id TEXT NOT NULL,
        status TEXT NOT NULL,
        payload JSONB NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS pricing_catalog_entries (
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
    )",
    r"CREATE TABLE IF NOT EXISTS codex_auth_accounts (
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
    )",
    r"CREATE TABLE IF NOT EXISTS oauth_sharing_leases (
        lease_id TEXT PRIMARY KEY,
        provider TEXT NOT NULL,
        pool_id TEXT NOT NULL,
        borrower_workspace_id TEXT NOT NULL,
        status TEXT NOT NULL,
        payload JSONB NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS oauth_carpools (
        carpool_id TEXT PRIMARY KEY,
        provider TEXT NOT NULL,
        enabled BOOLEAN NOT NULL,
        payload JSONB NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS oauth_pool_runtime_leases (
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
    )",
    r"CREATE INDEX IF NOT EXISTS oauth_pool_runtime_leases_active_account_idx
       ON oauth_pool_runtime_leases (account_id, status, expires_at)",
    r"CREATE TABLE IF NOT EXISTS oauth_pool_session_bindings (
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
    )",
    r"CREATE TABLE IF NOT EXISTS oauth_sharing_audit_events (
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
    )",
];
