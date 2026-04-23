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
  configSnapshotsState = structuredClone(configSnapshotsInitialState);
  providerResourcesState = structuredClone(providerResourcesInitialState);
  routePoliciesState = structuredClone(routePoliciesInitialState);

  return vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const path = resolvePath(input);

    if (path === "/v1/tenants") {
      return Promise.resolve(jsonResponse(200, tenantsResponse));
    }

    if (path === "/v1/projects") {
      return Promise.resolve(jsonResponse(200, projectsResponse));
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
          !["streaming", "tool_calling", "json_mode", "chat_completions"].includes(
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
          !["streaming", "tool_calling", "json_mode", "chat_completions"].includes(
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
      const nextKey = {
        api_key_id: `key_${apiKeysState.length + 1}`,
        can_revoke: true,
        created_at: "2026-04-23T00:00:00Z",
        display_name: String(body.display_name ?? "New API Key"),
        is_active: true,
        key_prefix: `${String(body.api_key ?? "akp_new").slice(0, 6)}...`,
        provider_resource_id: String(body.provider_resource_id ?? "prvrsrc_unknown"),
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
