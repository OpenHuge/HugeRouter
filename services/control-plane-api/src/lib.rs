#![allow(clippy::too_many_lines, clippy::uninlined_format_args)]

mod store;

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
use core_domain::{
    AuthProvider, AuthProviderLinksResponse, AuthSessionResponse, ConfigSnapshot,
    EmailLoginCompleteRequest, EmailLoginStartRequest, EmailLoginStartResponse,
    OAuthCallbackRequest, OAuthLoginStartRequest, OAuthLoginStartResponse, Project,
    ProviderResource, ProviderResourceId, RoutePolicy, RoutePolicyId, Tenant, TenantMembership,
    TenantMembershipRole, TenantMembershipStatus, UnlinkAuthProviderResponse,
};
use protocol_ir::{
    BalanceProjectionResponse, BillingExportJobResponse, BillingExportJobsResponse,
    BillingExportRequest, ConfigSnapshotResponse, PricingCatalogResponse, PricingSimulationRequest,
    PricingSimulationResponse, ProjectsResponse, ProviderResourcesResponse, RoutePoliciesResponse,
    RouteReceiptResponse, RouteSimulationRequest, RouteSimulationResponse, TenantsResponse,
    UsageBreakdownResponse, UsageSummaryResponse,
};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use store::{
    ApiKey, ApiKeysResponse, ConcurrencyResult, ConfigSnapshotsResponse, EMAIL_BOOTSTRAP_CODE,
    IdentityLookup, RouteReceiptsResponse, SESSION_TTL_SECONDS, StoreMode, ensure_workspace_slug,
    expires_at, now_rfc3339, oauth_provider_slug,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

const CONTROL_PLANE_SERVICE_NAME: &str = "control-plane-api";
const FRONTEND_BASE_URL: &str = "http://127.0.0.1:3000";
const PLATFORM_ADMIN_TENANT_SLUG: &str = "platform-admin";
const SESSION_COOKIE_NAME: &str = "huge_router_session";

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(10_000);

#[derive(Clone, Debug)]
pub struct ControlPlaneState {
    frontend_base_url: String,
    internal_gateway_token: Option<String>,
    store: StoreMode,
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

#[derive(Debug, Clone, Deserialize)]
struct RouteReceiptsQuery {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
    pub protocol_family: Option<String>,
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
struct ApiError {
    code: &'static str,
    message: String,
    request_id: String,
    status: axum::http::StatusCode,
    trace_id: String,
}

#[derive(Debug, Clone)]
struct RequestContext {
    request_id: String,
    trace_id: String,
    sequence: u64,
}

#[derive(Debug, Clone)]
struct ControlPlaneAuthorizer {
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

    fn ensure_read_tenant(
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

    fn ensure_manage_tenant(
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
}

/// # Errors
///
/// Returns an error when the configured control-plane state cannot be initialized.
pub async fn app() -> Result<Router> {
    Ok(app_with_state(ControlPlaneState::from_env().await?))
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
            "/internal/gateway/api-keys/resolve",
            post(resolve_api_key_for_gateway),
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
        .route("/v1/route-receipts", get(list_route_receipts))
        .route(
            "/v1/route-receipts/{route_receipt_id}",
            get(get_route_receipt),
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
            "no bootstrap user exists for this email".to_string(),
            &context,
        ));
    }
    if !ensure_workspace_slug(&request.workspace_slug) {
        return Err(ApiError::bad_request(
            "workspace_unknown",
            format!(
                "workspace `{}` is not available in bootstrap mode",
                request.workspace_slug
            ),
            &context,
        ));
    }

    let flow_id = format!("authflow_{}", context.sequence);
    state
        .store
        .create_email_flow(
            &flow_id,
            &request.email,
            &request.workspace_slug,
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
        code_hint: Some(format!(
            "Use local bootstrap verification code {EMAIL_BOOTSTRAP_CODE}."
        )),
    }))
}

async fn complete_email_login(
    State(state): State<ControlPlaneState>,
    Json(request): Json<EmailLoginCompleteRequest>,
) -> Result<Response, ApiError> {
    let context = next_request_context();
    if request.code != EMAIL_BOOTSTRAP_CODE {
        return Err(ApiError::unauthorized(
            "auth_invalid_code",
            "verification code is invalid".to_string(),
            &context,
        ));
    }

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
    if provider == core_domain::OAuthProvider::Oidc && !store::oidc_enabled() {
        return Err(ApiError::forbidden(
            "provider_disabled",
            "OIDC login is not configured in this environment".to_string(),
            &context,
        ));
    }
    if !ensure_workspace_slug(&request.workspace_slug) {
        return Err(ApiError::bad_request(
            "workspace_unknown",
            format!(
                "workspace `{}` is not available in bootstrap mode",
                request.workspace_slug
            ),
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
        ),
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

    let is_mock_code = request.code.starts_with("mock-");
    if provider != core_domain::OAuthProvider::Oidc && !is_mock_code {
        return Err(ApiError::unauthorized(
            "auth_invalid_code",
            "oauth callback code is invalid".to_string(),
            &context,
        ));
    }

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
        (oauth_subject(provider), pending.workspace_slug.clone())
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
) -> Result<Json<ProviderResourcesResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let mut response = state
        .store
        .list_provider_resources()
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

async fn get_route_receipt(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(route_receipt_id): Path<String>,
) -> Result<Json<RouteReceiptResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let receipt = state
        .store
        .get_route_receipt(&route_receipt_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route receipt: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "route_receipt_not_found",
                format!("route receipt `{route_receipt_id}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(receipt.route_receipt.tenant_id.as_str(), &context)?;
    Ok(Json(receipt))
}

async fn list_route_receipts(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<RouteReceiptsQuery>,
) -> Result<Json<RouteReceiptsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = query.tenant_id.filter(|value| !value.trim().is_empty());
    let project_id = query.project_id.filter(|value| !value.trim().is_empty());
    let protocol_family = query
        .protocol_family
        .filter(|value| !value.trim().is_empty());
    if let Some(project_id) = project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        if let Some(tenant_id) = tenant_id.as_deref() {
            ensure_project_matches_tenant(&project, tenant_id, &context)?;
            authz.ensure_read_tenant(tenant_id, &context)?;
        }
        authz.ensure_read_project(&project, &context)?;
    } else if let Some(tenant_id) = tenant_id.as_deref() {
        authz.ensure_read_tenant(tenant_id, &context)?;
    }
    let mut response = state
        .store
        .list_route_receipts(tenant_id, project_id, protocol_family)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route receipts: {error}"),
                &context,
            )
        })?;
    response.data = authz.filter_route_receipts(response.data);
    Ok(Json(response))
}

fn validate_route_policy_protocol_and_capabilities(
    route_policy: &RoutePolicy,
) -> Result<(), &'static str> {
    const SUPPORTED_PROTOCOL_FAMILIES: [&str; 6] = [
        "openai_chat",
        "openai_responses",
        "mcp_streamable_http",
        "realtime_webrtc",
        "anthropic_messages",
        "gemini_generate_content",
    ];
    const SUPPORTED_CAPABILITIES: [&str; 4] =
        ["streaming", "tool_calling", "json_mode", "chat_completions"];

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

async fn authorize_v1_request(
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

fn oauth_authorization_url(
    frontend_base_url: &str,
    provider: core_domain::OAuthProvider,
    state_token: &str,
    redirect_query: &str,
) -> String {
    if provider == core_domain::OAuthProvider::Oidc
        && let Ok(base) = std::env::var("CONTROL_PLANE_OIDC_AUTHORIZATION_URL")
        && !base.trim().is_empty()
    {
        let redirect_uri = std::env::var("CONTROL_PLANE_OIDC_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{frontend_base_url}/login/callback?provider=oidc"));
        return format!(
            "{base}?response_type=code&client_id={}&scope=openid%20profile%20email%20groups&state={state_token}&redirect_uri={}",
            std::env::var("CONTROL_PLANE_OIDC_CLIENT_ID")
                .unwrap_or_else(|_| "huge-router-console".to_string()),
            urlencoding::encode(&redirect_uri),
        );
    }

    format!(
        "{frontend_base_url}/login/callback?provider={}&state={state_token}&code=mock-{}-code{redirect_query}",
        oauth_provider_slug(provider),
        oauth_provider_slug(provider),
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
    (
        [(
            SET_COOKIE,
            format!("{SESSION_COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"),
        )],
        body,
    )
        .into_response()
}

fn build_session_cookie(session_id: &str) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={session_id}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_SECONDS}"
    )
}

fn next_request_context() -> RequestContext {
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
    use super::{ControlPlaneState, app_with_state, resolve_oidc_membership};
    use crate::store::{IdentityLookup, UserIdentityKey, UserSeed};
    use axum::{
        body::{Body, to_bytes},
        http::{
            Request, StatusCode,
            header::{AUTHORIZATION, COOKIE, SET_COOKIE},
        },
    };
    use core_domain::{
        AdmissionResult, AuthProvider, AuthProviderLink, AuthProviderLinkId, ConfigSnapshotId,
        ExcludedTarget, FallbackTransition, ProjectId, ProviderResourceId, RouteReceipt,
        ScoreBreakdown, TenantId, TenantMembership, TenantMembershipId, TenantMembershipRole,
        TenantMembershipStatus, TenantSummary, UserId, UserIdentity,
    };
    use serde_json::{Value, json};
    use std::sync::{Arc, RwLock};

    async fn platform_admin_cookie(state: &ControlPlaneState) -> String {
        let session_id = "sess_platform_admin_test";
        state
            .store
            .issue_session(
                session_id,
                AuthProvider::Email,
                &IdentityLookup::Email("ops@huge-router.dev".to_string()),
                "platform-admin",
                "2026-04-22T00:00:00Z",
                "2026-04-22T08:00:00Z",
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
        state
            .store
            .issue_session(
                &session_id,
                AuthProvider::Email,
                &IdentityLookup::Email(email.to_string()),
                workspace_slug,
                "2026-04-22T00:00:00Z",
                "2026-04-22T08:00:00Z",
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
            request_id: format!("{route_receipt_id}_request"),
            trace_id: format!("{route_receipt_id}_trace"),
            protocol_family: "openai_chat".to_string(),
            model_alias: model_alias.to_string(),
            config_snapshot_id: ConfigSnapshotId::parse(config_snapshot_id).unwrap(),
            admission_result: AdmissionResult::Admitted,
            selected_target: Some(ProviderResourceId::parse(provider_resource_id).unwrap()),
            excluded_targets: Vec::<ExcludedTarget>::new(),
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
        assert_eq!(list_all["data"].as_array().unwrap().len(), 3);
        let receipt_ids: Vec<_> = list_all["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["route_receipt_id"].as_str().unwrap())
            .collect();
        assert_eq!(
            receipt_ids,
            vec!["routercpt_cp_b", "routercpt_cp_c", "routercpt_cp_a"]
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
        assert_eq!(filtered_ids, vec!["routercpt_cp_a"]);
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
                    == "provider protocol is incompatible with route policy")
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
                            "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true},
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
                            "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true},
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
                            "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true},
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
            vec!["prvrsrc_openai_primary", "prvrsrc_openai_backup"]
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
            vec!["routepol_openai_chat_default", "routepol_acme_support"]
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
            vec!["cfgsnap_gateway_v1"]
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
        assert_eq!(
            ids(&route_receipts["data"], "route_receipt_id"),
            vec!["routercpt_acme_1"]
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
                        "capabilities":{"supports_streaming":true,"supports_tool_calling":true,"supports_json_mode":true},
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
        assert_eq!(ids(&providers["data"], "provider_resource_id").len(), 3);

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
        assert_eq!(ids(&route_policies["data"], "route_policy_id").len(), 3);

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
