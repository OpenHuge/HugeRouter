pub const REQUIRED_TABLES: &[&str] = &[
    "tenants",
    "projects",
    "provider_resources",
    "route_policies",
    "api_keys",
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
    "billing_export_jobs",
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
];
