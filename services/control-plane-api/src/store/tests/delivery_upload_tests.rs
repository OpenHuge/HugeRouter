use super::*;

#[test]
fn upload_batch_item_hydration_restores_postgres_ciphertext_column() {
    let item = super::super::DeliveryUploadBatchItemRecord {
        item_id: "dlvupitem_test_1".to_string(),
        batch_id: "dlvup_test".to_string(),
        row_index: 1,
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        delivery_id: "delivery_test".to_string(),
        artifact_id: None,
        status: super::super::DELIVERY_UPLOAD_ITEM_STATUS_PENDING.to_string(),
        artifact_kind: "browser_account_bundle".to_string(),
        file_name: Some("hugecode-browser-data.hcbrowser".to_string()),
        content_type: "application/octet-stream".to_string(),
        carrier_valid_until: None,
        encryption_protocol: super::super::DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_PROTOCOL_V2
            .to_string(),
        encryption_version: super::super::DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_VERSION_V2.to_string(),
        secret_kind: super::super::DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK.to_string(),
        payload_sha256: "sha256:payload".to_string(),
        size_bytes: 20,
        error_code: None,
        error_message: None,
        created_at: "2026-05-06T00:00:00Z".to_string(),
        updated_at: "2026-05-06T00:00:00Z".to_string(),
        version: 1,
        ciphertext: Vec::new(),
    };

    let hydrated =
        super::super::hydrate_upload_batch_item_ciphertext(item, b"encrypted-hcbrowser".to_vec());

    assert_eq!(hydrated.ciphertext, b"encrypted-hcbrowser");
}
