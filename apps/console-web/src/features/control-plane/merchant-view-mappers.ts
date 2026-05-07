import type {
  cardProductSchema,
  merchantShopSchema,
} from "@huge-router/ts-shared-schema";
import type { CardProductView, MerchantShopView } from "./types";

export function toMerchantShopView(
  shop: ReturnType<typeof merchantShopSchema.parse>,
): MerchantShopView {
  return {
    announcement: shop.announcement,
    createdAt: shop.created_at,
    displayName: shop.display_name,
    fulfillmentMode: shop.fulfillment_mode,
    merchantShopId: shop.merchant_shop_id,
    slug: shop.slug,
    status: shop.status,
    updatedAt: shop.updated_at,
    version: shop.version,
  };
}

export function toCardProductView(
  product: ReturnType<typeof cardProductSchema.parse>,
): CardProductView {
  return {
    cardProductId: product.card_product_id,
    createdAt: product.created_at,
    deliveryKind: product.delivery_kind,
    description: product.description,
    faceValueUsd: product.face_value_usd,
    inventoryCount: product.inventory_count,
    merchantShopId: product.merchant_shop_id,
    projectId: product.project_id,
    retailPriceCnyTotal: product.retail_price_cny_total,
    retailPriceUsd: product.retail_price_usd,
    saleEnabled: product.sale_enabled,
    status: product.status,
    supportsTrial: product.supports_trial,
    title: product.title,
    updatedAt: product.updated_at,
    version: product.version,
  };
}
