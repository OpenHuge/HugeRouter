#![allow(clippy::needless_for_each)]

use anyhow::Context;
use core_domain::{
    AdmissionResult, ConfigSnapshot, ConfigSnapshotId, ErrorEnvelope, LedgerEntry, MonetaryAmount,
    Project, ProjectId, ProviderResource, ProviderResourceId, RoutePolicy, RouteReceipt,
    RouteReceiptId, ServiceName, Tenant, TenantId, UsageEvent, UsagePhase,
};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use utoipa::{OpenApi, ToSchema};

const CONTRACT_VERSION: &str = "v1";
const CONTRACT_GENERATION_COMMAND: &str = "pnpm generate";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub enum ProtocolFamily {
    #[serde(rename = "openai_chat")]
    #[schema(rename = "openai_chat")]
    OpenAiChat,
    #[serde(rename = "openai_responses")]
    #[schema(rename = "openai_responses")]
    OpenAiResponses,
    #[serde(rename = "mcp_streamable_http")]
    #[schema(rename = "mcp_streamable_http")]
    McpStreamableHttp,
    #[serde(rename = "realtime_webrtc")]
    #[schema(rename = "realtime_webrtc")]
    RealtimeWebRtc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub enum MessageType {
    #[serde(rename = "usage_event.recorded")]
    #[schema(rename = "usage_event.recorded")]
    UsageEventRecorded,
    #[serde(rename = "ledger_entry.created")]
    #[schema(rename = "ledger_entry.created")]
    LedgerEntryCreated,
    #[serde(rename = "budget_threshold.exceeded")]
    #[schema(rename = "budget_threshold.exceeded")]
    BudgetThresholdExceeded,
    #[serde(rename = "provider_resource.quarantined")]
    #[schema(rename = "provider_resource.quarantined")]
    ProviderResourceQuarantined,
    #[serde(rename = "provider_resource.degraded")]
    #[schema(rename = "provider_resource.degraded")]
    ProviderResourceDegraded,
    #[serde(rename = "audit_event.created")]
    #[schema(rename = "audit_event.created")]
    AuditEventCreated,
    #[serde(rename = "config_snapshot.activated")]
    #[schema(rename = "config_snapshot.activated")]
    ConfigSnapshotActivated,
    #[serde(rename = "replay_capsule.build_requested")]
    #[schema(rename = "replay_capsule.build_requested")]
    ReplayCapsuleBuildRequested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RequestEnvelope {
    pub protocol_family: ProtocolFamily,
    pub source_service: ServiceName,
    pub request_id: String,
    pub trace_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
}

impl RequestEnvelope {
    #[must_use]
    pub fn new(
        protocol_family: ProtocolFamily,
        source_service: ServiceName,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        tenant_id: TenantId,
        project_id: ProjectId,
    ) -> Self {
        Self {
            protocol_family,
            source_service,
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            tenant_id,
            project_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub message_id: String,
    pub message_type: MessageType,
    pub schema_version: u16,
    pub occurred_at: String,
    pub producer: ServiceName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub idempotency_key: String,
    pub payload: T,
}

impl<T> MessageEnvelope<T> {
    #[must_use]
    pub fn new(
        message_id: impl Into<String>,
        message_type: MessageType,
        occurred_at: impl Into<String>,
        producer: ServiceName,
        idempotency_key: impl Into<String>,
        payload: T,
    ) -> Self {
        Self {
            message_id: message_id.into(),
            message_type,
            schema_version: 1,
            occurred_at: occurred_at.into(),
            producer,
            trace_id: None,
            request_id: None,
            idempotency_key: idempotency_key.into(),
            payload,
        }
    }

    #[must_use]
    pub fn with_request_context(
        mut self,
        trace_id: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        self.trace_id = Some(trace_id.into());
        self.request_id = Some(request_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageEventRecorded {
    pub usage_event: UsageEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LedgerEntryCreated {
    pub ledger_entry: LedgerEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ConfigSnapshotActivated {
    pub config_snapshot: ConfigSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageEventRecordedMessage {
    pub message_id: String,
    pub message_type: MessageType,
    pub schema_version: u16,
    pub occurred_at: String,
    pub producer: ServiceName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub idempotency_key: String,
    pub payload: UsageEventRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ConfigSnapshotActivatedMessage {
    pub message_id: String,
    pub message_type: MessageType,
    pub schema_version: u16,
    pub occurred_at: String,
    pub producer: ServiceName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub idempotency_key: String,
    pub payload: ConfigSnapshotActivated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChatMessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ChatMessage {
    pub role: ChatMessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ChatRequest {
    pub model_alias: String,
    pub messages: Vec<ChatMessage>,
    pub required_capabilities: Vec<String>,
    pub expected_prompt_tokens: u32,
    pub max_output_tokens: u32,
    pub temperature_milli: u16,
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

impl ChatRequest {
    /// # Errors
    ///
    /// Returns an error when the chat request does not describe a routable conversation.
    pub fn validate(&self) -> Result<(), core_domain::DomainError> {
        if self.model_alias.trim().is_empty() {
            return Err(core_domain::DomainError::EmptyField {
                field: "model_alias",
            });
        }

        if self.messages.is_empty() {
            return Err(core_domain::DomainError::EmptyCollection { field: "messages" });
        }

        if self.max_output_tokens == 0 {
            return Err(core_domain::DomainError::EmptyField {
                field: "max_output_tokens",
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayChatRequest {
    pub request: RequestEnvelope,
    pub chat: ChatRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayChatResponse {
    pub route_receipt: RouteReceipt,
    pub usage_event: UsageEvent,
    pub provider_response_id: String,
    pub output_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EligibleCandidate {
    pub provider_resource_id: ProviderResourceId,
    pub score_breakdown: core_domain::ScoreBreakdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteSimulationRequest {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub credential_scope: core_domain::CredentialId,
    pub protocol_family: ProtocolFamily,
    pub model_alias: String,
    pub required_capabilities: Vec<String>,
    pub region: String,
    pub expected_prompt_tokens: u32,
    pub expected_max_output_tokens: u32,
    pub traffic_class: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteSimulationResponse {
    pub simulation_id: String,
    pub config_snapshot_id: ConfigSnapshotId,
    pub admission_result: AdmissionResult,
    pub eligible_candidates: Vec<EligibleCandidate>,
    pub excluded_candidates: Vec<core_domain::ExcludedTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<ProviderResourceId>,
    pub estimated_cost: MonetaryAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct TenantsResponse {
    pub data: Vec<Tenant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ProjectsResponse {
    pub data: Vec<Project>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ProviderResourcesResponse {
    pub data: Vec<ProviderResource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RoutePoliciesResponse {
    pub data: Vec<RoutePolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ConfigSnapshotResponse {
    pub config_snapshot: ConfigSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptResponse {
    pub route_receipt: RouteReceipt,
}

#[derive(Debug, Clone)]
pub struct ArtifactFile {
    pub relative_path: &'static str,
    pub contents: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ContractManifest {
    pub contract_version: String,
    pub generation_command: String,
    pub source_of_truth: String,
    pub contract_digest: String,
    pub stable_contracts: Vec<String>,
    pub compatibility_rules: Vec<String>,
    pub generated_packages: Vec<String>,
    pub openapi_documents: Vec<String>,
    pub json_schema_documents: Vec<String>,
    pub example_documents: Vec<String>,
}

#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "HugeRouter Control Plane API",
        version = "v1",
        description = "Stable control-plane contracts for tenants, projects, routing, and snapshots."
    ),
    paths(
        list_tenants,
        list_projects,
        list_provider_resources,
        get_provider_resource,
        list_route_policies,
        get_config_snapshot,
        activate_config_snapshot,
        create_route_simulation,
        get_route_receipt,
    ),
    components(
        schemas(
            AdmissionResult,
            ConfigSnapshot,
            ConfigSnapshotId,
            ConfigSnapshotResponse,
            EligibleCandidate,
            ErrorEnvelope,
            MonetaryAmount,
            Project,
            ProjectId,
            ProjectsResponse,
            ProtocolFamily,
            ProviderResource,
            ProviderResourceId,
            ProviderResourcesResponse,
            RequestEnvelope,
            RoutePolicy,
            RoutePoliciesResponse,
            RouteReceipt,
            RouteReceiptId,
            RouteReceiptResponse,
            RouteSimulationRequest,
            RouteSimulationResponse,
            Tenant,
            TenantId,
            TenantsResponse,
        )
    ),
    tags(
        (name = "tenants", description = "Tenant and project resources"),
        (name = "routing", description = "Provider resources, route policies, and simulations"),
        (name = "snapshots", description = "Config snapshot read and activation endpoints"),
    )
)]
pub struct ControlPlaneApiDoc;

#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "HugeRouter Gateway API",
        version = "v1",
        description = "Stable gateway contracts for chat routing and normalized failures."
    ),
    paths(gateway_chat_completion),
    components(
        schemas(
            ErrorEnvelope,
            GatewayChatRequest,
            GatewayChatResponse,
            ProtocolFamily,
            RequestEnvelope,
            RouteReceipt,
            UsageEvent,
            ChatRequest,
            ChatMessage,
            ChatMessageRole,
            ToolDefinition,
        )
    ),
    tags((name = "gateway", description = "Protocol ingress contracts")))
]
pub struct GatewayApiDoc;

#[utoipa::path(
    get,
    path = "/v1/tenants",
    tag = "tenants",
    responses(
        (status = 200, description = "List tenants", body = TenantsResponse),
        (status = 500, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn list_tenants() {}

#[utoipa::path(
    get,
    path = "/v1/projects",
    tag = "tenants",
    responses(
        (status = 200, description = "List projects", body = ProjectsResponse),
        (status = 500, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn list_projects() {}

#[utoipa::path(
    get,
    path = "/v1/provider-resources",
    tag = "routing",
    responses(
        (status = 200, description = "List provider resources", body = ProviderResourcesResponse),
        (status = 500, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn list_provider_resources() {}

#[utoipa::path(
    get,
    path = "/v1/provider-resources/{provider_resource_id}",
    tag = "routing",
    params(
        ("provider_resource_id" = String, Path, description = "HugeRouter provider resource id")
    ),
    responses(
        (status = 200, description = "Get one provider resource", body = ProviderResource),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_provider_resource() {}

#[utoipa::path(
    get,
    path = "/v1/route-policies",
    tag = "routing",
    responses(
        (status = 200, description = "List route policies", body = RoutePoliciesResponse),
        (status = 500, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn list_route_policies() {}

#[utoipa::path(
    get,
    path = "/v1/config-snapshots/{config_snapshot_id}",
    tag = "snapshots",
    params(
        ("config_snapshot_id" = String, Path, description = "HugeRouter config snapshot id")
    ),
    responses(
        (status = 200, description = "Get config snapshot", body = ConfigSnapshotResponse),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_config_snapshot() {}

#[utoipa::path(
    post,
    path = "/v1/config-snapshots/{config_snapshot_id}/activate",
    tag = "snapshots",
    params(
        ("config_snapshot_id" = String, Path, description = "HugeRouter config snapshot id")
    ),
    responses(
        (status = 200, description = "Activate config snapshot", body = ConfigSnapshotResponse),
        (status = 409, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn activate_config_snapshot() {}

#[utoipa::path(
    post,
    path = "/v1/route-simulations",
    tag = "routing",
    request_body = RouteSimulationRequest,
    responses(
        (status = 200, description = "Route simulation result", body = RouteSimulationResponse),
        (status = 422, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn create_route_simulation() {}

#[utoipa::path(
    get,
    path = "/v1/route-receipts/{route_receipt_id}",
    tag = "routing",
    params(
        ("route_receipt_id" = String, Path, description = "HugeRouter route receipt id")
    ),
    responses(
        (status = 200, description = "Get route receipt", body = RouteReceiptResponse),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_route_receipt() {}

#[utoipa::path(
    post,
    path = "/v1/gateway/chat/completions",
    tag = "gateway",
    request_body = GatewayChatRequest,
    responses(
        (status = 200, description = "Chat routed successfully", body = GatewayChatResponse),
        (status = 422, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn gateway_chat_completion() {}

fn json_schema_artifacts() -> anyhow::Result<Vec<ArtifactFile>> {
    let schemas = vec![
        schema_artifact::<Tenant>("schemas/jsonschema/tenant.v1.schema.json")?,
        schema_artifact::<Project>("schemas/jsonschema/project.v1.schema.json")?,
        schema_artifact::<ProviderResource>("schemas/jsonschema/provider-resource.v1.schema.json")?,
        schema_artifact::<RoutePolicy>("schemas/jsonschema/route-policy.v1.schema.json")?,
        schema_artifact::<ConfigSnapshot>("schemas/jsonschema/config-snapshot.v1.schema.json")?,
        schema_artifact::<RouteReceipt>("schemas/jsonschema/route-receipt.v1.schema.json")?,
        schema_artifact::<UsageEvent>("schemas/jsonschema/usage-event.v1.schema.json")?,
        schema_artifact::<core_domain::NormalizedError>(
            "schemas/jsonschema/normalized-error.v1.schema.json",
        )?,
        schema_artifact::<ErrorEnvelope>("schemas/jsonschema/error-envelope.v1.schema.json")?,
        schema_artifact::<ChatRequest>("schemas/jsonschema/chat-request.v1.schema.json")?,
        schema_artifact::<GatewayChatRequest>(
            "schemas/jsonschema/gateway-chat-request.v1.schema.json",
        )?,
        schema_artifact::<GatewayChatResponse>(
            "schemas/jsonschema/gateway-chat-response.v1.schema.json",
        )?,
        schema_artifact::<RouteSimulationRequest>(
            "schemas/jsonschema/route-simulation-request.v1.schema.json",
        )?,
        schema_artifact::<RouteSimulationResponse>(
            "schemas/jsonschema/route-simulation-response.v1.schema.json",
        )?,
        schema_artifact::<UsageEventRecordedMessage>(
            "schemas/jsonschema/usage-event-recorded-message.v1.schema.json",
        )?,
        schema_artifact::<ConfigSnapshotActivatedMessage>(
            "schemas/jsonschema/config-snapshot-activated-message.v1.schema.json",
        )?,
    ];

    Ok(schemas)
}

fn openapi_artifacts() -> anyhow::Result<Vec<ArtifactFile>> {
    let control_plane = serde_json::to_string_pretty(&ControlPlaneApiDoc::openapi())
        .context("serialize control-plane openapi")?;
    let gateway = serde_json::to_string_pretty(&GatewayApiDoc::openapi())
        .context("serialize gateway openapi")?;

    Ok(vec![
        ArtifactFile {
            relative_path: "schemas/openapi/control-plane-v1.openapi.json",
            contents: control_plane,
        },
        ArtifactFile {
            relative_path: "schemas/openapi/gateway-v1.openapi.json",
            contents: gateway,
        },
    ])
}

fn example_artifacts() -> anyhow::Result<Vec<ArtifactFile>> {
    let tenant = sample_tenant();
    let project = sample_project();
    let provider_resource = sample_provider_resource();
    let route_policy = sample_route_policy();
    let config_snapshot = sample_config_snapshot();
    let route_receipt = sample_route_receipt();
    let usage_event = sample_usage_event();
    let simulation_request = sample_route_simulation_request();
    let simulation_response = sample_route_simulation_response();
    let gateway_request = sample_gateway_chat_request();
    let gateway_response = sample_gateway_chat_response();
    let usage_message = sample_usage_event_recorded_message();
    let snapshot_message = sample_config_snapshot_activated_message();
    let error_envelope = sample_error_envelope();

    let examples = vec![
        example_artifact(
            "schemas/examples/control-plane/tenants.response.json",
            &TenantsResponse { data: vec![tenant] },
        )?,
        example_artifact(
            "schemas/examples/control-plane/projects.response.json",
            &ProjectsResponse {
                data: vec![project],
            },
        )?,
        example_artifact(
            "schemas/examples/control-plane/provider-resources.response.json",
            &ProviderResourcesResponse {
                data: vec![provider_resource],
            },
        )?,
        example_artifact(
            "schemas/examples/control-plane/route-policies.response.json",
            &RoutePoliciesResponse {
                data: vec![route_policy],
            },
        )?,
        example_artifact(
            "schemas/examples/control-plane/config-snapshot.response.json",
            &ConfigSnapshotResponse { config_snapshot },
        )?,
        example_artifact(
            "schemas/examples/control-plane/route-simulation.request.json",
            &simulation_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/route-simulation.response.json",
            &simulation_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/route-receipt.response.json",
            &RouteReceiptResponse { route_receipt },
        )?,
        example_artifact(
            "schemas/examples/gateway/chat.request.json",
            &gateway_request,
        )?,
        example_artifact(
            "schemas/examples/gateway/chat.response.json",
            &gateway_response,
        )?,
        example_artifact(
            "schemas/examples/gateway/error.response.json",
            &error_envelope,
        )?,
        example_artifact(
            "schemas/examples/events/usage-event-recorded.message.json",
            &usage_message,
        )?,
        example_artifact(
            "schemas/examples/events/config-snapshot-activated.message.json",
            &snapshot_message,
        )?,
        example_artifact("schemas/examples/domain/usage-event.json", &usage_event)?,
    ];

    Ok(examples)
}

fn generated_package_artifacts(contract_digest: &str) -> Vec<ArtifactFile> {
    let shared_schema_meta = format!(
        "export const CONTRACT_VERSION = {CONTRACT_VERSION:?} as const;\n\
export const CONTRACT_DIGEST = {contract_digest:?} as const;\n\
export const COMPATIBILITY_RULES = [\n\
  'Breaking changes require a new explicit contract version or a new endpoint family.',\n\
  'Additive fields must remain optional until all first-party consumers can tolerate them.',\n\
  'Enum expansions are additive only when consumers treat unknown values defensively.',\n\
  'Serialization key changes are always breaking for v1 contracts.',\n\
  'Checked-in schemas, examples, and generated package metadata must be regenerated together.',\n\
] as const;\n\
export const PROTOCOL_FAMILIES = ['openai_chat', 'openai_responses', 'mcp_streamable_http', 'realtime_webrtc'] as const;\n\
export const ADMISSION_RESULTS = ['admitted', 'rejected_budget', 'rejected_rate_limit', 'rejected_concurrency', 'rejected_policy', 'rejected_no_candidate'] as const;\n\
export const PROVIDER_RESOURCE_STATUSES = ['active', 'disabled', 'draining', 'quarantined', 'deleted'] as const;\n\
export const USAGE_PHASES = ['reserve', 'partial', 'final', 'release'] as const;\n",
    );

    let api_client_meta = format!(
        "export const CONTRACT_VERSION = {CONTRACT_VERSION:?} as const;\n\
export const CONTRACT_DIGEST = {contract_digest:?} as const;\n\
export const CONTROL_PLANE_OPERATIONS = [\n\
  {{ id: 'listTenants', method: 'GET', path: '/v1/tenants' }},\n\
  {{ id: 'listProjects', method: 'GET', path: '/v1/projects' }},\n\
  {{ id: 'listProviderResources', method: 'GET', path: '/v1/provider-resources' }},\n\
  {{ id: 'getProviderResource', method: 'GET', path: '/v1/provider-resources/{{provider_resource_id}}' }},\n\
  {{ id: 'listRoutePolicies', method: 'GET', path: '/v1/route-policies' }},\n\
  {{ id: 'getConfigSnapshot', method: 'GET', path: '/v1/config-snapshots/{{config_snapshot_id}}' }},\n\
  {{ id: 'activateConfigSnapshot', method: 'POST', path: '/v1/config-snapshots/{{config_snapshot_id}}/activate' }},\n\
  {{ id: 'simulateRoute', method: 'POST', path: '/v1/route-simulations' }},\n\
  {{ id: 'getRouteReceipt', method: 'GET', path: '/v1/route-receipts/{{route_receipt_id}}' }},\n\
] as const;\n\
export const GATEWAY_OPERATIONS = [\n\
  {{ id: 'createChatCompletion', method: 'POST', path: '/v1/gateway/chat/completions' }},\n\
] as const;\n",
    );

    vec![
        ArtifactFile {
            relative_path: "packages/ts-shared-schema/src/generated/contract-meta.ts",
            contents: shared_schema_meta,
        },
        ArtifactFile {
            relative_path: "packages/ts-api-client/src/generated/operation-meta.ts",
            contents: api_client_meta,
        },
    ]
}

fn compatibility_rules() -> Vec<String> {
    vec![
        "Breaking changes require a new explicit contract version or a new endpoint family.".to_string(),
        "Additive fields must remain optional until all first-party consumers can tolerate them.".to_string(),
        "Enum expansions are additive only when consumers treat unknown values defensively.".to_string(),
        "Serialization key changes are always breaking for v1 contracts.".to_string(),
        "Checked-in schemas, examples, and generated package metadata must be regenerated together.".to_string(),
    ]
}

fn stable_contracts() -> Vec<String> {
    vec![
        "tenant".to_string(),
        "project".to_string(),
        "provider_resource".to_string(),
        "route_policy".to_string(),
        "config_snapshot".to_string(),
        "chat_request".to_string(),
        "route_receipt".to_string(),
        "usage_event".to_string(),
        "normalized_error".to_string(),
        "usage_event.recorded".to_string(),
        "config_snapshot.activated".to_string(),
    ]
}

/// # Errors
///
/// Returns an error when any `OpenAPI`, `JSON Schema`, example, or generated package
/// artifact cannot be rendered.
pub fn collect_contract_artifacts() -> anyhow::Result<Vec<ArtifactFile>> {
    let mut base_artifacts = Vec::new();
    base_artifacts.extend(openapi_artifacts()?);
    base_artifacts.extend(json_schema_artifacts()?);
    base_artifacts.extend(example_artifacts()?);

    let contract_digest = compute_digest(&base_artifacts);

    let openapi_documents = base_artifacts
        .iter()
        .filter(|artifact| artifact.relative_path.starts_with("schemas/openapi/"))
        .map(|artifact| artifact.relative_path.to_string())
        .collect::<Vec<_>>();
    let json_schema_documents = base_artifacts
        .iter()
        .filter(|artifact| artifact.relative_path.starts_with("schemas/jsonschema/"))
        .map(|artifact| artifact.relative_path.to_string())
        .collect::<Vec<_>>();
    let example_documents = base_artifacts
        .iter()
        .filter(|artifact| artifact.relative_path.starts_with("schemas/examples/"))
        .map(|artifact| artifact.relative_path.to_string())
        .collect::<Vec<_>>();

    let generated_package_artifacts = generated_package_artifacts(&contract_digest);
    let generated_packages = generated_package_artifacts
        .iter()
        .map(|artifact| artifact.relative_path.to_string())
        .collect::<Vec<_>>();

    let manifest = ContractManifest {
        contract_version: CONTRACT_VERSION.to_string(),
        generation_command: CONTRACT_GENERATION_COMMAND.to_string(),
        source_of_truth: "Rust crates `core-domain` and `protocol-ir`".to_string(),
        contract_digest,
        stable_contracts: stable_contracts(),
        compatibility_rules: compatibility_rules(),
        generated_packages,
        openapi_documents,
        json_schema_documents,
        example_documents,
    };

    let mut artifacts = base_artifacts;
    artifacts.extend(generated_package_artifacts);
    artifacts.push(ArtifactFile {
        relative_path: "schemas/jsonschema/contracts.manifest.json",
        contents: serde_json::to_string_pretty(&manifest).context("serialize contract manifest")?,
    });
    Ok(artifacts)
}

/// # Errors
///
/// Returns an error when the contract artifacts cannot be rendered or written to disk.
pub fn write_contract_artifacts(workspace_root: impl AsRef<Path>) -> anyhow::Result<()> {
    for artifact in collect_contract_artifacts()? {
        let path = workspace_root.as_ref().join(artifact.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create directory {}", parent.display()))?;
        }

        fs::write(&path, artifact.contents)
            .with_context(|| format!("write generated artifact {}", path.display()))?;
    }

    Ok(())
}

fn schema_artifact<T: JsonSchema>(relative_path: &'static str) -> anyhow::Result<ArtifactFile> {
    let schema = schema_for!(T);
    Ok(ArtifactFile {
        relative_path,
        contents: serde_json::to_string_pretty(&schema).context("serialize json schema")?,
    })
}

fn example_artifact<T: Serialize>(
    relative_path: &'static str,
    value: &T,
) -> anyhow::Result<ArtifactFile> {
    Ok(ArtifactFile {
        relative_path,
        contents: serde_json::to_string_pretty(value).context("serialize example payload")?,
    })
}

fn compute_digest(artifacts: &[ArtifactFile]) -> String {
    let mut paths = artifacts
        .iter()
        .map(|artifact| (artifact.relative_path, artifact.contents.as_str()))
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| left.0.cmp(right.0));

    let mut digest = Sha256::new();
    for (path, contents) in paths {
        digest.update(path.as_bytes());
        digest.update(contents.as_bytes());
    }

    let mut hex = String::new();
    for byte in digest.finalize() {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn sample_tenant() -> Tenant {
    Tenant::new(
        TenantId::parse("tenant_acme").unwrap(),
        "acme-platform",
        "Acme Platform",
        3,
        "2026-04-20T00:00:00Z",
        "2026-04-21T08:30:00Z",
    )
    .unwrap()
}

fn sample_project() -> Project {
    Project::new(
        ProjectId::parse("proj_core").unwrap(),
        TenantId::parse("tenant_acme").unwrap(),
        "core-routing",
        "Core Routing",
        9,
        "2026-04-20T00:00:00Z",
        "2026-04-21T09:45:00Z",
    )
    .unwrap()
}

fn sample_provider_resource() -> ProviderResource {
    ProviderResource {
        provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: Some(ProjectId::parse("proj_core").unwrap()),
        provider_id: "openai".to_string(),
        name: "openai-us-east-primary".to_string(),
        status: core_domain::ProviderResourceStatus::Active,
        provenance_class: core_domain::ProvenanceClass::OfficialApi,
        credential_owner_type: core_domain::CredentialOwnerType::Platform,
        deployment_scope: core_domain::DeploymentScope::Shared,
        region: "us-east-1".to_string(),
        endpoint_base_url: "https://api.openai.com/v1".to_string(),
        auth_kind: core_domain::AuthKind::ApiKey,
        health_state: core_domain::HealthState::Healthy,
        budget_policy_id: Some(core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap()),
        capabilities: core_domain::ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_json_mode: true,
        },
        version: 7,
        created_at: "2026-04-20T00:00:00Z".to_string(),
        updated_at: "2026-04-21T11:15:00Z".to_string(),
    }
}

fn sample_route_policy() -> RoutePolicy {
    RoutePolicy {
        route_policy_id: core_domain::RoutePolicyId::parse("routepol_default").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        display_name: "Default Interactive Route".to_string(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: vec!["tool_calling".to_string(), "json_mode".to_string()],
        preferred_regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
        version: 4,
        created_at: "2026-04-20T00:00:00Z".to_string(),
        updated_at: "2026-04-21T12:00:00Z".to_string(),
    }
}

fn sample_config_snapshot() -> ConfigSnapshot {
    ConfigSnapshot {
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        revision: 12,
        status: core_domain::ConfigSnapshotStatus::Active,
        activated_at: Some("2026-04-21T12:15:00Z".to_string()),
        provider_resource_ids: vec![ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()],
        route_policy_id: core_domain::RoutePolicyId::parse("routepol_default").unwrap(),
        budget_policy_id: core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap(),
    }
}

fn sample_route_receipt() -> RouteReceipt {
    RouteReceipt {
        route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        request_id: "req_123".to_string(),
        trace_id: "trace_123".to_string(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
        admission_result: AdmissionResult::Admitted,
        selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
        excluded_targets: vec![core_domain::ExcludedTarget {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_backup").unwrap(),
            reason: "rejected_provenance_class".to_string(),
        }],
        score_breakdown: core_domain::ScoreBreakdown {
            latency: 0.82,
            cost: 0.66,
            health: 0.97,
            trust: 1.0,
        },
        fallback_transitions: Vec::new(),
        normalized_error: None,
        created_at: "2026-04-21T12:16:00Z".to_string(),
    }
}

fn sample_usage_event() -> UsageEvent {
    UsageEvent {
        usage_event_id: core_domain::UsageEventId::parse("usageevt_123").unwrap(),
        route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
        model_alias: "reasoning-fast".to_string(),
        phase: UsagePhase::Final,
        idempotency_key: "usageevt_123:final".to_string(),
        usage: core_domain::UsageMetrics {
            input_tokens: 1200,
            output_tokens: 320,
            cached_input_tokens: 64,
        },
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.1420".to_string(),
        },
        recorded_at: "2026-04-21T12:16:05Z".to_string(),
    }
}

fn sample_error_envelope() -> ErrorEnvelope {
    ErrorEnvelope {
        error: core_domain::NormalizedError {
            code: "validation_failed".to_string(),
            message: "Route policy requires at least one capability.".to_string(),
            request_id: "req_123".to_string(),
            retryable: false,
            upstream_code: None,
            upstream_status_code: None,
            validation_issues: vec![core_domain::ValidationIssue {
                field: "required_capabilities".to_string(),
                message: "expected at least one value".to_string(),
            }],
            details: std::collections::BTreeMap::from([(
                "resource".to_string(),
                "route_policy".to_string(),
            )]),
        },
    }
}

fn sample_route_simulation_request() -> RouteSimulationRequest {
    RouteSimulationRequest {
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        credential_scope: core_domain::CredentialId::parse("cred_primary").unwrap(),
        protocol_family: ProtocolFamily::OpenAiChat,
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: vec!["tool_calling".to_string(), "json_mode".to_string()],
        region: "us-east-1".to_string(),
        expected_prompt_tokens: 12_000,
        expected_max_output_tokens: 4_000,
        traffic_class: "interactive".to_string(),
    }
}

fn sample_route_simulation_response() -> RouteSimulationResponse {
    RouteSimulationResponse {
        simulation_id: "routesim_123".to_string(),
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
        admission_result: AdmissionResult::Admitted,
        eligible_candidates: vec![EligibleCandidate {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            score_breakdown: core_domain::ScoreBreakdown {
                latency: 0.82,
                cost: 0.66,
                health: 0.97,
                trust: 1.0,
            },
        }],
        excluded_candidates: vec![core_domain::ExcludedTarget {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_backup").unwrap(),
            reason: "rejected_provenance_class".to_string(),
        }],
        selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.1420".to_string(),
        },
    }
}

fn sample_gateway_chat_request() -> GatewayChatRequest {
    GatewayChatRequest {
        request: RequestEnvelope::new(
            ProtocolFamily::OpenAiChat,
            ServiceName::parse("gateway-api").unwrap(),
            "req_123",
            "trace_123",
            TenantId::parse("tenant_acme").unwrap(),
            ProjectId::parse("proj_core").unwrap(),
        ),
        chat: ChatRequest {
            model_alias: "reasoning-fast".to_string(),
            messages: vec![
                ChatMessage {
                    role: ChatMessageRole::System,
                    content: "You are a concise routing assistant.".to_string(),
                },
                ChatMessage {
                    role: ChatMessageRole::User,
                    content: "Summarize the last deployment incident.".to_string(),
                },
            ],
            required_capabilities: vec!["tool_calling".to_string(), "json_mode".to_string()],
            expected_prompt_tokens: 12_000,
            max_output_tokens: 1_024,
            temperature_milli: 200,
            tools: vec![ToolDefinition {
                name: "incident_lookup".to_string(),
                description: "Look up incident summaries from the operations system.".to_string(),
            }],
            conversation_id: Some("conv_ops_42".to_string()),
        },
    }
}

fn sample_gateway_chat_response() -> GatewayChatResponse {
    GatewayChatResponse {
        route_receipt: sample_route_receipt(),
        usage_event: sample_usage_event(),
        provider_response_id: "resp_openai_123".to_string(),
        output_text: "The incident was caused by a stale config snapshot activation.".to_string(),
    }
}

fn sample_usage_event_recorded_message() -> UsageEventRecordedMessage {
    let envelope = MessageEnvelope::new(
        "msg_usage_123",
        MessageType::UsageEventRecorded,
        "2026-04-21T12:16:05Z",
        ServiceName::parse("gateway-api").unwrap(),
        "usageevt_123:final",
        UsageEventRecorded {
            usage_event: sample_usage_event(),
        },
    )
    .with_request_context("trace_123", "req_123");

    UsageEventRecordedMessage {
        message_id: envelope.message_id,
        message_type: envelope.message_type,
        schema_version: envelope.schema_version,
        occurred_at: envelope.occurred_at,
        producer: envelope.producer,
        trace_id: envelope.trace_id,
        request_id: envelope.request_id,
        idempotency_key: envelope.idempotency_key,
        payload: envelope.payload,
    }
}

fn sample_config_snapshot_activated_message() -> ConfigSnapshotActivatedMessage {
    let envelope = MessageEnvelope::new(
        "msg_cfgsnap_123",
        MessageType::ConfigSnapshotActivated,
        "2026-04-21T12:15:00Z",
        ServiceName::parse("control-plane-api").unwrap(),
        "cfgsnap_default:activated",
        ConfigSnapshotActivated {
            config_snapshot: sample_config_snapshot(),
        },
    );

    ConfigSnapshotActivatedMessage {
        message_id: envelope.message_id,
        message_type: envelope.message_type,
        schema_version: envelope.schema_version,
        occurred_at: envelope.occurred_at,
        producer: envelope.producer,
        trace_id: envelope.trace_id,
        request_id: envelope.request_id,
        idempotency_key: envelope.idempotency_key,
        payload: envelope.payload,
    }
}

/// # Panics
///
/// Panics if the crate is no longer nested under the workspace root.
#[must_use]
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::{
        ChatRequest, ConfigSnapshotActivated, MessageEnvelope, MessageType, ProtocolFamily,
        RequestEnvelope, collect_contract_artifacts, sample_gateway_chat_request,
        sample_usage_event_recorded_message, workspace_root,
    };
    use core_domain::{ConfigSnapshot, ProjectId, ServiceName, TenantId};

    #[test]
    fn request_envelope_uses_stable_public_protocol_names() {
        let envelope = RequestEnvelope::new(
            ProtocolFamily::OpenAiChat,
            ServiceName::parse("gateway-api").unwrap(),
            "req_123",
            "trace_123",
            TenantId::parse("tenant_acme").unwrap(),
            ProjectId::parse("proj_core").unwrap(),
        );

        let json = serde_json::to_value(&envelope).unwrap();

        assert_eq!(json["protocol_family"], "openai_chat");
        assert_eq!(json["source_service"], "gateway-api");
        assert_eq!(json["tenant_id"], "tenant_acme");
    }

    #[test]
    fn message_envelope_uses_dotted_event_names() {
        let payload = ConfigSnapshotActivated {
            config_snapshot: ConfigSnapshot {
                config_snapshot_id: core_domain::ConfigSnapshotId::parse("cfgsnap_123").unwrap(),
                tenant_id: TenantId::parse("tenant_acme").unwrap(),
                project_id: ProjectId::parse("proj_core").unwrap(),
                revision: 1,
                status: core_domain::ConfigSnapshotStatus::Active,
                activated_at: Some("2026-04-20T00:00:00Z".to_string()),
                provider_resource_ids: vec![
                    core_domain::ProviderResourceId::parse("prvrsrc_123").unwrap(),
                ],
                route_policy_id: core_domain::RoutePolicyId::parse("routepol_default").unwrap(),
                budget_policy_id: core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap(),
            },
        };

        let envelope = MessageEnvelope::new(
            "msg_123",
            MessageType::ConfigSnapshotActivated,
            "2026-04-20T00:00:00Z",
            ServiceName::parse("control-plane-api").unwrap(),
            "cfgsnap_123:activated",
            payload,
        )
        .with_request_context("trace_123", "req_123");

        let json = serde_json::to_value(&envelope).unwrap();

        assert_eq!(json["message_type"], "config_snapshot.activated");
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["trace_id"], "trace_123");
        assert_eq!(
            json["payload"]["config_snapshot"]["config_snapshot_id"],
            "cfgsnap_123"
        );
    }

    #[test]
    fn chat_request_validation_rejects_empty_messages() {
        let request = ChatRequest {
            model_alias: "reasoning-fast".to_string(),
            messages: Vec::new(),
            required_capabilities: vec!["json_mode".to_string()],
            expected_prompt_tokens: 100,
            max_output_tokens: 32,
            temperature_milli: 100,
            tools: Vec::new(),
            conversation_id: None,
        };

        assert_eq!(
            request.validate().unwrap_err(),
            core_domain::DomainError::EmptyCollection { field: "messages" }
        );
    }

    #[test]
    fn checked_in_artifacts_match_generated_contracts() {
        let root = workspace_root();

        for artifact in collect_contract_artifacts().unwrap() {
            let disk = std::fs::read_to_string(root.join(artifact.relative_path)).unwrap();
            assert_eq!(
                disk, artifact.contents,
                "artifact drift: {}",
                artifact.relative_path
            );
        }
    }

    #[test]
    fn gateway_examples_round_trip_to_json() {
        let request = sample_gateway_chat_request();
        request.chat.validate().unwrap();

        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["request"]["protocol_family"], "openai_chat");
        assert_eq!(value["chat"]["messages"][0]["role"], "system");

        let event = sample_usage_event_recorded_message();
        let event_json = serde_json::to_value(&event).unwrap();
        assert_eq!(event_json["message_type"], "usage_event.recorded");
    }
}
