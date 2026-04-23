import { beforeEach, afterEach, vi } from "vitest";

const tenantsResponse = {
  data: [
    {
      tenant_id: "tenant_platform",
      slug: "platform-admin",
      display_name: "Platform Admin",
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      tenant_id: "tenant_acme",
      slug: "acme-retail",
      display_name: "Acme Retail",
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      tenant_id: "tenant_northstar",
      slug: "northstar-labs",
      display_name: "Northstar Labs",
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
  ],
};

const projectsResponse = {
  data: [
    {
      project_id: "proj_core",
      tenant_id: "tenant_acme",
      slug: "core-gateway",
      display_name: "Core Gateway",
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      project_id: "proj_acme_ops",
      tenant_id: "tenant_acme",
      slug: "retail-ops",
      display_name: "Retail Operations Control",
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      project_id: "proj_acme_support",
      tenant_id: "tenant_acme",
      slug: "store-support",
      display_name: "Store Support Agent",
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      project_id: "proj_ns_research",
      tenant_id: "tenant_northstar",
      slug: "research-qa",
      display_name: "Research QA",
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
  ],
};

const providerResourcesResponse = {
  data: [
    {
      provider_resource_id: "prvrsrc_openai_primary",
      tenant_id: "tenant_acme",
      project_id: "proj_core",
      provider_id: "openai",
      name: "OpenAI Primary",
      status: "active",
      provenance_class: "official_api",
      credential_owner_type: "platform",
      deployment_scope: "shared",
      region: "us-east-1",
      endpoint_base_url: "https://api.openai.com/v1",
      auth_kind: "api_key",
      health_state: "healthy",
      health_message: "probe latency within SLO",
      quarantine_reason: undefined,
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: true,
        supports_json_mode: true,
        supports_realtime: false,
        supports_response_model_metadata: true,
      },
      supported_protocol_families: ["openai_chat", "openai_responses"],
      is_transit_gateway: false,
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      provider_resource_id: "prvrsrc_openai_backup",
      tenant_id: "tenant_acme",
      project_id: "proj_core",
      provider_id: "openai",
      name: "OpenAI Backup",
      status: "active",
      provenance_class: "official_api",
      credential_owner_type: "platform",
      deployment_scope: "shared",
      region: "us-west-2",
      endpoint_base_url: "https://api.openai.com/v1",
      auth_kind: "api_key",
      health_state: "healthy",
      health_message: "backup target healthy",
      quarantine_reason: undefined,
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: true,
        supports_json_mode: true,
        supports_realtime: false,
        supports_response_model_metadata: true,
      },
      supported_protocol_families: ["openai_chat", "openai_responses"],
      is_transit_gateway: false,
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      provider_resource_id: "prvrsrc_openai_research",
      tenant_id: "tenant_northstar",
      project_id: "proj_ns_research",
      provider_id: "openai",
      name: "OpenAI Research",
      status: "active",
      provenance_class: "official_api",
      credential_owner_type: "tenant",
      deployment_scope: "tenant_dedicated",
      region: "us-west-2",
      endpoint_base_url: "https://api.openai.com/v1",
      auth_kind: "api_key",
      health_state: "degraded",
      health_message: "research endpoint latency elevated",
      quarantine_reason: undefined,
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: false,
        supports_json_mode: true,
        supports_realtime: false,
        supports_response_model_metadata: true,
      },
      supported_protocol_families: ["openai_chat"],
      is_transit_gateway: false,
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
  ],
};

const providerResourcesInitialState = structuredClone(providerResourcesResponse.data);

const routePoliciesResponse = {
  data: [
    {
      route_policy_id: "routepol_openai_chat_default",
      tenant_id: "tenant_acme",
      display_name: "Acme Reasoning Fast",
      protocol_family: "openai_chat",
      model_alias: "reasoning-fast",
      required_capabilities: ["json_mode"],
      preferred_regions: ["us-east-1"],
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      route_policy_id: "routepol_acme_support",
      tenant_id: "tenant_acme",
      display_name: "Acme Support Safe",
      protocol_family: "openai_chat",
      model_alias: "support-safe",
      required_capabilities: ["json_mode"],
      preferred_regions: ["us-west-2"],
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      route_policy_id: "routepol_northstar_research",
      tenant_id: "tenant_northstar",
      display_name: "Northstar Research",
      protocol_family: "openai_chat",
      model_alias: "research-fast",
      required_capabilities: ["json_mode"],
      preferred_regions: ["us-west-2"],
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
  ],
};

const routePoliciesInitialState = structuredClone(routePoliciesResponse.data);

const configSnapshotResponse = {
  config_snapshot: {
    config_snapshot_id: "cfgsnap_gateway_v1",
    tenant_id: "tenant_acme",
    project_id: "proj_core",
    revision: 1,
    status: "active",
    activated_at: "2026-04-22T00:00:00Z",
    provider_resource_ids: ["prvrsrc_openai_primary", "prvrsrc_openai_backup"],
    route_policy_id: "routepol_openai_chat_default",
    budget_policy_id: "budgetpol_default",
  },
};

const configSnapshotsResponse = {
  data: [
    {
      config_snapshot_id: "cfgsnap_gateway_v2",
      tenant_id: "tenant_acme",
      project_id: "proj_core",
      revision: 3,
      status: "draft",
      activated_at: "2026-04-20T00:00:00Z",
      provider_resource_ids: ["prvrsrc_openai_primary"],
      route_policy_id: "routepol_openai_chat_default",
      budget_policy_id: "budgetpol_default",
    },
    {
      config_snapshot_id: "cfgsnap_gateway_v1",
      tenant_id: "tenant_acme",
      project_id: "proj_core",
      revision: 1,
      status: "active",
      activated_at: "2026-04-22T00:00:00Z",
      provider_resource_ids: [
        "prvrsrc_openai_primary",
        "prvrsrc_openai_backup",
      ],
      route_policy_id: "routepol_openai_chat_default",
      budget_policy_id: "budgetpol_default",
    },
  ],
};

const configSnapshotsInitialState = structuredClone(configSnapshotsResponse.data);

const routeReceiptsResponse = {
  data: [
    {
      route_receipt: {
        route_receipt_id: "routercpt_openai_primary_recent",
        tenant_id: "tenant_acme",
        project_id: "proj_core",
        route_policy_id: "routepol_openai_chat_default",
        request_id: "req_openai_primary",
        trace_id: "trace_openai_primary",
        protocol_family: "openai_chat",
        model_alias: "reasoning-fast",
        config_snapshot_id: "cfgsnap_gateway_v1",
        admission_result: "admitted",
        selected_target: "prvrsrc_openai_primary",
        excluded_targets: [
          {
            provider_resource_id: "prvrsrc_openai_backup",
            reason_code: "provider_region_mismatch",
            reason: "provider_region_mismatch",
          },
        ],
        score_breakdown: {
          latency: 0.82,
          cost: 0.66,
          health: 0.9,
          trust: 1,
        },
        fallback_transitions: [
          {
            from_provider_resource_id: "prvrsrc_openai_backup",
            to_provider_resource_id: "prvrsrc_openai_primary",
            reason: "replayed_after_transient_timeout",
          },
        ],
        failure_reason: undefined,
        created_at: "2026-04-22T13:00:00Z",
      },
    },
    {
      route_receipt: {
        route_receipt_id: "routercpt_openai_rejection",
        tenant_id: "tenant_northstar",
        project_id: "proj_ns_research",
        route_policy_id: "routepol_northstar_research",
        request_id: "req_openai_rejection",
        trace_id: "trace_openai_rejection",
        protocol_family: "openai_chat",
        model_alias: "research-fast",
        config_snapshot_id: "cfgsnap_gateway_v1",
        admission_result: "rejected_policy",
        excluded_targets: [
          {
            provider_resource_id: "prvrsrc_openai_research",
            reason_code: "provider_quarantined",
            reason: "provider_quarantined",
          },
        ],
        score_breakdown: {
          latency: 0,
          cost: 0,
          health: 0,
          trust: 0,
        },
        fallback_transitions: [],
        failure_reason: "No policy match for requested capabilities.",
        normalized_error: {
          code: "routing_rejected",
          message: "No policy match for requested capabilities.",
          request_id: "req_openai_rejection",
          retryable: false,
          upstream_code: "policy_deny",
          upstream_status_code: 403,
          validation_issues: [
            {
              field: "required_capabilities",
              message: "tool_calling is required for this protocol",
            },
          ],
          details: {
            policy: "require_tool_calling",
          },
        },
        created_at: "2026-04-22T10:00:00Z",
      },
    },
  ],
};

const usageSummaryResponse = {
  data: {
    tenant_id: "tenant_acme",
    project_id: "proj_core",
    window_start: "2026-04-01T00:00:00Z",
    window_end: "2026-04-30T23:59:59Z",
    currency: "USD",
    event_count: 14,
    input_tokens: 18420,
    output_tokens: 6245,
    cached_input_tokens: 1220,
    provider_cost: {
      currency: "USD",
      amount: "0.124500",
    },
    billable_price: {
      currency: "USD",
      amount: "0.152025",
    },
  },
};

const usageBreakdownResponse = {
  data: [
    {
      bucket: "openai",
      provider_id: "openai",
      input_tokens: 10000,
      output_tokens: 4000,
      cached_input_tokens: 500,
      provider_cost: {
        currency: "USD",
        amount: "0.082000",
      },
      billable_price: {
        currency: "USD",
        amount: "0.098400",
      },
    },
    {
      bucket: "reasoning-fast",
      model_alias: "reasoning-fast",
      input_tokens: 8420,
      output_tokens: 2245,
      cached_input_tokens: 720,
      provider_cost: {
        currency: "USD",
        amount: "0.042500",
      },
      billable_price: {
        currency: "USD",
        amount: "0.053625",
      },
    },
  ],
  next_cursor: "2",
};

const balanceProjectionResponse = {
  data: {
    tenant_id: "tenant_acme",
    project_id: "proj_core",
    currency: "USD",
    provider_cost_total: {
      currency: "USD",
      amount: "1.244000",
    },
    billable_total: {
      currency: "USD",
      amount: "1.540000",
    },
    configured_budget: {
      currency: "USD",
      amount: "75.000000",
    },
    remaining_budget: {
      currency: "USD",
      amount: "73.460000",
    },
    threshold_status: "ok",
    last_projected_at: "2026-04-22T13:00:00Z",
    projection_lag_seconds: 18,
  },
};

const billingExportResponse = {
  data: {
    export_job_id: "export_123",
    status: "completed",
    format: "csv",
    requested_at: "2026-04-22T13:00:10Z",
    completed_at: "2026-04-22T13:00:20Z",
    tenant_id: "tenant_acme",
    project_id: "proj_core",
  },
};

type BillingExportJobState = {
  completed_at?: string;
  export_job_id: string;
  format: string;
  project_id?: string;
  requested_at: string;
  status: string;
  tenant_id?: string;
};

const billingExportInitialState: BillingExportJobState[] = [
  structuredClone(billingExportResponse.data),
];

const routeReceiptDiagnosticsById: Record<string, unknown> = {
  routercpt_openai_primary_recent: {
    route_receipt: routeReceiptsResponse.data[0].route_receipt,
    decision_timeline: [
      {
        stage: "admission",
        status: "passed",
        message: "Tenant policy accepted request",
        score: 1,
        notes: ["all constraints satisfied"],
      },
      {
        stage: "candidate_selection",
        status: "passed",
        message: "Selected OpenAI Primary",
        score: 0.91,
        notes: [],
      },
    ],
    policy_checks: [
      {
        policy_id: "routepol_openai_chat_default",
        status: "passed",
        reason: "policy satisfied",
      },
    ],
    provider_attempts: [
      {
        provider_resource_id: "prvrsrc_openai_primary",
        attempt: 1,
        status: "succeeded",
        started_at: "2026-04-22T13:00:01Z",
        finished_at: "2026-04-22T13:00:02Z",
        latency_ms: 1100,
        reason: "succeeded with output",
      },
    ],
    metadata: {
      candidate_pool_size: "2",
      policy_cache_hit: "true",
    },
  },
};

const routeDiagnosticsByPolicyId: Record<string, unknown> = {
  routepol_openai_chat_default: {
    route_policy: routePoliciesResponse.data[0],
    active_snapshot: configSnapshotResponse.config_snapshot,
    active_snapshot_matches_route_policy: true,
    last_route_receipt: {
      route_receipt_id: "routercpt_openai_primary_recent",
      admission_result: "admitted",
      selected_target: "prvrsrc_openai_primary",
      created_at: "2026-04-22T13:00:00Z",
    },
    recent_receipts: [
      {
        route_receipt_id: "routercpt_openai_primary_recent",
        admission_result: "admitted",
        selected_target: "prvrsrc_openai_primary",
        created_at: "2026-04-22T13:00:00Z",
      },
    ],
    targets: [
      {
        provider_resource: providerResourcesResponse.data[0],
        decision: "selected",
        in_active_snapshot: true,
        supports_protocol_family: true,
        capability_gaps: [],
        reason_code: "selected_recent_receipt",
        reason: "Selected by the most recent route receipt.",
        recent_receipt_id: "routercpt_openai_primary_recent",
      },
      {
        provider_resource: providerResourcesResponse.data[1],
        decision: "excluded",
        in_active_snapshot: true,
        supports_protocol_family: true,
        capability_gaps: [],
        reason_code: "provider_region_mismatch",
        reason: "provider_region_mismatch",
        recent_receipt_id: "routercpt_openai_primary_recent",
        recent_receipt_reason: "provider_region_mismatch",
      },
    ],
  },
};

let providerResourcesState = structuredClone(providerResourcesInitialState);
let routePoliciesState = structuredClone(routePoliciesInitialState);
let configSnapshotsState = structuredClone(configSnapshotsInitialState);
let billingExportJobsState = structuredClone(billingExportInitialState);
let billingExportPollCount = 0;

let apiKeysState = [
  {
    api_key_id: "key_acme_primary",
    tenant_id: "tenant_acme",
    display_name: "Acme Primary Key",
    key_prefix: "ak-prim",
    provider_resource_id: "prvrsrc_openai_primary",
    can_revoke: true,
    is_active: true,
    created_at: "2026-04-01T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    version: 1,
  },
  {
    api_key_id: "key_acme_secondary",
    tenant_id: "tenant_acme",
    display_name: "Acme Secondary Key",
    key_prefix: "ak-sec",
    provider_resource_id: "prvrsrc_openai_backup",
    can_revoke: true,
    is_active: true,
    created_at: "2026-03-01T00:00:00Z",
    updated_at: "2026-03-10T00:00:00Z",
    version: 1,
  },
  {
    api_key_id: "key_northstar_research",
    tenant_id: "tenant_northstar",
    display_name: "Northstar Research Key",
    key_prefix: "ak-ns",
    provider_resource_id: "prvrsrc_openai_research",
    can_revoke: false,
    is_active: true,
    created_at: "2026-03-14T00:00:00Z",
    updated_at: "2026-03-14T00:00:00Z",
    version: 1,
  },
];

const apiKeysInitialState = [...apiKeysState];

type MerchantShopFixture = {
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

type CardProductFixture = {
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

type TrialConnectionFixture = {
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

type RelayEvaluationFixture = {
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

const merchantShopsInitialState: MerchantShopFixture[] = [
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

const cardProductsInitialState: CardProductFixture[] = [
  {
    card_product_id: "cardprod_acme_trial",
    tenant_id: "tenant_acme",
    merchant_shop_id: "mshop_acme",
    title: "Claude Trial Pack",
    description: "Starter batch for relay verification and low-risk onboarding.",
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

const trialConnectionsInitialState: TrialConnectionFixture[] = [
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

const relayEvaluationsInitialState: RelayEvaluationFixture[] = [
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
    summary: "Replay capsule captured; protocol and token behavior still need manual follow-up.",
    created_at: "2026-04-22T00:00:00Z",
  },
];

let merchantShopsState = structuredClone(merchantShopsInitialState);
let cardProductsState = structuredClone(cardProductsInitialState);
let trialConnectionsState = structuredClone(trialConnectionsInitialState);
let relayEvaluationsState = structuredClone(relayEvaluationsInitialState);

const emptyResponse = (status: number) =>
  new Response("", {
    status,
    headers: {
      "content-type": "application/json",
    },
  });

const routeSimulationResponse = {
  simulation_id: "sim_reasoning-fast",
  config_snapshot_id: "cfgsnap_gateway_v1",
  admission_result: "admitted",
  eligible_candidates: [
    {
      provider_resource_id: "prvrsrc_openai_primary",
      score_breakdown: {
        latency: 0.95,
        cost: 0.7,
        health: 1,
        trust: 1,
      },
    },
  ],
  excluded_candidates: [],
  selected_target: "prvrsrc_openai_primary",
  estimated_cost: {
    currency: "USD",
    amount: "0.000210",
  },
};

function jsonResponse(status: number, payload: unknown) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: {
      "content-type": "application/json",
    },
  });
}

function resolvePath(input: RequestInfo | URL) {
  if (typeof input === "string") {
    return new URL(input, "http://127.0.0.1").pathname;
  }

  if (input instanceof URL) {
    return input.pathname;
  }

  return new URL(input.url, "http://127.0.0.1").pathname;
}

function parseRequestBody(init?: RequestInit) {
  if (!init?.body || typeof init.body !== "string") {
    return null;
  }

  return JSON.parse(init.body) as Record<string, unknown>;
}

export function createControlPlaneFetchMock() {
  apiKeysState = [...apiKeysInitialState];
  billingExportJobsState = structuredClone(billingExportInitialState);
  billingExportPollCount = 0;
  cardProductsState = structuredClone(cardProductsInitialState);
  configSnapshotsState = structuredClone(configSnapshotsInitialState);
  merchantShopsState = structuredClone(merchantShopsInitialState);
  providerResourcesState = structuredClone(providerResourcesInitialState);
  relayEvaluationsState = structuredClone(relayEvaluationsInitialState);
  routePoliciesState = structuredClone(routePoliciesInitialState);
  trialConnectionsState = structuredClone(trialConnectionsInitialState);

  return vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const path = resolvePath(input);

    if (path === "/v1/tenants") {
      return Promise.resolve(jsonResponse(200, tenantsResponse));
    }

    if (path === "/v1/projects") {
      return Promise.resolve(jsonResponse(200, projectsResponse));
    }

    if (path === "/v1/merchant/workspace") {
      return Promise.resolve(
        jsonResponse(200, {
          data: {
            merchant_enabled: merchantShopsState.length > 0,
            tenant_id: "tenant_acme",
            shops: merchantShopsState,
            card_products: cardProductsState,
            trial_connections: trialConnectionsState,
            recent_evaluations: relayEvaluationsState,
          },
        }),
      );
    }

    if (path === "/v1/merchant/shops" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      const nextShop: MerchantShopFixture = {
        announcement:
          typeof body.announcement === "string" ? body.announcement : undefined,
        created_at: "2026-04-23T00:00:00Z",
        display_name:
          typeof body.display_name === "string"
            ? body.display_name
            : "Merchant Shop",
        fulfillment_mode: "auto_card_secret",
        merchant_shop_id:
          typeof body.merchant_shop_id === "string"
            ? body.merchant_shop_id
            : `mshop_${merchantShopsState.length + 1}`,
        slug:
          typeof body.slug === "string"
            ? body.slug
            : `merchant-shop-${merchantShopsState.length + 1}`,
        status: "active",
        tenant_id: "tenant_acme",
        updated_at: "2026-04-23T00:00:00Z",
        version: 1,
      };
      merchantShopsState = [...merchantShopsState, nextShop];
      return Promise.resolve(jsonResponse(200, nextShop));
    }

    if (path === "/v1/merchant/card-products" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      const nextProduct: CardProductFixture = {
        card_product_id:
          typeof body.card_product_id === "string"
            ? body.card_product_id
            : `cardprod_${cardProductsState.length + 1}`,
        created_at: "2026-04-23T00:00:00Z",
        delivery_kind: "direct_secret",
        description:
          typeof body.description === "string"
            ? body.description
            : "Merchant product",
        face_value_usd:
          typeof body.face_value_usd === "string" ? body.face_value_usd : "1.00",
        inventory_count:
          typeof body.inventory_count === "number" ? body.inventory_count : 10,
        merchant_shop_id:
          typeof body.merchant_shop_id === "string"
            ? body.merchant_shop_id
            : merchantShopsState[0]?.merchant_shop_id ?? "mshop_unknown",
        retail_price_usd:
          typeof body.retail_price_usd === "string"
            ? body.retail_price_usd
            : "1.99",
        status: "active",
        supports_trial:
          typeof body.supports_trial === "boolean" ? body.supports_trial : true,
        tenant_id: "tenant_acme",
        title:
          typeof body.title === "string" ? body.title : "Merchant Card Product",
        updated_at: "2026-04-23T00:00:00Z",
        version: 1,
      };
      cardProductsState = [...cardProductsState, nextProduct];
      return Promise.resolve(jsonResponse(200, nextProduct));
    }

    if (path === "/v1/merchant/trial-connections" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      const apiKey =
        typeof body.api_key === "string" ? body.api_key.trim() : "sk-trial-default";
      const nextConnection: TrialConnectionFixture = {
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
            : `trialconn_${trialConnectionsState.length + 1}`,
        updated_at: "2026-04-23T00:00:00Z",
        version: 1,
      };
      trialConnectionsState = [...trialConnectionsState, nextConnection];
      return Promise.resolve(jsonResponse(200, nextConnection));
    }

    if (path === "/v1/merchant/evaluations" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      const trialConnectionId =
        typeof body.trial_connection_id === "string"
          ? body.trial_connection_id
          : trialConnectionsState[0]?.trial_connection_id ?? "trialconn_unknown";
      const connection = trialConnectionsState.find(
        (item) => item.trial_connection_id === trialConnectionId,
      );
      const nextEvaluation: RelayEvaluationFixture = {
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
        relay_evaluation_id: `reval_${relayEvaluationsState.length + 1}`,
        replay_capsule_id: `replay_${relayEvaluationsState.length + 1}`,
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
      relayEvaluationsState = [nextEvaluation, ...relayEvaluationsState];
      return Promise.resolve(jsonResponse(200, nextEvaluation));
    }

    if (path === "/v1/provider-resources" && (!init?.method || init.method === "GET")) {
      return Promise.resolve(jsonResponse(200, { data: providerResourcesState }));
    }

    if (path === "/v1/provider-resources" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      providerResourcesState = [...providerResourcesState, body as (typeof providerResourcesResponse)["data"][number]];
      return Promise.resolve(jsonResponse(200, body));
    }

    if (
      path.startsWith("/v1/provider-resources/") &&
      !path.endsWith("/disable") &&
      init?.method === "PUT"
    ) {
      const providerResourceId = decodeURIComponent(path.split("/")[3] ?? "");
      const body = parseRequestBody(init) ?? {};
      const expectedVersion = Number(body.expected_version ?? 0);
      const index = providerResourcesState.findIndex(
        (provider) => provider.provider_resource_id === providerResourceId,
      );

      if (index < 0) {
        return Promise.resolve(
          jsonResponse(404, {
            error: {
              code: "provider_resource_not_found",
              message: "Provider resource not found.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      if (providerResourcesState[index]?.version !== expectedVersion) {
        return Promise.resolve(
          jsonResponse(409, {
            error: {
              code: "provider_resource_version_conflict",
              message: "Provider resource version is stale.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      const updated = {
        ...providerResourcesState[index],
        ...body,
        provider_resource_id: providerResourceId,
        version: expectedVersion + 1,
      };
      providerResourcesState[index] = updated;
      return Promise.resolve(jsonResponse(200, updated));
    }

    if (
      path.startsWith("/v1/provider-resources/") &&
      path.endsWith("/disable") &&
      init?.method === "POST"
    ) {
      const providerResourceId = decodeURIComponent(path.split("/")[3] ?? "");
      const body = parseRequestBody(init) ?? {};
      const expectedVersion = Number(body.expected_version ?? 0);
      const index = providerResourcesState.findIndex(
        (provider) => provider.provider_resource_id === providerResourceId,
      );

      if (index < 0) {
        return Promise.resolve(
          jsonResponse(404, {
            error: {
              code: "provider_resource_not_found",
              message: "Provider resource not found.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      if (providerResourcesState[index]?.version !== expectedVersion) {
        return Promise.resolve(
          jsonResponse(409, {
            error: {
              code: "provider_resource_version_conflict",
              message: "Provider resource version is stale.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      const disabled = {
        ...providerResourcesState[index],
        status: "disabled",
        health_state: "disabled",
        version: expectedVersion + 1,
      };
      providerResourcesState[index] = disabled;
      return Promise.resolve(jsonResponse(200, disabled));
    }

    if (path === "/v1/route-policies" && (!init?.method || init.method === "GET")) {
      return Promise.resolve(jsonResponse(200, { data: routePoliciesState }));
    }

    if (path === "/v1/route-policies" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      const capabilities = Array.isArray(body.required_capabilities)
        ? body.required_capabilities
        : [];
      const unsupportedCapabilities = capabilities.filter(
        (capability) =>
          ![
            "streaming",
            "tool_calling",
            "tool_related",
            "json_mode",
            "chat_completions",
            "realtime",
            "response_model_metadata",
          ].includes(
            String(capability),
          ),
      );

      if (unsupportedCapabilities.length > 0) {
        return Promise.resolve(
          jsonResponse(400, {
            error: {
              code: "route_policy_compatibility_invalid",
              message: "Route policy compatibility validation failed.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      routePoliciesState = [...routePoliciesState, body as (typeof routePoliciesResponse)["data"][number]];
      return Promise.resolve(jsonResponse(200, body));
    }

    if (
      path.startsWith("/v1/route-policies/") &&
      !path.endsWith("/disable") &&
      init?.method === "PUT"
    ) {
      const routePolicyId = decodeURIComponent(path.split("/")[3] ?? "");
      const body = parseRequestBody(init) ?? {};
      const expectedVersion = Number(body.expected_version ?? 0);
      const capabilities = Array.isArray(body.required_capabilities)
        ? body.required_capabilities
        : [];
      const unsupportedCapabilities = capabilities.filter(
        (capability) =>
          ![
            "streaming",
            "tool_calling",
            "tool_related",
            "json_mode",
            "chat_completions",
            "realtime",
            "response_model_metadata",
          ].includes(
            String(capability),
          ),
      );
      const index = routePoliciesState.findIndex(
        (policy) => policy.route_policy_id === routePolicyId,
      );

      if (unsupportedCapabilities.length > 0) {
        return Promise.resolve(
          jsonResponse(400, {
            error: {
              code: "route_policy_compatibility_invalid",
              message: "Route policy compatibility validation failed.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      if (index < 0) {
        return Promise.resolve(
          jsonResponse(404, {
            error: {
              code: "route_policy_not_found",
              message: "Route policy not found.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      if (routePoliciesState[index]?.version !== expectedVersion) {
        return Promise.resolve(
          jsonResponse(409, {
            error: {
              code: "route_policy_version_conflict",
              message: "Route policy version is stale.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      const updated = {
        ...routePoliciesState[index],
        ...body,
        route_policy_id: routePolicyId,
        version: expectedVersion + 1,
      };
      routePoliciesState[index] = updated;
      return Promise.resolve(jsonResponse(200, updated));
    }

    if (
      path.startsWith("/v1/route-policies/") &&
      path.endsWith("/disable") &&
      init?.method === "POST"
    ) {
      const routePolicyId = decodeURIComponent(path.split("/")[3] ?? "");
      const body = parseRequestBody(init) ?? {};
      const expectedVersion = Number(body.expected_version ?? 0);
      const current = routePoliciesState.find(
        (policy) => policy.route_policy_id === routePolicyId,
      );

      if (!current) {
        return Promise.resolve(
          jsonResponse(404, {
            error: {
              code: "route_policy_not_found",
              message: "Route policy not found.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      if (current.version !== expectedVersion) {
        return Promise.resolve(
          jsonResponse(409, {
            error: {
              code: "route_policy_version_conflict",
              message: "Route policy version is stale.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      routePoliciesState = routePoliciesState.filter(
        (policy) => policy.route_policy_id !== routePolicyId,
      );
      return Promise.resolve(
        jsonResponse(200, {
          ...current,
          version: expectedVersion + 1,
        }),
      );
    }

    if (path === "/v1/config-snapshots/active") {
      const activeSnapshot =
        configSnapshotsState.find((snapshot) => snapshot.status === "active") ??
        configSnapshotsState[0];
      return Promise.resolve(
        jsonResponse(200, {
          config_snapshot: activeSnapshot ?? configSnapshotResponse.config_snapshot,
        }),
      );
    }

    if (path === "/v1/config-snapshots" && (!init?.method || init.method === "GET")) {
      return Promise.resolve(jsonResponse(200, { data: configSnapshotsState }));
    }

    if (path === "/v1/config-snapshots" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      configSnapshotsState = [...configSnapshotsState, body as (typeof configSnapshotsResponse)["data"][number]];
      return Promise.resolve(jsonResponse(200, body));
    }

    if (
      path.startsWith("/v1/config-snapshots/") &&
      path.endsWith("/activate") &&
      init?.method === "POST"
    ) {
      const snapshotId = decodeURIComponent(
        path.replace("/v1/config-snapshots/", "").replace("/activate", ""),
      );
      configSnapshotsState = configSnapshotsState.map((snapshot) =>
        snapshot.config_snapshot_id === snapshotId
          ? {
              ...snapshot,
              status: "active",
              activated_at: "2026-04-23T00:00:00Z",
            }
          : snapshot.status === "active"
            ? {
                ...snapshot,
                status: "superseded",
              }
            : snapshot,
      );
      const activatedSnapshot = configSnapshotsState.find(
        (snapshot) => snapshot.config_snapshot_id === snapshotId,
      );

      return Promise.resolve(
        jsonResponse(200, {
          config_snapshot: activatedSnapshot ?? {
            ...configSnapshotsResponse.data[0],
            config_snapshot_id: snapshotId,
          },
        }),
      );
    }

    if (path === "/v1/api-keys" && (!init?.method || init.method === "GET")) {
      return Promise.resolve(jsonResponse(200, { data: apiKeysState }));
    }

    if (path === "/v1/api-keys" && init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      const displayName =
        typeof body.display_name === "string"
          ? body.display_name
          : "New API Key";
      const apiKey =
        typeof body.api_key === "string" ? body.api_key : "akp_new";
      const providerResourceId =
        typeof body.provider_resource_id === "string"
          ? body.provider_resource_id
          : "prvrsrc_unknown";
      const nextKey = {
        api_key_id: `key_${apiKeysState.length + 1}`,
        can_revoke: true,
        created_at: "2026-04-23T00:00:00Z",
        display_name: displayName,
        is_active: true,
        key_prefix: `${apiKey.slice(0, 6)}...`,
        provider_resource_id: providerResourceId,
        tenant_id: "tenant_acme",
        updated_at: "2026-04-23T00:00:00Z",
        version: 1,
      };
      apiKeysState = [...apiKeysState, nextKey];
      return Promise.resolve(jsonResponse(200, nextKey));
    }

    if (path.startsWith("/v1/api-keys/") && path.endsWith("/revoke")) {
      const segments = path.split("/");
      const apiKeyId = decodeURIComponent(segments[3] ?? "");
      const revoke = () => {
        apiKeysState = apiKeysState.map((key) =>
          key.api_key_id === apiKeyId
            ? { ...key, is_active: false, version: key.version + 1 }
            : key,
        );

        return emptyResponse(200);
      };

      if (init?.method === "DELETE" || init?.method === "POST") {
        return Promise.resolve(revoke());
      }

      return Promise.resolve(emptyResponse(405));
    }

    if (path === "/v1/route-simulations" && init?.method === "POST") {
      return Promise.resolve(jsonResponse(200, routeSimulationResponse));
    }

    if (path === "/v1/route-receipts") {
      return Promise.resolve(jsonResponse(200, routeReceiptsResponse));
    }

    if (path.startsWith("/v1/route-diagnostics/")) {
      const routePolicyId = decodeURIComponent(path.split("/")[3] ?? "");
      const diagnostics = routeDiagnosticsByPolicyId[routePolicyId];

      if (diagnostics) {
        return Promise.resolve(jsonResponse(200, diagnostics));
      }

      return Promise.resolve(
        jsonResponse(404, {
          error: {
            code: "not_found",
            message: `No diagnostics for ${routePolicyId}`,
            request_id: "req_test",
            retryable: false,
          },
        }),
      );
    }

    if (path === "/v1/usage/summary") {
      return Promise.resolve(jsonResponse(200, usageSummaryResponse));
    }

    if (path === "/v1/usage/breakdown") {
      return Promise.resolve(jsonResponse(200, usageBreakdownResponse));
    }

    if (path === "/v1/billing/projection") {
      return Promise.resolve(jsonResponse(200, balanceProjectionResponse));
    }

    if (path === "/v1/billing/exports" && (!init?.method || init.method === "GET")) {
      if (billingExportJobsState.some((job) => job.status === "queued")) {
        if (billingExportPollCount > 0) {
          billingExportJobsState = billingExportJobsState.map((job) =>
            job.status === "queued"
              ? {
                  ...job,
                  completed_at: "2026-04-23T00:11:00Z",
                  status: "completed",
                }
              : job,
          );
        } else {
          billingExportPollCount += 1;
        }
      }
      return Promise.resolve(jsonResponse(200, { data: billingExportJobsState }));
    }

    if (path === "/v1/pricing/simulations" && init?.method === "POST") {
      return Promise.resolve(
        jsonResponse(200, {
          catalog_id: "pricing_catalog_default",
          catalog_version: 1,
          currency: "USD",
          provider_cost: {
            currency: "USD",
            amount: "0.005188",
          },
          billable_price: {
            currency: "USD",
            amount: "0.006225",
          },
          line_items: [],
        }),
      );
    }

    if (path === "/v1/billing/exports" && init?.method === "POST") {
      const nextJob = {
        completed_at: undefined,
        export_job_id: `export_${billingExportJobsState.length + 200}`,
        format: "csv",
        project_id: "proj_core",
        requested_at: "2026-04-23T00:10:00Z",
        status: "queued",
        tenant_id: "tenant_acme",
      };
      billingExportPollCount = 0;
      billingExportJobsState = [nextJob, ...billingExportJobsState];
      return Promise.resolve(jsonResponse(202, { data: nextJob }));
    }

    if (
      path.startsWith("/v1/billing/exports/") &&
      path.endsWith("/download") &&
      (!init?.method || init.method === "GET")
    ) {
      const exportJobId = decodeURIComponent(path.split("/")[4] ?? "");
      const job = billingExportJobsState.find(
        (candidate) => candidate.export_job_id === exportJobId,
      );

      if (!job || job.status !== "completed") {
        return Promise.resolve(
          jsonResponse(404, {
            error: {
              code: "billing_export_not_found",
              message: "Billing export not ready.",
              request_id: "req_test",
              retryable: false,
            },
          }),
        );
      }

      return Promise.resolve(
        new Response("date,provider_cost,billable_total\n2026-04-22,1.24,1.54\n", {
          status: 200,
          headers: {
            "content-type": "text/csv",
          },
        }),
      );
    }
    if (
      path.startsWith("/v1/route-receipts/") &&
      path.endsWith("/diagnostics")
    ) {
      const routeReceiptId = decodeURIComponent(
        path.replace("/v1/route-receipts/", "").replace("/diagnostics", ""),
      );
      const diagnostics = routeReceiptDiagnosticsById[routeReceiptId];

      if (diagnostics) {
        return Promise.resolve(jsonResponse(200, diagnostics));
      }

      return Promise.resolve(
        jsonResponse(404, {
          error: {
            code: "not_found",
            message: `No diagnostics for ${routeReceiptId}`,
            request_id: "req_test",
            retryable: false,
          },
        }),
      );
    }

    return Promise.resolve(
      jsonResponse(404, {
        error: {
          code: "not_found",
          message: `Unhandled test path ${path}`,
          request_id: "req_test",
          retryable: false,
        },
      }),
    );
  });
}

export function useControlPlaneFetchMock() {
  let fetchMock: ReturnType<typeof createControlPlaneFetchMock>;

  beforeEach(() => {
    fetchMock = createControlPlaneFetchMock();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  return () => fetchMock;
}
