import type { ControlPlaneMockState } from "./control-plane-fetch-state";
import {
  jsonResponse,
  parseRequestBody,
} from "./control-plane-fetch-route-utils";

export function handleMerchantRequest(
  state: ControlPlaneMockState,
  path: string,
  init?: RequestInit,
) {
  if (path === "/v1/merchant/workspace") {
    return jsonResponse(200, {
      data: {
        merchant_enabled: state.merchantShops.length > 0,
        tenant_id: "tenant_acme",
        shops: state.merchantShops,
        card_products: state.cardProducts,
        recent_orders: state.tradeOrders,
        trial_connections: state.trialConnections,
        recent_evaluations: state.relayEvaluations,
      },
    });
  }

  if (path === "/v1/merchant/shops" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const nextShop = {
      announcement:
        typeof body.announcement === "string" ? body.announcement : undefined,
      created_at: "2026-04-23T00:00:00Z",
      display_name:
        typeof body.display_name === "string"
          ? body.display_name
          : "Merchant Shop",
      dispute_rate_bps: 0,
      fulfillment_mode: "auto_card_secret",
      guarantee_deposit_usd: "0.00",
      identity_level: "l2_kyc",
      merchant_shop_id:
        typeof body.merchant_shop_id === "string"
          ? body.merchant_shop_id
          : `mshop_${state.merchantShops.length + 1}`,
      seller_alias: `seller-l2-${state.merchantShops.length + 1}`,
      slug:
        typeof body.slug === "string"
          ? body.slug
          : `merchant-shop-${state.merchantShops.length + 1}`,
      status: "active",
      tenant_id: "tenant_acme",
      updated_at: "2026-04-23T00:00:00Z",
      version: 1,
    };
    state.merchantShops = [...state.merchantShops, nextShop];
    return jsonResponse(200, nextShop);
  }

  if (path === "/v1/merchant/card-products" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const nextProduct = {
      card_product_id:
        typeof body.card_product_id === "string"
          ? body.card_product_id
          : `cardprod_${state.cardProducts.length + 1}`,
      created_at: "2026-04-23T00:00:00Z",
      delivery_kind: "direct_secret",
      description:
        typeof body.description === "string"
          ? body.description
          : "Merchant product",
      escrow_mode: "platform_ledger",
      evidence_requirement:
        "Replay-backed quality evaluation required before promoted listing.",
      face_value_usd:
        typeof body.face_value_usd === "string" ? body.face_value_usd : "1.00",
      inventory_count:
        typeof body.inventory_count === "number" ? body.inventory_count : 10,
      merchant_shop_id:
        typeof body.merchant_shop_id === "string"
          ? body.merchant_shop_id
          : (state.merchantShops[0]?.merchant_shop_id ?? "mshop_unknown"),
      retail_price_usd:
        typeof body.retail_price_usd === "string"
          ? body.retail_price_usd
          : "1.99",
      required_kyc_level: "l1_basic",
      review_status: "approved",
      risk_tier: "green",
      status: "active",
      supports_trial:
        typeof body.supports_trial === "boolean" ? body.supports_trial : true,
      tenant_id: "tenant_acme",
      title:
        typeof body.title === "string" ? body.title : "Merchant Card Product",
      updated_at: "2026-04-23T00:00:00Z",
      version: 1,
    };
    state.cardProducts = [...state.cardProducts, nextProduct];
    return jsonResponse(200, nextProduct);
  }

  if (path === "/v1/merchant/trial-connections" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const apiKey =
      typeof body.api_key === "string"
        ? body.api_key.trim()
        : "sk-trial-default";
    const nextConnection = {
      api_key_masked: `${apiKey.slice(0, 7)}...${apiKey.slice(-4)}`,
      created_at: "2026-04-23T00:00:00Z",
      endpoint_base_url:
        typeof body.endpoint_base_url === "string"
          ? body.endpoint_base_url
          : "https://relay.example.com/v1",
      last_verified_at: undefined,
      notes: typeof body.notes === "string" ? body.notes : undefined,
      provider_label:
        typeof body.provider_label === "string"
          ? body.provider_label
          : "Merchant Relay",
      status: "active",
      target_model:
        typeof body.target_model === "string"
          ? body.target_model
          : "claude-sonnet",
      tenant_id: "tenant_acme",
      trial_connection_id:
        typeof body.trial_connection_id === "string"
          ? body.trial_connection_id
          : `trialconn_${state.trialConnections.length + 1}`,
      updated_at: "2026-04-23T00:00:00Z",
      version: 1,
    };
    state.trialConnections = [...state.trialConnections, nextConnection];
    return jsonResponse(200, nextConnection);
  }

  if (path === "/v1/merchant/evaluations" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const trialConnectionId =
      typeof body.trial_connection_id === "string"
        ? body.trial_connection_id
        : (state.trialConnections[0]?.trial_connection_id ??
          "trialconn_unknown");
    const connection = state.trialConnections.find(
      (item) => item.trial_connection_id === trialConnectionId,
    );
    const nextEvaluation = {
      created_at: "2026-04-23T00:00:00Z",
      detected_channel: connection?.endpoint_base_url.includes("vertex")
        ? "vertex"
        : undefined,
      endpoint_base_url:
        connection?.endpoint_base_url ?? "https://relay.example.com/v1",
      estimated_tokens_saved: 2400,
      fingerprint_status: "pass",
      multimodal_status: "not_tested",
      overall_score: 88,
      protocol_status: "warning",
      provider_label: connection?.provider_label ?? "Merchant Relay",
      relay_evaluation_id: `reval_${state.relayEvaluations.length + 1}`,
      replay_capsule_id: `replay_${state.relayEvaluations.length + 1}`,
      runner_mode: "simulated",
      sample_request_count: 5,
      summary:
        "Replay-ready evaluation recorded. Review protocol consistency before spending live token budget.",
      target_model: connection?.target_model ?? "claude-sonnet",
      tenant_id: "tenant_acme",
      token_status: "warning",
      trial_connection_id: trialConnectionId,
      verdict: "warning",
    };
    state.relayEvaluations = [nextEvaluation, ...state.relayEvaluations];
    state.replayCapsules = [
      {
        replay_capsule_id: nextEvaluation.replay_capsule_id,
        request_id: `req_${nextEvaluation.replay_capsule_id}`,
        trace_id: `trace_${nextEvaluation.replay_capsule_id}`,
        route_receipt_id: `routercpt_${nextEvaluation.replay_capsule_id}`,
        config_snapshot_id: "cfgsnap_gateway_v1",
        redaction_tier: "structured_redacted",
        normalized_request_summary: {
          protocol_family: "openai_chat",
          model_alias: nextEvaluation.target_model,
          estimated_prompt_tokens: 480,
        },
        upstream_error_summary: {
          code: "protocol_shape_warning",
        },
      },
      ...state.replayCapsules,
    ];
    return jsonResponse(200, nextEvaluation);
  }

  if (
    path.startsWith("/v1/replay-capsules/") &&
    (!init?.method || init.method === "GET")
  ) {
    const replayCapsuleId = decodeURIComponent(path.split("/")[3] ?? "");
    const capsule = state.replayCapsules.find(
      (item) => item.replay_capsule_id === replayCapsuleId,
    );

    if (!capsule) {
      return jsonResponse(404, {
        error: {
          code: "not_found",
          message: "Replay capsule not found.",
          request_id: "req_test",
          retryable: false,
        },
      });
    }

    return jsonResponse(200, {
      replay_capsule: capsule,
    });
  }

  return null;
}
