#![allow(
    clippy::assigning_clones,
    clippy::branches_sharing_code,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::items_after_test_module,
    clippy::match_like_matches_macro,
    clippy::needless_pass_by_value,
    clippy::option_as_ref_deref,
    clippy::or_fun_call,
    clippy::redundant_closure,
    clippy::significant_drop_tightening,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]

use anyhow::{Context, Result, anyhow};
use core_domain::{
    AdmissionResult, AuthKind, AuthLoginResult, AuthProvider, AuthProviderAvailability,
    AuthProviderLink, AuthSession, AuthSessionId, AuthSessionState, BudgetPolicyId, ConfigSnapshot,
    ConfigSnapshotId, ConfigSnapshotStatus, CredentialOwnerType, DeploymentScope, ExcludedTarget,
    HealthState, LogoutResponse, MonetaryAmount, OAuthProvider, Project, ProjectId,
    ProvenanceClass, ProviderCapabilities, ProviderResource, ProviderResourceId,
    ProviderResourceStatus, RoutePolicy, RoutePolicyId, RouteReceipt, ScoreBreakdown, Tenant,
    TenantId, TenantMembership, TenantMembershipId, TenantMembershipRole, TenantMembershipStatus,
    TenantSummary, UnlinkAuthProviderResponse, UserId, UserIdentity,
};
use protocol_ir::{
    ConfigSnapshotResponse, ProjectsResponse, ProviderResourcesResponse, RoutePoliciesResponse,
    RouteReceiptResponse, RouteSimulationRequest, RouteSimulationResponse, TenantsResponse,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres, Row, postgres::PgPoolOptions, types::Json};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const EMAIL_BOOTSTRAP_CODE: &str = "111111";
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
    active_config_snapshot_id: String,
    users: HashMap<String, UserIdentity>,
    memberships_by_user: HashMap<String, Vec<TenantMembership>>,
    provider_links_by_user: HashMap<String, Vec<AuthProviderLink>>,
    email_identity_to_user_id: HashMap<String, String>,
    provider_subject_to_user_id: HashMap<String, String>,
    sessions: HashMap<String, StoredSession>,
    login_flows: HashMap<String, LoginFlow>,
    route_receipts: HashMap<String, RouteReceipt>,
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
    pub active_config_snapshot_id: String,
    pub users: Vec<UserSeed>,
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
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
            },
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
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
            },
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
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_json_mode: true,
            },
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
        let seed = SeedData::bootstrap();
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
            active_config_snapshot_id: seed.active_config_snapshot_id,
            users,
            memberships_by_user,
            provider_links_by_user,
            email_identity_to_user_id,
            provider_subject_to_user_id,
            sessions: HashMap::new(),
            login_flows: HashMap::new(),
            route_receipts: HashMap::new(),
            route_policy_disabled_ids: HashSet::new(),
            api_keys: Vec::new(),
        }
    }
}

impl StoreMode {
    pub fn memory() -> Self {
        Self::Memory(Arc::new(RwLock::new(MemoryStore::bootstrap())))
    }

    pub async fn from_env() -> Result<Self> {
        match std::env::var("CONTROL_PLANE_DATABASE_URL") {
            Ok(database_url) => Ok(Self::Postgres(PostgresStore::connect(&database_url).await?)),
            Err(_) => Ok(Self::memory()),
        }
    }

    pub fn provider_catalog() -> Vec<AuthProviderAvailability> {
        PROVIDER_CATALOG
            .iter()
            .map(
                |(provider, display_name, start_path)| AuthProviderAvailability {
                    provider: *provider,
                    display_name: (*display_name).to_string(),
                    enabled: true,
                    start_path: (*start_path).to_string(),
                    reason_code: None,
                },
            )
            .collect()
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
        expires_at: &str,
    ) -> Result<LoginFlow> {
        let flow = LoginFlow {
            flow_id: flow_id.to_string(),
            flow_kind: LoginFlowKind::Email,
            email: Some(email.to_lowercase()),
            provider: None,
            workspace_slug: workspace_slug.to_string(),
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
        match self {
            Self::Memory(store) => Ok(store
                .write()
                .expect("memory store write lock")
                .login_flows
                .remove(flow_id)),
            Self::Postgres(store) => store.consume_login_flow(flow_id).await,
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

    pub async fn get_session(&self, session_id: &str) -> Result<Option<AuthLoginResult>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .sessions
                .get(session_id)
                .map(|session| AuthLoginResult {
                    session: session.session.clone(),
                    links: session.links.clone(),
                })),
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

    pub async fn list_provider_resources(&self) -> Result<ProviderResourcesResponse> {
        match self {
            Self::Memory(store) => Ok(ProviderResourcesResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .provider_resources
                    .clone(),
            }),
            Self::Postgres(store) => store.list_provider_resources().await,
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
        tenant_id: Option<String>,
        project_id: Option<String>,
        protocol_family: Option<String>,
    ) -> Result<RouteReceiptsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(RouteReceiptsResponse {
                    data: filter_and_order_route_receipts(
                        store.route_receipts.values().cloned().collect::<Vec<_>>(),
                        tenant_id,
                        project_id,
                        protocol_family,
                    ),
                })
            }
            Self::Postgres(store) => {
                store
                    .list_route_receipts(tenant_id, project_id, protocol_family)
                    .await
            }
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
            }
            Self::Postgres(_) => panic!("test helper only supports memory store"),
        }
    }
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
        let store = Self { pool };
        store.migrate().await?;
        store.seed().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<()> {
        for statement in MIGRATIONS {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    async fn seed(&self) -> Result<()> {
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
        let user = self
            .lookup_user(identity_key)
            .await?
            .context("identity not found")?;
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

    async fn get_session(&self, session_id: &str) -> Result<Option<AuthLoginResult>> {
        let row =
            sqlx::query("SELECT payload FROM sessions WHERE session_id = $1 AND state = 'active'")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let session = row.get::<Json<AuthSession>, _>("payload").0;
        let links = self.list_links(&session.user.user_id).await?;
        Ok(Some(AuthLoginResult { session, links }))
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
        Ok(Some(UnlinkAuthProviderResponse { provider, removed }))
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

    async fn list_provider_resources(&self) -> Result<ProviderResourcesResponse> {
        let rows =
            sqlx::query("SELECT payload FROM provider_resources ORDER BY provider_resource_id")
                .fetch_all(&self.pool)
                .await?;
        Ok(ProviderResourcesResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<ProviderResource>, _>("payload").0)
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
        snapshot.status = ConfigSnapshotStatus::Active;
        snapshot.activated_at = Some(now_rfc3339());
        sqlx::query(
            "UPDATE config_snapshots SET status = 'active', payload = $2 WHERE config_snapshot_id = $1",
        )
        .bind(config_snapshot_id)
        .bind(Json(snapshot.clone()))
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT INTO active_config_pointers (pointer_key, config_snapshot_id)
             VALUES ('default', $1)
             ON CONFLICT (pointer_key) DO UPDATE SET config_snapshot_id = EXCLUDED.config_snapshot_id",
        )
        .bind(config_snapshot_id)
        .execute(&self.pool)
        .await?;
        Ok(Some(ConfigSnapshotResponse {
            config_snapshot: snapshot,
        }))
    }

    async fn simulate_route(
        &self,
        request: RouteSimulationRequest,
    ) -> Result<RouteSimulationResponse> {
        let tenants = self.list_provider_resources().await?.data;
        let policies = self.list_route_policies().await?.data;
        let active_snapshot = self
            .get_config_snapshot(ACTIVE_CONFIG_ALIAS)
            .await?
            .context("active config snapshot missing")?
            .config_snapshot;
        build_route_simulation_response(&tenants, &policies, &active_snapshot, &request)
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
        tenant_id: Option<String>,
        project_id: Option<String>,
        protocol_family: Option<String>,
    ) -> Result<RouteReceiptsResponse> {
        let rows = sqlx::query("SELECT payload FROM route_receipts")
            .fetch_all(&self.pool)
            .await?;
        Ok(RouteReceiptsResponse {
            data: filter_and_order_route_receipts(
                rows.into_iter()
                    .map(|row| row.get::<Json<RouteReceipt>, _>("payload").0)
                    .collect(),
                tenant_id,
                project_id,
                protocol_family,
            ),
        })
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
    }
    .context("identity not found")?;

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

fn unlink_memory_provider(
    store: &mut MemoryStore,
    session_id: &str,
    provider: AuthProvider,
) -> Option<UnlinkAuthProviderResponse> {
    let session = store.sessions.get_mut(session_id)?;
    let removed = if provider == AuthProvider::Email {
        false
    } else {
        let before = session.links.len();
        session.links.retain(|link| link.provider != provider);
        if let Some(user_links) = store
            .provider_links_by_user
            .get_mut(session.session.user.user_id.as_str())
        {
            user_links.retain(|link| link.provider != provider);
        }
        session.links.len() != before
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

fn activate_memory_config_snapshot(
    store: &mut MemoryStore,
    config_snapshot_id: &str,
) -> Option<ConfigSnapshotResponse> {
    let index = store
        .config_snapshots
        .iter()
        .position(|item| item.config_snapshot_id.as_str() == config_snapshot_id)?;
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
        .find(|snapshot| snapshot.config_snapshot_id.as_str() == store.active_config_snapshot_id)
        .cloned()
        .context("active config snapshot missing")?;
    build_route_simulation_response(
        &store.provider_resources,
        &store.route_policies,
        &active_snapshot,
        request,
    )
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
                reason: format!("provider status is {:?}", candidate.status),
            });
            continue;
        }
        if !resource_protocol_matches(&candidate.provider_id, &route_policy.protocol_family) {
            excluded_candidates.push(ExcludedTarget {
                provider_resource_id: candidate.provider_resource_id.clone(),
                reason: "provider protocol is incompatible with route policy".to_string(),
            });
            continue;
        }

        if !route_policy
            .required_capabilities
            .iter()
            .all(|capability| route_capability_supported(capability, &candidate.capabilities))
        {
            excluded_candidates.push(ExcludedTarget {
                provider_resource_id: candidate.provider_resource_id.clone(),
                reason: "required capabilities are not satisfied by the target".to_string(),
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

const fn protocol_family_slug(protocol_family: &protocol_ir::ProtocolFamily) -> &'static str {
    match protocol_family {
        protocol_ir::ProtocolFamily::OpenAiChat => "openai_chat",
        protocol_ir::ProtocolFamily::OpenAiResponses => "openai_responses",
        protocol_ir::ProtocolFamily::McpStreamableHttp => "mcp_streamable_http",
        protocol_ir::ProtocolFamily::RealtimeWebRtc => "realtime_webrtc",
        protocol_ir::ProtocolFamily::AnthropicMessages => "anthropic_messages",
        protocol_ir::ProtocolFamily::GeminiGenerateContent => "gemini_generate_content",
    }
}

fn route_capability_supported_by_provider_capabilities(capability: &str) -> bool {
    matches!(
        capability,
        "streaming" | "tool_calling" | "json_mode" | "chat_completions"
    )
}

fn route_capability_supported(capability: &str, target: &ProviderCapabilities) -> bool {
    match capability {
        "streaming" => target.supports_streaming,
        "tool_calling" => target.supports_tool_calling,
        "json_mode" => target.supports_json_mode,
        "chat_completions" => true,
        _ => false,
    }
}

fn resource_protocol_matches(provider_id: &str, protocol_family: &str) -> bool {
    match (provider_id, protocol_family) {
        ("openai", "openai_chat" | "openai_responses") => true,
        _ => false,
    }
}

fn route_receipt_created_at(route_receipt: &RouteReceipt) -> OffsetDateTime {
    OffsetDateTime::parse(&route_receipt.created_at, &Rfc3339).unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

fn filter_and_order_route_receipts(
    mut route_receipts: Vec<RouteReceipt>,
    tenant_id: Option<String>,
    project_id: Option<String>,
    protocol_family: Option<String>,
) -> Vec<RouteReceipt> {
    route_receipts.retain(|receipt| {
        if let Some(tenant_id) = tenant_id.as_deref() {
            if receipt.tenant_id.as_str() != tenant_id {
                return false;
            }
        }
        if let Some(project_id) = project_id.as_deref() {
            if receipt.project_id.as_str() != project_id {
                return false;
            }
        }
        if let Some(protocol_family) = protocol_family.as_deref() {
            if receipt.protocol_family != protocol_family {
                return false;
            }
        }
        true
    });

    route_receipts.sort_by(|left, right| {
        route_receipt_created_at(right)
            .cmp(&route_receipt_created_at(left))
            .then_with(|| {
                right
                    .route_receipt_id
                    .as_str()
                    .cmp(left.route_receipt_id.as_str())
            })
    });
    route_receipts
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

pub fn ensure_workspace_slug(workspace_slug: &str) -> bool {
    matches!(
        workspace_slug,
        "platform-admin" | "acme-retail" | "northstar-labs"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        AdmissionResult, ConcurrencyResult, ConfigSnapshotId, ExcludedTarget, ProjectId,
        ProviderResource, ProviderResourceId, RoutePolicy, RoutePolicyId, RouteReceipt,
        ScoreBreakdown, StoreMode, TenantId,
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
            request_id: format!("{route_receipt_id}_request"),
            trace_id: format!("{route_receipt_id}_trace"),
            protocol_family: protocol_family.to_string(),
            model_alias: "reasoning-fast".to_string(),
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
            admission_result: AdmissionResult::Admitted,
            selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
            excluded_targets: vec![ExcludedTarget {
                provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_backup").unwrap(),
                reason: "sample".to_string(),
            }],
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
            budget_policy_id: None,
            capabilities: core_domain::ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
            },
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
            .list_route_receipts(None, None, None)
            .await
            .unwrap()
            .data;
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].route_receipt_id.as_str(), "routercpt_store_b");
        assert_eq!(all[1].route_receipt_id.as_str(), "routercpt_store_c");
        assert_eq!(all[2].route_receipt_id.as_str(), "routercpt_store_a");

        let tenant_filtered = store
            .list_route_receipts(Some("tenant_acme".to_string()), None, None)
            .await
            .unwrap()
            .data;
        assert_eq!(tenant_filtered.len(), 2);
        assert_eq!(
            tenant_filtered[0].route_receipt_id.as_str(),
            "routercpt_store_b"
        );

        let protocol_filtered = store
            .list_route_receipts(None, None, Some("openai_chat".to_string()))
            .await
            .unwrap()
            .data;
        assert_eq!(protocol_filtered.len(), 2);
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

pub const fn oauth_provider_slug(provider: OAuthProvider) -> &'static str {
    match provider {
        OAuthProvider::Github => "github",
        OAuthProvider::Google => "google",
        OAuthProvider::Wechat => "wechat",
    }
}

pub const fn auth_provider_slug(provider: AuthProvider) -> &'static str {
    match provider {
        AuthProvider::Email => "email",
        AuthProvider::Github => "github",
        AuthProvider::Google => "google",
        AuthProvider::Wechat => "wechat",
    }
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
];
