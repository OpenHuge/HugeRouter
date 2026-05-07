use super::*;
use crate::BASE64;
use base64::Engine as _;

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
    assert_eq!(prepared["data"]["capacity"], 8);
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

    assert_error(
        app.clone()
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
            .unwrap(),
        StatusCode::CONFLICT,
        "delivery_redemption_units_already_issued",
    )
    .await;

    assert_error(
        app.clone()
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
            .unwrap(),
        StatusCode::CONFLICT,
        "delivery_redemption_units_already_issued",
    )
    .await;

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
    assert_eq!(inventory["data"]["owners"][0]["active_count"], 8);
    assert_eq!(inventory["data"]["owners"][0]["capacity"], 8);
    assert_eq!(inventory["data"]["owners"][0]["status"], "full");
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
    assert_eq!(inventory_after["data"]["owners"][0]["active_count"], 7);
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
