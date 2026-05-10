use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use serde_json::Value;

use crate::alipay::{
    AlipayClient, AlipayPrecreateRequest, AlipayPrecreateResponse, alipay_trade_status_is_paid,
    is_alipay_trade_not_exist_error, new_out_trade_no,
};
use crate::store::{AlipayPaymentOrderRecord, AlipayPaymentOrderResponse, expires_at, now_rfc3339};
use crate::{
    ApiError, ControlPlaneState, RequestContext, authorize_v1_request,
    ensure_project_matches_tenant, load_project, next_request_context,
};

#[derive(Debug, Clone, Deserialize)]
pub struct AlipayPaymentOrderQuery {
    #[serde(default)]
    pub refresh: bool,
}

pub async fn create_alipay_prepay(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Json(request): Json<AlipayPrecreateRequest>,
) -> Result<Json<AlipayPrecreateResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    validate_alipay_payment_request(&request, &context)?;
    authz.ensure_manage_tenant(&request.tenant_id, &context)?;
    if let Some(project_id) = request.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        ensure_project_matches_tenant(&project, &request.tenant_id, &context)?;
        authz.ensure_manage_project(&project, &context)?;
    }
    let client = alipay_client(&context)?;
    let out_trade_no = new_out_trade_no(&request.tenant_id, request.project_id.as_deref());
    let mut order = new_alipay_order(
        &out_trade_no,
        &request.tenant_id,
        request.project_id.clone(),
        request.amount_total,
        serde_json::json!({
            "description": request.description,
            "attach": request.attach,
        }),
    );
    state
        .store
        .create_alipay_payment_order(order.clone())
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
            order.status = "failed".to_string();
            order.updated_at = now_rfc3339();
            order.metadata = serde_json::json!({
                "failure_stage": "alipay_precreate",
                "failure_message": error.to_string(),
            });
            let _ = state.store.update_alipay_payment_order(order).await;
            return Err(ApiError::bad_request(
                "alipay_precreate_failed",
                format!("Alipay precreate request failed: {error}"),
                &context,
            ));
        }
    };
    order.status = "pending".to_string();
    order.code_url = Some(response.qr_code.clone());
    order.updated_at = now_rfc3339();
    state
        .store
        .update_alipay_payment_order(order)
        .await
        .map_err(|error| {
            ApiError::internal(
                "alipay_order_persist_failed",
                format!("failed to update Alipay order: {error}"),
                &context,
            )
        })?;
    Ok(Json(response))
}

pub async fn accept_alipay_notification(
    State(state): State<ControlPlaneState>,
    body: Bytes,
) -> Result<&'static str, ApiError> {
    let context = next_request_context();
    let client = alipay_client(&context)?;
    let body_text = std::str::from_utf8(&body).map_err(|error| {
        ApiError::bad_request(
            "alipay_notification_invalid",
            format!("Alipay notification body is not UTF-8: {error}"),
            &context,
        )
    })?;
    let notification = client.verify_notify(body_text).map_err(|error| {
        ApiError::bad_request(
            "alipay_notification_invalid",
            format!("invalid Alipay notification: {error}"),
            &context,
        )
    })?;
    let Some(existing) = state
        .store
        .get_alipay_payment_order(&notification.out_trade_no)
        .await
        .map_err(|error| {
            ApiError::internal(
                "alipay_order_unavailable",
                format!("failed to load Alipay order: {error}"),
                &context,
            )
        })?
    else {
        return Ok("success");
    };
    let updated = apply_alipay_trade_to_order(
        existing.data,
        &notification.out_trade_no,
        notification.trade_no.as_deref(),
        &notification.trade_status,
        notification.total_amount.as_deref(),
        notification.notify_id.as_deref(),
        notification.gmt_payment.as_deref(),
        &context,
    )?;
    let updated_status = updated.status.clone();
    state
        .store
        .update_alipay_payment_order(updated)
        .await
        .map_err(|error| {
            ApiError::internal(
                "alipay_order_persist_failed",
                format!("failed to update Alipay order: {error}"),
                &context,
            )
        })?;
    if updated_status == "paid" {
        crate::merchant_checkout_api::settle_merchant_product_order_for_payment(
            &state,
            &notification.out_trade_no,
            &context,
        )
        .await?;
    }
    Ok("success")
}

pub async fn get_alipay_payment_order(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(out_trade_no): Path<String>,
    Query(query): Query<AlipayPaymentOrderQuery>,
) -> Result<Json<AlipayPaymentOrderResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let order = state
        .store
        .get_alipay_payment_order(&out_trade_no)
        .await
        .map_err(|error| {
            ApiError::internal(
                "alipay_order_unavailable",
                format!("failed to load Alipay order: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "alipay_order_not_found",
                format!("Alipay order `{out_trade_no}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(&order.data.tenant_id, &context)?;
    if query.refresh && !alipay_order_status_is_terminal(&order.data.status) {
        let client = alipay_client(&context)?;
        let queried = match client.query_order(&out_trade_no).await {
            Ok(queried) => queried,
            Err(error) if is_alipay_trade_not_exist_error(&error) => {
                return Ok(Json(order));
            }
            Err(error) => {
                return Err(ApiError::bad_request(
                    "alipay_order_query_failed",
                    format!("Alipay order query failed: {error}"),
                    &context,
                ));
            }
        };
        let updated = apply_alipay_query_to_order(order.data, &queried, &context)?;
        let updated_status = updated.status.clone();
        let response = state
            .store
            .update_alipay_payment_order(updated)
            .await
            .map_err(|error| {
                ApiError::internal(
                    "alipay_order_persist_failed",
                    format!("failed to update Alipay order: {error}"),
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

pub fn new_alipay_order(
    out_trade_no: &str,
    tenant_id: &str,
    project_id: Option<String>,
    amount_total: u32,
    metadata: Value,
) -> AlipayPaymentOrderRecord {
    let now = now_rfc3339();
    AlipayPaymentOrderRecord {
        out_trade_no: out_trade_no.to_string(),
        tenant_id: tenant_id.to_string(),
        project_id,
        amount_total,
        currency: "CNY".to_string(),
        channel: "alipay_qr".to_string(),
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
        metadata,
    }
}

pub fn apply_alipay_query_to_order(
    order: AlipayPaymentOrderRecord,
    trade: &Value,
    context: &RequestContext,
) -> Result<AlipayPaymentOrderRecord, ApiError> {
    let out_trade_no = trade_string(trade, "out_trade_no").ok_or_else(|| {
        ApiError::bad_request(
            "alipay_trade_invalid",
            "Alipay trade is missing out_trade_no".to_string(),
            context,
        )
    })?;
    let trade_status = trade_string(trade, "trade_status").unwrap_or_else(|| "UNKNOWN".to_string());
    apply_alipay_trade_to_order(
        order,
        &out_trade_no,
        trade_string(trade, "trade_no").as_deref(),
        &trade_status,
        trade_string(trade, "total_amount").as_deref(),
        None,
        trade_string(trade, "send_pay_date").as_deref(),
        context,
    )
}

pub fn apply_alipay_trade_to_order(
    mut order: AlipayPaymentOrderRecord,
    out_trade_no: &str,
    trade_no: Option<&str>,
    trade_status: &str,
    total_amount: Option<&str>,
    notification_id: Option<&str>,
    paid_at: Option<&str>,
    context: &RequestContext,
) -> Result<AlipayPaymentOrderRecord, ApiError> {
    if out_trade_no != order.out_trade_no {
        return Err(ApiError::bad_request(
            "alipay_trade_mismatch",
            "Alipay trade does not match the stored order".to_string(),
            context,
        ));
    }
    if let Some(total_amount) = total_amount
        && total_amount != amount_total_to_yuan(order.amount_total)
    {
        return Err(ApiError::bad_request(
            "alipay_amount_mismatch",
            "Alipay trade amount does not match the stored order".to_string(),
            context,
        ));
    }
    order.trade_state = Some(trade_status.to_string());
    order.transaction_id = trade_no.map(str::to_string);
    if let Some(notification_id) = notification_id {
        order.notification_id = Some(notification_id.to_string());
    }
    order.updated_at = now_rfc3339();
    order.metadata = serde_json::json!({
        "trade_status": trade_status,
        "total_amount": total_amount,
    });
    if alipay_trade_status_is_paid(trade_status) {
        order.status = "paid".to_string();
        order.paid_at = paid_at.map(str::to_string).or_else(|| Some(now_rfc3339()));
    } else if matches!(trade_status, "TRADE_CLOSED") {
        order.status = "failed".to_string();
    } else {
        order.status = "pending".to_string();
    }
    Ok(order)
}

pub fn validate_alipay_payment_request(
    request: &AlipayPrecreateRequest,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let min_amount = std::env::var("ALIPAY_MIN_AMOUNT_TOTAL")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let max_amount = std::env::var("ALIPAY_MAX_AMOUNT_TOTAL")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1_000_000);
    if request.amount_total < min_amount || request.amount_total > max_amount {
        return Err(ApiError::bad_request(
            "alipay_amount_invalid",
            format!("amount_total must be between {min_amount} and {max_amount} cents"),
            context,
        ));
    }
    if request.description.trim().is_empty() || request.description.chars().count() > 256 {
        return Err(ApiError::bad_request(
            "alipay_description_invalid",
            "description must be present and at most 256 characters".to_string(),
            context,
        ));
    }
    Ok(())
}

fn alipay_client(context: &RequestContext) -> Result<AlipayClient, ApiError> {
    AlipayClient::from_env().map_err(|error| {
        ApiError::internal(
            "alipay_not_configured",
            format!("Alipay is not configured: {error}"),
            context,
        )
    })
}

fn amount_total_to_yuan(amount_total: u32) -> String {
    format!("{}.{:02}", amount_total / 100, amount_total % 100)
}

fn trade_string(trade: &Value, key: &str) -> Option<String> {
    trade
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn alipay_order_status_is_terminal(status: &str) -> bool {
    matches!(status, "paid" | "closed" | "failed" | "refunded")
}

#[cfg(test)]
mod tests {
    use super::{apply_alipay_trade_to_order, new_alipay_order};
    use crate::RequestContext;

    #[test]
    fn paid_alipay_trade_marks_order_paid() {
        let context = RequestContext {
            request_id: "req_test".to_string(),
            trace_id: "trace_test".to_string(),
        };
        let order = new_alipay_order("ha_test", "tenant_acme", Some("proj_core".to_string()), 1, serde_json::json!({}));
        let updated = apply_alipay_trade_to_order(
            order,
            "ha_test",
            Some("trade_1"),
            "TRADE_SUCCESS",
            Some("0.01"),
            Some("notify_1"),
            Some("2026-05-10 18:00:00"),
            &context,
        )
        .unwrap();
        assert_eq!(updated.status, "paid");
        assert_eq!(updated.transaction_id.as_deref(), Some("trade_1"));
    }
}
