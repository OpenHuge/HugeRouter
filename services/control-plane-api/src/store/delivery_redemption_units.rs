use super::{
    DELIVERY_CODE_STATUS_ACTIVE, DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK,
    DELIVERY_CODE_TYPE_REDEMPTION, DeliveryCodeRecord, DeliveryPrepareResult, DeliveryRecord,
    PostgresStore, PreparedDeliveryRecords, StoreMode, delivery_projection, delivery_secret_key,
    now_rfc3339,
};
use anyhow::{Result, anyhow};
use core_domain::{ProjectId, TenantId};
use protocol_ir::{
    DeliveryOneTimeCodes, DeliveryRedemptionInventory, DeliveryRedemptionInventoryOwner,
    DeliveryRedemptionInventoryResponse, DeliveryRedemptionInventoryUnit, DeliveryRedemptionUnit,
    DeliveryRedemptionUnitsBatch, PrepareDeliveryRedemptionUnitsResponse,
};
use sqlx::{Postgres, Row, types::Json};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum DeliveryRedemptionUnitsPrepareResult {
    Prepared(Box<PrepareDeliveryRedemptionUnitsResponse>),
    OwnerAlreadyIssued {
        active_count: usize,
        capacity: usize,
    },
}

#[derive(Debug, Clone)]
pub struct DeliveryRedemptionUnitDraft {
    pub records: PreparedDeliveryRecords,
    pub redemption_code: String,
    pub browser_file_unlock_code: String,
}

pub fn owner_redemption_capacity_blocker_for_memory(
    deliveries: &[DeliveryRecord],
    codes: &[DeliveryCodeRecord],
    prepared: &PreparedDeliveryRecords,
) -> Option<DeliveryPrepareResult> {
    if !prepared.enforce_owner_redemption_capacity {
        return None;
    }
    let active_count = active_redemption_code_count_for_owner(
        deliveries,
        codes,
        &prepared.delivery.tenant_id,
        &prepared.delivery.project_id,
        &prepared.delivery.owner_account_id,
    );
    (active_count > 0).then_some(DeliveryPrepareResult::OwnerAlreadyIssued {
        active_count,
        capacity: crate::delivery_redemption_policy::MAX_ACTIVE_REDEMPTION_UNITS_PER_OWNER,
    })
}

pub async fn owner_redemption_capacity_blocker_for_postgres(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    prepared: &PreparedDeliveryRecords,
) -> Result<Option<DeliveryPrepareResult>> {
    if !prepared.enforce_owner_redemption_capacity {
        return Ok(None);
    }
    sqlx::query(
        "INSERT INTO delivery_redemption_owner_locks
            (tenant_id, project_id, owner_account_id, created_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (tenant_id, project_id, owner_account_id) DO NOTHING",
    )
    .bind(prepared.delivery.tenant_id.as_str())
    .bind(prepared.delivery.project_id.as_str())
    .bind(&prepared.delivery.owner_account_id)
    .bind(&prepared.delivery.created_at)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "SELECT owner_account_id
           FROM delivery_redemption_owner_locks
          WHERE tenant_id = $1 AND project_id = $2 AND owner_account_id = $3
          FOR UPDATE",
    )
    .bind(prepared.delivery.tenant_id.as_str())
    .bind(prepared.delivery.project_id.as_str())
    .bind(&prepared.delivery.owner_account_id)
    .execute(&mut **tx)
    .await?;
    let active_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
           FROM delivery_codes codes
           JOIN deliveries deliveries ON deliveries.delivery_id = codes.delivery_id
          WHERE deliveries.tenant_id = $1
            AND deliveries.project_id = $2
            AND deliveries.owner_account_id = $3
            AND codes.code_type = $4
            AND codes.status = $5
            AND codes.expires_at > $6",
    )
    .bind(prepared.delivery.tenant_id.as_str())
    .bind(prepared.delivery.project_id.as_str())
    .bind(&prepared.delivery.owner_account_id)
    .bind(DELIVERY_CODE_TYPE_REDEMPTION)
    .bind(DELIVERY_CODE_STATUS_ACTIVE)
    .bind(now_rfc3339())
    .fetch_one(&mut **tx)
    .await?;
    Ok(
        (active_count > 0).then_some(DeliveryPrepareResult::OwnerAlreadyIssued {
            active_count: usize::try_from(active_count).unwrap_or(usize::MAX),
            capacity: crate::delivery_redemption_policy::MAX_ACTIVE_REDEMPTION_UNITS_PER_OWNER,
        }),
    )
}

impl StoreMode {
    pub(crate) async fn prepare_delivery_redemption_units(
        &self,
        batch_id: String,
        owner_account_id: String,
        units: Vec<DeliveryRedemptionUnitDraft>,
    ) -> Result<DeliveryRedemptionUnitsPrepareResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let active_count = active_redemption_code_count_for_owner(
                    &store.deliveries,
                    &store.delivery_codes,
                    &units[0].records.delivery.tenant_id,
                    &units[0].records.delivery.project_id,
                    &owner_account_id,
                );
                if active_count > 0 {
                    return Ok(DeliveryRedemptionUnitsPrepareResult::OwnerAlreadyIssued {
                        active_count,
                        capacity:
                            crate::delivery_redemption_policy::MAX_ACTIVE_REDEMPTION_UNITS_PER_OWNER,
                    });
                }
                for unit in &units {
                    store.deliveries.push(unit.records.delivery.clone());
                    store.delivery_codes.extend(unit.records.codes.clone());
                    store.delivery_secret_plaintexts.insert(
                        delivery_secret_key(
                            &unit.records.delivery.delivery_id,
                            DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK,
                        ),
                        unit.browser_file_unlock_code.clone(),
                    );
                    store
                        .delivery_entitlements
                        .push(unit.records.entitlement.clone());
                }
                Ok(DeliveryRedemptionUnitsPrepareResult::Prepared(Box::new(
                    delivery_redemption_units_response(batch_id, owner_account_id, &units),
                )))
            }
            Self::Postgres(store) => {
                store
                    .prepare_delivery_redemption_units(batch_id, owner_account_id, &units)
                    .await
            }
        }
    }

    pub(crate) async fn list_delivery_redemption_inventory(
        &self,
        tenant_id: &TenantId,
        project_id: &ProjectId,
        owner_account_id: Option<&str>,
    ) -> Result<DeliveryRedemptionInventoryResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(delivery_redemption_inventory_response(
                    &store.deliveries,
                    &store.delivery_codes,
                    tenant_id,
                    project_id,
                    owner_account_id,
                ))
            }
            Self::Postgres(store) => {
                store
                    .list_delivery_redemption_inventory(tenant_id, project_id, owner_account_id)
                    .await
            }
        }
    }
}

impl PostgresStore {
    async fn prepare_delivery_redemption_units(
        &self,
        batch_id: String,
        owner_account_id: String,
        units: &[DeliveryRedemptionUnitDraft],
    ) -> Result<DeliveryRedemptionUnitsPrepareResult> {
        let first = units
            .first()
            .ok_or_else(|| anyhow!("delivery_redemption_units_required"))?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO delivery_redemption_owner_locks
                (tenant_id, project_id, owner_account_id, created_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (tenant_id, project_id, owner_account_id) DO NOTHING",
        )
        .bind(first.records.delivery.tenant_id.as_str())
        .bind(first.records.delivery.project_id.as_str())
        .bind(&owner_account_id)
        .bind(&first.records.delivery.created_at)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "SELECT owner_account_id
               FROM delivery_redemption_owner_locks
              WHERE tenant_id = $1 AND project_id = $2 AND owner_account_id = $3
              FOR UPDATE",
        )
        .bind(first.records.delivery.tenant_id.as_str())
        .bind(first.records.delivery.project_id.as_str())
        .bind(&owner_account_id)
        .execute(&mut *tx)
        .await?;
        let active_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
               FROM delivery_codes codes
               JOIN deliveries deliveries ON deliveries.delivery_id = codes.delivery_id
              WHERE deliveries.tenant_id = $1
                AND deliveries.project_id = $2
                AND deliveries.owner_account_id = $3
                AND codes.code_type = $4
                AND codes.status = $5
                AND codes.expires_at > $6",
        )
        .bind(first.records.delivery.tenant_id.as_str())
        .bind(first.records.delivery.project_id.as_str())
        .bind(&owner_account_id)
        .bind(DELIVERY_CODE_TYPE_REDEMPTION)
        .bind(DELIVERY_CODE_STATUS_ACTIVE)
        .bind(now_rfc3339())
        .fetch_one(&mut *tx)
        .await?;
        if active_count > 0 {
            tx.rollback().await?;
            return Ok(DeliveryRedemptionUnitsPrepareResult::OwnerAlreadyIssued {
                active_count: usize::try_from(active_count).unwrap_or(usize::MAX),
                capacity: crate::delivery_redemption_policy::MAX_ACTIVE_REDEMPTION_UNITS_PER_OWNER,
            });
        }
        for unit in units {
            sqlx::query(
                "INSERT INTO deliveries
                    (delivery_id, tenant_id, project_id, owner_account_id, redemption_batch_id, provider, status, operator_id, payload, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(&unit.records.delivery.delivery_id)
            .bind(unit.records.delivery.tenant_id.as_str())
            .bind(unit.records.delivery.project_id.as_str())
            .bind(&unit.records.delivery.owner_account_id)
            .bind(&unit.records.delivery.redemption_batch_id)
            .bind(&unit.records.delivery.provider)
            .bind(&unit.records.delivery.status)
            .bind(&unit.records.delivery.operator_id)
            .bind(Json(&unit.records.delivery))
            .bind(&unit.records.delivery.created_at)
            .bind(&unit.records.delivery.updated_at)
            .execute(&mut *tx)
            .await?;

            for code in &unit.records.codes {
                sqlx::query(
                    "INSERT INTO delivery_codes
                        (code_id, delivery_id, code_type, code_hash, status, expires_at, payload, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                )
                .bind(&code.code_id)
                .bind(&code.delivery_id)
                .bind(&code.code_type)
                .bind(&code.code_hash)
                .bind(&code.status)
                .bind(&code.expires_at)
                .bind(Json(code))
                .bind(&code.created_at)
                .bind(&code.updated_at)
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query(
                "INSERT INTO delivery_secret_plaintexts
                    (delivery_id, code_type, secret_plaintext, created_at)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(&unit.records.delivery.delivery_id)
            .bind(DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK)
            .bind(&unit.browser_file_unlock_code)
            .bind(&unit.records.delivery.created_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO delivery_entitlements
                    (entitlement_id, delivery_id, status, ends_at, payload, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&unit.records.entitlement.entitlement_id)
            .bind(&unit.records.entitlement.delivery_id)
            .bind(&unit.records.entitlement.status)
            .bind(&unit.records.entitlement.ends_at)
            .bind(Json(&unit.records.entitlement))
            .bind(&unit.records.entitlement.created_at)
            .bind(&unit.records.entitlement.updated_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(DeliveryRedemptionUnitsPrepareResult::Prepared(Box::new(
            delivery_redemption_units_response(batch_id, owner_account_id, units),
        )))
    }

    async fn list_delivery_redemption_inventory(
        &self,
        tenant_id: &TenantId,
        project_id: &ProjectId,
        owner_account_id: Option<&str>,
    ) -> Result<DeliveryRedemptionInventoryResponse> {
        let rows = sqlx::query(
            "SELECT deliveries.payload AS delivery_payload, codes.payload AS code_payload
               FROM delivery_codes codes
               JOIN deliveries deliveries ON deliveries.delivery_id = codes.delivery_id
              WHERE deliveries.tenant_id = $1
                AND deliveries.project_id = $2
                AND codes.code_type = $3
                AND ($4::TEXT IS NULL OR deliveries.owner_account_id = $4)
              ORDER BY deliveries.owner_account_id, codes.created_at",
        )
        .bind(tenant_id.as_str())
        .bind(project_id.as_str())
        .bind(DELIVERY_CODE_TYPE_REDEMPTION)
        .bind(owner_account_id)
        .fetch_all(&self.pool)
        .await?;
        let mut deliveries = Vec::new();
        let mut codes = Vec::new();
        for row in rows {
            deliveries.push(row.get::<Json<DeliveryRecord>, _>("delivery_payload").0);
            codes.push(row.get::<Json<DeliveryCodeRecord>, _>("code_payload").0);
        }
        Ok(delivery_redemption_inventory_response(
            &deliveries,
            &codes,
            tenant_id,
            project_id,
            owner_account_id,
        ))
    }
}

fn delivery_redemption_units_response(
    batch_id: String,
    owner_account_id: String,
    units: &[DeliveryRedemptionUnitDraft],
) -> PrepareDeliveryRedemptionUnitsResponse {
    PrepareDeliveryRedemptionUnitsResponse {
        data: DeliveryRedemptionUnitsBatch {
            batch_id,
            owner_account_id,
            unit_count: u32::try_from(units.len()).unwrap_or(u32::MAX),
            capacity: u32::try_from(
                crate::delivery_redemption_policy::MAX_ACTIVE_REDEMPTION_UNITS_PER_OWNER,
            )
            .unwrap_or(u32::MAX),
            units: units
                .iter()
                .enumerate()
                .map(|(index, unit)| DeliveryRedemptionUnit {
                    unit_id: unit.records.delivery.delivery_id.clone(),
                    index: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    delivery: delivery_projection(
                        &unit.records.delivery,
                        &unit.records.codes,
                        &unit.records.entitlement,
                    ),
                    one_time_codes: DeliveryOneTimeCodes {
                        redemption_code: unit.redemption_code.clone(),
                        browser_file_unlock_code: unit.browser_file_unlock_code.clone(),
                    },
                })
                .collect(),
        },
    }
}

pub fn active_redemption_code_count_for_owner(
    deliveries: &[DeliveryRecord],
    codes: &[DeliveryCodeRecord],
    tenant_id: &TenantId,
    project_id: &ProjectId,
    owner_account_id: &str,
) -> usize {
    codes
        .iter()
        .filter(|code| {
            code.code_type == DELIVERY_CODE_TYPE_REDEMPTION
                && code.effective_status() == DELIVERY_CODE_STATUS_ACTIVE
                && deliveries.iter().any(|delivery| {
                    delivery.delivery_id == code.delivery_id
                        && delivery.tenant_id == *tenant_id
                        && delivery.project_id == *project_id
                        && delivery.owner_account_id == owner_account_id
                })
        })
        .count()
}

fn delivery_redemption_inventory_response(
    deliveries: &[DeliveryRecord],
    codes: &[DeliveryCodeRecord],
    tenant_id: &TenantId,
    project_id: &ProjectId,
    owner_account_id: Option<&str>,
) -> DeliveryRedemptionInventoryResponse {
    let mut units = Vec::new();
    let mut active_counts = BTreeMap::<String, usize>::new();
    for code in codes
        .iter()
        .filter(|code| code.code_type == DELIVERY_CODE_TYPE_REDEMPTION)
    {
        let Some(delivery) = deliveries.iter().find(|delivery| {
            delivery.delivery_id == code.delivery_id
                && delivery.tenant_id == *tenant_id
                && delivery.project_id == *project_id
                && owner_account_id.is_none_or(|owner| delivery.owner_account_id == owner)
        }) else {
            continue;
        };
        let status = code.effective_status();
        if status == DELIVERY_CODE_STATUS_ACTIVE {
            *active_counts
                .entry(delivery.owner_account_id.clone())
                .or_insert(0) += 1;
        }
        units.push(DeliveryRedemptionInventoryUnit {
            tenant_id: delivery.tenant_id.clone(),
            project_id: delivery.project_id.clone(),
            owner_account_id: delivery.owner_account_id.clone(),
            redemption_batch_id: delivery.redemption_batch_id.clone(),
            delivery_id: code.delivery_id.clone(),
            code_id: code.code_id.clone(),
            code_prefix: code.code_prefix.clone(),
            code_last_four: code.code_last_four.clone(),
            status,
            expires_at: code.expires_at.clone(),
            used_at: code.used_at.clone(),
            revoked_at: code.revoked_at.clone(),
            created_at: code.created_at.clone(),
            updated_at: code.updated_at.clone(),
        });
    }
    for unit in &units {
        active_counts
            .entry(unit.owner_account_id.clone())
            .or_insert(0);
    }
    DeliveryRedemptionInventoryResponse {
        data: DeliveryRedemptionInventory {
            owners: active_counts
                .into_iter()
                .map(|(owner, active_count)| DeliveryRedemptionInventoryOwner {
                    tenant_id: tenant_id.clone(),
                    project_id: project_id.clone(),
                    owner_account_id: owner,
                    active_count: u32::try_from(active_count).unwrap_or(u32::MAX),
                    capacity: u32::try_from(
                        crate::delivery_redemption_policy::MAX_ACTIVE_REDEMPTION_UNITS_PER_OWNER,
                    )
                    .unwrap_or(u32::MAX),
                    status: crate::delivery_redemption_policy::inventory_status(active_count)
                        .to_string(),
                })
                .collect(),
            units,
        },
    }
}
