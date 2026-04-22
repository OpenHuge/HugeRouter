use anyhow::{Context, Result};
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
use sqlx::{Pool, Postgres, Row, postgres::PgPoolOptions, types::Json};
use std::{
    collections::HashMap,
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

    pub async fn list_route_policies(&self) -> Result<RoutePoliciesResponse> {
        match self {
            Self::Memory(store) => Ok(RoutePoliciesResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .route_policies
                    .clone(),
            }),
            Self::Postgres(store) => store.list_route_policies().await,
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

    async fn list_route_policies(&self) -> Result<RoutePoliciesResponse> {
        let rows = sqlx::query("SELECT payload FROM route_policies ORDER BY route_policy_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(RoutePoliciesResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<RoutePolicy>, _>("payload").0)
                .collect(),
        })
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
    let route_policy = route_policies
        .iter()
        .find(|policy| policy.route_policy_id == active_snapshot.route_policy_id)
        .cloned()
        .context("route policy missing for snapshot")?;

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
        if !candidate.capabilities.supports_json_mode
            && route_policy
                .required_capabilities
                .iter()
                .any(|capability| capability == "json_mode")
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
