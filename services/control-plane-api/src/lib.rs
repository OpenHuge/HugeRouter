#![allow(clippy::too_many_lines, clippy::uninlined_format_args)]

mod store;

use anyhow::Result;
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
    OAuthCallbackRequest, OAuthLoginStartRequest, OAuthLoginStartResponse, ProviderResource,
    ProviderResourceId, RoutePolicy, RoutePolicyId, UnlinkAuthProviderResponse,
};
use protocol_ir::{
    ConfigSnapshotResponse, ProjectsResponse, ProviderResourcesResponse, RoutePoliciesResponse,
    RouteReceiptResponse, RouteSimulationRequest, RouteSimulationResponse, TenantsResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use store::{
    ApiKey, ApiKeysResponse, ConcurrencyResult, ConfigSnapshotsResponse, EMAIL_BOOTSTRAP_CODE,
    IdentityLookup, RouteReceiptsResponse, SESSION_TTL_SECONDS, StoreMode,
    ensure_workspace_slug, expires_at, now_rfc3339, oauth_provider_slug,
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
) -> Result<Json<ProviderResourcesResponse>, ApiError> {
    Ok(Json(state.store.list_provider_resources().await.map_err(
        |error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load provider resources: {error}"),
                &next_request_context(),
            )
        },
    )?))
}

async fn create_provider_resource(
    State(state): State<ControlPlaneState>,
    Json(provider_resource): Json<ProviderResource>,
) -> Result<Json<ProviderResource>, ApiError> {
    let context = next_request_context();
    provider_resource.validate().map_err(|error| {
        ApiError::bad_request(
            "provider_resource_invalid",
            format!("provider resource validation failed: {error}"),
            &context,
        )
    })?;
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
    Path(provider_resource_id): Path<String>,
    Json(request): Json<ProviderResourceUpdateRequest>,
) -> Result<Json<ProviderResource>, ApiError> {
    let context = next_request_context();
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
    Path(provider_resource_id): Path<String>,
    Json(request): Json<ConcurrencyRequest>,
) -> Result<Json<ProviderResource>, ApiError> {
    let context = next_request_context();
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

async fn create_route_policy(
    State(state): State<ControlPlaneState>,
    Json(route_policy): Json<RoutePolicy>,
) -> Result<Json<RoutePolicy>, ApiError> {
    let context = next_request_context();
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
    Path(route_policy_id): Path<String>,
    Json(request): Json<RoutePolicyUpdateRequest>,
) -> Result<Json<RoutePolicy>, ApiError> {
    let context = next_request_context();
    let route_policy_id = RoutePolicyId::parse(route_policy_id).map_err(|error| {
        ApiError::bad_request(
            "route_policy_id_invalid",
            format!("invalid route_policy_id: {error}"),
            &context,
        )
    })?;

    let mut route_policy = request.route_policy;
    route_policy.route_policy_id = route_policy_id.clone();
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
    Path(route_policy_id): Path<String>,
    Json(request): Json<ConcurrencyRequest>,
) -> Result<Json<RoutePolicy>, ApiError> {
    let context = next_request_context();
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
) -> Result<Json<ConfigSnapshotsResponse>, ApiError> {
    Ok(Json(state.store.list_config_snapshots().await.map_err(
        |error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load config snapshots: {error}"),
                &next_request_context(),
            )
        },
    )?))
}

async fn create_config_snapshot(
    State(state): State<ControlPlaneState>,
    Json(config_snapshot): Json<ConfigSnapshot>,
) -> Result<Json<ConfigSnapshot>, ApiError> {
    let context = next_request_context();
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
    let _session = require_platform_admin_session(&state, &headers, &context).await?;
    Ok(Json(state.store.list_api_keys().await.map_err(
        |error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to list API keys: {error}"),
                &context,
            )
        },
    )?))
}

async fn create_api_key(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKey>, ApiError> {
    let context = next_request_context();
    let _session = require_platform_admin_session(&state, &headers, &context).await?;
    let provider_resource_id =
        ProviderResourceId::parse(&request.provider_resource_id).map_err(|_| {
            ApiError::bad_request(
                "provider_resource_id_invalid",
                "provider_resource_id is not valid".to_string(),
                &context,
            )
        })?;

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
    let _session = require_platform_admin_session(&state, &headers, &context).await?;
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
    require_internal_gateway_auth(&headers, &context)?;
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

async fn list_route_receipts(
    State(state): State<ControlPlaneState>,
    Query(query): Query<RouteReceiptsQuery>,
) -> Result<Json<RouteReceiptsResponse>, ApiError> {
    let context = next_request_context();
    Ok(Json(
        state
            .store
            .list_route_receipts(
                query.tenant_id.filter(|value| !value.trim().is_empty()),
                query.project_id.filter(|value| !value.trim().is_empty()),
                query
                    .protocol_family
                    .filter(|value| !value.trim().is_empty()),
            )
            .await
            .map_err(|error| {
                ApiError::internal(
                    "storage_unavailable",
                    format!("failed to load route receipts: {error}"),
                    &context,
                )
            })?,
    ))
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

async fn require_platform_admin_session(
    state: &ControlPlaneState,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<core_domain::AuthLoginResult, ApiError> {
    let session = require_session(state, headers, context).await?;
    let is_platform_admin = session.session.memberships.iter().any(|membership| {
        membership.tenant.slug == "platform-admin"
            && matches!(
                membership.role,
                core_domain::TenantMembershipRole::Owner
                    | core_domain::TenantMembershipRole::Admin
            )
    });

    if !is_platform_admin {
        return Err(ApiError::forbidden(
            "forbidden",
            "platform-admin membership is required for API key management".to_string(),
            context,
        ));
    }

    Ok(session)
}

#[allow(clippy::result_large_err)]
fn require_internal_gateway_auth(
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let configured_token = std::env::var("CONTROL_PLANE_INTERNAL_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "dev-internal-token".to_string());
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
    use super::{ControlPlaneState, app_with_state};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::{AUTHORIZATION, COOKIE, SET_COOKIE}},
    };
    use core_domain::{
        AdmissionResult, AuthProvider, ConfigSnapshotId, ExcludedTarget, ProjectId, ProviderResourceId,
        RouteReceipt, ScoreBreakdown, TenantId,
    };
    use serde_json::Value;
    use crate::store::IdentityLookup;

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
        state.store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_cp_a",
            "tenant_acme",
            "proj_core",
            "openai_chat",
            "2026-04-22T00:01:00Z",
        ));
        state.store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_cp_b",
            "tenant_acme",
            "proj_core",
            "openai_responses",
            "2026-04-22T00:03:00Z",
        ));
        state.store.insert_route_receipt_for_tests(sample_route_receipt(
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
    async fn create_route_policy_rejects_unsupported_protocol_and_capability() {
        let app = app_with_state(ControlPlaneState::memory());

        let unsupported_protocol = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-policies")
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
        let app = app_with_state(ControlPlaneState::memory());

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/provider-resources")
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
        let app = app_with_state(ControlPlaneState::memory());

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/route-policies")
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
        let app = app_with_state(ControlPlaneState::memory());

        let before_list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/config-snapshots")
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
                            "route_policy_id":"routepol_default",
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
                    .header(AUTHORIZATION, "Bearer dev-internal-token")
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
                    .header(AUTHORIZATION, "Bearer dev-internal-token")
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
