import {
  balanceProjectionResponse,
  configSnapshotResponse,
  routeDiagnosticsByPolicyId,
  routeReceiptDiagnosticsById,
  routeReceiptsResponse,
  routeSimulationResponse,
  tenantsResponse,
  projectsResponse,
  usageBreakdownResponse,
  usageSummaryResponse,
} from "./control-plane-fetch-fixtures";
import type { ControlPlaneMockState } from "./control-plane-fetch-state";
import {
  emptyResponse,
  jsonResponse,
  parseRequestBody,
} from "./control-plane-fetch-route-utils";

const supportedCapabilities = [
  "streaming",
  "tool_calling",
  "tool_related",
  "json_mode",
  "chat_completions",
  "realtime",
  "response_model_metadata",
];

function notFoundResponse(code: string, message: string) {
  return jsonResponse(404, {
    error: {
      code,
      message,
      request_id: "req_test",
      retryable: false,
    },
  });
}

function staleVersionResponse(code: string, message: string) {
  return jsonResponse(409, {
    error: {
      code,
      message,
      request_id: "req_test",
      retryable: false,
    },
  });
}

function compatibilityErrorResponse() {
  return jsonResponse(400, {
    error: {
      code: "route_policy_compatibility_invalid",
      message: "Route policy compatibility validation failed.",
      request_id: "req_test",
      retryable: false,
    },
  });
}

function validationErrorResponse(code: string, message: string) {
  return jsonResponse(400, {
    error: {
      code,
      message,
      request_id: "req_test",
      retryable: false,
    },
  });
}

function isActiveOpeningGrant(grant: { expires_at: string; status: string }) {
  return grant.status === "active" && Date.parse(grant.expires_at) > Date.parse("2026-04-23T00:00:00Z");
}

export function handleReadOnlyRequest(
  state: ControlPlaneMockState,
  path: string,
) {
  if (path === "/v1/tenants") {
    return jsonResponse(200, tenantsResponse);
  }

  if (path === "/v1/projects") {
    return jsonResponse(200, projectsResponse);
  }

  if (path === "/v1/route-simulations") {
    return jsonResponse(200, routeSimulationResponse);
  }

  if (path === "/v1/route-receipts") {
    return jsonResponse(200, routeReceiptsResponse);
  }

  if (path === "/v1/usage/summary") {
    return jsonResponse(200, usageSummaryResponse);
  }

  if (path === "/v1/usage/breakdown") {
    return jsonResponse(200, usageBreakdownResponse);
  }

  if (path === "/v1/billing/projection") {
    return jsonResponse(200, balanceProjectionResponse);
  }

  if (path.startsWith("/v1/route-diagnostics/")) {
    const routePolicyId = decodeURIComponent(path.split("/")[3] ?? "");
    const diagnostics = routeDiagnosticsByPolicyId[routePolicyId];

    return diagnostics
      ? jsonResponse(200, diagnostics)
      : notFoundResponse("not_found", `No diagnostics for ${routePolicyId}`);
  }

  if (path.startsWith("/v1/route-receipts/") && path.endsWith("/diagnostics")) {
    const routeReceiptId = decodeURIComponent(
      path.replace("/v1/route-receipts/", "").replace("/diagnostics", ""),
    );
    const diagnostics = routeReceiptDiagnosticsById[routeReceiptId];

    return diagnostics
      ? jsonResponse(200, diagnostics)
      : notFoundResponse("not_found", `No diagnostics for ${routeReceiptId}`);
  }

  if (path === "/v1/provider-resources") {
    return jsonResponse(200, { data: state.providerResources });
  }

  if (path === "/v1/route-policies") {
    return jsonResponse(200, { data: state.routePolicies });
  }

  if (path === "/v1/config-snapshots/active") {
    const activeSnapshot =
      state.configSnapshots.find((snapshot) => snapshot.status === "active") ??
      state.configSnapshots[0];
    return jsonResponse(200, {
      config_snapshot: activeSnapshot ?? configSnapshotResponse.config_snapshot,
    });
  }

  if (path === "/v1/config-snapshots") {
    return jsonResponse(200, { data: state.configSnapshots });
  }

  if (path === "/v1/api-keys") {
    return jsonResponse(200, { data: state.apiKeys });
  }

  if (path === "/v1/opening-grants") {
    return jsonResponse(200, { data: state.openingGrants });
  }

  if (path === "/v1/oauth-sharing-leases") {
    return jsonResponse(200, { data: state.oauthSharingLeases });
  }

  if (path === "/v1/oauth-carpools") {
    return jsonResponse(200, { data: state.oauthCarpools });
  }

  if (path === "/v1/oauth-sharing-usage") {
    return jsonResponse(200, {
      audit_events: state.oauthSharingAuditEvents,
      data: [],
    });
  }

  if (path === "/v1/billing/exports") {
    if (state.billingExportJobs.some((job) => job.status === "queued")) {
      if (state.billingExportPollCount > 0) {
        state.billingExportJobs = state.billingExportJobs.map((job) =>
          job.status === "queued"
            ? {
                ...job,
                completed_at: "2026-04-23T00:11:00Z",
                status: "completed",
              }
            : job,
        );
      } else {
        state.billingExportPollCount += 1;
      }
    }

    return jsonResponse(200, { data: state.billingExportJobs });
  }

  if (path.startsWith("/v1/billing/exports/") && path.endsWith("/download")) {
    const exportJobId = decodeURIComponent(path.split("/")[4] ?? "");
    const job = state.billingExportJobs.find(
      (candidate) => candidate.export_job_id === exportJobId,
    );

    if (!job || job.status !== "completed") {
      return jsonResponse(404, {
        error: {
          code: "billing_export_not_found",
          message: "Billing export not ready.",
          request_id: "req_test",
          retryable: false,
        },
      });
    }

    return new Response(
      "date,provider_cost,billable_total\n2026-04-22,1.24,1.54\n",
      {
        status: 200,
        headers: {
          "content-type": "text/csv",
        },
      },
    );
  }

  return null;
}

export function handleMutationRequest(
  state: ControlPlaneMockState,
  path: string,
  init?: RequestInit,
) {
  if (path === "/v1/provider-resources" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    state.providerResources = [
      ...state.providerResources,
      body as (typeof state.providerResources)[number],
    ];
    return jsonResponse(200, body);
  }

  if (
    path.startsWith("/v1/provider-resources/") &&
    !path.endsWith("/disable") &&
    init?.method === "PUT"
  ) {
    const providerResourceId = decodeURIComponent(path.split("/")[3] ?? "");
    const body = parseRequestBody(init) ?? {};
    const expectedVersion = Number(body.expected_version ?? 0);
    const index = state.providerResources.findIndex(
      (provider) => provider.provider_resource_id === providerResourceId,
    );

    if (index < 0) {
      return notFoundResponse(
        "provider_resource_not_found",
        "Provider resource not found.",
      );
    }

    if (state.providerResources[index]?.version !== expectedVersion) {
      return staleVersionResponse(
        "provider_resource_version_conflict",
        "Provider resource version is stale.",
      );
    }

    const updated = {
      ...state.providerResources[index],
      ...body,
      provider_resource_id: providerResourceId,
      version: expectedVersion + 1,
    };
    state.providerResources[index] = updated;
    return jsonResponse(200, updated);
  }

  if (
    path.startsWith("/v1/provider-resources/") &&
    path.endsWith("/disable") &&
    init?.method === "POST"
  ) {
    const providerResourceId = decodeURIComponent(path.split("/")[3] ?? "");
    const body = parseRequestBody(init) ?? {};
    const expectedVersion = Number(body.expected_version ?? 0);
    const index = state.providerResources.findIndex(
      (provider) => provider.provider_resource_id === providerResourceId,
    );

    if (index < 0) {
      return notFoundResponse(
        "provider_resource_not_found",
        "Provider resource not found.",
      );
    }

    if (state.providerResources[index]?.version !== expectedVersion) {
      return staleVersionResponse(
        "provider_resource_version_conflict",
        "Provider resource version is stale.",
      );
    }

    const disabled = {
      ...state.providerResources[index],
      status: "disabled",
      health_state: "disabled",
      version: expectedVersion + 1,
    };
    state.providerResources[index] = disabled;
    return jsonResponse(200, disabled);
  }

  if (path === "/v1/route-policies" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const capabilities = Array.isArray(body.required_capabilities)
      ? body.required_capabilities
      : [];
    const unsupportedCapabilities = capabilities.filter(
      (capability) => !supportedCapabilities.includes(String(capability)),
    );

    if (unsupportedCapabilities.length > 0) {
      return compatibilityErrorResponse();
    }

    state.routePolicies = [
      ...state.routePolicies,
      body as (typeof state.routePolicies)[number],
    ];
    return jsonResponse(200, body);
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
      (capability) => !supportedCapabilities.includes(String(capability)),
    );
    const index = state.routePolicies.findIndex(
      (policy) => policy.route_policy_id === routePolicyId,
    );

    if (unsupportedCapabilities.length > 0) {
      return compatibilityErrorResponse();
    }

    if (index < 0) {
      return notFoundResponse(
        "route_policy_not_found",
        "Route policy not found.",
      );
    }

    if (state.routePolicies[index]?.version !== expectedVersion) {
      return staleVersionResponse(
        "route_policy_version_conflict",
        "Route policy version is stale.",
      );
    }

    const updated = {
      ...state.routePolicies[index],
      ...body,
      route_policy_id: routePolicyId,
      version: expectedVersion + 1,
    };
    state.routePolicies[index] = updated;
    return jsonResponse(200, updated);
  }

  if (
    path.startsWith("/v1/route-policies/") &&
    path.endsWith("/disable") &&
    init?.method === "POST"
  ) {
    const routePolicyId = decodeURIComponent(path.split("/")[3] ?? "");
    const body = parseRequestBody(init) ?? {};
    const expectedVersion = Number(body.expected_version ?? 0);
    const current = state.routePolicies.find(
      (policy) => policy.route_policy_id === routePolicyId,
    );

    if (!current) {
      return notFoundResponse(
        "route_policy_not_found",
        "Route policy not found.",
      );
    }

    if (current.version !== expectedVersion) {
      return staleVersionResponse(
        "route_policy_version_conflict",
        "Route policy version is stale.",
      );
    }

    state.routePolicies = state.routePolicies.filter(
      (policy) => policy.route_policy_id !== routePolicyId,
    );
    return jsonResponse(200, {
      ...current,
      version: expectedVersion + 1,
    });
  }

  if (path === "/v1/config-snapshots" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    state.configSnapshots = [
      ...state.configSnapshots,
      body as (typeof state.configSnapshots)[number],
    ];
    return jsonResponse(200, body);
  }

  if (
    path.startsWith("/v1/config-snapshots/") &&
    path.endsWith("/activate") &&
    init?.method === "POST"
  ) {
    const snapshotId = decodeURIComponent(
      path.replace("/v1/config-snapshots/", "").replace("/activate", ""),
    );
    state.configSnapshots = state.configSnapshots.map((snapshot) =>
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
    const activatedSnapshot = state.configSnapshots.find(
      (snapshot) => snapshot.config_snapshot_id === snapshotId,
    );

    return jsonResponse(200, {
      config_snapshot: activatedSnapshot ?? {
        ...configSnapshotResponse.config_snapshot,
        config_snapshot_id: snapshotId,
      },
    });
  }

  if (path === "/v1/api-keys" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const displayName =
      typeof body.display_name === "string" ? body.display_name : "New API Key";
    const apiKey = typeof body.api_key === "string" ? body.api_key : "akp_new";
    const providerResourceId =
      typeof body.provider_resource_id === "string"
        ? body.provider_resource_id
        : "prvrsrc_unknown";
    const nextKey = {
      api_key_id: `key_${state.apiKeys.length + 1}`,
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
    state.apiKeys = [...state.apiKeys, nextKey];
    return jsonResponse(200, nextKey);
  }

  if (path.startsWith("/v1/api-keys/") && path.endsWith("/revoke")) {
    const segments = path.split("/");
    const apiKeyId = decodeURIComponent(segments[3] ?? "");

    if (init?.method === "POST") {
      const body = parseRequestBody(init) ?? {};
      const expectedVersion = Number(body.expected_version ?? 0);
      const index = state.apiKeys.findIndex(
        (key) => key.api_key_id === apiKeyId,
      );

      if (index < 0) {
        return notFoundResponse("api_key_not_found", "API key not found.");
      }

      if (state.apiKeys[index]?.version !== expectedVersion) {
        return staleVersionResponse(
          "api_key_version_conflict",
          "API key version is stale.",
        );
      }

      const revoked = {
        ...state.apiKeys[index],
        is_active: false,
        version: expectedVersion + 1,
      };
      state.apiKeys[index] = revoked;
      return jsonResponse(200, revoked);
    }

    return emptyResponse(405);
  }

  if (path === "/v1/opening-grants" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const configSnapshotId =
      typeof body.config_snapshot_id === "string"
        ? body.config_snapshot_id
        : "";
    const ownerAccountId =
      typeof body.owner_account_id === "string" ? body.owner_account_id : "";
    const granteeKind =
      typeof body.grantee_kind === "string" ? body.grantee_kind : "";
    const granteeId =
      typeof body.grantee_id === "string" ? body.grantee_id : "";
    const granteeLabel =
      typeof body.grantee_label === "string" && body.grantee_label.length > 0
        ? body.grantee_label
        : undefined;
    const expiresAt =
      typeof body.expires_at === "string" ? body.expires_at : "";
    const scopes = Array.isArray(body.scopes)
      ? body.scopes.map(String).filter(Boolean)
      : [];
    const snapshot = state.configSnapshots.find(
      (candidate) => candidate.config_snapshot_id === configSnapshotId,
    );

    if (!snapshot) {
      return notFoundResponse(
        "config_snapshot_not_found",
        "Config snapshot not found.",
      );
    }

    if (!/^[A-Za-z0-9_.-]{1,128}$/.test(ownerAccountId)) {
      return validationErrorResponse(
        "opening_owner_account_id_invalid",
        "Owner account id is invalid.",
      );
    }

    if (!["user", "workspace", "agent"].includes(granteeKind)) {
      return validationErrorResponse(
        "opening_grantee_kind_invalid",
        "Grantee kind is invalid.",
      );
    }

    if (!granteeId) {
      return validationErrorResponse(
        "opening_grantee_required",
        "Grantee id is required.",
      );
    }

    if (!expiresAt || Number.isNaN(Date.parse(expiresAt))) {
      return validationErrorResponse(
        "opening_expires_at_invalid",
        "Expiration timestamp is invalid.",
      );
    }

    if (scopes.length === 0) {
      return validationErrorResponse(
        "opening_scope_invalid",
        "At least one scope is required.",
      );
    }

    const activeOwnerGrants = state.openingGrants.filter(
      (grant) =>
        grant.tenant_id === snapshot.tenant_id &&
        grant.project_id === snapshot.project_id &&
        grant.owner_account_id === ownerAccountId &&
        isActiveOpeningGrant(grant),
    );

    if (activeOwnerGrants.length >= 8) {
      return staleVersionResponse(
        "opening_grant_limit_reached",
        "Owner account already has 8 active child keys.",
      );
    }

    if (
      activeOwnerGrants.some(
        (grant) =>
          grant.grantee_kind === granteeKind && grant.grantee_id === granteeId,
      )
    ) {
      return staleVersionResponse(
        "opening_grant_grantee_active",
        "Grantee already has an active child key for this owner account.",
      );
    }

    const sequence = state.openingGrants.length + 1;
    const nextGrant = {
      grant_id: `opengrant_test_${sequence}`,
      tenant_id: snapshot.tenant_id,
      project_id: snapshot.project_id,
      owner_account_id: ownerAccountId,
      grantee_kind: granteeKind,
      grantee_id: granteeId,
      grantee_label: granteeLabel,
      config_snapshot_id: snapshot.config_snapshot_id,
      route_policy_id: snapshot.route_policy_id,
      budget_policy_id: snapshot.budget_policy_id,
      provider_resource_ids: snapshot.provider_resource_ids,
      credential_kind: "api_key",
      credential_id: `cred_opening_test_${sequence}`,
      credential_key_prefix: `akp-child-${sequence}`,
      credential_last_four: String(sequence).padStart(4, "0"),
      scopes,
      expires_at: expiresAt,
      status: "active",
      created_by: "user_test",
      created_at: "2026-04-23T00:00:00Z",
      updated_at: "2026-04-23T00:00:00Z",
      version: 1,
    };

    state.openingGrants = [...state.openingGrants, nextGrant];

    return jsonResponse(200, {
      grant: nextGrant,
      credential: {
        credential_kind: "api_key",
        credential_id: nextGrant.credential_id,
        key_prefix: nextGrant.credential_key_prefix,
        last_four: nextGrant.credential_last_four,
        plaintext: "akp_test_child_once",
      },
    });
  }

  if (path.startsWith("/v1/opening-grants/") && path.endsWith("/revoke")) {
    if (init?.method !== "POST") {
      return emptyResponse(405);
    }

    const grantId = decodeURIComponent(path.split("/")[3] ?? "");
    const body = parseRequestBody(init) ?? {};
    const expectedVersion = Number(body.expected_version ?? 0);
    const index = state.openingGrants.findIndex(
      (grant) => grant.grant_id === grantId,
    );

    if (index < 0) {
      return notFoundResponse(
        "opening_grant_not_found",
        "Opening grant not found.",
      );
    }

    if (state.openingGrants[index]?.version !== expectedVersion) {
      return staleVersionResponse(
        "opening_grant_version_conflict",
        "Opening grant version is stale.",
      );
    }

    const revoked = {
      ...state.openingGrants[index],
      revoked_at: "2026-04-23T00:00:00Z",
      revoked_by: "user_test",
      status: "revoked",
      updated_at: "2026-04-23T00:00:00Z",
      version: expectedVersion + 1,
    };
    state.openingGrants[index] = revoked;
    return jsonResponse(200, revoked);
  }

  if (path === "/v1/oauth-sharing-leases" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const nextLease = {
      ...body,
      created_at: "2026-04-23T00:00:00Z",
      updated_at: "2026-04-23T00:00:00Z",
    } as (typeof state.oauthSharingLeases)[number];
    state.oauthSharingLeases = [
      ...state.oauthSharingLeases.filter(
        (lease) => lease.lease_id !== nextLease.lease_id,
      ),
      nextLease,
    ];
    return jsonResponse(200, nextLease);
  }

  if (
    path.startsWith("/v1/oauth-sharing-leases/") &&
    path.endsWith("/revoke") &&
    init?.method === "POST"
  ) {
    const leaseId = decodeURIComponent(path.split("/")[3] ?? "");
    const current = state.oauthSharingLeases.find(
      (lease) => lease.lease_id === leaseId,
    );
    if (!current) {
      return notFoundResponse(
        "oauth_sharing_lease_not_found",
        "OAuth sharing lease not found.",
      );
    }
    const revoked = {
      ...current,
      status: "revoked",
      updated_at: "2026-04-23T00:00:00Z",
    };
    state.oauthSharingLeases = state.oauthSharingLeases.map((lease) =>
      lease.lease_id === leaseId ? revoked : lease,
    );
    return jsonResponse(200, revoked);
  }

  if (path === "/v1/oauth-carpools" && init?.method === "POST") {
    const body = parseRequestBody(init) ?? {};
    const nextCarpool = {
      ...body,
      created_at: "2026-04-23T00:00:00Z",
      updated_at: "2026-04-23T00:00:00Z",
    } as (typeof state.oauthCarpools)[number];
    state.oauthCarpools = [
      ...state.oauthCarpools.filter(
        (carpool) => carpool.carpool_id !== nextCarpool.carpool_id,
      ),
      nextCarpool,
    ];
    return jsonResponse(200, nextCarpool);
  }

  if (path.startsWith("/v1/oauth-carpools/") && init?.method === "DELETE") {
    const carpoolId = decodeURIComponent(path.split("/")[3] ?? "");
    const current = state.oauthCarpools.find(
      (carpool) => carpool.carpool_id === carpoolId,
    );
    if (!current) {
      return notFoundResponse(
        "oauth_carpool_not_found",
        "OAuth carpool not found.",
      );
    }
    const disabled = {
      ...current,
      enabled: false,
      updated_at: "2026-04-23T00:00:00Z",
    };
    state.oauthCarpools = state.oauthCarpools.filter(
      (carpool) => carpool.carpool_id !== carpoolId,
    );
    return jsonResponse(200, disabled);
  }

  if (path === "/v1/pricing/simulations" && init?.method === "POST") {
    return jsonResponse(200, {
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
    });
  }

  if (path === "/v1/billing/exports" && init?.method === "POST") {
    const nextJob = {
      completed_at: undefined,
      export_job_id: `export_${state.billingExportJobs.length + 200}`,
      format: "csv",
      project_id: "proj_core",
      requested_at: "2026-04-23T00:10:00Z",
      status: "queued",
      tenant_id: "tenant_acme",
    };
    state.billingExportPollCount = 0;
    state.billingExportJobs = [nextJob, ...state.billingExportJobs];
    return jsonResponse(202, { data: nextJob });
  }

  return null;
}
