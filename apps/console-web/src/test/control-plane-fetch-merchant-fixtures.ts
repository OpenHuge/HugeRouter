export type MerchantShopRecord = {
  announcement?: string;
  created_at: string;
  display_name: string;
  fulfillment_mode: string;
  merchant_shop_id: string;
  slug: string;
  status: string;
  tenant_id: string;
  updated_at: string;
  version: number;
};

export type CardProductRecord = {
  card_product_id: string;
  created_at: string;
  delivery_kind: string;
  description: string;
  face_value_usd: string;
  inventory_count: number;
  merchant_shop_id: string;
  retail_price_usd: string;
  status: string;
  supports_trial: boolean;
  tenant_id: string;
  title: string;
  updated_at: string;
  version: number;
};

export type TrialConnectionRecord = {
  api_key_masked: string;
  created_at: string;
  endpoint_base_url: string;
  last_verified_at?: string;
  notes?: string;
  provider_label: string;
  status: string;
  target_model: string;
  tenant_id: string;
  trial_connection_id: string;
  updated_at: string;
  version: number;
};

export type RelayEvaluationRecord = {
  created_at: string;
  detected_channel?: string;
  endpoint_base_url: string;
  estimated_tokens_saved: number;
  fingerprint_status: string;
  multimodal_status: string;
  overall_score: number;
  protocol_status: string;
  provider_label: string;
  relay_evaluation_id: string;
  replay_capsule_id: string;
  runner_mode: string;
  sample_request_count: number;
  summary: string;
  target_model: string;
  tenant_id: string;
  token_status: string;
  trial_connection_id: string;
  verdict: string;
};

export type ReplayCapsuleRecord = {
  config_snapshot_id: string;
  normalized_request_summary: {
    estimated_prompt_tokens: number;
    model_alias: string;
    protocol_family: string;
  };
  redaction_tier: string;
  replay_capsule_id: string;
  request_id: string;
  route_receipt_id: string;
  trace_id: string;
  upstream_error_summary?: {
    code: string;
  };
};

export const merchantShopsInitialState: MerchantShopRecord[] = [
  {
    merchant_shop_id: "mshop_acme",
    tenant_id: "tenant_acme",
    slug: "acme-small-shop",
    display_name: "Acme Small Shop",
    status: "active",
    announcement: "Fresh relay trial cards with replay-backed evaluation.",
    fulfillment_mode: "auto_card_secret",
    version: 1,
    created_at: "2026-04-22T00:00:00Z",
    updated_at: "2026-04-22T00:00:00Z",
  },
];

export const cardProductsInitialState: CardProductRecord[] = [
  {
    card_product_id: "cardprod_acme_trial",
    tenant_id: "tenant_acme",
    merchant_shop_id: "mshop_acme",
    title: "Claude Trial Pack",
    description:
      "Starter batch for relay verification and low-risk onboarding.",
    status: "active",
    inventory_count: 32,
    face_value_usd: "1.00",
    retail_price_usd: "1.99",
    delivery_kind: "direct_secret",
    supports_trial: true,
    version: 1,
    created_at: "2026-04-22T00:00:00Z",
    updated_at: "2026-04-22T00:00:00Z",
  },
];

export const trialConnectionsInitialState: TrialConnectionRecord[] = [
  {
    trial_connection_id: "trialconn_acme_relay",
    tenant_id: "tenant_acme",
    provider_label: "Acme Relay",
    endpoint_base_url: "https://relay.acme.example/v1",
    api_key_masked: "sk-tria...acme",
    target_model: "claude-sonnet",
    status: "active",
    notes: "Dedicated trial key only; never attach production traffic.",
    last_verified_at: "2026-04-22T00:00:00Z",
    version: 1,
    created_at: "2026-04-22T00:00:00Z",
    updated_at: "2026-04-22T00:00:00Z",
  },
];

export const relayEvaluationsInitialState: RelayEvaluationRecord[] = [
  {
    relay_evaluation_id: "reval_acme_relay",
    tenant_id: "tenant_acme",
    trial_connection_id: "trialconn_acme_relay",
    replay_capsule_id: "replay_acme_relay_eval",
    provider_label: "Acme Relay",
    endpoint_base_url: "https://relay.acme.example/v1",
    target_model: "claude-sonnet",
    runner_mode: "simulated",
    sample_request_count: 5,
    estimated_tokens_saved: 2400,
    overall_score: 82,
    verdict: "warning",
    fingerprint_status: "pass",
    protocol_status: "warning",
    token_status: "warning",
    multimodal_status: "not_tested",
    detected_channel: "vertex",
    summary:
      "Replay capsule captured; protocol and token behavior still need manual follow-up.",
    created_at: "2026-04-22T00:00:00Z",
  },
];

export const replayCapsulesInitialState: ReplayCapsuleRecord[] = [
  {
    replay_capsule_id: "replay_acme_relay_eval",
    request_id: "req_merchant_eval_acme",
    trace_id: "trace_merchant_eval_acme",
    route_receipt_id: "routercpt_acme_relay_eval",
    config_snapshot_id: "cfgsnap_gateway_v1",
    redaction_tier: "structured_redacted",
    normalized_request_summary: {
      protocol_family: "openai_chat",
      model_alias: "claude-sonnet",
      estimated_prompt_tokens: 480,
    },
    upstream_error_summary: {
      code: "provider_signature_mismatch",
    },
  },
];
