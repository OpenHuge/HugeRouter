#![allow(clippy::wildcard_imports)]

use super::*;

#[utoipa::path(
    post,
    path = "/v1/deliveries/redemption-units/prepare",
    tag = "deliveries",
    request_body = PrepareDeliveryRedemptionUnitsRequest,
    responses(
        (status = 200, description = "Prepare eight owner-scoped v2 delivery redemption units and return plaintext once", body = PrepareDeliveryRedemptionUnitsResponse),
        (status = 400, description = "Normalized error", body = ErrorEnvelope),
        (status = 403, description = "Normalized error", body = ErrorEnvelope),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
        (status = 409, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
pub const fn prepare_delivery_redemption_units() {}

#[utoipa::path(
    get,
    path = "/v1/delivery-redemption-inventory",
    tag = "deliveries",
    params(
        ("tenant_id" = TenantId, Query, description = "Tenant id"),
        ("project_id" = ProjectId, Query, description = "Project id"),
        ("owner_account_id" = Option<String>, Query, description = "Owner account id")
    ),
    responses(
        (status = 200, description = "List owner-scoped delivery redemption code inventory without plaintext secrets", body = DeliveryRedemptionInventoryResponse),
        (status = 400, description = "Normalized error", body = ErrorEnvelope),
        (status = 403, description = "Normalized error", body = ErrorEnvelope),
        (status = 404, description = "Normalized error", body = ErrorEnvelope),
    )
)]
#[allow(dead_code)]
pub const fn list_delivery_redemption_inventory() {}
