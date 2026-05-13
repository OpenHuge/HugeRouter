use super::*;
use crate::BASE64;
use base64::Engine as _;

fn request_with_bearer(
    method: &str,
    uri: &str,
    bearer: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {bearer}"));
    if let Some(body) = body {
        builder = builder.header("content-type", "application/json");
        builder.body(Body::from(body.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    }
}

async fn issue_producer_authorization_for_test(app: axum::Router, owner_account_id: &str) -> Value {
    let response = app
        .oneshot(request_with_bearer(
            "POST",
            "/internal/producer-authorizations",
            "test-internal-token",
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": owner_account_id,
                "service_kind": "manual_browser_account",
                "service_days": 30
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}

async fn redeem_producer_authorization_for_test(
    app: axum::Router,
    authorization_code: &str,
) -> Value {
    let response = app
        .oneshot(request(
            "POST",
            "/v1/producer-authorizations/redeem",
            None,
            Some(json!({ "authorization_code": authorization_code })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

async fn producer_token_for_owner_for_test(app: axum::Router, owner_account_id: &str) -> String {
    let issued = issue_producer_authorization_for_test(app.clone(), owner_account_id).await;
    let authorization_code = issued["authorization_code"].as_str().unwrap();
    let redeemed = redeem_producer_authorization_for_test(app, authorization_code).await;
    redeemed["producer_token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn producer_authorization_redeems_short_lived_token_and_limits_scope() {
    let (_state, _admin_cookie, app) = platform_admin_app().await;
    let owner_account_id = "acct_producer_token_owner";
    let issued = issue_producer_authorization_for_test(app.clone(), owner_account_id).await;
    let authorization_code = issued["authorization_code"].as_str().unwrap();
    assert!(authorization_code.starts_with("prodaz_"));
    assert!(!issued.to_string().contains("prodtok_"));

    let first = redeem_producer_authorization_for_test(app.clone(), authorization_code).await;
    let second = redeem_producer_authorization_for_test(app.clone(), authorization_code).await;
    let producer_token = first["producer_token"].as_str().unwrap();
    let second_token = second["producer_token"].as_str().unwrap();
    assert!(producer_token.starts_with("prodtok_"));
    assert!(second_token.starts_with("prodtok_"));
    assert_ne!(producer_token, second_token);
    assert_eq!(first["tenant_id"], "tenant_acme");
    assert_eq!(first["project_id"], "proj_core");
    assert_eq!(first["owner_account_id"], owner_account_id);
    assert_eq!(first["service_kind"], "manual_browser_account");
    assert_eq!(first["service_days"], 30);
    assert!(!first.to_string().contains(authorization_code));

    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "POST",
                "/internal/deliveries/redemption-units/prepare",
                producer_token,
                Some(json!({
                    "tenant_id": "tenant_acme",
                    "project_id": "proj_core",
                    "provider": "chatgpt",
                    "owner_account_id": "wrong_owner",
                    "service_kind": "manual_browser_account",
                    "service_days": 30
                })),
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_owner_denied",
    )
    .await;

    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "POST",
                "/internal/deliveries/redemption-units/prepare",
                producer_token,
                Some(json!({
                    "tenant_id": "tenant_northstar",
                    "project_id": "proj_core",
                    "provider": "chatgpt",
                    "owner_account_id": owner_account_id,
                    "service_kind": "manual_browser_account",
                    "service_days": 30
                })),
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_scope_denied",
    )
    .await;

    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "POST",
                "/internal/deliveries/redemption-units/prepare",
                "prodtok_missing",
                Some(json!({
                    "tenant_id": "tenant_acme",
                    "project_id": "proj_core",
                    "provider": "chatgpt",
                    "owner_account_id": owner_account_id,
                    "service_kind": "manual_browser_account",
                    "service_days": 30
                })),
            ))
            .await
            .unwrap(),
        StatusCode::UNAUTHORIZED,
        "producer_token_invalid",
    )
    .await;

    let prepared = app
        .clone()
        .oneshot(request_with_bearer(
            "POST",
            "/internal/deliveries/redemption-units/prepare",
            producer_token,
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": owner_account_id,
                "service_kind": "manual_browser_account",
                "service_days": 30
            })),
        ))
        .await
        .unwrap();
    assert_eq!(prepared.status(), StatusCode::OK);
    let prepared = response_json(prepared).await;
    assert_eq!(prepared["data"]["owner_account_id"], owner_account_id);
    assert_eq!(prepared["data"]["unit_count"], 8);
}

#[tokio::test]
async fn producer_authorization_requires_effective_owner_for_single_prepare() {
    let (_state, _admin_cookie, app) = platform_admin_app().await;
    let producer_token =
        producer_token_for_owner_for_test(app.clone(), "acct_non_default_owner").await;

    assert_error(
        app.oneshot(request_with_bearer(
            "POST",
            "/internal/deliveries/prepare",
            &producer_token,
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "service_kind": "manual_browser_account",
                "service_days": 30
            })),
        ))
        .await
        .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_owner_denied",
    )
    .await;
}

#[tokio::test]
async fn producer_authorization_restricts_upload_batches_to_item_delivery_scope() {
    let (_state, _admin_cookie, app) = platform_admin_app().await;
    let owner_a = "acct_upload_scope_owner_a";
    let owner_b = "acct_upload_scope_owner_b";
    let token_a = producer_token_for_owner_for_test(app.clone(), owner_a).await;
    let token_b = producer_token_for_owner_for_test(app.clone(), owner_b).await;

    let prepared_b = app
        .clone()
        .oneshot(request_with_bearer(
            "POST",
            "/internal/deliveries/redemption-units/prepare",
            &token_b,
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": owner_b,
                "service_kind": "manual_browser_account",
                "service_days": 30
            })),
        ))
        .await
        .unwrap();
    assert_eq!(prepared_b.status(), StatusCode::OK);
    let prepared_b = response_json(prepared_b).await;
    let delivery_id = prepared_b["data"]["units"][0]["delivery"]["delivery"]["delivery_id"]
        .as_str()
        .unwrap();
    let payload_base64 = BASE64.encode(b"encrypted-owner-b-bundle");
    let upload_body = json!({
        "tenant_id": "tenant_acme",
        "project_id": "proj_core",
        "provider": "chatgpt",
        "source_file_name": "owner-b.jsonl",
        "items": [{
            "row_index": 1,
            "delivery_id": delivery_id,
            "artifact_kind": "browser_account_bundle",
            "file_name": "owner-b.hcbrowser",
            "content_type": "application/octet-stream",
            "payload_base64": payload_base64,
            "encryption_protocol": "delivery_account_bundle_v2",
            "encryption_version": "2",
            "secret_kind": "browser_file_unlock_code"
        }]
    });

    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "POST",
                "/internal/delivery-uploads",
                &token_a,
                Some(upload_body.clone()),
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_owner_denied",
    )
    .await;

    let queued = app
        .clone()
        .oneshot(request_with_bearer(
            "POST",
            "/internal/delivery-uploads",
            "test-internal-token",
            Some(upload_body),
        ))
        .await
        .unwrap();
    assert_eq!(queued.status(), StatusCode::ACCEPTED);
    let queued = response_json(queued).await;
    let batch_id = queued["data"]["batch_id"].as_str().unwrap();

    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "GET",
                &format!("/internal/delivery-uploads/{batch_id}/items"),
                &token_a,
                None,
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_owner_denied",
    )
    .await;
    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "POST",
                &format!("/internal/delivery-uploads/{batch_id}/process"),
                &token_a,
                None,
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_owner_denied",
    )
    .await;

    let owner_b_process = app
        .oneshot(request_with_bearer(
            "POST",
            &format!("/internal/delivery-uploads/{batch_id}/process"),
            &token_b,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(owner_b_process.status(), StatusCode::OK);
}

#[tokio::test]
async fn producer_authorization_rejects_expired_codes() {
    let (_state, _admin_cookie, app) = platform_admin_app().await;
    let response = app
        .clone()
        .oneshot(request_with_bearer(
            "POST",
            "/internal/producer-authorizations",
            "test-internal-token",
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": "acct_expired_producer_authorization",
                "service_kind": "manual_browser_account",
                "service_days": 30,
                "expires_at": "2020-01-01T00:00:00Z"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let issued = response_json(response).await;
    let authorization_code = issued["authorization_code"].as_str().unwrap();
    assert_error(
        app.oneshot(request(
            "POST",
            "/v1/producer-authorizations/redeem",
            None,
            Some(json!({ "authorization_code": authorization_code })),
        ))
        .await
        .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_authorization_expired",
    )
    .await;
}

#[tokio::test]
async fn producer_authorization_rotation_revokes_same_scope_codes_and_tokens() {
    let (_state, _admin_cookie, app) = platform_admin_app().await;
    let owner_account_id = "acct_rotating_producer_authorization";
    let first_issued = issue_producer_authorization_for_test(app.clone(), owner_account_id).await;
    let first_code = first_issued["authorization_code"].as_str().unwrap();
    let first = redeem_producer_authorization_for_test(app.clone(), first_code).await;
    let first_token = first["producer_token"].as_str().unwrap();
    let first_current = app
        .clone()
        .oneshot(request_with_bearer(
            "GET",
            "/v1/producer-authorizations/current-token",
            first_token,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(first_current.status(), StatusCode::OK);

    let second_issued = issue_producer_authorization_for_test(app.clone(), owner_account_id).await;
    let second_code = second_issued["authorization_code"].as_str().unwrap();
    assert_ne!(first_code, second_code);

    assert_error(
        app.clone()
            .oneshot(request(
                "POST",
                "/v1/producer-authorizations/redeem",
                None,
                Some(json!({ "authorization_code": first_code })),
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_authorization_revoked",
    )
    .await;
    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "GET",
                "/v1/producer-authorizations/current-token",
                first_token,
                None,
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_revoked",
    )
    .await;
    assert_error(
        app.clone()
            .oneshot(request_with_bearer(
                "POST",
                "/internal/deliveries/redemption-units/prepare",
                first_token,
                Some(json!({
                    "tenant_id": "tenant_acme",
                    "project_id": "proj_core",
                    "provider": "chatgpt",
                    "owner_account_id": owner_account_id,
                    "service_kind": "manual_browser_account",
                    "service_days": 30
                })),
            ))
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
        "producer_token_revoked",
    )
    .await;

    let second = redeem_producer_authorization_for_test(app.clone(), second_code).await;
    let second_token = second["producer_token"].as_str().unwrap();
    let prepared = app
        .clone()
        .oneshot(request_with_bearer(
            "POST",
            "/internal/deliveries/redemption-units/prepare",
            second_token,
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": owner_account_id,
                "service_kind": "manual_browser_account",
                "service_days": 30
            })),
        ))
        .await
        .unwrap();
    assert_eq!(prepared.status(), StatusCode::OK);
}

#[tokio::test]
async fn producer_authorization_rotation_does_not_revoke_other_owner_scope() {
    let (_state, _admin_cookie, app) = platform_admin_app().await;
    let first_issued =
        issue_producer_authorization_for_test(app.clone(), "acct_rotation_owner_a").await;
    let first_code = first_issued["authorization_code"].as_str().unwrap();
    let first = redeem_producer_authorization_for_test(app.clone(), first_code).await;
    let first_token = first["producer_token"].as_str().unwrap();

    let _second_issued =
        issue_producer_authorization_for_test(app.clone(), "acct_rotation_owner_b").await;
    let still_current = app
        .clone()
        .oneshot(request_with_bearer(
            "GET",
            "/v1/producer-authorizations/current-token",
            first_token,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(still_current.status(), StatusCode::OK);
}

#[tokio::test]
async fn delivery_redemption_units_prepare_audit_and_redeem_are_owner_scoped() {
    let (_state, admin_cookie, app) = platform_admin_app().await;
    let owner_account_id = "acct_redemption_units_owner";
    let prepared = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/deliveries/redemption-units/prepare",
            Some(&admin_cookie),
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": owner_account_id,
                "customer_label": "Eight unit owner",
                "service_kind": "manual_browser_account",
                "service_days": 30
            })),
        ))
        .await
        .unwrap();
    assert_eq!(prepared.status(), StatusCode::OK);
    let prepared = response_json(prepared).await;
    assert_eq!(prepared["data"]["owner_account_id"], owner_account_id);
    assert_eq!(prepared["data"]["unit_count"], 8);
    assert_eq!(prepared["data"]["capacity"], u32::MAX);
    let first_batch_id = prepared["data"]["batch_id"].as_str().unwrap().to_string();
    let units = prepared["data"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 8);

    let mut delivery_ids = std::collections::HashSet::new();
    let mut red_codes = std::collections::HashSet::new();
    let mut brw_codes = std::collections::HashSet::new();
    for unit in units {
        let delivery_id = unit["delivery"]["delivery"]["delivery_id"]
            .as_str()
            .unwrap();
        let red = unit["one_time_codes"]["redemption_code"].as_str().unwrap();
        let brw = unit["one_time_codes"]["browser_file_unlock_code"]
            .as_str()
            .unwrap();
        assert!(delivery_ids.insert(delivery_id.to_string()));
        assert!(red_codes.insert(red.to_string()));
        assert!(brw_codes.insert(brw.to_string()));
        assert!(red.starts_with("ku0-red-v2-"));
        assert!(brw.starts_with("ku0-brw-v2-"));
        assert!(!unit["delivery"].to_string().contains(red));
        assert!(!unit["delivery"].to_string().contains(brw));
    }

    let second_prepared = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/deliveries/redemption-units/prepare",
            Some(&admin_cookie),
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": owner_account_id,
                "service_days": 30
            })),
        ))
        .await
        .unwrap();
    assert_eq!(second_prepared.status(), StatusCode::OK);
    let second_prepared = response_json(second_prepared).await;
    assert_ne!(
        second_prepared["data"]["batch_id"].as_str().unwrap(),
        first_batch_id
    );
    assert_eq!(
        second_prepared["data"]["owner_account_id"],
        owner_account_id
    );
    assert_eq!(second_prepared["data"]["unit_count"], 8);
    assert_eq!(second_prepared["data"]["capacity"], u32::MAX);
    let second_units = second_prepared["data"]["units"].as_array().unwrap();
    assert_eq!(second_units.len(), 8);
    for unit in second_units {
        let delivery_id = unit["delivery"]["delivery"]["delivery_id"]
            .as_str()
            .unwrap();
        let red = unit["one_time_codes"]["redemption_code"].as_str().unwrap();
        let brw = unit["one_time_codes"]["browser_file_unlock_code"]
            .as_str()
            .unwrap();
        assert!(delivery_ids.insert(delivery_id.to_string()));
        assert!(red_codes.insert(red.to_string()));
        assert!(brw_codes.insert(brw.to_string()));
        assert!(red.starts_with("ku0-red-v2-"));
        assert!(brw.starts_with("ku0-brw-v2-"));
        assert!(!unit["delivery"].to_string().contains(red));
        assert!(!unit["delivery"].to_string().contains(brw));
    }

    let owner_scoped_single = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/deliveries/prepare",
            Some(&admin_cookie),
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "owner_account_id": owner_account_id,
                "service_days": 30
            })),
        ))
        .await
        .unwrap();
    assert_eq!(owner_scoped_single.status(), StatusCode::OK);
    let owner_scoped_single = response_json(owner_scoped_single).await;
    let single_delivery_id = owner_scoped_single["data"]["delivery"]["delivery_id"]
        .as_str()
        .unwrap();
    let single_red = owner_scoped_single["one_time_codes"]["redemption_code"]
        .as_str()
        .unwrap();
    let single_brw = owner_scoped_single["one_time_codes"]["browser_file_unlock_code"]
        .as_str()
        .unwrap();
    assert!(delivery_ids.insert(single_delivery_id.to_string()));
    assert!(red_codes.insert(single_red.to_string()));
    assert!(brw_codes.insert(single_brw.to_string()));

    let inventory = app
        .clone()
        .oneshot(request(
            "GET",
            &format!(
                "/v1/delivery-redemption-inventory?tenant_id=tenant_acme&project_id=proj_core&owner_account_id={owner_account_id}"
            ),
            Some(&admin_cookie),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(inventory.status(), StatusCode::OK);
    let inventory = response_json(inventory).await;
    assert_eq!(inventory["data"]["owners"][0]["active_count"], 17);
    assert_eq!(inventory["data"]["owners"][0]["capacity"], u32::MAX);
    assert_eq!(inventory["data"]["owners"][0]["status"], "issued");
    let inventory_text = inventory.to_string();
    for red in &red_codes {
        assert!(!inventory_text.contains(red));
    }
    for brw in &brw_codes {
        assert!(!inventory_text.contains(brw));
    }

    let first = &units[0];
    let first_delivery_id = first["delivery"]["delivery"]["delivery_id"]
        .as_str()
        .unwrap();
    let first_red = first["one_time_codes"]["redemption_code"].as_str().unwrap();
    let first_brw = first["one_time_codes"]["browser_file_unlock_code"]
        .as_str()
        .unwrap();
    upload_artifact_for_test(
        app.clone(),
        &admin_cookie,
        first_delivery_id,
        b"encrypted-redemption-unit",
        "unit.hcbrowser",
    )
    .await;
    let activation = redeem_delivery_public_for_test(app.clone(), first_red).await;
    assert_eq!(
        activation["restore"]["artifact_import_secret"],
        json!(first_brw)
    );
    assert!(!activation.to_string().contains(first_red));

    let inventory_after = app
        .oneshot(request(
            "GET",
            &format!(
                "/v1/delivery-redemption-inventory?tenant_id=tenant_acme&project_id=proj_core&owner_account_id={owner_account_id}"
            ),
            Some(&admin_cookie),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(inventory_after.status(), StatusCode::OK);
    let inventory_after = response_json(inventory_after).await;
    assert_eq!(inventory_after["data"]["owners"][0]["active_count"], 16);
    assert_eq!(inventory_after["data"]["owners"][0]["capacity"], u32::MAX);
    assert_eq!(inventory_after["data"]["owners"][0]["status"], "issued");
}

#[tokio::test]
async fn delivery_upload_batches_require_browser_unlock_metadata() {
    let (_state, admin_cookie, app) = platform_admin_app().await;
    let prepared = prepare_delivery_for_test(app.clone(), &admin_cookie).await;
    let delivery_id = prepared["data"]["delivery"]["delivery_id"]
        .as_str()
        .unwrap();

    assert_error(
        app.clone()
            .oneshot(request(
                "POST",
                "/v1/delivery-uploads",
                Some(&admin_cookie),
                Some(json!({
                    "tenant_id": "tenant_acme",
                    "project_id": "proj_core",
                    "provider": "chatgpt",
                    "source_file_name": "missing-metadata.jsonl",
                    "items": [{
                        "delivery_id": delivery_id,
                        "row_index": 1,
                        "payload_base64": BASE64.encode(b"encrypted")
                    }]
                })),
            ))
            .await
            .unwrap(),
        StatusCode::BAD_REQUEST,
        "delivery_artifact_encryption_metadata_required",
    )
    .await;

    assert_error(
        app.oneshot(request(
            "POST",
            "/v1/delivery-uploads",
            Some(&admin_cookie),
            Some(json!({
                "tenant_id": "tenant_acme",
                "project_id": "proj_core",
                "provider": "chatgpt",
                "source_file_name": "wrong-secret-kind.jsonl",
                "items": [{
                    "delivery_id": delivery_id,
                    "row_index": 1,
                    "encryption_protocol": "delivery_account_bundle_v2",
                    "encryption_version": "2",
                    "secret_kind": "redemption_code",
                    "payload_base64": BASE64.encode(b"encrypted")
                }]
            })),
        ))
        .await
        .unwrap(),
        StatusCode::BAD_REQUEST,
        "delivery_artifact_secret_kind_unsupported",
    )
    .await;
}
