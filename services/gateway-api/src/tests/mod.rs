use super::{
    ActiveConfigStore, ActiveGatewayConfig, ApiKeyScopeStore, AppState, ChatCompletionRequest,
    ChatMessage, ControlPlaneApiKeyStore, ControlPlaneConfigStore, GatewayApiKeyResolveRequest,
    GatewayApiKeyResolveResponse, GatewayApiKeyScope, GatewayState, ImageGenerationRequest,
    InMemoryRouteTokenStore, InternalGatewayConfigResponse, ProviderTargetRuntime, RequestContext,
    ResponsesApiInputContent, ResponsesApiInputMessage, ResponsesApiRequest, RuntimeEventSink,
    StaticBudgetProjectionStore, StaticConfigStore, app_with_state, composition, evaluate_route,
    normalize_request, normalize_responses_request,
};
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::Json as ExtractJson,
    extract::State,
    http::{Request, StatusCode},
    routing::{get, post},
};
use core_domain::{
    AuthKind, BudgetPolicyId, ConfigSnapshot, ConfigSnapshotId, ConfigSnapshotStatus,
    CredentialOwnerType, DeploymentScope, HealthState, MonetaryAmount, ProjectId, ProvenanceClass,
    ProviderResource, ProviderResourceId, ProviderResourceStatus, RoutePolicy, RoutePolicyId,
    RouteReceipt, TenantId, UsageEvent,
};
use protocol_ir::{BalanceProjectionResponse, RouteReceiptRecorded};
use provider_gateway::{
    GatewayAdapter, HttpRequest as GatewayHttpRequest, HttpResponse as GatewayHttpResponse,
    HttpTransport as GatewayHttpTransport,
};
use provider_traits::{
    AdapterLifecycleFamily, AdapterManifest, AdapterStability,
    CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION, ProviderAdapter, ProviderAdapterRegistry,
    ProviderError, ProviderErrorKind, ProviderExecutionContext, ProviderImageData,
    ProviderImageRequest, ProviderImageResponse, ProviderRequest, ProviderResponse, ProviderUsage,
    StreamingSupport,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering as AtomicOrdering},
    },
    time::Duration,
};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::ServiceExt;

fn valid_http_request() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "reasoning-fast".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "hello router".to_string(),
        }],
        stream: false,
    }
}

fn valid_responses_request() -> ResponsesApiRequest {
    ResponsesApiRequest {
        model: "reasoning-fast".to_string(),
        input: vec![ResponsesApiInputMessage {
            role: "user".to_string(),
            content: vec![ResponsesApiInputContent {
                kind: "input_text".to_string(),
                text: "hello router".to_string(),
            }],
        }],
        stream: false,
    }
}

fn valid_image_generation_request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        model: "chatgpt-image-2".to_string(),
        prompt: "draw a reliable router appliance".to_string(),
        n: Some(1),
        size: Some("1024x1024".to_string()),
        quality: Some("auto".to_string()),
        response_format: Some("b64_json".to_string()),
    }
}

fn request_context() -> super::RequestContext {
    super::RequestContext {
        request_id: "req_test".to_string(),
        trace_id: "trace_test".to_string(),
        sequence: 42,
    }
}

fn ok_budget_projection() -> BalanceProjectionResponse {
    BalanceProjectionResponse {
        data: protocol_ir::BalanceProjection {
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
            currency: "USD".to_string(),
            provider_cost_total: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.00".to_string(),
            },
            billable_total: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.00".to_string(),
            },
            configured_budget: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "1000.00".to_string(),
            },
            remaining_budget: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "1000.00".to_string(),
            },
            threshold_status: "ok".to_string(),
            last_projected_at: "2026-04-20T00:00:00Z".to_string(),
            projection_lag_seconds: 0,
        },
    }
}

fn default_route_policy() -> RoutePolicy {
    let tenant_id = TenantId::parse("tenant_acme").unwrap();
    RoutePolicy {
        route_policy_id: RoutePolicyId::parse("routepol_default").unwrap(),
        tenant_id,
        display_name: "default".to_string(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: vec!["json_mode".to_string()],
        preferred_regions: vec!["us-east-1".to_string()],
        version: 1,
        created_at: "2026-04-20T00:00:00Z".to_string(),
        updated_at: "2026-04-20T00:00:00Z".to_string(),
    }
}

fn build_config(targets: Vec<ProviderTargetRuntime>) -> ActiveGatewayConfig {
    let tenant_id = TenantId::parse("tenant_acme").unwrap();
    let project_id = ProjectId::parse("proj_core").unwrap();
    let route_policy = default_route_policy();

    ActiveGatewayConfig {
        config_snapshot: ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_test").unwrap(),
            tenant_id,
            project_id,
            revision: 1,
            status: ConfigSnapshotStatus::Active,
            activated_at: Some("2026-04-20T00:00:00Z".to_string()),
            provider_resource_ids: targets
                .iter()
                .map(|target| target.resource.provider_resource_id.clone())
                .collect(),
            route_policy_id: route_policy.route_policy_id.clone(),
            budget_policy_id: BudgetPolicyId::parse("budgetpol_test").unwrap(),
        },
        route_policy,
        provider_targets: targets,
    }
}

fn build_target(
    provider_resource_id: &str,
    region: &str,
    latency: f32,
    cost: f32,
    health_state: HealthState,
) -> ProviderTargetRuntime {
    let resource = ProviderResource {
        provider_resource_id: ProviderResourceId::parse(provider_resource_id).unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: Some(ProjectId::parse("proj_core").unwrap()),
        provider_id: "openai".to_string(),
        name: provider_resource_id.to_string(),
        status: ProviderResourceStatus::Active,
        provenance_class: ProvenanceClass::OfficialApi,
        credential_owner_type: CredentialOwnerType::Platform,
        deployment_scope: DeploymentScope::Shared,
        region: region.to_string(),
        endpoint_base_url: "https://api.openai.example/v1".to_string(),
        auth_kind: AuthKind::ApiKey,
        health_state,
        health_message: Some("test health".to_string()),
        quarantine_reason: None,
        budget_policy_id: None,
        capabilities: core_domain::ProviderCapabilities {
            supports_streaming: true,
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
        is_transit_gateway: false,
        version: 1,
        created_at: "2026-04-20T00:00:00Z".to_string(),
        updated_at: "2026-04-20T00:00:00Z".to_string(),
    };

    ProviderTargetRuntime {
        target_kind: super::provider_target_kind(&resource.provider_id),
        transit_metadata: None,
        resource,
        priority: 1,
        upstream_model: Some("gpt-4.1-mini".to_string()),
        api_key: "secret".to_string(),
        static_latency_score: latency,
        static_cost_score: cost,
        usd_per_1k_tokens: 0.01,
    }
}

fn build_target_with_provider(
    provider_resource_id: &str,
    provider_id: &str,
    region: &str,
    latency: f32,
    cost: f32,
    health_state: HealthState,
) -> ProviderTargetRuntime {
    let mut target = build_target(provider_resource_id, region, latency, cost, health_state);
    target.resource.provider_id = provider_id.to_string();
    target.resource.supported_protocol_families = match provider_id {
        "anthropic" => vec!["anthropic_messages".to_string()],
        "bedrock" => vec!["openai_chat".to_string()],
        "gateway" => vec![
            "openai_chat".to_string(),
            "openai_responses".to_string(),
            "anthropic_messages".to_string(),
            "gemini_generate_content".to_string(),
        ],
        "gemini" => vec!["gemini_generate_content".to_string()],
        _ => vec!["openai_chat".to_string(), "openai_responses".to_string()],
    };
    target.target_kind = super::provider_target_kind(provider_id);
    target.transit_metadata =
        super::transit_metadata_for_target(&target.resource, &default_route_policy());
    target
}

fn build_gateway_target(provider_resource_id: &str) -> ProviderTargetRuntime {
    let mut target = build_target_with_provider(
        provider_resource_id,
        "gateway",
        "us-east-1",
        0.96,
        0.92,
        HealthState::Healthy,
    );
    target.resource.endpoint_base_url = "https://gateway.example.com/v1".to_string();
    target.resource.provenance_class = ProvenanceClass::OfficialGateway;
    target
}

struct MockGatewayTransport {
    requests: Arc<Mutex<Vec<GatewayHttpRequest>>>,
    response: Mutex<Result<GatewayHttpResponse, provider_gateway::HttpTransportError>>,
}

#[async_trait::async_trait]
impl GatewayHttpTransport for MockGatewayTransport {
    async fn post_json(
        &self,
        request: GatewayHttpRequest,
    ) -> Result<GatewayHttpResponse, provider_gateway::HttpTransportError> {
        self.requests.lock().await.push(request);
        self.response.lock().await.clone()
    }
}

#[derive(Debug)]
struct StaticApiKeyScopeStore {
    scope: GatewayApiKeyScope,
}

impl StaticApiKeyScopeStore {
    fn matching_config() -> Self {
        Self {
            scope: GatewayApiKeyScope {
                credential_id: "cred_gateway_test".to_string(),
                grant_id: None,
                tenant_id: "tenant_acme".to_string(),
                project_id: Some("proj_core".to_string()),
                status: "active".to_string(),
                config_snapshot_id: None,
                route_policy_id: None,
                scopes: Vec::new(),
            },
        }
    }
}

#[async_trait::async_trait]
impl ApiKeyScopeStore for StaticApiKeyScopeStore {
    async fn resolve(&self, _api_key: &str) -> Result<GatewayApiKeyScope, String> {
        Ok(self.scope.clone())
    }
}

#[derive(Default)]
struct RecordingRuntimeEventSink {
    route_receipt_publish_error: Option<String>,
    usage_event_publish_error: Option<String>,
    published_audit_events: Arc<Mutex<Vec<serde_json::Value>>>,
    published_route_receipts: Arc<Mutex<Vec<RouteReceiptRecorded>>>,
    published_usage_events: Arc<Mutex<Vec<UsageEvent>>>,
}

#[async_trait::async_trait]
impl RuntimeEventSink for RecordingRuntimeEventSink {
    async fn publish_route_receipt(
        &self,
        _route_receipt: &RouteReceipt,
        payload: &RouteReceiptRecorded,
        _context: &RequestContext,
    ) -> Result<(), String> {
        if let Some(error) = &self.route_receipt_publish_error {
            return Err(error.clone());
        }

        self.published_route_receipts
            .lock()
            .await
            .push(payload.clone());
        Ok(())
    }

    async fn publish(
        &self,
        _route_receipt: &RouteReceipt,
        usage_event: &UsageEvent,
        _context: &RequestContext,
    ) -> Result<(), String> {
        if let Some(error) = &self.usage_event_publish_error {
            return Err(error.clone());
        }

        self.published_usage_events
            .lock()
            .await
            .push(usage_event.clone());
        Ok(())
    }

    async fn publish_audit(
        &self,
        action: &str,
        outcome: &str,
        context: &RequestContext,
        details: BTreeMap<String, String>,
    ) -> Result<(), String> {
        self.published_audit_events
            .lock()
            .await
            .push(serde_json::json!({
                "action": action,
                "outcome": outcome,
                "request_id": context.request_id,
                "trace_id": context.trace_id,
                "details": details,
            }));
        Ok(())
    }
}

fn test_state(
    adapter: Arc<dyn ProviderAdapter>,
    targets: Vec<ProviderTargetRuntime>,
) -> GatewayState {
    test_state_with_debug(adapter, targets, false)
}

fn test_state_with_debug(
    adapter: Arc<dyn ProviderAdapter>,
    targets: Vec<ProviderTargetRuntime>,
    debug_headers_enabled: bool,
) -> GatewayState {
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();

    Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(targets))),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        route_token_store: Arc::new(super::InMemoryRouteTokenStore::default()),
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    })
}

fn test_state_with_route_token_store(
    adapter: Arc<dyn ProviderAdapter>,
    targets: Vec<ProviderTargetRuntime>,
    route_token_store: Arc<dyn super::RouteTokenStore>,
) -> GatewayState {
    let mut registry = ProviderAdapterRegistry::new();
    registry.register(adapter).unwrap();

    Arc::new(AppState {
        config_store: Arc::new(StaticConfigStore::new(build_config(targets))),
        auth_store: Arc::new(StaticApiKeyScopeStore::matching_config()),
        route_token_store,
        budget_store: Arc::new(StaticBudgetProjectionStore {
            response: ok_budget_projection(),
        }),
        adapter_registry: registry,
        debug_headers_enabled: false,
        event_sink: Arc::new(RecordingRuntimeEventSink::default()),
    })
}

struct MockAdapter {
    outcomes: BTreeMap<String, Result<ProviderResponse, ProviderError>>,
}

struct ProtocolMockAdapter {
    provider_kind: &'static str,
    protocol_family: &'static str,
    outcomes: BTreeMap<String, Result<ProviderResponse, ProviderError>>,
}

struct ImageMockAdapter {
    outcomes: BTreeMap<String, Result<ProviderImageResponse, ProviderError>>,
    requests: Arc<Mutex<Vec<ProviderImageRequest>>>,
}

#[derive(Clone)]
struct ControlPlaneFixture {
    active_config: InternalGatewayConfigResponse,
}

#[derive(Clone)]
struct FixtureState {
    fail: bool,
    fixture: ControlPlaneFixture,
    request_count: Arc<AtomicUsize>,
}

async fn control_plane_api_key_resolution(
    State(state): State<FixtureState>,
    ExtractJson(request): ExtractJson<GatewayApiKeyResolveRequest>,
) -> Result<Json<GatewayApiKeyResolveResponse>, StatusCode> {
    state.request_count.fetch_add(1, AtomicOrdering::Relaxed);
    if state.fail || request.api_key != "test" {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(GatewayApiKeyResolveResponse {
        credential_id: "cred_gateway_test".to_string(),
        grant_id: None,
        tenant_id: "tenant_acme".to_string(),
        project_id: Some("proj_core".to_string()),
        status: "active".to_string(),
        config_snapshot_id: None,
        route_policy_id: None,
        scopes: Vec::new(),
    }))
}

#[async_trait::async_trait]
impl ProviderAdapter for MockAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
            adapter_id: "mock-openai",
            provider_kind: "openai",
            display_name: "Mock OpenAI",
            protocol_family: "openai_chat",
            supported_protocol_families: &["openai_chat", "openai_responses"],
            lifecycle_family: AdapterLifecycleFamily::Inference,
            stability: AdapterStability::Stable,
            streaming_support: StreamingSupport::Unsupported,
            configuration_schema_ref: Some("test:mock-openai"),
        }
    }

    async fn execute_chat(
        &self,
        _request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        self.outcomes
            .get(&context.endpoint.provider_resource_id)
            .cloned()
            .expect("test outcome should exist")
    }
}

#[async_trait::async_trait]
impl ProviderAdapter for ProtocolMockAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
            adapter_id: "mock-protocol",
            provider_kind: self.provider_kind,
            display_name: "Mock Protocol Adapter",
            protocol_family: self.protocol_family,
            supported_protocol_families: &[
                "openai_chat",
                "anthropic_messages",
                "gemini_generate_content",
            ],
            lifecycle_family: AdapterLifecycleFamily::Inference,
            stability: AdapterStability::Stable,
            streaming_support: StreamingSupport::Unsupported,
            configuration_schema_ref: Some("test:mock-protocol"),
        }
    }

    async fn execute_chat(
        &self,
        _request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        self.outcomes
            .get(&context.endpoint.provider_resource_id)
            .cloned()
            .expect("test outcome should exist")
    }
}

#[async_trait::async_trait]
impl ProviderAdapter for ImageMockAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
            adapter_id: "mock-openai-images",
            provider_kind: "openai",
            display_name: "Mock OpenAI Images",
            protocol_family: "openai_images",
            supported_protocol_families: &["openai_images"],
            lifecycle_family: AdapterLifecycleFamily::Inference,
            stability: AdapterStability::Stable,
            streaming_support: StreamingSupport::Unsupported,
            configuration_schema_ref: Some("test:mock-openai-images"),
        }
    }

    async fn execute_chat(
        &self,
        _request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::new(
            ProviderErrorKind::InvalidRequest,
            "chat is not supported by image mock",
            false,
        )
        .with_detail(
            "provider_resource_id",
            &context.endpoint.provider_resource_id,
        ))
    }

    async fn execute_image_generation(
        &self,
        request: &ProviderImageRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderImageResponse, ProviderError> {
        self.requests.lock().await.push(request.clone());
        self.outcomes
            .get(&context.endpoint.provider_resource_id)
            .cloned()
            .expect("test image outcome should exist")
    }
}

fn control_plane_fixture() -> ControlPlaneFixture {
    let config = build_config(vec![
        build_target(
            "prvrsrc_openai_primary",
            "us-east-1",
            0.9,
            0.6,
            HealthState::Healthy,
        ),
        build_target(
            "prvrsrc_openai_backup",
            "us-west-2",
            0.85,
            0.7,
            HealthState::Healthy,
        ),
    ]);

    ControlPlaneFixture {
        active_config: InternalGatewayConfigResponse {
            config_snapshot: config.config_snapshot,
            provider_resources: config
                .provider_targets
                .iter()
                .map(|target| target.resource.clone())
                .collect(),
            route_policy: config.route_policy,
        },
    }
}

async fn control_plane_active_config(
    State(state): State<FixtureState>,
) -> Result<Json<InternalGatewayConfigResponse>, StatusCode> {
    state.request_count.fetch_add(1, AtomicOrdering::Relaxed);
    if state.fail {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(Json(state.fixture.active_config))
}

async fn spawn_control_plane_server(
    fail: bool,
) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let request_count = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route(
            "/internal/gateway/config/current",
            get(control_plane_active_config),
        )
        .route(
            "/internal/gateway/api-keys/resolve",
            post(control_plane_api_key_resolution),
        )
        .with_state(FixtureState {
            fail,
            fixture: control_plane_fixture(),
            request_count: request_count.clone(),
        });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{address}"), request_count, handle)
}

mod control_plane;
mod events_protocol;
mod http_basic;
mod routing;
