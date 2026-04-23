mod store;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
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
    OAuthLoginStartResponse, UnlinkAuthProviderResponse,
};
use protocol_ir::{
    ConfigSnapshotResponse, ProjectsResponse, ProviderResourcesResponse, RouteDiagnosticsResponse,
    RoutePoliciesResponse, RouteReceiptResponse, RouteReceiptsResponse, RouteSimulationRequest,
    RouteSimulationResponse, TenantsResponse,
};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use store::{
    EMAIL_BOOTSTRAP_CODE, IdentityLookup, ProviderResourceFilters, RouteReceiptFilters,
    SESSION_TTL_SECONDS, StoreMode, ensure_workspace_slug, expires_at, now_rfc3339,
    oauth_provider_slug,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

const CONTROL_PLANE_SERVICE_NAME: &str = "control-plane-api";
const FRONTEND_BASE_URL: &str = "http://127.0.0.1:3000";
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
        .route("/v1/route-receipts", get(list_route_receipts))
        .route(
            "/v1/route-receipts/{route_receipt_id}",
            get(get_route_receipt),
        )
        .route(
            "/v1/route-diagnostics/{route_policy_id}",
            get(get_route_diagnostics),
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
) -> Result<Json<TenantsResponse>, ApiError> {
    Ok(Json(state.store.list_tenants().await.map_err(|error| {
        ApiError::internal(
            "storage_unavailable",
            format!("failed to load tenants: {error}"),
            &next_request_context(),
        )
    })?))
}

async fn list_projects(
    State(state): State<ControlPlaneState>,
) -> Result<Json<ProjectsResponse>, ApiError> {
    Ok(Json(state.store.list_projects().await.map_err(
        |error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load projects: {error}"),
                &next_request_context(),
            )
        },
    )?))
}

async fn list_provider_resources(
    State(state): State<ControlPlaneState>,
    Query(filters): Query<ProviderResourceFilters>,
) -> Result<Json<ProviderResourcesResponse>, ApiError> {
    Ok(Json(
        state
            .store
            .list_provider_resources(&filters)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to load provider resources: {error}"),
                    &next_request_context(),
                )
            })?,
    ))
}

async fn list_route_receipts(
    State(state): State<ControlPlaneState>,
    Query(filters): Query<RouteReceiptFilters>,
) -> Result<Json<RouteReceiptsResponse>, ApiError> {
    Ok(Json(
        state
            .store
            .list_route_receipts(&filters)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to load route receipts: {error}"),
                    &next_request_context(),
                )
            })?,
    ))
}

async fn get_provider_resource(
    State(state): State<ControlPlaneState>,
    Path(provider_resource_id): Path<String>,
) -> Result<Json<core_domain::ProviderResource>, ApiError> {
    let context = next_request_context();
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
    Ok(Json(provider_resource))
}

async fn list_route_policies(
    State(state): State<ControlPlaneState>,
) -> Result<Json<RoutePoliciesResponse>, ApiError> {
    Ok(Json(state.store.list_route_policies().await.map_err(
        |error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route policies: {error}"),
                &next_request_context(),
            )
        },
    )?))
}

async fn get_config_snapshot(
    State(state): State<ControlPlaneState>,
    Path(config_snapshot_id): Path<String>,
) -> Result<Json<ConfigSnapshotResponse>, ApiError> {
    let context = next_request_context();
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
    Ok(Json(snapshot))
}

async fn activate_config_snapshot(
    State(state): State<ControlPlaneState>,
    Path(config_snapshot_id): Path<String>,
) -> Result<Json<ConfigSnapshotResponse>, ApiError> {
    let context = next_request_context();
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

async fn create_route_simulation(
    State(state): State<ControlPlaneState>,
    Json(request): Json<RouteSimulationRequest>,
) -> Result<Json<RouteSimulationResponse>, ApiError> {
    let context = next_request_context();
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
    Path(route_receipt_id): Path<String>,
) -> Result<Json<RouteReceiptResponse>, ApiError> {
    let context = next_request_context();
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
    Ok(Json(receipt))
}

async fn get_route_diagnostics(
    State(state): State<ControlPlaneState>,
    Path(route_policy_id): Path<String>,
) -> Result<Json<RouteDiagnosticsResponse>, ApiError> {
    let context = next_request_context();
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
    Ok(Json(diagnostics))
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
    use super::{ControlPlaneState, app_with_state};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::SET_COOKIE},
    };
    use serde_json::Value;
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
        let app = app_with_state(ControlPlaneState::memory());

        let tenants = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/tenants")
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
    async fn provider_resource_filters_and_route_receipts_are_operator_visible() {
        let app = app_with_state(ControlPlaneState::memory());

        let provider_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/provider-resources?capability=realtime&transit_gateway=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(provider_response.status(), StatusCode::OK);
        let provider_body: Value = serde_json::from_slice(
            &to_bytes(provider_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(provider_body["data"].as_array().unwrap().len(), 1);
        assert_eq!(
            provider_body["data"][0]["provider_resource_id"],
            "prvrsrc_transit_relay"
        );

        let receipts_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/route-receipts?route_policy_id=routepol_acme_realtime&limit=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(receipts_response.status(), StatusCode::OK);
        let receipts_body: Value = serde_json::from_slice(
            &to_bytes(receipts_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(receipts_body["data"].as_array().unwrap().len(), 1);
        assert_eq!(
            receipts_body["data"][0]["failure_reason"],
            "No healthy target satisfied realtime_webrtc plus required tool-related capabilities."
        );
        assert_eq!(
            receipts_body["data"][0]["excluded_targets"][0]["reason_code"],
            "capability_gap_realtime"
        );
    }

    #[tokio::test]
    async fn route_diagnostics_connect_capabilities_health_and_recent_receipts() {
        let app = app_with_state(ControlPlaneState::memory());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/route-diagnostics/routepol_acme_realtime")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(
            body["route_policy"]["route_policy_id"],
            "routepol_acme_realtime"
        );
        assert_eq!(
            body["last_route_receipt"]["admission_result"],
            "rejected_no_candidate"
        );
        assert_eq!(
            body["targets"][2]["provider_resource"]["provider_resource_id"],
            "prvrsrc_transit_relay"
        );
        assert_eq!(body["targets"][2]["reason_code"], "health_draining");
    }

    #[tokio::test]
    async fn route_simulation_returns_selected_target() {
        let app = app_with_state(ControlPlaneState::memory());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-simulations")
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
}
