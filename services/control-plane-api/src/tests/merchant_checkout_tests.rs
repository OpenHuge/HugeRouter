use super::*;
use crate::store::{MerchantProductFulfillmentDraft, MerchantProductFulfillmentResult};

#[tokio::test]
async fn merchant_pickup_requires_fulfilled_payment_order() {
    let (state, admin_cookie, app) = platform_admin_app().await;
    let wechat_cookie = platform_wechat_cookie(&state).await;

    let shop = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/merchant/shops",
            Some(&admin_cookie),
            Some(json!({
                "merchant_shop_id": "mshop_hugecode_payment_test",
                "slug": "hugecode-payment-test",
                "display_name": "HugeCode Payment Test",
                "announcement": "Payment test shop"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(shop.status(), StatusCode::OK);

    let delivery_body = prepare_delivery_for_test(app.clone(), &admin_cookie).await;
    let delivery_id = delivery_body["data"]["delivery"]["delivery_id"]
        .as_str()
        .unwrap();

    let product = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/merchant/card-products",
            Some(&admin_cookie),
            Some(json!({
                "card_product_id": "cprod_hugecode_payment_test",
                "merchant_shop_id": "mshop_hugecode_payment_test",
                "project_id": "proj_core",
                "title": "HugeCode Pro Week",
                "description": "Payment test product",
                "inventory_count": 1,
                "face_value_usd": "0",
                "retail_price_usd": "0",
                "retail_price_cny_total": 1,
                "supports_trial": false,
                "sale_enabled": true,
                "delivery_ids": [delivery_id]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(product.status(), StatusCode::OK);

    let order = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/merchant-product-orders",
            Some(&wechat_cookie),
            Some(json!({
                "card_product_id": "cprod_hugecode_payment_test"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(order.status(), StatusCode::OK);
    let order_body = response_json(order).await;
    assert_eq!(order_body["data"]["status"], "created");
    assert!(order_body["data"]["pickup_token"].is_null());

    let pickup = app
        .oneshot(request(
            "GET",
            "/v1/pickups/not-yet-issued",
            Some(&admin_cookie),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(pickup.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fulfilled_merchant_order_exposes_pickup_token_once() {
    let (state, admin_cookie, app) = platform_admin_app().await;
    let wechat_cookie = platform_wechat_cookie(&state).await;

    let shop = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/merchant/shops",
            Some(&admin_cookie),
            Some(json!({
                "merchant_shop_id": "mshop_hugecode_fulfilled_test",
                "slug": "hugecode-fulfilled-test",
                "display_name": "HugeCode Fulfilled Test",
                "announcement": "Fulfillment test shop"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(shop.status(), StatusCode::OK);

    let delivery_body = prepare_delivery_for_test(app.clone(), &admin_cookie).await;
    let delivery_id = delivery_body["data"]["delivery"]["delivery_id"]
        .as_str()
        .unwrap();

    let product = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/merchant/card-products",
            Some(&admin_cookie),
            Some(json!({
                "card_product_id": "cprod_hugecode_fulfilled_test",
                "merchant_shop_id": "mshop_hugecode_fulfilled_test",
                "project_id": "proj_core",
                "title": "HugeCode Pro Fulfilled Week",
                "description": "Fulfillment test product",
                "inventory_count": 1,
                "face_value_usd": "0",
                "retail_price_usd": "0",
                "retail_price_cny_total": 1,
                "supports_trial": false,
                "sale_enabled": true,
                "delivery_ids": [delivery_id]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(product.status(), StatusCode::OK);

    let order = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/merchant-product-orders",
            Some(&wechat_cookie),
            Some(json!({
                "card_product_id": "cprod_hugecode_fulfilled_test"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(order.status(), StatusCode::OK);
    let order_body = response_json(order).await;
    let order_id = order_body["data"]["order_id"].as_str().unwrap().to_string();

    let fulfilled = state
        .store
        .fulfill_merchant_product_order(MerchantProductFulfillmentDraft {
            order_id,
            activation_id: "activation_hugecode_fulfilled_test".to_string(),
            download_grant_id: "dlgrant_hugecode_fulfilled_test".to_string(),
            download_token: "dltok_hugecode_fulfilled_test".to_string(),
            pickup_token: "pickup_hugecode_fulfilled_test".to_string(),
            fulfilled_by: "test".to_string(),
        })
        .await
        .unwrap();
    assert!(matches!(
        fulfilled,
        MerchantProductFulfillmentResult::Fulfilled(_)
    ));

    let pickup = app
        .oneshot(request(
            "GET",
            "/v1/pickups/pickup_hugecode_fulfilled_test",
            Some(&admin_cookie),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(pickup.status(), StatusCode::OK);
    let pickup_body = response_json(pickup).await;
    assert_eq!(pickup_body["data"]["order"]["status"], "fulfilled");
    assert_eq!(
        pickup_body["data"]["download_token"],
        "dltok_hugecode_fulfilled_test"
    );
}
