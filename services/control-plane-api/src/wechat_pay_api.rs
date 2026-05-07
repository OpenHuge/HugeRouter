use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use serde_json::Value;

use crate::store::{WechatPaymentOrderRecord, WechatPaymentOrderResponse, expires_at, now_rfc3339};
use crate::wechat_pay::{
    WechatPayClient, WechatPayHeaders, WechatPayPrepayRequest, WechatPayPrepayResponse,
    new_out_trade_no,
};
use crate::{
    ApiError, ControlPlaneState, RequestContext, authorize_v1_request,
    ensure_project_matches_tenant, load_project, next_request_context, required_header, sha256_hex,
};

#[derive(Debug, Clone, Deserialize)]
pub struct WechatPaymentOrderQuery {
    #[serde(default)]
    pub refresh: bool,
}

pub async fn create_wechat_pay_prepay(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<WechatPayPrepayRequest>,
) -> Result<Json<WechatPayPrepayResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    validate_wechat_payment_request(&request, &context)?;
    authz.ensure_manage_tenant(&request.tenant_id, &context)?;
    if let Some(project_id) = request.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        ensure_project_matches_tenant(&project, &request.tenant_id, &context)?;
        authz.ensure_manage_project(&project, &context)?;
    }

    let client = WechatPayClient::from_env().map_err(|error| {
        ApiError::internal(
            "wechat_pay_not_configured",
            format!("WeChat Pay is not configured: {error}"),
            &context,
        )
    })?;
    let out_trade_no = new_out_trade_no(&request.tenant_id, request.project_id.as_deref());
    let now = now_rfc3339();
    let mut order = WechatPaymentOrderRecord {
        out_trade_no: out_trade_no.clone(),
        tenant_id: request.tenant_id.clone(),
        project_id: request.project_id.clone(),
        amount_total: request.amount_total,
        currency: request.currency.clone(),
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
            "description": request.description,
            "attach": request.attach,
        }),
    };
    state
        .store
        .create_wechat_payment_order(order.clone())
        .await
        .map_err(|error| {
            ApiError::internal(
                "wechat_pay_order_persist_failed",
                format!("failed to persist WeChat Pay order: {error}"),
                &context,
            )
        })?;

    let response = match client.create_prepay(request, &out_trade_no).await {
        Ok(response) => response,
        Err(error) => {
            order.status = "failed".to_string();
            order.updated_at = now_rfc3339();
            order.metadata = serde_json::json!({
                "failure_stage": "wechat_prepay",
                "failure_message": error.to_string(),
            });
            let _ = state.store.update_wechat_payment_order(order).await;
            return Err(ApiError::bad_request(
                "wechat_pay_prepay_failed",
                format!("WeChat Pay prepay request failed: {error}"),
                &context,
            ));
        }
    };
    order.status = "pending".to_string();
    order.code_url.clone_from(&response.code_url);
    order.prepay_id.clone_from(&response.prepay_id);
    order.updated_at = now_rfc3339();
    state
        .store
        .update_wechat_payment_order(order)
        .await
        .map_err(|error| {
            ApiError::internal(
                "wechat_pay_order_persist_failed",
                format!("failed to update WeChat Pay order: {error}"),
                &context,
            )
        })?;

    Ok(Json(response))
}

pub async fn accept_wechat_pay_notification(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let context = next_request_context();
    let client = WechatPayClient::from_env().map_err(|error| {
        ApiError::internal(
            "wechat_pay_not_configured",
            format!("WeChat Pay is not configured: {error}"),
            &context,
        )
    })?;
    let wechat_headers = WechatPayHeaders {
        timestamp: required_header(&headers, "Wechatpay-Timestamp", &context)?,
        nonce: required_header(&headers, "Wechatpay-Nonce", &context)?,
        signature: required_header(&headers, "Wechatpay-Signature", &context)?,
        serial: required_header(&headers, "Wechatpay-Serial", &context)?,
    };
    let notification = client
        .decode_notification(&wechat_headers, &body)
        .map_err(|error| {
            ApiError::bad_request(
                "wechat_pay_notification_invalid",
                format!("invalid WeChat Pay notification: {error}"),
                &context,
            )
        })?;
    let out_trade_no = transaction_out_trade_no(&notification.transaction, &context)?;
    let Some(existing) = state
        .store
        .get_wechat_payment_order(&out_trade_no)
        .await
        .map_err(|error| {
            ApiError::internal(
                "wechat_pay_order_unavailable",
                format!("failed to load WeChat Pay order: {error}"),
                &context,
            )
        })?
    else {
        return Err(ApiError::bad_request(
            "wechat_pay_order_unknown",
            format!("WeChat Pay order `{out_trade_no}` is not known"),
            &context,
        ));
    };
    let updated = apply_wechat_transaction_to_order(
        existing.data,
        &notification.transaction,
        Some(notification.id.clone()),
        client.app_id(),
        client.mchid(),
        &context,
    )?;
    let updated_status = updated.status.clone();
    state
        .store
        .update_wechat_payment_order(updated)
        .await
        .map_err(|error| {
            ApiError::internal(
                "wechat_pay_order_persist_failed",
                format!("failed to update WeChat Pay order: {error}"),
                &context,
            )
        })?;
    if updated_status == "paid" {
        crate::merchant_checkout_api::settle_merchant_product_order_for_payment(
            &state,
            &out_trade_no,
            &context,
        )
        .await?;
    }

    tracing::info!(
        event_type = notification.event_type,
        notification_id = notification.id,
        resource_type = notification.resource_type,
        "accepted WeChat Pay notification"
    );

    Ok(Json(serde_json::json!({
        "code": "SUCCESS",
        "message": "success"
    })))
}

pub async fn get_wechat_payment_order(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(out_trade_no): Path<String>,
    Query(query): Query<WechatPaymentOrderQuery>,
) -> Result<Json<WechatPaymentOrderResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let order = state
        .store
        .get_wechat_payment_order(&out_trade_no)
        .await
        .map_err(|error| {
            ApiError::internal(
                "wechat_pay_order_unavailable",
                format!("failed to load WeChat Pay order: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "wechat_pay_order_not_found",
                format!("WeChat Pay order `{out_trade_no}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(&order.data.tenant_id, &context)?;

    if query.refresh && !wechat_order_status_is_terminal(&order.data.status) {
        let client = WechatPayClient::from_env().map_err(|error| {
            ApiError::internal(
                "wechat_pay_not_configured",
                format!("WeChat Pay is not configured: {error}"),
                &context,
            )
        })?;
        let queried = client.query_order(&out_trade_no).await.map_err(|error| {
            ApiError::bad_request(
                "wechat_pay_order_query_failed",
                format!("WeChat Pay order query failed: {error}"),
                &context,
            )
        })?;
        let updated = apply_wechat_transaction_to_order(
            order.data,
            &queried.transaction,
            None,
            client.app_id(),
            client.mchid(),
            &context,
        )?;
        let updated_status = updated.status.clone();
        let response = state
            .store
            .update_wechat_payment_order(updated)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "wechat_pay_order_persist_failed",
                    format!("failed to update WeChat Pay order: {error}"),
                    &context,
                )
            })?;
        if updated_status == "paid" {
            crate::merchant_checkout_api::settle_merchant_product_order_for_payment(
                &state,
                &out_trade_no,
                &context,
            )
            .await?;
        }
        return Ok(Json(response));
    }

    Ok(Json(order))
}

pub const fn wechat_pay_channel_slug(
    channel: &crate::wechat_pay::WechatPayChannel,
) -> &'static str {
    match channel {
        crate::wechat_pay::WechatPayChannel::Native => "native",
        crate::wechat_pay::WechatPayChannel::Jsapi => "jsapi",
    }
}

fn validate_wechat_payment_request(
    request: &WechatPayPrepayRequest,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let min_amount = std::env::var("WECHAT_PAY_MIN_AMOUNT_TOTAL")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let max_amount = std::env::var("WECHAT_PAY_MAX_AMOUNT_TOTAL")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1_000_000);

    if request.amount_total < min_amount || request.amount_total > max_amount {
        return Err(ApiError::bad_request(
            "wechat_pay_amount_invalid",
            format!("amount_total must be between {min_amount} and {max_amount} cents"),
            context,
        ));
    }
    if request.description.trim().is_empty() || request.description.chars().count() > 127 {
        return Err(ApiError::bad_request(
            "wechat_pay_description_invalid",
            "description must be present and at most 127 characters".to_string(),
            context,
        ));
    }

    Ok(())
}

fn wechat_order_status_is_terminal(status: &str) -> bool {
    matches!(status, "paid" | "closed" | "failed" | "refunded")
}

fn transaction_out_trade_no(
    transaction: &Value,
    context: &RequestContext,
) -> Result<String, ApiError> {
    transaction
        .get("out_trade_no")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            ApiError::bad_request(
                "wechat_pay_transaction_invalid",
                "WeChat Pay transaction is missing out_trade_no".to_string(),
                context,
            )
        })
}

fn transaction_string(transaction: &Value, key: &str) -> Option<String> {
    transaction
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn transaction_amount_total(transaction: &Value) -> Option<u32> {
    transaction
        .get("amount")
        .and_then(|amount| amount.get("total"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn apply_wechat_transaction_to_order(
    mut order: WechatPaymentOrderRecord,
    transaction: &Value,
    notification_id: Option<String>,
    expected_app_id: &str,
    expected_mchid: &str,
    context: &RequestContext,
) -> Result<WechatPaymentOrderRecord, ApiError> {
    let out_trade_no = transaction_out_trade_no(transaction, context)?;
    if out_trade_no != order.out_trade_no {
        return Err(ApiError::bad_request(
            "wechat_pay_transaction_mismatch",
            "WeChat Pay transaction does not match the stored order".to_string(),
            context,
        ));
    }

    if transaction_string(transaction, "appid").as_deref() != Some(expected_app_id)
        || transaction_string(transaction, "mchid").as_deref() != Some(expected_mchid)
    {
        return Err(ApiError::bad_request(
            "wechat_pay_merchant_mismatch",
            "WeChat Pay transaction appid or mchid does not match configuration".to_string(),
            context,
        ));
    }

    if transaction_amount_total(transaction) != Some(order.amount_total) {
        return Err(ApiError::bad_request(
            "wechat_pay_amount_mismatch",
            "WeChat Pay transaction amount does not match the stored order".to_string(),
            context,
        ));
    }

    let trade_state =
        transaction_string(transaction, "trade_state").unwrap_or_else(|| "UNKNOWN".to_string());
    order.trade_state = Some(trade_state.clone());
    order.transaction_id = transaction_string(transaction, "transaction_id");
    if notification_id.is_some() {
        order.notification_id = notification_id;
    }
    order.updated_at = now_rfc3339();
    order.metadata = serde_json::json!({
        "trade_state_desc": transaction_string(transaction, "trade_state_desc"),
        "bank_type": transaction_string(transaction, "bank_type"),
        "success_time": transaction_string(transaction, "success_time"),
        "payer_openid_sha256": transaction
            .get("payer")
            .and_then(|payer| payer.get("openid"))
            .and_then(Value::as_str)
            .map(sha256_hex),
    });

    match trade_state.as_str() {
        "SUCCESS" => {
            order.status = "paid".to_string();
            order.paid_at =
                transaction_string(transaction, "success_time").or_else(|| Some(now_rfc3339()));
        }
        "CLOSED" | "REVOKED" | "PAYERROR" => {
            order.status = "failed".to_string();
        }
        _ => {
            order.status = "pending".to_string();
        }
    }

    Ok(order)
}
