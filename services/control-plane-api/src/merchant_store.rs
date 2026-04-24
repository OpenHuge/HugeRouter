use anyhow::{Context, Result, anyhow};
use core_domain::{
    CardProduct, ConfigSnapshot, MerchantShop, NormalizedRequestSummary, RedactionTier,
    RelayEvaluation, ReplayCapsule, ReplayCapsuleId, RouteReceiptId, TenantId, TrialConnection,
    UpstreamErrorSummary,
};
use sqlx::{Row, types::Json};

use crate::merchant_replay::{
    build_merchant_replay_route_receipt, build_relay_evaluation, next_id_suffix,
    replay_upstream_error_code,
};
use crate::store::{
    MerchantWorkspaceEnvelope, MerchantWorkspaceResponse, PostgresStore, ReplayCapsuleResponse,
    now_rfc3339,
};

impl PostgresStore {
    pub(crate) async fn get_merchant_workspace(
        &self,
        tenant_id: &TenantId,
    ) -> Result<MerchantWorkspaceEnvelope> {
        let shops = self
            .list_json_payloads::<MerchantShop>("merchant_shops", tenant_id)
            .await?;
        let card_products = self
            .list_json_payloads::<CardProduct>("card_products", tenant_id)
            .await?;
        let trial_connections = self
            .list_json_payloads::<TrialConnection>("trial_connections", tenant_id)
            .await?;
        let mut recent_evaluations = self
            .list_json_payloads::<RelayEvaluation>("relay_evaluations", tenant_id)
            .await?;
        recent_evaluations.sort_by(|left, right| right.created_at.cmp(&left.created_at));

        Ok(MerchantWorkspaceEnvelope {
            data: MerchantWorkspaceResponse {
                merchant_enabled: !shops.is_empty(),
                tenant_id: tenant_id.clone(),
                shops,
                card_products,
                trial_connections,
                recent_evaluations,
            },
        })
    }

    pub(crate) async fn create_merchant_shop(&self, shop: &MerchantShop) -> Result<MerchantShop> {
        shop.validate()?;
        let duplicate = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM merchant_shops
             WHERE merchant_shop_id = $1 OR (tenant_id = $2 AND slug = $3)",
        )
        .bind(shop.merchant_shop_id.as_str())
        .bind(shop.tenant_id.as_str())
        .bind(&shop.slug)
        .fetch_one(&self.pool)
        .await?;
        if duplicate > 0 {
            return Err(anyhow!("merchant_shop_already_exists"));
        }

        sqlx::query(
            "INSERT INTO merchant_shops (merchant_shop_id, tenant_id, slug, payload)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(shop.merchant_shop_id.as_str())
        .bind(shop.tenant_id.as_str())
        .bind(&shop.slug)
        .bind(Json(shop))
        .execute(&self.pool)
        .await?;
        Ok(shop.clone())
    }

    pub(crate) async fn create_card_product(&self, product: &CardProduct) -> Result<CardProduct> {
        product.validate()?;
        let duplicate = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM card_products WHERE card_product_id = $1",
        )
        .bind(product.card_product_id.as_str())
        .fetch_one(&self.pool)
        .await?;
        if duplicate > 0 {
            return Err(anyhow!("card_product_already_exists"));
        }

        let shop_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM merchant_shops WHERE merchant_shop_id = $1 AND tenant_id = $2",
        )
        .bind(product.merchant_shop_id.as_str())
        .bind(product.tenant_id.as_str())
        .fetch_one(&self.pool)
        .await?;
        if shop_exists == 0 {
            return Err(anyhow!("merchant_shop_not_found"));
        }

        sqlx::query(
            "INSERT INTO card_products (card_product_id, tenant_id, merchant_shop_id, payload)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(product.card_product_id.as_str())
        .bind(product.tenant_id.as_str())
        .bind(product.merchant_shop_id.as_str())
        .bind(Json(product))
        .execute(&self.pool)
        .await?;
        Ok(product.clone())
    }

    pub(crate) async fn create_trial_connection(
        &self,
        connection: &TrialConnection,
    ) -> Result<TrialConnection> {
        connection.validate()?;
        let duplicate = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM trial_connections WHERE trial_connection_id = $1",
        )
        .bind(connection.trial_connection_id.as_str())
        .fetch_one(&self.pool)
        .await?;
        if duplicate > 0 {
            return Err(anyhow!("trial_connection_already_exists"));
        }

        sqlx::query(
            "INSERT INTO trial_connections (trial_connection_id, tenant_id, payload)
             VALUES ($1, $2, $3)",
        )
        .bind(connection.trial_connection_id.as_str())
        .bind(connection.tenant_id.as_str())
        .bind(Json(connection))
        .execute(&self.pool)
        .await?;
        Ok(connection.clone())
    }

    pub(crate) async fn create_relay_evaluation(
        &self,
        tenant_id: &TenantId,
        trial_connection_id: &str,
    ) -> Result<RelayEvaluation> {
        let connection = self
            .get_trial_connection(tenant_id, trial_connection_id)
            .await?
            .context("trial_connection_not_found")?;
        let config_snapshot = self
            .active_or_any_config_snapshot(tenant_id)
            .await?
            .context("config_snapshot_not_found")?;
        let created_at = now_rfc3339();
        let replay_capsule_id =
            ReplayCapsuleId::parse(format!("replay_{}", next_id_suffix())).unwrap();
        let route_receipt_id =
            RouteReceiptId::parse(format!("routercpt_{}", next_id_suffix())).unwrap();
        let request_id = format!("req_{}", next_id_suffix());
        let trace_id = format!("trace_{}", next_id_suffix());

        let replay_capsule = ReplayCapsule {
            replay_capsule_id: replay_capsule_id.clone(),
            request_id: request_id.clone(),
            trace_id: trace_id.clone(),
            route_receipt_id: route_receipt_id.clone(),
            config_snapshot_id: config_snapshot.config_snapshot_id.clone(),
            redaction_tier: RedactionTier::StructuredRedacted,
            normalized_request_summary: NormalizedRequestSummary {
                protocol_family: "openai_chat".to_string(),
                model_alias: connection.target_model.clone(),
                estimated_prompt_tokens: 480,
            },
            upstream_error_summary: Some(UpstreamErrorSummary {
                code: replay_upstream_error_code(&connection),
            }),
        };
        let evaluation =
            build_relay_evaluation(tenant_id, &connection, replay_capsule_id, created_at);
        evaluation.validate()?;
        let route_receipt = build_merchant_replay_route_receipt(
            tenant_id,
            &config_snapshot,
            route_receipt_id,
            request_id,
            trace_id,
            connection.target_model.clone(),
            evaluation.created_at.clone(),
        );

        let mut tx = self.pool.begin().await?;
        let replay_capsule_id = replay_capsule.replay_capsule_id.as_str().to_string();
        sqlx::query("INSERT INTO replay_capsules (replay_capsule_id, payload) VALUES ($1, $2)")
            .bind(&replay_capsule_id)
            .bind(Json(replay_capsule))
            .execute(&mut *tx)
            .await?;
        let route_receipt_id = route_receipt.route_receipt_id.as_str().to_string();
        sqlx::query("INSERT INTO route_receipts (route_receipt_id, payload) VALUES ($1, $2)")
            .bind(&route_receipt_id)
            .bind(Json(route_receipt))
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO relay_evaluations
                (relay_evaluation_id, tenant_id, trial_connection_id, replay_capsule_id, created_at, payload)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(evaluation.relay_evaluation_id.as_str())
        .bind(evaluation.tenant_id.as_str())
        .bind(evaluation.trial_connection_id.as_str())
        .bind(evaluation.replay_capsule_id.as_str())
        .bind(&evaluation.created_at)
        .bind(Json(evaluation.clone()))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(evaluation)
    }

    pub(crate) async fn get_replay_capsule(
        &self,
        tenant_id: &TenantId,
        replay_capsule_id: &str,
    ) -> Result<Option<ReplayCapsuleResponse>> {
        let has_tenant_evaluation = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM relay_evaluations
             WHERE tenant_id = $1 AND replay_capsule_id = $2",
        )
        .bind(tenant_id.as_str())
        .bind(replay_capsule_id)
        .fetch_one(&self.pool)
        .await?;
        if has_tenant_evaluation == 0 {
            return Ok(None);
        }

        let row = sqlx::query("SELECT payload FROM replay_capsules WHERE replay_capsule_id = $1")
            .bind(replay_capsule_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| ReplayCapsuleResponse {
            replay_capsule: row.get::<Json<ReplayCapsule>, _>("payload").0,
        }))
    }

    pub(crate) async fn seed_merchant(
        &self,
        shops: Vec<MerchantShop>,
        products: Vec<CardProduct>,
        connections: Vec<TrialConnection>,
        evaluations: Vec<RelayEvaluation>,
        capsules: Vec<ReplayCapsule>,
    ) -> Result<()> {
        for shop in shops {
            let shop_id = shop.merchant_shop_id.as_str().to_string();
            let tenant_id = shop.tenant_id.as_str().to_string();
            let slug = shop.slug.clone();
            sqlx::query(
                "INSERT INTO merchant_shops (merchant_shop_id, tenant_id, slug, payload)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (merchant_shop_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   slug = EXCLUDED.slug,
                   payload = EXCLUDED.payload",
            )
            .bind(&shop_id)
            .bind(&tenant_id)
            .bind(&slug)
            .bind(Json(shop))
            .execute(&self.pool)
            .await?;
        }
        for product in products {
            let product_id = product.card_product_id.as_str().to_string();
            let tenant_id = product.tenant_id.as_str().to_string();
            let shop_id = product.merchant_shop_id.as_str().to_string();
            sqlx::query(
                "INSERT INTO card_products (card_product_id, tenant_id, merchant_shop_id, payload)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (card_product_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   merchant_shop_id = EXCLUDED.merchant_shop_id,
                   payload = EXCLUDED.payload",
            )
            .bind(&product_id)
            .bind(&tenant_id)
            .bind(&shop_id)
            .bind(Json(product))
            .execute(&self.pool)
            .await?;
        }
        for connection in connections {
            let connection_id = connection.trial_connection_id.as_str().to_string();
            let tenant_id = connection.tenant_id.as_str().to_string();
            sqlx::query(
                "INSERT INTO trial_connections (trial_connection_id, tenant_id, payload)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (trial_connection_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   payload = EXCLUDED.payload",
            )
            .bind(&connection_id)
            .bind(&tenant_id)
            .bind(Json(connection))
            .execute(&self.pool)
            .await?;
        }
        for capsule in capsules {
            let capsule_id = capsule.replay_capsule_id.as_str().to_string();
            sqlx::query(
                "INSERT INTO replay_capsules (replay_capsule_id, payload)
                 VALUES ($1, $2)
                 ON CONFLICT (replay_capsule_id) DO UPDATE SET payload = EXCLUDED.payload",
            )
            .bind(&capsule_id)
            .bind(Json(capsule))
            .execute(&self.pool)
            .await?;
        }
        for evaluation in evaluations {
            let evaluation_id = evaluation.relay_evaluation_id.as_str().to_string();
            let tenant_id = evaluation.tenant_id.as_str().to_string();
            let connection_id = evaluation.trial_connection_id.as_str().to_string();
            let capsule_id = evaluation.replay_capsule_id.as_str().to_string();
            let created_at = evaluation.created_at.clone();
            sqlx::query(
                "INSERT INTO relay_evaluations
                    (relay_evaluation_id, tenant_id, trial_connection_id, replay_capsule_id, created_at, payload)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (relay_evaluation_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   trial_connection_id = EXCLUDED.trial_connection_id,
                   replay_capsule_id = EXCLUDED.replay_capsule_id,
                   created_at = EXCLUDED.created_at,
                   payload = EXCLUDED.payload",
            )
            .bind(&evaluation_id)
            .bind(&tenant_id)
            .bind(&connection_id)
            .bind(&capsule_id)
            .bind(&created_at)
            .bind(Json(evaluation))
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn list_json_payloads<T>(&self, table_name: &str, tenant_id: &TenantId) -> Result<Vec<T>>
    where
        T: for<'de> serde::Deserialize<'de> + Send + Unpin,
    {
        let query = format!("SELECT payload FROM {table_name} WHERE tenant_id = $1");
        let rows = sqlx::query(&query)
            .bind(tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<Json<T>, _>("payload").0)
            .collect())
    }

    async fn get_trial_connection(
        &self,
        tenant_id: &TenantId,
        trial_connection_id: &str,
    ) -> Result<Option<TrialConnection>> {
        let row = sqlx::query(
            "SELECT payload FROM trial_connections
             WHERE tenant_id = $1 AND trial_connection_id = $2",
        )
        .bind(tenant_id.as_str())
        .bind(trial_connection_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get::<Json<TrialConnection>, _>("payload").0))
    }

    async fn active_or_any_config_snapshot(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Option<ConfigSnapshot>> {
        let rows = sqlx::query(
            "SELECT payload FROM config_snapshots
             WHERE tenant_id = $1
             ORDER BY CASE WHEN status = 'active' THEN 0 ELSE 1 END, config_snapshot_id
             LIMIT 1",
        )
        .bind(tenant_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        Ok(rows.map(|row| row.get::<Json<ConfigSnapshot>, _>("payload").0))
    }
}
