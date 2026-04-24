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

INSERT INTO users (user_id, primary_email, payload)
VALUES
    (
        'user_ops',
        'ops@huge-router.dev',
        $json${"userId":"user_ops","primaryEmail":"ops@huge-router.dev","displayName":"Operations Admin","avatarUrl":null,"createdAt":"2026-04-22T00:00:00Z","lastLoginAt":null}$json$::jsonb
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
        'authlink_ops@huge-router.dev',
        'user_ops',
        'email',
        'ops@huge-router.dev',
        'ops@huge-router.dev',
        FALSE,
        $json${"linkId":"authlink_ops@huge-router.dev","provider":"email","providerSubject":"ops@huge-router.dev","email":"ops@huge-router.dev","linkedAt":"2026-04-22T00:00:00Z","lastUsedAt":null,"canUnlink":false}$json$::jsonb
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
    )
ON CONFLICT (link_id) DO UPDATE
SET user_id = EXCLUDED.user_id,
    provider = EXCLUDED.provider,
    provider_subject = EXCLUDED.provider_subject,
    email = EXCLUDED.email,
    can_unlink = EXCLUDED.can_unlink,
    payload = EXCLUDED.payload;

COMMIT;
