#![allow(
    clippy::assigning_clones,
    clippy::branches_sharing_code,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::format_collect,
    clippy::format_push_string,
    clippy::items_after_test_module,
    clippy::match_like_matches_macro,
    clippy::needless_raw_string_hashes,
    clippy::needless_pass_by_value,
    clippy::option_as_ref_deref,
    clippy::or_fun_call,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::significant_drop_tightening,
    clippy::struct_field_names,
    clippy::unused_async,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::trivially_copy_pass_by_ref
)]

use anyhow::{Context, Result, anyhow};
use core_domain::{
    AdmissionResult, AuthKind, AuthLoginResult, AuthProvider, AuthProviderAvailability,
    AuthProviderLink, AuthSession, AuthSessionId, AuthSessionState, BudgetPolicyId,
    CardDeliveryKind, CardProduct, CardProductId, CardProductStatus, ConfigSnapshot,
    ConfigSnapshotId, ConfigSnapshotStatus, CredentialOwnerType, DeploymentScope, ExcludedTarget,
    HealthState, LogoutResponse, MerchantFulfillmentMode, MerchantShop, MerchantShopId,
    MerchantShopStatus, MonetaryAmount, NormalizedRequestSummary, OAuthProvider, Project,
    ProjectId, ProvenanceClass, ProviderCapabilities, ProviderResource, ProviderResourceId,
    ProviderResourceStatus, RedactionTier, RelayCheckStatus, RelayEvaluation, RelayEvaluationId,
    RelayEvaluationRunnerMode, RelayEvaluationVerdict, ReplayCapsule, ReplayCapsuleId, RoutePolicy,
    RoutePolicyId, RouteReceipt, RouteReceiptId, ScoreBreakdown, Tenant, TenantId,
    TenantMembership, TenantMembershipId, TenantMembershipRole, TenantMembershipStatus,
    TenantSummary, TrialConnection, TrialConnectionId, TrialConnectionStatus,
    UnlinkAuthProviderResponse, UpstreamErrorSummary, UserId, UserIdentity,
};
use metering::{
    AdditionalUsageDimensions, PricingCatalog, PricingSource, default_budget_micros,
    default_catalog, quote_usage_with_additions,
};
use protocol_ir::{
    BalanceProjection, BalanceProjectionResponse, BillingExportJob, BillingExportJobResponse,
    BillingExportJobsResponse, BillingExportRequest, ConfigSnapshotResponse, PricingCatalogEntry,
    PricingCatalogResponse, PricingSimulationLineItem, PricingSimulationRequest,
    PricingSimulationResponse, ProjectsResponse, ProtocolFamily, ProviderResourcesResponse,
    RouteDiagnosticDecision, RouteDiagnosticTarget, RouteDiagnosticsResponse,
    RoutePoliciesResponse, RouteReceiptResponse, RouteReceiptSummary, RouteSimulationRequest,
    RouteSimulationResponse, TenantsResponse, UsageBreakdownResponse, UsageBreakdownRow,
    UsageSummary, UsageSummaryResponse,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres, Row, postgres::PgPoolOptions, types::Json};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const SESSION_TTL_SECONDS: u64 = 60 * 60 * 8;
pub const ACTIVE_CONFIG_ALIAS: &str = "active";

const PROVIDER_CATALOG: &[(AuthProvider, &str, &str)] = &[
    (
        AuthProvider::Email,
        "Continue with Email",
        "/api/control-plane/auth/email/start",
    ),
    (
        AuthProvider::Github,
        "Continue with GitHub",
        "/api/control-plane/auth/oauth/github/start",
    ),
    (
        AuthProvider::Google,
        "Continue with Google",
        "/api/control-plane/auth/oauth/google/start",
    ),
    (
        AuthProvider::Wechat,
        "Continue with WeChat",
        "/api/control-plane/auth/oauth/wechat/start",
    ),
];

const OIDC_PROVIDER_CATALOG_ENTRY: (AuthProvider, &str, &str) = (
    AuthProvider::Oidc,
    "Continue with Enterprise SSO",
    "/api/control-plane/auth/oauth/oidc/start",
);

#[derive(Debug, Clone)]
pub enum StoreMode {
    Memory(Arc<RwLock<MemoryStore>>),
    Postgres(PostgresStore),
}

#[derive(Debug, Clone)]
pub struct PostgresStore {
    pool: Pool<Postgres>,
}

#[derive(Debug, Default)]
pub struct MemoryStore {
    tenants: Vec<Tenant>,
    projects: Vec<Project>,
    provider_resources: Vec<ProviderResource>,
    route_policies: Vec<RoutePolicy>,
    config_snapshots: Vec<ConfigSnapshot>,
    merchant_shops: Vec<MerchantShop>,
    card_products: Vec<CardProduct>,
    trial_connections: Vec<TrialConnection>,
    relay_evaluations: Vec<RelayEvaluation>,
    replay_capsules: HashMap<String, ReplayCapsule>,
    active_config_snapshot_id: String,
    users: HashMap<String, UserIdentity>,
    memberships_by_user: HashMap<String, Vec<TenantMembership>>,
    provider_links_by_user: HashMap<String, Vec<AuthProviderLink>>,
    email_identity_to_user_id: HashMap<String, String>,
    provider_subject_to_user_id: HashMap<String, String>,
    sessions: HashMap<String, StoredSession>,
    login_flows: HashMap<String, LoginFlow>,
    route_receipts: HashMap<String, RouteReceipt>,
    billing_export_jobs: Vec<BillingExportJobRecord>,
    route_policy_disabled_ids: HashSet<String>,
    api_keys: Vec<ApiKeyRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub api_key_id: String,
    pub provider_resource_id: ProviderResourceId,
    pub display_name: String,
    pub key_prefix: String,
    pub hash: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKey {
    pub api_key_id: String,
    pub provider_resource_id: ProviderResourceId,
    pub display_name: String,
    pub key_prefix: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeysResponse {
    pub data: Vec<ApiKey>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteReceiptsResponse {
    pub data: Vec<RouteReceipt>,
}

#[derive(Debug, Clone)]
pub struct BillingExportJobRecord {
    pub job: BillingExportJob,
    pub content: Option<String>,
    pub content_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageBreakdownGroupBy {
    Provider,
    Model,
    Day,
}

#[derive(Debug, Clone)]
pub struct ResolvedApiKey {
    pub api_key_id: String,
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSnapshotsResponse {
    pub data: Vec<ConfigSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantWorkspaceResponse {
    pub merchant_enabled: bool,
    pub tenant_id: TenantId,
    pub shops: Vec<MerchantShop>,
    pub card_products: Vec<CardProduct>,
    pub trial_connections: Vec<TrialConnection>,
    pub recent_evaluations: Vec<RelayEvaluation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantWorkspaceEnvelope {
    pub data: MerchantWorkspaceResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayCapsuleResponse {
    pub replay_capsule: ReplayCapsule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrencyResult<T> {
    Applied(T),
    NotFound,
    VersionConflict,
}

impl ApiKeyRecord {
    fn public_view(&self) -> ApiKey {
        ApiKey {
            api_key_id: self.api_key_id.clone(),
            provider_resource_id: self.provider_resource_id.clone(),
            display_name: self.display_name.clone(),
            key_prefix: self.key_prefix.clone(),
            is_active: self.is_active,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        }
    }

    fn to_resolved(&self, provider_resources: &[ProviderResource]) -> Option<ResolvedApiKey> {
        if !self.is_active {
            return None;
        }
        let provider_resource = provider_resources
            .iter()
            .find(|resource| resource.provider_resource_id == self.provider_resource_id)?;
        Some(ResolvedApiKey {
            api_key_id: self.api_key_id.clone(),
            tenant_id: provider_resource.tenant_id.clone(),
            project_id: provider_resource.project_id.clone(),
            is_active: true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFlow {
    pub flow_id: String,
    pub flow_kind: LoginFlowKind,
    pub email: Option<String>,
    pub provider: Option<OAuthProvider>,
    pub workspace_slug: String,
    pub verification_code: Option<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoginFlowKind {
    Email,
    OAuth,
}

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub session: AuthSession,
    pub links: Vec<AuthProviderLink>,
}

#[derive(Debug, Clone)]
pub struct SeedData {
    pub tenants: Vec<Tenant>,
    pub projects: Vec<Project>,
    pub provider_resources: Vec<ProviderResource>,
    pub route_policies: Vec<RoutePolicy>,
    pub config_snapshots: Vec<ConfigSnapshot>,
    pub merchant_shops: Vec<MerchantShop>,
    pub card_products: Vec<CardProduct>,
    pub trial_connections: Vec<TrialConnection>,
    pub relay_evaluations: Vec<RelayEvaluation>,
    pub replay_capsules: Vec<ReplayCapsule>,
    pub route_receipts: Vec<RouteReceipt>,
    pub active_config_snapshot_id: String,
    pub users: Vec<UserSeed>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProviderResourceFilters {
    pub tenant_id: Option<String>,
    pub health_state: Option<String>,
    pub protocol_family: Option<String>,
    pub capability: Option<String>,
    pub transit_gateway: Option<bool>,
    pub quarantined: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RouteReceiptFilters {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
    pub protocol_family: Option<String>,
    pub route_policy_id: Option<String>,
    pub provider_resource_id: Option<String>,
    pub admission_result: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct UserSeed {
    pub user: UserIdentity,
    pub identities: Vec<UserIdentityKey>,
    pub memberships: Vec<TenantMembership>,
    pub links: Vec<AuthProviderLink>,
}

#[derive(Debug, Clone)]
pub enum UserIdentityKey {
    Email(String),
    ProviderSubject(AuthProvider, String),
}

impl SeedData {
    #[allow(clippy::too_many_lines)]
    pub fn bootstrap() -> Self {
        let now = "2026-04-22T00:00:00Z".to_string();
        let tenant_platform = Tenant::new(
            TenantId::parse("tenant_platform").unwrap(),
            "platform-admin",
            "Platform Admin",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let tenant_acme = Tenant::new(
            TenantId::parse("tenant_acme").unwrap(),
            "acme-retail",
            "Acme Retail",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let tenant_northstar = Tenant::new(
            TenantId::parse("tenant_northstar").unwrap(),
            "northstar-labs",
            "Northstar Labs",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();

        let proj_core = Project::new(
            ProjectId::parse("proj_core").unwrap(),
            tenant_acme.tenant_id.clone(),
            "core-gateway",
            "Core Gateway",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let proj_ops = Project::new(
            ProjectId::parse("proj_acme_ops").unwrap(),
            tenant_acme.tenant_id.clone(),
            "retail-ops",
            "Retail Operations Control",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let proj_support = Project::new(
            ProjectId::parse("proj_acme_support").unwrap(),
            tenant_acme.tenant_id.clone(),
            "store-support",
            "Store Support Agent",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let proj_research = Project::new(
            ProjectId::parse("proj_ns_research").unwrap(),
            tenant_northstar.tenant_id.clone(),
            "research-qa",
            "Research QA",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();

        let openai_primary = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: Some(proj_core.project_id.clone()),
            provider_id: "openai".to_string(),
            name: "OpenAI Primary".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Platform,
            deployment_scope: DeploymentScope::Shared,
            region: "us-east-1".to_string(),
            endpoint_base_url: "https://api.openai.com/v1".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Healthy,
            health_message: Some("probe latency within SLO".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec![
                "openai_chat".to_string(),
                "openai_responses".to_string(),
            ],
            is_transit_gateway: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let openai_backup = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_backup").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: Some(proj_core.project_id.clone()),
            provider_id: "openai".to_string(),
            name: "OpenAI Backup".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Platform,
            deployment_scope: DeploymentScope::Shared,
            region: "us-west-2".to_string(),
            endpoint_base_url: "https://api.openai.com/v1".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Healthy,
            health_message: Some("backup target healthy".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec![
                "openai_chat".to_string(),
                "openai_responses".to_string(),
            ],
            is_transit_gateway: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let northstar_openai = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_research").unwrap(),
            tenant_id: tenant_northstar.tenant_id.clone(),
            project_id: Some(proj_research.project_id.clone()),
            provider_id: "openai".to_string(),
            name: "OpenAI Research".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Tenant,
            deployment_scope: DeploymentScope::TenantDedicated,
            region: "us-west-2".to_string(),
            endpoint_base_url: "https://api.openai.com/v1".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Degraded,
            health_message: Some("research endpoint latency elevated".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_json_mode: true,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec!["openai_chat".to_string()],
            is_transit_gateway: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let route_default = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_openai_chat_default").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            display_name: "Acme Reasoning Fast".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-east-1".to_string()],
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let route_support = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_acme_support").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            display_name: "Acme Support Safe".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "support-safe".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-west-2".to_string()],
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let route_research = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_northstar_research").unwrap(),
            tenant_id: tenant_northstar.tenant_id.clone(),
            display_name: "Northstar Research".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "research-fast".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-west-2".to_string()],
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let config_active = ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_gateway_v1").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: proj_core.project_id.clone(),
            revision: 1,
            status: ConfigSnapshotStatus::Active,
            activated_at: Some(now.clone()),
            provider_resource_ids: vec![
                openai_primary.provider_resource_id.clone(),
                openai_backup.provider_resource_id.clone(),
            ],
            route_policy_id: route_default.route_policy_id.clone(),
            budget_policy_id: BudgetPolicyId::parse("budgetpol_default").unwrap(),
        };
        let config_research = ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_research_v1").unwrap(),
            tenant_id: tenant_northstar.tenant_id.clone(),
            project_id: proj_research.project_id.clone(),
            revision: 1,
            status: ConfigSnapshotStatus::Draft,
            activated_at: None,
            provider_resource_ids: vec![northstar_openai.provider_resource_id.clone()],
            route_policy_id: route_research.route_policy_id.clone(),
            budget_policy_id: BudgetPolicyId::parse("budgetpol_default").unwrap(),
        };

        let merchant_shop = MerchantShop {
            merchant_shop_id: MerchantShopId::parse("mshop_acme").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            slug: "acme-small-shop".to_string(),
            display_name: "Acme Small Shop".to_string(),
            status: MerchantShopStatus::Active,
            announcement: Some(
                "Fresh relay trial cards with replay-backed evaluation.".to_string(),
            ),
            fulfillment_mode: MerchantFulfillmentMode::AutoCardSecret,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let card_product = CardProduct {
            card_product_id: CardProductId::parse("cardprod_acme_trial").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            merchant_shop_id: merchant_shop.merchant_shop_id.clone(),
            title: "Claude Trial Pack".to_string(),
            description: "Starter batch for relay verification and low-risk onboarding."
                .to_string(),
            status: CardProductStatus::Active,
            inventory_count: 32,
            face_value_usd: "1.00".to_string(),
            retail_price_usd: "1.99".to_string(),
            delivery_kind: CardDeliveryKind::DirectSecret,
            supports_trial: true,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let trial_connection = TrialConnection {
            trial_connection_id: TrialConnectionId::parse("trialconn_acme_relay").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            provider_label: "Acme Relay".to_string(),
            endpoint_base_url: "https://relay.acme.example/v1".to_string(),
            api_key_masked: "sk-trial...acme".to_string(),
            target_model: "claude-sonnet".to_string(),
            status: TrialConnectionStatus::Active,
            notes: Some("Dedicated trial key only; never attach production traffic.".to_string()),
            last_verified_at: Some(now.clone()),
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let replay_capsule = ReplayCapsule {
            replay_capsule_id: ReplayCapsuleId::parse("replay_acme_relay_eval").unwrap(),
            request_id: "req_merchant_eval_acme".to_string(),
            trace_id: "trace_merchant_eval_acme".to_string(),
            route_receipt_id: RouteReceiptId::parse("routercpt_acme_relay_eval").unwrap(),
            config_snapshot_id: config_active.config_snapshot_id.clone(),
            redaction_tier: RedactionTier::StructuredRedacted,
            normalized_request_summary: NormalizedRequestSummary {
                protocol_family: "openai_chat".to_string(),
                model_alias: "claude-sonnet".to_string(),
                estimated_prompt_tokens: 480,
            },
            upstream_error_summary: Some(UpstreamErrorSummary {
                code: "provider_signature_mismatch".to_string(),
            }),
        };
        let relay_evaluation = RelayEvaluation {
            relay_evaluation_id: RelayEvaluationId::parse("reval_acme_relay").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            trial_connection_id: trial_connection.trial_connection_id.clone(),
            replay_capsule_id: replay_capsule.replay_capsule_id.clone(),
            provider_label: trial_connection.provider_label.clone(),
            endpoint_base_url: trial_connection.endpoint_base_url.clone(),
            target_model: trial_connection.target_model.clone(),
            runner_mode: RelayEvaluationRunnerMode::Simulated,
            sample_request_count: 5,
            estimated_tokens_saved: 2400,
            overall_score: 82,
            verdict: RelayEvaluationVerdict::Warning,
            fingerprint_status: RelayCheckStatus::Pass,
            protocol_status: RelayCheckStatus::Warning,
            token_status: RelayCheckStatus::Warning,
            multimodal_status: RelayCheckStatus::NotTested,
            detected_channel: Some("vertex".to_string()),
            summary:
                "Replay capsule captured; protocol and token behavior still need manual follow-up."
                    .to_string(),
            created_at: now.clone(),
        };

        let ops_user = UserIdentity {
            user_id: UserId::parse("user_ops").unwrap(),
            primary_email: Some("ops@huge-router.dev".to_string()),
            display_name: "Operations Admin".to_string(),
            avatar_url: None,
            created_at: now,
            last_login_at: None,
        };

        let memberships = vec![
            membership(
                "tmemb_platform",
                &tenant_platform,
                TenantMembershipRole::Admin,
            ),
            membership("tmemb_acme", &tenant_acme, TenantMembershipRole::Admin),
            membership(
                "tmemb_northstar",
                &tenant_northstar,
                TenantMembershipRole::Member,
            ),
        ];
        let links = vec![
            link(
                AuthProvider::Email,
                "ops@huge-router.dev",
                Some("ops@huge-router.dev"),
                false,
            ),
            link(
                AuthProvider::Github,
                "github_ops",
                Some("ops@huge-router.dev"),
                true,
            ),
            link(
                AuthProvider::Google,
                "google_ops",
                Some("ops@huge-router.dev"),
                true,
            ),
            link(
                AuthProvider::Wechat,
                "wechat_ops",
                Some("ops@huge-router.dev"),
                true,
            ),
        ];

        Self {
            tenants: vec![tenant_platform, tenant_acme, tenant_northstar],
            projects: vec![proj_core, proj_ops, proj_support, proj_research],
            provider_resources: vec![openai_primary, openai_backup, northstar_openai],
            route_policies: vec![route_default, route_support, route_research],
            config_snapshots: vec![config_active.clone(), config_research],
            merchant_shops: vec![merchant_shop],
            card_products: vec![card_product],
            trial_connections: vec![trial_connection],
            relay_evaluations: vec![relay_evaluation],
            replay_capsules: vec![replay_capsule],
            route_receipts: Vec::new(),
            active_config_snapshot_id: config_active.config_snapshot_id.as_str().to_string(),
            users: vec![UserSeed {
                user: ops_user,
                identities: vec![
                    UserIdentityKey::Email("ops@huge-router.dev".to_string()),
                    UserIdentityKey::ProviderSubject(
                        AuthProvider::Github,
                        "github_ops".to_string(),
                    ),
                    UserIdentityKey::ProviderSubject(
                        AuthProvider::Google,
                        "google_ops".to_string(),
                    ),
                    UserIdentityKey::ProviderSubject(
                        AuthProvider::Wechat,
                        "wechat_ops".to_string(),
                    ),
                ],
                memberships,
                links,
            }],
        }
    }
}

impl MemoryStore {
    pub fn bootstrap() -> Self {
        Self::from_seed(SeedData::bootstrap())
    }

    pub(crate) fn from_seed(seed: SeedData) -> Self {
        let mut users = HashMap::new();
        let mut memberships_by_user = HashMap::new();
        let mut provider_links_by_user = HashMap::new();
        let mut email_identity_to_user_id = HashMap::new();
        let mut provider_subject_to_user_id = HashMap::new();

        for entry in seed.users {
            let user_id = entry.user.user_id.as_str().to_string();
            memberships_by_user.insert(user_id.clone(), entry.memberships);
            provider_links_by_user.insert(user_id.clone(), entry.links);
            for identity in entry.identities {
                match identity {
                    UserIdentityKey::Email(email) => {
                        email_identity_to_user_id.insert(email.to_lowercase(), user_id.clone());
                    }
                    UserIdentityKey::ProviderSubject(provider, subject) => {
                        provider_subject_to_user_id.insert(
                            format!("{}:{}", auth_provider_slug(provider), subject),
                            user_id.clone(),
                        );
                    }
                }
            }
            users.insert(user_id, entry.user);
        }

        Self {
            tenants: seed.tenants,
            projects: seed.projects,
            provider_resources: seed.provider_resources,
            route_policies: seed.route_policies,
            config_snapshots: seed.config_snapshots,
            merchant_shops: seed.merchant_shops,
            card_products: seed.card_products,
            trial_connections: seed.trial_connections,
            relay_evaluations: seed.relay_evaluations,
            replay_capsules: seed
                .replay_capsules
                .into_iter()
                .map(|capsule| (capsule.replay_capsule_id.as_str().to_string(), capsule))
                .collect(),
            active_config_snapshot_id: seed.active_config_snapshot_id,
            users,
            memberships_by_user,
            provider_links_by_user,
            email_identity_to_user_id,
            provider_subject_to_user_id,
            sessions: HashMap::new(),
            login_flows: HashMap::new(),
            route_receipts: seed
                .route_receipts
                .into_iter()
                .map(|receipt| (receipt.route_receipt_id.as_str().to_string(), receipt))
                .collect(),
            billing_export_jobs: Vec::new(),
            route_policy_disabled_ids: HashSet::new(),
            api_keys: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_route_receipt(&mut self, route_receipt: RouteReceipt) {
        self.route_receipts.insert(
            route_receipt.route_receipt_id.as_str().to_string(),
            route_receipt,
        );
    }
}

impl StoreMode {
    pub fn memory() -> Self {
        Self::Memory(Arc::new(RwLock::new(MemoryStore::bootstrap())))
    }

    pub async fn from_env() -> Result<Self> {
        let store_mode =
            std::env::var("CONTROL_PLANE_STORE_MODE").unwrap_or_else(|_| "postgres".to_string());

        match store_mode.as_str() {
            "memory" => Ok(Self::memory()),
            "postgres" => {
                let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL").map_err(|_| {
                    anyhow!(
                        "CONTROL_PLANE_DATABASE_URL is required when CONTROL_PLANE_STORE_MODE=postgres"
                    )
                })?;
                let store = PostgresStore::connect(&database_url).await?;
                store.ensure_schema_ready().await?;
                Ok(Self::Postgres(store))
            }
            other => Err(anyhow!(
                "unsupported CONTROL_PLANE_STORE_MODE `{other}`; expected `postgres` or `memory`"
            )),
        }
    }

    pub fn provider_catalog() -> Vec<AuthProviderAvailability> {
        let mut catalog = PROVIDER_CATALOG
            .iter()
            .map(|(provider, display_name, start_path)| {
                let enabled = auth_provider_enabled(*provider);
                AuthProviderAvailability {
                    provider: *provider,
                    display_name: (*display_name).to_string(),
                    enabled,
                    start_path: (*start_path).to_string(),
                    reason_code: if enabled {
                        None
                    } else {
                        Some("provider_not_configured".to_string())
                    },
                }
            })
            .collect::<Vec<_>>();

        let oidc_available = oidc_enabled() || mock_auth_enabled();
        catalog.push(AuthProviderAvailability {
            provider: OIDC_PROVIDER_CATALOG_ENTRY.0,
            display_name: OIDC_PROVIDER_CATALOG_ENTRY.1.to_string(),
            enabled: oidc_available,
            start_path: OIDC_PROVIDER_CATALOG_ENTRY.2.to_string(),
            reason_code: if oidc_available {
                None
            } else {
                Some("provider_not_configured".to_string())
            },
        });

        catalog
    }

    pub async fn ensure_known_email(&self, email: &str) -> Result<bool> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .email_identity_to_user_id
                .contains_key(&email.to_lowercase())),
            Self::Postgres(store) => store.ensure_known_email(email).await,
        }
    }

    pub async fn create_email_flow(
        &self,
        flow_id: &str,
        email: &str,
        workspace_slug: &str,
        verification_code: &str,
        expires_at: &str,
    ) -> Result<LoginFlow> {
        let flow = LoginFlow {
            flow_id: flow_id.to_string(),
            flow_kind: LoginFlowKind::Email,
            email: Some(email.to_lowercase()),
            provider: None,
            workspace_slug: workspace_slug.to_string(),
            verification_code: Some(verification_code.to_string()),
            expires_at: expires_at.to_string(),
        };
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .login_flows
                    .insert(flow_id.to_string(), flow.clone());
                Ok(flow)
            }
            Self::Postgres(store) => store.create_login_flow(&flow).await,
        }
    }

    pub async fn create_oauth_flow(
        &self,
        flow_id: &str,
        provider: OAuthProvider,
        workspace_slug: &str,
        expires_at: &str,
    ) -> Result<LoginFlow> {
        let flow = LoginFlow {
            flow_id: flow_id.to_string(),
            flow_kind: LoginFlowKind::OAuth,
            email: None,
            provider: Some(provider),
            workspace_slug: workspace_slug.to_string(),
            verification_code: None,
            expires_at: expires_at.to_string(),
        };
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .login_flows
                    .insert(flow_id.to_string(), flow.clone());
                Ok(flow)
            }
            Self::Postgres(store) => store.create_login_flow(&flow).await,
        }
    }

    pub async fn consume_login_flow(&self, flow_id: &str) -> Result<Option<LoginFlow>> {
        let pending = match self {
            Self::Memory(store) => store
                .write()
                .expect("memory store write lock")
                .login_flows
                .remove(flow_id),
            Self::Postgres(store) => store.consume_login_flow(flow_id).await?,
        };

        Ok(pending.filter(|flow| !timestamp_is_expired(&flow.expires_at)))
    }

    pub async fn workspace_exists(&self, workspace_slug: &str) -> Result<bool> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .tenants
                .iter()
                .any(|tenant| tenant.slug == workspace_slug)),
            Self::Postgres(store) => store.workspace_exists(workspace_slug).await,
        }
    }

    pub async fn issue_session(
        &self,
        session_id: &str,
        provider: AuthProvider,
        identity_key: &IdentityLookup,
        workspace_slug: &str,
        now: &str,
        expires_at: &str,
    ) -> Result<AuthLoginResult> {
        match self {
            Self::Memory(store) => issue_memory_session(
                &mut store.write().expect("memory store write lock"),
                session_id,
                provider,
                identity_key,
                workspace_slug,
                now,
                expires_at,
            ),
            Self::Postgres(store) => {
                store
                    .issue_session(
                        session_id,
                        provider,
                        identity_key,
                        workspace_slug,
                        now,
                        expires_at,
                    )
                    .await
            }
        }
    }

    pub async fn upsert_oidc_user(
        &self,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let user_id = upsert_memory_oidc_user(
                    &mut store,
                    subject,
                    email,
                    display_name,
                    workspace_slug,
                    role,
                    now,
                )?;
                store
                    .users
                    .get(&user_id)
                    .cloned()
                    .context("oidc user should exist after upsert")
            }
            Self::Postgres(store) => {
                store
                    .upsert_oidc_user(subject, email, display_name, workspace_slug, role, now)
                    .await
            }
        }
    }

    pub async fn upsert_oauth_user(
        &self,
        provider: AuthProvider,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let user_id = upsert_memory_provider_user(
                    &mut store,
                    provider,
                    subject,
                    email,
                    display_name,
                    workspace_slug,
                    role,
                    now,
                )?;
                store
                    .users
                    .get(&user_id)
                    .cloned()
                    .context("oauth user should exist after upsert")
            }
            Self::Postgres(store) => {
                store
                    .upsert_oauth_user(
                        provider,
                        subject,
                        email,
                        display_name,
                        workspace_slug,
                        role,
                        now,
                    )
                    .await
            }
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<AuthLoginResult>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let Some(stored) = store.sessions.get(session_id).cloned() else {
                    return Ok(None);
                };
                if timestamp_is_expired(&stored.session.expires_at) {
                    store.sessions.remove(session_id);
                    return Ok(None);
                }
                Ok(Some(refresh_memory_session(&store, stored)))
            }
            Self::Postgres(store) => store.get_session(session_id).await,
        }
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<LogoutResponse> {
        match self {
            Self::Memory(store) => {
                let removed = store
                    .write()
                    .expect("memory store write lock")
                    .sessions
                    .remove(session_id)
                    .is_some();
                Ok(LogoutResponse {
                    session_id: AuthSessionId::parse(session_id.to_string()).unwrap(),
                    revoked: removed,
                })
            }
            Self::Postgres(store) => store.revoke_session(session_id).await,
        }
    }

    pub async fn unlink_provider(
        &self,
        session_id: &str,
        provider: AuthProvider,
    ) -> Result<Option<UnlinkAuthProviderResponse>> {
        match self {
            Self::Memory(store) => Ok(unlink_memory_provider(
                &mut store.write().expect("memory store write lock"),
                session_id,
                provider,
            )),
            Self::Postgres(store) => store.unlink_provider(session_id, provider).await,
        }
    }

    pub async fn list_tenants(&self) -> Result<TenantsResponse> {
        match self {
            Self::Memory(store) => Ok(TenantsResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .tenants
                    .clone(),
            }),
            Self::Postgres(store) => store.list_tenants().await,
        }
    }

    pub async fn list_projects(&self) -> Result<ProjectsResponse> {
        match self {
            Self::Memory(store) => Ok(ProjectsResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .projects
                    .clone(),
            }),
            Self::Postgres(store) => store.list_projects().await,
        }
    }

    pub async fn get_merchant_workspace(
        &self,
        tenant_id: &TenantId,
    ) -> Result<MerchantWorkspaceEnvelope> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let tenant_id_str = tenant_id.as_str();
                let mut recent_evaluations = store
                    .relay_evaluations
                    .iter()
                    .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                    .cloned()
                    .collect::<Vec<_>>();
                recent_evaluations.sort_by(|left, right| right.created_at.cmp(&left.created_at));

                Ok(MerchantWorkspaceEnvelope {
                    data: MerchantWorkspaceResponse {
                        merchant_enabled: store
                            .merchant_shops
                            .iter()
                            .any(|shop| shop.tenant_id.as_str() == tenant_id_str),
                        tenant_id: tenant_id.clone(),
                        shops: store
                            .merchant_shops
                            .iter()
                            .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                            .cloned()
                            .collect(),
                        card_products: store
                            .card_products
                            .iter()
                            .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                            .cloned()
                            .collect(),
                        trial_connections: store
                            .trial_connections
                            .iter()
                            .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                            .cloned()
                            .collect(),
                        recent_evaluations,
                    },
                })
            }
            Self::Postgres(_) => Err(anyhow!(
                "merchant workspace persistence is not yet implemented for postgres mode"
            )),
        }
    }

    pub async fn create_merchant_shop(&self, mut shop: MerchantShop) -> Result<MerchantShop> {
        shop.version = 1;
        shop.created_at = now_rfc3339();
        shop.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                shop.validate()?;
                store
                    .write()
                    .expect("memory store write lock")
                    .merchant_shops
                    .push(shop.clone());
                Ok(shop)
            }
            Self::Postgres(_) => Err(anyhow!(
                "merchant shop persistence is not yet implemented for postgres mode"
            )),
        }
    }

    pub async fn create_card_product(&self, mut product: CardProduct) -> Result<CardProduct> {
        product.version = 1;
        product.created_at = now_rfc3339();
        product.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                if !store.merchant_shops.iter().any(|shop| {
                    shop.merchant_shop_id == product.merchant_shop_id
                        && shop.tenant_id == product.tenant_id
                }) {
                    return Err(anyhow!("merchant shop not found for tenant"));
                }
                product.validate()?;
                store.card_products.push(product.clone());
                Ok(product)
            }
            Self::Postgres(_) => Err(anyhow!(
                "card product persistence is not yet implemented for postgres mode"
            )),
        }
    }

    pub async fn create_trial_connection(
        &self,
        mut connection: TrialConnection,
    ) -> Result<TrialConnection> {
        connection.version = 1;
        connection.created_at = now_rfc3339();
        connection.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                connection.validate()?;
                store
                    .write()
                    .expect("memory store write lock")
                    .trial_connections
                    .push(connection.clone());
                Ok(connection)
            }
            Self::Postgres(_) => Err(anyhow!(
                "trial connection persistence is not yet implemented for postgres mode"
            )),
        }
    }

    pub async fn create_relay_evaluation(
        &self,
        tenant_id: &TenantId,
        trial_connection_id: &str,
    ) -> Result<RelayEvaluation> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let connection = store
                    .trial_connections
                    .iter()
                    .find(|item| {
                        item.tenant_id == *tenant_id
                            && item.trial_connection_id.as_str() == trial_connection_id
                    })
                    .cloned()
                    .context("trial connection not found")?;

                let created_at = now_rfc3339();
                let replay_capsule_id =
                    ReplayCapsuleId::parse(format!("replay_{}", next_id_suffix())).unwrap();
                let route_receipt_id =
                    RouteReceiptId::parse(format!("routercpt_{}", next_id_suffix())).unwrap();

                let replay_capsule = ReplayCapsule {
                    replay_capsule_id: replay_capsule_id.clone(),
                    request_id: format!("req_{}", next_id_suffix()),
                    trace_id: format!("trace_{}", next_id_suffix()),
                    route_receipt_id,
                    config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_merchant_eval").unwrap(),
                    redaction_tier: RedactionTier::StructuredRedacted,
                    normalized_request_summary: NormalizedRequestSummary {
                        protocol_family: "openai_chat".to_string(),
                        model_alias: connection.target_model.clone(),
                        estimated_prompt_tokens: 480,
                    },
                    upstream_error_summary: Some(UpstreamErrorSummary {
                        code: replay_upstream_error_code(&connection),
                    }),
                };

                let evaluation =
                    build_relay_evaluation(tenant_id, &connection, replay_capsule_id, created_at);
                evaluation.validate()?;

                store.replay_capsules.insert(
                    replay_capsule.replay_capsule_id.as_str().to_string(),
                    replay_capsule,
                );
                store.relay_evaluations.push(evaluation.clone());
                Ok(evaluation)
            }
            Self::Postgres(_) => Err(anyhow!(
                "relay evaluation persistence is not yet implemented for postgres mode"
            )),
        }
    }

    pub async fn get_replay_capsule(
        &self,
        replay_capsule_id: &str,
    ) -> Result<Option<ReplayCapsuleResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .replay_capsules
                .get(replay_capsule_id)
                .cloned()
                .map(|replay_capsule| ReplayCapsuleResponse { replay_capsule })),
            Self::Postgres(_) => Err(anyhow!(
                "replay capsule persistence is not yet implemented for postgres mode"
            )),
        }
    }

    pub async fn get_project(&self, project_id: &str) -> Result<Option<Project>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .projects
                .iter()
                .find(|item| item.project_id.as_str() == project_id)
                .cloned()),
            Self::Postgres(store) => store.get_project(project_id).await,
        }
    }

    pub async fn list_provider_resources(
        &self,
        filters: &ProviderResourceFilters,
    ) -> Result<ProviderResourcesResponse> {
        match self {
            Self::Memory(store) => Ok(ProviderResourcesResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .provider_resources
                    .iter()
                    .filter(|resource| provider_matches_filters(resource, filters))
                    .cloned()
                    .collect(),
            }),
            Self::Postgres(store) => store.list_provider_resources(filters).await,
        }
    }

    pub async fn get_provider_resource(
        &self,
        provider_resource_id: &str,
    ) -> Result<Option<ProviderResource>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .provider_resources
                .iter()
                .find(|item| item.provider_resource_id.as_str() == provider_resource_id)
                .cloned()),
            Self::Postgres(store) => store.get_provider_resource(provider_resource_id).await,
        }
    }

    pub async fn create_provider_resource(
        &self,
        mut provider_resource: ProviderResource,
    ) -> Result<ProviderResource> {
        provider_resource.version = 1;
        provider_resource.created_at = now_rfc3339();
        provider_resource.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .provider_resources
                    .push(provider_resource.clone());
                Ok(provider_resource)
            }
            Self::Postgres(store) => store.create_provider_resource(&provider_resource).await,
        }
    }

    pub async fn update_provider_resource(
        &self,
        provider_resource: ProviderResource,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        let mut provider_resource = provider_resource;
        provider_resource.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => Ok(update_memory_provider_resource(
                &mut store.write().expect("memory store write lock"),
                &provider_resource,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .update_provider_resource(&provider_resource, expected_version)
                    .await
            }
        }
    }

    pub async fn disable_provider_resource(
        &self,
        provider_resource_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        match self {
            Self::Memory(store) => Ok(disable_memory_provider_resource(
                &mut store.write().expect("memory store write lock"),
                provider_resource_id,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .disable_provider_resource(provider_resource_id, expected_version)
                    .await
            }
        }
    }

    pub async fn list_route_policies(&self) -> Result<RoutePoliciesResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(RoutePoliciesResponse {
                    data: store
                        .route_policies
                        .iter()
                        .filter(|policy| {
                            !store
                                .route_policy_disabled_ids
                                .contains(policy.route_policy_id.as_str())
                        })
                        .cloned()
                        .collect(),
                })
            }
            Self::Postgres(store) => store.list_route_policies().await,
        }
    }

    #[allow(dead_code)]
    pub async fn get_route_policy(&self, route_policy_id: &str) -> Result<Option<RoutePolicy>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .route_policies
                .iter()
                .find(|item| item.route_policy_id.as_str() == route_policy_id)
                .cloned()),
            Self::Postgres(store) => store.get_route_policy(route_policy_id).await,
        }
    }

    pub async fn create_route_policy(&self, mut route_policy: RoutePolicy) -> Result<RoutePolicy> {
        route_policy.version = 1;
        route_policy.created_at = now_rfc3339();
        route_policy.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .route_policies
                    .push(route_policy.clone());
                Ok(route_policy)
            }
            Self::Postgres(store) => store.create_route_policy(&route_policy).await,
        }
    }

    pub async fn update_route_policy(
        &self,
        route_policy: RoutePolicy,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        let mut route_policy = route_policy;
        route_policy.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => Ok(update_memory_route_policy(
                &mut store.write().expect("memory store write lock"),
                &route_policy,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .update_route_policy(&route_policy, expected_version)
                    .await
            }
        }
    }

    pub async fn disable_route_policy(
        &self,
        route_policy_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        match self {
            Self::Memory(store) => Ok(disable_memory_route_policy(
                &mut store.write().expect("memory store write lock"),
                route_policy_id,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .disable_route_policy(route_policy_id, expected_version)
                    .await
            }
        }
    }

    pub async fn list_config_snapshots(&self) -> Result<ConfigSnapshotsResponse> {
        match self {
            Self::Memory(store) => Ok(ConfigSnapshotsResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .config_snapshots
                    .clone(),
            }),
            Self::Postgres(store) => store.list_config_snapshots().await,
        }
    }

    pub async fn create_config_snapshot(
        &self,
        mut config_snapshot: ConfigSnapshot,
    ) -> Result<ConfigSnapshot> {
        config_snapshot.activated_at = None;
        config_snapshot.status = ConfigSnapshotStatus::Draft;
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .config_snapshots
                    .push(config_snapshot.clone());
                Ok(config_snapshot)
            }
            Self::Postgres(store) => store.create_config_snapshot(&config_snapshot).await,
        }
    }

    pub async fn create_api_key(
        &self,
        provider_resource_id: ProviderResourceId,
        display_name: &str,
        api_key: &str,
    ) -> Result<ApiKey> {
        let now = now_rfc3339();
        let prefix = api_key_prefix(api_key);
        let hash = hash_api_key(api_key);
        match self {
            Self::Memory(store) => Ok(insert_memory_api_key(
                &mut store.write().expect("memory store write lock"),
                provider_resource_id,
                display_name,
                &prefix,
                hash,
                now,
            )),
            Self::Postgres(store) => {
                store
                    .create_api_key(provider_resource_id.as_str(), display_name, &prefix, &hash)
                    .await
            }
        }
    }

    pub async fn list_api_keys(&self) -> Result<ApiKeysResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(ApiKeysResponse {
                    data: store
                        .api_keys
                        .iter()
                        .map(ApiKeyRecord::public_view)
                        .collect(),
                })
            }
            Self::Postgres(store) => store.list_api_keys().await,
        }
    }

    pub async fn revoke_api_key(
        &self,
        api_key_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ApiKey>> {
        match self {
            Self::Memory(store) => Ok(revoke_memory_api_key(
                &mut store.write().expect("memory store write lock"),
                api_key_id,
                expected_version,
            )),
            Self::Postgres(store) => store.revoke_api_key(api_key_id, expected_version).await,
        }
    }

    pub async fn resolve_api_key(&self, api_key: &str) -> Result<Option<ResolvedApiKey>> {
        let hash = hash_api_key(api_key);
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .api_keys
                    .iter()
                    .find(|item| item.hash == hash && item.is_active)
                    .and_then(|item| item.to_resolved(&store.provider_resources)))
            }
            Self::Postgres(store) => store.resolve_api_key(&hash).await,
        }
    }

    pub async fn get_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let snapshot_id = if config_snapshot_id == ACTIVE_CONFIG_ALIAS {
                    store.active_config_snapshot_id.as_str()
                } else {
                    config_snapshot_id
                };
                Ok(store
                    .config_snapshots
                    .iter()
                    .find(|item| item.config_snapshot_id.as_str() == snapshot_id)
                    .cloned()
                    .map(|config_snapshot| ConfigSnapshotResponse { config_snapshot }))
            }
            Self::Postgres(store) => store.get_config_snapshot(config_snapshot_id).await,
        }
    }

    pub async fn activate_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        match self {
            Self::Memory(store) => Ok(activate_memory_config_snapshot(
                &mut store.write().expect("memory store write lock"),
                config_snapshot_id,
            )),
            Self::Postgres(store) => store.activate_config_snapshot(config_snapshot_id).await,
        }
    }

    pub async fn simulate_route(
        &self,
        request: RouteSimulationRequest,
    ) -> Result<RouteSimulationResponse> {
        match self {
            Self::Memory(store) => {
                simulate_memory_route(&store.read().expect("memory store read lock"), &request)
            }
            Self::Postgres(store) => store.simulate_route(request).await,
        }
    }

    pub async fn get_route_receipt(
        &self,
        route_receipt_id: &str,
    ) -> Result<Option<RouteReceiptResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .route_receipts
                .get(route_receipt_id)
                .cloned()
                .map(|route_receipt| RouteReceiptResponse { route_receipt })),
            Self::Postgres(store) => store.get_route_receipt(route_receipt_id).await,
        }
    }

    pub async fn list_route_receipts(
        &self,
        filters: &RouteReceiptFilters,
    ) -> Result<RouteReceiptsResponse> {
        match self {
            Self::Memory(store) => {
                let mut receipts = store
                    .read()
                    .expect("memory store read lock")
                    .route_receipts
                    .values()
                    .filter(|receipt| route_receipt_matches_filters(receipt, filters))
                    .cloned()
                    .collect::<Vec<_>>();
                receipts.sort_by(|left, right| right.created_at.cmp(&left.created_at));
                if let Some(limit) = filters.limit {
                    receipts.truncate(limit);
                }
                Ok(RouteReceiptsResponse { data: receipts })
            }
            Self::Postgres(store) => store.list_route_receipts(filters).await,
        }
    }

    pub async fn get_route_diagnostics(
        &self,
        route_policy_id: &str,
    ) -> Result<Option<RouteDiagnosticsResponse>> {
        match self {
            Self::Memory(store) => Ok(build_memory_route_diagnostics(
                &store.read().expect("memory store read lock"),
                route_policy_id,
            )),
            Self::Postgres(store) => store.get_route_diagnostics(route_policy_id).await,
        }
    }

    pub async fn get_usage_summary(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
    ) -> Result<UsageSummaryResponse> {
        match self {
            Self::Memory(_) => Ok(sample_usage_summary_response(
                tenant_id,
                project_id.as_deref(),
                window_start.as_deref(),
                window_end.as_deref(),
            )),
            Self::Postgres(store) => {
                store
                    .get_usage_summary(tenant_id, project_id, window_start, window_end)
                    .await
            }
        }
    }

    pub async fn get_usage_breakdown(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
        group_by: UsageBreakdownGroupBy,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<UsageBreakdownResponse> {
        match self {
            Self::Memory(_) => Ok(sample_usage_breakdown_response(
                tenant_id,
                project_id.as_deref(),
                group_by,
                cursor,
                limit,
            )),
            Self::Postgres(store) => {
                store
                    .get_usage_breakdown(
                        tenant_id,
                        project_id,
                        window_start,
                        window_end,
                        group_by,
                        cursor,
                        limit,
                    )
                    .await
            }
        }
    }

    pub async fn get_balance_projection(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
    ) -> Result<BalanceProjectionResponse> {
        match self {
            Self::Memory(_) => Ok(sample_balance_projection_response(
                tenant_id,
                project_id.as_deref(),
            )),
            Self::Postgres(store) => store.get_balance_projection(tenant_id, project_id).await,
        }
    }

    pub async fn get_pricing_catalog(&self) -> Result<PricingCatalogResponse> {
        Ok(sample_pricing_catalog_response())
    }

    pub async fn create_pricing_simulation(
        &self,
        request: PricingSimulationRequest,
    ) -> Result<PricingSimulationResponse> {
        Ok(simulate_pricing(request))
    }

    pub async fn create_billing_export(
        &self,
        request: BillingExportRequest,
    ) -> Result<BillingExportJobResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let export_job_id = format!("export_mem_{}", store.billing_export_jobs.len() + 1);
                let requested_at = now_rfc3339();
                let job = BillingExportJob {
                    export_job_id: export_job_id.clone(),
                    status: "queued".to_string(),
                    format: request.format.clone(),
                    requested_at: requested_at.clone(),
                    completed_at: None,
                    error_message: None,
                    tenant_id: request.tenant_id.clone(),
                    project_id: request.project_id.clone(),
                };
                let record = BillingExportJobRecord {
                    job: job.clone(),
                    content: Some("bucket,provider_id,model_alias,input_tokens,output_tokens,cached_input_tokens,provider_cost,billable_price\n2026-04-21,openai,reasoning-fast,18420,6245,1220,0.124500,0.152025\n".to_string()),
                    content_type: "text/csv".to_string(),
                };
                store.billing_export_jobs.push(record);
                Ok(BillingExportJobResponse { data: job })
            }
            Self::Postgres(store) => store.create_billing_export(request).await,
        }
    }

    pub async fn list_billing_exports(
        &self,
        tenant_id: Option<String>,
        project_id: Option<String>,
    ) -> Result<BillingExportJobsResponse> {
        match self {
            Self::Memory(store) => Ok(BillingExportJobsResponse {
                data: filter_billing_export_jobs(
                    store
                        .read()
                        .expect("memory store read lock")
                        .billing_export_jobs
                        .iter()
                        .map(|record| record.job.clone())
                        .collect(),
                    tenant_id,
                    project_id,
                ),
            }),
            Self::Postgres(store) => store.list_billing_exports(tenant_id, project_id).await,
        }
    }

    pub async fn get_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<BillingExportJobResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .billing_export_jobs
                .iter()
                .find(|record| record.job.export_job_id == export_job_id)
                .map(|record| BillingExportJobResponse {
                    data: maybe_complete_memory_export_job(
                        record.job.clone(),
                        record.content.is_some(),
                    ),
                })),
            Self::Postgres(store) => store.get_billing_export(export_job_id).await,
        }
    }

    pub async fn download_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<(String, String)>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .billing_export_jobs
                .iter()
                .find(|record| record.job.export_job_id == export_job_id)
                .and_then(|record| {
                    record
                        .content
                        .as_ref()
                        .map(|content| (record.content_type.clone(), content.clone()))
                })),
            Self::Postgres(store) => store.download_billing_export(export_job_id).await,
        }
    }

    #[cfg(test)]
    pub fn insert_route_receipt_for_tests(&self, route_receipt: RouteReceipt) {
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .route_receipts
                    .insert(
                        route_receipt.route_receipt_id.as_str().to_string(),
                        route_receipt,
                    );
            }
            Self::Postgres(_) => panic!("test helper only supports memory store"),
        }
    }

    #[cfg(test)]
    pub fn set_provider_resource_provider_id_for_tests(
        &self,
        provider_resource_id: &str,
        provider_id: &str,
    ) {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let resource = store
                    .provider_resources
                    .iter_mut()
                    .find(|resource| resource.provider_resource_id.as_str() == provider_resource_id)
                    .expect("provider resource should exist in memory store");
                resource.provider_id = provider_id.to_string();
                resource.supported_protocol_families = match provider_id {
                    "anthropic" => vec!["anthropic_messages".to_string()],
                    "gemini" => vec!["gemini_generate_content".to_string()],
                    _ => vec!["openai_chat".to_string(), "openai_responses".to_string()],
                };
            }
            Self::Postgres(_) => panic!("test helper only supports memory store"),
        }
    }
}

pub async fn migrate_from_env() -> Result<()> {
    let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL")
        .map_err(|_| anyhow!("CONTROL_PLANE_DATABASE_URL is required to run migrations"))?;
    let store = PostgresStore::connect(&database_url).await?;
    store.migrate().await
}

pub async fn bootstrap_from_env() -> Result<()> {
    let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL").map_err(|_| {
        anyhow!("CONTROL_PLANE_DATABASE_URL is required to bootstrap control-plane data")
    })?;
    let store = PostgresStore::connect(&database_url).await?;
    store.migrate().await?;
    store.seed().await
}

pub async fn schema_status_from_env() -> Result<String> {
    let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL").map_err(|_| {
        anyhow!("CONTROL_PLANE_DATABASE_URL is required to inspect control-plane schema")
    })?;
    let store = PostgresStore::connect(&database_url).await?;
    store.ensure_schema_ready().await?;
    Ok("control-plane schema is ready".to_string())
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .with_context(|| {
                format!("failed to connect to control-plane database at {database_url}")
            })?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        for statement in MIGRATIONS {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn ensure_schema_ready(&self) -> Result<()> {
        for table_name in REQUIRED_TABLES {
            let exists = sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass($1)")
                .bind(table_name)
                .fetch_one(&self.pool)
                .await?;
            if exists.is_none() {
                return Err(anyhow!(
                    "required control-plane table `{table_name}` is missing; run `control-plane-api migrate` first"
                ));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub async fn seed(&self) -> Result<()> {
        let seed = SeedData::bootstrap();
        for tenant in seed.tenants {
            let tenant_id = tenant.tenant_id.to_string();
            sqlx::query(
                "INSERT INTO tenants (tenant_id, payload) VALUES ($1, $2)
                 ON CONFLICT (tenant_id) DO UPDATE SET payload = EXCLUDED.payload",
            )
            .bind(&tenant_id)
            .bind(Json(tenant))
            .execute(&self.pool)
            .await?;
        }

        for project in seed.projects {
            let project_id = project.project_id.to_string();
            let tenant_id = project.tenant_id.to_string();
            sqlx::query(
                "INSERT INTO projects (project_id, tenant_id, payload) VALUES ($1, $2, $3)
                 ON CONFLICT (project_id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
            )
            .bind(&project_id)
            .bind(&tenant_id)
            .bind(Json(project))
            .execute(&self.pool)
            .await?;
        }

        for provider_resource in seed.provider_resources {
            let provider_resource_id = provider_resource.provider_resource_id.to_string();
            let tenant_id = provider_resource.tenant_id.to_string();
            let project_id = provider_resource
                .project_id
                .as_ref()
                .map(ToString::to_string);
            let provider_id = provider_resource.provider_id.clone();
            sqlx::query(
                "INSERT INTO provider_resources (provider_resource_id, tenant_id, project_id, provider_id, payload)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (provider_resource_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   project_id = EXCLUDED.project_id,
                   provider_id = EXCLUDED.provider_id,
                   payload = EXCLUDED.payload",
            )
            .bind(&provider_resource_id)
            .bind(&tenant_id)
            .bind(project_id.as_deref())
            .bind(&provider_id)
            .bind(Json(provider_resource))
            .execute(&self.pool)
            .await?;
        }

        for route_policy in seed.route_policies {
            let route_policy_id = route_policy.route_policy_id.to_string();
            let tenant_id = route_policy.tenant_id.to_string();
            sqlx::query(
                "INSERT INTO route_policies (route_policy_id, tenant_id, payload) VALUES ($1, $2, $3)
                 ON CONFLICT (route_policy_id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
            )
            .bind(&route_policy_id)
            .bind(&tenant_id)
            .bind(Json(route_policy))
            .execute(&self.pool)
            .await?;
        }

        for config_snapshot in seed.config_snapshots {
            let config_snapshot_id = config_snapshot.config_snapshot_id.to_string();
            let tenant_id = config_snapshot.tenant_id.to_string();
            let project_id = config_snapshot.project_id.to_string();
            let status = config_snapshot_status_slug(config_snapshot.status);
            sqlx::query(
                "INSERT INTO config_snapshots (config_snapshot_id, tenant_id, project_id, status, payload)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (config_snapshot_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   project_id = EXCLUDED.project_id,
                   status = EXCLUDED.status,
                   payload = EXCLUDED.payload",
            )
            .bind(&config_snapshot_id)
            .bind(&tenant_id)
            .bind(&project_id)
            .bind(status)
            .bind(Json(config_snapshot))
            .execute(&self.pool)
            .await?;
        }

        sqlx::query(
            "INSERT INTO active_config_pointers (pointer_key, config_snapshot_id)
             VALUES ('default', $1)
             ON CONFLICT (pointer_key) DO UPDATE SET config_snapshot_id = EXCLUDED.config_snapshot_id",
        )
        .bind(seed.active_config_snapshot_id)
        .execute(&self.pool)
        .await?;

        for user_seed in seed.users {
            let user_id = user_seed.user.user_id.as_str().to_string();
            let primary_email = user_seed.user.primary_email.clone();
            sqlx::query(
                "INSERT INTO users (user_id, primary_email, payload) VALUES ($1, $2, $3)
                 ON CONFLICT (user_id) DO UPDATE SET primary_email = EXCLUDED.primary_email, payload = EXCLUDED.payload",
            )
            .bind(&user_id)
            .bind(primary_email.as_deref())
            .bind(Json(user_seed.user))
            .execute(&self.pool)
            .await?;

            for membership in user_seed.memberships {
                let membership_id = membership.membership_id.to_string();
                let tenant_id = membership.tenant.id.to_string();
                sqlx::query(
                    "INSERT INTO tenant_memberships (membership_id, user_id, tenant_id, payload)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (membership_id) DO UPDATE SET user_id = EXCLUDED.user_id, tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
                )
                .bind(&membership_id)
                .bind(&user_id)
                .bind(&tenant_id)
                .bind(Json(membership))
                .execute(&self.pool)
                .await?;
            }

            for link in user_seed.links {
                let link_id = link.link_id.to_string();
                let provider_slug = auth_provider_slug(link.provider);
                let provider_subject = link.provider_subject.clone();
                let email = link.email.clone();
                let can_unlink = link.can_unlink;
                sqlx::query(
                    "INSERT INTO auth_provider_links (link_id, user_id, provider, provider_subject, email, can_unlink, payload)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)
                     ON CONFLICT (link_id) DO UPDATE SET
                       user_id = EXCLUDED.user_id,
                       provider = EXCLUDED.provider,
                       provider_subject = EXCLUDED.provider_subject,
                       email = EXCLUDED.email,
                       can_unlink = EXCLUDED.can_unlink,
                       payload = EXCLUDED.payload",
                )
                .bind(&link_id)
                .bind(&user_id)
                .bind(provider_slug)
                .bind(&provider_subject)
                .bind(email.as_deref())
                .bind(can_unlink)
                .bind(Json(link))
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn ensure_known_email(&self, email: &str) -> Result<bool> {
        let row =
            sqlx::query("SELECT user_id FROM users WHERE lower(primary_email) = lower($1) LIMIT 1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.is_some())
    }

    async fn create_login_flow(&self, flow: &LoginFlow) -> Result<LoginFlow> {
        sqlx::query(
            "INSERT INTO login_flows (flow_id, flow_kind, email, provider, workspace_slug, expires_at, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (flow_id) DO UPDATE SET
               flow_kind = EXCLUDED.flow_kind,
               email = EXCLUDED.email,
               provider = EXCLUDED.provider,
               workspace_slug = EXCLUDED.workspace_slug,
               expires_at = EXCLUDED.expires_at,
               payload = EXCLUDED.payload",
        )
        .bind(&flow.flow_id)
        .bind(login_flow_kind_slug(flow.flow_kind))
        .bind(flow.email.as_deref())
        .bind(flow.provider.map(oauth_provider_slug))
        .bind(&flow.workspace_slug)
        .bind(&flow.expires_at)
        .bind(Json(flow))
        .execute(&self.pool)
        .await?;
        Ok(flow.clone())
    }

    async fn consume_login_flow(&self, flow_id: &str) -> Result<Option<LoginFlow>> {
        let row = sqlx::query("DELETE FROM login_flows WHERE flow_id = $1 RETURNING payload")
            .bind(flow_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<LoginFlow>, _>("payload").0))
    }

    async fn issue_session(
        &self,
        session_id: &str,
        provider: AuthProvider,
        identity_key: &IdentityLookup,
        workspace_slug: &str,
        now: &str,
        expires_at: &str,
    ) -> Result<AuthLoginResult> {
        let user = match self.lookup_user(identity_key).await? {
            Some(user) => user,
            None if provider == AuthProvider::Oidc => match identity_key {
                IdentityLookup::ProviderSubject(AuthProvider::Oidc, subject) => {
                    self.ensure_oidc_user(subject, workspace_slug, now).await?
                }
                _ => anyhow::bail!("identity not found"),
            },
            None => anyhow::bail!("identity not found"),
        };
        let memberships = self.list_memberships(&user.user_id).await?;
        let active_membership = memberships
            .iter()
            .find(|membership| membership.tenant.slug == workspace_slug)
            .or_else(|| memberships.first())
            .cloned()
            .context("no tenant memberships available for login")?;
        let links = self.list_links(&user.user_id).await?;

        let session = AuthSession {
            session_id: AuthSessionId::parse(session_id.to_string()).unwrap(),
            state: AuthSessionState::Active,
            user: user.clone(),
            active_tenant_id: Some(active_membership.tenant.id.clone()),
            memberships,
            authenticated_by: provider,
            created_at: now.to_string(),
            expires_at: expires_at.to_string(),
            last_authenticated_at: now.to_string(),
        };

        sqlx::query(
            "INSERT INTO sessions (session_id, user_id, state, active_tenant_id, authenticated_by, expires_at, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (session_id) DO UPDATE SET
               user_id = EXCLUDED.user_id,
               state = EXCLUDED.state,
               active_tenant_id = EXCLUDED.active_tenant_id,
               authenticated_by = EXCLUDED.authenticated_by,
               expires_at = EXCLUDED.expires_at,
               payload = EXCLUDED.payload",
        )
        .bind(session.session_id.as_str())
        .bind(user.user_id.as_str())
        .bind("active")
        .bind(session.active_tenant_id.as_ref().map(core_domain::TenantId::as_str))
        .bind(auth_provider_slug(provider))
        .bind(&session.expires_at)
        .bind(Json(session.clone()))
        .execute(&self.pool)
        .await?;

        Ok(AuthLoginResult { session, links })
    }

    async fn ensure_oidc_user(
        &self,
        subject: &str,
        workspace_slug: &str,
        now: &str,
    ) -> Result<UserIdentity> {
        self.upsert_oidc_user(
            subject,
            std::env::var("CONTROL_PLANE_OIDC_EMAIL").ok().as_deref(),
            std::env::var("CONTROL_PLANE_OIDC_DISPLAY_NAME")
                .ok()
                .as_deref(),
            workspace_slug,
            if workspace_slug == "platform-admin" {
                TenantMembershipRole::Admin
            } else {
                TenantMembershipRole::Member
            },
            now,
        )
        .await
    }

    async fn upsert_oidc_user(
        &self,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        let row = sqlx::query("SELECT payload FROM tenants WHERE payload->>'slug' = $1 LIMIT 1")
            .bind(workspace_slug)
            .fetch_optional(&self.pool)
            .await?
            .context("workspace not found for oidc login")?;
        let tenant = row.get::<Json<Tenant>, _>("payload").0;

        let digest = Sha256::digest(subject.as_bytes());
        let subject_hash = digest
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let user_id = format!("user_{subject_hash}");
        let primary_email = email
            .map(str::to_string)
            .or_else(|| {
                std::env::var("CONTROL_PLANE_OIDC_EMAIL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| format!("{subject}@enterprise.example"));
        let display_name = display_name
            .map(str::to_string)
            .or_else(|| {
                std::env::var("CONTROL_PLANE_OIDC_DISPLAY_NAME")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| "Enterprise SSO User".to_string());

        let user = UserIdentity {
            user_id: UserId::parse(user_id.clone()).unwrap(),
            primary_email: Some(primary_email.clone()),
            display_name,
            avatar_url: None,
            created_at: now.to_string(),
            last_login_at: Some(now.to_string()),
        };

        sqlx::query(
            "INSERT INTO users (user_id, primary_email, payload) VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET primary_email = EXCLUDED.primary_email, payload = EXCLUDED.payload",
        )
        .bind(&user_id)
        .bind(&primary_email)
        .bind(Json(user.clone()))
        .execute(&self.pool)
        .await?;

        let membership = membership(&format!("tmemb_oidc_{subject_hash}"), &tenant, role);
        let membership_id = membership.membership_id.to_string();
        let membership_tenant_id = membership.tenant.id.to_string();
        sqlx::query(
            "INSERT INTO tenant_memberships (membership_id, user_id, tenant_id, payload)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (membership_id) DO UPDATE SET user_id = EXCLUDED.user_id, tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
        )
        .bind(&membership_id)
        .bind(&user_id)
        .bind(&membership_tenant_id)
        .bind(Json(membership))
        .execute(&self.pool)
        .await?;

        let link = link(AuthProvider::Oidc, subject, Some(&primary_email), false);
        let link_id = link.link_id.to_string();
        sqlx::query(
            "INSERT INTO auth_provider_links (link_id, user_id, provider, provider_subject, email, can_unlink, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (link_id) DO UPDATE SET
               user_id = EXCLUDED.user_id,
               provider = EXCLUDED.provider,
               provider_subject = EXCLUDED.provider_subject,
               email = EXCLUDED.email,
               can_unlink = EXCLUDED.can_unlink,
               payload = EXCLUDED.payload",
        )
        .bind(&link_id)
        .bind(&user_id)
        .bind(auth_provider_slug(AuthProvider::Oidc))
        .bind(subject)
        .bind(Some(primary_email.as_str()))
        .bind(false)
        .bind(Json(link))
        .execute(&self.pool)
        .await?;

        Ok(user)
    }

    async fn upsert_oauth_user(
        &self,
        provider: AuthProvider,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        self.upsert_federated_user(
            provider,
            subject,
            email,
            display_name,
            workspace_slug,
            role,
            now,
        )
        .await
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AuthLoginResult>> {
        let row = sqlx::query(
            "SELECT user_id, payload FROM sessions WHERE session_id = $1 AND state = 'active'",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let session = row.get::<Json<AuthSession>, _>("payload").0;
        if timestamp_is_expired(&session.expires_at) {
            let _ = self.revoke_session(session_id).await?;
            return Ok(None);
        }
        let user_id = row.get::<String, _>("user_id");
        let user = self
            .load_user(&UserId::parse(user_id).unwrap())
            .await?
            .unwrap_or(session.user.clone());
        let memberships = self.list_memberships(&user.user_id).await?;
        let links = self.list_links(&user.user_id).await?;
        let active_tenant_id =
            resolve_active_tenant_id(session.active_tenant_id.as_ref(), &memberships);

        Ok(Some(AuthLoginResult {
            session: AuthSession {
                user,
                memberships,
                active_tenant_id,
                ..session
            },
            links,
        }))
    }

    async fn revoke_session(&self, session_id: &str) -> Result<LogoutResponse> {
        let revoked = sqlx::query("DELETE FROM sessions WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0;
        Ok(LogoutResponse {
            session_id: AuthSessionId::parse(session_id.to_string()).unwrap(),
            revoked,
        })
    }

    async fn unlink_provider(
        &self,
        session_id: &str,
        provider: AuthProvider,
    ) -> Result<Option<UnlinkAuthProviderResponse>> {
        let Some(session) = self.get_session(session_id).await? else {
            return Ok(None);
        };
        let removed = if provider == AuthProvider::Email {
            false
        } else {
            sqlx::query("DELETE FROM auth_provider_links WHERE user_id = $1 AND provider = $2")
                .bind(session.session.user.user_id.as_str())
                .bind(auth_provider_slug(provider))
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0
        };
        if removed {
            self.revoke_sessions_for_user(&session.session.user.user_id)
                .await?;
        }
        Ok(Some(UnlinkAuthProviderResponse { provider, removed }))
    }

    async fn workspace_exists(&self, workspace_slug: &str) -> Result<bool> {
        Ok(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tenants WHERE payload->>'slug' = $1",
            )
            .bind(workspace_slug)
            .fetch_one(&self.pool)
            .await?
                > 0,
        )
    }

    async fn list_tenants(&self) -> Result<TenantsResponse> {
        let rows = sqlx::query("SELECT payload FROM tenants ORDER BY tenant_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(TenantsResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<Tenant>, _>("payload").0)
                .collect(),
        })
    }

    async fn list_projects(&self) -> Result<ProjectsResponse> {
        let rows = sqlx::query("SELECT payload FROM projects ORDER BY project_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(ProjectsResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<Project>, _>("payload").0)
                .collect(),
        })
    }

    async fn get_project(&self, project_id: &str) -> Result<Option<Project>> {
        let row = sqlx::query("SELECT payload FROM projects WHERE project_id = $1")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<Project>, _>("payload").0))
    }

    async fn list_provider_resources(
        &self,
        filters: &ProviderResourceFilters,
    ) -> Result<ProviderResourcesResponse> {
        let rows =
            sqlx::query("SELECT payload FROM provider_resources ORDER BY provider_resource_id")
                .fetch_all(&self.pool)
                .await?;
        Ok(ProviderResourcesResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<ProviderResource>, _>("payload").0)
                .filter(|resource| provider_matches_filters(resource, filters))
                .collect(),
        })
    }

    async fn get_provider_resource(
        &self,
        provider_resource_id: &str,
    ) -> Result<Option<ProviderResource>> {
        let row =
            sqlx::query("SELECT payload FROM provider_resources WHERE provider_resource_id = $1")
                .bind(provider_resource_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|row| row.get::<Json<ProviderResource>, _>("payload").0))
    }

    async fn create_provider_resource(
        &self,
        provider_resource: &ProviderResource,
    ) -> Result<ProviderResource> {
        sqlx::query(
            "INSERT INTO provider_resources (provider_resource_id, tenant_id, project_id, provider_id, payload)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(provider_resource.provider_resource_id.as_str())
        .bind(provider_resource.tenant_id.as_str())
        .bind(provider_resource.project_id.as_ref().map(ProjectId::as_str))
        .bind(&provider_resource.provider_id)
        .bind(Json(provider_resource))
        .execute(&self.pool)
        .await?;
        Ok(provider_resource.clone())
    }

    async fn update_provider_resource(
        &self,
        provider_resource: &ProviderResource,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        let current = self
            .get_provider_resource(provider_resource.provider_resource_id.as_str())
            .await?;
        let Some(current) = current else {
            return Ok(ConcurrencyResult::NotFound);
        };
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut updated = provider_resource.clone();
        updated.version = expected_version.saturating_add(1);
        updated.created_at = current.created_at;
        sqlx::query(
            "UPDATE provider_resources
              SET tenant_id = $2, project_id = $3, provider_id = $4, payload = $5
              WHERE provider_resource_id = $1",
        )
        .bind(provider_resource.provider_resource_id.as_str())
        .bind(provider_resource.tenant_id.as_str())
        .bind(provider_resource.project_id.as_ref().map(ProjectId::as_str))
        .bind(&provider_resource.provider_id)
        .bind(Json(updated.clone()))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(updated))
    }

    async fn disable_provider_resource(
        &self,
        provider_resource_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        let current = self
            .get_provider_resource(provider_resource_id)
            .await?
            .context("provider resource not found")?;
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut disabled = current.clone();
        disabled.version = expected_version.saturating_add(1);
        disabled.status = ProviderResourceStatus::Disabled;
        disabled.updated_at = now_rfc3339();
        sqlx::query(
            "UPDATE provider_resources
             SET payload = $2
             WHERE provider_resource_id = $1",
        )
        .bind(provider_resource_id)
        .bind(Json(disabled.clone()))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(disabled))
    }

    async fn list_route_policies(&self) -> Result<RoutePoliciesResponse> {
        let rows = sqlx::query(
            "SELECT rp.payload
               FROM route_policies rp
               LEFT JOIN disabled_route_policies d ON rp.route_policy_id = d.route_policy_id
              WHERE d.route_policy_id IS NULL
              ORDER BY rp.route_policy_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(RoutePoliciesResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<RoutePolicy>, _>("payload").0)
                .collect(),
        })
    }

    async fn get_route_policy(&self, route_policy_id: &str) -> Result<Option<RoutePolicy>> {
        let row = sqlx::query("SELECT payload FROM route_policies WHERE route_policy_id = $1")
            .bind(route_policy_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<RoutePolicy>, _>("payload").0))
    }

    async fn create_route_policy(&self, route_policy: &RoutePolicy) -> Result<RoutePolicy> {
        sqlx::query(
            "INSERT INTO route_policies (route_policy_id, tenant_id, payload)
             VALUES ($1, $2, $3)",
        )
        .bind(route_policy.route_policy_id.as_str())
        .bind(route_policy.tenant_id.as_str())
        .bind(Json(route_policy))
        .execute(&self.pool)
        .await?;
        Ok(route_policy.clone())
    }

    async fn update_route_policy(
        &self,
        route_policy: &RoutePolicy,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        let current = self
            .get_route_policy(route_policy.route_policy_id.as_str())
            .await?;
        let Some(current) = current else {
            return Ok(ConcurrencyResult::NotFound);
        };
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut updated = route_policy.clone();
        updated.version = expected_version.saturating_add(1);
        updated.created_at = current.created_at;
        sqlx::query(
            "UPDATE route_policies
              SET tenant_id = $2, payload = $3
              WHERE route_policy_id = $1",
        )
        .bind(route_policy.route_policy_id.as_str())
        .bind(route_policy.tenant_id.as_str())
        .bind(Json(updated.clone()))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(updated))
    }

    async fn disable_route_policy(
        &self,
        route_policy_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        let current = self
            .get_route_policy(route_policy_id)
            .await?
            .context("route policy not found")?;
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut disabled = current.clone();
        disabled.version = expected_version.saturating_add(1);
        sqlx::query(
            "INSERT INTO disabled_route_policies (route_policy_id, disabled_at)
             VALUES ($1, $2)
             ON CONFLICT (route_policy_id)
             DO UPDATE SET disabled_at = EXCLUDED.disabled_at",
        )
        .bind(route_policy_id)
        .bind(now_rfc3339())
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE route_policies SET payload = $2 WHERE route_policy_id = $1")
            .bind(route_policy_id)
            .bind(Json(disabled.clone()))
            .execute(&self.pool)
            .await?;
        Ok(ConcurrencyResult::Applied(disabled))
    }

    async fn list_config_snapshots(&self) -> Result<ConfigSnapshotsResponse> {
        let rows = sqlx::query("SELECT payload FROM config_snapshots ORDER BY config_snapshot_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(ConfigSnapshotsResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<ConfigSnapshot>, _>("payload").0)
                .collect(),
        })
    }

    async fn active_snapshot_id(&self) -> Result<Option<String>> {
        Ok(sqlx::query(
            "SELECT config_snapshot_id FROM active_config_pointers WHERE pointer_key = 'default'",
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|row| row.get::<String, _>("config_snapshot_id")))
    }

    async fn create_config_snapshot(
        &self,
        config_snapshot: &ConfigSnapshot,
    ) -> Result<ConfigSnapshot> {
        sqlx::query(
            "INSERT INTO config_snapshots (config_snapshot_id, tenant_id, project_id, status, payload)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(config_snapshot.config_snapshot_id.as_str())
        .bind(config_snapshot.tenant_id.as_str())
        .bind(config_snapshot.project_id.as_str())
        .bind(config_snapshot_status_slug(config_snapshot.status))
        .bind(Json(config_snapshot))
        .execute(&self.pool)
        .await?;
        Ok(config_snapshot.clone())
    }

    async fn create_api_key(
        &self,
        provider_resource_id: &str,
        display_name: &str,
        key_prefix: &str,
        hash: &str,
    ) -> Result<ApiKey> {
        let row = sqlx::query(
            "SELECT tenant_id, project_id FROM provider_resources WHERE provider_resource_id = $1",
        )
        .bind(provider_resource_id)
        .fetch_one(&self.pool)
        .await?;
        let tenant_id = TenantId::parse(row.get::<String, _>("tenant_id"))
            .context("invalid tenant id in provider resource record")?;
        let project_id = row.get::<Option<String>, _>("project_id");

        let now = now_rfc3339();
        let api_key_id = format!("ak_{}", &hash[..16]);
        sqlx::query(
            "INSERT INTO api_keys
                (api_key_id, provider_resource_id, tenant_id, project_id, display_name, key_prefix, hash, is_active, version, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, true, 1, $8, $8)",
        )
        .bind(&api_key_id)
        .bind(provider_resource_id)
        .bind(tenant_id.as_str())
        .bind(project_id.as_ref().map(String::as_str))
        .bind(display_name)
        .bind(key_prefix)
        .bind(hash)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(ApiKey {
            api_key_id,
            provider_resource_id: ProviderResourceId::parse(provider_resource_id.to_string())?,
            display_name: display_name.to_string(),
            key_prefix: key_prefix.to_string(),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
            version: 1,
        })
    }

    async fn list_api_keys(&self) -> Result<ApiKeysResponse> {
        let rows = sqlx::query(
            "SELECT api_key_id, provider_resource_id, display_name, key_prefix, is_active, created_at, updated_at, version
               FROM api_keys
              ORDER BY api_key_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(ApiKeysResponse {
            data: rows
                .into_iter()
                .map(|row| ApiKey {
                    api_key_id: row.get::<String, _>("api_key_id"),
                    provider_resource_id: ProviderResourceId::parse(
                        row.get::<String, _>("provider_resource_id"),
                    )
                    .expect("stored provider_resource_id should be valid"),
                    display_name: row.get::<String, _>("display_name"),
                    key_prefix: row.get::<String, _>("key_prefix"),
                    is_active: row.get::<bool, _>("is_active"),
                    created_at: row.get::<String, _>("created_at"),
                    updated_at: row.get::<String, _>("updated_at"),
                    version: row.get::<i64, _>("version") as u64,
                })
                .collect(),
        })
    }

    async fn revoke_api_key(
        &self,
        api_key_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ApiKey>> {
        let row = sqlx::query(
            "SELECT api_key_id, provider_resource_id, display_name, key_prefix, is_active, created_at, updated_at, version
               FROM api_keys
              WHERE api_key_id = $1",
        )
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(ConcurrencyResult::NotFound);
        };
        let current_version = row.get::<i64, _>("version") as u64;
        if current_version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let provider_resource_id = row.get::<String, _>("provider_resource_id");
        let new_version = current_version.saturating_add(1);
        let now = now_rfc3339();
        sqlx::query(
            "UPDATE api_keys
                SET is_active = false, updated_at = $2, version = $3
              WHERE api_key_id = $1",
        )
        .bind(api_key_id)
        .bind(&now)
        .bind(i64::try_from(new_version).unwrap_or(i64::MAX))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(ApiKey {
            api_key_id: api_key_id.to_string(),
            provider_resource_id: ProviderResourceId::parse(provider_resource_id)
                .context("bad provider resource id in api key record")?,
            display_name: row.get::<String, _>("display_name"),
            key_prefix: row.get::<String, _>("key_prefix"),
            is_active: false,
            created_at: row.get::<String, _>("created_at"),
            updated_at: now,
            version: new_version,
        }))
    }

    async fn resolve_api_key(&self, hash: &str) -> Result<Option<ResolvedApiKey>> {
        let row = sqlx::query(
            "SELECT ak.api_key_id, ak.provider_resource_id, ak.is_active, ak.version, ak.created_at, ak.updated_at,
                    pr.tenant_id, pr.project_id
               FROM api_keys ak
               JOIN provider_resources pr ON pr.provider_resource_id = ak.provider_resource_id
              WHERE ak.hash = $1
              LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(ResolvedApiKey {
            api_key_id: row.get::<String, _>("api_key_id"),
            tenant_id: TenantId::parse(row.get::<String, _>("tenant_id"))
                .context("invalid tenant id for api key")?,
            project_id: row
                .get::<Option<String>, _>("project_id")
                .map(|project_id| ProjectId::parse(project_id))
                .transpose()
                .ok()
                .flatten(),
            is_active: row.get::<bool, _>("is_active"),
        }))
    }

    async fn get_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        let resolved_id = if config_snapshot_id == ACTIVE_CONFIG_ALIAS {
            sqlx::query("SELECT config_snapshot_id FROM active_config_pointers WHERE pointer_key = 'default'")
                .fetch_optional(&self.pool)
                .await?
                .map(|row| row.get::<String, _>("config_snapshot_id"))
        } else {
            Some(config_snapshot_id.to_string())
        };
        let Some(snapshot_id) = resolved_id else {
            return Ok(None);
        };
        let row = sqlx::query("SELECT payload FROM config_snapshots WHERE config_snapshot_id = $1")
            .bind(snapshot_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| ConfigSnapshotResponse {
            config_snapshot: row.get::<Json<ConfigSnapshot>, _>("payload").0,
        }))
    }

    async fn activate_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        let row = sqlx::query("SELECT payload FROM config_snapshots WHERE config_snapshot_id = $1")
            .bind(config_snapshot_id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut snapshot = row.get::<Json<ConfigSnapshot>, _>("payload").0;
        let mut tx = self.pool.begin().await?;
        let sibling_rows = sqlx::query(
            "SELECT config_snapshot_id, payload
             FROM config_snapshots
             WHERE tenant_id = $1 AND project_id = $2",
        )
        .bind(snapshot.tenant_id.as_str())
        .bind(snapshot.project_id.as_str())
        .fetch_all(&mut *tx)
        .await?;

        for row in sibling_rows {
            let sibling_id = row.get::<String, _>("config_snapshot_id");
            if sibling_id == config_snapshot_id {
                continue;
            }
            let mut sibling = row.get::<Json<ConfigSnapshot>, _>("payload").0;
            if sibling.status == ConfigSnapshotStatus::Active {
                sibling.status = ConfigSnapshotStatus::Superseded;
                sqlx::query(
                    "UPDATE config_snapshots
                     SET status = 'superseded', payload = $2
                     WHERE config_snapshot_id = $1",
                )
                .bind(&sibling_id)
                .bind(Json(sibling))
                .execute(&mut *tx)
                .await?;
            }
        }

        snapshot.status = ConfigSnapshotStatus::Active;
        snapshot.activated_at = Some(now_rfc3339());
        sqlx::query(
            "UPDATE config_snapshots SET status = 'active', payload = $2 WHERE config_snapshot_id = $1",
        )
        .bind(config_snapshot_id)
        .bind(Json(snapshot.clone()))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO active_config_pointers (pointer_key, config_snapshot_id)
             VALUES ('default', $1)
             ON CONFLICT (pointer_key) DO UPDATE SET config_snapshot_id = EXCLUDED.config_snapshot_id",
        )
        .bind(config_snapshot_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(ConfigSnapshotResponse {
            config_snapshot: snapshot,
        }))
    }

    async fn simulate_route(
        &self,
        request: RouteSimulationRequest,
    ) -> Result<RouteSimulationResponse> {
        let provider_resources = self
            .list_provider_resources(&ProviderResourceFilters::default())
            .await?
            .data
            .into_iter()
            .filter(|resource| resource.tenant_id == request.tenant_id)
            .collect::<Vec<_>>();
        let policies = self
            .list_route_policies()
            .await?
            .data
            .into_iter()
            .filter(|policy| policy.tenant_id == request.tenant_id)
            .collect::<Vec<_>>();
        let active_snapshot = self
            .get_active_project_snapshot(&request.tenant_id, &request.project_id)
            .await?
            .context("active config snapshot missing for project")?;
        build_route_simulation_response(&provider_resources, &policies, &active_snapshot, &request)
    }

    async fn get_route_receipt(
        &self,
        route_receipt_id: &str,
    ) -> Result<Option<RouteReceiptResponse>> {
        let row = sqlx::query("SELECT payload FROM route_receipts WHERE route_receipt_id = $1")
            .bind(route_receipt_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| RouteReceiptResponse {
            route_receipt: row.get::<Json<RouteReceipt>, _>("payload").0,
        }))
    }

    async fn list_route_receipts(
        &self,
        filters: &RouteReceiptFilters,
    ) -> Result<RouteReceiptsResponse> {
        let rows = sqlx::query("SELECT payload FROM route_receipts ORDER BY route_receipt_id")
            .fetch_all(&self.pool)
            .await?;
        let mut receipts = rows
            .into_iter()
            .map(|row| row.get::<Json<RouteReceipt>, _>("payload").0)
            .filter(|receipt| route_receipt_matches_filters(receipt, filters))
            .collect::<Vec<_>>();
        receipts.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        if let Some(limit) = filters.limit {
            receipts.truncate(limit);
        }
        Ok(RouteReceiptsResponse { data: receipts })
    }

    async fn get_route_diagnostics(
        &self,
        route_policy_id: &str,
    ) -> Result<Option<RouteDiagnosticsResponse>> {
        let provider_resources = self
            .list_provider_resources(&ProviderResourceFilters::default())
            .await?
            .data;
        let route_policies = self.list_route_policies().await?.data;
        let config_snapshots = self.list_config_snapshots().await?.data;
        let route_receipts = self
            .list_route_receipts(&RouteReceiptFilters::default())
            .await?
            .data;
        Ok(build_route_diagnostics_response(
            &provider_resources,
            &route_policies,
            &config_snapshots,
            &route_receipts,
            route_policy_id,
            self.active_snapshot_id().await?.as_deref(),
        ))
    }

    async fn get_usage_summary(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
    ) -> Result<UsageSummaryResponse> {
        let window_start_value = window_start.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        let window_end_value = window_end.unwrap_or_else(now_rfc3339);
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(event_count), 0) AS event_count,
                COALESCE(SUM(input_tokens), 0) AS input_tokens,
                COALESCE(SUM(output_tokens), 0) AS output_tokens,
                COALESCE(SUM(cached_input_tokens), 0) AS cached_input_tokens,
                COALESCE(SUM(provider_cost_micros), 0) AS provider_cost_micros,
                COALESCE(SUM(billable_cost_micros), 0) AS billable_cost_micros,
                COALESCE(MIN(currency), 'USD') AS currency
            FROM usage_daily_projections
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR project_id = $2)
              AND usage_date >= DATE($3::timestamptz)
              AND usage_date <= DATE($4::timestamptz)
            "#,
        )
        .bind(tenant_id)
        .bind(project_id.as_deref())
        .bind(&window_start_value)
        .bind(&window_end_value)
        .fetch_one(&self.pool)
        .await?;

        let currency = row.get::<String, _>("currency");
        Ok(UsageSummaryResponse {
            data: UsageSummary {
                tenant_id: TenantId::parse(tenant_id.to_string())?,
                project_id: project_id.map(ProjectId::parse).transpose()?,
                window_start: window_start_value,
                window_end: window_end_value,
                currency: currency.clone(),
                event_count: u64::try_from(row.get::<i64, _>("event_count")).unwrap_or_default(),
                input_tokens: u64::try_from(row.get::<i64, _>("input_tokens")).unwrap_or_default(),
                output_tokens: u64::try_from(row.get::<i64, _>("output_tokens"))
                    .unwrap_or_default(),
                cached_input_tokens: u64::try_from(row.get::<i64, _>("cached_input_tokens"))
                    .unwrap_or_default(),
                provider_cost: format_monetary_amount(
                    &currency,
                    row.get::<i64, _>("provider_cost_micros"),
                ),
                billable_price: format_monetary_amount(
                    &currency,
                    row.get::<i64, _>("billable_cost_micros"),
                ),
            },
        })
    }

    async fn get_usage_breakdown(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
        group_by: UsageBreakdownGroupBy,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<UsageBreakdownResponse> {
        let offset = parse_cursor_offset(cursor.as_deref());
        let limit = limit.unwrap_or(50).max(1);
        let window_start_value = window_start.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        let window_end_value = window_end.unwrap_or_else(now_rfc3339);
        let (bucket_select, provider_select, model_select, group_expr) = match group_by {
            UsageBreakdownGroupBy::Provider => (
                "provider_id AS bucket",
                "provider_id AS provider_id",
                "NULL::text AS model_alias",
                "provider_id",
            ),
            UsageBreakdownGroupBy::Model => (
                "model_alias AS bucket",
                "NULL::text AS provider_id",
                "model_alias AS model_alias",
                "model_alias",
            ),
            UsageBreakdownGroupBy::Day => (
                "TO_CHAR(usage_date, 'YYYY-MM-DD') AS bucket",
                "NULL::text AS provider_id",
                "NULL::text AS model_alias",
                "usage_date",
            ),
        };
        let sql = format!(
            r#"
            SELECT
                {bucket_select},
                {provider_select},
                {model_select},
                COALESCE(SUM(input_tokens), 0) AS input_tokens,
                COALESCE(SUM(output_tokens), 0) AS output_tokens,
                COALESCE(SUM(cached_input_tokens), 0) AS cached_input_tokens,
                COALESCE(SUM(provider_cost_micros), 0) AS provider_cost_micros,
                COALESCE(SUM(billable_cost_micros), 0) AS billable_cost_micros,
                COALESCE(MIN(currency), 'USD') AS currency
            FROM usage_daily_projections
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR project_id = $2)
              AND usage_date >= DATE($3::timestamptz)
              AND usage_date <= DATE($4::timestamptz)
            GROUP BY {group_expr}
            ORDER BY {group_expr}
            OFFSET $5 LIMIT $6
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(tenant_id)
            .bind(project_id.as_deref())
            .bind(&window_start_value)
            .bind(&window_end_value)
            .bind(i64::try_from(offset).unwrap_or(i64::MAX))
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?;

        let data = rows
            .iter()
            .map(|row| {
                let currency = row.get::<String, _>("currency");
                UsageBreakdownRow {
                    bucket: row.get::<String, _>("bucket"),
                    provider_id: row.get::<Option<String>, _>("provider_id"),
                    model_alias: row.get::<Option<String>, _>("model_alias"),
                    input_tokens: u64::try_from(row.get::<i64, _>("input_tokens"))
                        .unwrap_or_default(),
                    output_tokens: u64::try_from(row.get::<i64, _>("output_tokens"))
                        .unwrap_or_default(),
                    cached_input_tokens: u64::try_from(row.get::<i64, _>("cached_input_tokens"))
                        .unwrap_or_default(),
                    provider_cost: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("provider_cost_micros"),
                    ),
                    billable_price: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("billable_cost_micros"),
                    ),
                }
            })
            .collect::<Vec<_>>();

        let next_cursor = if data.len() == usize::try_from(limit).unwrap_or(usize::MAX) {
            Some((offset + data.len()).to_string())
        } else {
            None
        };

        Ok(UsageBreakdownResponse { data, next_cursor })
    }

    async fn get_balance_projection(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
    ) -> Result<BalanceProjectionResponse> {
        let row = sqlx::query(
            r#"
            SELECT
                currency,
                provider_cost_micros,
                billable_cost_micros,
                configured_budget_micros,
                remaining_budget_micros,
                threshold_status,
                last_projected_at::text AS last_projected_at
            FROM balance_projections
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR project_id = $2)
            ORDER BY last_projected_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(project_id.as_deref())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let currency = row.get::<String, _>("currency");
            let last_projected_at = row.get::<String, _>("last_projected_at");
            let lag = projection_lag_seconds(&last_projected_at);
            Ok(BalanceProjectionResponse {
                data: BalanceProjection {
                    tenant_id: TenantId::parse(tenant_id.to_string())?,
                    project_id: project_id.map(ProjectId::parse).transpose()?,
                    currency: currency.clone(),
                    provider_cost_total: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("provider_cost_micros"),
                    ),
                    billable_total: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("billable_cost_micros"),
                    ),
                    configured_budget: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("configured_budget_micros"),
                    ),
                    remaining_budget: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("remaining_budget_micros"),
                    ),
                    threshold_status: row.get::<String, _>("threshold_status"),
                    last_projected_at,
                    projection_lag_seconds: lag,
                },
            })
        } else {
            let currency = "USD".to_string();
            let configured_budget_micros =
                default_budget_micros(tenant_id, project_id.as_deref().unwrap_or("project"));
            Ok(BalanceProjectionResponse {
                data: BalanceProjection {
                    tenant_id: TenantId::parse(tenant_id.to_string())?,
                    project_id: project_id.map(ProjectId::parse).transpose()?,
                    currency: currency.clone(),
                    provider_cost_total: format_monetary_amount(&currency, 0),
                    billable_total: format_monetary_amount(&currency, 0),
                    configured_budget: format_monetary_amount(&currency, configured_budget_micros),
                    remaining_budget: format_monetary_amount(&currency, configured_budget_micros),
                    threshold_status: "ok".to_string(),
                    last_projected_at: now_rfc3339(),
                    projection_lag_seconds: 0,
                },
            })
        }
    }

    async fn create_billing_export(
        &self,
        request: BillingExportRequest,
    ) -> Result<BillingExportJobResponse> {
        let export_job_id = format!("export_{}", OffsetDateTime::now_utc().unix_timestamp());
        let requested_at = now_rfc3339();
        let export_content = render_billing_export_csv(
            self,
            request
                .tenant_id
                .as_ref()
                .map(core_domain::TenantId::as_str),
            request
                .project_id
                .as_ref()
                .map(core_domain::ProjectId::as_str),
            &request.window_start,
            &request.window_end,
        )
        .await?;
        sqlx::query(
            r#"
            INSERT INTO billing_export_jobs (
                export_job_id,
                tenant_id,
                project_id,
                window_start,
                window_end,
                format,
                status,
                requested_at,
                export_content,
                content_type
            ) VALUES ($1, $2, $3, $4, $5, $6, 'queued', $7, $8, 'text/csv')
            "#,
        )
        .bind(&export_job_id)
        .bind(
            request
                .tenant_id
                .as_ref()
                .map(core_domain::TenantId::as_str),
        )
        .bind(
            request
                .project_id
                .as_ref()
                .map(core_domain::ProjectId::as_str),
        )
        .bind(&request.window_start)
        .bind(&request.window_end)
        .bind(&request.format)
        .bind(&requested_at)
        .bind(&export_content)
        .execute(&self.pool)
        .await?;

        Ok(BillingExportJobResponse {
            data: BillingExportJob {
                export_job_id,
                status: "queued".to_string(),
                format: request.format,
                requested_at,
                completed_at: None,
                error_message: None,
                tenant_id: request.tenant_id,
                project_id: request.project_id,
            },
        })
    }

    async fn list_billing_exports(
        &self,
        tenant_id: Option<String>,
        project_id: Option<String>,
    ) -> Result<BillingExportJobsResponse> {
        let rows = sqlx::query(
            r#"
            SELECT
                export_job_id,
                tenant_id,
                project_id,
                format,
                status,
                requested_at,
                completed_at,
                error_message,
                export_content
            FROM billing_export_jobs
            ORDER BY requested_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(maybe_complete_postgres_export_job(&self.pool, row).await?);
        }

        Ok(BillingExportJobsResponse {
            data: filter_billing_export_jobs(jobs, tenant_id, project_id),
        })
    }

    async fn get_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<BillingExportJobResponse>> {
        let row = sqlx::query(
            r#"
            SELECT
                export_job_id,
                tenant_id,
                project_id,
                format,
                status,
                requested_at,
                completed_at,
                error_message,
                export_content
            FROM billing_export_jobs
            WHERE export_job_id = $1
            "#,
        )
        .bind(export_job_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(BillingExportJobResponse {
            data: maybe_complete_postgres_export_job(&self.pool, row).await?,
        }))
    }

    async fn download_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<(String, String)>> {
        let row = sqlx::query(
            r#"
            SELECT content_type, export_content
            FROM billing_export_jobs
            WHERE export_job_id = $1
            "#,
        )
        .bind(export_job_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let content = row.get::<Option<String>, _>("export_content");
        let content_type = row.get::<Option<String>, _>("content_type");
        Ok(content.map(|value| {
            (
                content_type.unwrap_or_else(|| "text/plain".to_string()),
                value,
            )
        }))
    }

    async fn lookup_user(&self, identity_key: &IdentityLookup) -> Result<Option<UserIdentity>> {
        let row = match identity_key {
            IdentityLookup::Email(email) => {
                sqlx::query(
                    "SELECT payload FROM users WHERE lower(primary_email) = lower($1) LIMIT 1",
                )
                .bind(email)
                .fetch_optional(&self.pool)
                .await?
            }
            IdentityLookup::ProviderSubject(provider, subject) => {
                sqlx::query(
                    "SELECT u.payload AS payload
                     FROM auth_provider_links apl
                     JOIN users u ON u.user_id = apl.user_id
                     WHERE apl.provider = $1 AND apl.provider_subject = $2
                     LIMIT 1",
                )
                .bind(auth_provider_slug(*provider))
                .bind(subject)
                .fetch_optional(&self.pool)
                .await?
            }
        };
        Ok(row.map(|row| row.get::<Json<UserIdentity>, _>("payload").0))
    }

    async fn list_memberships(&self, user_id: &UserId) -> Result<Vec<TenantMembership>> {
        let rows = sqlx::query(
            "SELECT payload FROM tenant_memberships WHERE user_id = $1 ORDER BY tenant_id",
        )
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<Json<TenantMembership>, _>("payload").0)
            .collect())
    }

    async fn list_links(&self, user_id: &UserId) -> Result<Vec<AuthProviderLink>> {
        let rows = sqlx::query(
            "SELECT payload FROM auth_provider_links WHERE user_id = $1 ORDER BY provider",
        )
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<Json<AuthProviderLink>, _>("payload").0)
            .collect())
    }

    async fn load_user(&self, user_id: &UserId) -> Result<Option<UserIdentity>> {
        let row = sqlx::query("SELECT payload FROM users WHERE user_id = $1 LIMIT 1")
            .bind(user_id.as_str())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<UserIdentity>, _>("payload").0))
    }

    async fn revoke_sessions_for_user(&self, user_id: &UserId) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn upsert_federated_user(
        &self,
        provider: AuthProvider,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        let row = sqlx::query("SELECT payload FROM tenants WHERE payload->>'slug' = $1 LIMIT 1")
            .bind(workspace_slug)
            .fetch_optional(&self.pool)
            .await?
            .context("workspace not found for provider login")?;
        let tenant = row.get::<Json<Tenant>, _>("payload").0;

        let digest =
            Sha256::digest(format!("{}:{subject}", auth_provider_slug(provider)).as_bytes());
        let subject_hash = digest
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let user_id = format!("user_{subject_hash}");
        let primary_email = email.map_or_else(
            || format!("{subject}@{}.login.local", auth_provider_slug(provider)),
            str::to_string,
        );
        let user = UserIdentity {
            user_id: UserId::parse(user_id.clone()).unwrap(),
            primary_email: Some(primary_email.clone()),
            display_name: display_name.map_or_else(
                || format!("{} user", auth_provider_slug(provider)),
                str::to_string,
            ),
            avatar_url: None,
            created_at: now.to_string(),
            last_login_at: Some(now.to_string()),
        };

        sqlx::query(
            "INSERT INTO users (user_id, primary_email, payload) VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET primary_email = EXCLUDED.primary_email, payload = EXCLUDED.payload",
        )
        .bind(&user_id)
        .bind(&primary_email)
        .bind(Json(user.clone()))
        .execute(&self.pool)
        .await?;

        let membership = membership(
            &format!("tmemb_{}_{}", auth_provider_slug(provider), subject_hash),
            &tenant,
            role,
        );
        sqlx::query(
            "INSERT INTO tenant_memberships (membership_id, user_id, tenant_id, payload)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (membership_id) DO UPDATE SET user_id = EXCLUDED.user_id, tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
        )
        .bind(membership.membership_id.as_str().to_string())
        .bind(&user_id)
        .bind(membership.tenant.id.as_str())
        .bind(Json(membership.clone()))
        .execute(&self.pool)
        .await?;

        let link = link(
            provider,
            subject,
            Some(&primary_email),
            provider != AuthProvider::Oidc,
        );
        sqlx::query(
            "INSERT INTO auth_provider_links (link_id, user_id, provider, provider_subject, email, can_unlink, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (link_id) DO UPDATE SET
               user_id = EXCLUDED.user_id,
               provider = EXCLUDED.provider,
               provider_subject = EXCLUDED.provider_subject,
               email = EXCLUDED.email,
               can_unlink = EXCLUDED.can_unlink,
               payload = EXCLUDED.payload",
        )
        .bind(link.link_id.as_str().to_string())
        .bind(&user_id)
        .bind(auth_provider_slug(provider))
        .bind(subject)
        .bind(Some(primary_email.as_str()))
        .bind(provider != AuthProvider::Oidc)
        .bind(Json(link.clone()))
        .execute(&self.pool)
        .await?;

        self.revoke_sessions_for_user(&UserId::parse(user_id).unwrap())
            .await?;

        Ok(user)
    }

    async fn get_active_project_snapshot(
        &self,
        tenant_id: &TenantId,
        project_id: &ProjectId,
    ) -> Result<Option<ConfigSnapshot>> {
        let row = sqlx::query(
            "SELECT payload
             FROM config_snapshots
             WHERE tenant_id = $1 AND project_id = $2 AND status = 'active'
             ORDER BY activated_at DESC NULLS LAST, config_snapshot_id DESC
             LIMIT 1",
        )
        .bind(tenant_id.as_str())
        .bind(project_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get::<Json<ConfigSnapshot>, _>("payload").0))
    }
}

#[derive(Debug, Clone)]
pub enum IdentityLookup {
    Email(String),
    ProviderSubject(AuthProvider, String),
}

fn issue_memory_session(
    store: &mut MemoryStore,
    session_id: &str,
    provider: AuthProvider,
    identity_key: &IdentityLookup,
    workspace_slug: &str,
    now: &str,
    expires_at: &str,
) -> Result<AuthLoginResult> {
    let user_id = match identity_key {
        IdentityLookup::Email(email) => store
            .email_identity_to_user_id
            .get(&email.to_lowercase())
            .cloned(),
        IdentityLookup::ProviderSubject(provider, subject) => store
            .provider_subject_to_user_id
            .get(&format!("{}:{}", auth_provider_slug(*provider), subject))
            .cloned(),
    };
    let user_id = match user_id {
        Some(user_id) => user_id,
        None if provider == AuthProvider::Oidc => match identity_key {
            IdentityLookup::ProviderSubject(AuthProvider::Oidc, subject) => {
                upsert_memory_oidc_user(
                    store,
                    subject,
                    None,
                    None,
                    workspace_slug,
                    if workspace_slug == "platform-admin" {
                        TenantMembershipRole::Admin
                    } else {
                        TenantMembershipRole::Member
                    },
                    now,
                )?
            }
            _ => anyhow::bail!("identity not found"),
        },
        None => anyhow::bail!("identity not found"),
    };

    let user = store.users.get(&user_id).cloned().context("missing user")?;
    let memberships = store
        .memberships_by_user
        .get(&user_id)
        .cloned()
        .unwrap_or_default();
    let active_membership = memberships
        .iter()
        .find(|membership| membership.tenant.slug == workspace_slug)
        .or_else(|| memberships.first())
        .cloned()
        .context("no tenant memberships")?;
    let links = store
        .provider_links_by_user
        .get(&user_id)
        .cloned()
        .unwrap_or_default();

    let session = AuthSession {
        session_id: AuthSessionId::parse(session_id.to_string()).unwrap(),
        state: AuthSessionState::Active,
        user,
        active_tenant_id: Some(active_membership.tenant.id),
        memberships,
        authenticated_by: provider,
        created_at: now.to_string(),
        expires_at: expires_at.to_string(),
        last_authenticated_at: now.to_string(),
    };
    store.sessions.insert(
        session_id.to_string(),
        StoredSession {
            session: session.clone(),
            links: links.clone(),
        },
    );

    Ok(AuthLoginResult { session, links })
}

fn refresh_memory_session(store: &MemoryStore, stored: StoredSession) -> AuthLoginResult {
    let user_id = stored.session.user.user_id.as_str();
    let memberships = store
        .memberships_by_user
        .get(user_id)
        .cloned()
        .unwrap_or_else(|| stored.session.memberships.clone());
    let links = store
        .provider_links_by_user
        .get(user_id)
        .cloned()
        .unwrap_or_else(|| stored.links.clone());

    AuthLoginResult {
        session: AuthSession {
            active_tenant_id: resolve_active_tenant_id(
                stored.session.active_tenant_id.as_ref(),
                &memberships,
            ),
            memberships,
            ..stored.session
        },
        links,
    }
}

fn upsert_memory_oidc_user(
    store: &mut MemoryStore,
    subject: &str,
    email: Option<&str>,
    display_name: Option<&str>,
    workspace_slug: &str,
    role: TenantMembershipRole,
    now: &str,
) -> Result<String> {
    let digest = Sha256::digest(subject.as_bytes());
    let subject_hash = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let user_id = format!("user_{subject_hash}");

    if store.users.contains_key(&user_id) {
        return Ok(user_id);
    }

    let tenant = store
        .tenants
        .iter()
        .find(|tenant| tenant.slug == workspace_slug)
        .cloned()
        .context("workspace not found for oidc login")?;
    let primary_email = email
        .map(str::to_string)
        .or_else(|| {
            std::env::var("CONTROL_PLANE_OIDC_EMAIL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| format!("{subject}@enterprise.example"));
    let user = UserIdentity {
        user_id: UserId::parse(user_id.clone()).unwrap(),
        primary_email: Some(primary_email.clone()),
        display_name: display_name
            .map(str::to_string)
            .or_else(|| {
                std::env::var("CONTROL_PLANE_OIDC_DISPLAY_NAME")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| "Enterprise SSO User".to_string()),
        avatar_url: None,
        created_at: now.to_string(),
        last_login_at: Some(now.to_string()),
    };
    store.users.insert(user_id.clone(), user);
    store
        .email_identity_to_user_id
        .insert(primary_email.to_lowercase(), user_id.clone());
    store.provider_subject_to_user_id.insert(
        format!("{}:{}", auth_provider_slug(AuthProvider::Oidc), subject),
        user_id.clone(),
    );
    let membership = membership(&format!("tmemb_oidc_{subject_hash}"), &tenant, role);
    store
        .memberships_by_user
        .entry(user_id.clone())
        .or_default()
        .push(membership);
    store.provider_links_by_user.insert(
        user_id.clone(),
        vec![link(
            AuthProvider::Oidc,
            subject,
            Some(&primary_email),
            false,
        )],
    );

    Ok(user_id)
}

fn upsert_memory_provider_user(
    store: &mut MemoryStore,
    provider: AuthProvider,
    subject: &str,
    email: Option<&str>,
    display_name: Option<&str>,
    workspace_slug: &str,
    role: TenantMembershipRole,
    now: &str,
) -> Result<String> {
    if provider == AuthProvider::Oidc {
        return upsert_memory_oidc_user(
            store,
            subject,
            email,
            display_name,
            workspace_slug,
            role,
            now,
        );
    }

    let digest = Sha256::digest(format!("{}:{subject}", auth_provider_slug(provider)).as_bytes());
    let subject_hash = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let user_id = format!("user_{subject_hash}");

    let tenant = store
        .tenants
        .iter()
        .find(|tenant| tenant.slug == workspace_slug)
        .cloned()
        .context("workspace not found for oauth login")?;
    let primary_email = email.map_or_else(
        || format!("{subject}@{}.login.local", auth_provider_slug(provider)),
        str::to_string,
    );
    let user = UserIdentity {
        user_id: UserId::parse(user_id.clone()).unwrap(),
        primary_email: Some(primary_email.clone()),
        display_name: display_name.map_or_else(
            || format!("{} user", auth_provider_slug(provider)),
            str::to_string,
        ),
        avatar_url: None,
        created_at: now.to_string(),
        last_login_at: Some(now.to_string()),
    };
    store.users.insert(user_id.clone(), user);
    store
        .email_identity_to_user_id
        .insert(primary_email.to_lowercase(), user_id.clone());
    store.provider_subject_to_user_id.insert(
        format!("{}:{}", auth_provider_slug(provider), subject),
        user_id.clone(),
    );

    let membership = membership(
        &format!("tmemb_{}_{}", auth_provider_slug(provider), subject_hash),
        &tenant,
        role,
    );
    let memberships = store
        .memberships_by_user
        .entry(user_id.clone())
        .or_default();
    if !memberships
        .iter()
        .any(|existing| existing.membership_id == membership.membership_id)
    {
        memberships.push(membership);
    }
    store.provider_links_by_user.insert(
        user_id.clone(),
        vec![link(provider, subject, Some(&primary_email), true)],
    );
    revoke_memory_sessions_for_user(store, &user_id);

    Ok(user_id)
}

fn revoke_memory_sessions_for_user(store: &mut MemoryStore, user_id: &str) {
    store
        .sessions
        .retain(|_, stored| stored.session.user.user_id.as_str() != user_id);
}

fn unlink_memory_provider(
    store: &mut MemoryStore,
    session_id: &str,
    provider: AuthProvider,
) -> Option<UnlinkAuthProviderResponse> {
    let user_id = store
        .sessions
        .get(session_id)?
        .session
        .user
        .user_id
        .as_str()
        .to_string();
    let removed = if provider == AuthProvider::Email {
        false
    } else {
        let before = store.sessions.get(session_id)?.links.len();
        if let Some(session) = store.sessions.get_mut(session_id) {
            session.links.retain(|link| link.provider != provider);
        }
        if let Some(user_links) = store.provider_links_by_user.get_mut(&user_id) {
            user_links.retain(|link| link.provider != provider);
        }
        let removed = store
            .sessions
            .get(session_id)
            .is_some_and(|session| session.links.len() != before);
        if removed {
            revoke_memory_sessions_for_user(store, &user_id);
        }
        removed
    };

    Some(UnlinkAuthProviderResponse { provider, removed })
}

fn update_memory_provider_resource(
    store: &mut MemoryStore,
    provider_resource: &ProviderResource,
    expected_version: u64,
) -> ConcurrencyResult<ProviderResource> {
    let index = store
        .provider_resources
        .iter()
        .position(|item| item.provider_resource_id == provider_resource.provider_resource_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .provider_resources
        .get(index)
        .expect("indexed provider resource should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut updated = provider_resource.clone();
    updated.version = expected_version.saturating_add(1);
    updated.created_at = current.created_at.clone();
    store.provider_resources[index] = updated.clone();
    ConcurrencyResult::Applied(updated)
}

fn disable_memory_provider_resource(
    store: &mut MemoryStore,
    provider_resource_id: &str,
    expected_version: u64,
) -> ConcurrencyResult<ProviderResource> {
    let index = store
        .provider_resources
        .iter()
        .position(|item| item.provider_resource_id.as_str() == provider_resource_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .provider_resources
        .get(index)
        .expect("indexed provider resource should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut disabled = current.clone();
    disabled.version = expected_version.saturating_add(1);
    disabled.status = ProviderResourceStatus::Disabled;
    disabled.updated_at = now_rfc3339();
    store.provider_resources[index] = disabled.clone();
    ConcurrencyResult::Applied(disabled)
}

fn update_memory_route_policy(
    store: &mut MemoryStore,
    route_policy: &RoutePolicy,
    expected_version: u64,
) -> ConcurrencyResult<RoutePolicy> {
    let index = store
        .route_policies
        .iter()
        .position(|item| item.route_policy_id == route_policy.route_policy_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .route_policies
        .get(index)
        .expect("indexed route policy should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut updated = route_policy.clone();
    updated.version = expected_version.saturating_add(1);
    updated.created_at = current.created_at.clone();
    store.route_policies[index] = updated.clone();
    ConcurrencyResult::Applied(updated)
}

fn disable_memory_route_policy(
    store: &mut MemoryStore,
    route_policy_id: &str,
    expected_version: u64,
) -> ConcurrencyResult<RoutePolicy> {
    let index = store
        .route_policies
        .iter()
        .position(|item| item.route_policy_id.as_str() == route_policy_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .route_policies
        .get(index)
        .expect("indexed route policy should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut disabled = current.clone();
    disabled.version = expected_version.saturating_add(1);
    store.route_policies[index] = disabled.clone();
    store
        .route_policy_disabled_ids
        .insert(route_policy_id.to_string());
    ConcurrencyResult::Applied(disabled)
}

fn insert_memory_api_key(
    store: &mut MemoryStore,
    provider_resource_id: ProviderResourceId,
    display_name: &str,
    key_prefix: &str,
    hash: String,
    now: String,
) -> ApiKey {
    let api_key_id = format!("ak_{}", &hash[..16]);
    let record = ApiKeyRecord {
        api_key_id,
        provider_resource_id,
        display_name: display_name.to_string(),
        key_prefix: key_prefix.to_string(),
        hash,
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };
    store.api_keys.push(record.clone());
    record.public_view()
}

fn revoke_memory_api_key(
    store: &mut MemoryStore,
    api_key_id: &str,
    expected_version: u64,
) -> ConcurrencyResult<ApiKey> {
    let index = store
        .api_keys
        .iter()
        .position(|item| item.api_key_id == api_key_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .api_keys
        .get(index)
        .expect("indexed api key should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    if current.is_active {
        let mut updated = current.clone();
        updated.version = expected_version.saturating_add(1);
        updated.is_active = false;
        updated.updated_at = now_rfc3339();
        store.api_keys[index] = updated.clone();
        ConcurrencyResult::Applied(updated.public_view())
    } else {
        let mut updated = current.clone();
        updated.version = expected_version.saturating_add(1);
        updated.updated_at = now_rfc3339();
        store.api_keys[index] = updated.clone();
        ConcurrencyResult::Applied(updated.public_view())
    }
}

fn api_key_prefix(api_key: &str) -> String {
    if api_key.len() <= 6 {
        api_key.to_string()
    } else {
        format!("{}...", &api_key[..6])
    }
}

fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn next_id_suffix() -> String {
    OffsetDateTime::now_utc().unix_timestamp_nanos().to_string()
}

fn replay_upstream_error_code(connection: &TrialConnection) -> String {
    let endpoint = connection.endpoint_base_url.to_ascii_lowercase();

    if endpoint.contains("vertex") {
        "provider_signature_mismatch".to_string()
    } else if endpoint.contains("bedrock") {
        "provider_channel_proxy".to_string()
    } else {
        "protocol_shape_warning".to_string()
    }
}

fn build_relay_evaluation(
    tenant_id: &TenantId,
    connection: &TrialConnection,
    replay_capsule_id: ReplayCapsuleId,
    created_at: String,
) -> RelayEvaluation {
    let endpoint = connection.endpoint_base_url.to_ascii_lowercase();
    let detected_channel = if endpoint.contains("vertex") {
        Some("vertex".to_string())
    } else if endpoint.contains("bedrock") {
        Some("aws-bedrock".to_string())
    } else {
        None
    };
    let protocol_status = if detected_channel.is_some() {
        RelayCheckStatus::Warning
    } else {
        RelayCheckStatus::Pass
    };
    let token_status = if connection
        .target_model
        .to_ascii_lowercase()
        .contains("flash")
    {
        RelayCheckStatus::Warning
    } else {
        RelayCheckStatus::Pass
    };
    let verdict = if detected_channel.is_some() {
        RelayEvaluationVerdict::Warning
    } else {
        RelayEvaluationVerdict::Healthy
    };
    let overall_score = if detected_channel.is_some() { 82 } else { 91 };

    RelayEvaluation {
        relay_evaluation_id: RelayEvaluationId::parse(format!("reval_{}", next_id_suffix()))
            .unwrap(),
        tenant_id: tenant_id.clone(),
        trial_connection_id: connection.trial_connection_id.clone(),
        replay_capsule_id,
        provider_label: connection.provider_label.clone(),
        endpoint_base_url: connection.endpoint_base_url.clone(),
        target_model: connection.target_model.clone(),
        runner_mode: RelayEvaluationRunnerMode::Simulated,
        sample_request_count: 5,
        estimated_tokens_saved: 2400,
        overall_score,
        verdict,
        fingerprint_status: RelayCheckStatus::Pass,
        protocol_status,
        token_status,
        multimodal_status: RelayCheckStatus::NotTested,
        detected_channel,
        summary:
            "Replay-ready evaluation recorded. Review protocol consistency before spending live token budget."
                .to_string(),
        created_at,
    }
}

fn activate_memory_config_snapshot(
    store: &mut MemoryStore,
    config_snapshot_id: &str,
) -> Option<ConfigSnapshotResponse> {
    let index = store
        .config_snapshots
        .iter()
        .position(|item| item.config_snapshot_id.as_str() == config_snapshot_id)?;
    let tenant_id = store.config_snapshots[index].tenant_id.clone();
    let project_id = store.config_snapshots[index].project_id.clone();
    for (candidate_index, snapshot) in store.config_snapshots.iter_mut().enumerate() {
        if candidate_index != index
            && snapshot.tenant_id == tenant_id
            && snapshot.project_id == project_id
            && snapshot.status == ConfigSnapshotStatus::Active
        {
            snapshot.status = ConfigSnapshotStatus::Superseded;
        }
    }
    let snapshot = &mut store.config_snapshots[index];
    snapshot.status = ConfigSnapshotStatus::Active;
    snapshot.activated_at = Some(now_rfc3339());
    store.active_config_snapshot_id = config_snapshot_id.to_string();
    Some(ConfigSnapshotResponse {
        config_snapshot: snapshot.clone(),
    })
}

fn simulate_memory_route(
    store: &MemoryStore,
    request: &RouteSimulationRequest,
) -> Result<RouteSimulationResponse> {
    let active_snapshot = store
        .config_snapshots
        .iter()
        .filter(|snapshot| {
            snapshot.status == ConfigSnapshotStatus::Active
                && snapshot.tenant_id == request.tenant_id
                && snapshot.project_id == request.project_id
        })
        .max_by(|left, right| {
            left.activated_at
                .cmp(&right.activated_at)
                .then_with(|| left.config_snapshot_id.cmp(&right.config_snapshot_id))
        })
        .cloned()
        .context("active config snapshot missing for project")?;
    let provider_resources = store
        .provider_resources
        .iter()
        .filter(|resource| resource.tenant_id == request.tenant_id)
        .cloned()
        .collect::<Vec<_>>();
    let route_policies = store
        .route_policies
        .iter()
        .filter(|policy| policy.tenant_id == request.tenant_id)
        .cloned()
        .collect::<Vec<_>>();
    build_route_simulation_response(
        &provider_resources,
        &route_policies,
        &active_snapshot,
        request,
    )
}

fn format_monetary_amount(currency: &str, micros: i64) -> MonetaryAmount {
    let sign = if micros < 0 { "-" } else { "" };
    let absolute = micros.abs();
    let whole = absolute / 1_000_000;
    let fractional = absolute % 1_000_000;
    MonetaryAmount {
        currency: currency.to_string(),
        amount: format!("{sign}{whole}.{fractional:06}"),
    }
}

fn projection_lag_seconds(timestamp: &str) -> u64 {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .and_then(|parsed| {
            let duration =
                chrono::Utc::now().signed_duration_since(parsed.with_timezone(&chrono::Utc));
            duration.num_seconds().try_into().ok()
        })
        .unwrap_or_default()
}

fn parse_cursor_offset(cursor: Option<&str>) -> usize {
    cursor
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default()
}

fn simulate_pricing(request: PricingSimulationRequest) -> PricingSimulationResponse {
    let quote = quote_usage_with_additions(
        &request.provider_id,
        &request.usage,
        AdditionalUsageDimensions {
            image_generation_units: request.image_generation_units.unwrap_or_default(),
            audio_seconds: request.audio_seconds.unwrap_or_default(),
        },
    );
    PricingSimulationResponse {
        catalog_id: quote.catalog_id,
        catalog_version: quote.catalog_version,
        currency: quote.currency.clone(),
        provider_cost: format_monetary_amount(&quote.currency, quote.provider_cost_micros),
        billable_price: format_monetary_amount(&quote.currency, quote.billable_cost_micros),
        line_items: quote
            .line_items
            .into_iter()
            .map(|line_item| PricingSimulationLineItem {
                dimension: pricing_dimension_slug(line_item.dimension),
                units: line_item.units,
                provider_cost: format_monetary_amount(
                    &quote.currency,
                    line_item.provider_cost_micros,
                ),
                billable_price: format_monetary_amount(
                    &quote.currency,
                    line_item.billable_cost_micros,
                ),
                rate_source: line_item.rate_source,
            })
            .collect(),
    }
}

fn sample_usage_summary_response(
    tenant_id: &str,
    project_id: Option<&str>,
    window_start: Option<&str>,
    window_end: Option<&str>,
) -> UsageSummaryResponse {
    UsageSummaryResponse {
        data: UsageSummary {
            tenant_id: TenantId::parse(tenant_id.to_string()).unwrap(),
            project_id: project_id.map(|value| ProjectId::parse(value.to_string()).unwrap()),
            window_start: window_start.unwrap_or("2026-04-01T00:00:00Z").to_string(),
            window_end: window_end.unwrap_or("2026-04-30T23:59:59Z").to_string(),
            currency: "USD".to_string(),
            event_count: 14,
            input_tokens: 18_420,
            output_tokens: 6_245,
            cached_input_tokens: 1_220,
            provider_cost: format_monetary_amount("USD", 124_500),
            billable_price: format_monetary_amount("USD", 152_025),
        },
    }
}

fn sample_usage_breakdown_response(
    _tenant_id: &str,
    _project_id: Option<&str>,
    group_by: UsageBreakdownGroupBy,
    cursor: Option<String>,
    limit: Option<u32>,
) -> UsageBreakdownResponse {
    let rows = match group_by {
        UsageBreakdownGroupBy::Provider => vec![
            UsageBreakdownRow {
                bucket: "openai".to_string(),
                provider_id: Some("openai".to_string()),
                model_alias: None,
                input_tokens: 10_000,
                output_tokens: 4_000,
                cached_input_tokens: 500,
                provider_cost: format_monetary_amount("USD", 82_000),
                billable_price: format_monetary_amount("USD", 98_400),
            },
            UsageBreakdownRow {
                bucket: "anthropic".to_string(),
                provider_id: Some("anthropic".to_string()),
                model_alias: None,
                input_tokens: 8_420,
                output_tokens: 2_245,
                cached_input_tokens: 720,
                provider_cost: format_monetary_amount("USD", 42_500),
                billable_price: format_monetary_amount("USD", 53_625),
            },
        ],
        UsageBreakdownGroupBy::Model => vec![
            UsageBreakdownRow {
                bucket: "reasoning-fast".to_string(),
                provider_id: None,
                model_alias: Some("reasoning-fast".to_string()),
                input_tokens: 8_420,
                output_tokens: 2_245,
                cached_input_tokens: 720,
                provider_cost: format_monetary_amount("USD", 42_500),
                billable_price: format_monetary_amount("USD", 53_625),
            },
            UsageBreakdownRow {
                bucket: "support-safe".to_string(),
                provider_id: None,
                model_alias: Some("support-safe".to_string()),
                input_tokens: 10_000,
                output_tokens: 4_000,
                cached_input_tokens: 500,
                provider_cost: format_monetary_amount("USD", 82_000),
                billable_price: format_monetary_amount("USD", 98_400),
            },
        ],
        UsageBreakdownGroupBy::Day => vec![UsageBreakdownRow {
            bucket: "2026-04-21".to_string(),
            provider_id: None,
            model_alias: None,
            input_tokens: 18_420,
            output_tokens: 6_245,
            cached_input_tokens: 1_220,
            provider_cost: format_monetary_amount("USD", 124_500),
            billable_price: format_monetary_amount("USD", 152_025),
        }],
    };
    let offset = parse_cursor_offset(cursor.as_deref());
    let limit = usize::try_from(limit.unwrap_or(50)).unwrap_or(50);
    let paged = rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = if paged.len() == limit {
        Some((offset + paged.len()).to_string())
    } else {
        None
    };
    UsageBreakdownResponse {
        data: paged,
        next_cursor,
    }
}

fn sample_balance_projection_response(
    tenant_id: &str,
    project_id: Option<&str>,
) -> BalanceProjectionResponse {
    let configured_budget = default_budget_micros(tenant_id, project_id.unwrap_or("project"));
    let billable_total = 1_540_000;
    BalanceProjectionResponse {
        data: BalanceProjection {
            tenant_id: TenantId::parse(tenant_id.to_string()).unwrap(),
            project_id: project_id.map(|value| ProjectId::parse(value.to_string()).unwrap()),
            currency: "USD".to_string(),
            provider_cost_total: format_monetary_amount("USD", 1_244_000),
            billable_total: format_monetary_amount("USD", billable_total),
            configured_budget: format_monetary_amount("USD", configured_budget),
            remaining_budget: format_monetary_amount("USD", configured_budget - billable_total),
            threshold_status: "ok".to_string(),
            last_projected_at: now_rfc3339(),
            projection_lag_seconds: 0,
        },
    }
}

#[allow(dead_code)]
fn sample_billing_export_job_response(
    tenant_id: Option<&str>,
    project_id: Option<&str>,
    format: &str,
) -> BillingExportJobResponse {
    BillingExportJobResponse {
        data: BillingExportJob {
            export_job_id: "export_123".to_string(),
            status: "queued".to_string(),
            format: format.to_string(),
            requested_at: now_rfc3339(),
            completed_at: None,
            error_message: None,
            tenant_id: tenant_id.map(|value| TenantId::parse(value.to_string()).unwrap()),
            project_id: project_id.map(|value| ProjectId::parse(value.to_string()).unwrap()),
        },
    }
}

fn sample_pricing_catalog_response() -> PricingCatalogResponse {
    let catalog: PricingCatalog = default_catalog();
    PricingCatalogResponse {
        catalog_id: catalog.catalog_id,
        catalog_version: catalog.catalog_version,
        currency: catalog.currency,
        entries: catalog
            .entries
            .into_iter()
            .map(|entry| PricingCatalogEntry {
                dimension: pricing_dimension_slug(entry.dimension),
                provider_id: entry.provider_id,
                model_alias: entry.model_alias,
                region: entry.region,
                micros_per_unit: entry.micros_per_unit,
                unit_denominator: entry.unit_denominator,
                source: pricing_source_slug(entry.source),
            })
            .collect(),
    }
}

fn pricing_dimension_slug(dimension: metering::PricingDimension) -> String {
    match dimension {
        metering::PricingDimension::InputTokens => "input_tokens".to_string(),
        metering::PricingDimension::OutputTokens => "output_tokens".to_string(),
        metering::PricingDimension::CachedInputTokens => "cached_input_tokens".to_string(),
        metering::PricingDimension::ImageGenerations => "image_generations".to_string(),
        metering::PricingDimension::AudioSeconds => "audio_seconds".to_string(),
    }
}

fn pricing_source_slug(source: PricingSource) -> String {
    match source {
        PricingSource::PlatformCatalog => "platform_catalog".to_string(),
        PricingSource::ProviderNative => "provider_native".to_string(),
        PricingSource::ContractOverride => "contract_override".to_string(),
        PricingSource::TenantOverride => "tenant_override".to_string(),
        PricingSource::Promotional => "promotional".to_string(),
    }
}

fn maybe_complete_memory_export_job(
    mut job: BillingExportJob,
    has_content: bool,
) -> BillingExportJob {
    if job.status == "queued" && has_content {
        job.status = "completed".to_string();
        job.completed_at = Some(now_rfc3339());
    }
    job
}

fn filter_billing_export_jobs(
    mut jobs: Vec<BillingExportJob>,
    tenant_id: Option<String>,
    project_id: Option<String>,
) -> Vec<BillingExportJob> {
    jobs.retain(|job| {
        if let Some(tenant_id) = tenant_id.as_deref()
            && job.tenant_id.as_ref().map(core_domain::TenantId::as_str) != Some(tenant_id)
        {
            return false;
        }
        if let Some(project_id) = project_id.as_deref()
            && job.project_id.as_ref().map(core_domain::ProjectId::as_str) != Some(project_id)
        {
            return false;
        }
        true
    });
    jobs.sort_by(|left, right| right.requested_at.cmp(&left.requested_at));
    jobs
}

async fn render_billing_export_csv(
    store: &PostgresStore,
    tenant_id: Option<&str>,
    project_id: Option<&str>,
    window_start: &str,
    window_end: &str,
) -> Result<String> {
    let rows = store
        .get_usage_breakdown(
            tenant_id.unwrap_or("tenant_acme"),
            project_id.map(str::to_string),
            Some(window_start.to_string()),
            Some(window_end.to_string()),
            UsageBreakdownGroupBy::Day,
            None,
            Some(500),
        )
        .await?;

    let mut csv = String::from(
        "bucket,provider_id,model_alias,input_tokens,output_tokens,cached_input_tokens,provider_cost,billable_price\n",
    );
    for row in rows.data {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            row.bucket,
            row.provider_id.unwrap_or_default(),
            row.model_alias.unwrap_or_default(),
            row.input_tokens,
            row.output_tokens,
            row.cached_input_tokens,
            row.provider_cost.amount,
            row.billable_price.amount,
        ));
    }

    Ok(csv)
}

async fn maybe_complete_postgres_export_job(
    pool: &Pool<Postgres>,
    row: sqlx::postgres::PgRow,
) -> Result<BillingExportJob> {
    let export_job_id = row.get::<String, _>("export_job_id");
    let mut status = row.get::<String, _>("status");
    let mut completed_at = row.get::<Option<String>, _>("completed_at");
    let export_content = row.get::<Option<String>, _>("export_content");

    if status == "queued" && export_content.is_some() {
        completed_at = Some(now_rfc3339());
        status = "completed".to_string();
        sqlx::query(
            "UPDATE billing_export_jobs SET status = 'completed', completed_at = $2 WHERE export_job_id = $1",
        )
        .bind(&export_job_id)
        .bind(completed_at.as_deref())
        .execute(pool)
        .await?;
    }

    Ok(BillingExportJob {
        export_job_id,
        status,
        format: row.get::<String, _>("format"),
        requested_at: row.get::<String, _>("requested_at"),
        completed_at,
        error_message: row.get::<Option<String>, _>("error_message"),
        tenant_id: row
            .get::<Option<String>, _>("tenant_id")
            .map(TenantId::parse)
            .transpose()?,
        project_id: row
            .get::<Option<String>, _>("project_id")
            .map(ProjectId::parse)
            .transpose()?,
    })
}

fn build_route_simulation_response(
    provider_resources: &[ProviderResource],
    route_policies: &[RoutePolicy],
    active_snapshot: &ConfigSnapshot,
    request: &RouteSimulationRequest,
) -> Result<RouteSimulationResponse> {
    let request_protocol_family = protocol_family_slug(&request.protocol_family);

    let route_policy = route_policies
        .iter()
        .find(|policy| policy.route_policy_id == active_snapshot.route_policy_id)
        .cloned()
        .context("route policy missing for snapshot")?;

    if route_policy.protocol_family != request_protocol_family {
        return Err(anyhow!(
            "route policy protocol family `{}` does not match request protocol family `{}`",
            route_policy.protocol_family,
            request_protocol_family,
        ));
    }

    if !route_policy
        .required_capabilities
        .iter()
        .all(|capability| route_capability_supported_by_provider_capabilities(capability.as_str()))
    {
        return Err(anyhow!(
            "route policy requires unsupported capability: {}",
            route_policy
                .required_capabilities
                .iter()
                .find(|capability| {
                    !route_capability_supported_by_provider_capabilities(capability.as_str())
                })
                .unwrap_or(&"unknown".to_string())
        ));
    }

    let candidates = provider_resources
        .iter()
        .filter(|resource| {
            active_snapshot
                .provider_resource_ids
                .iter()
                .any(|id| id == &resource.provider_resource_id)
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut excluded_candidates = Vec::new();
    let mut eligible_candidates = Vec::new();

    for candidate in candidates {
        if candidate.status != ProviderResourceStatus::Active {
            excluded_candidates.push(ExcludedTarget {
                provider_resource_id: candidate.provider_resource_id.clone(),
                reason_code: "provider_inactive".to_string(),
                reason: format!("provider status is {:?}", candidate.status),
            });
            continue;
        }
        if !provider_supports_protocol_family(&candidate, &route_policy.protocol_family) {
            excluded_candidates.push(ExcludedTarget {
                provider_resource_id: candidate.provider_resource_id.clone(),
                reason_code: "protocol_family_unsupported".to_string(),
                reason: format!(
                    "provider does not advertise protocol family `{}`",
                    route_policy.protocol_family
                ),
            });
            continue;
        }

        let capability_gaps =
            provider_capability_gaps(&candidate, &route_policy.required_capabilities);
        if !capability_gaps.is_empty() {
            excluded_candidates.push(ExcludedTarget {
                provider_resource_id: candidate.provider_resource_id.clone(),
                reason_code: format!("capability_gap_{}", capability_gaps[0]),
                reason: format!(
                    "required capabilities are not satisfied: {}",
                    capability_gaps.join(", ")
                ),
            });
            continue;
        }

        if is_health_blocked(candidate.health_state) {
            excluded_candidates.push(ExcludedTarget {
                provider_resource_id: candidate.provider_resource_id.clone(),
                reason_code: format!("health_{}", health_state_slug(candidate.health_state)),
                reason: candidate
                    .health_message
                    .clone()
                    .unwrap_or_else(|| "provider health state blocks routing".to_string()),
            });
            continue;
        }

        let preferred = route_policy
            .preferred_regions
            .iter()
            .any(|region| region == &candidate.region);
        let score_breakdown = ScoreBreakdown {
            latency: if preferred { 0.95 } else { 0.7 },
            cost: if candidate.deployment_scope == DeploymentScope::Shared {
                0.8
            } else {
                0.7
            },
            health: match candidate.health_state {
                HealthState::Healthy => 1.0,
                HealthState::Degraded => 0.6,
                HealthState::Quarantined | HealthState::Draining | HealthState::Disabled => 0.0,
            },
            trust: 1.0,
        };
        eligible_candidates.push(protocol_ir::EligibleCandidate {
            provider_resource_id: candidate.provider_resource_id.clone(),
            score_breakdown,
        });
    }

    eligible_candidates.sort_by(|left, right| {
        let left_score = left.score_breakdown.latency
            + left.score_breakdown.cost
            + left.score_breakdown.health
            + left.score_breakdown.trust;
        let right_score = right.score_breakdown.latency
            + right.score_breakdown.cost
            + right.score_breakdown.health
            + right.score_breakdown.trust;
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let selected_target = eligible_candidates
        .first()
        .map(|candidate| candidate.provider_resource_id.clone());

    Ok(RouteSimulationResponse {
        simulation_id: format!("sim_{}", request.model_alias),
        config_snapshot_id: active_snapshot.config_snapshot_id.clone(),
        admission_result: if selected_target.is_some() {
            AdmissionResult::Admitted
        } else {
            AdmissionResult::RejectedNoCandidate
        },
        eligible_candidates,
        excluded_candidates,
        selected_target,
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.000210".to_string(),
        },
    })
}

const fn protocol_family_slug(protocol_family: &ProtocolFamily) -> &'static str {
    match protocol_family {
        ProtocolFamily::OpenAiChat => "openai_chat",
        ProtocolFamily::OpenAiResponses => "openai_responses",
        ProtocolFamily::McpStreamableHttp => "mcp_streamable_http",
        ProtocolFamily::RealtimeWebRtc => "realtime_webrtc",
        ProtocolFamily::AnthropicMessages => "anthropic_messages",
        ProtocolFamily::GeminiGenerateContent => "gemini_generate_content",
    }
}

fn route_capability_supported_by_provider_capabilities(capability: &str) -> bool {
    matches!(
        capability,
        "streaming"
            | "tool_calling"
            | "tool_related"
            | "json_mode"
            | "chat_completions"
            | "realtime"
            | "response_model_metadata"
    )
}

fn route_capability_supported(capability: &str, target: &ProviderCapabilities) -> bool {
    match capability {
        "streaming" => target.supports_streaming,
        "tool_calling" | "tool_related" => target.supports_tool_calling,
        "json_mode" => target.supports_json_mode,
        "chat_completions" => true,
        "realtime" => target.supports_realtime,
        "response_model_metadata" => target.supports_response_model_metadata,
        _ => false,
    }
}

fn provider_supports_protocol_family(
    provider_resource: &ProviderResource,
    protocol_family: &str,
) -> bool {
    provider_resource
        .supported_protocol_families
        .iter()
        .any(|candidate| candidate == protocol_family)
}

fn provider_supports_capability(provider_resource: &ProviderResource, capability: &str) -> bool {
    route_capability_supported(capability, &provider_resource.capabilities)
}

fn provider_capability_gaps(
    provider_resource: &ProviderResource,
    required_capabilities: &[String],
) -> Vec<String> {
    required_capabilities
        .iter()
        .filter(|capability| !provider_supports_capability(provider_resource, capability))
        .cloned()
        .collect()
}

const fn is_health_blocked(health_state: HealthState) -> bool {
    matches!(
        health_state,
        HealthState::Quarantined | HealthState::Draining | HealthState::Disabled
    )
}

const fn health_state_slug(health_state: HealthState) -> &'static str {
    match health_state {
        HealthState::Healthy => "healthy",
        HealthState::Degraded => "degraded",
        HealthState::Quarantined => "quarantined",
        HealthState::Draining => "draining",
        HealthState::Disabled => "disabled",
    }
}

const fn admission_result_slug(admission_result: AdmissionResult) -> &'static str {
    match admission_result {
        AdmissionResult::Admitted => "admitted",
        AdmissionResult::RejectedBudget => "rejected_budget",
        AdmissionResult::RejectedRateLimit => "rejected_rate_limit",
        AdmissionResult::RejectedConcurrency => "rejected_concurrency",
        AdmissionResult::RejectedPolicy => "rejected_policy",
        AdmissionResult::RejectedNoCandidate => "rejected_no_candidate",
    }
}

fn provider_matches_filters(
    resource: &ProviderResource,
    filters: &ProviderResourceFilters,
) -> bool {
    if let Some(tenant_id) = &filters.tenant_id {
        if resource.tenant_id.as_str() != tenant_id {
            return false;
        }
    }
    if let Some(health_state) = &filters.health_state {
        if health_state_slug(resource.health_state) != health_state {
            return false;
        }
    }
    if let Some(protocol_family) = &filters.protocol_family {
        if !provider_supports_protocol_family(resource, protocol_family) {
            return false;
        }
    }
    if let Some(capability) = &filters.capability {
        if !provider_supports_capability(resource, capability) {
            return false;
        }
    }
    if let Some(transit_gateway) = filters.transit_gateway {
        if resource.is_transit_gateway != transit_gateway {
            return false;
        }
    }
    if let Some(quarantined) = filters.quarantined {
        if matches!(resource.health_state, HealthState::Quarantined) != quarantined {
            return false;
        }
    }
    true
}

fn route_receipt_matches_filters(receipt: &RouteReceipt, filters: &RouteReceiptFilters) -> bool {
    if let Some(tenant_id) = &filters.tenant_id {
        if receipt.tenant_id.as_str() != tenant_id {
            return false;
        }
    }
    if let Some(project_id) = &filters.project_id {
        if receipt.project_id.as_str() != project_id {
            return false;
        }
    }
    if let Some(protocol_family) = &filters.protocol_family {
        if receipt.protocol_family != *protocol_family {
            return false;
        }
    }
    if let Some(route_policy_id) = &filters.route_policy_id {
        if receipt.route_policy_id.as_str() != route_policy_id {
            return false;
        }
    }
    if let Some(admission_result) = &filters.admission_result {
        if admission_result_slug(receipt.admission_result) != admission_result {
            return false;
        }
    }
    if let Some(provider_resource_id) = &filters.provider_resource_id {
        let selected = receipt
            .selected_target
            .as_ref()
            .is_some_and(|target| target.as_str() == provider_resource_id);
        let excluded = receipt
            .excluded_targets
            .iter()
            .any(|target| target.provider_resource_id.as_str() == provider_resource_id);
        if !selected && !excluded {
            return false;
        }
    }
    true
}

fn to_route_receipt_summary(receipt: &RouteReceipt) -> RouteReceiptSummary {
    RouteReceiptSummary {
        route_receipt_id: receipt.route_receipt_id.clone(),
        admission_result: receipt.admission_result,
        selected_target: receipt.selected_target.clone(),
        failure_reason: receipt.failure_reason.clone(),
        created_at: receipt.created_at.clone(),
    }
}

fn build_memory_route_diagnostics(
    store: &MemoryStore,
    route_policy_id: &str,
) -> Option<RouteDiagnosticsResponse> {
    build_route_diagnostics_response(
        &store.provider_resources,
        &store.route_policies,
        &store.config_snapshots,
        &store.route_receipts.values().cloned().collect::<Vec<_>>(),
        route_policy_id,
        Some(store.active_config_snapshot_id.as_str()),
    )
}

fn build_route_diagnostics_response(
    provider_resources: &[ProviderResource],
    route_policies: &[RoutePolicy],
    config_snapshots: &[ConfigSnapshot],
    route_receipts: &[RouteReceipt],
    route_policy_id: &str,
    active_snapshot_id: Option<&str>,
) -> Option<RouteDiagnosticsResponse> {
    let route_policy = route_policies
        .iter()
        .find(|policy| policy.route_policy_id.as_str() == route_policy_id)
        .cloned()?;

    let active_snapshot = active_snapshot_id.and_then(|snapshot_id| {
        config_snapshots
            .iter()
            .find(|snapshot| snapshot.config_snapshot_id.as_str() == snapshot_id)
            .cloned()
    });
    let active_snapshot_matches_route_policy = active_snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.route_policy_id == route_policy.route_policy_id);

    let mut recent_receipts = route_receipts
        .iter()
        .filter(|receipt| receipt.route_policy_id == route_policy.route_policy_id)
        .cloned()
        .collect::<Vec<_>>();
    recent_receipts.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    let recent_receipt_summaries = recent_receipts
        .iter()
        .take(5)
        .map(to_route_receipt_summary)
        .collect::<Vec<_>>();
    let last_route_receipt = recent_receipts.first().map(to_route_receipt_summary);

    let targets = provider_resources
        .iter()
        .filter(|resource| resource.tenant_id == route_policy.tenant_id)
        .cloned()
        .map(|provider_resource| {
            let recent_receipt = recent_receipts.iter().find(|receipt| {
                receipt
                    .selected_target
                    .as_ref()
                    .is_some_and(|selected| selected == &provider_resource.provider_resource_id)
                    || receipt.excluded_targets.iter().any(|excluded| {
                        excluded.provider_resource_id == provider_resource.provider_resource_id
                    })
            });
            let recent_exclusion = recent_receipt.and_then(|receipt| {
                receipt.excluded_targets.iter().find(|excluded| {
                    excluded.provider_resource_id == provider_resource.provider_resource_id
                })
            });
            let capability_gaps =
                provider_capability_gaps(&provider_resource, &route_policy.required_capabilities);
            let supports_protocol_family = provider_supports_protocol_family(
                &provider_resource,
                &route_policy.protocol_family,
            );
            let in_active_snapshot = active_snapshot.as_ref().is_some_and(|snapshot| {
                snapshot
                    .provider_resource_ids
                    .iter()
                    .any(|id| id == &provider_resource.provider_resource_id)
            });

            let (decision, reason_code, reason, recent_receipt_reason) = if recent_receipt
                .and_then(|receipt| receipt.selected_target.as_ref())
                .is_some_and(|selected| selected == &provider_resource.provider_resource_id)
            {
                (
                    RouteDiagnosticDecision::Selected,
                    "selected_recent_receipt".to_string(),
                    "Selected by the most recent route receipt.".to_string(),
                    None,
                )
            } else if let Some(excluded) = recent_exclusion {
                (
                    RouteDiagnosticDecision::Excluded,
                    excluded.reason_code.clone(),
                    excluded.reason.clone(),
                    Some(excluded.reason.clone()),
                )
            } else if !supports_protocol_family {
                (
                    RouteDiagnosticDecision::Excluded,
                    "protocol_family_unsupported".to_string(),
                    format!(
                        "Provider does not advertise protocol family `{}`.",
                        route_policy.protocol_family
                    ),
                    None,
                )
            } else if !capability_gaps.is_empty() {
                (
                    RouteDiagnosticDecision::Excluded,
                    format!("capability_gap_{}", capability_gaps[0]),
                    format!(
                        "Missing required capabilities: {}.",
                        capability_gaps.join(", ")
                    ),
                    None,
                )
            } else if is_health_blocked(provider_resource.health_state) {
                (
                    RouteDiagnosticDecision::Excluded,
                    format!(
                        "health_{}",
                        health_state_slug(provider_resource.health_state)
                    ),
                    provider_resource
                        .health_message
                        .clone()
                        .unwrap_or_else(|| "Provider health state blocks routing.".to_string()),
                    None,
                )
            } else {
                (
                    RouteDiagnosticDecision::Eligible,
                    "eligible".to_string(),
                    "Provider satisfies current protocol, capability, and health requirements."
                        .to_string(),
                    None,
                )
            };

            RouteDiagnosticTarget {
                provider_resource,
                decision,
                in_active_snapshot,
                supports_protocol_family,
                capability_gaps,
                reason_code,
                reason,
                recent_receipt_id: recent_receipt.map(|receipt| receipt.route_receipt_id.clone()),
                recent_receipt_reason,
            }
        })
        .collect::<Vec<_>>();

    Some(RouteDiagnosticsResponse {
        route_policy,
        active_snapshot,
        active_snapshot_matches_route_policy,
        last_route_receipt,
        recent_receipts: recent_receipt_summaries,
        targets,
    })
}

fn membership(
    membership_id: &str,
    tenant: &Tenant,
    role: TenantMembershipRole,
) -> TenantMembership {
    TenantMembership {
        membership_id: TenantMembershipId::parse(membership_id.to_string()).unwrap(),
        tenant: TenantSummary {
            id: tenant.tenant_id.clone(),
            slug: tenant.slug.clone(),
            display_name: tenant.display_name.clone(),
        },
        role,
        status: TenantMembershipStatus::Active,
    }
}

fn link(
    provider: AuthProvider,
    provider_subject: &str,
    email: Option<&str>,
    can_unlink: bool,
) -> AuthProviderLink {
    AuthProviderLink {
        link_id: core_domain::AuthProviderLinkId::parse(format!("authlink_{provider_subject}"))
            .unwrap(),
        provider,
        provider_subject: provider_subject.to_string(),
        email: email.map(std::string::ToString::to_string),
        linked_at: "2026-04-22T00:00:00Z".to_string(),
        last_used_at: None,
        can_unlink,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AdmissionResult, ConcurrencyResult, ConfigSnapshotId, ExcludedTarget, ProjectId,
        ProviderResource, ProviderResourceId, RoutePolicy, RoutePolicyId, RouteReceipt,
        RouteReceiptFilters, ScoreBreakdown, StoreMode, TenantId,
    };
    use core_domain::RouteReceiptId;

    fn sample_route_receipt(
        route_receipt_id: &str,
        tenant_id: &str,
        project_id: &str,
        protocol_family: &str,
        created_at: &str,
    ) -> RouteReceipt {
        RouteReceipt {
            route_receipt_id: RouteReceiptId::parse(route_receipt_id).unwrap(),
            tenant_id: TenantId::parse(tenant_id).unwrap(),
            project_id: ProjectId::parse(project_id).unwrap(),
            route_policy_id: RoutePolicyId::parse("routepol_openai_chat_default").unwrap(),
            request_id: format!("{route_receipt_id}_request"),
            trace_id: format!("{route_receipt_id}_trace"),
            protocol_family: protocol_family.to_string(),
            model_alias: "reasoning-fast".to_string(),
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
            admission_result: AdmissionResult::Admitted,
            selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
            excluded_targets: vec![ExcludedTarget {
                provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_backup").unwrap(),
                reason_code: "sample".to_string(),
                reason: "sample".to_string(),
            }],
            failure_reason: None,
            score_breakdown: ScoreBreakdown {
                latency: 0.8,
                cost: 0.6,
                health: 1.0,
                trust: 1.0,
            },
            fallback_transitions: Vec::new(),
            normalized_error: None,
            created_at: created_at.to_string(),
        }
    }

    fn sample_provider_resource() -> ProviderResource {
        ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_cp_test_store").unwrap(),
            tenant_id: core_domain::TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(core_domain::ProjectId::parse("proj_core").unwrap()),
            provider_id: "openai".to_string(),
            name: "cp-store-provider".to_string(),
            status: core_domain::ProviderResourceStatus::Active,
            provenance_class: core_domain::ProvenanceClass::OfficialApi,
            credential_owner_type: core_domain::CredentialOwnerType::Platform,
            deployment_scope: core_domain::DeploymentScope::Shared,
            region: "us-east-1".to_string(),
            endpoint_base_url: "https://api.openai.com/v1".to_string(),
            auth_kind: core_domain::AuthKind::ApiKey,
            health_state: core_domain::HealthState::Healthy,
            health_message: Some("probe latency within SLO".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: core_domain::ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec!["openai_chat".to_string()],
            is_transit_gateway: false,
            version: 1,
            created_at: "2026-04-22T00:00:00Z".to_string(),
            updated_at: "2026-04-22T00:00:00Z".to_string(),
        }
    }

    fn sample_route_policy() -> RoutePolicy {
        RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_cp_store".to_string()).unwrap(),
            tenant_id: core_domain::TenantId::parse("tenant_acme").unwrap(),
            display_name: "store-route-policy".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-east-1".to_string()],
            version: 1,
            created_at: "2026-04-22T00:00:00Z".to_string(),
            updated_at: "2026-04-22T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn memory_store_enforces_version_for_provider_resource_updates() {
        let store = StoreMode::memory();
        let created = store
            .create_provider_resource(sample_provider_resource())
            .await
            .unwrap();
        assert_eq!(created.version, 1);

        let mut stale_update = created.clone();
        stale_update.name = "stale-update".to_string();
        let stale = store
            .update_provider_resource(stale_update, 0)
            .await
            .unwrap();
        assert!(matches!(stale, ConcurrencyResult::VersionConflict));

        let mut update = created.clone();
        update.name = "updated-name".to_string();
        let applied = store.update_provider_resource(update, 1).await.unwrap();
        match applied {
            ConcurrencyResult::Applied(resource) => assert_eq!(resource.version, 2),
            _ => panic!("expected applied update"),
        }
    }

    #[tokio::test]
    async fn memory_store_route_policy_disable_is_filterable() {
        let store = StoreMode::memory();
        let created = store
            .create_route_policy(sample_route_policy())
            .await
            .unwrap();
        let before = store.list_route_policies().await.unwrap().data;
        let before_len = before.len();
        let applied = store
            .disable_route_policy(&created.route_policy_id.to_string(), 1)
            .await;
        let applied = match applied.unwrap() {
            ConcurrencyResult::Applied(route_policy) => route_policy,
            other => panic!("expected applied route policy: {other:?}"),
        };
        assert_eq!(applied.version, 2);
        let after = store.list_route_policies().await.unwrap().data;
        assert_eq!(after.len(), before_len - 1);
        assert!(
            !after
                .iter()
                .any(|policy| policy.route_policy_id == created.route_policy_id)
        );
    }

    #[tokio::test]
    async fn memory_store_only_persists_api_key_hash() {
        let store = StoreMode::memory();
        let api_key = "akp_store_secret_value_123";
        let created = store
            .create_api_key(
                ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
                "store-api-key",
                api_key,
            )
            .await
            .unwrap();
        assert_eq!(created.key_prefix, "akp_st...");
        assert_eq!(created.version, 1);

        let keys = store.list_api_keys().await.unwrap().data;
        assert_eq!(keys.len(), 1);

        let store = &store;
        if let StoreMode::Memory(memory) = store {
            let (hash, api_key_id) = {
                let memory = memory.read().unwrap();
                let keys: Vec<_> = memory.api_keys.iter().collect();
                assert_eq!(keys.len(), 1);
                let stored = &keys[0];
                assert_ne!(stored.hash, api_key);
                assert_eq!(stored.hash.len(), 64);
                assert_eq!(
                    stored.provider_resource_id,
                    ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()
                );
                assert_eq!(stored.display_name, "store-api-key");
                assert_eq!(stored.key_prefix, "akp_st...");
                (stored.hash.clone(), stored.api_key_id.clone())
            };
            let revoked = store.revoke_api_key(&api_key_id, 1).await.unwrap();
            assert!(matches!(revoked, ConcurrencyResult::Applied(_)));
            assert_ne!(hash, api_key);
        } else {
            panic!("expected memory store");
        }
    }

    #[tokio::test]
    async fn memory_store_list_route_receipts_supports_filters_and_recent_order() {
        let store = StoreMode::memory();
        store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_store_a",
            "tenant_acme",
            "proj_core",
            "openai_chat",
            "2026-04-22T00:01:00Z",
        ));
        store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_store_b",
            "tenant_acme",
            "proj_core",
            "openai_chat",
            "2026-04-22T00:03:00Z",
        ));
        store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_store_c",
            "tenant_platform",
            "proj_core",
            "openai_responses",
            "2026-04-22T00:02:00Z",
        ));

        let all = store
            .list_route_receipts(&RouteReceiptFilters::default())
            .await
            .unwrap()
            .data;
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].route_receipt_id.as_str(), "routercpt_store_b");
        assert_eq!(all[1].route_receipt_id.as_str(), "routercpt_store_c");
        assert_eq!(all[2].route_receipt_id.as_str(), "routercpt_store_a");

        let tenant_filtered = store
            .list_route_receipts(&RouteReceiptFilters {
                tenant_id: Some("tenant_acme".to_string()),
                ..RouteReceiptFilters::default()
            })
            .await
            .unwrap()
            .data;
        assert_eq!(tenant_filtered.len(), 2);
        assert_eq!(
            tenant_filtered[0].route_receipt_id.as_str(),
            "routercpt_store_b"
        );

        let protocol_filtered = store
            .list_route_receipts(&RouteReceiptFilters {
                protocol_family: Some("openai_chat".to_string()),
                ..RouteReceiptFilters::default()
            })
            .await
            .unwrap()
            .data;
        assert_eq!(protocol_filtered.len(), 2);
    }

    #[tokio::test]
    async fn memory_store_activation_supersedes_previous_project_snapshot() {
        let store = StoreMode::memory();
        let newer = core_domain::ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_gateway_v2").unwrap(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            revision: 2,
            status: core_domain::ConfigSnapshotStatus::Draft,
            activated_at: None,
            provider_resource_ids: vec![
                ProviderResourceId::parse("prvrsrc_openai_backup").unwrap(),
            ],
            route_policy_id: RoutePolicyId::parse("routepol_acme_support").unwrap(),
            budget_policy_id: core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap(),
        };
        let created = store.create_config_snapshot(newer).await.unwrap();
        assert_eq!(created.config_snapshot_id.as_str(), "cfgsnap_gateway_v2");

        let activated = store
            .activate_config_snapshot("cfgsnap_gateway_v2")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            activated.config_snapshot.config_snapshot_id.as_str(),
            "cfgsnap_gateway_v2"
        );

        let snapshots = store.list_config_snapshots().await.unwrap().data;
        let gateway_v1 = snapshots
            .iter()
            .find(|snapshot| snapshot.config_snapshot_id.as_str() == "cfgsnap_gateway_v1")
            .unwrap();
        let gateway_v2 = snapshots
            .iter()
            .find(|snapshot| snapshot.config_snapshot_id.as_str() == "cfgsnap_gateway_v2")
            .unwrap();
        assert_eq!(
            gateway_v1.status,
            core_domain::ConfigSnapshotStatus::Superseded
        );
        assert_eq!(gateway_v2.status, core_domain::ConfigSnapshotStatus::Active);
    }
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn expires_at(seconds: u64) -> String {
    let seconds_i64 = i64::try_from(seconds).unwrap_or(i64::MAX);
    let timestamp = OffsetDateTime::now_utc() + time::Duration::seconds(seconds_i64);
    timestamp
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn timestamp_is_expired(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|timestamp| timestamp <= OffsetDateTime::now_utc())
        .unwrap_or(true)
}

fn resolve_active_tenant_id(
    existing: Option<&TenantId>,
    memberships: &[TenantMembership],
) -> Option<TenantId> {
    existing
        .filter(|tenant_id| {
            memberships
                .iter()
                .any(|membership| membership.tenant.id == **tenant_id)
        })
        .cloned()
        .or_else(|| {
            memberships
                .iter()
                .find(|membership| membership.status == TenantMembershipStatus::Active)
                .map(|membership| membership.tenant.id.clone())
        })
}

fn env_flag_enabled(name: &str, default: bool) -> bool {
    std::env::var(name).ok().map_or(default, |value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub fn mock_auth_enabled() -> bool {
    env_flag_enabled("CONTROL_PLANE_ALLOW_MOCK_AUTH", false)
}

pub fn auth_provider_enabled(provider: AuthProvider) -> bool {
    match provider {
        AuthProvider::Email => env_flag_enabled("CONTROL_PLANE_AUTH_EMAIL_ENABLED", true),
        AuthProvider::Github => {
            env_flag_enabled("CONTROL_PLANE_AUTH_GITHUB_ENABLED", false) || mock_auth_enabled()
        }
        AuthProvider::Google => {
            env_flag_enabled("CONTROL_PLANE_AUTH_GOOGLE_ENABLED", false) || mock_auth_enabled()
        }
        AuthProvider::Wechat => {
            env_flag_enabled("CONTROL_PLANE_AUTH_WECHAT_ENABLED", false) || mock_auth_enabled()
        }
        AuthProvider::Oidc => oidc_enabled() || mock_auth_enabled(),
    }
}

pub const fn oauth_provider_slug(provider: OAuthProvider) -> &'static str {
    match provider {
        OAuthProvider::Github => "github",
        OAuthProvider::Google => "google",
        OAuthProvider::Wechat => "wechat",
        OAuthProvider::Oidc => "oidc",
    }
}

pub const fn auth_provider_slug(provider: AuthProvider) -> &'static str {
    match provider {
        AuthProvider::Email => "email",
        AuthProvider::Github => "github",
        AuthProvider::Google => "google",
        AuthProvider::Wechat => "wechat",
        AuthProvider::Oidc => "oidc",
    }
}

pub fn oidc_enabled() -> bool {
    std::env::var("CONTROL_PLANE_OIDC_ISSUER_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
        || std::env::var("CONTROL_PLANE_OIDC_AUTHORIZATION_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_some()
}

const fn login_flow_kind_slug(kind: LoginFlowKind) -> &'static str {
    match kind {
        LoginFlowKind::Email => "email",
        LoginFlowKind::OAuth => "oauth",
    }
}

const fn config_snapshot_status_slug(status: ConfigSnapshotStatus) -> &'static str {
    match status {
        ConfigSnapshotStatus::Draft => "draft",
        ConfigSnapshotStatus::Active => "active",
        ConfigSnapshotStatus::Superseded => "superseded",
    }
}

const REQUIRED_TABLES: &[&str] = &[
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
    "route_receipts",
    "billing_export_jobs",
];

const MIGRATIONS: &[&str] = &[
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
