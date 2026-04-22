use axum::{
    extract::{Path, State},
    http::{
        header::{COOKIE, SET_COOKIE},
        HeaderMap, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use core_domain::{
    AuthFlowId, AuthLoginResult, AuthProvider, AuthProviderAvailability, AuthProviderLink,
    AuthProviderLinkId, AuthProviderLinksResponse, AuthSession, AuthSessionId, AuthSessionResponse,
    AuthSessionState, EmailLoginCompleteRequest, EmailLoginStartRequest, EmailLoginStartResponse,
    EmailLoginVerificationMode, LogoutResponse, OAuthCallbackRequest, OAuthLoginStartRequest,
    OAuthLoginStartResponse, OAuthProvider, TenantId, TenantMembership, TenantMembershipId,
    TenantMembershipRole, TenantMembershipStatus, TenantSummary, UnlinkAuthProviderResponse,
    UserId, UserIdentity,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;

const CONTROL_PLANE_SERVICE_NAME: &str = "control-plane-api";
const EMAIL_BOOTSTRAP_CODE: &str = "111111";
const FRONTEND_BASE_URL: &str = "http://127.0.0.1:3000";
const SESSION_COOKIE_NAME: &str = "huge_router_session";
const SESSION_TTL_SECONDS: u64 = 60 * 60 * 8;

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(10_000);

#[derive(Clone, Debug)]
pub struct ControlPlaneState {
    frontend_base_url: String,
    store: Arc<RwLock<MemoryStore>>,
}

impl Default for ControlPlaneState {
    fn default() -> Self {
        Self {
            frontend_base_url: std::env::var("CONSOLE_WEB_BASE_URL")
                .unwrap_or_else(|_| FRONTEND_BASE_URL.to_string()),
            store: Arc::new(RwLock::new(MemoryStore::bootstrap())),
        }
    }
}

#[derive(Debug)]
struct MemoryStore {
    email_flows: HashMap<String, PendingEmailFlow>,
    oauth_flows: HashMap<String, PendingOAuthFlow>,
    provider_catalog: Vec<AuthProviderAvailability>,
    sessions: HashMap<String, AuthLoginResult>,
    users: HashMap<String, StoredUser>,
}

#[derive(Debug, Clone)]
struct PendingEmailFlow {
    email: String,
    workspace_slug: String,
}

#[derive(Debug, Clone)]
struct PendingOAuthFlow {
    provider: OAuthProvider,
    workspace_slug: String,
}

#[derive(Debug, Clone)]
struct StoredUser {
    identity: UserIdentity,
    links: Vec<AuthProviderLink>,
    memberships: Vec<TenantMembership>,
}

impl MemoryStore {
    fn bootstrap() -> Self {
        let platform_tenant = TenantSummary {
            id: TenantId::parse("tenant_platform".to_string()).expect("valid tenant id"),
            slug: "platform-admin".to_string(),
            display_name: "Platform Admin".to_string(),
        };
        let acme_tenant = TenantSummary {
            id: TenantId::parse("tenant_acme".to_string()).expect("valid tenant id"),
            slug: "acme-retail".to_string(),
            display_name: "Acme Retail".to_string(),
        };
        let northstar_tenant = TenantSummary {
            id: TenantId::parse("tenant_northstar".to_string()).expect("valid tenant id"),
            slug: "northstar-labs".to_string(),
            display_name: "Northstar Labs".to_string(),
        };

        let user = StoredUser {
            identity: UserIdentity {
                user_id: UserId::parse("user_ops".to_string()).expect("valid user id"),
                primary_email: Some("ops@huge-router.dev".to_string()),
                display_name: "Operations Admin".to_string(),
                avatar_url: None,
                created_at: "2026-04-22T00:00:00Z".to_string(),
                last_login_at: None,
            },
            links: vec![
                build_link(AuthProvider::Email, "ops@huge-router.dev", Some("ops@huge-router.dev"), false),
                build_link(AuthProvider::Github, "github_ops", Some("ops@huge-router.dev"), true),
                build_link(AuthProvider::Google, "google_ops", Some("ops@huge-router.dev"), true),
                build_link(AuthProvider::Wechat, "wechat_ops", Some("ops@huge-router.dev"), true),
            ],
            memberships: vec![
                build_membership("tmemb_platform", platform_tenant, TenantMembershipRole::Admin),
                build_membership("tmemb_acme", acme_tenant, TenantMembershipRole::Admin),
                build_membership("tmemb_northstar", northstar_tenant, TenantMembershipRole::Member),
            ],
        };

        let mut users = HashMap::new();
        users.insert("email:ops@huge-router.dev".to_string(), user.clone());
        users.insert("github:github_ops".to_string(), user.clone());
        users.insert("google:google_ops".to_string(), user.clone());
        users.insert("wechat:wechat_ops".to_string(), user);

        Self {
            email_flows: HashMap::new(),
            oauth_flows: HashMap::new(),
            provider_catalog: vec![
                provider_availability(AuthProvider::Email, "Continue with Email", "/api/control-plane/auth/email/start"),
                provider_availability(AuthProvider::Github, "Continue with GitHub", "/api/control-plane/auth/oauth/github/start"),
                provider_availability(AuthProvider::Google, "Continue with Google", "/api/control-plane/auth/oauth/google/start"),
                provider_availability(AuthProvider::Wechat, "Continue with WeChat", "/api/control-plane/auth/oauth/wechat/start"),
            ],
            sessions: HashMap::new(),
            users,
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
    status: StatusCode,
    trace_id: String,
}

#[derive(Debug, Clone)]
struct RequestContext {
    request_id: String,
    trace_id: String,
    sequence: u64,
}

pub fn app() -> Router {
    app_with_state(ControlPlaneState::default())
}

fn app_with_state(state: ControlPlaneState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/api/control-plane/auth/providers", get(get_auth_providers))
        .route("/api/control-plane/auth/session", get(get_current_session))
        .route("/api/control-plane/auth/email/start", post(start_email_login))
        .route("/api/control-plane/auth/email/complete", post(complete_email_login))
        .route("/api/control-plane/auth/oauth/{provider}/start", post(start_oauth_login))
        .route("/api/control-plane/auth/oauth/{provider}/callback", post(complete_oauth_login))
        .route("/api/control-plane/auth/logout", post(logout))
        .route("/api/control-plane/auth/links", get(list_auth_provider_links))
        .route("/api/control-plane/auth/links/{provider}", post(unlink_auth_provider).delete(unlink_auth_provider))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: CONTROL_PLANE_SERVICE_NAME,
        status: "ok",
    })
}

async fn get_auth_providers(
    State(state): State<ControlPlaneState>,
) -> Json<core_domain::AuthProvidersResponse> {
    Json(core_domain::AuthProvidersResponse {
        providers: state
            .store
            .read()
            .expect("store read lock should succeed")
            .provider_catalog
            .clone(),
    })
}

async fn get_current_session(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<AuthSessionResponse>, ApiError> {
    let session = resolve_session(&state, &headers);

    Ok(Json(AuthSessionResponse {
        session: session.map(|result| result.session),
    }))
}

async fn start_email_login(
    State(state): State<ControlPlaneState>,
    Json(request): Json<EmailLoginStartRequest>,
) -> Result<Json<EmailLoginStartResponse>, ApiError> {
    let context = next_request_context();
    let key = format!("email:{}", request.email.to_lowercase());
    ensure_known_user(&state, &key, &context)?;
    ensure_workspace_slug(&request.workspace_slug, &context)?;

    let flow_id = format!("authflow_{}", context.sequence);
    state
        .store
        .write()
        .expect("store write lock should succeed")
        .email_flows
        .insert(
            flow_id.clone(),
            PendingEmailFlow {
                email: request.email.to_lowercase(),
                workspace_slug: request.workspace_slug,
            },
        );

    Ok(Json(EmailLoginStartResponse {
        code_hint: Some(format!("Use local bootstrap verification code {EMAIL_BOOTSTRAP_CODE}.")),
        expires_at: iso_timestamp(now_unix_seconds() + 600),
        flow_id: AuthFlowId::parse(flow_id).expect("valid flow id"),
        verification_mode: EmailLoginVerificationMode::OneTimeCode,
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
        .write()
        .expect("store write lock should succeed")
        .email_flows
        .remove(request.flow_id.as_str())
        .ok_or_else(|| {
            ApiError::unauthorized(
                "auth_flow_missing",
                "email login flow was not found or has already been consumed".to_string(),
                &context,
            )
        })?;

    let login_result = issue_login_result(
        &state,
        &format!("email:{}", pending.email),
        AuthProvider::Email,
        &pending.workspace_slug,
        &context,
    )?;

    let session_id = login_result.session.session_id.as_str().to_string();

    Ok(with_session_cookie(&session_id, Json(login_result)))
}

async fn start_oauth_login(
    State(state): State<ControlPlaneState>,
    Path(provider): Path<String>,
    Json(request): Json<OAuthLoginStartRequest>,
) -> Result<Json<OAuthLoginStartResponse>, ApiError> {
    let context = next_request_context();
    let provider = parse_oauth_provider(&provider, &context)?;
    ensure_workspace_slug(&request.workspace_slug, &context)?;

    let state_token = format!("oauth_state_{}", context.sequence);
    state
        .store
        .write()
        .expect("store write lock should succeed")
        .oauth_flows
        .insert(
            state_token.clone(),
            PendingOAuthFlow {
                provider,
                workspace_slug: request.workspace_slug,
            },
        );

    let redirect_query = request
        .redirect_to
        .as_deref()
        .map(|redirect| format!("&redirect={redirect}"))
        .unwrap_or_default();

    Ok(Json(OAuthLoginStartResponse {
        authorization_url: format!(
            "{}/login/callback?provider={}&state={}&code=mock-{}-code{}",
            state.frontend_base_url,
            provider_slug(provider),
            state_token,
            provider_slug(provider),
            redirect_query
        ),
        expires_at: iso_timestamp(now_unix_seconds() + 600),
        provider,
        state: state_token,
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
        .write()
        .expect("store write lock should succeed")
        .oauth_flows
        .remove(&request.state)
        .ok_or_else(|| {
            ApiError::unauthorized(
                "auth_state_missing",
                "oauth state was not found or has already been consumed".to_string(),
                &context,
            )
        })?;

    if pending.provider != provider {
        return Err(ApiError::unauthorized(
            "auth_provider_mismatch",
            "oauth provider does not match the pending login state".to_string(),
            &context,
        ));
    }

    let login_result = issue_login_result(
        &state,
        &format!("{}:{}_ops", provider_slug(provider), provider_slug(provider)),
        AuthProvider::from(provider),
        &pending.workspace_slug,
        &context,
    )?;

    let session_id = login_result.session.session_id.as_str().to_string();

    Ok(with_session_cookie(&session_id, Json(login_result)))
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

    let removed = state
        .store
        .write()
        .expect("store write lock should succeed")
        .sessions
        .remove(&session_id);

    let logout_response = LogoutResponse {
        revoked: removed.is_some(),
        session_id: AuthSessionId::parse(session_id).expect("valid session id"),
    };

    Ok(clear_session_cookie(Json(logout_response)))
}

async fn list_auth_provider_links(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<AuthProviderLinksResponse>, ApiError> {
    let context = next_request_context();
    let session = require_session(&state, &headers, &context)?;

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
    let mut store = state.store.write().expect("store write lock should succeed");
    let session = store.sessions.get_mut(&session_id).ok_or_else(|| {
        ApiError::unauthorized(
            "auth_invalid",
            "HugeRouter session was not found".to_string(),
            &context,
        )
    })?;

    let removable = provider != AuthProvider::Email;
    let original_len = session.links.len();
    if removable {
        session.links.retain(|link| link.provider != provider);
    }

    Ok(Json(UnlinkAuthProviderResponse {
        provider,
        removed: removable && session.links.len() != original_len,
    }))
}

fn issue_login_result(
    state: &ControlPlaneState,
    user_key: &str,
    provider: AuthProvider,
    workspace_slug: &str,
    context: &RequestContext,
) -> Result<AuthLoginResult, ApiError> {
    let mut store = state.store.write().expect("store write lock should succeed");
    let user = store.users.get(user_key).cloned().ok_or_else(|| {
        ApiError::unauthorized(
            "auth_user_not_found",
            "no bootstrap user exists for the requested identity".to_string(),
            context,
        )
    })?;

    let active_tenant = user
        .memberships
        .iter()
        .find(|membership| membership.tenant.slug == workspace_slug)
        .or_else(|| user.memberships.first())
        .cloned()
        .ok_or_else(|| {
            ApiError::forbidden(
                "tenant_access_denied",
                format!("HugeRouter could not map this login to workspace `{workspace_slug}`"),
                context,
            )
        })?;

    let session = AuthSession {
        active_tenant_id: Some(active_tenant.tenant.id.clone()),
        authenticated_by: provider,
        created_at: iso_timestamp(now_unix_seconds()),
        expires_at: iso_timestamp(now_unix_seconds() + SESSION_TTL_SECONDS),
        last_authenticated_at: iso_timestamp(now_unix_seconds()),
        memberships: user.memberships.clone(),
        session_id: AuthSessionId::parse(format!("sess_{}", context.sequence))
            .expect("valid session id"),
        state: AuthSessionState::Active,
        user: user.identity.clone(),
    };

    let result = AuthLoginResult {
        links: user.links,
        session: session.clone(),
    };

    info!(
        request_id = context.request_id,
        trace_id = context.trace_id,
        session_id = session.session_id.as_str(),
        workspace_slug,
        provider = auth_provider_slug(provider),
        "issued HugeRouter auth session"
    );

    store
        .sessions
        .insert(session.session_id.as_str().to_string(), result.clone());

    Ok(result)
}

fn require_session(
    state: &ControlPlaneState,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<AuthLoginResult, ApiError> {
    resolve_session(state, headers).ok_or_else(|| {
        ApiError::unauthorized(
            "auth_invalid",
            "HugeRouter session is missing or expired".to_string(),
            context,
        )
    })
}

fn resolve_session(state: &ControlPlaneState, headers: &HeaderMap) -> Option<AuthLoginResult> {
    let session_id = extract_session_cookie(headers)?;
    state
        .store
        .read()
        .expect("store read lock should succeed")
        .sessions
        .get(&session_id)
        .cloned()
}

fn ensure_known_user(
    state: &ControlPlaneState,
    user_key: &str,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let store = state.store.read().expect("store read lock should succeed");
    if store.users.contains_key(user_key) {
        Ok(())
    } else {
        Err(ApiError::unauthorized(
            "auth_user_not_found",
            "no bootstrap user exists for this email".to_string(),
            context,
        ))
    }
}

fn ensure_workspace_slug(workspace_slug: &str, context: &RequestContext) -> Result<(), ApiError> {
    match workspace_slug {
        "platform-admin" | "acme-retail" | "northstar-labs" => Ok(()),
        _ => Err(ApiError::bad_request(
            "workspace_unknown",
            format!("workspace `{workspace_slug}` is not available in bootstrap mode"),
            context,
        )),
    }
}

fn parse_oauth_provider(
    provider: &str,
    context: &RequestContext,
) -> Result<OAuthProvider, ApiError> {
    match provider {
        "github" => Ok(OAuthProvider::Github),
        "google" => Ok(OAuthProvider::Google),
        "wechat" => Ok(OAuthProvider::Wechat),
        _ => Err(ApiError::bad_request(
            "provider_invalid",
            format!("unsupported OAuth provider `{provider}`"),
            context,
        )),
    }
}

fn parse_auth_provider(
    provider: &str,
    context: &RequestContext,
) -> Result<AuthProvider, ApiError> {
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
    (
        [(SET_COOKIE, build_session_cookie(session_id))],
        body,
    )
        .into_response()
}

fn clear_session_cookie<T>(body: Json<T>) -> Response
where
    T: Serialize,
{
    (
        [(SET_COOKIE, format!("{SESSION_COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"))],
        body,
    )
        .into_response()
}

fn build_session_cookie(session_id: &str) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={session_id}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_SECONDS}"
    )
}

fn provider_availability(
    provider: AuthProvider,
    display_name: &str,
    start_path: &str,
) -> AuthProviderAvailability {
    AuthProviderAvailability {
        display_name: display_name.to_string(),
        enabled: true,
        provider,
        reason_code: None,
        start_path: start_path.to_string(),
    }
}

fn build_link(
    provider: AuthProvider,
    provider_subject: &str,
    email: Option<&str>,
    can_unlink: bool,
) -> AuthProviderLink {
    AuthProviderLink {
        can_unlink,
        email: email.map(std::string::ToString::to_string),
        last_used_at: None,
        link_id: AuthProviderLinkId::parse(format!("authlink_{}", provider_subject))
            .expect("valid auth link id"),
        linked_at: "2026-04-22T00:00:00Z".to_string(),
        provider,
        provider_subject: provider_subject.to_string(),
    }
}

fn build_membership(
    membership_id: &str,
    tenant: TenantSummary,
    role: TenantMembershipRole,
) -> TenantMembership {
    TenantMembership {
        membership_id: TenantMembershipId::parse(membership_id.to_string()).expect("valid membership id"),
        role,
        status: TenantMembershipStatus::Active,
        tenant,
    }
}

fn provider_slug(provider: OAuthProvider) -> &'static str {
    match provider {
        OAuthProvider::Github => "github",
        OAuthProvider::Google => "google",
        OAuthProvider::Wechat => "wechat",
    }
}

fn auth_provider_slug(provider: AuthProvider) -> &'static str {
    match provider {
        AuthProvider::Email => "email",
        AuthProvider::Github => "github",
        AuthProvider::Google => "google",
        AuthProvider::Wechat => "wechat",
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_secs()
}

fn iso_timestamp(unix_seconds: u64) -> String {
    format!("{unix_seconds}Z")
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
            status: StatusCode::BAD_REQUEST,
            trace_id: context.trace_id.clone(),
        }
    }

    fn forbidden(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: StatusCode::FORBIDDEN,
            trace_id: context.trace_id.clone(),
        }
    }

    fn unauthorized(code: &'static str, message: String, context: &RequestContext) -> Self {
        Self {
            code,
            message,
            request_id: context.request_id.clone(),
            status: StatusCode::UNAUTHORIZED,
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
    use super::app;
    use axum::{
        body::{to_bytes, Body},
        http::{header::SET_COOKIE, Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let response = app()
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn email_login_flow_sets_cookie_and_returns_session() {
        let app = app();

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

        let start_body: Value =
            serde_json::from_slice(&to_bytes(start_response.into_body(), usize::MAX).await.unwrap())
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

        let body: Value =
            serde_json::from_slice(&to_bytes(complete_response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["session"]["authenticatedBy"], "email");
        assert_eq!(body["links"][0]["provider"], "email");
    }

    #[tokio::test]
    async fn oauth_flow_round_trips_and_returns_absolute_callback_url() {
        let app = app();

        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/control-plane/auth/oauth/github/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "redirectTo": "/admin/tenants",
                            "workspaceSlug": "platform-admin"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let start_body: Value =
            serde_json::from_slice(&to_bytes(start_response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert!(start_body["authorizationUrl"]
            .as_str()
            .unwrap()
            .starts_with("http://127.0.0.1:3000/login/callback"));

        let callback_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/control-plane/auth/oauth/github/callback")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "code": "mock-github-code",
                            "state": start_body["state"].as_str().unwrap()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body: Value =
            serde_json::from_slice(&to_bytes(callback_response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["session"]["authenticatedBy"], "github");
    }

    #[tokio::test]
    async fn session_and_logout_use_cookie_based_identity() {
        let app = app();

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
                            "workspaceSlug": "acme-retail"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let start_body: Value =
            serde_json::from_slice(&to_bytes(start_response.into_body(), usize::MAX).await.unwrap())
                .unwrap();

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
                            "flowId": start_body["flowId"].as_str().unwrap()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let cookie = complete_response
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();

        let session_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/control-plane/auth/session")
                    .header("cookie", cookie.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let session_body: Value =
            serde_json::from_slice(&to_bytes(session_response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(session_body["session"]["activeTenantId"], "tenant_acme");

        let logout_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/control-plane/auth/logout")
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(logout_response.status(), StatusCode::OK);
        assert_eq!(
            logout_response.headers().get(SET_COOKIE).unwrap().to_str().unwrap(),
            "huge_router_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"
        );
    }
}
