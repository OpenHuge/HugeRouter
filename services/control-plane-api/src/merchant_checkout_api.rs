use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD};
use ring::rand;
use serde::Deserialize;

use crate::store::{
    self, MerchantProductFulfillmentDraft, MerchantProductFulfillmentResult,
    MerchantProductOrderCreateResult, MerchantProductOrderDraft, MerchantProductPrepayDraft,
    WechatPaymentOrderRecord, expires_at, now_rfc3339,
};
use crate::wechat_pay::{
    WechatPayClient, WechatPayPrepayRequest, WechatPayPrepayResponse, new_out_trade_no,
};
use crate::wechat_pay_api::wechat_pay_channel_slug;
use crate::{
    ApiError, AuthProvider, ControlPlaneAuthorizer, ControlPlaneState, RequestContext,
    authorize_v1_request, download_grant_issue_result_to_response,
    generate_delivery_download_token, generate_stable_id, next_request_context,
    redeem_result_to_response, sha256_hex,
};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMerchantProductOrderRequest {
    pub card_product_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MerchantProductPrepayRequest {
    pub channel: crate::wechat_pay::WechatPayChannel,
}

pub async fn get_public_shop(
    State(state): State<ControlPlaneState>,
    Path(slug): Path<String>,
) -> Result<Json<store::MerchantPublicShopResponse>, ApiError> {
    let context = next_request_context();
    state
        .store
        .get_public_shop_by_slug(&slug)
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_shop_unavailable",
                format!("failed to load public shop: {error}"),
                &context,
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            ApiError::not_found(
                "merchant_shop_not_found",
                format!("shop `{slug}` was not found"),
                &context,
            )
        })
}

pub async fn create_merchant_product_order(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<CreateMerchantProductOrderRequest>,
) -> Result<Json<store::MerchantProductOrderResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    ensure_wechat_buyer(&authz, &context)?;
    let card_product_id = request.card_product_id.trim();
    if card_product_id.is_empty() {
        return Err(ApiError::bad_request(
            "card_product_id_required",
            "card_product_id is required".to_string(),
            &context,
        ));
    }
    match state
        .store
        .create_merchant_product_order(MerchantProductOrderDraft {
            order_id: generate_stable_id("morder", &context)?,
            card_product_id: card_product_id.to_string(),
            buyer_user_id: authz.user_id().clone(),
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_create_failed",
                format!("failed to create merchant product order: {error}"),
                &context,
            )
        })? {
        MerchantProductOrderCreateResult::Created(response) => Ok(Json(*response)),
        MerchantProductOrderCreateResult::ProductNotFound => Err(ApiError::not_found(
            "card_product_not_found",
            format!("card product `{card_product_id}` was not found"),
            &context,
        )),
        MerchantProductOrderCreateResult::ProductNotSaleable => Err(ApiError::conflict(
            "card_product_not_saleable",
            "card product is not available for WeChat Pay checkout".to_string(),
            &context,
        )),
        MerchantProductOrderCreateResult::InventoryUnavailable => Err(ApiError::conflict(
            "card_product_inventory_unavailable",
            "card product has no available delivery inventory".to_string(),
            &context,
        )),
    }
}

pub async fn get_merchant_product_order(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<store::MerchantProductOrderResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let response = load_merchant_order_for_buyer(&state, &authz, &order_id, &context).await?;
    Ok(Json(response))
}

pub async fn create_merchant_product_order_wechat_prepay(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(request): Json<MerchantProductPrepayRequest>,
) -> Result<Json<WechatPayPrepayResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    ensure_wechat_buyer(&authz, &context)?;
    let order = load_merchant_order_for_buyer(&state, &authz, &order_id, &context).await?;
    if order.data.status == store::MERCHANT_ORDER_STATUS_FULFILLED {
        return Err(ApiError::conflict(
            "merchant_order_already_fulfilled",
            "merchant product order has already been fulfilled".to_string(),
            &context,
        ));
    }
    let payer_openid = if matches!(request.channel, crate::wechat_pay::WechatPayChannel::Jsapi) {
        Some(
            state
                .store
                .get_wechat_user_openid(authz.user_id())
                .await
                .map_err(|error| {
                    ApiError::internal(
                        "wechat_openid_unavailable",
                        format!("failed to load WeChat openid: {error}"),
                        &context,
                    )
                })?
                .ok_or_else(|| {
                    ApiError::conflict(
                        "wechat_openid_required",
                        "JSAPI checkout requires a WeChat login with openid; sign in with WeChat again".to_string(),
                        &context,
                    )
                })?,
        )
    } else {
        None
    };
    let client = WechatPayClient::from_env().map_err(|error| {
        ApiError::internal(
            "wechat_pay_not_configured",
            format!("WeChat Pay is not configured: {error}"),
            &context,
        )
    })?;
    let out_trade_no = new_out_trade_no(
        order.data.tenant_id.as_str(),
        Some(order.data.project_id.as_str()),
    );
    let now = now_rfc3339();
    let mut payment_order = WechatPaymentOrderRecord {
        out_trade_no: out_trade_no.clone(),
        tenant_id: order.data.tenant_id.as_str().to_string(),
        project_id: Some(order.data.project_id.as_str().to_string()),
        amount_total: order.data.amount_total,
        currency: "CNY".to_string(),
        channel: wechat_pay_channel_slug(&request.channel).to_string(),
        status: "creating".to_string(),
        trade_state: None,
        code_url: None,
        prepay_id: None,
        transaction_id: None,
        notification_id: None,
        created_at: now.clone(),
        updated_at: now,
        expires_at: expires_at(30 * 60),
        paid_at: None,
        metadata: serde_json::json!({
            "merchant_product_order_id": order.data.order_id,
            "card_product_id": order.data.card_product_id,
            "description": "Card product checkout",
        }),
    };
    state
        .store
        .create_wechat_payment_order(payment_order.clone())
        .await
        .map_err(|error| {
            ApiError::internal(
                "wechat_pay_order_persist_failed",
                format!("failed to persist WeChat Pay order: {error}"),
                &context,
            )
        })?;
    let prepay_request = WechatPayPrepayRequest {
        tenant_id: order.data.tenant_id.as_str().to_string(),
        project_id: Some(order.data.project_id.as_str().to_string()),
        amount_total: order.data.amount_total,
        currency: "CNY".to_string(),
        description: format!("Card product {}", order.data.card_product_id.as_str()),
        channel: request.channel,
        payer_openid,
        attach: Some(order.data.order_id.clone()),
    };
    let response = match client.create_prepay(prepay_request, &out_trade_no).await {
        Ok(response) => response,
        Err(error) => {
            payment_order.status = "failed".to_string();
            payment_order.updated_at = now_rfc3339();
            let _ = state.store.update_wechat_payment_order(payment_order).await;
            return Err(ApiError::bad_request(
                "wechat_pay_prepay_failed",
                format!("WeChat Pay prepay request failed: {error}"),
                &context,
            ));
        }
    };
    payment_order.status = "pending".to_string();
    payment_order.code_url.clone_from(&response.code_url);
    payment_order.prepay_id.clone_from(&response.prepay_id);
    payment_order.updated_at = now_rfc3339();
    state
        .store
        .update_wechat_payment_order(payment_order)
        .await
        .map_err(|error| {
            ApiError::internal(
                "wechat_pay_order_persist_failed",
                format!("failed to update WeChat Pay order: {error}"),
                &context,
            )
        })?;
    state
        .store
        .attach_merchant_order_prepay(MerchantProductPrepayDraft {
            order_id: order.data.order_id,
            out_trade_no,
            channel: wechat_pay_channel_slug(&response.channel).to_string(),
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_prepay_attach_failed",
                format!("failed to attach WeChat prepay to merchant order: {error}"),
                &context,
            )
        })?;
    Ok(Json(response))
}

pub async fn get_merchant_pickup(
    State(state): State<ControlPlaneState>,
    Path(pickup_token): Path<String>,
) -> Result<Json<store::MerchantPickupResponse>, ApiError> {
    let context = next_request_context();
    let pickup_token = pickup_token.trim();
    if pickup_token.is_empty() {
        return Err(ApiError::bad_request(
            "pickup_token_required",
            "pickup_token is required".to_string(),
            &context,
        ));
    }
    state
        .store
        .get_pickup_by_token_hash(&sha256_hex(pickup_token))
        .await
        .map_err(|error| {
            ApiError::internal(
                "pickup_unavailable",
                format!("failed to load pickup: {error}"),
                &context,
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            ApiError::not_found(
                "pickup_not_found",
                "pickup token was not found or the order is not fulfilled".to_string(),
                &context,
            )
        })
}

pub async fn settle_merchant_product_order_for_payment(
    state: &ControlPlaneState,
    out_trade_no: &str,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let Some(order) = state
        .store
        .get_merchant_product_order_by_out_trade_no(out_trade_no)
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_unavailable",
                format!("failed to load merchant product order for payment: {error}"),
                context,
            )
        })?
    else {
        return Ok(());
    };
    match state
        .store
        .fulfill_merchant_product_order(MerchantProductFulfillmentDraft {
            order_id: order.data.order_id,
            activation_id: generate_stable_id("activation", context)?,
            download_grant_id: generate_stable_id("dlgrant", context)?,
            download_token: generate_delivery_download_token(context)?,
            pickup_token: generate_pickup_token(context)?,
            fulfilled_by: "merchant_product_wechat_pay".to_string(),
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_fulfillment_failed",
                format!("failed to fulfill merchant product order: {error}"),
                context,
            )
        })? {
        MerchantProductFulfillmentResult::Fulfilled(response)
        | MerchantProductFulfillmentResult::AlreadyFulfilled(response) => {
            tracing::debug!(
                merchant_order_id = response.data.order_id,
                "settled merchant product order after WeChat Pay payment"
            );
            Ok(())
        }
        MerchantProductFulfillmentResult::OrderNotFound => Err(ApiError::not_found(
            "merchant_order_not_found",
            "merchant product order was not found during settlement".to_string(),
            context,
        )),
        MerchantProductFulfillmentResult::InventoryNotFound => Err(ApiError::conflict(
            "merchant_inventory_not_found",
            "merchant product inventory was not found during settlement".to_string(),
            context,
        )),
        MerchantProductFulfillmentResult::DeliveryActivation(result) => {
            redeem_result_to_response(result, context).map(|_| ())
        }
        MerchantProductFulfillmentResult::DownloadGrant(result) => {
            download_grant_issue_result_to_response(result, context).map(|_| ())
        }
    }
}

fn ensure_wechat_buyer(
    authz: &ControlPlaneAuthorizer,
    context: &RequestContext,
) -> Result<(), ApiError> {
    if authz.authenticated_by() == AuthProvider::Wechat {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "wechat_login_required",
            "merchant product checkout requires WeChat login".to_string(),
            context,
        ))
    }
}

async fn load_merchant_order_for_buyer(
    state: &ControlPlaneState,
    authz: &ControlPlaneAuthorizer,
    order_id: &str,
    context: &RequestContext,
) -> Result<store::MerchantProductOrderResponse, ApiError> {
    let response = state
        .store
        .get_merchant_product_order(order_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_unavailable",
                format!("failed to load merchant product order: {error}"),
                context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "merchant_order_not_found",
                format!("merchant product order `{order_id}` was not found"),
                context,
            )
        })?;
    if response.data.buyer_user_id != *authz.user_id() {
        return Err(ApiError::forbidden(
            "merchant_order_forbidden",
            "merchant product order belongs to another buyer".to_string(),
            context,
        ));
    }
    Ok(response)
}

fn generate_pickup_token(context: &RequestContext) -> Result<String, ApiError> {
    let rng = rand::SystemRandom::new();
    let mut token_bytes = [0_u8; 32];
    rand::SecureRandom::fill(&rng, &mut token_bytes).map_err(|_| {
        ApiError::internal(
            "pickup_token_generation_failed",
            "failed to generate merchant pickup token".to_string(),
            context,
        )
    })?;
    Ok(format!(
        "pickup_{}",
        BASE64_URL_SAFE_NO_PAD.encode(token_bytes)
    ))
}
