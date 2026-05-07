#![allow(clippy::wildcard_imports)]

use anyhow::Context;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use utoipa::OpenApi;

use super::*;
use crate::examples::*;
use crate::gateway_examples::*;

const CONTRACT_VERSION: &str = "v1";
const CONTRACT_GENERATION_COMMAND: &str = "pnpm generate";

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

#[allow(clippy::too_many_lines)]
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
        schema_artifact::<GatewayChatRequest>(
            "schemas/jsonschema/gateway-chat-request.v1.schema.json",
        )?,
        schema_artifact::<GatewayChatResponse>(
            "schemas/jsonschema/gateway-chat-response.v1.schema.json",
        )?,
        schema_artifact::<GatewayAnthropicMessagesRequest>(
            "schemas/jsonschema/gateway-anthropic-messages-request.v1.schema.json",
        )?,
        schema_artifact::<GatewayAnthropicMessagesResponse>(
            "schemas/jsonschema/gateway-anthropic-messages-response.v1.schema.json",
        )?,
        schema_artifact::<GatewayAnthropicMessagesError>(
            "schemas/jsonschema/gateway-anthropic-messages-error.v1.schema.json",
        )?,
        schema_artifact::<GatewayGeminiGenerateContentRequest>(
            "schemas/jsonschema/gateway-gemini-generate-content-request.v1.schema.json",
        )?,
        schema_artifact::<GatewayGeminiGenerateContentResponse>(
            "schemas/jsonschema/gateway-gemini-generate-content-response.v1.schema.json",
        )?,
        schema_artifact::<GatewayGeminiGenerateContentError>(
            "schemas/jsonschema/gateway-gemini-generate-content-error.v1.schema.json",
        )?,
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
        schema_artifact::<CreateOpeningGrantRequest>(
            "schemas/jsonschema/opening-grant-create-request.v1.schema.json",
        )?,
        schema_artifact::<OpeningGrant>("schemas/jsonschema/opening-grant.v1.schema.json")?,
        schema_artifact::<OpeningGrantCreateResponse>(
            "schemas/jsonschema/opening-grant-create-response.v1.schema.json",
        )?,
        schema_artifact::<OpeningGrantRevokeRequest>(
            "schemas/jsonschema/opening-grant-revoke-request.v1.schema.json",
        )?,
        schema_artifact::<OpeningGrantsResponse>(
            "schemas/jsonschema/opening-grants-response.v1.schema.json",
        )?,
        schema_artifact::<CreateDeliveryRequest>(
            "schemas/jsonschema/delivery-prepare-request.v1.schema.json",
        )?,
        schema_artifact::<DeliveryPrepareResponse>(
            "schemas/jsonschema/delivery-prepare-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryResponse>("schemas/jsonschema/delivery-response.v1.schema.json")?,
        schema_artifact::<DeliveryRevokeRequest>(
            "schemas/jsonschema/delivery-revoke-request.v1.schema.json",
        )?,
        schema_artifact::<CreateDeliveryArtifactRequest>(
            "schemas/jsonschema/delivery-artifact-create-request.v1.schema.json",
        )?,
        schema_artifact::<DeliveryArtifactResponse>(
            "schemas/jsonschema/delivery-artifact-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryArtifactsResponse>(
            "schemas/jsonschema/delivery-artifacts-response.v1.schema.json",
        )?,
        schema_artifact::<CreateDeliveryUploadBatchRequest>(
            "schemas/jsonschema/delivery-upload-batch-request.v1.schema.json",
        )?,
        schema_artifact::<DeliveryUploadBatchResponse>(
            "schemas/jsonschema/delivery-upload-batch-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryUploadBatchItemsResponse>(
            "schemas/jsonschema/delivery-upload-batch-items-response.v1.schema.json",
        )?,
        schema_artifact::<RedeemDeliveryRequest>(
            "schemas/jsonschema/delivery-activation-redeem-request.v1.schema.json",
        )?,
        schema_artifact::<DeliveryActivationRedeemResponse>(
            "schemas/jsonschema/delivery-activation-redeem-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryActivationResponse>(
            "schemas/jsonschema/delivery-activation-response.v1.schema.json",
        )?,
        schema_artifact::<CreateDeliveryDownloadGrantRequest>(
            "schemas/jsonschema/delivery-download-grant-request.v1.schema.json",
        )?,
        schema_artifact::<DeliveryDownloadGrantIssueResponse>(
            "schemas/jsonschema/delivery-download-grant-issue-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryDownloadGrantResponse>(
            "schemas/jsonschema/delivery-download-grant-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryDownloadGrantRevokeRequest>(
            "schemas/jsonschema/delivery-download-grant-revoke-request.v1.schema.json",
        )?,
        schema_artifact::<ExtendDeliveryEntitlementRequest>(
            "schemas/jsonschema/delivery-entitlement-extend-request.v1.schema.json",
        )?,
        schema_artifact::<DeliveryServiceSegmentsResponse>(
            "schemas/jsonschema/delivery-service-segments-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryLifecycleEventsResponse>(
            "schemas/jsonschema/delivery-lifecycle-events-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryLifecycleResponse>(
            "schemas/jsonschema/delivery-lifecycle-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryOperationsOverviewResponse>(
            "schemas/jsonschema/delivery-operations-overview-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryOperationsTimelineResponse>(
            "schemas/jsonschema/delivery-operations-timeline-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryOperationsExceptionsResponse>(
            "schemas/jsonschema/delivery-operations-exceptions-response.v1.schema.json",
        )?,
        schema_artifact::<DeliveryOperationsDetailResponse>(
            "schemas/jsonschema/delivery-operations-detail-response.v1.schema.json",
        )?,
        schema_artifact::<CreateRenewalIntentRequest>(
            "schemas/jsonschema/renewal-intent-request.v1.schema.json",
        )?,
        schema_artifact::<RenewalIntentResponse>(
            "schemas/jsonschema/renewal-intent-response.v1.schema.json",
        )?,
        schema_artifact::<RenewalIntentsResponse>(
            "schemas/jsonschema/renewal-intents-response.v1.schema.json",
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
    let gateway_request = sample_gateway_chat_request();
    let gateway_response = sample_gateway_chat_response();
    let anthropic_request = sample_gateway_anthropic_messages_request();
    let anthropic_response = sample_gateway_anthropic_messages_response();
    let anthropic_error = sample_gateway_anthropic_messages_error();
    let gemini_request = sample_gateway_gemini_generate_content_request();
    let gemini_response = sample_gateway_gemini_generate_content_response();
    let gemini_error = sample_gateway_gemini_generate_content_error();
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
    let delivery_prepare_request = sample_create_delivery_request();
    let delivery_prepare_response = sample_delivery_prepare_response();
    let delivery_response = sample_delivery_response();
    let delivery_artifact_request = sample_create_delivery_artifact_request();
    let single_artifact_response = sample_delivery_artifact_response();
    let artifact_list_response = sample_delivery_artifacts_response();
    let delivery_upload_batch_request = sample_create_delivery_upload_batch_request();
    let delivery_upload_batch_response = sample_delivery_upload_batch_response();
    let delivery_upload_batch_items_response = sample_delivery_upload_batch_items_response();
    let delivery_activation_request = sample_redeem_delivery_request();
    let delivery_activation_redeem_response = sample_delivery_activation_redeem_response();
    let delivery_activation_response = sample_delivery_activation_response();
    let delivery_download_grant_request = sample_create_delivery_download_grant_request();
    let delivery_download_grant_issue_response = sample_delivery_download_grant_issue_response();
    let delivery_download_grant_response = sample_delivery_download_grant_response();
    let delivery_download_grant_revoke_request = sample_delivery_download_grant_revoke_request();
    let delivery_entitlement_extend_request = sample_extend_delivery_entitlement_request();
    let delivery_service_segments_response = sample_delivery_service_segments_response();
    let delivery_lifecycle_events_response = sample_delivery_lifecycle_events_response();
    let delivery_lifecycle_response = sample_delivery_lifecycle_response();
    let delivery_operations_overview_response = sample_delivery_operations_overview_response();
    let delivery_operations_timeline_response = sample_delivery_operations_timeline_response();
    let delivery_operations_exceptions_response = sample_delivery_operations_exceptions_response();
    let delivery_operations_detail_response = sample_delivery_operations_detail_response();
    let renewal_intent_request = sample_create_renewal_intent_request();
    let renewal_intent_response = sample_renewal_intent_response();
    let renewal_intents_list_response = RenewalIntentsResponse {
        data: vec![renewal_intent_response.data.clone()],
    };
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
            "schemas/examples/gateway/chat.request.json",
            &gateway_request,
        )?,
        example_artifact(
            "schemas/examples/gateway/chat.response.json",
            &gateway_response,
        )?,
        example_artifact(
            "schemas/examples/gateway/anthropic-messages.request.json",
            &anthropic_request,
        )?,
        example_artifact(
            "schemas/examples/gateway/anthropic-messages.response.json",
            &anthropic_response,
        )?,
        example_artifact(
            "schemas/examples/gateway/anthropic-messages.error.response.json",
            &anthropic_error,
        )?,
        example_artifact(
            "schemas/examples/gateway/gemini-generate-content.request.json",
            &gemini_request,
        )?,
        example_artifact(
            "schemas/examples/gateway/gemini-generate-content.response.json",
            &gemini_response,
        )?,
        example_artifact(
            "schemas/examples/gateway/gemini-generate-content.error.response.json",
            &gemini_error,
        )?,
        example_artifact(
            "schemas/examples/gateway/error.response.json",
            &error_envelope,
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
            "schemas/examples/control-plane/delivery-prepare.request.json",
            &delivery_prepare_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-prepare.response.json",
            &delivery_prepare_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery.response.json",
            &delivery_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-artifact.request.json",
            &delivery_artifact_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-artifact.response.json",
            &single_artifact_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-artifacts.response.json",
            &artifact_list_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-upload-batch.request.json",
            &delivery_upload_batch_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-upload-batch.response.json",
            &delivery_upload_batch_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-upload-batch-items.response.json",
            &delivery_upload_batch_items_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-activation-redeem.request.json",
            &delivery_activation_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-activation-redeem.response.json",
            &delivery_activation_redeem_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-activation.response.json",
            &delivery_activation_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-download-grant.request.json",
            &delivery_download_grant_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-download-grant-issue.response.json",
            &delivery_download_grant_issue_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-download-grant.response.json",
            &delivery_download_grant_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-download-grant-revoke.request.json",
            &delivery_download_grant_revoke_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-entitlement-extend.request.json",
            &delivery_entitlement_extend_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-service-segments.response.json",
            &delivery_service_segments_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-lifecycle-events.response.json",
            &delivery_lifecycle_events_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-lifecycle.response.json",
            &delivery_lifecycle_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-operations-overview.response.json",
            &delivery_operations_overview_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-operations-timeline.response.json",
            &delivery_operations_timeline_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-operations-exceptions.response.json",
            &delivery_operations_exceptions_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/delivery-operations-detail.response.json",
            &delivery_operations_detail_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/renewal-intent.request.json",
            &renewal_intent_request,
        )?,
        example_artifact(
            "schemas/examples/control-plane/renewal-intent.response.json",
            &renewal_intent_response,
        )?,
        example_artifact(
            "schemas/examples/control-plane/renewal-intents.response.json",
            &renewal_intents_list_response,
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
  {{ id: 'listOpeningGrants', method: 'GET', path: '/v1/opening-grants' }},\n\
  {{ id: 'createOpeningGrant', method: 'POST', path: '/v1/opening-grants' }},\n\
  {{ id: 'revokeOpeningGrant', method: 'POST', path: '/v1/opening-grants/{{grant_id}}/revoke' }},\n\
  {{ id: 'prepareDelivery', method: 'POST', path: '/v1/deliveries/prepare' }},\n\
  {{ id: 'getDelivery', method: 'GET', path: '/v1/deliveries/{{delivery_id}}' }},\n\
  {{ id: 'revokeDelivery', method: 'POST', path: '/v1/deliveries/{{delivery_id}}/revoke' }},\n\
  {{ id: 'createDeliveryArtifact', method: 'POST', path: '/v1/deliveries/{{delivery_id}}/artifacts' }},\n\
  {{ id: 'listDeliveryArtifacts', method: 'GET', path: '/v1/deliveries/{{delivery_id}}/artifacts' }},\n\
  {{ id: 'getDeliveryArtifact', method: 'GET', path: '/v1/deliveries/{{delivery_id}}/artifacts/{{artifact_id}}' }},\n\
  {{ id: 'createDeliveryUploadBatch', method: 'POST', path: '/v1/delivery-uploads' }},\n\
  {{ id: 'getDeliveryUploadBatch', method: 'GET', path: '/v1/delivery-uploads/{{batch_id}}' }},\n\
  {{ id: 'listDeliveryUploadBatchItems', method: 'GET', path: '/v1/delivery-uploads/{{batch_id}}/items' }},\n\
  {{ id: 'redeemDeliveryActivation', method: 'POST', path: '/v1/delivery-activations/redeem' }},\n\
  {{ id: 'getDeliveryActivation', method: 'GET', path: '/v1/delivery-activations/{{activation_id}}' }},\n\
  {{ id: 'issueDeliveryDownloadGrant', method: 'POST', path: '/v1/delivery-download-grants' }},\n\
  {{ id: 'getDeliveryDownloadGrant', method: 'GET', path: '/v1/delivery-download-grants/{{grant_id}}' }},\n\
  {{ id: 'revokeDeliveryDownloadGrant', method: 'POST', path: '/v1/delivery-download-grants/{{grant_id}}/revoke' }},\n\
  {{ id: 'retrieveDeliveryDownloadArtifact', method: 'GET', path: '/v1/delivery-downloads/artifact' }},\n\
  {{ id: 'listDeliveryServiceSegments', method: 'GET', path: '/v1/delivery-entitlements/{{entitlement_id}}/segments' }},\n\
  {{ id: 'listDeliveryLifecycleEvents', method: 'GET', path: '/v1/delivery-entitlements/{{entitlement_id}}/lifecycle-events' }},\n\
  {{ id: 'reconcileDeliveryLifecycle', method: 'POST', path: '/v1/delivery-entitlements/{{entitlement_id}}/reconcile' }},\n\
  {{ id: 'extendDeliveryEntitlement', method: 'POST', path: '/v1/delivery-entitlements/{{entitlement_id}}/extend' }},\n\
  {{ id: 'getDeliveryOperationsOverview', method: 'GET', path: '/v1/delivery-operations/overview' }},\n\
  {{ id: 'getDeliveryOperationsTimeline', method: 'GET', path: '/v1/delivery-operations/timeline' }},\n\
  {{ id: 'listDeliveryOperationsExceptions', method: 'GET', path: '/v1/delivery-operations/exceptions' }},\n\
  {{ id: 'getDeliveryOperationsDetail', method: 'GET', path: '/v1/delivery-operations/detail' }},\n\
  {{ id: 'createRenewalIntent', method: 'POST', path: '/v1/billing/renewal-intents' }},\n\
  {{ id: 'listRenewalIntents', method: 'GET', path: '/v1/billing/renewal-intents' }},\n\
  {{ id: 'simulateRoute', method: 'POST', path: '/v1/route-simulations' }},\n\
  {{ id: 'listRouteReceipts', method: 'GET', path: '/v1/route-receipts' }},\n\
  {{ id: 'getRouteReceipt', method: 'GET', path: '/v1/route-receipts/{{route_receipt_id}}' }},\n\
  {{ id: 'getRouteDiagnostics', method: 'GET', path: '/v1/route-diagnostics/{{route_policy_id}}' }},\n\
  {{ id: 'getRouteReceiptDiagnostics', method: 'GET', path: '/v1/route-receipts/{{route_receipt_id}}/diagnostics' }},\n\
] as const;\n\
export const GATEWAY_OPERATIONS = [\n\
  {{ id: 'createChatCompletion', method: 'POST', path: '/v1/chat/completions' }},\n\
  {{ id: 'createAnthropicMessages', method: 'POST', path: '/v1/messages' }},\n\
  {{ id: 'createGeminiGenerateContent', method: 'POST', path: '/v1beta/models/{{model}}:generateContent' }},\n\
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
        "gateway_anthropic_messages".to_string(),
        "gateway_gemini_generate_content".to_string(),
        "route_receipt_diagnostics".to_string(),
        "usage_summary".to_string(),
        "usage_breakdown".to_string(),
        "balance_projection".to_string(),
        "pricing_catalog".to_string(),
        "pricing_simulation".to_string(),
        "billing_export_job".to_string(),
        "billing_export_jobs".to_string(),
        "delivery_prepare".to_string(),
        "delivery_projection".to_string(),
        "delivery_artifact".to_string(),
        "delivery_upload".to_string(),
        "delivery_activation".to_string(),
        "delivery_download_grant".to_string(),
        "delivery_lifecycle".to_string(),
        "delivery_operations".to_string(),
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
