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
      health_message: "Healthy across the last 15 minutes of probe traffic.",
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
      health_state: "quarantined",
      health_message:
        "Quarantined after repeated upstream 5xx bursts on the backup region.",
      quarantine_reason:
        "automatic quarantine after elevated upstream_error_rate",
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
      provider_resource_id: "prvrsrc_transit_relay",
      tenant_id: "tenant_acme",
      project_id: "proj_acme_support",
      provider_id: "transit",
      name: "Realtime Transit Relay",
      status: "active",
      provenance_class: "official_gateway",
      credential_owner_type: "platform",
      deployment_scope: "shared",
      region: "us-central-1",
      endpoint_base_url: "https://transit.hugerouter.dev/v1",
      auth_kind: "session_broker",
      health_state: "draining",
      health_message:
        "Realtime ingress is draining while a new transit build rolls out.",
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: true,
        supports_json_mode: false,
        supports_realtime: true,
        supports_response_model_metadata: true,
      },
      supported_protocol_families: [
        "openai_responses",
        "mcp_streamable_http",
        "realtime_webrtc",
      ],
      is_transit_gateway: true,
      version: 3,
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
      health_message:
        "Latency is elevated, but the target remains available for research traffic.",
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

const routePoliciesResponse = {
  data: [
    {
      route_policy_id: "routepol_openai_chat_default",
      tenant_id: "tenant_acme",
      display_name: "Acme Reasoning Fast",
      protocol_family: "openai_chat",
      model_alias: "reasoning-fast",
      required_capabilities: ["json_mode", "tool_calling"],
      preferred_regions: ["us-east-1"],
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      route_policy_id: "routepol_acme_support",
      tenant_id: "tenant_acme",
      display_name: "Acme Responses Safe",
      protocol_family: "openai_responses",
      model_alias: "support-safe",
      required_capabilities: ["streaming", "response_model_metadata"],
      preferred_regions: ["us-west-2"],
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z",
    },
    {
      route_policy_id: "routepol_acme_realtime",
      tenant_id: "tenant_acme",
      display_name: "Acme Realtime Agent",
      protocol_family: "realtime_webrtc",
      model_alias: "agent-live",
      required_capabilities: ["streaming", "realtime", "tool_calling"],
      preferred_regions: ["us-central-1"],
      version: 2,
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

const configSnapshotResponse = {
  config_snapshot: {
    config_snapshot_id: "cfgsnap_gateway_v1",
    tenant_id: "tenant_acme",
    project_id: "proj_core",
    revision: 1,
    status: "active",
    activated_at: "2026-04-22T00:00:00Z",
    provider_resource_ids: [
      "prvrsrc_openai_primary",
      "prvrsrc_openai_backup",
      "prvrsrc_transit_relay",
    ],
    route_policy_id: "routepol_openai_chat_default",
    budget_policy_id: "budgetpol_default",
  },
};

const routeReceiptsResponse = {
  data: [
    {
      route_receipt_id: "routercpt_acme_realtime",
      tenant_id: "tenant_acme",
      project_id: "proj_acme_support",
      route_policy_id: "routepol_acme_realtime",
      request_id: "req_acme_realtime",
      trace_id: "trace_acme_realtime",
      protocol_family: "realtime_webrtc",
      model_alias: "agent-live",
      config_snapshot_id: "cfgsnap_gateway_v1",
      admission_result: "rejected_no_candidate",
      excluded_targets: [
        {
          provider_resource_id: "prvrsrc_openai_primary",
          reason_code: "capability_gap_realtime",
          reason:
            "Primary OpenAI target does not advertise realtime capability.",
        },
        {
          provider_resource_id: "prvrsrc_openai_backup",
          reason_code: "health_quarantined",
          reason:
            "Backup target is quarantined and cannot receive realtime traffic.",
        },
        {
          provider_resource_id: "prvrsrc_transit_relay",
          reason_code: "health_draining",
          reason:
            "Transit relay is draining and temporarily excluded from live session routing.",
        },
      ],
      score_breakdown: {
        latency: 0,
        cost: 0,
        health: 0,
        trust: 0,
      },
      fallback_transitions: [],
      normalized_error: {
        code: "no_eligible_target",
        message: "HugeRouter could not find a healthy realtime-capable target.",
        request_id: "req_acme_realtime",
        retryable: true,
        validation_issues: [],
        details: {
          route_policy_id: "routepol_acme_realtime",
        },
      },
      failure_reason:
        "No healthy target satisfied realtime_webrtc plus required tool-related capabilities.",
      created_at: "2026-04-22T00:14:00Z",
    },
    {
      route_receipt_id: "routercpt_acme_support",
      tenant_id: "tenant_acme",
      project_id: "proj_acme_support",
      route_policy_id: "routepol_acme_support",
      request_id: "req_acme_support",
      trace_id: "trace_acme_support",
      protocol_family: "openai_responses",
      model_alias: "support-safe",
      config_snapshot_id: "cfgsnap_gateway_v1",
      admission_result: "admitted",
      selected_target: "prvrsrc_openai_primary",
      excluded_targets: [
        {
          provider_resource_id: "prvrsrc_openai_backup",
          reason_code: "health_quarantined",
          reason: "Excluded because the provider is currently quarantined.",
        },
        {
          provider_resource_id: "prvrsrc_transit_relay",
          reason_code: "capability_gap_json_mode",
          reason:
            "Transit relay excluded because it cannot emit the required JSON response mode.",
        },
      ],
      score_breakdown: {
        latency: 0.83,
        cost: 0.7,
        health: 1,
        trust: 0.97,
      },
      fallback_transitions: [],
      created_at: "2026-04-22T00:12:00Z",
    },
    {
      route_receipt_id: "routercpt_acme_default",
      tenant_id: "tenant_acme",
      project_id: "proj_core",
      route_policy_id: "routepol_openai_chat_default",
      request_id: "req_acme_default",
      trace_id: "trace_acme_default",
      protocol_family: "openai_chat",
      model_alias: "reasoning-fast",
      config_snapshot_id: "cfgsnap_gateway_v1",
      admission_result: "admitted",
      selected_target: "prvrsrc_openai_primary",
      excluded_targets: [
        {
          provider_resource_id: "prvrsrc_openai_backup",
          reason_code: "health_quarantined",
          reason: "Excluded because the provider is currently quarantined.",
        },
        {
          provider_resource_id: "prvrsrc_transit_relay",
          reason_code: "protocol_family_unsupported",
          reason:
            "Excluded because the provider does not advertise openai_chat.",
        },
      ],
      score_breakdown: {
        latency: 0.98,
        cost: 0.74,
        health: 1,
        trust: 0.98,
      },
      fallback_transitions: [],
      created_at: "2026-04-22T00:10:00Z",
    },
  ],
};

const routeDiagnosticsResponse = {
  route_policy: routePoliciesResponse.data[2],
  active_snapshot: configSnapshotResponse.config_snapshot,
  active_snapshot_matches_route_policy: false,
  last_route_receipt: {
    route_receipt_id: "routercpt_acme_realtime",
    admission_result: "rejected_no_candidate",
    failure_reason:
      "No healthy target satisfied realtime_webrtc plus required tool-related capabilities.",
    created_at: "2026-04-22T00:14:00Z",
  },
  recent_receipts: [
    {
      route_receipt_id: "routercpt_acme_realtime",
      admission_result: "rejected_no_candidate",
      failure_reason:
        "No healthy target satisfied realtime_webrtc plus required tool-related capabilities.",
      created_at: "2026-04-22T00:14:00Z",
    },
  ],
  targets: providerResourcesResponse.data
    .filter((provider) => provider.tenant_id === "tenant_acme")
    .map((provider) => ({
      provider_resource: provider,
      decision:
        provider.provider_resource_id === "prvrsrc_transit_relay"
          ? "excluded"
          : provider.provider_resource_id === "prvrsrc_openai_primary"
            ? "excluded"
            : "excluded",
      in_active_snapshot:
        configSnapshotResponse.config_snapshot.provider_resource_ids.includes(
          provider.provider_resource_id,
        ),
      supports_protocol_family:
        provider.supported_protocol_families.includes("realtime_webrtc"),
      capability_gaps:
        provider.provider_resource_id === "prvrsrc_openai_primary"
          ? ["realtime"]
          : provider.provider_resource_id === "prvrsrc_openai_backup"
            ? ["realtime"]
            : [],
      reason_code:
        provider.provider_resource_id === "prvrsrc_transit_relay"
          ? "health_draining"
          : provider.provider_resource_id === "prvrsrc_openai_backup"
            ? "health_quarantined"
            : "capability_gap_realtime",
      reason:
        provider.provider_resource_id === "prvrsrc_transit_relay"
          ? "Transit relay is draining and temporarily excluded from live session routing."
          : provider.provider_resource_id === "prvrsrc_openai_backup"
            ? "Backup target is quarantined and cannot receive realtime traffic."
            : "Primary OpenAI target does not advertise realtime capability.",
      recent_receipt_id: "routercpt_acme_realtime",
    })),
};

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
    return input;
  }

  if (input instanceof URL) {
    return input.pathname;
  }

  return new URL(input.url, "http://127.0.0.1").pathname;
}

export function createControlPlaneFetchMock() {
  return vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const path = resolvePath(input);

    if (path === "/v1/tenants") {
      return Promise.resolve(jsonResponse(200, tenantsResponse));
    }

    if (path === "/v1/projects") {
      return Promise.resolve(jsonResponse(200, projectsResponse));
    }

    if (path === "/v1/provider-resources") {
      return Promise.resolve(jsonResponse(200, providerResourcesResponse));
    }

    if (path.startsWith("/v1/provider-resources?")) {
      return Promise.resolve(jsonResponse(200, providerResourcesResponse));
    }

    if (path === "/v1/route-policies") {
      return Promise.resolve(jsonResponse(200, routePoliciesResponse));
    }

    if (path === "/v1/route-receipts") {
      return Promise.resolve(jsonResponse(200, routeReceiptsResponse));
    }

    if (path.startsWith("/v1/route-receipts?")) {
      return Promise.resolve(jsonResponse(200, routeReceiptsResponse));
    }

    if (path === "/v1/route-diagnostics/routepol_acme_realtime") {
      return Promise.resolve(jsonResponse(200, routeDiagnosticsResponse));
    }

    if (path === "/v1/config-snapshots/active") {
      return Promise.resolve(jsonResponse(200, configSnapshotResponse));
    }

    if (path === "/v1/route-simulations" && init?.method === "POST") {
      return Promise.resolve(jsonResponse(200, routeSimulationResponse));
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
