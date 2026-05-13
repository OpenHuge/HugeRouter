import {
  cardProductSchema,
  merchantShopSchema,
} from "@huge-router/ts-shared-schema";
import { z } from "zod";
import type {
  MerchantPickupView,
  MerchantProductOrderView,
  MerchantProductPrepayResult,
  MerchantPublicShopView,
} from "./types";
import { toCardProductView, toMerchantShopView } from "./merchant-view-mappers";

type ControlPlaneJsonRequester = <T>(
  path: string,
  parser: (payload: unknown) => T,
  init?: RequestInit,
) => Promise<T>;

export type MerchantCheckoutService = {
  getPublicShop: (slug: string) => Promise<MerchantPublicShopView>;
  createMerchantProductOrder: (
    cardProductId: string,
  ) => Promise<MerchantProductOrderView>;
  getMerchantProductOrder: (
    orderId: string,
  ) => Promise<MerchantProductOrderView>;
  createMerchantProductOrderWechatPrepay: (
    orderId: string,
    channel: "native" | "jsapi",
  ) => Promise<MerchantProductPrepayResult>;
  getMerchantPickup: (pickupToken: string) => Promise<MerchantPickupView>;
};

const merchantPublicShopResponseSchema = z.object({
  data: z.object({
    shop: merchantShopSchema,
    products: z.array(
      z.object({
        product: cardProductSchema,
        available_inventory_count: z.number().int().nonnegative(),
      }),
    ),
  }),
});

const merchantProductOrderPayloadSchema = z.object({
  order_id: z.string().min(1),
  tenant_id: z.string().min(1),
  project_id: z.string().min(1),
  merchant_shop_id: z.string().min(1),
  card_product_id: z.string().min(1),
  inventory_delivery_id: z.string().min(1),
  buyer_user_id: z.string().min(1),
  out_trade_no: z.string().min(1).optional(),
  amount_total: z.number().int().nonnegative(),
  currency: z.literal("CNY"),
  channel: z.string().min(1).optional(),
  status: z.string().min(1),
  pickup_token: z.string().min(1).optional(),
  activation_id: z.string().min(1).optional(),
  download_grant_id: z.string().min(1).optional(),
  created_at: z.string().min(1),
  updated_at: z.string().min(1),
});

const merchantProductOrderResponseSchema = z.object({
  data: merchantProductOrderPayloadSchema,
});

const merchantProductPrepayResponseSchema = z.object({
  app_id: z.string().min(1),
  mchid: z.string().min(1),
  out_trade_no: z.string().min(1),
  channel: z.enum(["native", "jsapi"]),
  code_url: z.string().min(1).optional(),
  code_qr_svg: z.string().min(1).optional(),
  prepay_id: z.string().min(1).optional(),
  jsapi_params: z
    .object({
      appId: z.string().min(1),
      timeStamp: z.string().min(1),
      nonceStr: z.string().min(1),
      package: z.string().min(1),
      signType: z.string().min(1),
      paySign: z.string().min(1),
    })
    .optional(),
});

const merchantPickupResponseSchema = z.object({
  data: z.object({
    order: merchantProductOrderPayloadSchema,
    download_token: z.string().min(1),
  }),
});

export function toMerchantProductOrderView(
  order: z.infer<typeof merchantProductOrderPayloadSchema>,
): MerchantProductOrderView {
  return {
    activationId: order.activation_id,
    amountTotal: order.amount_total,
    buyerUserId: order.buyer_user_id,
    cardProductId: order.card_product_id,
    channel: order.channel,
    createdAt: order.created_at,
    currency: order.currency,
    downloadGrantId: order.download_grant_id,
    inventoryDeliveryId: order.inventory_delivery_id,
    merchantShopId: order.merchant_shop_id,
    orderId: order.order_id,
    outTradeNo: order.out_trade_no,
    pickupToken: order.pickup_token,
    projectId: order.project_id,
    status: order.status,
    tenantId: order.tenant_id,
    updatedAt: order.updated_at,
  };
}

export function parsePublicShop(payload: unknown): MerchantPublicShopView {
  const parsed = merchantPublicShopResponseSchema.parse(payload).data;

  return {
    products: parsed.products.map((item) => ({
      availableInventoryCount: item.available_inventory_count,
      product: toCardProductView(item.product),
    })),
    shop: toMerchantShopView(parsed.shop),
  };
}

export function parseMerchantProductOrder(
  payload: unknown,
): MerchantProductOrderView {
  return toMerchantProductOrderView(
    merchantProductOrderResponseSchema.parse(payload).data,
  );
}

export function parseMerchantProductPrepay(
  payload: unknown,
): MerchantProductPrepayResult {
  const parsed = merchantProductPrepayResponseSchema.parse(payload);

  return {
    appId: parsed.app_id,
    channel: parsed.channel,
    codeQrSvg: parsed.code_qr_svg,
    codeUrl: parsed.code_url,
    jsapiParams: parsed.jsapi_params,
    mchid: parsed.mchid,
    outTradeNo: parsed.out_trade_no,
    prepayId: parsed.prepay_id,
  };
}

export function parseMerchantPickup(payload: unknown): MerchantPickupView {
  const parsed = merchantPickupResponseSchema.parse(payload).data;

  return {
    downloadToken: parsed.download_token,
    order: toMerchantProductOrderView(parsed.order),
  };
}

export function createMerchantCheckoutService(
  requestControlPlaneJson: ControlPlaneJsonRequester,
): MerchantCheckoutService {
  return {
    getPublicShop(slug) {
      return requestControlPlaneJson(
        `/v1/shops/${encodeURIComponent(slug)}`,
        parsePublicShop,
        {
          headers: {
            Accept: "application/json",
          },
          method: "GET",
        },
      );
    },

    createMerchantProductOrder(cardProductId) {
      return requestControlPlaneJson(
        "/v1/merchant-product-orders",
        parseMerchantProductOrder,
        {
          body: JSON.stringify({
            card_product_id: cardProductId,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },

    getMerchantProductOrder(orderId) {
      return requestControlPlaneJson(
        `/v1/merchant-product-orders/${encodeURIComponent(orderId)}`,
        parseMerchantProductOrder,
        {
          headers: {
            Accept: "application/json",
          },
          method: "GET",
        },
      );
    },

    createMerchantProductOrderWechatPrepay(orderId, channel) {
      return requestControlPlaneJson(
        `/v1/merchant-product-orders/${encodeURIComponent(orderId)}/wechat-pay/prepay`,
        parseMerchantProductPrepay,
        {
          body: JSON.stringify({ channel }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },

    getMerchantPickup(pickupToken) {
      return requestControlPlaneJson(
        `/v1/pickups/${encodeURIComponent(pickupToken)}`,
        parseMerchantPickup,
        {
          headers: {
            Accept: "application/json",
          },
          method: "GET",
        },
      );
    },
  };
}
