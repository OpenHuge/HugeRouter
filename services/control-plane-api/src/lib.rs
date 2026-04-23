mod store;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue, Method,
        header::{COOKIE, SET_COOKIE},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use core_domain::{
    AuthProvider, AuthProviderLinksResponse, AuthSessionResponse, EmailLoginCompleteRequest,
    EmailLoginStartRequest, EmailLoginStartResponse, OAuthCallbackRequest, OAuthLoginStartRequest,
    OAuthLoginStartResponse, Project, ProviderResource, RoutePolicy, Tenant, TenantMembership,
    TenantMembershipRole, TenantMembershipStatus, UnlinkAuthProviderResponse,
};
use protocol_ir::{
    ConfigSnapshotResponse, ProjectsResponse, ProviderResourcesResponse, RoutePoliciesResponse,
    RouteReceiptResponse, RouteSimulationRequest, RouteSimulationResponse, TenantsResponse,
};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use store::{
    EMAIL_BOOTSTRAP_CODE, IdentityLookup, SESSION_TTL_SECONDS, StoreMode, ensure_workspace_slug,
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
            store: StoreMode::from_env().await?,
        })
    }

    #[cfg(test)]
    fn memory() -> Self {
        Self {
            frontend_base_url: FRONTEND_BASE_URL.to_string(),
            store: StoreMode::memory(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
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
        .route("/v1/provider-resources", get(list_provider_resources))
        .route(
            "/v1/provider-resources/{provider_resource_id}",
            get(get_provider_resource),
        )
        .route("/v1/route-policies", get(list_route_policies))
        .route(
            "/v1/config-snapshots/{config_snapshot_id}",
            get(get_config_snapshot),
        )
        .route(
            "/v1/config-snapshots/{config_snapshot_id}/activate",
            post(activate_config_snapshot),
        )
        .route("/v1/route-simulations", post(create_route_simulation))
        .route(
            "/v1/route-receipts/{route_receipt_id}",
            get(get_route_receipt),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(allow_origin)
                .allow_credentials(true)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
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
        authorization_url: format!(
            "{}/login/callback?provider={}&state={}&code=mock-{}-code{}",
            state.frontend_base_url,
            oauth_provider_slug(provider),
            state_token,
            oauth_provider_slug(provider),
            redirect_query,
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

    if !request.code.starts_with("mock-") {
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

    let subject = format!("{}_ops", oauth_provider_slug(provider));
    let login_result = state
        .store
        .issue_session(
            &format!("sess_{}", context.sequence),
            AuthProvider::from(provider),
            &IdentityLookup::ProviderSubject(AuthProvider::from(provider), subject),
            &pending.workspace_slug,
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
    let current_snapshot = state
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
    authz.ensure_manage_tenant(
        current_snapshot.config_snapshot.tenant_id.as_str(),
        &context,
    )?;

    let snapshot = state
        .store
        .activate_config_snapshot(current_snapshot.config_snapshot.config_snapshot_id.as_str())
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

async fn create_route_simulation(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<RouteSimulationRequest>,
) -> Result<Json<RouteSimulationResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let project = state
        .store
        .get_project(request.project_id.as_str())
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load project: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "project_not_found",
                format!("project `{}` was not found", request.project_id),
                &context,
            )
        })?;
    if project.tenant_id != request.tenant_id {
        return Err(ApiError::forbidden(
            "tenant_access_denied",
            format!(
                "project `{}` does not belong to tenant `{}`",
                request.project_id, request.tenant_id
            ),
            &context,
        ));
    }
    authz.ensure_read_tenant(project.tenant_id.as_str(), &context)?;

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

fn parse_oauth_provider(
    provider: &str,
    context: &RequestContext,
) -> Result<core_domain::OAuthProvider, ApiError> {
    match provider {
        "github" => Ok(core_domain::OAuthProvider::Github),
        "google" => Ok(core_domain::OAuthProvider::Google),
        "wechat" => Ok(core_domain::OAuthProvider::Wechat),
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
        _ => Err(ApiError::bad_request(
            "provider_invalid",
            format!("unsupported auth provider `{provider}`"),
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
    use super::{ControlPlaneState, FRONTEND_BASE_URL, app_with_state};
    use axum::{
        body::{Body, to_bytes},
        http::{
            Request, StatusCode,
            header::{COOKIE, SET_COOKIE},
        },
    };
    use core_domain::{
        AdmissionResult, AuthProvider, AuthProviderLink, AuthProviderLinkId, ConfigSnapshotId,
        ExcludedTarget, FallbackTransition, ProjectId, ProviderResourceId, RouteReceipt,
        RouteReceiptId, ScoreBreakdown, TenantMembership, TenantMembershipId, TenantMembershipRole,
        TenantMembershipStatus, TenantSummary, UserId, UserIdentity,
    };
    use serde_json::{Value, json};
    use std::sync::{Arc, RwLock};
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
    async fn anonymous_access_to_v1_routes_is_rejected() {
        let app = authz_test_app();
        let requests = vec![
            request("GET", "/v1/tenants", None, None),
            request("GET", "/v1/projects", None, None),
            request("GET", "/v1/provider-resources", None, None),
            request(
                "GET",
                "/v1/provider-resources/prvrsrc_openai_primary",
                None,
                None,
            ),
            request("GET", "/v1/route-policies", None, None),
            request("GET", "/v1/config-snapshots/cfgsnap_gateway_v1", None, None),
            request(
                "POST",
                "/v1/config-snapshots/cfgsnap_gateway_v1/activate",
                None,
                None,
            ),
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
            request("GET", "/v1/route-receipts/routercpt_acme_1", None, None),
        ];

        for request in requests {
            let response = app.clone().oneshot(request).await.unwrap();
            assert_error(response, StatusCode::UNAUTHORIZED, "auth_invalid").await;
        }
    }

    #[tokio::test]
    async fn tenant_user_only_sees_membership_scoped_data() {
        let app = authz_test_app();
        let cookie = login_cookie(&app, "acme-admin@huge-router.dev", "acme-retail").await;

        let tenants = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/tenants", Some(&cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ids(&tenants["data"], "tenant_id"), vec!["tenant_acme"]);

        let projects = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/projects", Some(&cookie), None))
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
                    Some(&cookie),
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
                .oneshot(request("GET", "/v1/route-policies", Some(&cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            ids(&route_policies["data"], "route_policy_id"),
            vec!["routepol_openai_chat_default", "routepol_acme_support"]
        );

        let snapshot = app
            .clone()
            .oneshot(request(
                "GET",
                "/v1/config-snapshots/cfgsnap_gateway_v1",
                Some(&cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(snapshot.status(), StatusCode::OK);
        let snapshot_body = response_json(snapshot).await;
        assert_eq!(
            snapshot_body["config_snapshot"]["config_snapshot_id"],
            "cfgsnap_gateway_v1"
        );

        let receipt = app
            .clone()
            .oneshot(request(
                "GET",
                "/v1/route-receipts/routercpt_acme_1",
                Some(&cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(receipt.status(), StatusCode::OK);
        let receipt_body = response_json(receipt).await;
        assert_eq!(
            receipt_body["route_receipt"]["route_receipt_id"],
            "routercpt_acme_1"
        );

        let response = app
            .clone()
            .oneshot(request(
                "POST",
                "/v1/route-simulations",
                Some(&cookie),
                Some(route_simulation_body(
                    "tenant_acme",
                    "proj_core",
                    "reasoning-fast",
                )),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["selected_target"], "prvrsrc_openai_primary");
    }

    #[tokio::test]
    async fn tenant_user_cross_tenant_reads_and_writes_are_rejected() {
        let app = authz_test_app();
        let admin_cookie = login_cookie(&app, "acme-admin@huge-router.dev", "acme-retail").await;
        let member_cookie = login_cookie(&app, "acme-member@huge-router.dev", "acme-retail").await;

        assert_error(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/provider-resources/prvrsrc_openai_research",
                    Some(&admin_cookie),
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
                    Some(&admin_cookie),
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
                    Some(&admin_cookie),
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
                    "/v1/config-snapshots/cfgsnap_research_v1/activate",
                    Some(&admin_cookie),
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
                    "/v1/route-simulations",
                    Some(&admin_cookie),
                    Some(route_simulation_body(
                        "tenant_northstar",
                        "proj_ns_research",
                        "research-fast",
                    )),
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
                    "/v1/config-snapshots/cfgsnap_gateway_v1/activate",
                    Some(&member_cookie),
                    None,
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
        let app = authz_test_app();
        let cookie = login_cookie(&app, "ops@huge-router.dev", "platform-admin").await;

        let tenants = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/tenants", Some(&cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ids(&tenants["data"], "tenant_id").len(), 3);

        let provider_resources = response_json(
            app.clone()
                .oneshot(request(
                    "GET",
                    "/v1/provider-resources",
                    Some(&cookie),
                    None,
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            ids(&provider_resources["data"], "provider_resource_id").len(),
            3
        );

        let route_policies = response_json(
            app.clone()
                .oneshot(request("GET", "/v1/route-policies", Some(&cookie), None))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ids(&route_policies["data"], "route_policy_id").len(), 3);

        let northstar_snapshot = app
            .clone()
            .oneshot(request(
                "GET",
                "/v1/config-snapshots/cfgsnap_research_v1",
                Some(&cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(northstar_snapshot.status(), StatusCode::OK);

        let northstar_receipt = app
            .clone()
            .oneshot(request(
                "GET",
                "/v1/route-receipts/routercpt_northstar_1",
                Some(&cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(northstar_receipt.status(), StatusCode::OK);

        let activate = app
            .clone()
            .oneshot(request(
                "POST",
                "/v1/config-snapshots/cfgsnap_research_v1/activate",
                Some(&cookie),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(activate.status(), StatusCode::OK);
        let activate_body = response_json(activate).await;
        assert_eq!(activate_body["config_snapshot"]["status"], "active");
    }

    fn authz_test_app() -> axum::Router {
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

        app_with_state(ControlPlaneState {
            frontend_base_url: FRONTEND_BASE_URL.to_string(),
            store: crate::store::StoreMode::Memory(Arc::new(RwLock::new(store))),
        })
    }

    fn user_seed(
        user_id: &str,
        email: &str,
        display_name: &str,
        memberships: Vec<TenantMembership>,
    ) -> crate::store::UserSeed {
        crate::store::UserSeed {
            user: UserIdentity {
                user_id: UserId::parse(user_id.to_string()).unwrap(),
                primary_email: Some(email.to_string()),
                display_name: display_name.to_string(),
                avatar_url: None,
                created_at: "2026-04-22T00:00:00Z".to_string(),
                last_login_at: None,
            },
            identities: vec![crate::store::UserIdentityKey::Email(email.to_string())],
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
            route_receipt_id: RouteReceiptId::parse(route_receipt_id.to_string()).unwrap(),
            tenant_id: core_domain::TenantId::parse(tenant_id.to_string()).unwrap(),
            project_id: ProjectId::parse(project_id.to_string()).unwrap(),
            request_id: format!("req_{route_receipt_id}"),
            trace_id: format!("trace_{route_receipt_id}"),
            protocol_family: "openai_chat".to_string(),
            model_alias: model_alias.to_string(),
            config_snapshot_id: ConfigSnapshotId::parse(config_snapshot_id.to_string()).unwrap(),
            admission_result: AdmissionResult::Admitted,
            selected_target: Some(
                ProviderResourceId::parse(provider_resource_id.to_string()).unwrap(),
            ),
            excluded_targets: Vec::<ExcludedTarget>::new(),
            score_breakdown: ScoreBreakdown {
                latency: 0.95,
                cost: 0.8,
                health: 1.0,
                trust: 1.0,
            },
            fallback_transitions: Vec::<FallbackTransition>::new(),
            normalized_error: None,
            created_at: "2026-04-22T00:00:00Z".to_string(),
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

    async fn login_cookie(app: &axum::Router, email: &str, workspace_slug: &str) -> String {
        let start = app
            .clone()
            .oneshot(request(
                "POST",
                "/api/control-plane/auth/email/start",
                None,
                Some(json!({
                    "email": email,
                    "workspaceSlug": workspace_slug,
                })),
            ))
            .await
            .unwrap();
        assert_eq!(start.status(), StatusCode::OK);
        let start_body = response_json(start).await;
        let flow_id = start_body["flowId"].as_str().unwrap();

        let complete = app
            .clone()
            .oneshot(request(
                "POST",
                "/api/control-plane/auth/email/complete",
                None,
                Some(json!({
                    "code": "111111",
                    "flowId": flow_id,
                })),
            ))
            .await
            .unwrap();
        assert_eq!(complete.status(), StatusCode::OK);
        complete
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
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
}
