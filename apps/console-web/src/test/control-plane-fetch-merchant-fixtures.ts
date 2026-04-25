export type MerchantShopRecord = {
  announcement?: string;
  created_at: string;
  display_name: string;
  dispute_rate_bps: number;
  fulfillment_mode: string;
  guarantee_deposit_usd: string;
  identity_level: string;
  merchant_shop_id: string;
  seller_alias: string;
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
  escrow_mode: string;
  evidence_requirement: string;
  face_value_usd: string;
  inventory_count: number;
  merchant_shop_id: string;
  required_kyc_level: string;
  retail_price_usd: string;
  resource_type: string;
  review_status: string;
  risk_tier: string;
  status: string;
  supports_trial: boolean;
  tenant_id: string;
  title: string;
  updated_at: string;
  version: number;
};

export type TradeOrderRecord = {
  trade_order_id: string;
  tenant_id: string;
  merchant_shop_id: string;
  card_product_id: string;
  buyer_alias: string;
  seller_alias: string;
  state: string;
  escrow_mode: string;
  evidence_state: string;
  dispute_state: string;
  order_amount_usd: string;
  evidence_summary?: string;
  evidence_uri?: string;
  evidence_submitted_at?: string;
  created_at: string;
  updated_at: string;
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

export type DisclosureNoteRecord = {
  body: string;
  created_at: string;
  disclosure_note_id: string;
  risk_level: string;
  source_id: string;
  source_kind: string;
  tenant_id: string;
  title: string;
  visibility: string;
};

export const merchantShopsInitialState: MerchantShopRecord[] = [
  {
    merchant_shop_id: "mshop_acme",
    tenant_id: "tenant_acme",
    slug: "acme-small-shop",
    display_name: "Acme Small Shop",
    seller_alias: "acme-verified",
    identity_level: "l2_kyc",
    guarantee_deposit_usd: "250.00",
    dispute_rate_bps: 125,
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
    resource_type: "access_pack",
    title: "Claude Trial Pack",
    description:
      "Starter batch for relay verification and low-risk onboarding.",
    status: "active",
    inventory_count: 32,
    face_value_usd: "1.00",
    retail_price_usd: "1.99",
    delivery_kind: "direct_secret",
    supports_trial: true,
    risk_tier: "green",
    review_status: "approved",
    escrow_mode: "platform_ledger",
    required_kyc_level: "l1_basic",
    evidence_requirement:
      "Replay capsule and trial-key proof required before exposure.",
    version: 1,
    created_at: "2026-04-22T00:00:00Z",
    updated_at: "2026-04-22T00:00:00Z",
  },
];

export const tradeOrdersInitialState: TradeOrderRecord[] = [
  {
    trade_order_id: "tradeord_acme_trial_001",
    tenant_id: "tenant_acme",
    merchant_shop_id: "mshop_acme",
    card_product_id: "cardprod_acme_trial",
    buyer_alias: "buyer-l1-8291",
    seller_alias: "acme-verified",
    state: "escrow_funded",
    escrow_mode: "platform_ledger",
    evidence_state: "required",
    dispute_state: "none",
    order_amount_usd: "1.99",
    evidence_summary:
      "Escrow funded; awaiting replay-backed delivery evidence.",
    evidence_uri: "internal://orders/tradeord_acme_trial_001/evidence",
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

export const disclosureNotesInitialState: DisclosureNoteRecord[] = [
  {
    disclosure_note_id: "disc_acme_relay_watch",
    tenant_id: "tenant_acme",
    source_kind: "relay_evaluation",
    source_id: "reval_acme_relay",
    title: "Relay evaluation requires follow-up",
    body: "Replay evidence flagged protocol and token behavior for operator review.",
    risk_level: "watch",
    visibility: "operator_only",
    created_at: "2026-04-22T00:00:00Z",
  },
];
