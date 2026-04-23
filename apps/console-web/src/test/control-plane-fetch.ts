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
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: true,
        supports_json_mode: true,
      },
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
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: true,
        supports_json_mode: true,
      },
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
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: false,
        supports_json_mode: true,
      },
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

const routeReceiptsResponse = {
  data: [
    {
      route_receipt: {
        route_receipt_id: "routercpt_openai_primary_recent",
        tenant_id: "tenant_acme",
        project_id: "proj_core",
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
        created_at: "2026-04-22T13:00:00Z",
      },
    },
    {
      route_receipt: {
        route_receipt_id: "routercpt_openai_rejection",
        tenant_id: "tenant_northstar",
        project_id: "proj_ns_research",
        request_id: "req_openai_rejection",
        trace_id: "trace_openai_rejection",
        protocol_family: "openai_chat",
        model_alias: "research-fast",
        config_snapshot_id: "cfgsnap_gateway_v1",
        admission_result: "rejected_policy",
        excluded_targets: [
          {
            provider_resource_id: "prvrsrc_openai_research",
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
    status: "queued",
    format: "csv",
    requested_at: "2026-04-22T13:00:10Z",
    completed_at: "2026-04-22T13:00:20Z",
    tenant_id: "tenant_acme",
    project_id: "proj_core",
  },
};

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

const configSnapshotById: Record<
  string,
  (typeof configSnapshotsResponse)["data"][number]
> = {
  cfgsnap_gateway_v1: configSnapshotsResponse.data[1],
  cfgsnap_gateway_v2: configSnapshotsResponse.data[0],
};

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

export function createControlPlaneFetchMock() {
  apiKeysState = [...apiKeysInitialState];

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

    if (path === "/v1/route-policies") {
      return Promise.resolve(jsonResponse(200, routePoliciesResponse));
    }

    if (path === "/v1/config-snapshots/active") {
      return Promise.resolve(jsonResponse(200, configSnapshotResponse));
    }

    if (path === "/v1/config-snapshots") {
      return Promise.resolve(jsonResponse(200, configSnapshotsResponse));
    }

    if (
      path.startsWith("/v1/config-snapshots/") &&
      path.endsWith("/activate") &&
      init?.method === "POST"
    ) {
      const snapshotId = decodeURIComponent(
        path.replace("/v1/config-snapshots/", "").replace("/activate", ""),
      );

      return Promise.resolve(
        jsonResponse(200, {
          config_snapshot: configSnapshotById[snapshotId] ?? {
            ...configSnapshotsResponse.data[0],
            config_snapshot_id: snapshotId,
          },
        }),
      );
    }

    if (path === "/v1/api-keys") {
      return Promise.resolve(jsonResponse(200, { data: apiKeysState }));
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

    if (path === "/v1/usage/summary") {
      return Promise.resolve(jsonResponse(200, usageSummaryResponse));
    }

    if (path === "/v1/usage/breakdown") {
      return Promise.resolve(jsonResponse(200, usageBreakdownResponse));
    }

    if (path === "/v1/billing/projection") {
      return Promise.resolve(jsonResponse(200, balanceProjectionResponse));
    }

    if (
      path === "/v1/billing/exports" &&
      (!init?.method || init.method === "GET")
    ) {
      return Promise.resolve(
        jsonResponse(200, { data: [billingExportResponse.data] }),
      );
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
      return Promise.resolve(jsonResponse(202, billingExportResponse));
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
