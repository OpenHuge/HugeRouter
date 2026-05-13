import {
  cardProductSchema,
  merchantShopSchema,
  merchantWorkspaceResponseSchema,
  relayEvaluationSchema,
  replayCapsuleResponseSchema,
  trialConnectionSchema,
} from "@huge-router/ts-shared-schema";
import { createMerchantCheckoutService } from "./merchant-checkout-service";
import { toCardProductView, toMerchantShopView } from "./merchant-view-mappers";
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

export { toCardProductView, toMerchantShopView } from "./merchant-view-mappers";

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
  const checkoutService = createMerchantCheckoutService(requestControlPlaneJson);
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

    ...checkoutService,

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
