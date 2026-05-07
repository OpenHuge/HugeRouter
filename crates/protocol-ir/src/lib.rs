#![allow(clippy::needless_for_each)]

mod artifacts;
pub(crate) mod examples;
pub(crate) mod gateway_examples;
mod openapi_docs;

pub use artifacts::{
    ArtifactFile, ContractManifest, collect_contract_artifacts, workspace_root,
    write_contract_artifacts,
};
pub use openapi_docs::{ControlPlaneApiDoc, GatewayApiDoc};

use core_domain::{
    AdmissionResult, ConfigSnapshot, ConfigSnapshotId, ErrorEnvelope, LedgerEntry, MonetaryAmount,
    Project, ProjectId, ProviderResource, ProviderResourceId, RoutePolicy, RoutePolicyId,
    RouteReceipt, RouteReceiptId, ServiceName, Tenant, TenantId, UsageEvent, UsageMetrics,
    UsagePhase,
};
#[cfg(test)]
use examples::{
    sample_route_receipt_diagnostics, sample_route_receipt_recorded_message,
    sample_usage_event_recorded_message,
};
#[cfg(test)]
use gateway_examples::{
    sample_gateway_anthropic_messages_request, sample_gateway_anthropic_messages_response,
    sample_gateway_chat_request, sample_gateway_gemini_generate_content_request,
    sample_gateway_gemini_generate_content_response,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayAnthropicMessage {
    pub role: String,
    pub content: GatewayAnthropicMessageContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(untagged)]
pub enum GatewayAnthropicMessageContent {
    PlainText(String),
    Blocks(Vec<GatewayAnthropicMessageContentBlock>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayAnthropicMessageContentBlock {
    Text { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayAnthropicResponseContentBlock {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayAnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<GatewayAnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayAnthropicMessagesResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub content: Vec<GatewayAnthropicResponseContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<GatewayAnthropicUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayAnthropicMessagesError {
    pub error: core_domain::NormalizedError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GatewayGeminiRole {
    User,
    Assistant,
    Model,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayGeminiPart {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayGeminiContent {
    pub role: GatewayGeminiRole,
    pub parts: Vec<GatewayGeminiPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayGeminiSystemInstruction {
    pub parts: Vec<GatewayGeminiPart>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GatewayGeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayGeminiGenerateContentRequest {
    pub model: String,
    pub contents: Vec<GatewayGeminiContent>,
    #[serde(default)]
    pub tools: Vec<Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(
        rename = "systemInstruction",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub system_instruction: Option<GatewayGeminiSystemInstruction>,
    #[serde(
        rename = "generationConfig",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub generation_config: Option<GatewayGeminiGenerationConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayGeminiGenerateContentResponse {
    #[serde(rename = "responseId", skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    pub candidates: Vec<GatewayGeminiCandidate>,
    #[serde(rename = "usageMetadata", skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GatewayGeminiUsageMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "modelVersion")]
    pub model_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayGeminiGenerateContentError {
    pub error: core_domain::NormalizedError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayAnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayGeminiCandidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<GatewayGeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GatewayGeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: u32,
    #[serde(rename = "cachedContentTokenCount")]
    pub cached_content_token_count: u32,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_account_id: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_account_id: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CreateRenewalIntentRequest {
    pub out_trade_no: String,
    pub grant_id: String,
    pub renew_expires_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RenewalIntent {
    pub renewal_intent_id: String,
    pub out_trade_no: String,
    pub grant_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub payment_status: String,
    pub previous_grant_status: String,
    pub previous_expires_at: String,
    pub renew_expires_at: String,
    pub status: String,
    pub reason_code: String,
    pub reason: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RenewalIntentResponse {
    pub data: RenewalIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RenewalIntentsResponse {
    pub data: Vec<RenewalIntent>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SaleReadyCheck {
    pub name: String,
    pub status: String,
    pub reason_code: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SaleReadiness {
    pub status: String,
    pub reason_code: String,
    pub reason: String,
    pub checks: Vec<SaleReadyCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SaleReadyHandoff {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub config_snapshot_id: ConfigSnapshotId,
    pub route_policy_id: RoutePolicyId,
    pub budget_policy_id: core_domain::BudgetPolicyId,
    pub provider_resource_ids: Vec<ProviderResourceId>,
    pub readiness_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct SaleReadyPackageResponse {
    pub config_snapshot: ConfigSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_policy: Option<RoutePolicy>,
    pub provider_resources: Vec<ProviderResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_simulation: Option<RouteSimulationResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<PricingSimulationResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<BalanceProjection>,
    pub readiness: SaleReadiness,
    pub handoff: SaleReadyHandoff,
    pub creates_customer_api_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CreateDeliveryRequest {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_kind: Option<String>,
    pub service_days: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryRevokeRequest {
    pub expected_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Delivery {
    pub delivery_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub operator_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_label: Option<String>,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryCode {
    pub code_id: String,
    pub delivery_id: String,
    pub code_type: String,
    pub code_prefix: String,
    pub code_last_four: String,
    pub format_version: String,
    pub status: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryEntitlement {
    pub entitlement_id: String,
    pub delivery_id: String,
    pub service_kind: String,
    pub service_days: u32,
    pub starts_at: String,
    pub ends_at: String,
    pub service_starts_at: String,
    pub service_ends_at: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryProjection {
    pub delivery: Delivery,
    pub codes: Vec<DeliveryCode>,
    pub entitlement: DeliveryEntitlement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOneTimeCodes {
    pub redemption_code: String,
    pub browser_file_unlock_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryPrepareResponse {
    pub data: DeliveryProjection,
    pub one_time_codes: DeliveryOneTimeCodes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryResponse {
    pub data: DeliveryProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CreateDeliveryArtifactRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier_valid_until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption_protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_kind: Option<String>,
    pub payload_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryArtifact {
    pub artifact_id: String,
    pub delivery_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub artifact_kind: String,
    pub provider: String,
    pub status: String,
    pub version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    pub content_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub storage_backend: String,
    pub storage_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier_valid_until: Option<String>,
    pub encryption_protocol: String,
    pub encryption_version: String,
    pub secret_kind: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryArtifactResponse {
    pub data: DeliveryArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryArtifactsResponse {
    pub data: Vec<DeliveryArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryActivationRestoreInfo {
    pub artifact_import_secret: String,
    pub secret_kind: String,
    pub encryption_protocol: String,
    pub encryption_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryActivationRedeemResponse {
    pub data: DeliveryActivation,
    pub restore: DeliveryActivationRestoreInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CreateDeliveryUploadBatchRequest {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub source_file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub items: Vec<DeliveryUploadBatchItemInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryUploadBatchItemInput {
    pub delivery_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier_valid_until: Option<String>,
    pub payload_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryUploadBatch {
    pub batch_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub source_file_name: String,
    pub source_file_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub total_count: u32,
    pub success_count: u32,
    pub failed_count: u32,
    pub duplicate_count: u32,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_summary: Option<String>,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryUploadBatchItem {
    pub item_id: String,
    pub batch_id: String,
    pub row_index: u32,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub delivery_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    pub status: String,
    pub artifact_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    pub content_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier_valid_until: Option<String>,
    pub payload_sha256: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryUploadBatchResponse {
    pub data: DeliveryUploadBatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryUploadBatchItemsResponse {
    pub data: Vec<DeliveryUploadBatchItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RedeemDeliveryRequest {
    pub redemption_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryActivation {
    pub activation_id: String,
    pub delivery_id: String,
    pub artifact_id: String,
    pub entitlement_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub activation_source: String,
    pub activated_at: String,
    pub entitlement_ends_at: String,
    pub artifact: DeliveryArtifact,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryActivationResponse {
    pub data: DeliveryActivation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CreateDeliveryDownloadGrantRequest {
    pub activation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redemption_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryDownloadGrantRevokeRequest {
    pub expected_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryDownloadGrant {
    pub grant_id: String,
    pub activation_id: String,
    pub delivery_id: String,
    pub artifact_id: String,
    pub entitlement_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub token_prefix: String,
    pub token_last_four: String,
    pub expires_at: String,
    pub max_uses: u32,
    pub use_count: u32,
    pub artifact: DeliveryArtifact,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryDownloadGrantIssueResponse {
    pub data: DeliveryDownloadGrant,
    pub download_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryDownloadGrantResponse {
    pub data: DeliveryDownloadGrant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ExtendDeliveryEntitlementRequest {
    pub expected_version: u64,
    pub extend_days: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryServiceSegment {
    pub segment_id: String,
    pub entitlement_id: String,
    pub activation_id: String,
    pub delivery_id: String,
    pub artifact_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub segment_index: u32,
    pub effective_from: String,
    pub effective_until: String,
    pub carrier_valid_until: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryLifecycleEvent {
    pub event_id: String,
    pub entitlement_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    pub event_type: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub created_by: String,
    pub created_at: String,
    #[serde(default)]
    pub payload: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryServiceSegmentsResponse {
    pub data: Vec<DeliveryServiceSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryLifecycleEventsResponse {
    pub data: Vec<DeliveryLifecycleEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryLifecycleResponse {
    pub entitlement: DeliveryEntitlement,
    pub segments: Vec<DeliveryServiceSegment>,
    pub events: Vec<DeliveryLifecycleEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsTotals {
    pub deliveries: u64,
    pub artifacts: u64,
    pub upload_batches: u64,
    pub upload_items: u64,
    pub activations: u64,
    pub download_grants: u64,
    pub entitlements: u64,
    pub service_segments: u64,
    pub lifecycle_events: u64,
    pub exceptions: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsStatusCount {
    pub domain: String,
    pub status: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsTimelineEvent {
    pub event_id: String,
    pub event_type: String,
    pub object_type: String,
    pub object_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entitlement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_batch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_item_id: Option<String>,
    pub status: String,
    pub occurred_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsException {
    pub exception_id: String,
    pub exception_type: String,
    pub severity: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entitlement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_batch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_item_id: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub occurred_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsOverview {
    pub tenant_id: TenantId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub window_start: String,
    pub window_end: String,
    pub totals: DeliveryOperationsTotals,
    pub status_counts: Vec<DeliveryOperationsStatusCount>,
    pub recent_events: Vec<DeliveryOperationsTimelineEvent>,
    pub exceptions: Vec<DeliveryOperationsException>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsOverviewResponse {
    pub data: DeliveryOperationsOverview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsTimelineResponse {
    pub data: Vec<DeliveryOperationsTimelineEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsExceptionsResponse {
    pub data: Vec<DeliveryOperationsException>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsDetail {
    pub delivery: DeliveryProjection,
    pub artifacts: Vec<DeliveryArtifact>,
    #[serde(default)]
    pub upload_items: Vec<DeliveryUploadBatchItem>,
    pub activations: Vec<DeliveryActivation>,
    pub download_grants: Vec<DeliveryDownloadGrant>,
    pub service_segments: Vec<DeliveryServiceSegment>,
    pub lifecycle_events: Vec<DeliveryLifecycleEvent>,
    pub timeline: Vec<DeliveryOperationsTimelineEvent>,
    pub exceptions: Vec<DeliveryOperationsException>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryOperationsDetailResponse {
    pub data: DeliveryOperationsDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CreateOpeningGrantRequest {
    pub config_snapshot_id: ConfigSnapshotId,
    pub owner_account_id: String,
    pub grantee_kind: String,
    pub grantee_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grantee_label: Option<String>,
    pub expires_at: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct OpeningGrantRevokeRequest {
    pub expected_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct OpeningGrant {
    pub grant_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub owner_account_id: String,
    pub grantee_kind: String,
    pub grantee_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grantee_label: Option<String>,
    pub config_snapshot_id: ConfigSnapshotId,
    pub route_policy_id: RoutePolicyId,
    pub budget_policy_id: core_domain::BudgetPolicyId,
    pub provider_resource_ids: Vec<ProviderResourceId>,
    pub credential_kind: String,
    pub credential_id: String,
    pub credential_key_prefix: String,
    pub credential_last_four: String,
    pub scopes: Vec<String>,
    pub expires_at: String,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct OpeningCredential {
    pub credential_kind: String,
    pub credential_id: String,
    pub key_prefix: String,
    pub last_four: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plaintext: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct OpeningGrantCreateResponse {
    pub grant: OpeningGrant,
    pub credential: OpeningCredential,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct OpeningGrantsResponse {
    pub data: Vec<OpeningGrant>,
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

#[cfg(test)]
mod tests {
    use super::{
        ChatRequest, ConfigSnapshotActivated, MessageEnvelope, MessageType, ProtocolFamily,
        RequestEnvelope, RouteReceiptRecordedMessage, collect_contract_artifacts,
        sample_gateway_anthropic_messages_request, sample_gateway_anthropic_messages_response,
        sample_gateway_chat_request, sample_gateway_gemini_generate_content_request,
        sample_gateway_gemini_generate_content_response, sample_route_receipt_diagnostics,
        sample_route_receipt_recorded_message, sample_usage_event_recorded_message, workspace_root,
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
    fn gateway_examples_round_trip_to_json() {
        let request = sample_gateway_chat_request();
        request.chat.validate().unwrap();

        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["request"]["protocol_family"], "openai_chat");
        assert_eq!(value["chat"]["messages"][0]["role"], "system");

        let event = sample_usage_event_recorded_message();
        let event_json = serde_json::to_value(&event).unwrap();
        assert_eq!(event_json["message_type"], "usage_event.recorded");

        let route_receipt_event = sample_route_receipt_recorded_message();
        let route_receipt_event_json = serde_json::to_value(&route_receipt_event).unwrap();
        assert_eq!(
            route_receipt_event_json["message_type"],
            "route_receipt.recorded"
        );

        let anthropic_request = sample_gateway_anthropic_messages_request();
        let anthropic_response = sample_gateway_anthropic_messages_response();
        assert_eq!(
            serde_json::to_value(&anthropic_request).unwrap()["messages"][0]["role"],
            "user"
        );
        assert_eq!(
            serde_json::to_value(&anthropic_response).unwrap()["content"][0]["type"],
            "text"
        );

        let gemini_request = sample_gateway_gemini_generate_content_request();
        let gemini_response = sample_gateway_gemini_generate_content_response();
        assert_eq!(
            serde_json::to_value(&gemini_request).unwrap()["contents"][0]["role"],
            "user"
        );
        assert_eq!(
            serde_json::to_value(&gemini_response).unwrap()["candidates"][0]["content"]["role"],
            "model"
        );
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
            "succeeded"
        );
    }
}
