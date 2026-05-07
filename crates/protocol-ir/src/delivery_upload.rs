use core_domain::{ProjectId, TenantId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption_protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_kind: Option<String>,
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
    pub encryption_protocol: String,
    pub encryption_version: String,
    pub secret_kind: String,
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
