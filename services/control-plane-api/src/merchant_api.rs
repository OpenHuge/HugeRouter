use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use core_domain::{
    CardDeliveryKind, CardProduct, CardProductId, CardProductStatus, MerchantFulfillmentMode,
    MerchantShop, MerchantShopId, MerchantShopStatus, RelayEvaluation, TenantId, TrialConnection,
    TrialConnectionId, TrialConnectionStatus,
};
use serde::{Deserialize, Serialize};

use crate::store::{MerchantWorkspaceEnvelope, ReplayCapsuleResponse, now_rfc3339};
use crate::{
    ApiError, ControlPlaneAuthorizer, ControlPlaneState, RequestContext, authorize_v1_request,
    bad_request_error, internal_error, merchant_mutation_error, next_request_context,
    not_found_error,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMerchantShopRequest {
    pub merchant_shop_id: String,
    pub slug: String,
    pub display_name: String,
    pub announcement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCardProductRequest {
    pub card_product_id: String,
    pub merchant_shop_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub description: String,
    pub inventory_count: u32,
    pub face_value_usd: String,
    pub retail_price_usd: String,
    pub retail_price_cny_total: Option<u32>,
    pub supports_trial: bool,
    #[serde(default)]
    pub sale_enabled: bool,
    #[serde(default)]
    pub delivery_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTrialConnectionRequest {
    pub trial_connection_id: String,
    pub provider_label: String,
    pub endpoint_base_url: String,
    pub api_key: String,
    pub target_model: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelayEvaluationRequest {
    pub trial_connection_id: String,
}

pub async fn get_merchant_workspace(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<MerchantWorkspaceEnvelope>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = authz
        .active_tenant_id()
        .ok_or_else(|| {
            bad_request_error(
                "tenant_required",
                "an active tenant is required for merchant operations",
                &context,
            )
        })
        .and_then(|tenant_id| {
            authz.ensure_read_tenant(tenant_id, &context)?;
            TenantId::parse(tenant_id.to_string()).map_err(|error| {
                bad_request_error(
                    "tenant_required",
                    format!("invalid active tenant id: {error}"),
                    &context,
                )
            })
        })?;

    let response = state
        .store
        .get_merchant_workspace(&tenant_id)
        .await
        .map_err(|error| {
            internal_error(
                "merchant_workspace_load_failed",
                format!("failed to load merchant workspace: {error}"),
                &context,
            )
        })?;

    Ok(Json(response))
}

pub async fn create_merchant_shop(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CreateMerchantShopRequest>,
) -> Result<Json<MerchantShop>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = resolve_active_tenant(&authz, &context)?;
    let shop_id = MerchantShopId::parse(request.merchant_shop_id.clone()).map_err(|error| {
        bad_request_error(
            "validation_failed",
            format!("invalid merchant_shop_id: {error}"),
            &context,
        )
    })?;

    let shop = MerchantShop {
        merchant_shop_id: shop_id,
        tenant_id,
        slug: request.slug,
        display_name: request.display_name,
        status: MerchantShopStatus::Active,
        announcement: request.announcement,
        fulfillment_mode: MerchantFulfillmentMode::AutoCardSecret,
        version: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };

    let created = state
        .store
        .create_merchant_shop(shop)
        .await
        .map_err(|error| merchant_mutation_error(&error, &context))?;

    Ok(Json(created))
}

pub async fn create_card_product(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CreateCardProductRequest>,
) -> Result<Json<CardProduct>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = resolve_active_tenant(&authz, &context)?;
    let card_product_id =
        CardProductId::parse(request.card_product_id.clone()).map_err(|error| {
            bad_request_error(
                "validation_failed",
                format!("invalid card_product_id: {error}"),
                &context,
            )
        })?;
    let merchant_shop_id =
        MerchantShopId::parse(request.merchant_shop_id.clone()).map_err(|error| {
            bad_request_error(
                "validation_failed",
                format!("invalid merchant_shop_id: {error}"),
                &context,
            )
        })?;

    let project_id = request
        .project_id
        .as_deref()
        .map(core_domain::ProjectId::parse)
        .transpose()
        .map_err(|error| {
            bad_request_error(
                "validation_failed",
                format!("invalid project_id: {error}"),
                &context,
            )
        })?;

    let product = CardProduct {
        card_product_id,
        tenant_id,
        merchant_shop_id,
        project_id,
        title: request.title,
        description: request.description,
        status: CardProductStatus::Active,
        inventory_count: request.inventory_count,
        face_value_usd: request.face_value_usd,
        retail_price_usd: request.retail_price_usd,
        retail_price_cny_total: request.retail_price_cny_total,
        delivery_kind: CardDeliveryKind::DirectSecret,
        supports_trial: request.supports_trial,
        sale_enabled: request.sale_enabled,
        version: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };

    let created = state
        .store
        .create_card_product(product)
        .await
        .map_err(|error| merchant_mutation_error(&error, &context))?;
    if !request.delivery_ids.is_empty() {
        state
            .store
            .bind_card_product_deliveries(created.card_product_id.as_str(), request.delivery_ids)
            .await
            .map_err(|error| merchant_mutation_error(&error, &context))?;
    }

    Ok(Json(created))
}

pub async fn create_trial_connection(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CreateTrialConnectionRequest>,
) -> Result<Json<TrialConnection>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = resolve_active_tenant(&authz, &context)?;
    let trial_connection_id = TrialConnectionId::parse(request.trial_connection_id.clone())
        .map_err(|error| {
            bad_request_error(
                "validation_failed",
                format!("invalid trial_connection_id: {error}"),
                &context,
            )
        })?;

    let connection = TrialConnection {
        trial_connection_id,
        tenant_id,
        provider_label: request.provider_label,
        endpoint_base_url: request.endpoint_base_url,
        api_key_masked: mask_trial_api_key(&request.api_key),
        target_model: request.target_model,
        status: TrialConnectionStatus::Active,
        notes: request.notes,
        last_verified_at: None,
        version: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };

    let created = state
        .store
        .create_trial_connection(connection)
        .await
        .map_err(|error| merchant_mutation_error(&error, &context))?;

    Ok(Json(created))
}

pub async fn create_relay_evaluation(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CreateRelayEvaluationRequest>,
) -> Result<Json<RelayEvaluation>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = resolve_active_tenant(&authz, &context)?;

    let created = state
        .store
        .create_relay_evaluation(&tenant_id, &request.trial_connection_id)
        .await
        .map_err(|error| merchant_mutation_error(&error, &context))?;

    Ok(Json(created))
}

pub async fn get_replay_capsule(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(replay_capsule_id): Path<String>,
) -> Result<Json<ReplayCapsuleResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let tenant_id = authz.active_tenant_id().ok_or_else(|| {
        bad_request_error(
            "tenant_required",
            "an active tenant is required for replay access",
            &context,
        )
    })?;
    authz.ensure_read_tenant(tenant_id, &context)?;
    let tenant_id = TenantId::parse(tenant_id.to_string()).map_err(|error| {
        bad_request_error(
            "tenant_required",
            format!("invalid active tenant id: {error}"),
            &context,
        )
    })?;

    let response = state
        .store
        .get_replay_capsule(&tenant_id, &replay_capsule_id)
        .await
        .map_err(|error| {
            internal_error(
                "replay_capsule_load_failed",
                format!("failed to load replay capsule: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            not_found_error(
                "not_found",
                format!("replay capsule {replay_capsule_id} was not found"),
                &context,
            )
        })?;

    Ok(Json(response))
}

fn resolve_active_tenant(
    authz: &ControlPlaneAuthorizer,
    context: &RequestContext,
) -> Result<TenantId, ApiError> {
    let tenant_id = authz.active_tenant_id().ok_or_else(|| {
        bad_request_error(
            "tenant_required",
            "an active tenant is required for merchant operations",
            context,
        )
    })?;

    authz.ensure_manage_tenant(tenant_id, context)?;

    TenantId::parse(tenant_id.to_string()).map_err(|error| {
        bad_request_error(
            "tenant_required",
            format!("invalid active tenant id: {error}"),
            context,
        )
    })
}

fn mask_trial_api_key(api_key: &str) -> String {
    let trimmed = api_key.trim();
    if trimmed.len() <= 10 {
        return format!("{trimmed}...");
    }

    let prefix = &trimmed[..7];
    let suffix = &trimmed[trimmed.len() - 4..];
    format!("{prefix}...{suffix}")
}
