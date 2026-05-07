import {
  cardProductSchema,
  merchantShopSchema,
  merchantWorkspaceResponseSchema,
  relayEvaluationSchema,
  replayCapsuleResponseSchema,
  trialConnectionSchema,
} from "@huge-router/ts-shared-schema";
import { z } from "zod";
import type {
  CardProductView,
  MerchantPickupView,
  MerchantProductOrderView,
  MerchantProductPrepayResult,
  MerchantPublicShopView,
  MerchantShopView,
  MerchantWorkspaceData,
  RelayEvaluationView,
  ReplayCapsuleView,
  TrialConnectionView,
} from "./types";

type ControlPlaneJsonRequester = <T>(
  path: string,
  parser: (payload: unknown) => T,
  init?: RequestInit,
) => Promise<T>;

export type MerchantShopCreateInput = {
  merchantShopId: string;
  slug: string;
  displayName: string;
  announcement?: string;
};

export type CardProductCreateInput = {
  cardProductId: string;
  merchantShopId: string;
  projectId?: string;
  title: string;
  description: string;
  inventoryCount: number;
  faceValueUsd: string;
  retailPriceUsd: string;
  retailPriceCnyTotal?: number;
  saleEnabled?: boolean;
  deliveryIds?: string[];
  supportsTrial: boolean;
};

export type TrialConnectionCreateInput = {
  trialConnectionId: string;
  providerLabel: string;
  endpointBaseUrl: string;
  apiKey: string;
  targetModel: string;
  notes?: string;
};

export type RelayEvaluationCreateInput = {
  trialConnectionId: string;
};

export type MerchantService = {
  getMerchantWorkspace: () => Promise<MerchantWorkspaceData>;
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
  getReplayCapsule: (replayCapsuleId: string) => Promise<ReplayCapsuleView>;
  createMerchantShop: (
    input: MerchantShopCreateInput,
  ) => Promise<MerchantShopView>;
  createCardProduct: (
    input: CardProductCreateInput,
  ) => Promise<CardProductView>;
  createTrialConnection: (
    input: TrialConnectionCreateInput,
  ) => Promise<TrialConnectionView>;
  runRelayEvaluation: (
    input: RelayEvaluationCreateInput,
  ) => Promise<RelayEvaluationView>;
};

export type MerchantServiceDependencies = {
  requestControlPlaneJson: ControlPlaneJsonRequester;
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

export function toTrialConnectionView(
  connection: ReturnType<typeof trialConnectionSchema.parse>,
): TrialConnectionView {
  return {
    apiKeyMasked: connection.api_key_masked,
    createdAt: connection.created_at,
    endpointBaseUrl: connection.endpoint_base_url,
    lastVerifiedAt: connection.last_verified_at,
    notes: connection.notes,
    providerLabel: connection.provider_label,
    status: connection.status,
    targetModel: connection.target_model,
    trialConnectionId: connection.trial_connection_id,
    updatedAt: connection.updated_at,
    version: connection.version,
  };
}

export function toRelayEvaluationView(
  evaluation: ReturnType<typeof relayEvaluationSchema.parse>,
): RelayEvaluationView {
  return {
    createdAt: evaluation.created_at,
    detectedChannel: evaluation.detected_channel,
    endpointBaseUrl: evaluation.endpoint_base_url,
    estimatedTokensSaved: evaluation.estimated_tokens_saved,
    fingerprintStatus: evaluation.fingerprint_status,
    multimodalStatus: evaluation.multimodal_status,
    overallScore: evaluation.overall_score,
    protocolStatus: evaluation.protocol_status,
    providerLabel: evaluation.provider_label,
    relayEvaluationId: evaluation.relay_evaluation_id,
    replayCapsuleId: evaluation.replay_capsule_id,
    runnerMode: evaluation.runner_mode,
    sampleRequestCount: evaluation.sample_request_count,
    summary: evaluation.summary,
    targetModel: evaluation.target_model,
    tokenStatus: evaluation.token_status,
    trialConnectionId: evaluation.trial_connection_id,
    verdict: evaluation.verdict,
  };
}

export function parseMerchantWorkspace(payload: unknown): MerchantWorkspaceData {
  const parsed = merchantWorkspaceResponseSchema.parse(payload).data;

  return {
    cardProducts: parsed.card_products.map(toCardProductView),
    merchantEnabled: parsed.merchant_enabled,
    recentEvaluations: parsed.recent_evaluations.map(toRelayEvaluationView),
    shops: parsed.shops.map(toMerchantShopView),
    tenantId: parsed.tenant_id,
    trialConnections: parsed.trial_connections.map(toTrialConnectionView),
  };
}

export function parseReplayCapsule(payload: unknown): ReplayCapsuleView {
  const parsed = replayCapsuleResponseSchema.parse(payload).replay_capsule;

  return {
    configSnapshotId: parsed.config_snapshot_id,
    normalizedRequestSummary: {
      estimatedPromptTokens:
        parsed.normalized_request_summary.estimated_prompt_tokens,
      modelAlias: parsed.normalized_request_summary.model_alias,
      protocolFamily: parsed.normalized_request_summary.protocol_family,
    },
    redactionTier: parsed.redaction_tier,
    replayCapsuleId: parsed.replay_capsule_id,
    requestId: parsed.request_id,
    routeReceiptId: parsed.route_receipt_id,
    traceId: parsed.trace_id,
    upstreamErrorCode: parsed.upstream_error_summary?.code,
  };
}

export function createMerchantService({
  requestControlPlaneJson,
}: MerchantServiceDependencies): MerchantService {
  return {
    getMerchantWorkspace() {
      return requestControlPlaneJson(
        "/v1/merchant/workspace",
        parseMerchantWorkspace,
        {
          headers: {
            Accept: "application/json",
          },
          method: "GET",
        },
      );
    },

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

    getReplayCapsule(replayCapsuleId) {
      return requestControlPlaneJson(
        `/v1/replay-capsules/${encodeURIComponent(replayCapsuleId)}`,
        parseReplayCapsule,
        {
          headers: {
            Accept: "application/json",
          },
          method: "GET",
        },
      );
    },

    createMerchantShop(input) {
      return requestControlPlaneJson(
        "/v1/merchant/shops",
        (payload) => toMerchantShopView(merchantShopSchema.parse(payload)),
        {
          body: JSON.stringify({
            announcement: input.announcement,
            display_name: input.displayName,
            merchant_shop_id: input.merchantShopId,
            slug: input.slug,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },

    createCardProduct(input) {
      return requestControlPlaneJson(
        "/v1/merchant/card-products",
        (payload) => toCardProductView(cardProductSchema.parse(payload)),
        {
          body: JSON.stringify({
            card_product_id: input.cardProductId,
            description: input.description,
            delivery_ids: input.deliveryIds ?? [],
            face_value_usd: input.faceValueUsd,
            inventory_count: input.inventoryCount,
            merchant_shop_id: input.merchantShopId,
            project_id: input.projectId,
            retail_price_cny_total: input.retailPriceCnyTotal,
            retail_price_usd: input.retailPriceUsd,
            sale_enabled: input.saleEnabled ?? false,
            supports_trial: input.supportsTrial,
            title: input.title,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },

    createTrialConnection(input) {
      return requestControlPlaneJson(
        "/v1/merchant/trial-connections",
        (payload) => toTrialConnectionView(trialConnectionSchema.parse(payload)),
        {
          body: JSON.stringify({
            api_key: input.apiKey,
            endpoint_base_url: input.endpointBaseUrl,
            notes: input.notes,
            provider_label: input.providerLabel,
            target_model: input.targetModel,
            trial_connection_id: input.trialConnectionId,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },

    runRelayEvaluation(input) {
      return requestControlPlaneJson(
        "/v1/merchant/evaluations",
        (payload) => toRelayEvaluationView(relayEvaluationSchema.parse(payload)),
        {
          body: JSON.stringify({
            trial_connection_id: input.trialConnectionId,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },
  };
}
