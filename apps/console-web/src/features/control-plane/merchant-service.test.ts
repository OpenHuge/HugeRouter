import { describe, expect, it } from "vitest";
import {
  createMerchantService,
  parseMerchantWorkspace,
  parseReplayCapsule,
} from "./merchant-service";

const now = "2026-05-02T00:00:00Z";

const shopPayload = {
  announcement: "Launch week",
  created_at: now,
  display_name: "Acme Cards",
  fulfillment_mode: "auto_card_secret",
  merchant_shop_id: "mshop_acme",
  slug: "acme-cards",
  status: "active",
  tenant_id: "tenant_acme",
  updated_at: now,
  version: 1,
};

const cardProductPayload = {
  card_product_id: "cardprod_pro_month",
  created_at: now,
  delivery_kind: "direct_secret",
  description: "One month pro access",
  face_value_usd: "20.00",
  inventory_count: 8,
  merchant_shop_id: "mshop_acme",
  retail_price_usd: "12.00",
  status: "active",
  supports_trial: true,
  tenant_id: "tenant_acme",
  title: "Pro Month",
  updated_at: now,
  version: 1,
};

const trialConnectionPayload = {
  api_key_masked: "sk-...abcd",
  created_at: now,
  endpoint_base_url: "https://relay.example.com",
  last_verified_at: now,
  notes: "Primary relay",
  provider_label: "OpenAI",
  status: "active",
  target_model: "gpt-5.1",
  tenant_id: "tenant_acme",
  trial_connection_id: "trialconn_primary",
  updated_at: now,
  version: 1,
};

const relayEvaluationPayload = {
  created_at: now,
  detected_channel: "official_api",
  endpoint_base_url: "https://relay.example.com",
  estimated_tokens_saved: 120,
  fingerprint_status: "pass",
  multimodal_status: "not_tested",
  overall_score: 93,
  protocol_status: "pass",
  provider_label: "OpenAI",
  relay_evaluation_id: "reval_primary",
  replay_capsule_id: "replay_primary",
  runner_mode: "replay",
  sample_request_count: 3,
  summary: "Healthy relay",
  target_model: "gpt-5.1",
  tenant_id: "tenant_acme",
  token_status: "pass",
  trial_connection_id: "trialconn_primary",
  verdict: "healthy",
};

const replayCapsulePayload = {
  replay_capsule: {
    config_snapshot_id: "cfgsnap_gateway_v1",
    normalized_request_summary: {
      estimated_prompt_tokens: 64,
      model_alias: "gpt-5.1",
      protocol_family: "openai_chat",
    },
    redaction_tier: "metadata_only",
    replay_capsule_id: "replay_primary",
    request_id: "req_001",
    route_receipt_id: "routercpt_primary",
    trace_id: "trace_001",
    upstream_error_summary: {
      code: "rate_limited",
    },
  },
};

function parseJsonBody(init?: RequestInit) {
  if (typeof init?.body !== "string") {
    throw new Error("Expected a JSON string request body.");
  }

  return JSON.parse(init.body) as Record<string, unknown>;
}

describe("merchant service", () => {
  it("maps merchant workspace and replay capsule payloads into view models", () => {
    expect(
      parseMerchantWorkspace({
        data: {
          card_products: [cardProductPayload],
          merchant_enabled: true,
          recent_evaluations: [relayEvaluationPayload],
          shops: [shopPayload],
          tenant_id: "tenant_acme",
          trial_connections: [trialConnectionPayload],
        },
      }),
    ).toMatchObject({
      cardProducts: [{ cardProductId: "cardprod_pro_month" }],
      merchantEnabled: true,
      recentEvaluations: [{ relayEvaluationId: "reval_primary" }],
      shops: [{ merchantShopId: "mshop_acme" }],
      tenantId: "tenant_acme",
      trialConnections: [{ trialConnectionId: "trialconn_primary" }],
    });

    expect(parseReplayCapsule(replayCapsulePayload)).toEqual({
      configSnapshotId: "cfgsnap_gateway_v1",
      normalizedRequestSummary: {
        estimatedPromptTokens: 64,
        modelAlias: "gpt-5.1",
        protocolFamily: "openai_chat",
      },
      redactionTier: "metadata_only",
      replayCapsuleId: "replay_primary",
      requestId: "req_001",
      routeReceiptId: "routercpt_primary",
      traceId: "trace_001",
      upstreamErrorCode: "rate_limited",
    });
  });

  it("keeps merchant HTTP endpoints and mutation bodies behind the service boundary", async () => {
    const calls: { init?: RequestInit; path: string }[] = [];
    const service = createMerchantService({
      requestControlPlaneJson: async (path, parser, init) => {
        calls.push({ init, path });

        if (path === "/v1/merchant/workspace") {
          return parser({
            data: {
              card_products: [cardProductPayload],
              merchant_enabled: true,
              recent_evaluations: [relayEvaluationPayload],
              shops: [shopPayload],
              tenant_id: "tenant_acme",
              trial_connections: [trialConnectionPayload],
            },
          });
        }

        if (path.startsWith("/v1/replay-capsules/")) {
          return parser(replayCapsulePayload);
        }

        if (path === "/v1/merchant/shops") {
          return parser(shopPayload);
        }

        if (path === "/v1/merchant/card-products") {
          return parser(cardProductPayload);
        }

        if (path === "/v1/merchant/trial-connections") {
          return parser(trialConnectionPayload);
        }

        return parser(relayEvaluationPayload);
      },
    });

    await service.getMerchantWorkspace();
    await service.getReplayCapsule("replay_primary");
    await service.createMerchantShop({
      announcement: "Launch week",
      displayName: "Acme Cards",
      merchantShopId: "mshop_acme",
      slug: "acme-cards",
    });
    await service.createCardProduct({
      cardProductId: "cardprod_pro_month",
      description: "One month pro access",
      faceValueUsd: "20.00",
      inventoryCount: 8,
      merchantShopId: "mshop_acme",
      retailPriceUsd: "12.00",
      supportsTrial: true,
      title: "Pro Month",
    });
    await service.createTrialConnection({
      apiKey: "sk-test",
      endpointBaseUrl: "https://relay.example.com",
      notes: "Primary relay",
      providerLabel: "OpenAI",
      targetModel: "gpt-5.1",
      trialConnectionId: "trialconn_primary",
    });
    await service.runRelayEvaluation({
      trialConnectionId: "trialconn_primary",
    });

    expect(calls.map((call) => [call.path, call.init?.method])).toEqual([
      ["/v1/merchant/workspace", "GET"],
      ["/v1/replay-capsules/replay_primary", "GET"],
      ["/v1/merchant/shops", "POST"],
      ["/v1/merchant/card-products", "POST"],
      ["/v1/merchant/trial-connections", "POST"],
      ["/v1/merchant/evaluations", "POST"],
    ]);
    expect(parseJsonBody(calls[2]?.init)).toMatchObject({
      display_name: "Acme Cards",
      merchant_shop_id: "mshop_acme",
    });
    expect(parseJsonBody(calls[4]?.init)).toMatchObject({
      api_key: "sk-test",
      trial_connection_id: "trialconn_primary",
    });
    expect(parseJsonBody(calls[5]?.init)).toEqual({
      trial_connection_id: "trialconn_primary",
    });
  });
});
