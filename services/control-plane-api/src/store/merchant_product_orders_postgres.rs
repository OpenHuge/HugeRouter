use super::merchant_product_orders::{
    MERCHANT_INVENTORY_STATUS_AVAILABLE, MERCHANT_INVENTORY_STATUS_RESERVED,
    MERCHANT_INVENTORY_STATUS_SOLD, MERCHANT_ORDER_STATUS_CREATED, MERCHANT_ORDER_STATUS_FULFILLED,
    MERCHANT_ORDER_STATUS_PAYMENT_PENDING, MerchantPickupResponse, MerchantProductFulfillmentDraft,
    MerchantProductFulfillmentResult, MerchantProductInventoryRecord,
    MerchantProductOrderCreateResult, MerchantProductOrderDraft, MerchantProductOrderRecord,
    MerchantProductOrderResponse, MerchantProductPrepayDraft, MerchantPublicProduct,
    MerchantPublicShop, MerchantPublicShopResponse, merchant_pickup_response,
    merchant_product_order_response,
};
use super::{
    BTreeMap, CardProduct, CardProductStatus, DELIVERY_ARTIFACT_STATUS_ACTIVE,
    DELIVERY_CODE_TYPE_REDEMPTION, DELIVERY_LIFECYCLE_EVENT_SEGMENT_CREATED,
    DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE, DELIVERY_STATUS_PREPARED, DeliveryActivationResponse,
    DeliveryArtifactRecord, DeliveryCodeRecord, DeliveryDownloadGrantDraft,
    DeliveryDownloadGrantIssueResult, DeliveryEntitlementRecord, DeliveryLifecycleEventRecord,
    DeliveryRecord, DeliveryRedeemResult, DeliveryServiceSegmentRecord, Json, MerchantShop,
    MerchantShopStatus, PostgresStore, Result, Row, UserId, activate_delivery_entitlement, anyhow,
    blocked_delivery_activation_result, blocked_redemption_code_result,
    build_delivery_activation_record, build_postgres_service_segment_record, hash_api_key,
    insert_postgres_lifecycle_events, insert_postgres_service_segments, next_id_suffix,
    now_rfc3339, postgres_lifecycle_event_record, update_postgres_entitlement,
};

impl PostgresStore {
    pub(super) async fn bind_card_product_deliveries(
        &self,
        card_product_id: &str,
        delivery_ids: Vec<String>,
    ) -> Result<Vec<MerchantProductInventoryRecord>> {
        let product = sqlx::query("SELECT payload FROM card_products WHERE card_product_id = $1")
            .bind(card_product_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("card_product_not_found"))?
            .get::<Json<CardProduct>, _>("payload")
            .0;
        let mut records = Vec::new();
        for delivery_id in delivery_ids {
            let exists = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM merchant_product_inventory WHERE delivery_id = $1",
            )
            .bind(&delivery_id)
            .fetch_one(&self.pool)
            .await?;
            if exists > 0 {
                continue;
            }
            let delivery = sqlx::query("SELECT payload FROM deliveries WHERE delivery_id = $1")
                .bind(&delivery_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| anyhow!("delivery_not_found"))?
                .get::<Json<DeliveryRecord>, _>("payload")
                .0;
            if delivery.tenant_id != product.tenant_id
                || product
                    .project_id
                    .as_ref()
                    .is_some_and(|project_id| project_id != &delivery.project_id)
            {
                return Err(anyhow!("delivery_scope_mismatch"));
            }
            let entitlement =
                sqlx::query("SELECT payload FROM delivery_entitlements WHERE delivery_id = $1")
                    .bind(&delivery_id)
                    .fetch_optional(&self.pool)
                    .await?
                    .ok_or_else(|| anyhow!("delivery_entitlement_not_found"))?
                    .get::<Json<DeliveryEntitlementRecord>, _>("payload")
                    .0;
            if delivery.effective_status(&entitlement) != DELIVERY_STATUS_PREPARED {
                return Err(anyhow!("delivery_not_sale_ready"));
            }
            let artifact_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM delivery_artifacts WHERE delivery_id = $1 AND status = $2",
            )
            .bind(&delivery_id)
            .bind(DELIVERY_ARTIFACT_STATUS_ACTIVE)
            .fetch_one(&self.pool)
            .await?;
            if artifact_count == 0 {
                return Err(anyhow!("delivery_artifact_missing"));
            }
            let now = now_rfc3339();
            let record = MerchantProductInventoryRecord {
                inventory_id: format!("minv_{}", next_id_suffix()),
                tenant_id: product.tenant_id.clone(),
                project_id: delivery.project_id.clone(),
                merchant_shop_id: product.merchant_shop_id.clone(),
                card_product_id: product.card_product_id.clone(),
                delivery_id: delivery_id.clone(),
                status: MERCHANT_INVENTORY_STATUS_AVAILABLE.to_string(),
                reserved_order_id: None,
                sold_order_id: None,
                created_at: now.clone(),
                updated_at: now,
            };
            sqlx::query(
                "INSERT INTO merchant_product_inventory
                    (inventory_id, tenant_id, project_id, merchant_shop_id, card_product_id, delivery_id, status, reserved_order_id, sold_order_id, payload, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            )
            .bind(&record.inventory_id)
            .bind(record.tenant_id.as_str())
            .bind(record.project_id.as_str())
            .bind(record.merchant_shop_id.as_str())
            .bind(record.card_product_id.as_str())
            .bind(&record.delivery_id)
            .bind(&record.status)
            .bind(record.reserved_order_id.as_deref())
            .bind(record.sold_order_id.as_deref())
            .bind(Json(&record))
            .bind(&record.created_at)
            .bind(&record.updated_at)
            .execute(&self.pool)
            .await?;
            records.push(record);
        }
        Ok(records)
    }

    pub(super) async fn get_public_shop_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<MerchantPublicShopResponse>> {
        let Some(shop_row) =
            sqlx::query("SELECT payload FROM merchant_shops WHERE slug = $1 LIMIT 1")
                .bind(slug)
                .fetch_optional(&self.pool)
                .await?
        else {
            return Ok(None);
        };
        let shop = shop_row.get::<Json<MerchantShop>, _>("payload").0;
        if shop.status != MerchantShopStatus::Active {
            return Ok(None);
        }
        let product_rows = sqlx::query(
            "SELECT payload FROM card_products WHERE tenant_id = $1 AND merchant_shop_id = $2",
        )
        .bind(shop.tenant_id.as_str())
        .bind(shop.merchant_shop_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        let mut products = Vec::new();
        for row in product_rows {
            let product = row.get::<Json<CardProduct>, _>("payload").0;
            if product.status != CardProductStatus::Active
                || !product.sale_enabled
                || product.retail_price_cny_total.unwrap_or_default() == 0
            {
                continue;
            }
            let available_inventory_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM merchant_product_inventory WHERE card_product_id = $1 AND status = $2",
            )
            .bind(product.card_product_id.as_str())
            .bind(MERCHANT_INVENTORY_STATUS_AVAILABLE)
            .fetch_one(&self.pool)
            .await?;
            products.push(MerchantPublicProduct {
                product,
                available_inventory_count: usize::try_from(available_inventory_count).unwrap_or(0),
            });
        }
        Ok(Some(MerchantPublicShopResponse {
            data: MerchantPublicShop { shop, products },
        }))
    }

    pub(super) async fn create_merchant_product_order(
        &self,
        draft: MerchantProductOrderDraft,
    ) -> Result<MerchantProductOrderCreateResult> {
        let mut tx = self.pool.begin().await?;
        let Some(product_row) =
            sqlx::query("SELECT payload FROM card_products WHERE card_product_id = $1 FOR UPDATE")
                .bind(&draft.card_product_id)
                .fetch_optional(&mut *tx)
                .await?
        else {
            tx.commit().await?;
            return Ok(MerchantProductOrderCreateResult::ProductNotFound);
        };
        let product = product_row.get::<Json<CardProduct>, _>("payload").0;
        let Some(amount_total) = product.retail_price_cny_total else {
            tx.commit().await?;
            return Ok(MerchantProductOrderCreateResult::ProductNotSaleable);
        };
        if !product.sale_enabled || product.status != CardProductStatus::Active || amount_total == 0
        {
            tx.commit().await?;
            return Ok(MerchantProductOrderCreateResult::ProductNotSaleable);
        }
        let Some(inventory_row) = sqlx::query(
            "SELECT payload FROM merchant_product_inventory
              WHERE card_product_id = $1 AND status = $2
              ORDER BY created_at
              LIMIT 1
              FOR UPDATE SKIP LOCKED",
        )
        .bind(product.card_product_id.as_str())
        .bind(MERCHANT_INVENTORY_STATUS_AVAILABLE)
        .fetch_optional(&mut *tx)
        .await?
        else {
            tx.commit().await?;
            return Ok(MerchantProductOrderCreateResult::InventoryUnavailable);
        };
        let mut inventory = inventory_row
            .get::<Json<MerchantProductInventoryRecord>, _>("payload")
            .0;
        let now = now_rfc3339();
        let order = MerchantProductOrderRecord {
            order_id: draft.order_id,
            tenant_id: inventory.tenant_id.clone(),
            project_id: inventory.project_id.clone(),
            merchant_shop_id: inventory.merchant_shop_id.clone(),
            card_product_id: inventory.card_product_id.clone(),
            inventory_id: inventory.inventory_id.clone(),
            inventory_delivery_id: inventory.delivery_id.clone(),
            buyer_user_id: draft.buyer_user_id,
            out_trade_no: None,
            amount_total,
            currency: "CNY".to_string(),
            channel: None,
            status: MERCHANT_ORDER_STATUS_CREATED.to_string(),
            pickup_token: None,
            pickup_token_hash: None,
            activation_id: None,
            download_grant_id: None,
            download_token: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        inventory.status = MERCHANT_INVENTORY_STATUS_RESERVED.to_string();
        inventory.reserved_order_id = Some(order.order_id.clone());
        inventory.updated_at = now;
        sqlx::query(
            "UPDATE merchant_product_inventory
                SET status = $2, reserved_order_id = $3, payload = $4, updated_at = $5
              WHERE inventory_id = $1",
        )
        .bind(&inventory.inventory_id)
        .bind(&inventory.status)
        .bind(inventory.reserved_order_id.as_deref())
        .bind(Json(&inventory))
        .bind(&inventory.updated_at)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO merchant_product_orders
                (order_id, tenant_id, project_id, merchant_shop_id, card_product_id, inventory_id, inventory_delivery_id, buyer_user_id, out_trade_no, amount_total, currency, channel, status, pickup_token_hash, activation_id, download_grant_id, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)",
        )
        .bind(&order.order_id)
        .bind(order.tenant_id.as_str())
        .bind(order.project_id.as_str())
        .bind(order.merchant_shop_id.as_str())
        .bind(order.card_product_id.as_str())
        .bind(&order.inventory_id)
        .bind(&order.inventory_delivery_id)
        .bind(order.buyer_user_id.as_str())
        .bind(order.out_trade_no.as_deref())
        .bind(i64::from(order.amount_total))
        .bind(&order.currency)
        .bind(order.channel.as_deref())
        .bind(&order.status)
        .bind(order.pickup_token_hash.as_deref())
        .bind(order.activation_id.as_deref())
        .bind(order.download_grant_id.as_deref())
        .bind(Json(&order))
        .bind(&order.created_at)
        .bind(&order.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(MerchantProductOrderCreateResult::Created(Box::new(
            merchant_product_order_response(&order),
        )))
    }

    pub(super) async fn get_merchant_product_order(
        &self,
        order_id: &str,
    ) -> Result<Option<MerchantProductOrderResponse>> {
        let row = sqlx::query("SELECT payload FROM merchant_product_orders WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| {
            merchant_product_order_response(
                &row.get::<Json<MerchantProductOrderRecord>, _>("payload").0,
            )
        }))
    }

    pub(super) async fn get_merchant_product_order_by_out_trade_no(
        &self,
        out_trade_no: &str,
    ) -> Result<Option<MerchantProductOrderResponse>> {
        let row =
            sqlx::query("SELECT payload FROM merchant_product_orders WHERE out_trade_no = $1")
                .bind(out_trade_no)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|row| {
            merchant_product_order_response(
                &row.get::<Json<MerchantProductOrderRecord>, _>("payload").0,
            )
        }))
    }

    pub(super) async fn attach_merchant_order_prepay(
        &self,
        draft: MerchantProductPrepayDraft,
    ) -> Result<Option<MerchantProductOrderResponse>> {
        let Some(row) =
            sqlx::query("SELECT payload FROM merchant_product_orders WHERE order_id = $1")
                .bind(&draft.order_id)
                .fetch_optional(&self.pool)
                .await?
        else {
            return Ok(None);
        };
        let mut order = row.get::<Json<MerchantProductOrderRecord>, _>("payload").0;
        order.out_trade_no = Some(draft.out_trade_no);
        order.channel = Some(draft.channel);
        order.status = MERCHANT_ORDER_STATUS_PAYMENT_PENDING.to_string();
        order.updated_at = now_rfc3339();
        self.update_merchant_product_order(&order).await?;
        Ok(Some(merchant_product_order_response(&order)))
    }

    pub(super) async fn fulfill_merchant_product_order(
        &self,
        draft: MerchantProductFulfillmentDraft,
    ) -> Result<MerchantProductFulfillmentResult> {
        let Some(row) =
            sqlx::query("SELECT payload FROM merchant_product_orders WHERE order_id = $1")
                .bind(&draft.order_id)
                .fetch_optional(&self.pool)
                .await?
        else {
            return Ok(MerchantProductFulfillmentResult::OrderNotFound);
        };
        let mut order = row.get::<Json<MerchantProductOrderRecord>, _>("payload").0;
        if order.status == MERCHANT_ORDER_STATUS_FULFILLED {
            return Ok(MerchantProductFulfillmentResult::AlreadyFulfilled(
                merchant_product_order_response(&order),
            ));
        }
        let activation = match self
            .activate_delivery_for_merchant_order(
                &order.inventory_delivery_id,
                draft.activation_id.clone(),
                &draft.fulfilled_by,
            )
            .await?
        {
            DeliveryRedeemResult::Activated(response) => response.data,
            other => return Ok(MerchantProductFulfillmentResult::DeliveryActivation(other)),
        };
        let grant = match self
            .issue_delivery_download_grant(DeliveryDownloadGrantDraft {
                grant_id: draft.download_grant_id.clone(),
                activation_id: activation.activation_id.clone(),
                token_plaintext: draft.download_token.clone(),
                created_by: draft.fulfilled_by.clone(),
            })
            .await?
        {
            DeliveryDownloadGrantIssueResult::Issued(response) => response,
            other => return Ok(MerchantProductFulfillmentResult::DownloadGrant(other)),
        };
        let mut inventory =
            sqlx::query("SELECT payload FROM merchant_product_inventory WHERE inventory_id = $1")
                .bind(&order.inventory_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| anyhow!("inventory_not_found"))?
                .get::<Json<MerchantProductInventoryRecord>, _>("payload")
                .0;
        let now = now_rfc3339();
        inventory.status = MERCHANT_INVENTORY_STATUS_SOLD.to_string();
        inventory.sold_order_id = Some(order.order_id.clone());
        inventory.updated_at = now.clone();
        sqlx::query(
            "UPDATE merchant_product_inventory
                SET status = $2, sold_order_id = $3, payload = $4, updated_at = $5
              WHERE inventory_id = $1",
        )
        .bind(&inventory.inventory_id)
        .bind(&inventory.status)
        .bind(inventory.sold_order_id.as_deref())
        .bind(Json(&inventory))
        .bind(&inventory.updated_at)
        .execute(&self.pool)
        .await?;
        order.status = MERCHANT_ORDER_STATUS_FULFILLED.to_string();
        order.pickup_token = Some(draft.pickup_token.clone());
        order.pickup_token_hash = Some(hash_api_key(&draft.pickup_token));
        order.activation_id = Some(activation.activation_id);
        order.download_grant_id = Some(grant.data.grant_id);
        order.download_token = Some(draft.download_token);
        order.updated_at = now;
        self.update_merchant_product_order(&order).await?;
        Ok(MerchantProductFulfillmentResult::Fulfilled(
            merchant_product_order_response(&order),
        ))
    }

    pub(super) async fn get_pickup_by_token_hash(
        &self,
        pickup_token_hash: &str,
    ) -> Result<Option<MerchantPickupResponse>> {
        let row =
            sqlx::query("SELECT payload FROM merchant_product_orders WHERE pickup_token_hash = $1")
                .bind(pickup_token_hash)
                .fetch_optional(&self.pool)
                .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let order = row.get::<Json<MerchantProductOrderRecord>, _>("payload").0;
        let browser_file_unlock_code = self
            .get_delivery_secret_plaintext(
                &order.inventory_delivery_id,
                super::DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK,
            )
            .await?;
        Ok(merchant_pickup_response(&order, browser_file_unlock_code))
    }

    pub(super) async fn upsert_wechat_user_openid(
        &self,
        user_id: &UserId,
        openid: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO wechat_user_openids (user_id, openid, updated_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET openid = EXCLUDED.openid, updated_at = EXCLUDED.updated_at",
        )
        .bind(user_id.as_str())
        .bind(openid)
        .bind(now_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn get_wechat_user_openid(&self, user_id: &UserId) -> Result<Option<String>> {
        let row = sqlx::query("SELECT openid FROM wechat_user_openids WHERE user_id = $1")
            .bind(user_id.as_str())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<String, _>("openid")))
    }

    async fn update_merchant_product_order(
        &self,
        order: &MerchantProductOrderRecord,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE merchant_product_orders
                SET out_trade_no = $2, channel = $3, status = $4, pickup_token_hash = $5,
                    activation_id = $6, download_grant_id = $7, payload = $8, updated_at = $9
              WHERE order_id = $1",
        )
        .bind(&order.order_id)
        .bind(order.out_trade_no.as_deref())
        .bind(order.channel.as_deref())
        .bind(&order.status)
        .bind(order.pickup_token_hash.as_deref())
        .bind(order.activation_id.as_deref())
        .bind(order.download_grant_id.as_deref())
        .bind(Json(order))
        .bind(&order.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn activate_delivery_for_merchant_order(
        &self,
        delivery_id: &str,
        activation_id: String,
        activated_by: &str,
    ) -> Result<DeliveryRedeemResult> {
        let mut tx = self.pool.begin().await?;
        let Some(code_row) = sqlx::query(
            "SELECT payload FROM delivery_codes WHERE delivery_id = $1 AND code_type = $2 FOR UPDATE",
        )
        .bind(delivery_id)
        .bind(DELIVERY_CODE_TYPE_REDEMPTION)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryRedeemResult::RedemptionCodeNotFound);
        };
        let mut code = code_row.get::<Json<DeliveryCodeRecord>, _>("payload").0;
        if let Some(result) = blocked_redemption_code_result(&code) {
            return Ok(result);
        }
        let Some(delivery_row) =
            sqlx::query("SELECT payload FROM deliveries WHERE delivery_id = $1 FOR UPDATE")
                .bind(delivery_id)
                .fetch_optional(&mut *tx)
                .await?
        else {
            return Ok(DeliveryRedeemResult::DeliveryNotFound);
        };
        let delivery = delivery_row.get::<Json<DeliveryRecord>, _>("payload").0;
        let Some(entitlement_row) = sqlx::query(
            "SELECT payload FROM delivery_entitlements WHERE delivery_id = $1 FOR UPDATE",
        )
        .bind(delivery_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryRedeemResult::EntitlementNotFound);
        };
        let mut entitlement = entitlement_row
            .get::<Json<DeliveryEntitlementRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_delivery_activation_result(&delivery, &entitlement) {
            return Ok(result);
        }
        let Some(artifact_row) = sqlx::query(
            "SELECT payload FROM delivery_artifacts WHERE delivery_id = $1 AND status = $2 ORDER BY version DESC LIMIT 1 FOR UPDATE",
        )
        .bind(delivery_id)
        .bind(DELIVERY_ARTIFACT_STATUS_ACTIVE)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryRedeemResult::ArtifactMissing);
        };
        let artifact = artifact_row
            .get::<Json<DeliveryArtifactRecord>, _>("payload")
            .0;
        activate_delivery_entitlement(&mut entitlement, None);
        let activation = build_delivery_activation_record(
            &activation_id,
            &delivery,
            &mut code,
            &entitlement,
            &artifact,
        );
        let mut new_segments = Vec::<DeliveryServiceSegmentRecord>::new();
        let mut new_events = Vec::<DeliveryLifecycleEventRecord>::new();
        if let Some(segment) = build_postgres_service_segment_record(
            &[],
            &[],
            &entitlement,
            &activation,
            &artifact,
            activation.activated_at.clone(),
            activated_by,
        ) {
            new_events.push(postgres_lifecycle_event_record(
                &entitlement.entitlement_id,
                Some(segment.segment_id.clone()),
                DELIVERY_LIFECYCLE_EVENT_SEGMENT_CREATED,
                DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE,
                Some("merchant_product_payment".to_string()),
                activated_by,
                BTreeMap::from([("artifact_id".to_string(), artifact.artifact_id.clone())]),
            ));
            new_segments.push(segment);
        }
        sqlx::query(
            "UPDATE delivery_codes SET status = $2, payload = $3, updated_at = $4 WHERE code_id = $1",
        )
        .bind(&code.code_id)
        .bind(&code.status)
        .bind(Json(&code))
        .bind(&code.updated_at)
        .execute(&mut *tx)
        .await?;
        update_postgres_entitlement(&mut tx, &entitlement).await?;
        sqlx::query(
            "INSERT INTO delivery_activations
                (activation_id, delivery_id, code_id, artifact_id, entitlement_id, tenant_id, project_id, status, activated_at, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&activation.activation_id)
        .bind(&activation.delivery_id)
        .bind(&activation.code_id)
        .bind(&activation.artifact_id)
        .bind(&activation.entitlement_id)
        .bind(activation.tenant_id.as_str())
        .bind(activation.project_id.as_str())
        .bind(&activation.status)
        .bind(&activation.activated_at)
        .bind(Json(&activation))
        .bind(&activation.created_at)
        .bind(&activation.updated_at)
        .execute(&mut *tx)
        .await?;
        insert_postgres_service_segments(&mut tx, &new_segments).await?;
        insert_postgres_lifecycle_events(&mut tx, &new_events).await?;
        tx.commit().await?;
        Ok(DeliveryRedeemResult::Activated(Box::new(
            DeliveryActivationResponse {
                data: activation.public_view(),
            },
        )))
    }
}
