use core_domain::{ProjectId, TenantId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{DeliveryOneTimeCodes, DeliveryProjection};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PrepareDeliveryRedemptionUnitsRequest {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub owner_account_id: String,
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
pub struct DeliveryRedemptionUnit {
    pub unit_id: String,
    pub index: u32,
    pub delivery: DeliveryProjection,
    pub one_time_codes: DeliveryOneTimeCodes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryRedemptionUnitsBatch {
    pub batch_id: String,
    pub owner_account_id: String,
    pub unit_count: u32,
    pub capacity: u32,
    pub units: Vec<DeliveryRedemptionUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PrepareDeliveryRedemptionUnitsResponse {
    pub data: DeliveryRedemptionUnitsBatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryRedemptionInventoryOwner {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub owner_account_id: String,
    pub active_count: u32,
    pub capacity: u32,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryRedemptionInventoryUnit {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub owner_account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redemption_batch_id: Option<String>,
    pub delivery_id: String,
    pub code_id: String,
    pub code_prefix: String,
    pub code_last_four: String,
    pub status: String,
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryRedemptionInventory {
    pub owners: Vec<DeliveryRedemptionInventoryOwner>,
    pub units: Vec<DeliveryRedemptionInventoryUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DeliveryRedemptionInventoryResponse {
    pub data: DeliveryRedemptionInventory,
}
