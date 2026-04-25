#![allow(clippy::too_many_lines, clippy::uninlined_format_args)]

mod merchant_api;
mod merchant_replay;
mod merchant_store;
mod pricing_catalog;
mod route_receipts;
mod store;
mod store_schema;

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue, Method,
        header::{AUTHORIZATION, COOKIE, SET_COOKIE},
    },
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use core_domain::{
    AuthKind, AuthProvider, AuthProviderLinksResponse, AuthSessionResponse, ConfigSnapshot,
    CredentialOwnerType, DeploymentScope, EmailLoginCompleteRequest, EmailLoginStartRequest,
    EmailLoginStartResponse, HealthState, OAuthCallbackRequest, OAuthLoginStartRequest,
    OAuthLoginStartResponse, Project, ProvenanceClass, ProviderCapabilities, ProviderResource,
    ProviderResourceId, ProviderResourceStatus, RoutePolicy, RoutePolicyId, Tenant,
    TenantMembership, TenantMembershipRole, TenantMembershipStatus, UnlinkAuthProviderResponse,
};
use protocol_ir::{
    BalanceProjectionResponse, BillingExportJobResponse, BillingExportJobsResponse,
    BillingExportRequest, ConfigSnapshotResponse, PricingCatalogResponse, PricingSimulationRequest,
    PricingSimulationResponse, ProjectsResponse, ProviderResourcesResponse,
    RouteDiagnosticsResponse, RoutePoliciesResponse, RouteSimulationRequest,
    RouteSimulationResponse, TenantsResponse, UsageBreakdownResponse, UsageSummaryResponse,
};
use reqwest::Client as HttpClient;
use ring::{aead, rand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use store::{
    ApiKey, ApiKeysResponse, CodexAuthAccount, CodexAuthAccountRecord, CodexAuthAccountsResponse,
    ConcurrencyResult, ConfigSnapshotsResponse, EncryptedSecretBlob, IdentityLookup,
    OAuthCarpoolRecord, OAuthCarpoolsResponse, OAuthPoolSelectionRequest, OAuthSharingLeaseRecord,
    OAuthSharingLeasesResponse, OAuthSharingUsageBudget, OAuthSharingUsageFilters,
    OAuthSharingUsageResponse, ProviderResourceFilters, SESSION_TTL_SECONDS, StoreMode,
    auth_provider_enabled, expires_at, mock_auth_enabled, now_rfc3339, oauth_provider_slug,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

const CONTROL_PLANE_SERVICE_NAME: &str = "control-plane-api";
const FRONTEND_BASE_URL: &str = "http://127.0.0.1:3000";
const PLATFORM_ADMIN_TENANT_SLUG: &str = "platform-admin";
const SESSION_COOKIE_NAME: &str = "huge_router_session";
const CODEX_AUTH_ENCRYPTION_ALGORITHM: &str = "AES-256-GCM";
const DEFAULT_CODEX_REVERSE_PROXY_ENDPOINT: &str = "https://chatgpt-reverse-proxy.local/v1";

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(10_000);

#[derive(Clone, Debug)]
pub struct ControlPlaneState {
    frontend_base_url: String,
    internal_gateway_token: Option<String>,
    pub(crate) store: StoreMode,
}

impl ControlPlaneState {
    /// # Errors
    ///
    /// Returns an error when the configured persistent store cannot be initialized.
    pub async fn from_env() -> Result<Self> {
        Ok(Self {
            frontend_base_url: std::env::var("CONSOLE_WEB_BASE_URL")
                .unwrap_or_else(|_| FRONTEND_BASE_URL.to_string()),
            internal_gateway_token: std::env::var("CONTROL_PLANE_INTERNAL_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            store: StoreMode::from_env().await?,
        })
    }

    #[cfg(test)]
    fn memory() -> Self {
        Self {
            frontend_base_url: FRONTEND_BASE_URL.to_string(),
            internal_gateway_token: Some("test-internal-token".to_string()),
            store: StoreMode::memory(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConcurrencyRequest {
    pub expected_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderResourceUpdateRequest {
    #[serde(flatten)]
    pub provider_resource: ProviderResource,
    pub expected_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoutePolicyUpdateRequest {
    #[serde(flatten)]
    pub route_policy: RoutePolicy,
    pub expected_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateApiKeyRequest {
    pub provider_resource_id: String,
    pub display_name: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayApiKeyResolveRequest {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize)]
struct GatewayApiKeyResolveResponse {
    pub credential_id: String,
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexAuthAccountUploadRequest {
    pub display_name: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub provider_resource_id: Option<String>,
    #[serde(default)]
    pub endpoint_base_url: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    pub auth_json: Value,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexAuthAccountLeaseRequest {
    #[serde(default)]
    pub provider_resource_id: Option<String>,
    #[serde(default)]
    pub pool_id: Option<String>,
    #[serde(default)]
    pub lease_id: Option<String>,
    #[serde(default)]
    pub carpool_id: Option<String>,
    #[serde(default)]
    pub borrower_workspace_id: Option<String>,
    #[serde(default)]
    pub borrower_user_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CodexAuthAccountLeaseResponse {
    pub codex_account_id: String,
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub provider_resource_id: String,
    pub display_name: String,
    pub leased_until: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carpool_id: Option<String>,
    pub auth_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthSharingLeaseUpsertRequest {
    pub lease_id: String,
    #[serde(default)]
    pub owner_workspace_id: Option<String>,
    #[serde(default)]
    pub owner_user_id: Option<String>,
    pub borrower_workspace_id: String,
    #[serde(default)]
    pub borrower_user_id: Option<String>,
    pub provider: String,
    pub pool_id: String,
    #[serde(default)]
    pub allowed_account_ids: Option<Vec<String>>,
    pub status: String,
    pub starts_at: String,
    pub expires_at: String,
    pub max_concurrent_runs: u32,
    #[serde(default)]
    pub usage_budget: OAuthSharingUsageBudget,
    pub policy: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthCarpoolUpsertRequest {
    pub carpool_id: String,
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub member_workspace_ids: Vec<String>,
    #[serde(default)]
    pub pool_ids: Vec<String>,
    pub strategy: String,
    #[serde(default)]
    pub member_weights: std::collections::BTreeMap<String, u32>,
    #[serde(default)]
    pub per_member_concurrency_limit: Option<u32>,
    #[serde(default)]
    pub per_member_turn_budget: Option<u64>,
    pub enabled: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct OAuthSharingUsageQuery {
    #[serde(default)]
    pub lease_id: Option<String>,
    #[serde(default)]
    pub carpool_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InternalGatewayConfigResponse {
    pub config_snapshot: ConfigSnapshot,
    pub route_policy: RoutePolicy,
    pub provider_resources: Vec<ProviderResource>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageQuery {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
    pub window_start: Option<String>,
    pub window_end: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageBreakdownQuery {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
    pub window_start: Option<String>,
    pub window_end: Option<String>,
    pub group_by: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct BalanceProjectionQuery {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BillingExportsQuery {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone)]
struct OidcIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone)]
struct OAuthIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ErrorEnvelope {
    code: &'static str,
    message: String,
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "traceId")]
    trace_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ApiError {
    code: &'static str,
    message: String,
    request_id: String,
    status: axum::http::StatusCode,
    trace_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RequestContext {
    request_id: String,
    trace_id: String,
    sequence: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ControlPlaneAuthorizer {
    session: core_domain::AuthLoginResult,
}

impl ControlPlaneAuthorizer {
    const fn new(session: core_domain::AuthLoginResult) -> Self {
        Self { session }
    }

    fn is_platform_admin(&self) -> bool {
        self.membership_by_slug(PLATFORM_ADMIN_TENANT_SLUG)
            .is_some_and(|membership| membership_can_manage(membership.role))
    }

    pub(crate) fn ensure_read_tenant(
        &self,
        tenant_id: &str,
        context: &RequestContext,
    ) -> Result<(), ApiError> {
        if self.is_platform_admin() || self.membership(tenant_id).is_some() {
            Ok(())
        } else {
            Err(tenant_access_denied("access", tenant_id, context))
        }
    }

    pub(crate) fn ensure_manage_tenant(
        &self,
        tenant_id: &str,
        context: &RequestContext,
    ) -> Result<(), ApiError> {
        if self.is_platform_admin()
            || self
                .membership(tenant_id)
                .is_some_and(|membership| membership_can_manage(membership.role))
        {
            Ok(())
        } else {
            Err(tenant_access_denied("manage", tenant_id, context))
        }
    }

    fn ensure_read_project(
        &self,
        project: &Project,
        context: &RequestContext,
    ) -> Result<(), ApiError> {
        self.ensure_read_tenant(project.tenant_id.as_str(), context)
    }

    fn ensure_manage_project(
        &self,
        project: &Project,
        context: &RequestContext,
    ) -> Result<(), ApiError> {
        self.ensure_manage_tenant(project.tenant_id.as_str(), context)
    }

    fn filter_tenants(&self, tenants: Vec<Tenant>) -> Vec<Tenant> {
        self.filter_by_tenant(tenants, |tenant| tenant.tenant_id.as_str())
    }

    fn filter_projects(&self, projects: Vec<Project>) -> Vec<Project> {
        self.filter_by_tenant(projects, |project| project.tenant_id.as_str())
    }

    fn filter_provider_resources(
        &self,
        provider_resources: Vec<ProviderResource>,
    ) -> Vec<ProviderResource> {
        self.filter_by_tenant(provider_resources, |resource| resource.tenant_id.as_str())
    }

    fn filter_route_policies(&self, route_policies: Vec<RoutePolicy>) -> Vec<RoutePolicy> {
        self.filter_by_tenant(route_policies, |policy| policy.tenant_id.as_str())
    }

    fn filter_config_snapshots(
        &self,
        config_snapshots: Vec<ConfigSnapshot>,
    ) -> Vec<ConfigSnapshot> {
        self.filter_by_tenant(config_snapshots, |snapshot| snapshot.tenant_id.as_str())
    }

    fn filter_route_receipts(
        &self,
        route_receipts: Vec<core_domain::RouteReceipt>,
    ) -> Vec<core_domain::RouteReceipt> {
        self.filter_by_tenant(route_receipts, |receipt| receipt.tenant_id.as_str())
    }

    fn filter_by_tenant<T>(&self, items: Vec<T>, tenant_id: impl Fn(&T) -> &str) -> Vec<T> {
        if self.is_platform_admin() {
            items
        } else {
            items
                .into_iter()
                .filter(|item| self.membership(tenant_id(item)).is_some())
                .collect()
        }
    }

    fn membership(&self, tenant_id: &str) -> Option<&TenantMembership> {
        self.session.session.memberships.iter().find(|membership| {
            membership.status == TenantMembershipStatus::Active
                && membership.tenant.id.as_str() == tenant_id
        })
    }

    fn membership_by_slug(&self, tenant_slug: &str) -> Option<&TenantMembership> {
        self.session.session.memberships.iter().find(|membership| {
            membership.status == TenantMembershipStatus::Active
                && membership.tenant.slug == tenant_slug
        })
    }

    pub(crate) fn active_tenant_id(&self) -> Option<&str> {
        self.session
            .session
            .active_tenant_id
            .as_ref()
            .map(core_domain::TenantId::as_str)
            .or_else(|| {
                self.session
                    .session
                    .memberships
                    .iter()
                    .find(|membership| membership.status == TenantMembershipStatus::Active)
                    .map(|membership| membership.tenant.id.as_str())
            })
    }
}

/// # Errors
///
/// Returns an error when the configured control-plane state cannot be initialized.
pub async fn app() -> Result<Router> {
    Ok(app_with_state(ControlPlaneState::from_env().await?))
}

/// # Errors
///
/// Returns an error when the configured database cannot be migrated.
pub async fn migrate() -> Result<()> {
    store::migrate_from_env().await
}

/// # Errors
///
/// Returns an error when the configured database cannot be bootstrapped.
pub async fn bootstrap() -> Result<()> {
    store::bootstrap_from_env().await
}

/// # Errors
///
/// Returns an error when the configured database schema is unavailable.
pub async fn status() -> Result<String> {
    store::schema_status_from_env().await
}

fn app_with_state(state: ControlPlaneState) -> Router {
    let allow_origin = AllowOrigin::exact(
        HeaderValue::from_str(&state.frontend_base_url)
            .unwrap_or_else(|_| HeaderValue::from_static(FRONTEND_BASE_URL)),
    );

    Router::new()
        .route("/healthz", get(health))
        .route("/api/control-plane/auth/providers", get(get_auth_providers))
        .route("/api/control-plane/auth/session", get(get_current_session))
        .route(
            "/api/control-plane/auth/email/start",
            post(start_email_login),
        )
        .route(
            "/api/control-plane/auth/email/complete",
            post(complete_email_login),
        )
        .route(
            "/api/control-plane/auth/oauth/{provider}/start",
            post(start_oauth_login),
        )
        .route(
            "/api/control-plane/auth/oauth/{provider}/callback",
            post(complete_oauth_login),
        )
        .route("/api/control-plane/auth/logout", post(logout))
        .route(
            "/api/control-plane/auth/links",
            get(list_auth_provider_links),
        )
        .route(
            "/api/control-plane/auth/links/{provider}",
            post(unlink_auth_provider).delete(unlink_auth_provider),
        )
        .route("/v1/tenants", get(list_tenants))
        .route("/v1/projects", get(list_projects))
        .route(
            "/v1/merchant/workspace",
            get(merchant_api::get_merchant_workspace),
        )
        .route(
            "/v1/merchant/shops",
            post(merchant_api::create_merchant_shop),
        )
        .route(
            "/v1/merchant/card-products",
            post(merchant_api::create_card_product),
        )
        .route(
            "/v1/merchant/trial-connections",
            post(merchant_api::create_trial_connection),
        )
        .route(
            "/v1/merchant/evaluations",
            post(merchant_api::create_relay_evaluation),
        )
        .route(
            "/v1/replay-capsules/{replay_capsule_id}",
            get(merchant_api::get_replay_capsule),
        )
        .route(
            "/v1/provider-resources",
            get(list_provider_resources).post(create_provider_resource),
        )
        .route(
            "/v1/provider-resources/{provider_resource_id}",
            get(get_provider_resource).put(update_provider_resource),
        )
        .route(
            "/v1/provider-resources/{provider_resource_id}/disable",
            post(disable_provider_resource),
        )
        .route(
            "/v1/route-policies",
            get(list_route_policies).post(create_route_policy),
        )
        .route(
            "/v1/route-policies/{route_policy_id}",
            put(update_route_policy),
        )
        .route(
            "/v1/route-policies/{route_policy_id}/disable",
            post(disable_route_policy),
        )
        .route(
            "/v1/config-snapshots/{config_snapshot_id}",
            get(get_config_snapshot),
        )
        .route(
            "/v1/config-snapshots",
            get(list_config_snapshots).post(create_config_snapshot),
        )
        .route(
            "/v1/config-snapshots/{config_snapshot_id}/activate",
            post(activate_config_snapshot),
        )
        .route("/v1/api-keys", get(list_api_keys).post(create_api_key))
        .route("/v1/api-keys/{api_key_id}/revoke", post(revoke_api_key))
        .route(
            "/v1/codex-auth-accounts",
            get(list_codex_auth_accounts).post(upload_codex_auth_account),
        )
        .route(
            "/v1/oauth-sharing-leases",
            get(list_oauth_sharing_leases).post(upsert_oauth_sharing_lease),
        )
        .route(
            "/v1/oauth-sharing-leases/{lease_id}/revoke",
            post(revoke_oauth_sharing_lease),
        )
        .route(
            "/v1/oauth-carpools",
            get(list_oauth_carpools).post(upsert_oauth_carpool),
        )
        .route(
            "/v1/oauth-carpools/{carpool_id}",
            put(upsert_oauth_carpool).delete(remove_oauth_carpool),
        )
        .route("/v1/oauth-sharing-usage", get(read_oauth_sharing_usage))
        .route(
            "/internal/gateway/api-keys/resolve",
            post(resolve_api_key_for_gateway),
        )
        .route(
            "/internal/gateway/oauth-pool/select",
            post(select_oauth_pool_account_for_gateway),
        )
        .route(
            "/internal/gateway/codex-account-pool/lease",
            post(lease_codex_auth_account_for_gateway),
        )
        .route(
            "/internal/gateway/config/current",
            get(get_internal_gateway_config),
        )
        .route(
            "/internal/gateway/billing-projection",
            get(get_internal_gateway_balance_projection),
        )
        .route("/v1/usage/summary", get(get_usage_summary))
        .route("/v1/usage/breakdown", get(get_usage_breakdown))
        .route("/v1/billing/projection", get(get_balance_projection))
        .route("/v1/pricing/catalog", get(get_pricing_catalog))
        .route("/v1/pricing/simulations", post(create_pricing_simulation))
        .route(
            "/v1/billing/exports",
            get(list_billing_exports).post(create_billing_export),
        )
        .route(
            "/v1/billing/exports/{export_job_id}",
            get(get_billing_export),
        )
        .route(
            "/v1/billing/exports/{export_job_id}/download",
            get(download_billing_export),
        )
        .route("/v1/route-simulations", post(create_route_simulation))
        .route("/v1/route-receipts", get(route_receipts::list))
        .route(
            "/v1/route-receipts/{route_receipt_id}/diagnostics",
            get(route_receipts::get_diagnostics),
        )
        .route(
            "/v1/route-receipts/{route_receipt_id}",
            get(route_receipts::get),
        )
        .route(
            "/v1/route-diagnostics/{route_policy_id}",
            get(get_route_diagnostics),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(allow_origin)
                .allow_credentials(true)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([
                    axum::http::header::ACCEPT,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::COOKIE,
                ]),
        )
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: CONTROL_PLANE_SERVICE_NAME,
        status: "ok",
    })
}

async fn get_auth_providers(
    State(_state): State<ControlPlaneState>,
) -> Json<core_domain::AuthProvidersResponse> {
    Json(core_domain::AuthProvidersResponse {
        providers: StoreMode::provider_catalog(),
    })
}

async fn get_current_session(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<AuthSessionResponse>, ApiError> {
    let session = resolve_session(&state, &headers).await?;
    Ok(Json(AuthSessionResponse {
        session: session.map(|result| result.session),
    }))
}

async fn start_email_login(
    State(state): State<ControlPlaneState>,
    Json(request): Json<EmailLoginStartRequest>,
) -> Result<Json<EmailLoginStartResponse>, ApiError> {
    let context = next_request_context();
    if !state
        .store
        .ensure_known_email(&request.email)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to verify login email: {error}"),
                &context,
            )
        })?
    {
        return Err(ApiError::unauthorized(
            "auth_user_not_found",
            "no known user exists for this email".to_string(),
            &context,
        ));
    }
    if !state
        .store
        .workspace_exists(&request.workspace_slug)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to verify workspace: {error}"),
                &context,
            )
        })?
    {
        return Err(ApiError::bad_request(
            "workspace_unknown",
            format!("workspace `{}` is not available", request.workspace_slug),
            &context,
        ));
    }

    let flow_id = format!("authflow_{}", context.sequence);
    let verification_code = issue_email_verification_code(context.sequence);
    state
        .store
        .create_email_flow(
            &flow_id,
            &request.email,
            &request.workspace_slug,
            &verification_code,
            &expires_at(600),
        )
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to create email login flow: {error}"),
                &context,
            )
        })?;

    Ok(Json(EmailLoginStartResponse {
        flow_id: core_domain::AuthFlowId::parse(flow_id).unwrap(),
        verification_mode: core_domain::EmailLoginVerificationMode::OneTimeCode,
        expires_at: expires_at(600),
        code_hint: email_code_hint(&verification_code),
    }))
}

async fn complete_email_login(
    State(state): State<ControlPlaneState>,
    Json(request): Json<EmailLoginCompleteRequest>,
) -> Result<Response, ApiError> {
    let context = next_request_context();
    let pending = state
        .store
        .consume_login_flow(request.flow_id.as_str())
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load email login flow: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::unauthorized(
                "auth_flow_missing",
                "email login flow was not found or has already been consumed".to_string(),
                &context,
            )
        })?;

    if pending.verification_code.as_deref() != Some(request.code.as_str()) {
        return Err(ApiError::unauthorized(
            "auth_invalid_code",
            "verification code is invalid".to_string(),
            &context,
        ));
    }

    let login_result = state
        .store
        .issue_session(
            &format!("sess_{}", context.sequence),
            AuthProvider::Email,
            &IdentityLookup::Email(pending.email.unwrap_or_default()),
            &pending.workspace_slug,
            &now_rfc3339(),
            &expires_at(SESSION_TTL_SECONDS),
        )
        .await
        .map_err(|error| {
            ApiError::forbidden(
                "tenant_access_denied",
                format!("HugeRouter could not create a session for this email login: {error}"),
                &context,
            )
        })?;

    info!(
        request_id = context.request_id,
        trace_id = context.trace_id,
        session_id = login_result.session.session_id.as_str(),
        "issued HugeRouter auth session"
    );

    let session_cookie = login_result.session.session_id.to_string();
    Ok(with_session_cookie(&session_cookie, Json(login_result)))
}

async fn start_oauth_login(
    State(state): State<ControlPlaneState>,
    Path(provider): Path<String>,
    Json(request): Json<OAuthLoginStartRequest>,
) -> Result<Json<OAuthLoginStartResponse>, ApiError> {
    let context = next_request_context();
    let provider = parse_oauth_provider(&provider, &context)?;
    if !oauth_provider_available(provider) {
        return Err(ApiError::forbidden(
            "provider_disabled",
            format!(
                "{} login is not configured in this environment",
                oauth_provider_slug(provider)
            ),
            &context,
        ));
    }
    if !state
        .store
        .workspace_exists(&request.workspace_slug)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to verify workspace: {error}"),
                &context,
            )
        })?
    {
        return Err(ApiError::bad_request(
            "workspace_unknown",
            format!("workspace `{}` is not available", request.workspace_slug),
            &context,
        ));
    }

    let state_token = format!("oauth_state_{}", context.sequence);
    state
        .store
        .create_oauth_flow(
            &state_token,
            provider,
            &request.workspace_slug,
            &expires_at(600),
        )
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to create oauth flow: {error}"),
                &context,
            )
        })?;

    let redirect_query = request
        .redirect_to
        .as_deref()
        .map(|redirect| format!("&redirect={redirect}"))
        .unwrap_or_default();

    Ok(Json(OAuthLoginStartResponse {
        provider,
        authorization_url: oauth_authorization_url(
            &state.frontend_base_url,
            provider,
            &state_token,
            &redirect_query,
        )
        .ok_or_else(|| {
            ApiError::forbidden(
                "provider_disabled",
                format!(
                    "{} login is enabled but missing OAuth client configuration",
                    oauth_provider_slug(provider)
                ),
                &context,
            )
        })?,
        state: state_token,
        expires_at: expires_at(600),
    }))
}

async fn complete_oauth_login(
    State(state): State<ControlPlaneState>,
    Path(provider): Path<String>,
    Json(request): Json<OAuthCallbackRequest>,
) -> Result<Response, ApiError> {
    let context = next_request_context();
    let provider = parse_oauth_provider(&provider, &context)?;
    let is_mock_code = mock_auth_enabled() && request.code.starts_with("mock-");

    let pending = state
        .store
        .consume_login_flow(&request.state)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load oauth login flow: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::unauthorized(
                "auth_state_missing",
                "oauth state was not found or has already been consumed".to_string(),
                &context,
            )
        })?;

    if pending.provider != Some(provider) {
        return Err(ApiError::unauthorized(
            "auth_provider_mismatch",
            "oauth provider does not match the pending login state".to_string(),
            &context,
        ));
    }

    let (subject, workspace_slug) = if provider == core_domain::OAuthProvider::Oidc {
        let identity = if is_mock_code {
            OidcIdentity {
                subject: oauth_subject(provider),
                email: std::env::var("CONTROL_PLANE_OIDC_EMAIL")
                    .ok()
                    .filter(|value| !value.trim().is_empty()),
                display_name: std::env::var("CONTROL_PLANE_OIDC_DISPLAY_NAME")
                    .ok()
                    .filter(|value| !value.trim().is_empty()),
                groups: oidc_groups_from_env(),
            }
        } else {
            exchange_oidc_identity(&request.code)
                .await
                .map_err(|error| {
                    ApiError::unauthorized(
                        "auth_invalid_code",
                        format!("oidc callback exchange failed: {error}"),
                        &context,
                    )
                })?
        };

        let (resolved_workspace_slug, resolved_role) =
            resolve_oidc_membership(identity.groups.as_slice(), &pending.workspace_slug);
        state
            .store
            .upsert_oidc_user(
                &identity.subject,
                identity.email.as_deref(),
                identity.display_name.as_deref(),
                &resolved_workspace_slug,
                resolved_role,
                &now_rfc3339(),
            )
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to upsert oidc user: {error}"),
                    &context,
                )
            })?;

        (identity.subject, resolved_workspace_slug)
    } else {
        let role = if pending.workspace_slug == PLATFORM_ADMIN_TENANT_SLUG {
            TenantMembershipRole::Admin
        } else {
            TenantMembershipRole::Member
        };
        let identity = if is_mock_code {
            OAuthIdentity {
                subject: oauth_subject(provider),
                email: Some(format!("{}@example.local", oauth_provider_slug(provider))),
                display_name: Some(format!("{} operator", oauth_provider_slug(provider))),
            }
        } else {
            exchange_oauth_identity(
                provider,
                &request.code,
                request.redirect_uri.as_deref(),
                &state.frontend_base_url,
            )
            .await
            .map_err(|error| {
                ApiError::unauthorized(
                    "auth_invalid_code",
                    format!("oauth callback exchange failed: {error}"),
                    &context,
                )
            })?
        };

        state
            .store
            .upsert_oauth_user(
                AuthProvider::from(provider),
                &identity.subject,
                identity.email.as_deref(),
                identity.display_name.as_deref(),
                &pending.workspace_slug,
                role,
                &now_rfc3339(),
            )
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to upsert oauth user: {error}"),
                    &context,
                )
            })?;

        (identity.subject, pending.workspace_slug.clone())
    };
    let login_result = state
        .store
        .issue_session(
            &format!("sess_{}", context.sequence),
            AuthProvider::from(provider),
            &IdentityLookup::ProviderSubject(AuthProvider::from(provider), subject),
            &workspace_slug,
            &now_rfc3339(),
            &expires_at(SESSION_TTL_SECONDS),
        )
        .await
        .map_err(|error| {
            ApiError::forbidden(
                "tenant_access_denied",
                format!("HugeRouter could not create a session for this provider login: {error}"),
                &context,
            )
        })?;

    let session_cookie = login_result.session.session_id.to_string();
    Ok(with_session_cookie(&session_cookie, Json(login_result)))
}

async fn logout(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let context = next_request_context();
    let session_id = extract_session_cookie(&headers).ok_or_else(|| {
        ApiError::unauthorized(
            "auth_invalid",
            "no HugeRouter session cookie was provided".to_string(),
            &context,
        )
    })?;

    let logout_response = state
        .store
        .revoke_session(&session_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to revoke session: {error}"),
                &context,
            )
        })?;

    Ok(clear_session_cookie(Json(logout_response)))
}

async fn list_auth_provider_links(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<AuthProviderLinksResponse>, ApiError> {
    let context = next_request_context();
    let session = require_session(&state, &headers, &context).await?;
    Ok(Json(AuthProviderLinksResponse {
        links: session.links,
    }))
}

async fn unlink_auth_provider(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
) -> Result<Json<UnlinkAuthProviderResponse>, ApiError> {
    let context = next_request_context();
    let session_id = extract_session_cookie(&headers).ok_or_else(|| {
        ApiError::unauthorized(
            "auth_invalid",
            "no HugeRouter session cookie was provided".to_string(),
            &context,
        )
    })?;
    let provider = parse_auth_provider(&provider, &context)?;
    let response = state
        .store
        .unlink_provider(&session_id, provider)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to unlink provider: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::unauthorized(
                "auth_invalid",
                "HugeRouter session was not found".to_string(),
                &context,
            )
        })?;

    Ok(Json(response))
}

async fn list_tenants(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<TenantsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state.store.list_tenants().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to load tenants: {error}"),
            &context,
        )
    })?;
    response.data = authz.filter_tenants(response.data);
    Ok(Json(response))
}

async fn list_projects(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<ProjectsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state.store.list_projects().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to load projects: {error}"),
            &context,
        )
    })?;
    response.data = authz.filter_projects(response.data);
    Ok(Json(response))
}

async fn list_provider_resources(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(filters): Query<ProviderResourceFilters>,
) -> Result<Json<ProviderResourcesResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    if let Some(tenant_id) = filters.tenant_id.as_deref() {
        authz.ensure_read_tenant(tenant_id, &context)?;
    }
    let mut response = state
        .store
        .list_provider_resources(&filters)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load provider resources: {error}"),
                &context,
            )
        })?;
    response.data = authz.filter_provider_resources(response.data);
    Ok(Json(response))
}

async fn create_provider_resource(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(provider_resource): Json<ProviderResource>,
) -> Result<Json<ProviderResource>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    provider_resource.validate().map_err(|error| {
        ApiError::bad_request(
            "provider_resource_invalid",
            format!("provider resource validation failed: {error}"),
            &context,
        )
    })?;
    if let Some(project_id) = provider_resource.project_id.as_ref() {
        let project = load_project(&state, project_id.as_str(), &context).await?;
        ensure_project_matches_tenant(&project, provider_resource.tenant_id.as_str(), &context)?;
        authz.ensure_manage_project(&project, &context)?;
    } else {
        authz.ensure_manage_tenant(provider_resource.tenant_id.as_str(), &context)?;
    }
    Ok(Json(
        state
            .store
            .create_provider_resource(provider_resource)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to create provider resource: {error}"),
                    &context,
                )
            })?,
    ))
}

async fn update_provider_resource(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(provider_resource_id): Path<String>,
    Json(request): Json<ProviderResourceUpdateRequest>,
) -> Result<Json<ProviderResource>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let provider_resource_id =
        ProviderResourceId::parse(provider_resource_id).map_err(|error| {
            ApiError::bad_request(
                "provider_resource_id_invalid",
                format!("invalid provider_resource_id: {error}"),
                &context,
            )
        })?;

    let mut provider_resource = request.provider_resource;
    provider_resource.provider_resource_id = provider_resource_id.clone();
    let existing = load_provider_resource(&state, provider_resource_id.as_str(), &context).await?;
    authz.ensure_manage_tenant(existing.tenant_id.as_str(), &context)?;
    if provider_resource.tenant_id != existing.tenant_id {
        return Err(ApiError::forbidden(
            "tenant_access_denied",
            format!(
                "provider resource `{}` cannot move from tenant `{}` to tenant `{}`",
                provider_resource_id, existing.tenant_id, provider_resource.tenant_id
            ),
            &context,
        ));
    }
    if let Some(project_id) = provider_resource.project_id.as_ref() {
        let project = load_project(&state, project_id.as_str(), &context).await?;
        ensure_project_matches_tenant(&project, provider_resource.tenant_id.as_str(), &context)?;
        authz.ensure_manage_project(&project, &context)?;
    }

    let response = state
        .store
        .update_provider_resource(provider_resource, request.expected_version)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to update provider resource: {error}"),
                &context,
            )
        })?;

    match response {
        ConcurrencyResult::Applied(provider_resource) => Ok(Json(provider_resource)),
        ConcurrencyResult::NotFound => Err(ApiError::not_found(
            "provider_resource_not_found",
            format!("provider resource `{}` was not found", provider_resource_id),
            &context,
        )),
        ConcurrencyResult::VersionConflict => Err(ApiError::conflict(
            "provider_resource_version_conflict",
            "provider resource version is stale".to_string(),
            &context,
        )),
    }
}

async fn disable_provider_resource(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(provider_resource_id): Path<String>,
    Json(request): Json<ConcurrencyRequest>,
) -> Result<Json<ProviderResource>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let provider_resource = load_provider_resource(&state, &provider_resource_id, &context).await?;
    authz.ensure_manage_tenant(provider_resource.tenant_id.as_str(), &context)?;
    let response = state
        .store
        .disable_provider_resource(&provider_resource_id, request.expected_version)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to disable provider resource: {error}"),
                &context,
            )
        })?;

    match response {
        ConcurrencyResult::Applied(provider_resource) => Ok(Json(provider_resource)),
        ConcurrencyResult::NotFound => Err(ApiError::not_found(
            "provider_resource_not_found",
            format!("provider resource `{provider_resource_id}` was not found"),
            &context,
        )),
        ConcurrencyResult::VersionConflict => Err(ApiError::conflict(
            "provider_resource_version_conflict",
            "provider resource version is stale".to_string(),
            &context,
        )),
    }
}

async fn get_provider_resource(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(provider_resource_id): Path<String>,
) -> Result<Json<core_domain::ProviderResource>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let provider_resource = state
        .store
        .get_provider_resource(&provider_resource_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load provider resource: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "provider_resource_not_found",
                format!("provider resource `{provider_resource_id}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(provider_resource.tenant_id.as_str(), &context)?;
    Ok(Json(provider_resource))
}

async fn list_route_policies(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<RoutePoliciesResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state.store.list_route_policies().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to load route policies: {error}"),
            &context,
        )
    })?;
    response.data = authz.filter_route_policies(response.data);
    Ok(Json(response))
}

async fn create_route_policy(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(route_policy): Json<RoutePolicy>,
) -> Result<Json<RoutePolicy>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    route_policy.validate().map_err(|error| {
        ApiError::bad_request(
            "route_policy_invalid",
            format!("route policy validation failed: {error}"),
            &context,
        )
    })?;
    validate_route_policy_protocol_and_capabilities(&route_policy).map_err(|error| {
        ApiError::bad_request(
            "route_policy_compatibility_invalid",
            format!("route policy compatibility validation failed: {error}"),
            &context,
        )
    })?;
    authz.ensure_manage_tenant(route_policy.tenant_id.as_str(), &context)?;
    Ok(Json(
        state
            .store
            .create_route_policy(route_policy)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to create route policy: {error}"),
                    &context,
                )
            })?,
    ))
}

async fn update_route_policy(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(route_policy_id): Path<String>,
    Json(request): Json<RoutePolicyUpdateRequest>,
) -> Result<Json<RoutePolicy>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let route_policy_id = RoutePolicyId::parse(route_policy_id).map_err(|error| {
        ApiError::bad_request(
            "route_policy_id_invalid",
            format!("invalid route_policy_id: {error}"),
            &context,
        )
    })?;

    let mut route_policy = request.route_policy;
    route_policy.route_policy_id = route_policy_id.clone();
    let existing = load_route_policy(&state, route_policy_id.as_str(), &context).await?;
    authz.ensure_manage_tenant(existing.tenant_id.as_str(), &context)?;
    if route_policy.tenant_id != existing.tenant_id {
        return Err(ApiError::forbidden(
            "tenant_access_denied",
            format!(
                "route policy `{}` cannot move from tenant `{}` to tenant `{}`",
                route_policy_id, existing.tenant_id, route_policy.tenant_id
            ),
            &context,
        ));
    }
    validate_route_policy_protocol_and_capabilities(&route_policy).map_err(|error| {
        ApiError::bad_request(
            "route_policy_compatibility_invalid",
            format!("route policy compatibility validation failed: {error}"),
            &context,
        )
    })?;

    let response = state
        .store
        .update_route_policy(route_policy, request.expected_version)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to update route policy: {error}"),
                &context,
            )
        })?;

    match response {
        ConcurrencyResult::Applied(route_policy) => Ok(Json(route_policy)),
        ConcurrencyResult::NotFound => Err(ApiError::not_found(
            "route_policy_not_found",
            format!("route policy `{route_policy_id}` was not found"),
            &context,
        )),
        ConcurrencyResult::VersionConflict => Err(ApiError::conflict(
            "route_policy_version_conflict",
            "route policy version is stale".to_string(),
            &context,
        )),
    }
}

async fn disable_route_policy(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(route_policy_id): Path<String>,
    Json(request): Json<ConcurrencyRequest>,
) -> Result<Json<RoutePolicy>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let route_policy = load_route_policy(&state, &route_policy_id, &context).await?;
    authz.ensure_manage_tenant(route_policy.tenant_id.as_str(), &context)?;
    let response = state
        .store
        .disable_route_policy(&route_policy_id, request.expected_version)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to disable route policy: {error}"),
                &context,
            )
        })?;

    match response {
        ConcurrencyResult::Applied(route_policy) => Ok(Json(route_policy)),
        ConcurrencyResult::NotFound => Err(ApiError::not_found(
            "route_policy_not_found",
            format!("route policy `{route_policy_id}` was not found"),
            &context,
        )),
        ConcurrencyResult::VersionConflict => Err(ApiError::conflict(
            "route_policy_version_conflict",
            "route policy version is stale".to_string(),
            &context,
        )),
    }
}

async fn list_config_snapshots(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<ConfigSnapshotsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state.store.list_config_snapshots().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to load config snapshots: {error}"),
            &context,
        )
    })?;
    response.data = authz.filter_config_snapshots(response.data);
    Ok(Json(response))
}

async fn create_config_snapshot(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(config_snapshot): Json<ConfigSnapshot>,
) -> Result<Json<ConfigSnapshot>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    authz.ensure_manage_tenant(config_snapshot.tenant_id.as_str(), &context)?;
    let project = load_project(&state, config_snapshot.project_id.as_str(), &context).await?;
    ensure_project_matches_tenant(&project, config_snapshot.tenant_id.as_str(), &context)?;
    authz.ensure_manage_project(&project, &context)?;
    let route_policy =
        load_route_policy(&state, config_snapshot.route_policy_id.as_str(), &context).await?;
    ensure_route_policy_matches_tenant(
        &route_policy,
        config_snapshot.tenant_id.as_str(),
        &context,
    )?;
    for provider_resource_id in &config_snapshot.provider_resource_ids {
        let provider_resource =
            load_provider_resource(&state, provider_resource_id.as_str(), &context).await?;
        ensure_provider_resource_matches_tenant(
            &provider_resource,
            config_snapshot.tenant_id.as_str(),
            &context,
        )?;
    }
    Ok(Json(
        state
            .store
            .create_config_snapshot(config_snapshot)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to create config snapshot: {error}"),
                    &context,
                )
            })?,
    ))
}

async fn list_api_keys(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<ApiKeysResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state.store.list_api_keys().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to list API keys: {error}"),
            &context,
        )
    })?;
    if !authz.is_platform_admin() {
        let mut filtered = Vec::new();
        for api_key in response.data {
            let provider_resource =
                load_provider_resource(&state, api_key.provider_resource_id.as_str(), &context)
                    .await?;
            if authz
                .membership(provider_resource.tenant_id.as_str())
                .is_some()
            {
                filtered.push(api_key);
            }
        }
        response.data = filtered;
    }
    Ok(Json(response))
}

async fn create_api_key(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKey>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let provider_resource_id =
        ProviderResourceId::parse(&request.provider_resource_id).map_err(|_| {
            ApiError::bad_request(
                "provider_resource_id_invalid",
                "provider_resource_id is not valid".to_string(),
                &context,
            )
        })?;
    let provider_resource =
        load_provider_resource(&state, provider_resource_id.as_str(), &context).await?;
    authz.ensure_manage_tenant(provider_resource.tenant_id.as_str(), &context)?;

    Ok(Json(
        state
            .store
            .create_api_key(
                provider_resource_id,
                &request.display_name,
                &request.api_key,
            )
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to create API key: {error}"),
                    &context,
                )
            })?,
    ))
}

async fn list_codex_auth_accounts(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<CodexAuthAccountsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state
        .store
        .list_codex_auth_accounts()
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to list Codex auth accounts: {error}"),
                &context,
            )
        })?;

    if !authz.is_platform_admin() {
        response.data.retain(|account| {
            authz
                .membership(account.tenant_id.as_str())
                .is_some_and(|membership| membership.status == TenantMembershipStatus::Active)
        });
    }

    Ok(Json(response))
}

async fn upload_codex_auth_account(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CodexAuthAccountUploadRequest>,
) -> Result<Json<CodexAuthAccount>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(ApiError::bad_request(
            "codex_auth_display_name_required",
            "display_name is required".to_string(),
            &context,
        ));
    }
    validate_codex_auth_json(&request.auth_json, &context)?;

    let tenant_id = request
        .tenant_id
        .as_deref()
        .or_else(|| authz.active_tenant_id())
        .ok_or_else(|| {
            ApiError::forbidden(
                "tenant_required",
                "a tenant context is required to upload Codex auth accounts".to_string(),
                &context,
            )
        })?;
    authz.ensure_manage_tenant(tenant_id, &context)?;
    let tenant_id = core_domain::TenantId::parse(tenant_id).map_err(|error| {
        ApiError::bad_request(
            "tenant_id_invalid",
            format!("invalid tenant_id: {error}"),
            &context,
        )
    })?;
    let project_id = if let Some(project_id) = request.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        ensure_project_matches_tenant(&project, tenant_id.as_str(), &context)?;
        authz.ensure_manage_project(&project, &context)?;
        Some(project.project_id)
    } else {
        None
    };

    let provider_resource_id = ensure_codex_provider_resource(
        &state,
        &authz,
        &context,
        display_name,
        &tenant_id,
        project_id.as_ref(),
        request.provider_resource_id.as_deref(),
        request.endpoint_base_url.as_deref(),
        request.region.as_deref(),
    )
    .await?;
    let serialized_auth_json = serde_json::to_vec(&request.auth_json).map_err(|error| {
        ApiError::bad_request(
            "codex_auth_json_invalid",
            format!("auth_json must be serializable JSON: {error}"),
            &context,
        )
    })?;
    let auth_json_sha256 = hex_sha256(&serialized_auth_json);
    let encrypted_auth_json = encrypt_codex_auth_json(&serialized_auth_json, &context)?;
    let account = CodexAuthAccountRecord {
        codex_account_id: format!("codexacct_{}", context.sequence),
        provider: "codex".to_string(),
        tenant_id,
        project_id,
        provider_resource_id,
        display_name: display_name.to_string(),
        status: "active".to_string(),
        schedulable: true,
        credential_ready: true,
        concurrency_limit: None,
        active_runs: 0,
        rate_limited_until: None,
        overloaded_until: None,
        temp_unschedulable_until: None,
        auth_json_sha256,
        encrypted_auth_json,
        leased_until: None,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
        version: 1,
    };

    Ok(Json(
        state
            .store
            .create_codex_auth_account(account)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to store Codex auth account: {error}"),
                    &context,
                )
            })?,
    ))
}

async fn list_oauth_sharing_leases(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<OAuthSharingLeasesResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state
        .store
        .list_oauth_sharing_leases()
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to list OAuth sharing leases: {error}"),
                &context,
            )
        })?;
    if !authz.is_platform_admin() {
        response.data.retain(|lease| {
            tenant_visible_to_authorizer(&authz, lease.owner_workspace_id.as_deref())
                || tenant_visible_to_authorizer(&authz, Some(&lease.borrower_workspace_id))
        });
    }
    Ok(Json(response))
}

async fn upsert_oauth_sharing_lease(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<OAuthSharingLeaseUpsertRequest>,
) -> Result<Json<OAuthSharingLeaseRecord>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    validate_oauth_pool_provider(&request.provider, &context)?;
    validate_lease_status(&request.status, &context)?;
    validate_lease_policy(&request.policy, &context)?;
    authz.ensure_manage_tenant(&request.borrower_workspace_id, &context)?;
    if let Some(owner_workspace_id) = request.owner_workspace_id.as_deref() {
        authz.ensure_manage_tenant(owner_workspace_id, &context)?;
    }

    let now = now_rfc3339();
    let record = OAuthSharingLeaseRecord {
        lease_id: request.lease_id,
        owner_workspace_id: request.owner_workspace_id,
        owner_user_id: request.owner_user_id,
        borrower_workspace_id: request.borrower_workspace_id,
        borrower_user_id: request.borrower_user_id,
        provider: request.provider,
        pool_id: request.pool_id,
        allowed_account_ids: request.allowed_account_ids,
        status: request.status,
        starts_at: request.starts_at,
        expires_at: request.expires_at,
        max_concurrent_runs: request.max_concurrent_runs,
        usage_budget: request.usage_budget,
        policy: request.policy,
        metadata: request.metadata,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };
    let saved = state
        .store
        .upsert_oauth_sharing_lease(record)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to upsert OAuth sharing lease: {error}"),
                &context,
            )
        })?;
    Ok(Json(saved))
}

async fn revoke_oauth_sharing_lease(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Result<Json<OAuthSharingLeaseRecord>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let lease = state
        .store
        .list_oauth_sharing_leases()
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to list OAuth sharing leases: {error}"),
                &context,
            )
        })?
        .data
        .into_iter()
        .find(|candidate| candidate.lease_id == lease_id)
        .ok_or_else(|| {
            ApiError::not_found(
                "oauth_sharing_lease_not_found",
                format!("OAuth sharing lease `{lease_id}` was not found"),
                &context,
            )
        })?;
    authz.ensure_manage_tenant(&lease.borrower_workspace_id, &context)?;
    let revoked = state
        .store
        .revoke_oauth_sharing_lease(&lease_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to revoke OAuth sharing lease: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "oauth_sharing_lease_not_found",
                format!("OAuth sharing lease `{lease_id}` was not found"),
                &context,
            )
        })?;
    Ok(Json(revoked))
}

async fn list_oauth_carpools(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<OAuthCarpoolsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state.store.list_oauth_carpools().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to list OAuth carpools: {error}"),
            &context,
        )
    })?;
    if !authz.is_platform_admin() {
        response.data.retain(|carpool| {
            carpool
                .member_workspace_ids
                .iter()
                .any(|workspace_id| tenant_visible_to_authorizer(&authz, Some(workspace_id)))
        });
    }
    Ok(Json(response))
}

async fn upsert_oauth_carpool(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<OAuthCarpoolUpsertRequest>,
) -> Result<Json<OAuthCarpoolRecord>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    validate_oauth_pool_provider(&request.provider, &context)?;
    validate_carpool_strategy(&request.strategy, &context)?;
    for workspace_id in &request.member_workspace_ids {
        authz.ensure_manage_tenant(workspace_id, &context)?;
    }
    let now = now_rfc3339();
    let record = OAuthCarpoolRecord {
        carpool_id: request.carpool_id,
        provider: request.provider,
        name: request.name,
        member_workspace_ids: request.member_workspace_ids,
        pool_ids: request.pool_ids,
        strategy: request.strategy,
        member_weights: request.member_weights,
        per_member_concurrency_limit: request.per_member_concurrency_limit,
        per_member_turn_budget: request.per_member_turn_budget,
        enabled: request.enabled,
        metadata: request.metadata,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };
    Ok(Json(
        state
            .store
            .upsert_oauth_carpool(record)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to upsert OAuth carpool: {error}"),
                    &context,
                )
            })?,
    ))
}

async fn remove_oauth_carpool(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(carpool_id): Path<String>,
) -> Result<Json<OAuthCarpoolRecord>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let carpool = state
        .store
        .list_oauth_carpools()
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to list OAuth carpools: {error}"),
                &context,
            )
        })?
        .data
        .into_iter()
        .find(|candidate| candidate.carpool_id == carpool_id)
        .ok_or_else(|| {
            ApiError::not_found(
                "oauth_carpool_not_found",
                format!("OAuth carpool `{carpool_id}` was not found"),
                &context,
            )
        })?;
    for workspace_id in &carpool.member_workspace_ids {
        authz.ensure_manage_tenant(workspace_id, &context)?;
    }
    Ok(Json(
        state
            .store
            .remove_oauth_carpool(&carpool_id)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to remove OAuth carpool: {error}"),
                    &context,
                )
            })?
            .ok_or_else(|| {
                ApiError::not_found(
                    "oauth_carpool_not_found",
                    format!("OAuth carpool `{carpool_id}` was not found"),
                    &context,
                )
            })?,
    ))
}

async fn read_oauth_sharing_usage(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<OAuthSharingUsageQuery>,
) -> Result<Json<OAuthSharingUsageResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    if let Some(workspace_id) = query.workspace_id.as_deref() {
        authz.ensure_read_tenant(workspace_id, &context)?;
    }
    let mut usage = state
        .store
        .read_oauth_sharing_usage(OAuthSharingUsageFilters {
            lease_id: query.lease_id.as_deref(),
            carpool_id: query.carpool_id.as_deref(),
            workspace_id: query.workspace_id.as_deref(),
            provider: query.provider.as_deref(),
            account_id: query.account_id.as_deref(),
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to read OAuth sharing usage: {error}"),
                &context,
            )
        })?;
    if !authz.is_platform_admin() {
        usage
            .data
            .retain(|row| tenant_visible_to_authorizer(&authz, row.workspace_id.as_deref()));
        usage
            .audit_events
            .retain(|event| tenant_visible_to_authorizer(&authz, event.workspace_id.as_deref()));
    }
    Ok(Json(usage))
}

async fn revoke_api_key(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(api_key_id): Path<String>,
    Json(request): Json<ConcurrencyRequest>,
) -> Result<Json<ApiKey>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let api_key_scope = state
        .store
        .list_api_keys()
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to list API keys: {error}"),
                &context,
            )
        })?
        .data
        .into_iter()
        .find(|api_key| api_key.api_key_id == api_key_id)
        .ok_or_else(|| {
            ApiError::not_found(
                "api_key_not_found",
                format!("API key `{api_key_id}` was not found"),
                &context,
            )
        })?;
    let provider_resource = load_provider_resource(
        &state,
        api_key_scope.provider_resource_id.as_str(),
        &context,
    )
    .await?;
    authz.ensure_manage_tenant(provider_resource.tenant_id.as_str(), &context)?;
    let response = state
        .store
        .revoke_api_key(&api_key_id, request.expected_version)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to revoke API key: {error}"),
                &context,
            )
        })?;

    match response {
        ConcurrencyResult::Applied(api_key) => Ok(Json(api_key)),
        ConcurrencyResult::NotFound => Err(ApiError::not_found(
            "api_key_not_found",
            format!("API key `{api_key_id}` was not found"),
            &context,
        )),
        ConcurrencyResult::VersionConflict => Err(ApiError::conflict(
            "api_key_version_conflict",
            "API key version is stale".to_string(),
            &context,
        )),
    }
}

async fn resolve_api_key_for_gateway(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<GatewayApiKeyResolveRequest>,
) -> Result<Json<GatewayApiKeyResolveResponse>, ApiError> {
    let context = next_request_context();
    require_internal_gateway_auth(&state, &headers, &context)?;
    let resolved = state
        .store
        .resolve_api_key(&request.api_key)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to resolve API key: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "api_key_not_found",
                "api key was not found".to_string(),
                &context,
            )
        })?;

    if !resolved.is_active {
        return Err(ApiError::forbidden(
            "api_key_inactive",
            "api key is inactive".to_string(),
            &context,
        ));
    }

    Ok(Json(GatewayApiKeyResolveResponse {
        credential_id: resolved.api_key_id,
        tenant_id: resolved.tenant_id.to_string(),
        project_id: resolved.project_id.map(|project_id| project_id.to_string()),
        status: "active".to_string(),
    }))
}

async fn lease_codex_auth_account_for_gateway(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CodexAuthAccountLeaseRequest>,
) -> Result<Json<CodexAuthAccountLeaseResponse>, ApiError> {
    let context = next_request_context();
    require_internal_gateway_auth(&state, &headers, &context)?;
    let selection = state
        .store
        .select_oauth_pool_account(OAuthPoolSelectionRequest {
            provider: Some("codex".to_string()),
            provider_resource_id: request.provider_resource_id.clone(),
            pool_id: request.pool_id.or(request.provider_resource_id),
            lease_id: request.lease_id,
            carpool_id: request.carpool_id,
            borrower_workspace_id: request.borrower_workspace_id,
            borrower_user_id: request.borrower_user_id,
            session_id: request.session_id,
            model_id: request.model_id,
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to lease Codex auth account: {error}"),
                &context,
            )
        })?;
    let account = selection.account.ok_or_else(|| {
        ApiError::not_found(
            "codex_auth_account_not_found",
            format!(
                "no active Codex auth account is available for the requested pool: {}",
                selection.reason
            ),
            &context,
        )
    })?;
    let auth_json = decrypt_codex_auth_json(&account.encrypted_auth_json, &context)?;

    Ok(Json(CodexAuthAccountLeaseResponse {
        codex_account_id: account.codex_account_id,
        tenant_id: account.tenant_id.to_string(),
        project_id: account.project_id.map(|project_id| project_id.to_string()),
        provider_resource_id: account.provider_resource_id.to_string(),
        display_name: account.display_name,
        leased_until: account
            .leased_until
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
        reason: selection.reason,
        lease_id: selection.lease_id,
        carpool_id: selection.carpool_id,
        auth_json,
    }))
}

async fn select_oauth_pool_account_for_gateway(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<OAuthPoolSelectionRequest>,
) -> Result<Json<Value>, ApiError> {
    let context = next_request_context();
    require_internal_gateway_auth(&state, &headers, &context)?;
    let selection = state
        .store
        .select_oauth_pool_account(request)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to select OAuth pool account: {error}"),
                &context,
            )
        })?;
    let account = selection.account.map(|account| account.public_view());
    Ok(Json(serde_json::json!({
        "account": account,
        "blocked": selection.blocked,
        "reason": selection.reason,
        "lease_id": selection.lease_id,
        "carpool_id": selection.carpool_id,
    })))
}

async fn get_internal_gateway_config(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<InternalGatewayConfigResponse>, ApiError> {
    let context = next_request_context();
    require_internal_gateway_auth(&state, &headers, &context)?;
    let snapshot = state
        .store
        .get_config_snapshot(store::ACTIVE_CONFIG_ALIAS)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load active config snapshot: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "config_snapshot_not_found",
                "active config snapshot is not available".to_string(),
                &context,
            )
        })?
        .config_snapshot;
    let route_policy = state
        .store
        .get_route_policy(snapshot.route_policy_id.as_str())
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load active route policy: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "route_policy_not_found",
                "active route policy is not available".to_string(),
                &context,
            )
        })?;
    let mut provider_resources = Vec::with_capacity(snapshot.provider_resource_ids.len());
    for provider_resource_id in &snapshot.provider_resource_ids {
        let resource = state
            .store
            .get_provider_resource(provider_resource_id.as_str())
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to load provider resource: {error}"),
                    &context,
                )
            })?
            .ok_or_else(|| {
                ApiError::not_found(
                    "provider_resource_not_found",
                    format!(
                        "provider resource `{}` is not available",
                        provider_resource_id
                    ),
                    &context,
                )
            })?;
        provider_resources.push(resource);
    }

    Ok(Json(InternalGatewayConfigResponse {
        config_snapshot: snapshot,
        route_policy,
        provider_resources,
    }))
}

async fn get_internal_gateway_balance_projection(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<BalanceProjectionQuery>,
) -> Result<Json<BalanceProjectionResponse>, ApiError> {
    let context = next_request_context();
    require_internal_gateway_auth(&state, &headers, &context)?;
    let tenant_id = query.tenant_id.ok_or_else(|| {
        ApiError::bad_request(
            "tenant_id_required",
            "tenant_id is required for internal balance projection queries".to_string(),
            &context,
        )
    })?;

    let response = state
        .store
        .get_balance_projection(&tenant_id, query.project_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load balance projection: {error}"),
                &context,
            )
        })?;

    Ok(Json(response))
}

async fn get_config_snapshot(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(config_snapshot_id): Path<String>,
) -> Result<Json<ConfigSnapshotResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let snapshot = state
        .store
        .get_config_snapshot(&config_snapshot_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load config snapshot: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "config_snapshot_not_found",
                format!("config snapshot `{config_snapshot_id}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(snapshot.config_snapshot.tenant_id.as_str(), &context)?;
    Ok(Json(snapshot))
}

async fn activate_config_snapshot(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(config_snapshot_id): Path<String>,
) -> Result<Json<ConfigSnapshotResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let snapshot = load_config_snapshot(&state, &config_snapshot_id, &context).await?;
    authz.ensure_manage_tenant(snapshot.config_snapshot.tenant_id.as_str(), &context)?;
    let snapshot = state
        .store
        .activate_config_snapshot(&config_snapshot_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to activate config snapshot: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "config_snapshot_not_found",
                format!("config snapshot `{config_snapshot_id}` was not found"),
                &context,
            )
        })?;
    Ok(Json(snapshot))
}

async fn get_usage_summary(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<UsageQuery>,
) -> Result<Json<UsageSummaryResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = query.tenant_id.ok_or_else(|| {
        ApiError::bad_request(
            "tenant_id_required",
            "tenant_id is required for usage summary queries".to_string(),
            &context,
        )
    })?;
    authz.ensure_read_tenant(&tenant_id, &context)?;
    if let Some(project_id) = query.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        ensure_project_matches_tenant(&project, &tenant_id, &context)?;
        authz.ensure_read_project(&project, &context)?;
    }

    let response = state
        .store
        .get_usage_summary(
            &tenant_id,
            query.project_id,
            query.window_start,
            query.window_end,
        )
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load usage summary: {error}"),
                &context,
            )
        })?;

    Ok(Json(response))
}

async fn get_usage_breakdown(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<UsageBreakdownQuery>,
) -> Result<Json<UsageBreakdownResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = query.tenant_id.ok_or_else(|| {
        ApiError::bad_request(
            "tenant_id_required",
            "tenant_id is required for usage breakdown queries".to_string(),
            &context,
        )
    })?;
    authz.ensure_read_tenant(&tenant_id, &context)?;
    if let Some(project_id) = query.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        ensure_project_matches_tenant(&project, &tenant_id, &context)?;
        authz.ensure_read_project(&project, &context)?;
    }
    let group_by = parse_usage_breakdown_group_by(query.group_by.as_deref(), &context)?;

    let response = state
        .store
        .get_usage_breakdown(
            &tenant_id,
            query.project_id,
            query.window_start,
            query.window_end,
            group_by,
            query.cursor,
            query.limit,
        )
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load usage breakdown: {error}"),
                &context,
            )
        })?;

    Ok(Json(response))
}

async fn get_balance_projection(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<BalanceProjectionQuery>,
) -> Result<Json<BalanceProjectionResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = query.tenant_id.ok_or_else(|| {
        ApiError::bad_request(
            "tenant_id_required",
            "tenant_id is required for balance projection queries".to_string(),
            &context,
        )
    })?;
    authz.ensure_read_tenant(&tenant_id, &context)?;
    if let Some(project_id) = query.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        ensure_project_matches_tenant(&project, &tenant_id, &context)?;
        authz.ensure_read_project(&project, &context)?;
    }

    let response = state
        .store
        .get_balance_projection(&tenant_id, query.project_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load balance projection: {error}"),
                &context,
            )
        })?;

    Ok(Json(response))
}

async fn get_pricing_catalog(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<PricingCatalogResponse>, ApiError> {
    let context = next_request_context();
    let _authz = authorize_v1_request(&state, &headers, &context).await?;
    let response = state.store.get_pricing_catalog().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to load pricing catalog: {error}"),
            &context,
        )
    })?;

    Ok(Json(response))
}

async fn create_pricing_simulation(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<PricingSimulationRequest>,
) -> Result<Json<PricingSimulationResponse>, ApiError> {
    let context = next_request_context();
    let _authz = authorize_v1_request(&state, &headers, &context).await?;
    let response = state
        .store
        .create_pricing_simulation(request)
        .await
        .map_err(|error| {
            ApiError::bad_request(
                "pricing_simulation_failed",
                format!("pricing simulation could not be completed: {error}"),
                &context,
            )
        })?;

    Ok(Json(response))
}

async fn create_billing_export(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<BillingExportRequest>,
) -> Result<(axum::http::StatusCode, Json<BillingExportJobResponse>), ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    if let Some(project_id) = request.project_id.as_ref() {
        let project = load_project(&state, project_id.as_str(), &context).await?;
        if let Some(tenant_id) = request.tenant_id.as_ref() {
            ensure_project_matches_tenant(&project, tenant_id.as_str(), &context)?;
            authz.ensure_manage_tenant(tenant_id.as_str(), &context)?;
        }
        authz.ensure_manage_project(&project, &context)?;
    } else if let Some(tenant_id) = request.tenant_id.as_ref() {
        authz.ensure_manage_tenant(tenant_id.as_str(), &context)?;
    } else if !authz.is_platform_admin() {
        return Err(ApiError::forbidden(
            "tenant_access_denied",
            "HugeRouter session is not allowed to create a global billing export".to_string(),
            &context,
        ));
    }
    let response = state
        .store
        .create_billing_export(request)
        .await
        .map_err(|error| {
            ApiError::internal(
                "billing_export_failed",
                format!("billing export job could not be queued: {error}"),
                &context,
            )
        })?;

    Ok((axum::http::StatusCode::ACCEPTED, Json(response)))
}

async fn list_billing_exports(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<BillingExportsQuery>,
) -> Result<Json<BillingExportJobsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    if let Some(project_id) = query.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        if let Some(tenant_id) = query.tenant_id.as_deref() {
            ensure_project_matches_tenant(&project, tenant_id, &context)?;
            authz.ensure_read_tenant(tenant_id, &context)?;
        }
        authz.ensure_read_project(&project, &context)?;
    } else if let Some(tenant_id) = query.tenant_id.as_deref() {
        authz.ensure_read_tenant(tenant_id, &context)?;
    }
    let mut response = state
        .store
        .list_billing_exports(query.tenant_id, query.project_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "billing_exports_unavailable",
                format!("failed to list billing exports: {error}"),
                &context,
            )
        })?;
    if !authz.is_platform_admin() {
        response.data.retain(|job| {
            job.tenant_id
                .as_ref()
                .is_some_and(|tenant_id| authz.membership(tenant_id.as_str()).is_some())
        });
    }

    Ok(Json(response))
}

async fn get_billing_export(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(export_job_id): Path<String>,
) -> Result<Json<BillingExportJobResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let response = state
        .store
        .get_billing_export(&export_job_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "billing_export_unavailable",
                format!("failed to load billing export: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "billing_export_not_found",
                format!("billing export job `{export_job_id}` was not found"),
                &context,
            )
        })?;
    if let Some(tenant_id) = response.data.tenant_id.as_ref() {
        authz.ensure_read_tenant(tenant_id.as_str(), &context)?;
    } else if !authz.is_platform_admin() {
        return Err(ApiError::forbidden(
            "tenant_access_denied",
            "HugeRouter session is not allowed to access a global billing export".to_string(),
            &context,
        ));
    }

    Ok(Json(response))
}

async fn download_billing_export(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(export_job_id): Path<String>,
) -> Result<Response, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let export_job = state
        .store
        .get_billing_export(&export_job_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "billing_export_unavailable",
                format!("failed to load billing export: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "billing_export_not_found",
                format!("billing export job `{export_job_id}` was not found"),
                &context,
            )
        })?;
    if let Some(tenant_id) = export_job.data.tenant_id.as_ref() {
        authz.ensure_read_tenant(tenant_id.as_str(), &context)?;
    } else if !authz.is_platform_admin() {
        return Err(ApiError::forbidden(
            "tenant_access_denied",
            "HugeRouter session is not allowed to access a global billing export".to_string(),
            &context,
        ));
    }
    let response = state
        .store
        .download_billing_export(&export_job_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "billing_export_unavailable",
                format!("failed to download billing export: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "billing_export_not_found",
                format!("billing export job `{export_job_id}` was not found"),
                &context,
            )
        })?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, response.0),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{export_job_id}.csv\""),
            ),
        ],
        response.1,
    )
        .into_response())
}

async fn create_route_simulation(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<RouteSimulationRequest>,
) -> Result<Json<RouteSimulationResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let project = load_project(&state, request.project_id.as_str(), &context).await?;
    ensure_project_matches_tenant(&project, request.tenant_id.as_str(), &context)?;
    authz.ensure_read_project(&project, &context)?;
    Ok(Json(state.store.simulate_route(request).await.map_err(
        |error| {
            ApiError::bad_request(
                "route_simulation_failed",
                format!("route simulation could not be completed: {error}"),
                &context,
            )
        },
    )?))
}

async fn get_route_diagnostics(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(route_policy_id): Path<String>,
) -> Result<Json<RouteDiagnosticsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let diagnostics = state
        .store
        .get_route_diagnostics(&route_policy_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route diagnostics: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "route_policy_not_found",
                format!("route policy `{route_policy_id}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(diagnostics.route_policy.tenant_id.as_str(), &context)?;
    Ok(Json(diagnostics))
}

fn validate_route_policy_protocol_and_capabilities(
    route_policy: &RoutePolicy,
) -> Result<(), &'static str> {
    const SUPPORTED_PROTOCOL_FAMILIES: [&str; 7] = [
        "openai_chat",
        "openai_responses",
        "openai_images",
        "mcp_streamable_http",
        "realtime_webrtc",
        "anthropic_messages",
        "gemini_generate_content",
    ];
    const SUPPORTED_CAPABILITIES: [&str; 8] = [
        "streaming",
        "tool_calling",
        "tool_related",
        "json_mode",
        "chat_completions",
        "image_generation",
        "realtime",
        "response_model_metadata",
    ];

    if !SUPPORTED_PROTOCOL_FAMILIES.contains(&route_policy.protocol_family.as_str()) {
        return Err("unsupported protocol_family");
    }

    if route_policy
        .required_capabilities
        .iter()
        .any(|capability| !SUPPORTED_CAPABILITIES.contains(&capability.as_str()))
    {
        return Err("unsupported required_capabilities");
    }

    Ok(())
}

pub(crate) async fn authorize_v1_request(
    state: &ControlPlaneState,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<ControlPlaneAuthorizer, ApiError> {
    require_session(state, headers, context)
        .await
        .map(ControlPlaneAuthorizer::new)
}

async fn require_session(
    state: &ControlPlaneState,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<core_domain::AuthLoginResult, ApiError> {
    resolve_session(state, headers).await?.ok_or_else(|| {
        ApiError::unauthorized(
            "auth_invalid",
            "HugeRouter session is missing or expired".to_string(),
            context,
        )
    })
}

#[allow(clippy::result_large_err)]
fn require_internal_gateway_auth(
    state: &ControlPlaneState,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let configured_token = state.internal_gateway_token.as_deref().ok_or_else(|| {
        ApiError::internal(
            "internal_auth_unconfigured",
            "CONTROL_PLANE_INTERNAL_TOKEN must be configured for internal gateway calls"
                .to_string(),
            context,
        )
    })?;
    let Some(value) = headers.get(AUTHORIZATION) else {
        return Err(ApiError::unauthorized(
            "auth_invalid",
            "missing Authorization header for internal gateway call".to_string(),
            context,
        ));
    };
    let header = value.to_str().map_err(|_| {
        ApiError::unauthorized(
            "auth_invalid",
            "Authorization header must be valid UTF-8".to_string(),
            context,
        )
    })?;
    let Some(token) = header.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized(
            "auth_invalid",
            "Authorization header must use Bearer credentials".to_string(),
            context,
        ));
    };

    if token != configured_token {
        return Err(ApiError::forbidden(
            "forbidden",
            "internal gateway token is invalid".to_string(),
            context,
        ));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn ensure_codex_provider_resource(
    state: &ControlPlaneState,
    authz: &ControlPlaneAuthorizer,
    context: &RequestContext,
    display_name: &str,
    tenant_id: &core_domain::TenantId,
    project_id: Option<&core_domain::ProjectId>,
    requested_provider_resource_id: Option<&str>,
    endpoint_base_url: Option<&str>,
    region: Option<&str>,
) -> Result<ProviderResourceId, ApiError> {
    let provider_resource_id = if let Some(provider_resource_id) = requested_provider_resource_id {
        ProviderResourceId::parse(provider_resource_id).map_err(|error| {
            ApiError::bad_request(
                "provider_resource_id_invalid",
                format!("invalid provider_resource_id: {error}"),
                context,
            )
        })?
    } else {
        ProviderResourceId::parse(format!(
            "prvrsrc_codex_{}_{}",
            slug_fragment(display_name),
            context.sequence
        ))
        .expect("generated provider_resource_id should be valid")
    };

    if let Some(existing) = state
        .store
        .get_provider_resource(provider_resource_id.as_str())
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load provider resource: {error}"),
                context,
            )
        })?
    {
        if existing.tenant_id != *tenant_id {
            return Err(ApiError::forbidden(
                "tenant_access_denied",
                format!(
                    "provider resource `{}` belongs to tenant `{}`",
                    existing.provider_resource_id, existing.tenant_id
                ),
                context,
            ));
        }
        authz.ensure_manage_tenant(existing.tenant_id.as_str(), context)?;
        return Ok(existing.provider_resource_id);
    }

    let now = now_rfc3339();
    let provider_resource = ProviderResource {
        provider_resource_id: provider_resource_id.clone(),
        tenant_id: tenant_id.clone(),
        project_id: project_id.cloned(),
        provider_id: "chatgpt_web".to_string(),
        name: format!("Codex account pool - {display_name}"),
        status: ProviderResourceStatus::Active,
        provenance_class: ProvenanceClass::UnofficialClientChannel,
        credential_owner_type: CredentialOwnerType::Tenant,
        deployment_scope: if project_id.is_some() {
            DeploymentScope::ProjectDedicated
        } else {
            DeploymentScope::TenantDedicated
        },
        region: region
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("global")
            .to_string(),
        endpoint_base_url: endpoint_base_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_CODEX_REVERSE_PROXY_ENDPOINT)
            .to_string(),
        auth_kind: AuthKind::SessionBroker,
        health_state: HealthState::Healthy,
        health_message: Some("Codex auth.json account accepted into encrypted pool".to_string()),
        quarantine_reason: None,
        budget_policy_id: None,
        capabilities: ProviderCapabilities {
            supports_streaming: false,
            supports_tool_calling: true,
            supports_json_mode: true,
            supports_realtime: false,
            supports_response_model_metadata: true,
        },
        supported_protocol_families: vec![
            "openai_chat".to_string(),
            "openai_responses".to_string(),
            "openai_images".to_string(),
        ],
        is_transit_gateway: true,
        version: 1,
        created_at: now.clone(),
        updated_at: now,
    };
    provider_resource.validate().map_err(|error| {
        ApiError::bad_request(
            "provider_resource_invalid",
            format!("Codex reverse proxy provider resource is invalid: {error}"),
            context,
        )
    })?;
    state
        .store
        .create_provider_resource(provider_resource)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to create Codex reverse proxy provider resource: {error}"),
                context,
            )
        })?;

    Ok(provider_resource_id)
}

fn validate_codex_auth_json(auth_json: &Value, context: &RequestContext) -> Result<(), ApiError> {
    if !auth_json.is_object() {
        return Err(ApiError::bad_request(
            "codex_auth_json_invalid",
            "auth_json must be a JSON object from Codex auth.json".to_string(),
            context,
        ));
    }
    let serialized = serde_json::to_vec(auth_json).map_err(|error| {
        ApiError::bad_request(
            "codex_auth_json_invalid",
            format!("auth_json must be serializable JSON: {error}"),
            context,
        )
    })?;
    if serialized.len() > 512 * 1024 {
        return Err(ApiError::bad_request(
            "codex_auth_json_too_large",
            "auth_json must be 512 KiB or smaller".to_string(),
            context,
        ));
    }
    Ok(())
}

fn tenant_visible_to_authorizer(authz: &ControlPlaneAuthorizer, tenant_id: Option<&str>) -> bool {
    tenant_id.is_some_and(|tenant_id| authz.membership(tenant_id).is_some())
}

fn validate_oauth_pool_provider(provider: &str, context: &RequestContext) -> Result<(), ApiError> {
    if matches!(provider, "codex" | "gemini" | "claude_code") {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "oauth_pool_provider_invalid",
            "provider must be one of codex, gemini, or claude_code".to_string(),
            context,
        ))
    }
}

fn validate_lease_status(status: &str, context: &RequestContext) -> Result<(), ApiError> {
    if matches!(
        status,
        "pending" | "active" | "paused" | "expired" | "revoked"
    ) {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "oauth_sharing_lease_status_invalid",
            "lease status is not supported".to_string(),
            context,
        ))
    }
}

fn validate_lease_policy(policy: &str, context: &RequestContext) -> Result<(), ApiError> {
    if matches!(
        policy,
        "fair_share" | "owner_priority" | "borrower_priority"
    ) {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "oauth_sharing_lease_policy_invalid",
            "lease policy is not supported".to_string(),
            context,
        ))
    }
}

fn validate_carpool_strategy(strategy: &str, context: &RequestContext) -> Result<(), ApiError> {
    if matches!(
        strategy,
        "fair_share" | "weighted" | "cheapest_ready" | "fastest_ready"
    ) {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "oauth_carpool_strategy_invalid",
            "carpool strategy is not supported".to_string(),
            context,
        ))
    }
}

fn encrypt_codex_auth_json(
    plaintext: &[u8],
    context: &RequestContext,
) -> Result<EncryptedSecretBlob, ApiError> {
    let (key_id, key_bytes) = credential_encryption_key(context)?;
    let rng = rand::SystemRandom::new();
    let mut nonce_bytes = [0_u8; 12];
    rand::SecureRandom::fill(&rng, &mut nonce_bytes).map_err(|_| {
        ApiError::internal(
            "credential_encryption_failed",
            "failed to generate Codex auth encryption nonce".to_string(),
            context,
        )
    })?;
    let key = aead_key(&key_bytes, context)?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut in_out)
        .map_err(|_| {
            ApiError::internal(
                "credential_encryption_failed",
                "failed to encrypt Codex auth.json".to_string(),
                context,
            )
        })?;

    Ok(EncryptedSecretBlob {
        algorithm: CODEX_AUTH_ENCRYPTION_ALGORITHM.to_string(),
        key_id,
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(in_out),
    })
}

fn decrypt_codex_auth_json(
    encrypted: &EncryptedSecretBlob,
    context: &RequestContext,
) -> Result<Value, ApiError> {
    if encrypted.algorithm != CODEX_AUTH_ENCRYPTION_ALGORITHM {
        return Err(ApiError::internal(
            "credential_encryption_unsupported",
            format!(
                "unsupported Codex auth encryption algorithm `{}`",
                encrypted.algorithm
            ),
            context,
        ));
    }
    let (key_id, key_bytes) = credential_encryption_key(context)?;
    if encrypted.key_id != key_id {
        return Err(ApiError::internal(
            "credential_encryption_key_mismatch",
            "Codex auth account was encrypted with a different credential key".to_string(),
            context,
        ));
    }
    let nonce_bytes = BASE64.decode(&encrypted.nonce).map_err(|_| {
        ApiError::internal(
            "credential_decryption_failed",
            "stored Codex auth nonce is invalid".to_string(),
            context,
        )
    })?;
    let nonce_bytes: [u8; 12] = nonce_bytes.try_into().map_err(|_| {
        ApiError::internal(
            "credential_decryption_failed",
            "stored Codex auth nonce has an invalid length".to_string(),
            context,
        )
    })?;
    let mut ciphertext = BASE64.decode(&encrypted.ciphertext).map_err(|_| {
        ApiError::internal(
            "credential_decryption_failed",
            "stored Codex auth ciphertext is invalid".to_string(),
            context,
        )
    })?;
    let key = aead_key(&key_bytes, context)?;
    let plaintext = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_bytes),
            aead::Aad::empty(),
            &mut ciphertext,
        )
        .map_err(|_| {
            ApiError::internal(
                "credential_decryption_failed",
                "failed to decrypt Codex auth.json".to_string(),
                context,
            )
        })?;
    serde_json::from_slice(plaintext).map_err(|error| {
        ApiError::internal(
            "credential_decryption_failed",
            format!("decrypted Codex auth.json is not valid JSON: {error}"),
            context,
        )
    })
}

fn credential_encryption_key(context: &RequestContext) -> Result<(String, [u8; 32]), ApiError> {
    let configured = configured_credential_encryption_key().map_err(|_| {
        ApiError::internal(
            "credential_encryption_unconfigured",
            "CONTROL_PLANE_CREDENTIAL_ENCRYPTION_KEY must be configured to store Codex auth accounts".to_string(),
            context,
        )
    })?;
    let digest = Sha256::digest(configured.as_bytes());
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    Ok((format!("sha256:{}", hex_prefix(&digest, 12)), key))
}

#[cfg(test)]
#[allow(clippy::unnecessary_wraps)]
fn configured_credential_encryption_key() -> Result<String, std::env::VarError> {
    Ok(std::env::var("CONTROL_PLANE_CREDENTIAL_ENCRYPTION_KEY")
        .or_else(|_| std::env::var("CODEX_AUTH_ENCRYPTION_KEY"))
        .unwrap_or_else(|_| "huge-router-local-development-codex-auth-key".to_string()))
}

#[cfg(not(test))]
fn configured_credential_encryption_key() -> Result<String, std::env::VarError> {
    std::env::var("CONTROL_PLANE_CREDENTIAL_ENCRYPTION_KEY")
        .or_else(|_| std::env::var("CODEX_AUTH_ENCRYPTION_KEY"))
}

fn aead_key(key_bytes: &[u8; 32], context: &RequestContext) -> Result<aead::LessSafeKey, ApiError> {
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key_bytes).map_err(|_| {
        ApiError::internal(
            "credential_encryption_failed",
            "failed to initialize Codex auth encryption key".to_string(),
            context,
        )
    })?;
    Ok(aead::LessSafeKey::new(unbound_key))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_string(&digest)
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    hex_string(&bytes[..bytes.len().min(len)])
}

fn hex_string(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn slug_fragment(value: &str) -> String {
    let slug = value
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character == '-' || character == '_' {
                Some('_')
            } else {
                None
            }
        })
        .take(24)
        .collect::<String>();
    if slug.is_empty() {
        "account".to_string()
    } else {
        slug
    }
}

async fn load_project(
    state: &ControlPlaneState,
    project_id: &str,
    context: &RequestContext,
) -> Result<Project, ApiError> {
    state
        .store
        .get_project(project_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load project: {error}"),
                context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "project_not_found",
                format!("project `{project_id}` was not found"),
                context,
            )
        })
}

async fn load_provider_resource(
    state: &ControlPlaneState,
    provider_resource_id: &str,
    context: &RequestContext,
) -> Result<ProviderResource, ApiError> {
    state
        .store
        .get_provider_resource(provider_resource_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load provider resource: {error}"),
                context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "provider_resource_not_found",
                format!("provider resource `{provider_resource_id}` was not found"),
                context,
            )
        })
}

async fn load_route_policy(
    state: &ControlPlaneState,
    route_policy_id: &str,
    context: &RequestContext,
) -> Result<RoutePolicy, ApiError> {
    state
        .store
        .get_route_policy(route_policy_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route policy: {error}"),
                context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "route_policy_not_found",
                format!("route policy `{route_policy_id}` was not found"),
                context,
            )
        })
}

async fn load_config_snapshot(
    state: &ControlPlaneState,
    config_snapshot_id: &str,
    context: &RequestContext,
) -> Result<ConfigSnapshotResponse, ApiError> {
    state
        .store
        .get_config_snapshot(config_snapshot_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load config snapshot: {error}"),
                context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "config_snapshot_not_found",
                format!("config snapshot `{config_snapshot_id}` was not found"),
                context,
            )
        })
}

fn ensure_project_matches_tenant(
    project: &Project,
    tenant_id: &str,
    context: &RequestContext,
) -> Result<(), ApiError> {
    if project.tenant_id.as_str() == tenant_id {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "tenant_access_denied",
            format!(
                "project `{}` does not belong to tenant `{tenant_id}`",
                project.project_id
            ),
            context,
        ))
    }
}

fn ensure_route_policy_matches_tenant(
    route_policy: &RoutePolicy,
    tenant_id: &str,
    context: &RequestContext,
) -> Result<(), ApiError> {
    if route_policy.tenant_id.as_str() == tenant_id {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "tenant_access_denied",
            format!(
                "route policy `{}` does not belong to tenant `{tenant_id}`",
                route_policy.route_policy_id
            ),
            context,
        ))
    }
}

fn ensure_provider_resource_matches_tenant(
    provider_resource: &ProviderResource,
    tenant_id: &str,
    context: &RequestContext,
) -> Result<(), ApiError> {
    if provider_resource.tenant_id.as_str() == tenant_id {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "tenant_access_denied",
            format!(
                "provider resource `{}` does not belong to tenant `{tenant_id}`",
                provider_resource.provider_resource_id
            ),
            context,
        ))
    }
}

async fn resolve_session(
    state: &ControlPlaneState,
    headers: &HeaderMap,
) -> Result<Option<core_domain::AuthLoginResult>, ApiError> {
    let Some(session_id) = extract_session_cookie(headers) else {
        return Ok(None);
    };
    state.store.get_session(&session_id).await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to resolve session: {error}"),
            &next_request_context(),
        )
    })
}

fn issue_email_verification_code(sequence: u64) -> String {
    let _ = sequence;
    "111111".to_string()
}

fn email_code_hint(code: &str) -> Option<String> {
    if std::env::var("CONTROL_PLANE_EMAIL_DEBUG_CODE_HINTS")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    {
        Some(format!("Use verification code {code}."))
    } else {
        None
    }
}

fn oauth_provider_available(provider: core_domain::OAuthProvider) -> bool {
    match provider {
        core_domain::OAuthProvider::Oidc => auth_provider_enabled(AuthProvider::Oidc),
        core_domain::OAuthProvider::Github => auth_provider_enabled(AuthProvider::Github),
        core_domain::OAuthProvider::Google => auth_provider_enabled(AuthProvider::Google),
        core_domain::OAuthProvider::Wechat => auth_provider_enabled(AuthProvider::Wechat),
    }
}

fn oauth_authorization_url(
    frontend_base_url: &str,
    provider: core_domain::OAuthProvider,
    state_token: &str,
    redirect_query: &str,
) -> Option<String> {
    if provider == core_domain::OAuthProvider::Oidc
        && let Ok(base) = std::env::var("CONTROL_PLANE_OIDC_AUTHORIZATION_URL")
        && !base.trim().is_empty()
    {
        let redirect_uri = std::env::var("CONTROL_PLANE_OIDC_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{frontend_base_url}/login/callback?provider=oidc"));
        return Some(format!(
            "{base}?response_type=code&client_id={}&scope=openid%20profile%20email%20groups&state={state_token}&redirect_uri={}",
            std::env::var("CONTROL_PLANE_OIDC_CLIENT_ID")
                .unwrap_or_else(|_| "huge-router-console".to_string()),
            urlencoding::encode(&redirect_uri),
        ));
    }

    if let Some(config) = oauth_provider_config(provider, frontend_base_url) {
        return Some(oauth_authorization_url_from_config(
            provider,
            &config,
            state_token,
            redirect_query,
        ));
    }

    if !mock_auth_enabled() {
        return None;
    }

    Some(format!(
        "{frontend_base_url}/login/callback?provider={}&state={state_token}&code=mock-{}-code{redirect_query}",
        oauth_provider_slug(provider),
        oauth_provider_slug(provider),
    ))
}

fn oauth_authorization_url_from_config(
    provider: core_domain::OAuthProvider,
    config: &OAuthProviderConfig,
    state_token: &str,
    redirect_query: &str,
) -> String {
    if provider == core_domain::OAuthProvider::Wechat {
        return format!(
            "{}?appid={}&redirect_uri={}&response_type=code&scope={}&state={state_token}{redirect_query}#wechat_redirect",
            config.authorization_url,
            urlencoding::encode(&config.client_id),
            urlencoding::encode(&config.redirect_uri),
            urlencoding::encode(&config.scope),
        );
    }

    format!(
        "{}?response_type=code&client_id={}&scope={}&state={state_token}&redirect_uri={}{}",
        config.authorization_url,
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&config.scope),
        urlencoding::encode(&config.redirect_uri),
        redirect_query,
    )
}

fn oauth_subject(provider: core_domain::OAuthProvider) -> String {
    if provider == core_domain::OAuthProvider::Oidc {
        return std::env::var("CONTROL_PLANE_OIDC_SUBJECT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "oidc_ops".to_string());
    }

    format!("{}_ops", oauth_provider_slug(provider))
}

fn oidc_groups_from_env() -> Vec<String> {
    std::env::var("CONTROL_PLANE_OIDC_TEST_GROUPS")
        .ok()
        .map_or_else(Vec::new, |value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
}

#[derive(Debug, Clone)]
struct OAuthProviderConfig {
    authorization_url: String,
    token_url: String,
    userinfo_url: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    scope: String,
}

fn oauth_provider_config(
    provider: core_domain::OAuthProvider,
    frontend_base_url: &str,
) -> Option<OAuthProviderConfig> {
    if provider == core_domain::OAuthProvider::Oidc {
        return None;
    }
    let slug = oauth_provider_slug(provider).to_ascii_uppercase();
    let client_id = std::env::var(format!("CONTROL_PLANE_OAUTH_{slug}_CLIENT_ID")).ok()?;
    let client_secret = std::env::var(format!("CONTROL_PLANE_OAUTH_{slug}_CLIENT_SECRET")).ok()?;
    let authorization_url = std::env::var(format!("CONTROL_PLANE_OAUTH_{slug}_AUTHORIZATION_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| match provider {
            core_domain::OAuthProvider::Github => {
                "https://github.com/login/oauth/authorize".to_string()
            }
            core_domain::OAuthProvider::Google => {
                "https://accounts.google.com/o/oauth2/v2/auth".to_string()
            }
            core_domain::OAuthProvider::Wechat => {
                "https://open.weixin.qq.com/connect/qrconnect".to_string()
            }
            core_domain::OAuthProvider::Oidc => unreachable!(),
        });
    let token_url = std::env::var(format!("CONTROL_PLANE_OAUTH_{slug}_TOKEN_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| match provider {
            core_domain::OAuthProvider::Github => {
                "https://github.com/login/oauth/access_token".to_string()
            }
            core_domain::OAuthProvider::Google => "https://oauth2.googleapis.com/token".to_string(),
            core_domain::OAuthProvider::Wechat => {
                "https://api.weixin.qq.com/sns/oauth2/access_token".to_string()
            }
            core_domain::OAuthProvider::Oidc => unreachable!(),
        });
    let userinfo_url = std::env::var(format!("CONTROL_PLANE_OAUTH_{slug}_USERINFO_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| match provider {
            core_domain::OAuthProvider::Github => "https://api.github.com/user".to_string(),
            core_domain::OAuthProvider::Google => {
                "https://openidconnect.googleapis.com/v1/userinfo".to_string()
            }
            core_domain::OAuthProvider::Wechat => {
                "https://api.weixin.qq.com/sns/userinfo".to_string()
            }
            core_domain::OAuthProvider::Oidc => unreachable!(),
        });
    let redirect_uri = std::env::var(format!("CONTROL_PLANE_OAUTH_{slug}_REDIRECT_URI"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "{frontend_base_url}/login/callback?provider={}",
                oauth_provider_slug(provider)
            )
        });
    let scope = std::env::var(format!("CONTROL_PLANE_OAUTH_{slug}_SCOPE"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| match provider {
            core_domain::OAuthProvider::Github => "read:user user:email".to_string(),
            core_domain::OAuthProvider::Google => "openid email profile".to_string(),
            core_domain::OAuthProvider::Wechat => "snsapi_login".to_string(),
            core_domain::OAuthProvider::Oidc => unreachable!(),
        });

    Some(OAuthProviderConfig {
        authorization_url,
        token_url,
        userinfo_url,
        client_id,
        client_secret,
        redirect_uri,
        scope,
    })
}

async fn exchange_oauth_identity(
    provider: core_domain::OAuthProvider,
    code: &str,
    redirect_uri: Option<&str>,
    frontend_base_url: &str,
) -> Result<OAuthIdentity> {
    let config = oauth_provider_config(provider, frontend_base_url)
        .context("oauth provider is not configured for external login")?;
    let http = HttpClient::new();
    if provider == core_domain::OAuthProvider::Wechat {
        return exchange_wechat_identity(&http, &config, code).await;
    }

    let form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("client_id", config.client_id.clone()),
        ("client_secret", config.client_secret.clone()),
        (
            "redirect_uri",
            redirect_uri.unwrap_or(&config.redirect_uri).to_string(),
        ),
    ];

    let token_response = http
        .post(&config.token_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&form)
        .send()
        .await
        .context("oauth token request failed")?
        .error_for_status()
        .context("oauth token endpoint returned error status")?;
    let token_payload = token_response
        .json::<Value>()
        .await
        .context("oauth token payload was not valid json")?;
    ensure_oauth_payload_ok(provider, &token_payload)?;
    let access_token = token_payload
        .get("access_token")
        .and_then(Value::as_str)
        .context("oauth token payload missing access_token")?;
    let subject = match provider {
        core_domain::OAuthProvider::Github => token_payload
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    };

    let userinfo_response = http
        .get(&config.userinfo_url)
        .bearer_auth(access_token)
        .header("User-Agent", "HugeRouter Control Plane")
        .send()
        .await
        .context("oauth userinfo request failed")?
        .error_for_status()
        .context("oauth userinfo endpoint returned error status")?;
    let claims = userinfo_response
        .json::<Value>()
        .await
        .context("oauth userinfo payload was not valid json")?;
    ensure_oauth_payload_ok(provider, &claims)?;

    let subject = subject
        .or_else(|| {
            claims
                .get("id")
                .and_then(Value::as_i64)
                .map(|value| value.to_string())
        })
        .or_else(|| {
            claims
                .get("sub")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            claims
                .get("openid")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .context("oauth userinfo missing provider subject")?;
    let email = claims
        .get("email")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| github_primary_email(provider, &claims));
    let display_name = claims
        .get("name")
        .or_else(|| claims.get("login"))
        .or_else(|| claims.get("nickname"))
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(OAuthIdentity {
        subject,
        email,
        display_name,
    })
}

async fn exchange_wechat_identity(
    http: &HttpClient,
    config: &OAuthProviderConfig,
    code: &str,
) -> Result<OAuthIdentity> {
    let token_payload = http
        .get(&config.token_url)
        .query(&[
            ("appid", config.client_id.as_str()),
            ("secret", config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .context("wechat token request failed")?
        .error_for_status()
        .context("wechat token endpoint returned error status")?
        .json::<Value>()
        .await
        .context("wechat token payload was not valid json")?;
    ensure_oauth_payload_ok(core_domain::OAuthProvider::Wechat, &token_payload)?;

    let access_token = token_payload
        .get("access_token")
        .and_then(Value::as_str)
        .context("wechat token payload missing access_token")?;
    let openid = token_payload
        .get("openid")
        .and_then(Value::as_str)
        .context("wechat token payload missing openid")?;

    let claims = http
        .get(&config.userinfo_url)
        .query(&[
            ("access_token", access_token),
            ("openid", openid),
            ("lang", "zh_CN"),
        ])
        .send()
        .await
        .context("wechat userinfo request failed")?
        .error_for_status()
        .context("wechat userinfo endpoint returned error status")?
        .json::<Value>()
        .await
        .context("wechat userinfo payload was not valid json")?;
    ensure_oauth_payload_ok(core_domain::OAuthProvider::Wechat, &claims)?;

    let subject = claims
        .get("unionid")
        .and_then(Value::as_str)
        .or_else(|| claims.get("openid").and_then(Value::as_str))
        .unwrap_or(openid)
        .to_string();
    let display_name = claims
        .get("nickname")
        .or_else(|| claims.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(OAuthIdentity {
        subject,
        email: None,
        display_name,
    })
}

fn ensure_oauth_payload_ok(provider: core_domain::OAuthProvider, payload: &Value) -> Result<()> {
    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        let error_message = payload
            .get("error_description")
            .or_else(|| payload.get("errmsg"))
            .and_then(Value::as_str)
            .unwrap_or(error);
        anyhow::bail!(
            "{} oauth error: {}",
            oauth_provider_slug(provider),
            error_message
        )
    }

    let Some(error_code) = payload.get("errcode") else {
        return Ok(());
    };
    let is_ok = error_code
        .as_i64()
        .map_or_else(|| error_code.as_str() == Some("0"), |code| code == 0);
    if is_ok {
        return Ok(());
    }

    let error_message = payload
        .get("errmsg")
        .or_else(|| payload.get("error_description"))
        .or_else(|| payload.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("oauth provider returned an error payload");
    anyhow::bail!(
        "{} oauth error: {}",
        oauth_provider_slug(provider),
        error_message
    )
}

fn github_primary_email(provider: core_domain::OAuthProvider, claims: &Value) -> Option<String> {
    if provider != core_domain::OAuthProvider::Github {
        return None;
    }

    claims
        .get("email")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn resolve_oidc_membership(
    groups: &[String],
    requested_workspace_slug: &str,
) -> (String, core_domain::TenantMembershipRole) {
    if let Ok(mapping) = std::env::var("CONTROL_PLANE_OIDC_GROUP_ROLE_MAP") {
        for mapping_entry in mapping
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            let Some((group_name, assignment)) = mapping_entry.split_once('=') else {
                continue;
            };
            let Some((workspace_slug, role_slug)) = assignment.split_once(':') else {
                continue;
            };
            if groups.iter().any(|group| group == group_name) {
                let role = match role_slug {
                    "owner" => core_domain::TenantMembershipRole::Owner,
                    "admin" => core_domain::TenantMembershipRole::Admin,
                    _ => core_domain::TenantMembershipRole::Member,
                };
                return (workspace_slug.to_string(), role);
            }
        }
    }

    let admin_groups = std::env::var("CONTROL_PLANE_OIDC_PLATFORM_ADMIN_GROUPS")
        .ok()
        .map_or_else(
            || vec!["platform-admins".to_string()],
            |value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            },
        );

    if groups.iter().any(|group| admin_groups.contains(group)) {
        (
            "platform-admin".to_string(),
            core_domain::TenantMembershipRole::Admin,
        )
    } else {
        (
            requested_workspace_slug.to_string(),
            core_domain::TenantMembershipRole::Member,
        )
    }
}

async fn exchange_oidc_identity(code: &str) -> Result<OidcIdentity> {
    let token_url = std::env::var("CONTROL_PLANE_OIDC_TOKEN_URL")
        .context("CONTROL_PLANE_OIDC_TOKEN_URL is required for real OIDC exchange")?;
    let redirect_uri = std::env::var("CONTROL_PLANE_OIDC_REDIRECT_URI")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let client_id = std::env::var("CONTROL_PLANE_OIDC_CLIENT_ID")
        .unwrap_or_else(|_| "huge-router-console".to_string());
    let client_secret = std::env::var("CONTROL_PLANE_OIDC_CLIENT_SECRET").ok();
    let http = HttpClient::new();
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("client_id", client_id),
    ];
    if let Some(redirect_uri) = redirect_uri {
        form.push(("redirect_uri", redirect_uri));
    }
    if let Some(client_secret) = client_secret {
        form.push(("client_secret", client_secret));
    }

    let token_response = http
        .post(token_url)
        .form(&form)
        .send()
        .await
        .context("oidc token request failed")?
        .error_for_status()
        .context("oidc token endpoint returned error status")?;
    let token_payload: Value = token_response
        .json::<Value>()
        .await
        .context("oidc token payload was not valid json")?;
    let access_token = token_payload
        .get("access_token")
        .and_then(Value::as_str)
        .context("oidc token payload missing access_token")?;

    let claims_payload = if let Ok(userinfo_url) = std::env::var("CONTROL_PLANE_OIDC_USERINFO_URL")
    {
        if userinfo_url.trim().is_empty() {
            token_payload
        } else {
            http.get(userinfo_url)
                .bearer_auth(access_token)
                .send()
                .await
                .context("oidc userinfo request failed")?
                .error_for_status()
                .context("oidc userinfo endpoint returned error status")?
                .json::<Value>()
                .await
                .context("oidc userinfo payload was not valid json")?
        }
    } else {
        token_payload
    };

    let subject = claims_payload
        .get("sub")
        .and_then(Value::as_str)
        .context("oidc claims missing sub")?
        .to_string();
    let email = claims_payload
        .get("email")
        .and_then(Value::as_str)
        .map(str::to_string);
    let display_name = claims_payload
        .get("name")
        .or_else(|| claims_payload.get("preferred_username"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let groups = claims_payload
        .get("groups")
        .and_then(Value::as_array)
        .map(|values: &Vec<Value>| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(OidcIdentity {
        subject,
        email,
        display_name,
        groups,
    })
}

fn parse_oauth_provider(
    provider: &str,
    context: &RequestContext,
) -> Result<core_domain::OAuthProvider, ApiError> {
    match provider {
        "github" => Ok(core_domain::OAuthProvider::Github),
        "google" => Ok(core_domain::OAuthProvider::Google),
        "wechat" => Ok(core_domain::OAuthProvider::Wechat),
        "oidc" => Ok(core_domain::OAuthProvider::Oidc),
        _ => Err(ApiError::bad_request(
            "provider_invalid",
            format!("unsupported OAuth provider `{provider}`"),
            context,
        )),
    }
}

fn parse_auth_provider(provider: &str, context: &RequestContext) -> Result<AuthProvider, ApiError> {
    match provider {
        "email" => Ok(AuthProvider::Email),
        "github" => Ok(AuthProvider::Github),
        "google" => Ok(AuthProvider::Google),
        "wechat" => Ok(AuthProvider::Wechat),
        "oidc" => Ok(AuthProvider::Oidc),
        _ => Err(ApiError::bad_request(
            "provider_invalid",
            format!("unsupported auth provider `{provider}`"),
            context,
        )),
    }
}

fn parse_usage_breakdown_group_by(
    group_by: Option<&str>,
    context: &RequestContext,
) -> Result<store::UsageBreakdownGroupBy, ApiError> {
    match group_by.unwrap_or("provider") {
        "provider" => Ok(store::UsageBreakdownGroupBy::Provider),
        "model" => Ok(store::UsageBreakdownGroupBy::Model),
        "day" => Ok(store::UsageBreakdownGroupBy::Day),
        other => Err(ApiError::bad_request(
            "usage_group_by_invalid",
            format!("unsupported usage breakdown group_by `{other}`"),
            context,
        )),
    }
}

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|segment| {
        let trimmed = segment.trim();
        let expected = format!("{SESSION_COOKIE_NAME}=");
        trimmed
            .strip_prefix(&expected)
            .map(std::string::ToString::to_string)
    })
}

fn with_session_cookie<T>(session_id: &str, body: Json<T>) -> Response
where
    T: Serialize,
{
    ([(SET_COOKIE, build_session_cookie(session_id))], body).into_response()
}

fn clear_session_cookie<T>(body: Json<T>) -> Response
where
    T: Serialize,
{
    let secure = if secure_cookies_enabled() {
        "; Secure"
    } else {
        ""
    };
    (
        [(
            SET_COOKIE,
            format!("{SESSION_COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{secure}"),
        )],
        body,
    )
        .into_response()
}

fn build_session_cookie(session_id: &str) -> String {
    let secure = if secure_cookies_enabled() {
        "; Secure"
    } else {
        ""
    };
    format!(
        "{SESSION_COOKIE_NAME}={session_id}; Path=/; HttpOnly; SameSite=Lax{secure}; Max-Age={SESSION_TTL_SECONDS}"
    )
}

fn secure_cookies_enabled() -> bool {
    std::env::var("CONTROL_PLANE_SECURE_COOKIES")
        .ok()
        .map_or_else(
            || {
                std::env::var("CONSOLE_WEB_BASE_URL")
                    .map(|url| url.starts_with("https://"))
                    .unwrap_or(false)
            },
            |value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            },
        )
}

pub(crate) fn next_request_context() -> RequestContext {
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    RequestContext {
        request_id: format!("req_{sequence}"),
        trace_id: format!("trace_{sequence}"),
        sequence,
    }
}

const fn membership_can_manage(role: TenantMembershipRole) -> bool {
    matches!(
        role,
        TenantMembershipRole::Owner | TenantMembershipRole::Admin
    )
}

fn tenant_access_denied(action: &str, tenant_id: &str, context: &RequestContext) -> ApiError {
    ApiError::forbidden(
        "tenant_access_denied",
        format!("HugeRouter session is not allowed to {action} tenant `{tenant_id}`"),
        context,
    )
}

pub(crate) fn bad_request_error(
    code: &'static str,
    message: impl Into<String>,
    context: &RequestContext,
) -> ApiError {
    ApiError::bad_request(code, message.into(), context)
}

pub(crate) fn not_found_error(
    code: &'static str,
    message: impl Into<String>,
    context: &RequestContext,
) -> ApiError {
    ApiError::not_found(code, message.into(), context)
}

pub(crate) fn internal_error(
    code: &'static str,
    message: impl Into<String>,
    context: &RequestContext,
) -> ApiError {
    ApiError::internal(code, message.into(), context)
}

pub(crate) fn merchant_mutation_error(error: &anyhow::Error, context: &RequestContext) -> ApiError {
    let message = error.to_string();
    for code in [
        "merchant_shop_already_exists",
        "card_product_already_exists",
        "trial_connection_already_exists",
    ] {
        if message.contains(code) {
            return ApiError::conflict(
                code,
                "merchant resource already exists for this tenant".to_string(),
                context,
            );
        }
    }
    for code in [
        "merchant_shop_not_found",
        "trial_connection_not_found",
        "config_snapshot_not_found",
    ] {
        if message.contains(code) {
            return ApiError::not_found(
                code,
                "merchant resource was not found for this tenant".to_string(),
                context,
            );
        }
    }

    bad_request_error(
        "validation_failed",
        format!("failed to apply merchant mutation: {message}"),
        context,
    )
}

impl ApiError {
    fn bad_request(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: axum::http::StatusCode::BAD_REQUEST,
            trace_id: context.trace_id.clone(),
        }
    }

    fn forbidden(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: axum::http::StatusCode::FORBIDDEN,
            trace_id: context.trace_id.clone(),
        }
    }

    fn unauthorized(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: axum::http::StatusCode::UNAUTHORIZED,
            trace_id: context.trace_id.clone(),
        }
    }

    fn not_found(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: axum::http::StatusCode::NOT_FOUND,
            trace_id: context.trace_id.clone(),
        }
    }

    fn internal(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            trace_id: context.trace_id.clone(),
        }
    }

    fn conflict(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: axum::http::StatusCode::CONFLICT,
            trace_id: context.trace_id.clone(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let request_id = self.request_id.clone();
        let trace_id = self.trace_id.clone();

        (
            self.status,
            [
                ("x-request-id", request_id.clone()),
                ("x-trace-id", trace_id.clone()),
            ],
            Json(ErrorEnvelope {
                code: self.code,
                message: self.message,
                request_id,
                trace_id,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ControlPlaneState, OAuthProviderConfig, app_with_state, ensure_oauth_payload_ok,
        oauth_authorization_url_from_config, resolve_oidc_membership,
    };
    use crate::store::{IdentityLookup, SESSION_TTL_SECONDS, UserIdentityKey, UserSeed};
    use axum::{
        body::{Body, to_bytes},
        http::{
            Request, StatusCode,
            header::{AUTHORIZATION, COOKIE, SET_COOKIE},
        },
    };
    use core_domain::{
        AdmissionResult, AuthProvider, AuthProviderLink, AuthProviderLinkId, ConfigSnapshotId,
        ExcludedTarget, FallbackTransition, ProjectId, ProviderResourceId, RoutePolicyId,
        RouteReceipt, ScoreBreakdown, TenantId, TenantMembership, TenantMembershipId,
        TenantMembershipRole, TenantMembershipStatus, TenantSummary, UserId, UserIdentity,
    };
    use serde_json::{Value, json};
    use std::sync::{Arc, RwLock};

    fn fresh_session_window() -> (String, String) {
        (crate::now_rfc3339(), crate::expires_at(SESSION_TTL_SECONDS))
    }

    async fn platform_admin_cookie(state: &ControlPlaneState) -> String {
        let session_id = "sess_platform_admin_test";
        let (authenticated_at, expires_at) = fresh_session_window();
        state
            .store
            .issue_session(
                session_id,
                AuthProvider::Email,
                &IdentityLookup::Email("ops@huge-router.dev".to_string()),
                "platform-admin",
                &authenticated_at,
                &expires_at,
            )
            .await
            .expect("platform admin session should issue");
        format!("huge_router_session={session_id}")
    }

    async fn platform_admin_app() -> (ControlPlaneState, String, axum::Router) {
        let state = ControlPlaneState::memory();
        let cookie = platform_admin_cookie(&state).await;
        let app = app_with_state(state.clone());
        (state, cookie, app)
    }

    async fn issue_cookie(state: &ControlPlaneState, email: &str, workspace_slug: &str) -> String {
        let session_id = format!("sess_{}_{}", workspace_slug, email.replace(['@', '.'], "_"));
        let (authenticated_at, expires_at) = fresh_session_window();
        state
            .store
            .issue_session(
                &session_id,
                AuthProvider::Email,
                &IdentityLookup::Email(email.to_string()),
                workspace_slug,
                &authenticated_at,
                &expires_at,
            )
            .await
            .expect("test session should issue");
        format!("huge_router_session={session_id}")
    }

    fn authz_test_state() -> ControlPlaneState {
        let mut seed = crate::store::SeedData::bootstrap();
        let tenant_acme = seed
            .tenants
            .iter()
            .find(|tenant| tenant.tenant_id.as_str() == "tenant_acme")
            .unwrap()
            .clone();
        let tenant_northstar = seed
            .tenants
            .iter()
            .find(|tenant| tenant.tenant_id.as_str() == "tenant_northstar")
            .unwrap()
            .clone();

        seed.users.push(user_seed(
            "user_acme_admin",
            "acme-admin@huge-router.dev",
            "Acme Admin",
            vec![membership_seed(
                "tmemb_acme_admin",
                &tenant_acme,
                TenantMembershipRole::Admin,
            )],
        ));
        seed.users.push(user_seed(
            "user_acme_member",
            "acme-member@huge-router.dev",
            "Acme Member",
            vec![membership_seed(
                "tmemb_acme_member",
                &tenant_acme,
                TenantMembershipRole::Member,
            )],
        ));
        seed.users.push(user_seed(
            "user_northstar_member",
            "northstar-member@huge-router.dev",
            "Northstar Member",
            vec![membership_seed(
                "tmemb_northstar_member",
                &tenant_northstar,
                TenantMembershipRole::Member,
            )],
        ));

        let mut store = crate::store::MemoryStore::from_seed(seed);
        store.insert_route_receipt(route_receipt(
            "routercpt_acme_1",
            "tenant_acme",
            "proj_core",
            "cfgsnap_gateway_v1",
            "prvrsrc_openai_primary",
            "reasoning-fast",
        ));
        store.insert_route_receipt(route_receipt(
            "routercpt_northstar_1",
            "tenant_northstar",
            "proj_ns_research",
            "cfgsnap_research_v1",
            "prvrsrc_openai_research",
            "research-fast",
        ));

        ControlPlaneState {
            frontend_base_url: super::FRONTEND_BASE_URL.to_string(),
            internal_gateway_token: Some("test-internal-token".to_string()),
            store: crate::store::StoreMode::Memory(Arc::new(RwLock::new(store))),
        }
    }

    fn user_seed(
        user_id: &str,
        email: &str,
        display_name: &str,
        memberships: Vec<TenantMembership>,
    ) -> UserSeed {
        UserSeed {
            user: UserIdentity {
                user_id: UserId::parse(user_id.to_string()).unwrap(),
                primary_email: Some(email.to_string()),
                display_name: display_name.to_string(),
                avatar_url: None,
                created_at: "2026-04-22T00:00:00Z".to_string(),
                last_login_at: None,
            },
            identities: vec![UserIdentityKey::Email(email.to_string())],
            memberships,
            links: vec![AuthProviderLink {
                link_id: AuthProviderLinkId::parse(format!("authlink_{user_id}_email")).unwrap(),
                provider: AuthProvider::Email,
                provider_subject: email.to_string(),
                email: Some(email.to_string()),
                linked_at: "2026-04-22T00:00:00Z".to_string(),
                last_used_at: None,
                can_unlink: false,
            }],
        }
    }

    fn membership_seed(
        membership_id: &str,
        tenant: &core_domain::Tenant,
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

    fn route_receipt(
        route_receipt_id: &str,
        tenant_id: &str,
        project_id: &str,
        config_snapshot_id: &str,
        provider_resource_id: &str,
        model_alias: &str,
    ) -> RouteReceipt {
        RouteReceipt {
            route_receipt_id: core_domain::RouteReceiptId::parse(route_receipt_id).unwrap(),
            tenant_id: TenantId::parse(tenant_id).unwrap(),
            project_id: ProjectId::parse(project_id).unwrap(),
            route_policy_id: RoutePolicyId::parse("routepol_openai_chat_default").unwrap(),
            request_id: format!("{route_receipt_id}_request"),
            trace_id: format!("{route_receipt_id}_trace"),
            protocol_family: "openai_chat".to_string(),
            model_alias: model_alias.to_string(),
            config_snapshot_id: ConfigSnapshotId::parse(config_snapshot_id).unwrap(),
            admission_result: AdmissionResult::Admitted,
            selected_target: Some(ProviderResourceId::parse(provider_resource_id).unwrap()),
            excluded_targets: Vec::<ExcludedTarget>::new(),
            failure_reason: None,
            score_breakdown: ScoreBreakdown {
                latency: 0.8,
                cost: 0.6,
                health: 1.0,
                trust: 1.0,
            },
            fallback_transitions: Vec::<FallbackTransition>::new(),
            normalized_error: None,
            created_at: "2026-04-22T00:00:00Z".to_string(),
        }
    }

    fn request(
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: Option<Value>,
    ) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder = builder.header(COOKIE, cookie);
        }
        if let Some(body) = body {
            builder
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        }
    }

    fn route_simulation_body(tenant_id: &str, project_id: &str, model_alias: &str) -> Value {
        json!({
            "tenant_id": tenant_id,
            "project_id": project_id,
            "credential_scope": "cred_demo",
            "protocol_family": "openai_chat",
            "model_alias": model_alias,
            "required_capabilities": ["json_mode"],
            "region": "us-east-1",
            "expected_prompt_tokens": 64,
            "expected_max_output_tokens": 128,
            "traffic_class": "interactive"
        })
    }

    async fn response_json(response: axum::response::Response) -> Value {
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    async fn upload_codex_account_for_test(
        app: axum::Router,
        admin_cookie: &str,
        provider_resource_id: &str,
    ) -> Value {
        let upload = app
            .oneshot(request(
                "POST",
                "/v1/codex-auth-accounts",
                Some(admin_cookie),
                Some(json!({
                    "display_name": format!("Codex {provider_resource_id}"),
                    "tenant_id": "tenant_acme",
                    "project_id": "proj_core",
                    "provider_resource_id": provider_resource_id,
                    "endpoint_base_url": "https://codex-proxy.example.com/v1",
                    "region": "global",
                    "auth_json": {
                        "OPENAI_API_KEY": "sk-test-secret"
                    }
                })),
            ))
            .await
            .unwrap();
        assert_eq!(upload.status(), StatusCode::OK);
        response_json(upload).await
    }

    async fn upsert_lease_for_test(
        app: axum::Router,
        admin_cookie: &str,
        lease_id: &str,
        pool_id: &str,
        status: &str,
        turns: u64,
        expires_at: &str,
    ) {
        let response = app
            .oneshot(request(
                "POST",
                "/v1/oauth-sharing-leases",
                Some(admin_cookie),
                Some(json!({
                    "lease_id": lease_id,
                    "borrower_workspace_id": "tenant_acme",
                    "provider": "codex",
                    "pool_id": pool_id,
                    "status": status,
                    "starts_at": "2026-04-22T00:00:00Z",
                    "expires_at": expires_at,
                    "max_concurrent_runs": 99,
                    "usage_budget": {
                        "turns": turns
                    },
                    "policy": "fair_share"
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    async fn assert_error(response: axum::response::Response, status: StatusCode, code: &str) {
        assert_eq!(response.status(), status);
        let body = response_json(response).await;
        assert_eq!(body["code"], code);
    }

    fn ids(items: &Value, key: &str) -> Vec<String> {
        items
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item[key].as_str().unwrap().to_string())
            .collect()
    }
    use tower::ServiceExt;

    #[test]
    fn github_authorization_url_uses_standard_oauth_parameters() {
        let config = OAuthProviderConfig {
            authorization_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            userinfo_url: "https://api.github.com/user".to_string(),
            client_id: "github-client".to_string(),
            client_secret: "github-secret".to_string(),
            redirect_uri: "http://localhost:3000/login/callback?provider=github".to_string(),
            scope: "read:user user:email".to_string(),
        };

        let url = oauth_authorization_url_from_config(
            core_domain::OAuthProvider::Github,
            &config,
            "oauth_state_1",
            "",
        );

        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=github-client"));
        assert!(url.contains("scope=read%3Auser%20user%3Aemail"));
        assert!(url.contains("state=oauth_state_1"));
        assert!(url.contains(
            "redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Flogin%2Fcallback%3Fprovider%3Dgithub"
        ));
    }

    #[test]
    fn wechat_authorization_url_uses_qrconnect_parameters() {
        let config = OAuthProviderConfig {
            authorization_url: "https://open.weixin.qq.com/connect/qrconnect".to_string(),
            token_url: "https://api.weixin.qq.com/sns/oauth2/access_token".to_string(),
            userinfo_url: "https://api.weixin.qq.com/sns/userinfo".to_string(),
            client_id: "wx-client".to_string(),
            client_secret: "wx-secret".to_string(),
            redirect_uri: "https://ku0.com/login/callback?provider=wechat".to_string(),
            scope: "snsapi_login".to_string(),
        };

        let url = oauth_authorization_url_from_config(
            core_domain::OAuthProvider::Wechat,
            &config,
            "oauth_state_1",
            "",
        );

        assert!(url.starts_with("https://open.weixin.qq.com/connect/qrconnect?"));
        assert!(url.contains("appid=wx-client"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=snsapi_login"));
        assert!(url.contains("state=oauth_state_1"));
        assert!(
            url.contains(
                "redirect_uri=https%3A%2F%2Fku0.com%2Flogin%2Fcallback%3Fprovider%3Dwechat"
            )
        );
        assert!(url.ends_with("#wechat_redirect"));
        assert!(!url.contains("client_id="));
    }

    #[test]
    fn oauth_error_payloads_are_rejected() {
        let error = ensure_oauth_payload_ok(
            core_domain::OAuthProvider::Wechat,
            &json!({
                "errcode": 40029,
                "errmsg": "invalid code"
            }),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("wechat oauth error"));
        assert!(error.contains("invalid code"));
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let response = app_with_state(ControlPlaneState::memory())
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn email_login_flow_sets_cookie_and_returns_session() {
        let app = app_with_state(ControlPlaneState::memory());

        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/control-plane/auth/email/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "email": "ops@huge-router.dev",
                            "workspaceSlug": "platform-admin"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let start_body: Value = serde_json::from_slice(
            &to_bytes(start_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let flow_id = start_body["flowId"].as_str().unwrap();
        assert_eq!(start_body["verificationMode"], "one_time_code");

        let complete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/control-plane/auth/email/complete")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "code": "111111",
                            "flowId": flow_id
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(complete_response.status(), StatusCode::OK);
        let set_cookie = complete_response
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(set_cookie.contains("huge_router_session=sess_"));

        let body: Value = serde_json::from_slice(
            &to_bytes(complete_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["session"]["authenticatedBy"], "email");
        assert_eq!(body["links"][0]["provider"], "email");
    }

    #[tokio::test]
    async fn exposes_v1_resources_and_active_snapshot_alias() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let tenants = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/tenants")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(tenants.status(), StatusCode::OK);
        let tenants_body: Value =
            serde_json::from_slice(&to_bytes(tenants.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(tenants_body["data"].as_array().unwrap().len(), 3);

        let snapshot = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/v1/config-snapshots/{}",
                        crate::store::ACTIVE_CONFIG_ALIAS
                    ))
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(snapshot.status(), StatusCode::OK);
        let snapshot_body: Value =
            serde_json::from_slice(&to_bytes(snapshot.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(
            snapshot_body["config_snapshot"]["config_snapshot_id"],
            "cfgsnap_gateway_v1"
        );
    }

    #[tokio::test]
    async fn route_simulation_returns_selected_target() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-simulations")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "tenant_id": "tenant_acme",
                            "project_id": "proj_core",
                            "credential_scope": "cred_demo",
                            "protocol_family": "openai_chat",
                            "model_alias": "reasoning-fast",
                            "required_capabilities": ["json_mode"],
                            "region": "us-east-1",
                            "expected_prompt_tokens": 64,
                            "expected_max_output_tokens": 128,
                            "traffic_class": "interactive"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["selected_target"], "prvrsrc_openai_primary");
    }

    #[tokio::test]
    async fn route_simulation_can_select_bedrock_provider() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-simulations")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "tenant_id": "tenant_acme",
                            "project_id": "proj_acme_ops",
                            "credential_scope": "cred_demo",
                            "protocol_family": "openai_chat",
                            "model_alias": "claude-sonnet",
                            "required_capabilities": ["chat_completions"],
                            "region": "us-east-1",
                            "expected_prompt_tokens": 64,
                            "expected_max_output_tokens": 128,
                            "traffic_class": "interactive"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["admission_result"], "admitted");
        assert_eq!(body["config_snapshot_id"], "cfgsnap_bedrock_ops_v1");
        assert_eq!(body["selected_target"], "prvrsrc_bedrock_claude");
        assert_eq!(
            body["eligible_candidates"][0]["provider_resource_id"],
            "prvrsrc_bedrock_claude"
        );
        assert!(body["excluded_candidates"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn route_simulation_uses_latest_activated_project_snapshot() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/config-snapshots")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "config_snapshot_id":"cfgsnap_gateway_v2",
                            "tenant_id":"tenant_acme",
                            "project_id":"proj_core",
                            "revision":2,
                            "status":"draft",
                            "activated_at":null,
                            "provider_resource_ids":["prvrsrc_openai_backup"],
                            "route_policy_id":"routepol_openai_chat_default",
                            "budget_policy_id":"budgetpol_default"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);

        let activate = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/config-snapshots/cfgsnap_gateway_v2/activate")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(activate.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-simulations")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "tenant_id": "tenant_acme",
                            "project_id": "proj_core",
                            "credential_scope": "cred_demo",
                            "protocol_family": "openai_chat",
                            "model_alias": "reasoning-fast",
                            "required_capabilities": ["json_mode"],
                            "region": "us-east-1",
                            "expected_prompt_tokens": 64,
                            "expected_max_output_tokens": 128,
                            "traffic_class": "interactive"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["config_snapshot_id"], "cfgsnap_gateway_v2");
        assert_eq!(body["selected_target"], "prvrsrc_openai_backup");
    }

    fn sample_route_receipt(
        route_receipt_id: &str,
        tenant_id: &str,
        project_id: &str,
        protocol_family: &str,
        created_at: &str,
    ) -> RouteReceipt {
        RouteReceipt {
            route_receipt_id: core_domain::RouteReceiptId::parse(route_receipt_id).unwrap(),
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

    #[tokio::test]
    async fn list_route_receipts_supports_filters_and_sorting() {
        let state = ControlPlaneState::memory();
        let admin_cookie = platform_admin_cookie(&state).await;
        state
            .store
            .insert_route_receipt_for_tests(sample_route_receipt(
                "routercpt_cp_a",
                "tenant_acme",
                "proj_core",
                "openai_chat",
                "2026-04-22T00:01:00Z",
            ));
        state
            .store
            .insert_route_receipt_for_tests(sample_route_receipt(
                "routercpt_cp_b",
                "tenant_acme",
                "proj_core",
                "openai_responses",
                "2026-04-22T00:03:00Z",
            ));
        state
            .store
            .insert_route_receipt_for_tests(sample_route_receipt(
                "routercpt_cp_c",
                "tenant_platform",
                "proj_research",
                "openai_chat",
                "2026-04-22T00:02:00Z",
            ));

        let app = app_with_state(state.clone());
        let list_all = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/route-receipts")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_all.status(), StatusCode::OK);
        let list_all: Value =
            serde_json::from_slice(&to_bytes(list_all.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(list_all["data"].as_array().unwrap().len(), 4);
        let receipt_ids: Vec<_> = list_all["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["route_receipt_id"].as_str().unwrap())
            .collect();
        assert_eq!(
            receipt_ids,
            vec![
                "routercpt_cp_b",
                "routercpt_cp_c",
                "routercpt_cp_a",
                "routercpt_acme_relay_eval",
            ]
        );

        let diagnostics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/route-receipts/routercpt_cp_b/diagnostics")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(diagnostics.status(), StatusCode::OK);
        let diagnostics: Value =
            serde_json::from_slice(&to_bytes(diagnostics.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(
            diagnostics["route_receipt"]["route_receipt_id"],
            "routercpt_cp_b"
        );
        assert_eq!(diagnostics["metadata"]["source"], "memory_store");
        assert_eq!(
            diagnostics["decision_timeline"][0]["stage"],
            "route_receipt"
        );

        let filtered = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/route-receipts?tenant_id=tenant_acme&protocol_family=openai_chat")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(filtered.status(), StatusCode::OK);
        let filtered: Value =
            serde_json::from_slice(&to_bytes(filtered.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let filtered_ids: Vec<_> = filtered["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["route_receipt_id"].as_str().unwrap())
            .collect();
        assert_eq!(
            filtered_ids,
            vec!["routercpt_cp_a", "routercpt_acme_relay_eval"]
        );
    }

    #[tokio::test]
    async fn usage_summary_endpoint_returns_projection_payload() {
        let (_state, admin_cookie, app) = platform_admin_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/usage/summary?tenant_id=tenant_acme")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["data"]["tenant_id"], "tenant_acme");
        assert_eq!(body["data"]["event_count"], 14);
    }

    #[tokio::test]
    async fn pricing_simulation_endpoint_returns_billable_quote() {
        let (_state, admin_cookie, app) = platform_admin_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/pricing/simulations")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "provider_id": "openai",
                            "model_alias": "reasoning-fast",
                            "usage": {
                                "input_tokens": 1200,
                                "output_tokens": 320,
                                "cached_input_tokens": 64
                            }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["catalog_id"], "pricing_catalog_default");
        assert_eq!(body["line_items"][0]["dimension"], "input_tokens");
    }

    #[test]
    fn oidc_group_mapping_promotes_platform_admin_workspace() {
        let (workspace_slug, role) =
            resolve_oidc_membership(&["platform-admins".to_string()], "acme-retail");

        assert_eq!(workspace_slug, "platform-admin");
        assert_eq!(role, core_domain::TenantMembershipRole::Admin);
    }
    #[tokio::test]
    async fn billing_exports_can_be_created_and_listed() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/billing/exports")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "tenant_id": "tenant_acme",
                            "project_id": "proj_core",
                            "window_start": "2026-04-01T00:00:00Z",
                            "window_end": "2026-04-30T23:59:59Z",
                            "format": "csv"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::ACCEPTED);
        let created_body: Value =
            serde_json::from_slice(&to_bytes(created.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let export_job_id = created_body["data"]["export_job_id"].as_str().unwrap();

        let listed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/billing/exports?tenant_id=tenant_acme")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(listed.status(), StatusCode::OK);
        let listed_body: Value =
            serde_json::from_slice(&to_bytes(listed.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(listed_body["data"][0]["export_job_id"], export_job_id);

        let fetched = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/billing/exports/{export_job_id}"))
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(fetched.status(), StatusCode::OK);

        let downloaded = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/billing/exports/{export_job_id}/download"))
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(downloaded.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_route_policy_rejects_unsupported_protocol_and_capability() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let unsupported_protocol = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-policies")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "route_policy_id":"routepol_cp_bad",
                            "tenant_id":"tenant_acme",
                            "display_name":"bad-policy",
                            "protocol_family":"not_supported",
                            "model_alias":"reasoning-fast",
                            "required_capabilities":["json_mode"],
                            "preferred_regions":["us-east-1"],
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unsupported_protocol.status(), StatusCode::BAD_REQUEST);
        let body: Value = serde_json::from_slice(
            &to_bytes(unsupported_protocol.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["code"], "route_policy_compatibility_invalid");

        let unsupported_capability = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-policies")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "route_policy_id":"routepol_cp_bad_cap",
                            "tenant_id":"tenant_acme",
                            "display_name":"bad-capability-policy",
                            "protocol_family":"openai_chat",
                            "model_alias":"reasoning-fast",
                            "required_capabilities":["unknown_capability"],
                            "preferred_regions":["us-east-1"],
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unsupported_capability.status(), StatusCode::BAD_REQUEST);
        let body: Value = serde_json::from_slice(
            &to_bytes(unsupported_capability.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["code"], "route_policy_compatibility_invalid");
    }

    #[tokio::test]
    async fn route_simulation_rejects_incompatible_provider_protocol() {
        let state = ControlPlaneState::memory();
        let admin_cookie = platform_admin_cookie(&state).await;
        state
            .store
            .set_provider_resource_provider_id_for_tests("prvrsrc_openai_primary", "anthropic");

        let app = app_with_state(state);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-simulations")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "tenant_id": "tenant_acme",
                            "project_id": "proj_core",
                            "credential_scope": "cred_demo",
                            "protocol_family": "openai_chat",
                            "model_alias": "reasoning-fast",
                            "required_capabilities": ["json_mode"],
                            "region": "us-east-1",
                            "expected_prompt_tokens": 64,
                            "expected_max_output_tokens": 128,
                            "traffic_class": "interactive"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["admission_result"], "admitted");
        assert_eq!(body["selected_target"], "prvrsrc_openai_backup");
        assert!(
            body["excluded_candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["reason"]
                    == "provider does not advertise protocol family `openai_chat`")
        );
    }

    #[tokio::test]
    async fn route_simulation_rejects_protocol_family_mismatch() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-simulations")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "tenant_id": "tenant_acme",
                            "project_id": "proj_core",
                            "credential_scope": "cred_demo",
                            "protocol_family": "openai_responses",
                            "model_alias": "reasoning-fast",
                            "required_capabilities": ["json_mode"],
                            "region": "us-east-1",
                            "expected_prompt_tokens": 64,
                            "expected_max_output_tokens": 128,
                            "traffic_class": "interactive"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["code"], "route_simulation_failed");
    }

    #[tokio::test]
    async fn provider_resources_support_create_update_disable_with_version() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/provider-resources")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "provider_resource_id":"prvrsrc_cp_test",
                            "tenant_id":"tenant_acme",
                            "project_id":"proj_core",
                            "provider_id":"openai",
                            "name":"cp-test",
                            "status":"active",
                            "provenance_class":"official_api",
                            "credential_owner_type":"platform",
                            "deployment_scope":"shared",
                            "region":"us-east-1",
                            "endpoint_base_url":"https://api.openai.com/v1",
                            "auth_kind":"api_key",
                            "health_state":"healthy",
                            "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true,"supports_realtime":false,"supports_response_model_metadata":true},
                            "supported_protocol_families":["openai_chat","openai_responses"],
                            "is_transit_gateway":false,
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);

        let created: Value =
            serde_json::from_slice(&to_bytes(create.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(created["provider_resource_id"], "prvrsrc_cp_test");
        assert_eq!(created["version"], 1);

        let stale_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/v1/provider-resources/{}",
                        created["provider_resource_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "provider_resource_id":"prvrsrc_cp_test",
                            "tenant_id":"tenant_acme",
                            "project_id":"proj_core",
                            "provider_id":"openai",
                            "name":"cp-test-updated",
                            "status":"active",
                            "provenance_class":"official_api",
                            "credential_owner_type":"platform",
                            "deployment_scope":"shared",
                            "region":"us-east-1",
                            "endpoint_base_url":"https://api.openai.com/v1",
                            "auth_kind":"api_key",
                            "health_state":"healthy",
                            "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true,"supports_realtime":false,"supports_response_model_metadata":true},
                            "supported_protocol_families":["openai_chat","openai_responses"],
                            "is_transit_gateway":false,
                            "expected_version":0,
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stale_update.status(), StatusCode::CONFLICT);

        let update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/v1/provider-resources/{}",
                        created["provider_resource_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "provider_resource_id":"prvrsrc_cp_test",
                            "tenant_id":"tenant_acme",
                            "project_id":"proj_core",
                            "provider_id":"openai",
                            "name":"cp-test-updated",
                            "status":"active",
                            "provenance_class":"official_api",
                            "credential_owner_type":"platform",
                            "deployment_scope":"shared",
                            "region":"us-east-1",
                            "endpoint_base_url":"https://api.openai.com/v1",
                            "auth_kind":"api_key",
                            "health_state":"healthy",
                            "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true,"supports_realtime":false,"supports_response_model_metadata":true},
                            "supported_protocol_families":["openai_chat","openai_responses"],
                            "is_transit_gateway":false,
                            "expected_version":1,
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update.status(), StatusCode::OK);
        let updated: Value =
            serde_json::from_slice(&to_bytes(update.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(updated["version"], 2);
        assert_eq!(updated["name"], "cp-test-updated");

        let stale_disable = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/provider-resources/{}/disable",
                        created["provider_resource_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expected_version":1
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stale_disable.status(), StatusCode::CONFLICT);

        let disabled = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/provider-resources/{}/disable",
                        created["provider_resource_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expected_version":2
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(disabled.status(), StatusCode::OK);
        let disabled_body: Value =
            serde_json::from_slice(&to_bytes(disabled.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(disabled_body["status"], "disabled");
        assert_eq!(disabled_body["version"], 3);
    }

    #[tokio::test]
    async fn codex_auth_upload_stores_redacted_account_and_internal_lease_decrypts() {
        let (_state, admin_cookie, app) = platform_admin_app().await;
        let upload = app
            .clone()
            .oneshot(request(
                "POST",
                "/v1/codex-auth-accounts",
                Some(&admin_cookie),
                Some(json!({
                    "display_name": "Codex pooled account",
                    "tenant_id": "tenant_acme",
                    "project_id": "proj_core",
                    "provider_resource_id": "prvrsrc_codex_pool_test",
                    "endpoint_base_url": "https://codex-proxy.example.com/v1",
                    "region": "global",
                    "auth_json": {
                        "OPENAI_API_KEY": "sk-uploaded-secret",
                        "tokens": {
                            "access_token": "access-secret",
                            "refresh_token": "refresh-secret"
                        }
                    }
                })),
            ))
            .await
            .unwrap();
        assert_eq!(upload.status(), StatusCode::OK);
        let uploaded = response_json(upload).await;
        assert_eq!(uploaded["display_name"], "Codex pooled account");
        assert_eq!(uploaded["provider_resource_id"], "prvrsrc_codex_pool_test");
        assert!(
            uploaded["encrypted_auth_json_key_id"]
                .as_str()
                .unwrap()
                .starts_with("sha256:")
        );
        assert!(uploaded.get("auth_json").is_none());
        assert!(uploaded.get("encrypted_auth_json").is_none());

        let list = app
            .clone()
            .oneshot(request(
                "GET",
                "/v1/codex-auth-accounts",
                Some(&admin_cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        let listed = response_json(list).await;
        assert_eq!(listed["data"].as_array().unwrap().len(), 1);
        assert!(listed["data"][0].get("auth_json").is_none());

        let lease = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "provider_resource_id": "prvrsrc_codex_pool_test"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(lease.status(), StatusCode::OK);
        let leased = response_json(lease).await;
        assert_eq!(leased["provider_resource_id"], "prvrsrc_codex_pool_test");
        assert_eq!(leased["auth_json"]["OPENAI_API_KEY"], "sk-uploaded-secret");
        assert_eq!(
            leased["auth_json"]["tokens"]["refresh_token"],
            "refresh-secret"
        );
        assert!(leased["leased_until"].as_str().unwrap().ends_with('Z'));
    }

    #[tokio::test]
    async fn oauth_sharing_active_lease_selects_authorized_account() {
        let (_state, admin_cookie, app) = platform_admin_app().await;
        upload_codex_account_for_test(app.clone(), &admin_cookie, "prvrsrc_codex_share").await;
        upsert_lease_for_test(
            app.clone(),
            &admin_cookie,
            "lease_codex_share",
            "prvrsrc_codex_share",
            "active",
            10,
            "2999-01-01T00:00:00Z",
        )
        .await;

        let lease = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "lease_id": "lease_codex_share",
                            "borrower_workspace_id": "tenant_acme"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(lease.status(), StatusCode::OK);
        let leased = response_json(lease).await;
        assert_eq!(leased["provider_resource_id"], "prvrsrc_codex_share");
        assert_eq!(leased["lease_id"], "lease_codex_share");
        assert!(leased["auth_json"].get("OPENAI_API_KEY").is_some());
        assert!(
            leased["reason"]
                .as_str()
                .unwrap()
                .contains("authorized lease")
        );
    }

    #[tokio::test]
    async fn oauth_sharing_rejects_expired_revoked_and_exhausted_leases() {
        let (_state, admin_cookie, app) = platform_admin_app().await;
        upload_codex_account_for_test(app.clone(), &admin_cookie, "prvrsrc_codex_budget").await;
        upsert_lease_for_test(
            app.clone(),
            &admin_cookie,
            "lease_codex_expired",
            "prvrsrc_codex_budget",
            "active",
            10,
            "2000-01-01T00:00:00Z",
        )
        .await;

        let expired = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "lease_id": "lease_codex_expired",
                            "borrower_workspace_id": "tenant_acme"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(expired.status(), StatusCode::NOT_FOUND);

        upsert_lease_for_test(
            app.clone(),
            &admin_cookie,
            "lease_codex_budget",
            "prvrsrc_codex_budget",
            "active",
            1,
            "2999-01-01T00:00:00Z",
        )
        .await;
        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "lease_id": "lease_codex_budget",
                            "borrower_workspace_id": "tenant_acme"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let exhausted = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "lease_id": "lease_codex_budget",
                            "borrower_workspace_id": "tenant_acme"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(exhausted.status(), StatusCode::NOT_FOUND);

        let revoked = app
            .clone()
            .oneshot(request(
                "POST",
                "/v1/oauth-sharing-leases/lease_codex_budget/revoke",
                Some(&admin_cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(revoked.status(), StatusCode::OK);
        let after_revoke = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "lease_id": "lease_codex_budget",
                            "borrower_workspace_id": "tenant_acme"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(after_revoke.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn oauth_sharing_blocks_rate_limited_and_unauthorized_borrowers() {
        let (state, admin_cookie, app) = platform_admin_app().await;
        let uploaded =
            upload_codex_account_for_test(app.clone(), &admin_cookie, "prvrsrc_codex_guard").await;
        state
            .store
            .set_codex_auth_account_runtime_state(
                uploaded["codex_account_id"].as_str().unwrap(),
                Some("2999-01-01T00:00:00Z".to_string()),
                None,
                0,
            )
            .await
            .unwrap();
        upsert_lease_for_test(
            app.clone(),
            &admin_cookie,
            "lease_codex_guard",
            "prvrsrc_codex_guard",
            "active",
            10,
            "2999-01-01T00:00:00Z",
        )
        .await;

        let rate_limited = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "lease_id": "lease_codex_guard",
                            "borrower_workspace_id": "tenant_acme"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rate_limited.status(), StatusCode::NOT_FOUND);

        state
            .store
            .set_codex_auth_account_runtime_state(
                uploaded["codex_account_id"].as_str().unwrap(),
                None,
                None,
                0,
            )
            .await
            .unwrap();
        let unauthorized = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/codex-account-pool/lease")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "lease_id": "lease_codex_guard",
                            "borrower_workspace_id": "tenant_northstar"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn oauth_carpool_fair_share_rotates_ready_accounts() {
        let (_state, admin_cookie, app) = platform_admin_app().await;
        upload_codex_account_for_test(app.clone(), &admin_cookie, "prvrsrc_codex_a").await;
        upload_codex_account_for_test(app.clone(), &admin_cookie, "prvrsrc_codex_b").await;
        let carpool = app
            .clone()
            .oneshot(request(
                "POST",
                "/v1/oauth-carpools",
                Some(&admin_cookie),
                Some(json!({
                    "carpool_id": "carpool_codex_test",
                    "provider": "codex",
                    "name": "Codex carpool",
                    "member_workspace_ids": ["tenant_acme"],
                    "pool_ids": ["prvrsrc_codex_a", "prvrsrc_codex_b"],
                    "strategy": "fair_share",
                    "per_member_concurrency_limit": 99,
                    "per_member_turn_budget": 99,
                    "enabled": true
                })),
            ))
            .await
            .unwrap();
        assert_eq!(carpool.status(), StatusCode::OK);

        let mut selected = Vec::new();
        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/internal/gateway/codex-account-pool/lease")
                        .header(AUTHORIZATION, "Bearer test-internal-token")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "carpool_id": "carpool_codex_test",
                                "borrower_workspace_id": "tenant_acme"
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            selected.push(response_json(response).await["provider_resource_id"].clone());
        }

        assert_ne!(selected[0], selected[1]);
    }

    #[tokio::test]
    async fn route_policies_support_create_update_disable_with_version() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-policies")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "route_policy_id":"routepol_cp_test",
                            "tenant_id":"tenant_acme",
                            "display_name":"cp-route-test",
                            "protocol_family":"openai_chat",
                            "model_alias":"reasoning-fast",
                            "required_capabilities":["json_mode"],
                            "preferred_regions":["us-east-1"],
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);
        let created: Value =
            serde_json::from_slice(&to_bytes(create.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        let before_list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/route-policies")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let before_body: Value =
            serde_json::from_slice(&to_bytes(before_list.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let before_len = before_body["data"].as_array().unwrap().len();

        let stale_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/v1/route-policies/{}",
                        created["route_policy_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "route_policy_id":"routepol_cp_test",
                            "tenant_id":"tenant_acme",
                            "display_name":"cp-route-updated",
                            "protocol_family":"openai_chat",
                            "model_alias":"reasoning-fast",
                            "required_capabilities":["json_mode"],
                            "preferred_regions":["us-east-1"],
                            "expected_version":0,
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stale_update.status(), StatusCode::CONFLICT);

        let update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/v1/route-policies/{}",
                        created["route_policy_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "route_policy_id":"routepol_cp_test",
                            "tenant_id":"tenant_acme",
                            "display_name":"cp-route-updated",
                            "protocol_family":"openai_chat",
                            "model_alias":"reasoning-fast",
                            "required_capabilities":["json_mode"],
                            "preferred_regions":["us-east-1"],
                            "expected_version":1,
                            "version":1,
                            "created_at":"2026-04-22T00:00:00Z",
                            "updated_at":"2026-04-22T00:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update.status(), StatusCode::OK);

        let stale_disable = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/route-policies/{}/disable",
                        created["route_policy_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expected_version":1
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stale_disable.status(), StatusCode::CONFLICT);

        let disabled = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/route-policies/{}/disable",
                        created["route_policy_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expected_version":2
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(disabled.status(), StatusCode::OK);

        let after_list = app
            .oneshot(
                Request::builder()
                    .uri("/v1/route-policies")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let after_body: Value =
            serde_json::from_slice(&to_bytes(after_list.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let after_len = after_body["data"].as_array().unwrap().len();
        assert_eq!(after_len + 1, before_len);
        assert!(
            after_body["data"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["route_policy_id"] != created["route_policy_id"])
        );
    }

    #[tokio::test]
    async fn config_snapshots_support_create_list_activate() {
        let (_state, admin_cookie, app) = platform_admin_app().await;

        let before_list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/config-snapshots")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let before_body: Value =
            serde_json::from_slice(&to_bytes(before_list.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let before_len = before_body["data"].as_array().unwrap().len();

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/config-snapshots")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "config_snapshot_id":"cfgsnap_cp_create",
                            "tenant_id":"tenant_acme",
                            "project_id":"proj_core",
                            "revision":1,
                            "status":"draft",
                            "activated_at":null,
                            "provider_resource_ids":["prvrsrc_openai_primary"],
                            "route_policy_id":"routepol_openai_chat_default",
                            "budget_policy_id":"budgetpol_default"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);

        let created: Value =
            serde_json::from_slice(&to_bytes(create.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(created["config_snapshot_id"], "cfgsnap_cp_create");

        let list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/config-snapshots")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list_body: Value =
            serde_json::from_slice(&to_bytes(list.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(list_body["data"].as_array().unwrap().len(), before_len + 1);

        let activate = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/v1/config-snapshots/{}/activate",
                        created["config_snapshot_id"].as_str().unwrap()
                    ))
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(activate.status(), StatusCode::OK);
        let activated: Value =
            serde_json::from_slice(&to_bytes(activate.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(activated["config_snapshot"]["status"], "active");
        assert!(activated["config_snapshot"]["activated_at"].is_string());
    }

    #[tokio::test]
    async fn anonymous_access_to_v1_routes_is_rejected() {
        let app = app_with_state(authz_test_state());
        let requests = vec![
            request("GET", "/v1/tenants", None, None),
            request("GET", "/v1/projects", None, None),
            request("GET", "/v1/provider-resources", None, None),
            request("GET", "/v1/route-policies", None, None),
            request("GET", "/v1/config-snapshots", None, None),
            request("GET", "/v1/api-keys", None, None),
            request("GET", "/v1/usage/summary?tenant_id=tenant_acme", None, None),
            request(
                "GET",
                "/v1/billing/exports?tenant_id=tenant_acme",
                None,
                None,
            ),
            request("GET", "/v1/pricing/catalog", None, None),
            request(
                "POST",
                "/v1/route-simulations",
                None,
                Some(route_simulation_body(
                    "tenant_acme",
                    "proj_core",
                    "reasoning-fast",
                )),
            ),
        ];

        for request in requests {
            assert_error(
                app.clone().oneshot(request).await.unwrap(),
                StatusCode::UNAUTHORIZED,
                "auth_invalid",
            )
            .await;
        }
    }

    #[tokio::test]
    async fn tenant_user_only_sees_membership_scoped_data() {
        let state = authz_test_state();
        let platform_cookie = issue_cookie(&state, "ops@huge-router.dev", "platform-admin").await;
        let tenant_cookie = issue_cookie(&state, "acme-admin@huge-router.dev", "acme-retail").await;
        let app = app_with_state(state);

        let acme_export = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/billing/exports",
                    Some(&platform_cookie),
                    Some(json!({
                        "tenant_id": "tenant_acme",
                        "project_id": "proj_core",
                        "window_start": "2026-04-01T00:00:00Z",
                        "window_end": "2026-04-30T23:59:59Z",
                        "format": "csv"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;
        let northstar_export = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/billing/exports",
                    Some(&platform_cookie),
                    Some(json!({
                        "tenant_id": "tenant_northstar",
                        "project_id": "proj_ns_research",
                        "window_start": "2026-04-01T00:00:00Z",
                        "window_end": "2026-04-30T23:59:59Z",
                        "format": "csv"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;
        let _acme_key = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/api-keys",
                    Some(&platform_cookie),
                    Some(json!({
                        "provider_resource_id": "prvrsrc_openai_primary",
                        "display_name": "acme-key",
                        "api_key": "akp_acme_tenant_only"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;
        let _northstar_key = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/api-keys",
                    Some(&platform_cookie),
                    Some(json!({
                        "provider_resource_id": "prvrsrc_openai_research",
                        "display_name": "northstar-key",
                        "api_key": "akp_northstar_tenant_only"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;

        let tenants = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/tenants", Some(&tenant_cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ids(&tenants["data"], "tenant_id"), vec!["tenant_acme"]);

        let projects = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/projects", Some(&tenant_cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            ids(&projects["data"], "project_id"),
            vec!["proj_core", "proj_acme_ops", "proj_acme_support"]
        );

        let provider_resources = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/provider-resources",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            ids(&provider_resources["data"], "provider_resource_id"),
            vec![
                "prvrsrc_openai_primary",
                "prvrsrc_openai_backup",
                "prvrsrc_bedrock_claude"
            ]
        );

        let route_policies = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/route-policies",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            ids(&route_policies["data"], "route_policy_id"),
            vec![
                "routepol_openai_chat_default",
                "routepol_acme_support",
                "routepol_bedrock_claude_text"
            ]
        );

        let snapshots = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/config-snapshots",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            ids(&snapshots["data"], "config_snapshot_id"),
            vec!["cfgsnap_gateway_v1", "cfgsnap_bedrock_ops_v1"]
        );

        let route_receipts = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/route-receipts",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        let mut route_receipt_ids = ids(&route_receipts["data"], "route_receipt_id");
        route_receipt_ids.sort();
        assert_eq!(
            route_receipt_ids,
            vec!["routercpt_acme_1", "routercpt_acme_relay_eval"]
        );

        let usage = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/usage/summary?tenant_id=tenant_acme&project_id=proj_core",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(usage["data"]["tenant_id"], "tenant_acme");
        assert_eq!(usage["data"]["project_id"], "proj_core");

        let billing = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/billing/exports",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            ids(&billing["data"], "export_job_id"),
            vec![
                acme_export["data"]["export_job_id"]
                    .as_str()
                    .unwrap()
                    .to_string()
            ]
        );
        assert_ne!(
            billing["data"][0]["export_job_id"],
            northstar_export["data"]["export_job_id"]
        );

        let api_keys = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/api-keys", Some(&tenant_cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(api_keys["data"].as_array().unwrap().len(), 1);

        let northstar_lease = request(
            "POST",
            "/v1/oauth-sharing-leases",
            Some(&platform_cookie),
            Some(
                json!({"lease_id":"lease_codex_northstar_scope","borrower_workspace_id":"tenant_northstar","provider":"codex","pool_id":"prvrsrc_openai_research","status":"active","starts_at":"2026-04-22T00:00:00Z","expires_at":"2999-01-01T00:00:00Z","max_concurrent_runs":99,"usage_budget":{"turns":99},"policy":"fair_share"}),
            ),
        );
        assert_eq!(
            app.clone().oneshot(northstar_lease).await.unwrap().status(),
            StatusCode::OK
        );
        let oauth_usage = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/oauth-sharing-usage",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        let events = oauth_usage["audit_events"].as_array().unwrap();
        assert!(events.is_empty());

        let simulation = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/route-simulations",
                    Some(&tenant_cookie),
                    Some(route_simulation_body(
                        "tenant_acme",
                        "proj_core",
                        "reasoning-fast",
                    )),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(simulation["selected_target"], "prvrsrc_openai_primary");
    }

    #[tokio::test]
    async fn merchant_workspace_returns_replay_backed_evaluations_for_tenant() {
        let state = authz_test_state();
        let tenant_cookie = issue_cookie(&state, "acme-admin@huge-router.dev", "acme-retail").await;
        let app = app_with_state(state);

        let body = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/merchant/workspace",
                    Some(&tenant_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;

        assert_eq!(body["data"]["merchant_enabled"], true);
        assert_eq!(body["data"]["shops"][0]["merchant_shop_id"], "mshop_acme");
        assert_eq!(
            body["data"]["recent_evaluations"][0]["replay_capsule_id"],
            "replay_acme_relay_eval"
        );
        assert_eq!(
            body["data"]["recent_evaluations"][0]["estimated_tokens_saved"],
            2400
        );
    }

    #[tokio::test]
    async fn tenant_cannot_read_another_tenants_replay_capsule_by_id() {
        let state = authz_test_state();
        let acme_cookie = issue_cookie(&state, "acme-admin@huge-router.dev", "acme-retail").await;
        let northstar_cookie =
            issue_cookie(&state, "northstar-member@huge-router.dev", "northstar-labs").await;
        let app = app_with_state(state);

        let acme_capsule = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/replay-capsules/replay_acme_relay_eval",
                    Some(&acme_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            acme_capsule["replay_capsule"]["replay_capsule_id"],
            "replay_acme_relay_eval"
        );

        assert_error(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/replay-capsules/replay_acme_relay_eval",
                    Some(&northstar_cookie),
                    None,
                ))
                .await
                .unwrap(),
            StatusCode::NOT_FOUND,
            "not_found",
        )
        .await;
    }

    #[tokio::test]
    async fn tenant_admin_can_create_trial_connection_and_replay_evaluation() {
        let state = authz_test_state();
        let tenant_cookie = issue_cookie(&state, "acme-admin@huge-router.dev", "acme-retail").await;
        let app = app_with_state(state);

        let created_connection = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/merchant/trial-connections",
                    Some(&tenant_cookie),
                    Some(json!({
                        "trial_connection_id": "trialconn_eval_test",
                        "provider_label": "Eval Relay",
                        "endpoint_base_url": "https://vertex.eval.example/v1",
                        "api_key": "sk-trial-eval-123456",
                        "target_model": "claude-sonnet",
                        "notes": "dedicated replayable trial key"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;

        assert_eq!(
            created_connection["trial_connection_id"],
            "trialconn_eval_test"
        );
        assert!(
            created_connection["api_key_masked"]
                .as_str()
                .unwrap()
                .contains("...")
        );

        let created_evaluation = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/merchant/evaluations",
                    Some(&tenant_cookie),
                    Some(json!({
                        "trial_connection_id": "trialconn_eval_test"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;

        assert_eq!(created_evaluation["runner_mode"], "simulated");
        assert_eq!(created_evaluation["verdict"], "warning");
        assert!(
            created_evaluation["replay_capsule_id"]
                .as_str()
                .unwrap()
                .starts_with("replay_")
        );
        assert_eq!(created_evaluation["estimated_tokens_saved"], 2400);
    }

    #[tokio::test]
    async fn tenant_user_cross_tenant_reads_and_writes_are_rejected() {
        let state = authz_test_state();
        let platform_cookie = issue_cookie(&state, "ops@huge-router.dev", "platform-admin").await;
        let acme_admin_cookie =
            issue_cookie(&state, "acme-admin@huge-router.dev", "acme-retail").await;
        let acme_member_cookie =
            issue_cookie(&state, "acme-member@huge-router.dev", "acme-retail").await;
        let app = app_with_state(state);

        let northstar_export = response_json(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/billing/exports",
                    Some(&platform_cookie),
                    Some(json!({
                        "tenant_id": "tenant_northstar",
                        "project_id": "proj_ns_research",
                        "window_start": "2026-04-01T00:00:00Z",
                        "window_end": "2026-04-30T23:59:59Z",
                        "format": "csv"
                    })),
                ))
                .await
                .unwrap(),
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/provider-resources/prvrsrc_openai_research",
                    Some(&acme_admin_cookie),
                    None,
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/config-snapshots/cfgsnap_research_v1",
                    Some(&acme_admin_cookie),
                    None,
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/route-receipts/routercpt_northstar_1",
                    Some(&acme_admin_cookie),
                    None,
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/usage/summary?tenant_id=tenant_northstar&project_id=proj_ns_research",
                    Some(&acme_admin_cookie),
                    None,
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "GET",
                    &format!(
                        "/v1/billing/exports/{}",
                        northstar_export["data"]["export_job_id"].as_str().unwrap()
                    ),
                    Some(&acme_admin_cookie),
                    None,
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/api-keys",
                    Some(&acme_admin_cookie),
                    Some(json!({
                        "provider_resource_id": "prvrsrc_openai_research",
                        "display_name": "cross-tenant",
                        "api_key": "akp_cross_tenant_denied"
                    })),
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/config-snapshots/cfgsnap_research_v1/activate",
                    Some(&acme_admin_cookie),
                    None,
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/v1/provider-resources",
                    Some(&acme_member_cookie),
                    Some(json!({
                        "provider_resource_id":"prvrsrc_member_denied",
                        "tenant_id":"tenant_acme",
                        "project_id":"proj_core",
                        "provider_id":"openai",
                        "name":"member-denied",
                        "status":"active",
                        "provenance_class":"official_api",
                        "credential_owner_type":"platform",
                        "deployment_scope":"shared",
                        "region":"us-east-1",
                        "endpoint_base_url":"https://api.openai.com/v1",
                        "auth_kind":"api_key",
                        "health_state":"healthy",
                        "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true,"supports_realtime":false,"supports_response_model_metadata":true},
                        "supported_protocol_families":["openai_chat","openai_responses"],
                        "is_transit_gateway":false,
                        "version":1,
                        "created_at":"2026-04-22T00:00:00Z",
                        "updated_at":"2026-04-22T00:00:00Z"
                    })),
                ))
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
            "tenant_access_denied",
        )
        .await;
    }

    #[tokio::test]
    async fn platform_admin_retains_global_access() {
        let state = authz_test_state();
        let platform_cookie = issue_cookie(&state, "ops@huge-router.dev", "platform-admin").await;
        let app = app_with_state(state);

        let tenants = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/tenants", Some(&platform_cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ids(&tenants["data"], "tenant_id").len(), 3);

        let providers = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/provider-resources",
                    Some(&platform_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ids(&providers["data"], "provider_resource_id").len(), 4);

        let route_policies = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/route-policies",
                    Some(&platform_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ids(&route_policies["data"], "route_policy_id").len(), 4);

        let route_receipts = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/route-receipts",
                    Some(&platform_cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert!(
            ids(&route_receipts["data"], "route_receipt_id")
                .contains(&"routercpt_northstar_1".to_string())
        );

        let snapshot = app
            .clone()
            .oneshot(request(
                "GET",
                "/v1/config-snapshots/cfgsnap_research_v1",
                Some(&platform_cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(snapshot.status(), StatusCode::OK);

        let activate = app
            .clone()
            .oneshot(request(
                "POST",
                "/v1/config-snapshots/cfgsnap_research_v1/activate",
                Some(&platform_cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(activate.status(), StatusCode::OK);

        let global_catalog = app
            .oneshot(request(
                "GET",
                "/v1/pricing/catalog",
                Some(&platform_cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(global_catalog.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_keys_support_create_list_revoke_with_version() {
        let state = ControlPlaneState::memory();
        let admin_cookie = platform_admin_cookie(&state).await;
        let app = app_with_state(state);

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/api-keys")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "provider_resource_id":"prvrsrc_openai_primary",
                            "display_name":"integration-key",
                            "api_key":"akp_test_very_secret"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);
        let created: Value =
            serde_json::from_slice(&to_bytes(create.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(created["display_name"], "integration-key");
        assert_eq!(created["is_active"], true);
        assert_eq!(created["key_prefix"], "akp_te...");
        let api_key_id = created["api_key_id"].as_str().unwrap().to_string();
        let version = created["version"].as_u64().unwrap();

        let list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/api-keys")
                    .header(COOKIE, &admin_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list_body: Value =
            serde_json::from_slice(&to_bytes(list.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(list_body["data"].as_array().unwrap().len(), 1);
        let first = &list_body["data"][0];
        assert_eq!(first["display_name"], "integration-key");
        assert_eq!(first["key_prefix"], "akp_te...");
        assert_eq!(first.get("api_key"), None);

        let stale_revoke = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/api-keys/{}/revoke", api_key_id))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expected_version":version + 1
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stale_revoke.status(), StatusCode::CONFLICT);

        let revoke = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/api-keys/{}/revoke", api_key_id))
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expected_version":version
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(revoke.status(), StatusCode::OK);
        let revoked: Value =
            serde_json::from_slice(&to_bytes(revoke.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(revoked["is_active"], false);
    }

    #[tokio::test]
    async fn gateway_api_key_resolve_returns_scope_or_404() {
        let state = ControlPlaneState::memory();
        let admin_cookie = platform_admin_cookie(&state).await;
        let app = app_with_state(state);

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/api-keys")
                    .header(COOKIE, &admin_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "provider_resource_id":"prvrsrc_openai_primary",
                            "display_name":"integration-key",
                            "api_key":"akp_live_gateway_lookup"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);
        let created_key: Value =
            serde_json::from_slice(&to_bytes(create.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let api_key_id = created_key["api_key_id"].as_str().unwrap().to_string();

        let resolve = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/api-keys/resolve")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "api_key":"akp_live_gateway_lookup"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resolve.status(), StatusCode::OK);
        let resolved: Value =
            serde_json::from_slice(&to_bytes(resolve.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(resolved["credential_id"], api_key_id);
        assert_eq!(resolved["tenant_id"], "tenant_acme");
        assert_eq!(resolved["project_id"], "proj_core");
        assert_eq!(resolved["status"], "active");

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/api-keys/resolve")
                    .header(AUTHORIZATION, "Bearer test-internal-token")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "api_key":"akp_live_gateway_missing"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);

        let unauthorized = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/gateway/api-keys/resolve")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "api_key":"akp_live_gateway_lookup"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    }
}
