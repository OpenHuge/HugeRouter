use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD};
use core_domain::UserId;
use ring::rand;
use serde::Deserialize;

use crate::alipay::{
    AlipayClient, AlipayPrecreateRequest, AlipayPrecreateResponse,
    new_out_trade_no as new_alipay_out_trade_no,
};
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
    authorize_optional_v1_request, authorize_v1_request, download_grant_issue_result_to_response,
    generate_delivery_download_token, generate_stable_id, next_request_context,
    redeem_result_to_response, sha256_hex,
};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMerchantProductOrderRequest {
    pub card_product_id: String,
    pub buyer_phone: Option<String>,
    pub lookup_passphrase: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MerchantProductOrderHistoryRequest {
    pub buyer_phone: String,
    pub lookup_passphrase: String,
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
    let buyer_user_id = resolve_checkout_buyer_user_id(&state, &headers, &context).await?;
    let buyer_contact = build_buyer_contact(
        request.buyer_phone.as_deref(),
        request.lookup_passphrase.as_deref(),
        &context,
    )?;
    let generated_lookup_passphrase = buyer_contact
        .as_ref()
        .and_then(|contact| contact.generated_passphrase.clone());
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
            buyer_user_id,
            buyer_contact: buyer_contact.map(|contact| contact.record),
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_create_failed",
                format!("failed to create merchant product order: {error}"),
                &context,
            )
        })? {
        MerchantProductOrderCreateResult::Created(mut response) => {
            if let (Some(public_contact), Some(generated)) = (
                response.data.buyer_contact.as_mut(),
                generated_lookup_passphrase,
            ) {
                public_contact.lookup_passphrase = Some(generated);
            }
            Ok(Json(*response))
        }
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

pub async fn list_merchant_product_order_history(
    State(state): State<ControlPlaneState>,
    Json(request): Json<MerchantProductOrderHistoryRequest>,
) -> Result<Json<store::MerchantProductOrderHistoryResponse>, ApiError> {
    let context = next_request_context();
    let phone = normalize_buyer_phone(&request.buyer_phone, &context)?;
    let passphrase = normalize_lookup_passphrase(&request.lookup_passphrase, &context)?;
    state
        .store
        .list_merchant_product_orders_by_guest_lookup(
            &guest_phone_hash(&phone),
            &guest_lookup_passphrase_hash(&phone, &passphrase),
        )
        .await
        .map(Json)
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_history_unavailable",
                format!("failed to load merchant product order history: {error}"),
                &context,
            )
        })
}

pub async fn get_merchant_product_order(
    State(state): State<ControlPlaneState>,
    _headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<store::MerchantProductOrderResponse>, ApiError> {
    let context = next_request_context();
    let response = load_merchant_order(&state, &order_id, &context).await?;
    Ok(Json(response))
}

pub async fn create_merchant_product_order_wechat_prepay(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(request): Json<MerchantProductPrepayRequest>,
) -> Result<Json<WechatPayPrepayResponse>, ApiError> {
    let context = next_request_context();
    let order = load_merchant_order(&state, &order_id, &context).await?;
    if order.data.status == store::MERCHANT_ORDER_STATUS_FULFILLED {
        return Err(ApiError::conflict(
            "merchant_order_already_fulfilled",
            "merchant product order has already been fulfilled".to_string(),
            &context,
        ));
    }
    let payer_openid = if matches!(request.channel, crate::wechat_pay::WechatPayChannel::Jsapi) {
        let authz = authorize_v1_request(&state, &headers, &context).await?;
        ensure_wechat_buyer(&authz, &context)?;
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

pub async fn create_merchant_product_order_alipay_prepay(
    State(state): State<ControlPlaneState>,
    _headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<AlipayPrecreateResponse>, ApiError> {
    let context = next_request_context();
    let order = load_merchant_order(&state, &order_id, &context).await?;
    if order.data.status == store::MERCHANT_ORDER_STATUS_FULFILLED {
        return Err(ApiError::conflict(
            "merchant_order_already_fulfilled",
            "merchant product order has already been fulfilled".to_string(),
            &context,
        ));
    }
    let client = AlipayClient::from_env().map_err(|error| {
        ApiError::internal(
            "alipay_not_configured",
            format!("Alipay is not configured: {error}"),
            &context,
        )
    })?;
    let out_trade_no = new_alipay_out_trade_no(
        order.data.tenant_id.as_str(),
        Some(order.data.project_id.as_str()),
    );
    let request = AlipayPrecreateRequest {
        tenant_id: order.data.tenant_id.as_str().to_string(),
        project_id: Some(order.data.project_id.as_str().to_string()),
        amount_total: order.data.amount_total,
        currency: "CNY".to_string(),
        description: format!("Card product {}", order.data.card_product_id.as_str()),
        attach: Some(order.data.order_id.clone()),
    };
    crate::alipay_api::validate_alipay_payment_request(&request, &context)?;
    let mut payment_order = crate::alipay_api::new_alipay_order(
        &out_trade_no,
        order.data.tenant_id.as_str(),
        Some(order.data.project_id.as_str().to_string()),
        order.data.amount_total,
        serde_json::json!({
            "merchant_product_order_id": order.data.order_id,
            "card_product_id": order.data.card_product_id,
            "description": "Card product checkout",
        }),
    );
    state
        .store
        .create_alipay_payment_order(payment_order.clone())
        .await
        .map_err(|error| {
            ApiError::internal(
                "alipay_order_persist_failed",
                format!("failed to persist Alipay order: {error}"),
                &context,
            )
        })?;
    let response = match client.precreate(request, &out_trade_no).await {
        Ok(response) => response,
        Err(error) => {
            payment_order.status = "failed".to_string();
            payment_order.updated_at = now_rfc3339();
            let _ = state.store.update_alipay_payment_order(payment_order).await;
            return Err(ApiError::bad_request(
                "alipay_precreate_failed",
                format!("Alipay precreate request failed: {error}"),
                &context,
            ));
        }
    };
    payment_order.status = "pending".to_string();
    payment_order.code_url = Some(response.qr_code.clone());
    payment_order.updated_at = now_rfc3339();
    state
        .store
        .update_alipay_payment_order(payment_order)
        .await
        .map_err(|error| {
            ApiError::internal(
                "alipay_order_persist_failed",
                format!("failed to update Alipay order: {error}"),
                &context,
            )
        })?;
    state
        .store
        .attach_merchant_order_prepay(MerchantProductPrepayDraft {
            order_id: order.data.order_id,
            out_trade_no,
            channel: "alipay_qr".to_string(),
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "merchant_order_prepay_attach_failed",
                format!("failed to attach Alipay prepay to merchant order: {error}"),
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
            fulfilled_by: "merchant_product_payment".to_string(),
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

async fn resolve_checkout_buyer_user_id(
    state: &ControlPlaneState,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<UserId, ApiError> {
    if let Some(authz) = authorize_optional_v1_request(state, headers, context).await? {
        return Ok(authz.user_id().clone());
    }
    UserId::parse(generate_stable_id("user", context)?).map_err(|error| {
        ApiError::internal(
            "checkout_buyer_id_generation_failed",
            format!("failed to create public checkout buyer id: {error}"),
            context,
        )
    })
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

async fn load_merchant_order(
    state: &ControlPlaneState,
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

struct PreparedBuyerContact {
    record: store::MerchantProductOrderBuyerContactRecord,
    generated_passphrase: Option<String>,
}

fn build_buyer_contact(
    buyer_phone: Option<&str>,
    lookup_passphrase: Option<&str>,
    context: &RequestContext,
) -> Result<Option<PreparedBuyerContact>, ApiError> {
    let Some(phone) = buyer_phone else {
        return Ok(None);
    };
    let normalized_phone = normalize_buyer_phone(phone, context)?;
    let (normalized_passphrase, generated_passphrase) =
        match lookup_passphrase.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then_some(trimmed)
        }) {
            Some(passphrase) => (normalize_lookup_passphrase(passphrase, context)?, None),
            None => {
                let generated = generate_lookup_passphrase(context)?;
                (generated.clone(), Some(generated))
            }
        };
    Ok(Some(PreparedBuyerContact {
        record: store::MerchantProductOrderBuyerContactRecord {
            phone_masked: mask_buyer_phone(&normalized_phone),
            phone_hash: guest_phone_hash(&normalized_phone),
            lookup_passphrase_hash: guest_lookup_passphrase_hash(
                &normalized_phone,
                &normalized_passphrase,
            ),
            lookup_passphrase_hint: generated_passphrase
                .clone()
                .unwrap_or_else(|| mask_lookup_passphrase(&normalized_passphrase)),
        },
        generated_passphrase,
    }))
}

fn normalize_buyer_phone(value: &str, context: &RequestContext) -> Result<String, ApiError> {
    let digits = value
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    let normalized = digits
        .strip_prefix("86")
        .filter(|rest| rest.len() == 11)
        .unwrap_or(&digits);
    if normalized.len() == 11 && normalized.starts_with('1') {
        Ok(normalized.to_string())
    } else {
        Err(ApiError::bad_request(
            "buyer_phone_invalid",
            "buyer_phone must be a valid mainland China mobile number".to_string(),
            context,
        ))
    }
}

fn normalize_lookup_passphrase(value: &str, context: &RequestContext) -> Result<String, ApiError> {
    let normalized = value.trim();
    if (6..=32).contains(&normalized.chars().count())
        && normalized
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        Ok(normalized.to_ascii_uppercase())
    } else {
        Err(ApiError::bad_request(
            "lookup_passphrase_invalid",
            "lookup_passphrase must be 6 to 32 letters or digits".to_string(),
            context,
        ))
    }
}

fn mask_buyer_phone(phone: &str) -> String {
    format!("{}****{}", &phone[..3], &phone[7..])
}

fn mask_lookup_passphrase(passphrase: &str) -> String {
    if passphrase.len() <= 8 {
        return "已设置".to_string();
    }
    format!("{}***{}", &passphrase[..2], &passphrase[passphrase.len() - 2..])
}

fn guest_phone_hash(phone: &str) -> String {
    sha256_hex(&format!("merchant-order-phone:{phone}"))
}

fn guest_lookup_passphrase_hash(phone: &str, passphrase: &str) -> String {
    sha256_hex(&format!("merchant-order-lookup:{phone}:{passphrase}"))
}

fn generate_lookup_passphrase(context: &RequestContext) -> Result<String, ApiError> {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let rng = rand::SystemRandom::new();
    let mut bytes = [0_u8; 6];
    rand::SecureRandom::fill(&rng, &mut bytes).map_err(|_| {
        ApiError::internal(
            "lookup_passphrase_generation_failed",
            "failed to generate order lookup passphrase".to_string(),
            context,
        )
    })?;
    Ok(bytes
        .iter()
        .map(|byte| ALPHABET[usize::from(*byte) % ALPHABET.len()] as char)
        .collect())
}
