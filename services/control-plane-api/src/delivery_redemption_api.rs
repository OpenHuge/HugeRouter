use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use core_domain::{ProjectId, TenantId};
use protocol_ir::{
    CreateDeliveryRequest, DeliveryRedemptionInventoryResponse,
    PrepareDeliveryRedemptionUnitsRequest, PrepareDeliveryRedemptionUnitsResponse,
};
use serde::Deserialize;

use crate::delivery_redemption_policy::{
    BROWSER_FILE_UNLOCK_CODE_KIND, DEFAULT_REDEMPTION_UNIT_COUNT, REDEMPTION_CODE_KIND,
};
use crate::opening_grant_api::validate_opening_owner_account_id;
use crate::store::{
    DeliveryPrepareDraft, DeliveryRedemptionUnitDraft, DeliveryRedemptionUnitsPrepareResult,
    build_prepared_delivery_records,
};
use crate::{
    ApiError, ControlPlaneState, authorize_delivery_producer_request, authorize_v1_request,
    delivery_service_kind, delivery_timestamps, generate_delivery_code, generate_stable_id,
    load_project, next_request_context, validate_delivery_provider, validate_delivery_service_days,
};

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Deserialize)]
pub struct DeliveryRedemptionInventoryQuery {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    #[serde(default)]
    pub owner_account_id: Option<String>,
}

pub async fn prepare_delivery_redemption_units(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<PrepareDeliveryRedemptionUnitsRequest>,
) -> Result<Json<PrepareDeliveryRedemptionUnitsResponse>, ApiError> {
    let context = next_request_context();
    if let Ok(producer) = authorize_delivery_producer_request(&state, &headers, &context).await {
        let owner_account_id = validate_opening_owner_account_id(&request.owner_account_id, &context)?;
        let provider = validate_delivery_provider(&request.provider, &context)?;
        let service_kind = delivery_service_kind(request.service_kind.as_deref(), &context)?;
        validate_delivery_service_days(request.service_days, &context)?;
        producer.ensure_scope(
            request.tenant_id.as_str(),
            request.project_id.as_str(),
            &context,
        )?;
        producer.ensure_owner_account(&owner_account_id, &context)?;
        producer.ensure_provider(&provider, &context)?;
        producer.ensure_service_defaults(&service_kind, request.service_days, &context)?;
        return prepare_delivery_redemption_units_with_actor(
            &state,
            request,
            producer.actor_id(),
            &context,
        )
        .await;
    }
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    authz.ensure_manage_tenant(request.tenant_id.as_str(), &context)?;
    prepare_delivery_redemption_units_with_actor(&state, request, authz.actor_id(), &context).await
}

pub async fn prepare_delivery_redemption_units_internal(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<PrepareDeliveryRedemptionUnitsRequest>,
) -> Result<Json<PrepareDeliveryRedemptionUnitsResponse>, ApiError> {
    let context = next_request_context();
    let producer = authorize_delivery_producer_request(&state, &headers, &context).await?;
    let owner_account_id = validate_opening_owner_account_id(&request.owner_account_id, &context)?;
    let provider = validate_delivery_provider(&request.provider, &context)?;
    let service_kind = delivery_service_kind(request.service_kind.as_deref(), &context)?;
    validate_delivery_service_days(request.service_days, &context)?;
    producer.ensure_scope(
        request.tenant_id.as_str(),
        request.project_id.as_str(),
        &context,
    )?;
    producer.ensure_owner_account(&owner_account_id, &context)?;
    producer.ensure_provider(&provider, &context)?;
    producer.ensure_service_defaults(&service_kind, request.service_days, &context)?;
    prepare_delivery_redemption_units_with_actor(&state, request, producer.actor_id(), &context)
        .await
}

pub async fn list_delivery_redemption_inventory(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(query): Query<DeliveryRedemptionInventoryQuery>,
) -> Result<Json<DeliveryRedemptionInventoryResponse>, ApiError> {
    let context = next_request_context();
    let owner_account_id = query
        .owner_account_id
        .as_deref()
        .map(|value| validate_opening_owner_account_id(value, &context))
        .transpose()?;
    let project = load_project(&state, query.project_id.as_str(), &context).await?;
    if project.tenant_id != query.tenant_id {
        return Err(ApiError::bad_request(
            "delivery_project_tenant_mismatch",
            "project_id does not belong to tenant_id".to_string(),
            &context,
        ));
    }
    if let Ok(producer) = authorize_delivery_producer_request(&state, &headers, &context).await {
        producer.ensure_scope(
            query.tenant_id.as_str(),
            query.project_id.as_str(),
            &context,
        )?;
        if let Some(owner_account_id) = owner_account_id.as_deref() {
            producer.ensure_owner_account(owner_account_id, &context)?;
        }
        return state
            .store
            .list_delivery_redemption_inventory(
                &query.tenant_id,
                &query.project_id,
                owner_account_id.as_deref(),
            )
            .await
            .map(Json)
            .map_err(|error| {
                ApiError::internal(
                    "delivery_redemption_inventory_unavailable",
                    format!("failed to load delivery redemption inventory: {error}"),
                    &context,
                )
            });
    }
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    authz.ensure_read_tenant(query.tenant_id.as_str(), &context)?;
    state
        .store
        .list_delivery_redemption_inventory(
            &query.tenant_id,
            &query.project_id,
            owner_account_id.as_deref(),
        )
        .await
        .map(Json)
        .map_err(|error| {
            ApiError::internal(
                "delivery_redemption_inventory_unavailable",
                format!("failed to load delivery redemption inventory: {error}"),
                &context,
            )
        })
}

async fn prepare_delivery_redemption_units_with_actor(
    state: &ControlPlaneState,
    request: PrepareDeliveryRedemptionUnitsRequest,
    actor_id: String,
    context: &crate::RequestContext,
) -> Result<Json<PrepareDeliveryRedemptionUnitsResponse>, ApiError> {
    let owner_account_id = validate_opening_owner_account_id(&request.owner_account_id, context)?;
    let provider = validate_delivery_provider(&request.provider, context)?;
    validate_delivery_service_days(request.service_days, context)?;
    let service_kind = delivery_service_kind(request.service_kind.as_deref(), context)?;
    let customer_label = request
        .customer_label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let project = load_project(state, request.project_id.as_str(), context).await?;
    if project.tenant_id != request.tenant_id {
        return Err(ApiError::bad_request(
            "delivery_project_tenant_mismatch",
            "project_id does not belong to tenant_id".to_string(),
            context,
        ));
    }
    let timestamp_request = CreateDeliveryRequest {
        tenant_id: request.tenant_id.clone(),
        project_id: request.project_id.clone(),
        provider: request.provider.clone(),
        owner_account_id: Some(owner_account_id.clone()),
        customer_label: customer_label.clone(),
        service_kind: request.service_kind.clone(),
        service_days: request.service_days,
        starts_at: request.starts_at.clone(),
        code_expires_at: request.code_expires_at.clone(),
    };
    let (starts_at, ends_at, code_expires_at) = delivery_timestamps(&timestamp_request, context)?;
    let batch_id = generate_stable_id("dlvbatch", context)?;
    let mut units = Vec::with_capacity(DEFAULT_REDEMPTION_UNIT_COUNT);
    for _index in 0..DEFAULT_REDEMPTION_UNIT_COUNT {
        let redemption_code = generate_delivery_code(REDEMPTION_CODE_KIND, context)?;
        let browser_file_unlock_code =
            generate_delivery_code(BROWSER_FILE_UNLOCK_CODE_KIND, context)?;
        let records = build_prepared_delivery_records(
            DeliveryPrepareDraft {
                delivery_id: generate_stable_id("delivery", context)?,
                tenant_id: request.tenant_id.clone(),
                project_id: request.project_id.clone(),
                owner_account_id: owner_account_id.clone(),
                redemption_batch_id: Some(batch_id.clone()),
                provider: provider.clone(),
                operator_id: actor_id.clone(),
                customer_label: customer_label.clone(),
                service_kind: service_kind.clone(),
                service_days: request.service_days,
                starts_at: starts_at.clone(),
                ends_at: ends_at.clone(),
                code_expires_at: code_expires_at.clone(),
                enforce_owner_redemption_capacity: false,
            },
            &redemption_code,
            &browser_file_unlock_code,
        );
        units.push(DeliveryRedemptionUnitDraft {
            records,
            redemption_code,
            browser_file_unlock_code,
        });
    }
    match state
        .store
        .prepare_delivery_redemption_units(batch_id, owner_account_id, units)
        .await
        .map_err(|error| {
            ApiError::internal(
                "delivery_redemption_units_prepare_failed",
                format!("failed to prepare delivery redemption units: {error}"),
                context,
            )
        })? {
        DeliveryRedemptionUnitsPrepareResult::Prepared(response) => Ok(Json(*response)),
    }
}
