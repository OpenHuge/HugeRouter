#![allow(clippy::wildcard_imports)]

use super::*;
use crate::examples::sample_delivery_projection;

pub fn sample_prepare_delivery_redemption_units_request() -> PrepareDeliveryRedemptionUnitsRequest {
    PrepareDeliveryRedemptionUnitsRequest {
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        provider: "chatgpt".to_string(),
        owner_account_id: "acct_acme_owner".to_string(),
        customer_label: Some("Acme May browser handoff".to_string()),
        service_kind: Some("manual_browser_account".to_string()),
        service_days: 30,
        starts_at: Some("2026-05-05T10:00:00Z".to_string()),
        code_expires_at: Some("2026-06-04T10:00:00Z".to_string()),
    }
}

pub fn sample_prepare_delivery_redemption_units_response() -> PrepareDeliveryRedemptionUnitsResponse
{
    let units = (1..=8)
        .map(|index| {
            let mut delivery = sample_delivery_projection();
            delivery.delivery.delivery_id = format!("delivery_1000{index}");
            delivery.delivery.redemption_batch_id = Some("dlvbatch_10000".to_string());
            delivery
                .entitlement
                .delivery_id
                .clone_from(&delivery.delivery.delivery_id);
            delivery.entitlement.entitlement_id = format!("dlvent_delivery_1000{index}");
            for code in &mut delivery.codes {
                code.delivery_id.clone_from(&delivery.delivery.delivery_id);
            }
            delivery.codes[0].code_id = format!("dlvcode_delivery_1000{index}_redemption");
            delivery.codes[0].code_last_four = format!("{index}-7b");
            delivery.codes[1].code_id = format!("dlvcode_delivery_1000{index}_browser_unlock");
            delivery.codes[1].code_last_four = format!("{index}-76");
            DeliveryRedemptionUnit {
                unit_id: format!("dlvunit_dlvbatch_10000_{index}"),
                index,
                delivery,
                one_time_codes: DeliveryOneTimeCodes {
                    redemption_code: format!("ku0-red-v2-260505-a1b2-c3d4e5f6g7h{index}-{index}b"),
                    browser_file_unlock_code: format!(
                        "ku0-brw-v2-260505-j9k0-l1m2n3p4q5r{index}-{index}6"
                    ),
                },
            }
        })
        .collect();
    PrepareDeliveryRedemptionUnitsResponse {
        data: DeliveryRedemptionUnitsBatch {
            batch_id: "dlvbatch_10000".to_string(),
            owner_account_id: "acct_acme_owner".to_string(),
            unit_count: 8,
            capacity: 8,
            units,
        },
    }
}

pub fn sample_delivery_redemption_inventory_response() -> DeliveryRedemptionInventoryResponse {
    DeliveryRedemptionInventoryResponse {
        data: DeliveryRedemptionInventory {
            owners: vec![DeliveryRedemptionInventoryOwner {
                tenant_id: TenantId::parse("tenant_acme").unwrap(),
                project_id: ProjectId::parse("proj_core").unwrap(),
                owner_account_id: "acct_acme_owner".to_string(),
                active_count: 8,
                capacity: 8,
                status: "full".to_string(),
            }],
            units: (1..=8)
                .map(|index| DeliveryRedemptionInventoryUnit {
                    tenant_id: TenantId::parse("tenant_acme").unwrap(),
                    project_id: ProjectId::parse("proj_core").unwrap(),
                    owner_account_id: "acct_acme_owner".to_string(),
                    redemption_batch_id: Some("dlvbatch_10000".to_string()),
                    delivery_id: format!("delivery_1000{index}"),
                    code_id: format!("dlvcode_delivery_1000{index}_redemption"),
                    code_prefix: "ku0-red-v2-260505...".to_string(),
                    code_last_four: format!("{index}-7b"),
                    status: "active".to_string(),
                    expires_at: "2026-06-04T10:00:00Z".to_string(),
                    used_at: None,
                    revoked_at: None,
                    created_at: "2026-05-05T10:00:00Z".to_string(),
                    updated_at: "2026-05-05T10:00:00Z".to_string(),
                })
                .collect(),
        },
    }
}
