#![allow(clippy::needless_for_each)]

use anyhow::Context;
use core_domain::{
    AdmissionResult, ConfigSnapshot, ConfigSnapshotId, ErrorEnvelope, LedgerEntry, MonetaryAmount,
    Project, ProjectId, ProviderResource, ProviderResourceId, RoutePolicy, RoutePolicyId,
    RouteReceipt, RouteReceiptId, ServiceName, Tenant, TenantId, UsageEvent, UsageMetrics,
    UsagePhase,
};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
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
    #[serde(rename = "openai_images")]
    #[schema(rename = "openai_images")]
    OpenAiImages,
    #[serde(rename = "mcp_streamable_http")]
    #[schema(rename = "mcp_streamable_http")]
    McpStreamableHttp,
    #[serde(rename = "realtime_webrtc")]
    #[schema(rename = "realtime_webrtc")]
    RealtimeWebRtc,
    #[serde(rename = "anthropic_messages")]
    #[schema(rename = "anthropic_messages")]
    AnthropicMessages,
    #[serde(rename = "gemini_generate_content")]
    #[schema(rename = "gemini_generate_content")]
    GeminiGenerateContent,
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
pub enum RouteReceiptRecordedMessageType {
    #[serde(rename = "route_receipt.recorded")]
    #[schema(rename = "route_receipt.recorded")]
    RouteReceiptRecorded,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptRecorded {
    pub route_receipt: RouteReceipt,
    #[serde(default)]
    pub decision_timeline: Vec<RouteReceiptDecisionTraceStep>,
    #[serde(default)]
    pub policy_checks: Vec<RouteReceiptPolicyCheck>,
    #[serde(default)]
    pub provider_attempts: Vec<RouteReceiptProviderAttempt>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptRecordedMessage {
    pub message_id: String,
    pub message_type: RouteReceiptRecordedMessageType,
    pub schema_version: u16,
    pub occurred_at: String,
    pub producer: ServiceName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub idempotency_key: String,
    pub payload: RouteReceiptRecorded,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptDecisionTraceStep {
    pub stage: String,
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptPolicyCheck {
    pub policy_id: RoutePolicyId,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptProviderAttempt {
    pub provider_resource_id: ProviderResourceId,
    pub attempt: u8,
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason_code: String,
    #[serde(default)]
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_target: Option<ProviderResourceId>,
    pub started_at: String,
    pub finished_at: String,
    pub latency_ms: u32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptDiagnosticsResponse {
    pub route_receipt: RouteReceipt,
    pub decision_timeline: Vec<RouteReceiptDecisionTraceStep>,
    pub policy_checks: Vec<RouteReceiptPolicyCheck>,
    pub provider_attempts: Vec<RouteReceiptProviderAttempt>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageSummary {
    pub tenant_id: TenantId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub window_start: String,
    pub window_end: String,
    pub currency: String,
    pub event_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub provider_cost: MonetaryAmount,
    pub billable_price: MonetaryAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageSummaryResponse {
    pub data: UsageSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageBreakdownRow {
    pub bucket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_alias: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub provider_cost: MonetaryAmount,
    pub billable_price: MonetaryAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageBreakdownResponse {
    pub data: Vec<UsageBreakdownRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BalanceProjection {
    pub tenant_id: TenantId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub currency: String,
    pub provider_cost_total: MonetaryAmount,
    pub billable_total: MonetaryAmount,
    pub configured_budget: MonetaryAmount,
    pub remaining_budget: MonetaryAmount,
    pub threshold_status: String,
    pub last_projected_at: String,
    pub projection_lag_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BalanceProjectionResponse {
    pub data: BalanceProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PricingSimulationRequest {
    pub provider_id: String,
    pub model_alias: String,
    pub usage: UsageMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_generation_units: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_seconds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PricingCatalogEntry {
    pub dimension: String,
    pub provider_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub micros_per_unit: i64,
    pub unit_denominator: u64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PricingCatalogResponse {
    pub catalog_id: String,
    pub catalog_version: u32,
    pub currency: String,
    pub entries: Vec<PricingCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PricingSimulationLineItem {
    pub dimension: String,
    pub units: u64,
    pub provider_cost: MonetaryAmount,
    pub billable_price: MonetaryAmount,
    pub rate_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PricingSimulationResponse {
    pub catalog_id: String,
    pub catalog_version: u32,
    pub currency: String,
    pub provider_cost: MonetaryAmount,
    pub billable_price: MonetaryAmount,
    pub line_items: Vec<PricingSimulationLineItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BillingExportRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<TenantId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub window_start: String,
    pub window_end: String,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BillingExportJob {
    pub export_job_id: String,
    pub status: String,
    pub format: String,
    pub requested_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<TenantId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BillingExportJobResponse {
    pub data: BillingExportJob,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BillingExportJobsResponse {
    pub data: Vec<BillingExportJob>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptsResponse {
    pub data: Vec<RouteReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouteDiagnosticDecision {
    Selected,
    Eligible,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceiptSummary {
    pub route_receipt_id: RouteReceiptId,
    pub admission_result: AdmissionResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<ProviderResourceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteDiagnosticTarget {
    pub provider_resource: ProviderResource,
    pub decision: RouteDiagnosticDecision,
    pub in_active_snapshot: bool,
    pub supports_protocol_family: bool,
    pub capability_gaps: Vec<String>,
    pub reason_code: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_receipt_id: Option<RouteReceiptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_receipt_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteDiagnosticsResponse {
    pub route_policy: RoutePolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_snapshot: Option<ConfigSnapshot>,
    pub active_snapshot_matches_route_policy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_route_receipt: Option<RouteReceiptSummary>,
    pub recent_receipts: Vec<RouteReceiptSummary>,
    pub targets: Vec<RouteDiagnosticTarget>,
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
        get_usage_summary,
        get_usage_breakdown,
        get_balance_projection,
        get_pricing_catalog,
        create_pricing_simulation,
        create_billing_export,
        list_billing_exports,
        get_billing_export,
        create_route_simulation,
        list_route_receipts,
        get_route_receipt,
        get_route_diagnostics,
        get_route_receipt_diagnostics,
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
            PricingCatalogEntry,
            PricingCatalogResponse,
            PricingSimulationLineItem,
            PricingSimulationRequest,
            PricingSimulationResponse,
            RequestEnvelope,
            RoutePolicy,
            RoutePoliciesResponse,
            RouteReceipt,
            RouteReceiptsResponse,
            RouteDiagnosticDecision,
            RouteDiagnosticTarget,
            RouteDiagnosticsResponse,
            RouteReceiptId,
            RouteReceiptResponse,
            RouteReceiptDiagnosticsResponse,
            RouteReceiptSummary,
            RouteSimulationRequest,
            RouteSimulationResponse,
            Tenant,
            TenantId,
            TenantsResponse,
            UsageBreakdownResponse,
            UsageBreakdownRow,
            UsageSummary,
            UsageSummaryResponse,
            BalanceProjection,
            BalanceProjectionResponse,
            BillingExportJob,
            BillingExportJobResponse,
            BillingExportJobsResponse,
            BillingExportRequest,
            UsageMetrics,
        )
    ),
    tags(
        (name = "tenants", description = "Tenant and project resources"),
        (name = "routing", description = "Provider resources, route policies, and simulations"),
        (name = "snapshots", description = "Config snapshot read and activation endpoints"),
    )
)]
pub struct ControlPlaneApiDoc;

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
    get,
    path = "/v1/usage/summary",
    tag = "routing",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant id"),
        ("project_id" = Option<String>, Query, description = "Filter by project id"),
        ("window_start" = Option<String>, Query, description = "Inclusive RFC3339 start timestamp"),
        ("window_end" = Option<String>, Query, description = "Inclusive RFC3339 end timestamp")
    ),
    responses(
        (status = 200, description = "Usage summary for the requested scope", body = UsageSummaryResponse),
        (status = 400, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_usage_summary() {}

#[utoipa::path(
    get,
    path = "/v1/usage/breakdown",
    tag = "routing",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant id"),
        ("project_id" = Option<String>, Query, description = "Filter by project id"),
        ("window_start" = Option<String>, Query, description = "Inclusive RFC3339 start timestamp"),
        ("window_end" = Option<String>, Query, description = "Inclusive RFC3339 end timestamp"),
        ("group_by" = Option<String>, Query, description = "Breakdown dimension: provider, model, or day"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor"),
        ("limit" = Option<u32>, Query, description = "Maximum number of rows to return")
    ),
    responses(
        (status = 200, description = "Usage breakdown rows", body = UsageBreakdownResponse),
        (status = 400, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_usage_breakdown() {}

#[utoipa::path(
    get,
    path = "/v1/billing/projection",
    tag = "routing",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant id"),
        ("project_id" = Option<String>, Query, description = "Filter by project id")
    ),
    responses(
        (status = 200, description = "Billing projection summary", body = BalanceProjectionResponse),
        (status = 400, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_balance_projection() {}

#[utoipa::path(
    get,
    path = "/v1/pricing/catalog",
    tag = "routing",
    responses(
        (status = 200, description = "Pricing catalog entries", body = PricingCatalogResponse),
        (status = 500, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_pricing_catalog() {}

#[utoipa::path(
    post,
    path = "/v1/pricing/simulations",
    tag = "routing",
    request_body = PricingSimulationRequest,
    responses(
        (status = 200, description = "Pricing simulation result", body = PricingSimulationResponse),
        (status = 422, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn create_pricing_simulation() {}

#[utoipa::path(
    post,
    path = "/v1/billing/exports",
    tag = "routing",
    request_body = BillingExportRequest,
    responses(
        (status = 202, description = "Accepted billing export job", body = BillingExportJobResponse),
        (status = 422, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn create_billing_export() {}

#[utoipa::path(
    get,
    path = "/v1/billing/exports",
    tag = "routing",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant id"),
        ("project_id" = Option<String>, Query, description = "Filter by project id")
    ),
    responses(
        (status = 200, description = "List billing export jobs", body = BillingExportJobsResponse),
        (status = 400, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn list_billing_exports() {}

#[utoipa::path(
    get,
    path = "/v1/billing/exports/{export_job_id}",
    tag = "routing",
    params(
        ("export_job_id" = String, Path, description = "Billing export job id")
    ),
    responses(
        (status = 200, description = "Get billing export job", body = BillingExportJobResponse),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_billing_export() {}

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
    path = "/v1/route-receipts",
    tag = "routing",
    responses(
        (status = 200, description = "List route receipts", body = RouteReceiptsResponse),
        (status = 500, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn list_route_receipts() {}

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
    get,
    path = "/v1/route-diagnostics/{route_policy_id}",
    tag = "routing",
    params(
        ("route_policy_id" = String, Path, description = "HugeRouter route policy id")
    ),
    responses(
        (status = 200, description = "Operator-focused route diagnostics", body = RouteDiagnosticsResponse),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_route_diagnostics() {}

#[utoipa::path(
    get,
    path = "/v1/route-receipts/{route_receipt_id}/diagnostics",
    tag = "routing",
    params(
        ("route_receipt_id" = String, Path, description = "HugeRouter route receipt id")
    ),
    responses(
        (status = 200, description = "Get route receipt diagnostics", body = RouteReceiptDiagnosticsResponse),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn get_route_receipt_diagnostics() {}

fn json_schema_artifacts() -> anyhow::Result<Vec<ArtifactFile>> {
    let schemas = vec![
        schema_artifact::<Tenant>("schemas/jsonschema/tenant.v1.schema.json")?,
        schema_artifact::<Project>("schemas/jsonschema/project.v1.schema.json")?,
        schema_artifact::<ProviderResource>("schemas/jsonschema/provider-resource.v1.schema.json")?,
        schema_artifact::<RoutePolicy>("schemas/jsonschema/route-policy.v1.schema.json")?,
        schema_artifact::<ConfigSnapshot>("schemas/jsonschema/config-snapshot.v1.schema.json")?,
        schema_artifact::<RouteReceipt>("schemas/jsonschema/route-receipt.v1.schema.json")?,
        schema_artifact::<RouteReceiptsResponse>(
            "schemas/jsonschema/route-receipts-response.v1.schema.json",
        )?,
        schema_artifact::<RouteDiagnosticsResponse>(
            "schemas/jsonschema/route-diagnostics-response.v1.schema.json",
        )?,
        schema_artifact::<UsageEvent>("schemas/jsonschema/usage-event.v1.schema.json")?,
        schema_artifact::<core_domain::NormalizedError>(
            "schemas/jsonschema/normalized-error.v1.schema.json",
        )?,
        schema_artifact::<ErrorEnvelope>("schemas/jsonschema/error-envelope.v1.schema.json")?,
        schema_artifact::<ChatRequest>("schemas/jsonschema/chat-request.v1.schema.json")?,
        schema_artifact::<RouteReceiptDiagnosticsResponse>(
            "schemas/jsonschema/route-receipt-diagnostics-response.v1.schema.json",
        )?,
        schema_artifact::<UsageSummaryResponse>(
            "schemas/jsonschema/usage-summary-response.v1.schema.json",
        )?,
        schema_artifact::<UsageBreakdownResponse>(
            "schemas/jsonschema/usage-breakdown-response.v1.schema.json",
        )?,
        schema_artifact::<BalanceProjectionResponse>(
            "schemas/jsonschema/balance-projection-response.v1.schema.json",
        )?,
        schema_artifact::<PricingCatalogResponse>(
            "schemas/jsonschema/pricing-catalog-response.v1.schema.json",
        )?,
        schema_artifact::<PricingSimulationRequest>(
            "schemas/jsonschema/pricing-simulation-request.v1.schema.json",
        )?,
        schema_artifact::<PricingSimulationResponse>(
            "schemas/jsonschema/pricing-simulation-response.v1.schema.json",
        )?,
        schema_artifact::<BillingExportRequest>(
            "schemas/jsonschema/billing-export-request.v1.schema.json",
        )?,
        schema_artifact::<BillingExportJobResponse>(
            "schemas/jsonschema/billing-export-job-response.v1.schema.json",
        )?,
        schema_artifact::<BillingExportJobsResponse>(
            "schemas/jsonschema/billing-export-jobs-response.v1.schema.json",
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

    Ok(vec![ArtifactFile {
        relative_path: "schemas/openapi/control-plane-v1.openapi.json",
        contents: control_plane,
    }])
}

#[allow(clippy::too_many_lines)]
fn example_artifacts() -> anyhow::Result<Vec<ArtifactFile>> {
    let tenant = sample_tenant();
    let project = sample_project();
    let provider_resource = sample_provider_resource();
    let bedrock_provider_resource = sample_bedrock_provider_resource();
    let route_policy = sample_route_policy();
    let config_snapshot = sample_config_snapshot();
    let route_receipt = sample_route_receipt();
    let route_receipts = RouteReceiptsResponse {
        data: vec![route_receipt.clone()],
    };
    let route_diagnostics = sample_route_diagnostics_response();
    let usage_event = sample_usage_event();
    let simulation_request = sample_route_simulation_request();
    let simulation_response = sample_route_simulation_response();
    let route_receipt_diagnostics = sample_route_receipt_diagnostics();
    let usage_summary = sample_usage_summary_response();
    let usage_breakdown = sample_usage_breakdown_response();
    let balance_projection = sample_balance_projection_response();
    let pricing_catalog = sample_pricing_catalog_response();
    let pricing_simulation_request = sample_pricing_simulation_request();
    let pricing_simulation_response = sample_pricing_simulation_response();
    let billing_export_request = sample_billing_export_request();
    let billing_export_job = sample_billing_export_job_response();
    let billing_export_jobs = BillingExportJobsResponse {
        data: vec![billing_export_job.data.clone()],
    };
    let usage_message = sample_usage_event_recorded_message();
    let snapshot_message = sample_config_snapshot_activated_message();

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
                data: vec![provider_resource, bedrock_provider_resource],
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
            "schemas/examples/control-plane/route-receipts.response.json",
            &route_receipts,
        )?,
        example_artifact(
            "schemas/examples/control-plane/route-diagnostics.response.json",
            &route_diagnostics,
        )?,
        example_artifact(
            "schemas/examples/control-plane/route-receipt-diagnostics.response.json",
            &route_receipt_diagnostics,
        )?,
        example_artifact(
            "schemas/examples/control-plane/usage-summary.response.json",
            &usage_summary,
        )?,
        example_artifact(
            "schemas/examples/control-plane/usage-breakdown.response.json",
            &usage_breakdown,
        )?,
        example_artifact(
            "schemas/examples/control-plane/balance-projection.response.json",
            &balance_projection,
        )?,
        example_artifact(
            "schemas/examples/control-plane/pricing-catalog.response.json",
            &pricing_catalog,
        )?,
        example_artifact(
            "schemas/examples/control-plane/pricing-simulation.request.json",
            &pricing_simulation_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/pricing-simulation.response.json",
            &pricing_simulation_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/billing-export.request.json",
            &billing_export_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/billing-export.response.json",
            &billing_export_job,
        )?,
        example_artifact(
            "schemas/examples/control-plane/billing-exports.response.json",
            &billing_export_jobs,
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
export const PROTOCOL_FAMILIES = ['openai_chat', 'openai_responses', 'openai_images', 'mcp_streamable_http', 'realtime_webrtc', 'anthropic_messages', 'gemini_generate_content'] as const;\n\
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
  {{ id: 'getUsageSummary', method: 'GET', path: '/v1/usage/summary' }},\n\
  {{ id: 'getUsageBreakdown', method: 'GET', path: '/v1/usage/breakdown' }},\n\
  {{ id: 'getBalanceProjection', method: 'GET', path: '/v1/billing/projection' }},\n\
  {{ id: 'getPricingCatalog', method: 'GET', path: '/v1/pricing/catalog' }},\n\
  {{ id: 'createPricingSimulation', method: 'POST', path: '/v1/pricing/simulations' }},\n\
  {{ id: 'createBillingExport', method: 'POST', path: '/v1/billing/exports' }},\n\
  {{ id: 'listBillingExports', method: 'GET', path: '/v1/billing/exports' }},\n\
  {{ id: 'getBillingExport', method: 'GET', path: '/v1/billing/exports/{{export_job_id}}' }},\n\
  {{ id: 'simulateRoute', method: 'POST', path: '/v1/route-simulations' }},\n\
  {{ id: 'listRouteReceipts', method: 'GET', path: '/v1/route-receipts' }},\n\
  {{ id: 'getRouteReceipt', method: 'GET', path: '/v1/route-receipts/{{route_receipt_id}}' }},\n\
  {{ id: 'getRouteDiagnostics', method: 'GET', path: '/v1/route-diagnostics/{{route_policy_id}}' }},\n\
  {{ id: 'getRouteReceiptDiagnostics', method: 'GET', path: '/v1/route-receipts/{{route_receipt_id}}/diagnostics' }},\n\
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
        "route_receipt_diagnostics".to_string(),
        "usage_summary".to_string(),
        "usage_breakdown".to_string(),
        "balance_projection".to_string(),
        "pricing_catalog".to_string(),
        "pricing_simulation".to_string(),
        "billing_export_job".to_string(),
        "billing_export_jobs".to_string(),
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
        health_message: Some("probe latency within SLO".to_string()),
        quarantine_reason: None,
        budget_policy_id: Some(core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap()),
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
        ],
        is_transit_gateway: false,
        version: 7,
        created_at: "2026-04-20T00:00:00Z".to_string(),
        updated_at: "2026-04-21T11:15:00Z".to_string(),
    }
}

fn sample_bedrock_provider_resource() -> ProviderResource {
    ProviderResource {
        provider_resource_id: ProviderResourceId::parse("prvrsrc_bedrock_claude").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: Some(ProjectId::parse("proj_acme_ops").unwrap()),
        provider_id: "bedrock".to_string(),
        name: "Bedrock Claude".to_string(),
        status: core_domain::ProviderResourceStatus::Active,
        provenance_class: core_domain::ProvenanceClass::OfficialApi,
        credential_owner_type: core_domain::CredentialOwnerType::Platform,
        deployment_scope: core_domain::DeploymentScope::Shared,
        region: "us-east-1".to_string(),
        endpoint_base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
        auth_kind: core_domain::AuthKind::ApiKey,
        health_state: core_domain::HealthState::Healthy,
        health_message: Some("aws credential chain available".to_string()),
        quarantine_reason: None,
        budget_policy_id: None,
        capabilities: core_domain::ProviderCapabilities {
            supports_streaming: false,
            supports_tool_calling: false,
            supports_json_mode: false,
            supports_realtime: false,
            supports_response_model_metadata: true,
        },
        supported_protocol_families: vec!["openai_chat".to_string()],
        is_transit_gateway: false,
        version: 1,
        created_at: "2026-04-22T00:00:00Z".to_string(),
        updated_at: "2026-04-22T00:00:00Z".to_string(),
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
        route_policy_id: RoutePolicyId::parse("routepol_default").unwrap(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
        admission_result: AdmissionResult::Admitted,
        selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
        excluded_targets: vec![core_domain::ExcludedTarget {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_backup").unwrap(),
            reason_code: "rejected_provenance_class".to_string(),
            reason: "rejected_provenance_class".to_string(),
        }],
        failure_reason: None,
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
            reason_code: "rejected_provenance_class".to_string(),
            reason: "rejected_provenance_class".to_string(),
        }],
        selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.1420".to_string(),
        },
    }
}

fn sample_route_receipt_diagnostics() -> RouteReceiptDiagnosticsResponse {
    RouteReceiptDiagnosticsResponse {
        route_receipt: sample_route_receipt(),
        decision_timeline: vec![
            RouteReceiptDecisionTraceStep {
                stage: "admission".to_string(),
                status: "passed".to_string(),
                message: "Tenant policy accepted request".to_string(),
                score: Some(1.0),
                notes: vec!["all constraints satisfied".to_string()],
            },
            RouteReceiptDecisionTraceStep {
                stage: "candidate_selection".to_string(),
                status: "passed".to_string(),
                message: "Selected openai-us-east-primary".to_string(),
                score: Some(0.91),
                notes: Vec::new(),
            },
        ],
        policy_checks: vec![RouteReceiptPolicyCheck {
            policy_id: RoutePolicyId::parse("routepol_default").unwrap(),
            status: "passed".to_string(),
            reason: Some("policy satisfied".to_string()),
        }],
        provider_attempts: vec![RouteReceiptProviderAttempt {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            attempt: 1,
            status: "success".to_string(),
            reason_code: "provider_success".to_string(),
            retryable: false,
            fallback_target: None,
            started_at: "2026-04-21T12:16:01Z".to_string(),
            finished_at: "2026-04-21T12:16:03Z".to_string(),
            latency_ms: 1100,
            reason: "succeeded with output".to_string(),
        }],
        metadata: BTreeMap::from([
            ("policy_cache_hit".to_string(), "true".to_string()),
            ("candidate_pool_size".to_string(), "3".to_string()),
        ]),
    }
}

fn sample_route_diagnostics_response() -> RouteDiagnosticsResponse {
    let provider_resource = sample_provider_resource();
    let route_policy = sample_route_policy();
    let route_receipt = sample_route_receipt();

    RouteDiagnosticsResponse {
        route_policy,
        active_snapshot: Some(sample_config_snapshot()),
        active_snapshot_matches_route_policy: true,
        last_route_receipt: Some(RouteReceiptSummary {
            route_receipt_id: route_receipt.route_receipt_id.clone(),
            admission_result: route_receipt.admission_result,
            selected_target: route_receipt.selected_target.clone(),
            failure_reason: route_receipt.failure_reason.clone(),
            created_at: route_receipt.created_at.clone(),
        }),
        recent_receipts: vec![RouteReceiptSummary {
            route_receipt_id: route_receipt.route_receipt_id.clone(),
            admission_result: route_receipt.admission_result,
            selected_target: route_receipt.selected_target.clone(),
            failure_reason: route_receipt.failure_reason.clone(),
            created_at: route_receipt.created_at.clone(),
        }],
        targets: vec![RouteDiagnosticTarget {
            provider_resource,
            decision: RouteDiagnosticDecision::Selected,
            in_active_snapshot: true,
            supports_protocol_family: true,
            capability_gaps: Vec::new(),
            reason_code: "selected_recent_receipt".to_string(),
            reason: "Selected by the most recent route receipt.".to_string(),
            recent_receipt_id: Some(route_receipt.route_receipt_id),
            recent_receipt_reason: None,
        }],
    }
}

fn sample_usage_summary_response() -> UsageSummaryResponse {
    UsageSummaryResponse {
        data: UsageSummary {
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
            window_start: "2026-04-21T00:00:00Z".to_string(),
            window_end: "2026-04-21T23:59:59Z".to_string(),
            currency: "USD".to_string(),
            event_count: 14,
            input_tokens: 18_420,
            output_tokens: 6_245,
            cached_input_tokens: 1_220,
            provider_cost: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.124500".to_string(),
            },
            billable_price: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.152025".to_string(),
            },
        },
    }
}

fn sample_usage_breakdown_response() -> UsageBreakdownResponse {
    UsageBreakdownResponse {
        data: vec![
            UsageBreakdownRow {
                bucket: "openai".to_string(),
                provider_id: Some("openai".to_string()),
                model_alias: None,
                input_tokens: 10_000,
                output_tokens: 4_000,
                cached_input_tokens: 500,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.082000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.098400".to_string(),
                },
            },
            UsageBreakdownRow {
                bucket: "reasoning-fast".to_string(),
                provider_id: None,
                model_alias: Some("reasoning-fast".to_string()),
                input_tokens: 8_420,
                output_tokens: 2_245,
                cached_input_tokens: 720,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.042500".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.053625".to_string(),
                },
            },
        ],
        next_cursor: Some("2".to_string()),
    }
}

fn sample_balance_projection_response() -> BalanceProjectionResponse {
    BalanceProjectionResponse {
        data: BalanceProjection {
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
            currency: "USD".to_string(),
            provider_cost_total: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "1.244000".to_string(),
            },
            billable_total: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "1.540000".to_string(),
            },
            configured_budget: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "75.000000".to_string(),
            },
            remaining_budget: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "73.460000".to_string(),
            },
            threshold_status: "ok".to_string(),
            last_projected_at: "2026-04-21T12:20:00Z".to_string(),
            projection_lag_seconds: 18,
        },
    }
}

fn sample_pricing_catalog_response() -> PricingCatalogResponse {
    PricingCatalogResponse {
        catalog_id: "pricing_catalog_default".to_string(),
        catalog_version: 1,
        currency: "USD".to_string(),
        entries: vec![
            PricingCatalogEntry {
                dimension: "input_tokens".to_string(),
                provider_id: "openai".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 2_500,
                unit_denominator: 1_000,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "image_generations".to_string(),
                provider_id: "openai".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 18_000,
                unit_denominator: 1,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "audio_seconds".to_string(),
                provider_id: "openai".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 1_500,
                unit_denominator: 1,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "input_tokens".to_string(),
                provider_id: "bedrock".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 6_000,
                unit_denominator: 1_000,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "output_tokens".to_string(),
                provider_id: "bedrock".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 30_000,
                unit_denominator: 1_000,
                source: "provider_native".to_string(),
            },
        ],
    }
}

fn sample_pricing_simulation_request() -> PricingSimulationRequest {
    PricingSimulationRequest {
        provider_id: "openai".to_string(),
        model_alias: "reasoning-fast".to_string(),
        usage: UsageMetrics {
            input_tokens: 1_200,
            output_tokens: 320,
            cached_input_tokens: 64,
        },
        region: Some("us-east-1".to_string()),
        image_generation_units: Some(1),
        audio_seconds: Some(8),
    }
}

fn sample_pricing_simulation_response() -> PricingSimulationResponse {
    PricingSimulationResponse {
        catalog_id: "pricing_catalog_default".to_string(),
        catalog_version: 1,
        currency: "USD".to_string(),
        provider_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.005188".to_string(),
        },
        billable_price: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.006225".to_string(),
        },
        line_items: vec![
            PricingSimulationLineItem {
                dimension: "input_tokens".to_string(),
                units: 1_200,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.003000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.003600".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "output_tokens".to_string(),
                units: 320,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.002720".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.003264".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "cached_input_tokens".to_string(),
                units: 64,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.000048".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.000057".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "image_generations".to_string(),
                units: 1,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.018000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.021600".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "audio_seconds".to_string(),
                units: 8,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.012000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.014400".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
        ],
    }
}

fn sample_billing_export_request() -> BillingExportRequest {
    BillingExportRequest {
        tenant_id: Some(TenantId::parse("tenant_acme").unwrap()),
        project_id: Some(ProjectId::parse("proj_core").unwrap()),
        window_start: "2026-04-01T00:00:00Z".to_string(),
        window_end: "2026-04-30T23:59:59Z".to_string(),
        format: "csv".to_string(),
    }
}

fn sample_billing_export_job_response() -> BillingExportJobResponse {
    BillingExportJobResponse {
        data: BillingExportJob {
            export_job_id: "export_123".to_string(),
            status: "queued".to_string(),
            format: "csv".to_string(),
            requested_at: "2026-04-21T12:25:00Z".to_string(),
            completed_at: None,
            error_message: None,
            tenant_id: Some(TenantId::parse("tenant_acme").unwrap()),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
        },
    }
}

fn sample_usage_event_recorded_message() -> UsageEventRecordedMessage {
    let envelope = MessageEnvelope::new(
        "msg_usage_123",
        MessageType::UsageEventRecorded,
        "2026-04-21T12:16:05Z",
        ServiceName::parse("control-plane-api").unwrap(),
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

#[cfg(test)]
fn sample_route_receipt_recorded_message() -> RouteReceiptRecordedMessage {
    let diagnostics = sample_route_receipt_diagnostics();
    let route_receipt = diagnostics.route_receipt.clone();
    RouteReceiptRecordedMessage {
        message_id: "msg_routercpt_123".to_string(),
        message_type: RouteReceiptRecordedMessageType::RouteReceiptRecorded,
        schema_version: 1,
        occurred_at: route_receipt.created_at.clone(),
        producer: ServiceName::parse("control-plane-api").unwrap(),
        trace_id: Some("trace_123".to_string()),
        request_id: Some("req_123".to_string()),
        idempotency_key: format!("{}:recorded", route_receipt.route_receipt_id),
        payload: RouteReceiptRecorded {
            route_receipt,
            decision_timeline: diagnostics.decision_timeline,
            policy_checks: diagnostics.policy_checks,
            provider_attempts: diagnostics.provider_attempts,
        },
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
        RequestEnvelope, RouteReceiptRecordedMessage, collect_contract_artifacts,
        sample_route_receipt_diagnostics, sample_route_receipt_recorded_message, workspace_root,
    };
    use core_domain::{ConfigSnapshot, ProjectId, ServiceName, TenantId};

    #[test]
    fn request_envelope_uses_stable_public_protocol_names() {
        let envelope = RequestEnvelope::new(
            ProtocolFamily::OpenAiChat,
            ServiceName::parse("control-plane-api").unwrap(),
            "req_123",
            "trace_123",
            TenantId::parse("tenant_acme").unwrap(),
            ProjectId::parse("proj_core").unwrap(),
        );

        let json = serde_json::to_value(&envelope).unwrap();

        assert_eq!(json["protocol_family"], "openai_chat");
        assert_eq!(json["source_service"], "control-plane-api");
        assert_eq!(json["tenant_id"], "tenant_acme");
    }

    #[test]
    fn request_envelope_supports_new_protocol_families() {
        let families = serde_json::to_value(vec![
            ProtocolFamily::OpenAiImages,
            ProtocolFamily::AnthropicMessages,
            ProtocolFamily::GeminiGenerateContent,
        ])
        .unwrap();

        assert_eq!(families[0], "openai_images");
        assert_eq!(families[1], "anthropic_messages");
        assert_eq!(families[2], "gemini_generate_content");
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
    fn route_receipt_diagnostics_example_round_trips() {
        let diagnostics = sample_route_receipt_diagnostics();
        let value = serde_json::to_value(&diagnostics).unwrap();

        assert_eq!(value["route_receipt"]["route_receipt_id"], "routercpt_123");
        assert_eq!(value["decision_timeline"][0]["stage"], "admission");
        assert_eq!(value["provider_attempts"][0]["attempt"], 1);
    }

    #[test]
    fn route_receipt_recorded_message_round_trips() {
        let event = sample_route_receipt_recorded_message();
        let value = serde_json::to_value(&event).unwrap();
        let reparsed: RouteReceiptRecordedMessage = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(reparsed, event);
        assert_eq!(value["message_type"], "route_receipt.recorded");
        assert_eq!(
            value["payload"]["route_receipt"]["route_receipt_id"],
            "routercpt_123"
        );
        assert_eq!(
            value["payload"]["provider_attempts"][0]["status"],
            "success"
        );
    }
}
