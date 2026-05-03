BEGIN;

INSERT INTO tenants (tenant_id, payload)
VALUES
    (
        'tenant_platform',
        $json${"tenant_id":"tenant_platform","slug":"platform-admin","display_name":"Platform Admin","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'tenant_acme',
        $json${"tenant_id":"tenant_acme","slug":"acme-retail","display_name":"Acme Retail","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'tenant_northstar',
        $json${"tenant_id":"tenant_northstar","slug":"northstar-labs","display_name":"Northstar Labs","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    )
ON CONFLICT (tenant_id) DO UPDATE
SET payload = EXCLUDED.payload;

INSERT INTO projects (project_id, tenant_id, payload)
VALUES
    (
        'proj_core',
        'tenant_acme',
        $json${"project_id":"proj_core","tenant_id":"tenant_acme","slug":"core-gateway","display_name":"Core Gateway","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'proj_acme_ops',
        'tenant_acme',
        $json${"project_id":"proj_acme_ops","tenant_id":"tenant_acme","slug":"retail-ops","display_name":"Retail Operations Control","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'proj_acme_support',
        'tenant_acme',
        $json${"project_id":"proj_acme_support","tenant_id":"tenant_acme","slug":"store-support","display_name":"Store Support Agent","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'proj_ns_research',
        'tenant_northstar',
        $json${"project_id":"proj_ns_research","tenant_id":"tenant_northstar","slug":"research-qa","display_name":"Research QA","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    )
ON CONFLICT (project_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    payload = EXCLUDED.payload;

INSERT INTO provider_resources (
    provider_resource_id,
    tenant_id,
    project_id,
    provider_id,
    payload
)
VALUES
    (
        'prvrsrc_openai_primary',
        'tenant_acme',
        'proj_core',
        'openai',
        $json${"provider_resource_id":"prvrsrc_openai_primary","tenant_id":"tenant_acme","project_id":"proj_core","provider_id":"openai","name":"OpenAI Primary","status":"active","provenance_class":"official_api","credential_owner_type":"platform","deployment_scope":"shared","region":"us-east-1","endpoint_base_url":"https://api.openai.com/v1","auth_kind":"api_key","health_state":"healthy","health_message":"probe latency within SLO","quarantine_reason":null,"budget_policy_id":null,"capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true,"supports_realtime":false,"supports_response_model_metadata":true},"supported_protocol_families":["openai_chat","openai_responses","openai_images"],"is_transit_gateway":false,"version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'prvrsrc_openai_backup',
        'tenant_acme',
        'proj_core',
        'openai',
        $json${"provider_resource_id":"prvrsrc_openai_backup","tenant_id":"tenant_acme","project_id":"proj_core","provider_id":"openai","name":"OpenAI Backup","status":"active","provenance_class":"official_api","credential_owner_type":"platform","deployment_scope":"shared","region":"us-west-2","endpoint_base_url":"https://api.openai.com/v1","auth_kind":"api_key","health_state":"healthy","health_message":"backup target healthy","quarantine_reason":null,"budget_policy_id":null,"capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true,"supports_realtime":false,"supports_response_model_metadata":true},"supported_protocol_families":["openai_chat","openai_responses","openai_images"],"is_transit_gateway":false,"version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'prvrsrc_openai_research',
        'tenant_northstar',
        'proj_ns_research',
        'openai',
        $json${"provider_resource_id":"prvrsrc_openai_research","tenant_id":"tenant_northstar","project_id":"proj_ns_research","provider_id":"openai","name":"OpenAI Research","status":"active","provenance_class":"official_api","credential_owner_type":"tenant","deployment_scope":"tenant_dedicated","region":"us-west-2","endpoint_base_url":"https://api.openai.com/v1","auth_kind":"api_key","health_state":"degraded","health_message":"research endpoint latency elevated","quarantine_reason":null,"budget_policy_id":null,"capabilities":{"supports_streaming":true,"supports_tool_calling":false,"supports_json_mode":true,"supports_realtime":false,"supports_response_model_metadata":true},"supported_protocol_families":["openai_chat"],"is_transit_gateway":false,"version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    )
ON CONFLICT (provider_resource_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    project_id = EXCLUDED.project_id,
    provider_id = EXCLUDED.provider_id,
    payload = EXCLUDED.payload;

INSERT INTO route_policies (route_policy_id, tenant_id, payload)
VALUES
    (
        'routepol_openai_chat_default',
        'tenant_acme',
        $json${"route_policy_id":"routepol_openai_chat_default","tenant_id":"tenant_acme","display_name":"Acme Reasoning Fast","protocol_family":"openai_chat","model_alias":"reasoning-fast","required_capabilities":["json_mode"],"preferred_regions":["us-east-1"],"version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'routepol_acme_support',
        'tenant_acme',
        $json${"route_policy_id":"routepol_acme_support","tenant_id":"tenant_acme","display_name":"Acme Support Safe","protocol_family":"openai_chat","model_alias":"support-safe","required_capabilities":["json_mode"],"preferred_regions":["us-west-2"],"version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    ),
    (
        'routepol_northstar_research',
        'tenant_northstar',
        $json${"route_policy_id":"routepol_northstar_research","tenant_id":"tenant_northstar","display_name":"Northstar Research","protocol_family":"openai_chat","model_alias":"research-fast","required_capabilities":["json_mode"],"preferred_regions":["us-west-2"],"version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
    )
ON CONFLICT (route_policy_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    payload = EXCLUDED.payload;

INSERT INTO config_snapshots (
    config_snapshot_id,
    tenant_id,
    project_id,
    status,
    payload
)
VALUES
    (
        'cfgsnap_gateway_v1',
        'tenant_acme',
        'proj_core',
        'active',
        $json${"config_snapshot_id":"cfgsnap_gateway_v1","tenant_id":"tenant_acme","project_id":"proj_core","revision":1,"status":"active","activated_at":"2026-04-22T00:00:00Z","provider_resource_ids":["prvrsrc_openai_primary","prvrsrc_openai_backup"],"route_policy_id":"routepol_openai_chat_default","budget_policy_id":"budgetpol_default"}$json$::jsonb
    ),
    (
        'cfgsnap_research_v1',
        'tenant_northstar',
        'proj_ns_research',
        'draft',
        $json${"config_snapshot_id":"cfgsnap_research_v1","tenant_id":"tenant_northstar","project_id":"proj_ns_research","revision":1,"status":"draft","activated_at":null,"provider_resource_ids":["prvrsrc_openai_research"],"route_policy_id":"routepol_northstar_research","budget_policy_id":"budgetpol_default"}$json$::jsonb
    )
ON CONFLICT (config_snapshot_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    project_id = EXCLUDED.project_id,
    status = EXCLUDED.status,
    payload = EXCLUDED.payload;

INSERT INTO active_config_pointers (pointer_key, config_snapshot_id)
VALUES ('default', 'cfgsnap_gateway_v1')
ON CONFLICT (pointer_key) DO UPDATE
SET config_snapshot_id = EXCLUDED.config_snapshot_id;

INSERT INTO merchant_shops (merchant_shop_id, tenant_id, slug, payload)
VALUES (
    'mshop_acme',
    'tenant_acme',
    'acme-small-shop',
    $json${"merchant_shop_id":"mshop_acme","tenant_id":"tenant_acme","slug":"acme-small-shop","display_name":"Acme Small Shop","status":"active","announcement":"Fresh relay trial cards with replay-backed evaluation.","fulfillment_mode":"auto_card_secret","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
)
ON CONFLICT (merchant_shop_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    slug = EXCLUDED.slug,
    payload = EXCLUDED.payload;

INSERT INTO card_products (card_product_id, tenant_id, merchant_shop_id, payload)
VALUES (
    'cardprod_acme_trial',
    'tenant_acme',
    'mshop_acme',
    $json${"card_product_id":"cardprod_acme_trial","tenant_id":"tenant_acme","merchant_shop_id":"mshop_acme","title":"Claude Trial Pack","description":"Starter batch for relay verification and low-risk onboarding.","status":"active","inventory_count":32,"face_value_usd":"1.00","retail_price_usd":"1.99","delivery_kind":"direct_secret","supports_trial":true,"version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
)
ON CONFLICT (card_product_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    merchant_shop_id = EXCLUDED.merchant_shop_id,
    payload = EXCLUDED.payload;

INSERT INTO trial_connections (trial_connection_id, tenant_id, payload)
VALUES (
    'trialconn_acme_relay',
    'tenant_acme',
    $json${"trial_connection_id":"trialconn_acme_relay","tenant_id":"tenant_acme","provider_label":"Acme Relay","endpoint_base_url":"https://relay.acme.example/v1","api_key_masked":"sk-trial...acme","target_model":"claude-sonnet","status":"active","notes":"Dedicated trial key only; never attach production traffic.","last_verified_at":"2026-04-22T00:00:00Z","version":1,"created_at":"2026-04-22T00:00:00Z","updated_at":"2026-04-22T00:00:00Z"}$json$::jsonb
)
ON CONFLICT (trial_connection_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    payload = EXCLUDED.payload;

INSERT INTO replay_capsules (replay_capsule_id, payload)
VALUES (
    'replay_acme_relay_eval',
    $json${"replay_capsule_id":"replay_acme_relay_eval","request_id":"req_merchant_eval_acme","trace_id":"trace_merchant_eval_acme","route_receipt_id":"routercpt_acme_relay_eval","config_snapshot_id":"cfgsnap_gateway_v1","redaction_tier":"structured_redacted","normalized_request_summary":{"protocol_family":"openai_chat","model_alias":"claude-sonnet","estimated_prompt_tokens":480},"upstream_error_summary":{"code":"provider_signature_mismatch"}}$json$::jsonb
)
ON CONFLICT (replay_capsule_id) DO UPDATE
SET payload = EXCLUDED.payload;

INSERT INTO route_receipts (route_receipt_id, payload)
VALUES (
    'routercpt_acme_relay_eval',
    $json${"route_receipt_id":"routercpt_acme_relay_eval","tenant_id":"tenant_acme","project_id":"proj_core","route_policy_id":"routepol_acme_default","request_id":"req_merchant_eval_acme","trace_id":"trace_merchant_eval_acme","protocol_family":"openai_chat","model_alias":"claude-sonnet","config_snapshot_id":"cfgsnap_gateway_v1","admission_result":"admitted","selected_target":"prvrsrc_openai_primary","excluded_targets":[],"score_breakdown":{"latency":0.8,"cost":0.7,"health":1.0,"trust":0.9},"fallback_transitions":[],"created_at":"2026-04-22T00:00:00Z"}$json$::jsonb
)
ON CONFLICT (route_receipt_id) DO UPDATE
SET payload = EXCLUDED.payload;

INSERT INTO relay_evaluations (
    relay_evaluation_id,
    tenant_id,
    trial_connection_id,
    replay_capsule_id,
    created_at,
    payload
)
VALUES (
    'reval_acme_relay',
    'tenant_acme',
    'trialconn_acme_relay',
    'replay_acme_relay_eval',
    '2026-04-22T00:00:00Z',
    $json${"relay_evaluation_id":"reval_acme_relay","tenant_id":"tenant_acme","trial_connection_id":"trialconn_acme_relay","replay_capsule_id":"replay_acme_relay_eval","provider_label":"Acme Relay","endpoint_base_url":"https://relay.acme.example/v1","target_model":"claude-sonnet","runner_mode":"simulated","sample_request_count":5,"estimated_tokens_saved":2400,"overall_score":82,"verdict":"warning","fingerprint_status":"pass","protocol_status":"warning","token_status":"warning","multimodal_status":"not_tested","detected_channel":"vertex","summary":"Replay capsule captured; protocol and token behavior still need manual follow-up.","created_at":"2026-04-22T00:00:00Z"}$json$::jsonb
)
ON CONFLICT (relay_evaluation_id) DO UPDATE
SET tenant_id = EXCLUDED.tenant_id,
    trial_connection_id = EXCLUDED.trial_connection_id,
    replay_capsule_id = EXCLUDED.replay_capsule_id,
    created_at = EXCLUDED.created_at,
    payload = EXCLUDED.payload;
INSERT INTO pricing_catalog_entries (
    catalog_id,
    catalog_version,
    currency,
    dimension,
    provider_id,
    model_alias,
    region,
    micros_per_unit,
    billable_micros_per_unit,
    unit_denominator,
    source
)
VALUES
    ('pricing_catalog_default', 1, 'USD', 'input_tokens', 'openai', NULL, 'global', 2500, 3000, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'output_tokens', 'openai', NULL, 'global', 8500, 10200, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'cached_input_tokens', 'openai', NULL, 'global', 750, 900, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'image_generations', 'openai', NULL, 'global', 18000, 21600, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'audio_seconds', 'openai', NULL, 'global', 1500, 1800, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'input_tokens', 'anthropic', NULL, 'global', 3000, 3660, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'output_tokens', 'anthropic', NULL, 'global', 9000, 10980, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'cached_input_tokens', 'anthropic', NULL, 'global', 900, 1098, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'image_generations', 'anthropic', NULL, 'global', 21000, 25620, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'audio_seconds', 'anthropic', NULL, 'global', 1800, 2196, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'input_tokens', 'bedrock', NULL, 'global', 6000, 7320, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'output_tokens', 'bedrock', NULL, 'global', 30000, 36600, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'cached_input_tokens', 'bedrock', NULL, 'global', 600, 732, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'image_generations', 'bedrock', NULL, 'global', 20000, 24400, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'audio_seconds', 'bedrock', NULL, 'global', 1600, 1952, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'input_tokens', 'gemini', NULL, 'global', 1800, 2124, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'output_tokens', 'gemini', NULL, 'global', 5500, 6490, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'cached_input_tokens', 'gemini', NULL, 'global', 450, 531, 1000, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'image_generations', 'gemini', NULL, 'global', 14000, 16520, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'audio_seconds', 'gemini', NULL, 'global', 1000, 1180, 1, 'provider_native'),
    ('pricing_catalog_default', 1, 'USD', 'input_tokens', 'default', NULL, 'global', 2000, 2500, 1000, 'platform_catalog'),
    ('pricing_catalog_default', 1, 'USD', 'output_tokens', 'default', NULL, 'global', 6000, 7500, 1000, 'platform_catalog'),
    ('pricing_catalog_default', 1, 'USD', 'cached_input_tokens', 'default', NULL, 'global', 500, 625, 1000, 'platform_catalog'),
    ('pricing_catalog_default', 1, 'USD', 'image_generations', 'default', NULL, 'global', 15000, 18750, 1, 'platform_catalog'),
    ('pricing_catalog_default', 1, 'USD', 'audio_seconds', 'default', NULL, 'global', 1200, 1500, 1, 'platform_catalog')
ON CONFLICT (catalog_id, catalog_version, dimension, provider_id, model_alias_key, region_key)
DO UPDATE SET
    currency = EXCLUDED.currency,
    micros_per_unit = EXCLUDED.micros_per_unit,
    billable_micros_per_unit = EXCLUDED.billable_micros_per_unit,
    unit_denominator = EXCLUDED.unit_denominator,
    source = EXCLUDED.source,
    updated_at = NOW();

INSERT INTO users (user_id, primary_email, payload)
VALUES
    (
        'user_ops',
        'ops@huge-router.dev',
        $json${"userId":"user_ops","primaryEmail":"ops@huge-router.dev","displayName":"Operations Admin","avatarUrl":null,"createdAt":"2026-04-22T00:00:00Z","lastLoginAt":null}$json$::jsonb
    ),
    (
        'user_developer',
        'developer@huge-router.dev',
        $json${"userId":"user_developer","primaryEmail":"developer@huge-router.dev","displayName":"Acme Developer","avatarUrl":null,"createdAt":"2026-04-22T00:00:00Z","lastLoginAt":null}$json$::jsonb
    )
ON CONFLICT (user_id) DO UPDATE
SET primary_email = EXCLUDED.primary_email,
    payload = EXCLUDED.payload;

INSERT INTO tenant_memberships (membership_id, user_id, tenant_id, payload)
VALUES
    (
        'tmemb_platform',
        'user_ops',
        'tenant_platform',
        $json${"membershipId":"tmemb_platform","tenant":{"id":"tenant_platform","slug":"platform-admin","displayName":"Platform Admin"},"role":"admin","status":"active"}$json$::jsonb
    ),
    (
        'tmemb_acme',
        'user_ops',
        'tenant_acme',
        $json${"membershipId":"tmemb_acme","tenant":{"id":"tenant_acme","slug":"acme-retail","displayName":"Acme Retail"},"role":"admin","status":"active"}$json$::jsonb
    ),
    (
        'tmemb_northstar',
        'user_ops',
        'tenant_northstar',
        $json${"membershipId":"tmemb_northstar","tenant":{"id":"tenant_northstar","slug":"northstar-labs","displayName":"Northstar Labs"},"role":"member","status":"active"}$json$::jsonb
    ),
    (
        'tmemb_developer_acme',
        'user_developer',
        'tenant_acme',
        $json${"membershipId":"tmemb_developer_acme","tenant":{"id":"tenant_acme","slug":"acme-retail","displayName":"Acme Retail"},"role":"member","status":"active"}$json$::jsonb
    )
ON CONFLICT (membership_id) DO UPDATE
SET user_id = EXCLUDED.user_id,
    tenant_id = EXCLUDED.tenant_id,
    payload = EXCLUDED.payload;

INSERT INTO auth_provider_links (
    link_id,
    user_id,
    provider,
    provider_subject,
    email,
    can_unlink,
    payload
)
VALUES
    (
        'authlink_email_ops',
        'user_ops',
        'email',
        'ops@huge-router.dev',
        'ops@huge-router.dev',
        FALSE,
        $json${"linkId":"authlink_email_ops","provider":"email","providerSubject":"ops@huge-router.dev","email":"ops@huge-router.dev","linkedAt":"2026-04-22T00:00:00Z","lastUsedAt":null,"canUnlink":false}$json$::jsonb
    ),
    (
        'authlink_github_ops',
        'user_ops',
        'github',
        'github_ops',
        'ops@huge-router.dev',
        TRUE,
        $json${"linkId":"authlink_github_ops","provider":"github","providerSubject":"github_ops","email":"ops@huge-router.dev","linkedAt":"2026-04-22T00:00:00Z","lastUsedAt":null,"canUnlink":true}$json$::jsonb
    ),
    (
        'authlink_google_ops',
        'user_ops',
        'google',
        'google_ops',
        'ops@huge-router.dev',
        TRUE,
        $json${"linkId":"authlink_google_ops","provider":"google","providerSubject":"google_ops","email":"ops@huge-router.dev","linkedAt":"2026-04-22T00:00:00Z","lastUsedAt":null,"canUnlink":true}$json$::jsonb
    ),
    (
        'authlink_wechat_ops',
        'user_ops',
        'wechat',
        'wechat_ops',
        'ops@huge-router.dev',
        TRUE,
        $json${"linkId":"authlink_wechat_ops","provider":"wechat","providerSubject":"wechat_ops","email":"ops@huge-router.dev","linkedAt":"2026-04-22T00:00:00Z","lastUsedAt":null,"canUnlink":true}$json$::jsonb
    ),
    (
        'authlink_email_developer',
        'user_developer',
        'email',
        'developer@huge-router.dev',
        'developer@huge-router.dev',
        FALSE,
        $json${"linkId":"authlink_email_developer","provider":"email","providerSubject":"developer@huge-router.dev","email":"developer@huge-router.dev","linkedAt":"2026-04-22T00:00:00Z","lastUsedAt":null,"canUnlink":false}$json$::jsonb
    )
ON CONFLICT (link_id) DO UPDATE
SET user_id = EXCLUDED.user_id,
    provider = EXCLUDED.provider,
    provider_subject = EXCLUDED.provider_subject,
    email = EXCLUDED.email,
    can_unlink = EXCLUDED.can_unlink,
    payload = EXCLUDED.payload;

COMMIT;
