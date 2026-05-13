use super::{
    CardProduct, CardProductId, CardProductStatus, DELIVERY_ARTIFACT_STATUS_ACTIVE,
    DELIVERY_CODE_TYPE_REDEMPTION, DELIVERY_STATUS_PREPARED, DeliveryActivationResponse,
    DeliveryDownloadGrantDraft, DeliveryDownloadGrantIssueResult, DeliveryRedeemResult,
    MemoryStore, MerchantShop, MerchantShopId, MerchantShopStatus, ProjectId, Result, StoreMode,
    TenantId, UserId, activate_delivery_entitlement, anyhow, apply_segment_for_artifact,
    blocked_delivery_activation_result, blocked_redemption_code_result,
    build_delivery_activation_record, hash_api_key, issue_memory_delivery_download_grant,
    next_id_suffix, now_rfc3339,
};
use serde::{Deserialize, Serialize};

pub const MERCHANT_INVENTORY_STATUS_AVAILABLE: &str = "available";
pub const MERCHANT_INVENTORY_STATUS_RESERVED: &str = "reserved";
pub const MERCHANT_INVENTORY_STATUS_SOLD: &str = "sold";
pub const MERCHANT_ORDER_STATUS_CREATED: &str = "created";
pub const MERCHANT_ORDER_STATUS_PAYMENT_PENDING: &str = "payment_pending";
pub const MERCHANT_ORDER_STATUS_FULFILLED: &str = "fulfilled";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantProductInventoryRecord {
    pub inventory_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub merchant_shop_id: MerchantShopId,
    pub card_product_id: CardProductId,
    pub delivery_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserved_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sold_order_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantProductOrderRecord {
    pub order_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub merchant_shop_id: MerchantShopId,
    pub card_product_id: CardProductId,
    pub inventory_id: String,
    pub inventory_delivery_id: String,
    pub buyer_user_id: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    pub amount_total: u32,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pickup_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pickup_token_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_grant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_token: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantProductOrderResponse {
    pub data: MerchantProductOrderPublicView,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantProductOrderPublicView {
    pub order_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub merchant_shop_id: MerchantShopId,
    pub card_product_id: CardProductId,
    pub inventory_delivery_id: String,
    pub buyer_user_id: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    pub amount_total: u32,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pickup_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_grant_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantPublicShopResponse {
    pub data: MerchantPublicShop,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantPublicShop {
    pub shop: MerchantShop,
    pub products: Vec<MerchantPublicProduct>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantPublicProduct {
    pub product: CardProduct,
    pub available_inventory_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantPickupResponse {
    pub data: MerchantPickupPayload,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantPickupPayload {
    pub order: MerchantProductOrderPublicView,
    pub download_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_account_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MerchantProductOrderDraft {
    pub order_id: String,
    pub card_product_id: String,
    pub buyer_user_id: UserId,
}

#[derive(Debug, Clone)]
pub struct MerchantProductPrepayDraft {
    pub order_id: String,
    pub out_trade_no: String,
    pub channel: String,
}

#[derive(Debug, Clone)]
pub struct MerchantProductFulfillmentDraft {
    pub order_id: String,
    pub activation_id: String,
    pub download_grant_id: String,
    pub download_token: String,
    pub pickup_token: String,
    pub fulfilled_by: String,
}

#[derive(Debug, Clone)]
pub enum MerchantProductOrderCreateResult {
    Created(Box<MerchantProductOrderResponse>),
    ProductNotFound,
    ProductNotSaleable,
    InventoryUnavailable,
}

#[derive(Debug, Clone)]
pub enum MerchantProductFulfillmentResult {
    Fulfilled(MerchantProductOrderResponse),
    AlreadyFulfilled(MerchantProductOrderResponse),
    OrderNotFound,
    InventoryNotFound,
    DeliveryActivation(DeliveryRedeemResult),
    DownloadGrant(DeliveryDownloadGrantIssueResult),
}

impl StoreMode {
    pub async fn bind_card_product_deliveries(
        &self,
        card_product_id: &str,
        delivery_ids: Vec<String>,
    ) -> Result<Vec<MerchantProductInventoryRecord>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                bind_memory_card_product_deliveries(&mut store, card_product_id, delivery_ids)
            }
            Self::Postgres(store) => {
                store
                    .bind_card_product_deliveries(card_product_id, delivery_ids)
                    .await
            }
        }
    }

    pub async fn get_public_shop_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<MerchantPublicShopResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(public_shop_from_memory(&store, slug))
            }
            Self::Postgres(store) => store.get_public_shop_by_slug(slug).await,
        }
    }

    pub async fn create_merchant_product_order(
        &self,
        draft: MerchantProductOrderDraft,
    ) -> Result<MerchantProductOrderCreateResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(create_memory_merchant_product_order(&mut store, draft))
            }
            Self::Postgres(store) => store.create_merchant_product_order(draft).await,
        }
    }

    pub async fn get_merchant_product_order(
        &self,
        order_id: &str,
    ) -> Result<Option<MerchantProductOrderResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .merchant_product_orders
                    .iter()
                    .find(|order| order.order_id == order_id)
                    .map(merchant_product_order_response))
            }
            Self::Postgres(store) => store.get_merchant_product_order(order_id).await,
        }
    }

    pub async fn get_merchant_product_order_by_out_trade_no(
        &self,
        out_trade_no: &str,
    ) -> Result<Option<MerchantProductOrderResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .merchant_product_orders
                    .iter()
                    .find(|order| order.out_trade_no.as_deref() == Some(out_trade_no))
                    .map(merchant_product_order_response))
            }
            Self::Postgres(store) => {
                store
                    .get_merchant_product_order_by_out_trade_no(out_trade_no)
                    .await
            }
        }
    }

    pub async fn attach_merchant_order_prepay(
        &self,
        draft: MerchantProductPrepayDraft,
    ) -> Result<Option<MerchantProductOrderResponse>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(attach_memory_merchant_order_prepay(&mut store, draft))
            }
            Self::Postgres(store) => store.attach_merchant_order_prepay(draft).await,
        }
    }

    pub async fn fulfill_merchant_product_order(
        &self,
        draft: MerchantProductFulfillmentDraft,
    ) -> Result<MerchantProductFulfillmentResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(fulfill_memory_merchant_product_order(&mut store, draft))
            }
            Self::Postgres(store) => store.fulfill_merchant_product_order(draft).await,
        }
    }

    pub async fn get_pickup_by_token_hash(
        &self,
        pickup_token_hash: &str,
    ) -> Result<Option<MerchantPickupResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .merchant_product_orders
                    .iter()
                    .find(|order| order.pickup_token_hash.as_deref() == Some(pickup_token_hash))
                    .and_then(merchant_pickup_response))
            }
            Self::Postgres(store) => store.get_pickup_by_token_hash(pickup_token_hash).await,
        }
    }

    pub async fn upsert_wechat_user_openid(&self, user_id: &UserId, openid: &str) -> Result<()> {
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .wechat_user_openids
                    .insert(user_id.as_str().to_string(), openid.to_string());
                Ok(())
            }
            Self::Postgres(store) => store.upsert_wechat_user_openid(user_id, openid).await,
        }
    }

    pub async fn get_wechat_user_openid(&self, user_id: &UserId) -> Result<Option<String>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .wechat_user_openids
                .get(user_id.as_str())
                .cloned()),
            Self::Postgres(store) => store.get_wechat_user_openid(user_id).await,
        }
    }
}

pub(super) fn merchant_product_order_response(
    order: &MerchantProductOrderRecord,
) -> MerchantProductOrderResponse {
    MerchantProductOrderResponse {
        data: MerchantProductOrderPublicView {
            order_id: order.order_id.clone(),
            tenant_id: order.tenant_id.clone(),
            project_id: order.project_id.clone(),
            merchant_shop_id: order.merchant_shop_id.clone(),
            card_product_id: order.card_product_id.clone(),
            inventory_delivery_id: order.inventory_delivery_id.clone(),
            buyer_user_id: order.buyer_user_id.clone(),
            out_trade_no: order.out_trade_no.clone(),
            amount_total: order.amount_total,
            currency: order.currency.clone(),
            channel: order.channel.clone(),
            status: order.status.clone(),
            pickup_token: order.pickup_token.clone(),
            activation_id: order.activation_id.clone(),
            download_grant_id: order.download_grant_id.clone(),
            created_at: order.created_at.clone(),
            updated_at: order.updated_at.clone(),
        },
    }
}

pub(super) fn merchant_pickup_response(
    order: &MerchantProductOrderRecord,
) -> Option<MerchantPickupResponse> {
    if order.status != MERCHANT_ORDER_STATUS_FULFILLED {
        return None;
    }
    let download_token = order.download_token.clone()?;
    let public_view = merchant_product_order_response(order).data;
    Some(MerchantPickupResponse {
        data: MerchantPickupPayload {
            shared_account_id: Some(format!("shared:{}", public_view.inventory_delivery_id)),
            order: public_view,
            download_token,
        },
    })
}

fn public_shop_from_memory(store: &MemoryStore, slug: &str) -> Option<MerchantPublicShopResponse> {
    let shop = store
        .merchant_shops
        .iter()
        .find(|shop| shop.slug == slug && shop.status == MerchantShopStatus::Active)?
        .clone();
    let products = store
        .card_products
        .iter()
        .filter(|product| {
            product.merchant_shop_id == shop.merchant_shop_id
                && product.status == CardProductStatus::Active
                && product.sale_enabled
                && product.retail_price_cny_total.unwrap_or_default() > 0
        })
        .map(|product| MerchantPublicProduct {
            product: product.clone(),
            available_inventory_count: store
                .merchant_product_inventory
                .iter()
                .filter(|inventory| {
                    inventory.card_product_id == product.card_product_id
                        && inventory.status == MERCHANT_INVENTORY_STATUS_AVAILABLE
                })
                .count(),
        })
        .collect();
    Some(MerchantPublicShopResponse {
        data: MerchantPublicShop { shop, products },
    })
}

fn bind_memory_card_product_deliveries(
    store: &mut MemoryStore,
    card_product_id: &str,
    delivery_ids: Vec<String>,
) -> Result<Vec<MerchantProductInventoryRecord>> {
    let product = store
        .card_products
        .iter()
        .find(|product| product.card_product_id.as_str() == card_product_id)
        .cloned()
        .ok_or_else(|| anyhow!("card_product_not_found"))?;
    let mut records = Vec::new();
    for delivery_id in delivery_ids {
        if store
            .merchant_product_inventory
            .iter()
            .any(|inventory| inventory.delivery_id == delivery_id)
        {
            continue;
        }
        let delivery = store
            .deliveries
            .iter()
            .find(|delivery| delivery.delivery_id == delivery_id)
            .cloned()
            .ok_or_else(|| anyhow!("delivery_not_found"))?;
        if delivery.tenant_id != product.tenant_id
            || product
                .project_id
                .as_ref()
                .is_some_and(|project_id| project_id != &delivery.project_id)
        {
            return Err(anyhow!("delivery_scope_mismatch"));
        }
        let entitlement = store
            .delivery_entitlements
            .iter()
            .find(|entitlement| entitlement.delivery_id == delivery_id)
            .ok_or_else(|| anyhow!("delivery_entitlement_not_found"))?;
        if delivery.effective_status(entitlement) != DELIVERY_STATUS_PREPARED {
            return Err(anyhow!("delivery_not_sale_ready"));
        }
        if !store.delivery_artifacts.iter().any(|artifact| {
            artifact.delivery_id == delivery_id
                && artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE
        }) {
            return Err(anyhow!("delivery_artifact_missing"));
        }
        let now = now_rfc3339();
        let record = MerchantProductInventoryRecord {
            inventory_id: format!("minv_{}", next_id_suffix()),
            tenant_id: product.tenant_id.clone(),
            project_id: delivery.project_id.clone(),
            merchant_shop_id: product.merchant_shop_id.clone(),
            card_product_id: product.card_product_id.clone(),
            delivery_id,
            status: MERCHANT_INVENTORY_STATUS_AVAILABLE.to_string(),
            reserved_order_id: None,
            sold_order_id: None,
            created_at: now.clone(),
            updated_at: now,
        };
        store.merchant_product_inventory.push(record.clone());
        records.push(record);
    }
    Ok(records)
}

fn create_memory_merchant_product_order(
    store: &mut MemoryStore,
    draft: MerchantProductOrderDraft,
) -> MerchantProductOrderCreateResult {
    let Some(product) = store
        .card_products
        .iter()
        .find(|product| product.card_product_id.as_str() == draft.card_product_id)
        .cloned()
    else {
        return MerchantProductOrderCreateResult::ProductNotFound;
    };
    let Some(amount_total) = product.retail_price_cny_total else {
        return MerchantProductOrderCreateResult::ProductNotSaleable;
    };
    if !product.sale_enabled || product.status != CardProductStatus::Active || amount_total == 0 {
        return MerchantProductOrderCreateResult::ProductNotSaleable;
    }
    let Some(inventory_index) = store
        .merchant_product_inventory
        .iter()
        .position(|inventory| {
            inventory.card_product_id == product.card_product_id
                && inventory.status == MERCHANT_INVENTORY_STATUS_AVAILABLE
        })
    else {
        return MerchantProductOrderCreateResult::InventoryUnavailable;
    };
    let now = now_rfc3339();
    let inventory = store.merchant_product_inventory[inventory_index].clone();
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
        pickup_token_hash: None,
        pickup_token: None,
        activation_id: None,
        download_grant_id: None,
        download_token: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let inventory = &mut store.merchant_product_inventory[inventory_index];
    inventory.status = MERCHANT_INVENTORY_STATUS_RESERVED.to_string();
    inventory.reserved_order_id = Some(order.order_id.clone());
    inventory.updated_at = now;
    store.merchant_product_orders.push(order.clone());
    MerchantProductOrderCreateResult::Created(Box::new(merchant_product_order_response(&order)))
}

fn attach_memory_merchant_order_prepay(
    store: &mut MemoryStore,
    draft: MerchantProductPrepayDraft,
) -> Option<MerchantProductOrderResponse> {
    let order = store
        .merchant_product_orders
        .iter_mut()
        .find(|order| order.order_id == draft.order_id)?;
    order.out_trade_no = Some(draft.out_trade_no);
    order.channel = Some(draft.channel);
    order.status = MERCHANT_ORDER_STATUS_PAYMENT_PENDING.to_string();
    order.updated_at = now_rfc3339();
    Some(merchant_product_order_response(order))
}

fn activate_memory_delivery_for_merchant_order(
    store: &mut MemoryStore,
    delivery_id: &str,
    activation_id: String,
    activated_by: &str,
) -> DeliveryRedeemResult {
    let Some(code_index) = store.delivery_codes.iter().position(|code| {
        code.code_type == DELIVERY_CODE_TYPE_REDEMPTION && code.delivery_id == delivery_id
    }) else {
        return DeliveryRedeemResult::RedemptionCodeNotFound;
    };
    let mut code = store.delivery_codes[code_index].clone();
    if let Some(result) = blocked_redemption_code_result(&code) {
        return result;
    }
    let Some(delivery) = store
        .deliveries
        .iter()
        .find(|delivery| delivery.delivery_id == delivery_id)
        .cloned()
    else {
        return DeliveryRedeemResult::DeliveryNotFound;
    };
    let Some(entitlement_index) = store
        .delivery_entitlements
        .iter()
        .position(|entitlement| entitlement.delivery_id == delivery_id)
    else {
        return DeliveryRedeemResult::EntitlementNotFound;
    };
    let mut entitlement = store.delivery_entitlements[entitlement_index].clone();
    if let Some(result) = blocked_delivery_activation_result(&delivery, &entitlement) {
        return result;
    }
    let Some(artifact) = store
        .delivery_artifacts
        .iter()
        .filter(|artifact| {
            artifact.delivery_id == delivery_id
                && artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE
        })
        .max_by(|left, right| left.version.cmp(&right.version))
        .cloned()
    else {
        return DeliveryRedeemResult::ArtifactMissing;
    };
    activate_delivery_entitlement(&mut entitlement, None);
    let activation = build_delivery_activation_record(
        &activation_id,
        &delivery,
        &mut code,
        &entitlement,
        &artifact,
    );
    apply_segment_for_artifact(
        store,
        &mut entitlement,
        &activation,
        &artifact,
        activated_by,
        "merchant_product_payment",
    );
    store.delivery_codes[code_index] = code;
    store.delivery_entitlements[entitlement_index] = entitlement;
    store.delivery_activations.push(activation.clone());
    DeliveryRedeemResult::Activated(Box::new(DeliveryActivationResponse {
        data: activation.public_view(),
    }))
}

fn fulfill_memory_merchant_product_order(
    store: &mut MemoryStore,
    draft: MerchantProductFulfillmentDraft,
) -> MerchantProductFulfillmentResult {
    let Some(order_index) = store
        .merchant_product_orders
        .iter()
        .position(|order| order.order_id == draft.order_id)
    else {
        return MerchantProductFulfillmentResult::OrderNotFound;
    };
    let existing = store.merchant_product_orders[order_index].clone();
    if existing.status == MERCHANT_ORDER_STATUS_FULFILLED {
        return MerchantProductFulfillmentResult::AlreadyFulfilled(
            merchant_product_order_response(&existing),
        );
    }
    let Some(inventory_index) = store
        .merchant_product_inventory
        .iter()
        .position(|inventory| inventory.inventory_id == existing.inventory_id)
    else {
        return MerchantProductFulfillmentResult::InventoryNotFound;
    };
    let activation = match activate_memory_delivery_for_merchant_order(
        store,
        &existing.inventory_delivery_id,
        draft.activation_id.clone(),
        &draft.fulfilled_by,
    ) {
        DeliveryRedeemResult::Activated(response) => response.data,
        other => return MerchantProductFulfillmentResult::DeliveryActivation(other),
    };
    let grant = match issue_memory_delivery_download_grant(
        store,
        DeliveryDownloadGrantDraft {
            grant_id: draft.download_grant_id.clone(),
            activation_id: activation.activation_id.clone(),
            token_plaintext: draft.download_token.clone(),
            created_by: draft.fulfilled_by.clone(),
        },
    ) {
        DeliveryDownloadGrantIssueResult::Issued(response) => response,
        other => return MerchantProductFulfillmentResult::DownloadGrant(other),
    };
    let now = now_rfc3339();
    let inventory = &mut store.merchant_product_inventory[inventory_index];
    inventory.status = MERCHANT_INVENTORY_STATUS_SOLD.to_string();
    inventory.sold_order_id = Some(existing.order_id.clone());
    inventory.updated_at = now.clone();
    let order = &mut store.merchant_product_orders[order_index];
    order.status = MERCHANT_ORDER_STATUS_FULFILLED.to_string();
    order.pickup_token = Some(draft.pickup_token.clone());
    order.pickup_token_hash = Some(hash_api_key(&draft.pickup_token));
    order.activation_id = Some(activation.activation_id);
    order.download_grant_id = Some(grant.data.grant_id.clone());
    order.download_token = Some(draft.download_token);
    order.updated_at = now;
    MerchantProductFulfillmentResult::Fulfilled(merchant_product_order_response(order))
}
