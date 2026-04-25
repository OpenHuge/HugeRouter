#![allow(clippy::wildcard_imports)]

use super::*;
use utoipa::OpenApi;

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

#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "HugeRouter Gateway API",
        version = "v1",
        description = "Stable gateway contracts for chat routing and normalized failures."
    ),
    paths(
        gateway_chat_completion,
        gateway_anthropic_messages,
        gateway_gemini_generate_content,
    ),
    components(
        schemas(
            ErrorEnvelope,
            GatewayChatRequest,
            GatewayChatResponse,
            GatewayAnthropicMessagesError,
            GatewayAnthropicMessagesRequest,
            GatewayAnthropicMessagesResponse,
            GatewayGeminiGenerateContentError,
            GatewayGeminiGenerateContentRequest,
            GatewayGeminiGenerateContentResponse,
            GatewayAnthropicMessage,
            GatewayAnthropicMessageContent,
            GatewayAnthropicMessageContentBlock,
            GatewayAnthropicResponseContentBlock,
            GatewayGeminiContent,
            GatewayGeminiGenerationConfig,
            GatewayGeminiPart,
            GatewayGeminiRole,
            GatewayGeminiSystemInstruction,
            ProtocolFamily,
            RequestEnvelope,
            RouteReceipt,
            UsageEvent,
            ChatRequest,
            ChatMessage,
            ChatMessageRole,
            ToolDefinition,
            RouteReceiptDecisionTraceStep,
            RouteReceiptProviderAttempt,
            RouteReceiptPolicyCheck,
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

#[utoipa::path(
    post,
    path = "/v1/chat/completions",
    tag = "gateway",
    request_body = GatewayChatRequest,
    responses(
        (status = 200, description = "Chat routed successfully", body = GatewayChatResponse),
        (status = 422, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
const fn gateway_chat_completion() {}

#[utoipa::path(
    post,
    path = "/v1/messages",
    tag = "gateway",
    request_body = GatewayAnthropicMessagesRequest,
    responses(
        (status = 200, description = "Anthropic messages routed successfully", body = GatewayAnthropicMessagesResponse),
        (status = 422, description = "Normalized error", body = GatewayAnthropicMessagesError),
    )
)]
#[allow(dead_code)]
const fn gateway_anthropic_messages() {}

#[utoipa::path(
    post,
    path = "/v1beta/models/{model}:generateContent",
    tag = "gateway",
    params(
        ("model" = String, Path, description = "Gemini model name")
    ),
    request_body = GatewayGeminiGenerateContentRequest,
    responses(
        (status = 200, description = "Gemini generate-content routed successfully", body = GatewayGeminiGenerateContentResponse),
        (status = 422, description = "Normalized error", body = GatewayGeminiGenerateContentError),
    )
)]
#[allow(dead_code)]
const fn gateway_gemini_generate_content() {}
