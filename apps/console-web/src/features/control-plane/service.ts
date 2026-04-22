import {
  createControlPlaneClient,
  ControlPlaneClientError,
  type ControlPlaneClient,
} from "@huge-router/ts-api-client";
import {
  type BillingExportJob,
  configSnapshotSchema,
  type ConfigSnapshot,
  type Project,
  type ProviderResource,
  type RoutePolicy,
  type RouteReceiptDiagnosticsResponse,
  type RouteReceipt,
  routeReceiptSchema,
  type RouteSimulationResponse,
} from "@huge-router/ts-shared-schema";
import type { AuthSessionEnvelope } from "../auth/auth-contract";
import { authSessionQueryKey } from "../auth/auth-queries";
import { getQueryClient } from "../../lib/query-client";
import type {
  ApiKeyView,
  BillingExportJobView,
  BillingDashboardData,
  ConfigSnapshotView,
  OverviewData,
  ProjectSummary,
  RouteReceiptDiagnosticView,
  RoutePolicyView,
  TenantDetail,
  TenantSummary,
  UsageBreakdownView,
  UsageDashboardData,
} from "./types";

export type ConsoleDataService = {
  getOverview: () => Promise<OverviewData>;
  getUsageDashboard: (
    range: "7d" | "30d" | "90d",
    groupBy: "provider" | "model" | "day",
    projectId?: string,
    cursor?: string,
  ) => Promise<UsageDashboardData>;
  getBillingDashboard: (
    range: "7d" | "30d" | "90d",
    projectId?: string,
  ) => Promise<BillingDashboardData>;
  queueBillingExport: (
    range: "7d" | "30d" | "90d",
    projectId?: string,
  ) => Promise<BillingExportJobView>;
  getTenantDetail: (tenantId: string) => Promise<TenantDetail>;
  listProviderResources: () => Promise<ProviderResource[]>;
  listRoutePolicies: () => Promise<RoutePolicyView[]>;
  listRouteReceipts: () => Promise<RouteReceiptDiagnosticView[]>;
  listTenants: () => Promise<TenantSummary[]>;
  listConfigSnapshots: () => Promise<ConfigSnapshotView[]>;
  activateConfigSnapshot: (
    configSnapshotId: string,
  ) => Promise<ConfigSnapshotView>;
  listApiKeys: () => Promise<ApiKeyView[]>;
  revokeApiKey: (apiKeyId: string, version: number) => Promise<void>;
};

const CONTROL_PLANE_BASE_URL = import.meta.env.VITE_CONTROL_PLANE_BASE_URL
  ? String(import.meta.env.VITE_CONTROL_PLANE_BASE_URL)
  : "";

const client = createControlPlaneClient({
  baseUrl: CONTROL_PLANE_BASE_URL,
  fetch: (input, init) => globalThis.fetch(input, init),
});

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function stringOrUndefined(value: unknown): string | undefined {
  return typeof value === "string" && value.trim().length > 0
    ? value
    : undefined;
}

function numberOrUndefined(value: unknown): number | undefined {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }

  if (typeof value === "string" && value.trim().length > 0) {
    const parsed = Number(value);

    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }

  return undefined;
}

function booleanOrUndefined(value: unknown): boolean | undefined {
  if (typeof value === "boolean") {
    return value;
  }

  return undefined;
}

function toControlPlaneUrl(path: string) {
  if (!CONTROL_PLANE_BASE_URL) {
    return path;
  }

  const baseUrl = CONTROL_PLANE_BASE_URL.endsWith("/")
    ? CONTROL_PLANE_BASE_URL
    : `${CONTROL_PLANE_BASE_URL}/`;

  return new URL(path, baseUrl).toString();
}

function isNotFoundErrorStatus(status: number) {
  return status === 404 || status === 405 || status === 501;
}

function hasArray(value: unknown): value is unknown[] {
  return Array.isArray(value);
}

type ParsedControlPlaneError = {
  code?: string;
  message: string;
  requestId?: string;
  traceId?: string;
};

function parseErrorPayload(
  payload: unknown,
): ParsedControlPlaneError | undefined {
  if (!isRecord(payload)) {
    return undefined;
  }

  const nested = payload.error;
  const payloadRequestId = stringOrUndefined(
    pickRecordValue(payload, ["requestId", "request_id"]),
  );
  const payloadTraceId = stringOrUndefined(
    pickRecordValue(payload, ["traceId", "trace_id"]),
  );
  const payloadCode = stringOrUndefined(pickRecordValue(payload, ["code"]));
  const payloadMessage = stringOrUndefined(
    pickRecordValue(payload, ["message"]),
  );

  if (isRecord(nested)) {
    return {
      code: stringOrUndefined(pickRecordValue(nested, ["code"])) ?? payloadCode,
      message:
        stringOrUndefined(pickRecordValue(nested, ["message"])) ??
        payloadMessage ??
        "Request failed.",
      requestId:
        stringOrUndefined(
          pickRecordValue(nested, ["requestId", "request_id"]),
        ) ?? payloadRequestId,
      traceId:
        stringOrUndefined(pickRecordValue(nested, ["traceId", "trace_id"])) ??
        payloadTraceId,
    };
  }

  return {
    code: payloadCode,
    message: payloadMessage ?? "Request failed.",
    requestId: payloadRequestId,
    traceId: payloadTraceId,
  };
}

async function requestControlPlaneJson<T>(
  path: string,
  parse: (payload: unknown) => T,
  init: RequestInit = {},
): Promise<T> {
  const response = await globalThis.fetch(toControlPlaneUrl(path), {
    credentials: "include",
    ...init,
  });
  const responseText = await response.text();
  let payload: unknown = null;

  if (responseText) {
    try {
      payload = JSON.parse(responseText);
    } catch {
      payload = responseText;
    }
  }

  if (!response.ok) {
    const normalized = parseErrorPayload(payload);

    throw new ControlPlaneClientError(
      normalized?.message ??
        `Control plane request failed with status ${response.status}`,
      response.status,
      {
        code: normalized?.code,
        meta: {
          requestId: normalized?.requestId,
          traceId: normalized?.traceId,
        },
      },
    );
  }

  return parse(payload);
}

function pickRecordValue<T>(
  record: Record<string, unknown>,
  keys: string[],
): T | undefined {
  for (const key of keys) {
    const value = record[key];

    if (value !== undefined) {
      return value as T;
    }
  }

  return undefined;
}

function extractListPayload(
  payload: unknown,
  keys: string[],
  fallbackToSingle = false,
) {
  if (hasArray(payload)) {
    return payload;
  }

  if (!isRecord(payload)) {
    return [];
  }

  for (const key of keys) {
    const value = payload[key];

    if (hasArray(value)) {
      return value;
    }
  }

  if (
    fallbackToSingle &&
    (payload.config_snapshot || payload.config_snapshot_id)
  ) {
    return [payload];
  }

  return [];
}

function isControlPlaneError(value: unknown): value is ControlPlaneClientError {
  return value instanceof ControlPlaneClientError;
}

function isRecoverableMissingEndpoint(error: unknown) {
  return isControlPlaneError(error)
    ? isNotFoundErrorStatus(error.status)
    : false;
}

function getAuthEnvelope() {
  return getQueryClient().getQueryData<AuthSessionEnvelope>(
    authSessionQueryKey,
  );
}

function getActiveTenantId() {
  const envelope = getAuthEnvelope();

  if (envelope?.state.kind !== "authenticated") {
    return null;
  }

  return envelope.state.session.activeTenant?.tenantId ?? null;
}

function isPlatformAdmin() {
  const envelope = getAuthEnvelope();

  return envelope?.state.kind === "authenticated"
    ? envelope.state.session.user.isPlatformAdmin
    : false;
}

function tenantIdFromRecord(item: unknown) {
  if (!isRecord(item)) {
    return undefined;
  }

  return (
    stringOrUndefined(pickRecordValue<string>(item, ["tenant_id"])) ??
    stringOrUndefined(pickRecordValue<string>(item, ["tenantId"]))
  );
}

function filterByTenant<T>(items: T[]) {
  const tenantId = getActiveTenantId();

  if (!tenantId || isPlatformAdmin()) {
    return items;
  }

  return items.filter((item) => tenantIdFromRecord(item) === tenantId);
}

function toProjectSummary(project: Project): ProjectSummary {
  return {
    id: project.project_id,
    name: project.display_name,
    slug: project.slug,
  };
}

function formatUsdAmount(amount: { amount: string }) {
  return amount.amount;
}

function rangeToWindow(range: "7d" | "30d" | "90d") {
  const now = new Date();
  const end = now.toISOString();
  const start = new Date(now);
  const days = range === "7d" ? 7 : range === "90d" ? 90 : 30;
  start.setUTCDate(start.getUTCDate() - days);

  return {
    label:
      range === "7d"
        ? "Last 7 days"
        : range === "90d"
          ? "Last 90 days"
          : "Last 30 days",
    windowEnd: end,
    windowStart: start.toISOString(),
  };
}

function toUsageBreakdownView(
  row: Awaited<
    ReturnType<ControlPlaneClient["getUsageBreakdown"]>
  >["data"][number],
): UsageBreakdownView {
  return {
    billablePriceUsd: formatUsdAmount(row.billable_price),
    bucket: row.bucket,
    cachedInputTokens: row.cached_input_tokens,
    inputTokens: row.input_tokens,
    modelAlias: row.model_alias,
    outputTokens: row.output_tokens,
    providerCostUsd: formatUsdAmount(row.provider_cost),
    providerId: row.provider_id,
  };
}

function toBillingExportJobView(job: BillingExportJob): BillingExportJobView {
  return {
    completedAt: job.completed_at,
    errorMessage: job.error_message,
    exportJobId: job.export_job_id,
    format: job.format,
    projectId: job.project_id,
    requestedAt: job.requested_at,
    status: job.status,
    tenantId: job.tenant_id,
  };
}

function mapRoutePolicies(
  routePolicies: RoutePolicy[],
  providerResources: ProviderResource[],
  activeSnapshot: ConfigSnapshot | null,
): RoutePolicyView[] {
  const providerNames = new Map(
    providerResources.map((provider) => [
      provider.provider_resource_id,
      provider.name,
    ]),
  );

  return routePolicies.map((policy) => ({
    id: policy.route_policy_id,
    modelAlias: policy.model_alias,
    name: policy.display_name,
    preferredRegions: policy.preferred_regions,
    protocolFamily: policy.protocol_family,
    requiredCapabilities: policy.required_capabilities,
    selectedProviders:
      activeSnapshot?.route_policy_id === policy.route_policy_id
        ? activeSnapshot.provider_resource_ids.map(
            (providerId) => providerNames.get(providerId) ?? providerId,
          )
        : [],
  }));
}

function providerLabelById(providerResources: ProviderResource[]) {
  return new Map(
    providerResources.map((provider) => [
      provider.provider_resource_id,
      provider.name,
    ]),
  );
}

function mapProviderLabel(
  providerResourceId: string,
  providersById: Map<string, string>,
) {
  return providersById.get(providerResourceId) ?? providerResourceId;
}

function parseSingleRouteReceipt(payload: unknown): RouteReceipt {
  if (!isRecord(payload)) {
    throw new Error("Malformed route receipt response payload");
  }

  const rawReceipt = isRecord(payload.route_receipt)
    ? payload.route_receipt
    : payload;

  return routeReceiptSchema.parse(rawReceipt);
}

function parseRouteReceiptList(payload: unknown) {
  const receipts = extractListPayload(payload, [
    "data",
    "route_receipts",
    "receipts",
    "items",
  ]);

  return receipts.map(parseSingleRouteReceipt);
}

function toRouteReceiptDiagnostic(
  receipt: RouteReceipt,
  providerById: Map<string, string>,
  diagnostics?: RouteReceiptDiagnosticsResponse | null,
): RouteReceiptDiagnosticView {
  const selectedTargetLabel = receipt.selected_target
    ? mapProviderLabel(receipt.selected_target, providerById)
    : "No selected target";

  const excludedTargets = receipt.excluded_targets.map((target) => ({
    providerLabel: mapProviderLabel(target.provider_resource_id, providerById),
    providerResourceId: target.provider_resource_id,
    reason: target.reason,
  }));

  const fallbackTransitions = receipt.fallback_transitions.map(
    (transition) => ({
      fromProviderLabel: mapProviderLabel(
        transition.from_provider_resource_id,
        providerById,
      ),
      fromProviderResourceId: transition.from_provider_resource_id,
      reason: transition.reason,
      toProviderLabel: mapProviderLabel(
        transition.to_provider_resource_id,
        providerById,
      ),
      toProviderResourceId: transition.to_provider_resource_id,
    }),
  );

  return {
    admissionResult: receipt.admission_result,
    configSnapshotId: receipt.config_snapshot_id,
    createdAt: receipt.created_at,
    decisionTimeline:
      diagnostics?.decision_timeline.map((item) => ({
        message: item.message,
        notes: item.notes,
        score: item.score,
        stage: item.stage,
        status: item.status,
      })) ?? [],
    excludedTargets,
    fallbackTransitions,
    metadata: diagnostics?.metadata ?? {},
    modelAlias: receipt.model_alias,
    normalizedError: receipt.normalized_error,
    policyChecks:
      diagnostics?.policy_checks.map((item) => ({
        policyId: item.policy_id,
        reason: item.reason,
        status: item.status,
      })) ?? [],
    providerAttempts:
      diagnostics?.provider_attempts.map((item) => ({
        attempt: item.attempt,
        finishedAt: item.finished_at,
        latencyMs: item.latency_ms,
        providerLabel: mapProviderLabel(
          item.provider_resource_id,
          providerById,
        ),
        providerResourceId: item.provider_resource_id,
        reason: item.reason,
        startedAt: item.started_at,
        status: item.status,
      })) ?? [],
    protocolFamily: receipt.protocol_family,
    routeReceiptId: receipt.route_receipt_id,
    requestId: receipt.request_id,
    selectedTargetLabel,
    selectedTargetReason: receipt.selected_target ? null : "No target selected",
    selectedTargetResourceId: receipt.selected_target ?? "none",
    traceId: receipt.trace_id,
  };
}

function toConfigSnapshotView(snapshot: ConfigSnapshot): ConfigSnapshotView {
  return {
    budgetPolicyId: snapshot.budget_policy_id,
    configSnapshotId: snapshot.config_snapshot_id,
    activatedAt: snapshot.activated_at,
    providerResourceIds: snapshot.provider_resource_ids,
    routePolicyId: snapshot.route_policy_id,
    revision: snapshot.revision,
    projectId: snapshot.project_id,
    status: snapshot.status,
    tenantId: snapshot.tenant_id,
  };
}

function parseControlPlaneSnapshotList(payload: unknown): unknown[] {
  return extractListPayload(
    payload,
    ["data", "config_snapshots", "snapshots", "items"],
    true,
  );
}

function parseSingleSnapshot(payload: unknown) {
  if (!isRecord(payload)) {
    throw new Error("Malformed config snapshot response payload");
  }

  const snapshotRecord = payload.config_snapshot
    ? pickRecordValue<unknown>(payload, ["config_snapshot"])
    : payload;

  const snapshot = configSnapshotSchema.parse(snapshotRecord);

  return toConfigSnapshotView(snapshot);
}

function parseConfigSnapshotList(payload: unknown) {
  const snapshots = parseControlPlaneSnapshotList(payload);

  return snapshots.map((snapshot) => parseSingleSnapshot(snapshot));
}

function parseApiKeyRecord(record: unknown): ApiKeyView {
  if (!isRecord(record)) {
    throw new Error("Malformed api key response payload");
  }

  const apiKeyId =
    stringOrUndefined(
      pickRecordValue<string>(record, ["api_key_id", "apiKeyId", "id"]),
    ) ?? "unknown";
  const displayName =
    stringOrUndefined(
      pickRecordValue<string>(record, ["display_name", "displayName"]),
    ) ?? "API key";
  const keyPrefix =
    stringOrUndefined(
      pickRecordValue<string>(record, ["key_prefix", "keyPrefix"]),
    ) ?? "••••";
  const providerResourceId =
    stringOrUndefined(
      pickRecordValue<string>(record, [
        "provider_resource_id",
        "providerResourceId",
      ]),
    ) ?? "unassigned";
  const canRevoke =
    booleanOrUndefined(
      pickRecordValue<boolean>(record, ["can_revoke", "canRevoke"]),
    ) ?? true;
  const isActive =
    booleanOrUndefined(
      pickRecordValue<boolean>(record, ["is_active", "isActive"]),
    ) ?? true;
  const version =
    numberOrUndefined(pickRecordValue<number>(record, ["version"])) ??
    numberOrUndefined(pickRecordValue<string>(record, ["version"])) ??
    1;
  const tenantId =
    stringOrUndefined(
      pickRecordValue<string>(record, ["tenant_id", "tenantId"]),
    ) ?? undefined;

  return {
    apiKeyId,
    canRevoke,
    createdAt:
      stringOrUndefined(
        pickRecordValue<string>(record, ["created_at", "createdAt"]),
      ) ?? "",
    displayName,
    isActive,
    keyPrefix,
    providerResourceId,
    tenantId,
    updatedAt:
      stringOrUndefined(
        pickRecordValue<string>(record, ["updated_at", "updatedAt"]),
      ) ?? "",
    version,
  };
}

function parseApiKeyList(payload: unknown) {
  return extractListPayload(payload, [
    "data",
    "api_keys",
    "apiKeys",
    "items",
  ]).map(parseApiKeyRecord);
}

async function getActiveSnapshotOrNull(apiClient: ControlPlaneClient) {
  try {
    return await apiClient.getConfigSnapshot("active");
  } catch {
    return null;
  }
}

async function getRouteSimulationOrNull(
  apiClient: ControlPlaneClient,
  snapshot: ConfigSnapshot | null,
  routePolicies: RoutePolicy[],
  providerResources: ProviderResource[],
) {
  if (!snapshot) {
    return null;
  }

  const routePolicy = routePolicies.find(
    (policy) => policy.route_policy_id === snapshot.route_policy_id,
  );

  if (!routePolicy) {
    return null;
  }

  const region =
    providerResources.find(
      (provider) =>
        provider.provider_resource_id === snapshot.provider_resource_ids[0],
    )?.region ??
    routePolicy.preferred_regions[0] ??
    "us-east-1";

  try {
    return await apiClient.simulateRoute({
      credential_scope: "cred_console",
      expected_max_output_tokens: 256,
      expected_prompt_tokens: 64,
      model_alias: routePolicy.model_alias,
      project_id: snapshot.project_id,
      protocol_family: routePolicy.protocol_family,
      region,
      required_capabilities: routePolicy.required_capabilities,
      tenant_id: snapshot.tenant_id,
      traffic_class: "console_preview",
    });
  } catch {
    return null;
  }
}

async function listConfigSnapshotsFromControlPlane() {
  try {
    return await requestControlPlaneJson(
      "/v1/config-snapshots",
      parseConfigSnapshotList,
      {
        headers: {
          Accept: "application/json",
        },
        method: "GET",
      },
    );
  } catch (error) {
    if (isRecoverableMissingEndpoint(error)) {
      const activeSnapshot = await getActiveSnapshotOrNull(client);

      return activeSnapshot ? [toConfigSnapshotView(activeSnapshot)] : [];
    }

    throw error;
  }
}

async function activateConfigSnapshotFromControlPlane(
  configSnapshotId: string,
) {
  try {
    const activeSnapshot =
      await client.activateConfigSnapshot(configSnapshotId);

    return toConfigSnapshotView(activeSnapshot);
  } catch (error) {
    if (isRecoverableMissingEndpoint(error)) {
      const activated = await requestControlPlaneJson(
        `/v1/config-snapshots/${encodeURIComponent(configSnapshotId)}/activate`,
        parseSingleSnapshot,
        {
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );

      return activated;
    }

    throw error;
  }
}

async function listApiKeysFromControlPlane() {
  try {
    return await requestControlPlaneJson("/v1/api-keys", parseApiKeyList, {
      headers: {
        Accept: "application/json",
      },
      method: "GET",
    });
  } catch (error) {
    if (isRecoverableMissingEndpoint(error)) {
      return [];
    }

    throw error;
  }
}

async function listRouteReceiptsFromControlPlane() {
  try {
    return await requestControlPlaneJson(
      "/v1/route-receipts",
      parseRouteReceiptList,
      {
        headers: {
          Accept: "application/json",
        },
        method: "GET",
      },
    );
  } catch (error) {
    if (isRecoverableMissingEndpoint(error)) {
      return [];
    }

    throw error;
  }
}

async function getRouteReceiptDiagnosticsFromControlPlane(
  routeReceiptId: string,
) {
  try {
    return await client.getRouteReceiptDiagnostics(routeReceiptId);
  } catch (error) {
    if (isRecoverableMissingEndpoint(error)) {
      return null;
    }

    if (
      error instanceof ControlPlaneClientError &&
      isNotFoundErrorStatus(error.status)
    ) {
      return null;
    }

    throw error;
  }
}

async function revokeApiKeyFromControlPlane(apiKeyId: string, version: number) {
  const endpoint = `/v1/api-keys/${encodeURIComponent(apiKeyId)}/revoke`;
  const body = JSON.stringify({ version });

  const requestBodyHeaders = {
    Accept: "application/json",
    "Content-Type": "application/json",
  };

  try {
    await requestControlPlaneJson(endpoint, () => undefined, {
      body,
      headers: requestBodyHeaders,
      method: "DELETE",
    });

    return;
  } catch (error) {
    if (isRecoverableMissingEndpoint(error)) {
      await requestControlPlaneJson(endpoint, () => undefined, {
        body,
        headers: requestBodyHeaders,
        method: "POST",
      });

      return;
    }

    throw error;
  }
}

async function listTenantApiKeysFromControlPlane() {
  const keys = await listApiKeysFromControlPlane();
  const tenantId = getActiveTenantId();

  if (!tenantId || isPlatformAdmin()) {
    return keys;
  }

  const providerResources = await client.listProviderResources();
  const allowedProviderIds = new Set(
    providerResources
      .filter((provider) => provider.tenant_id === tenantId)
      .map((provider) => provider.provider_resource_id),
  );

  return keys.filter(
    (key) =>
      key.tenantId === tenantId ||
      (key.tenantId === undefined &&
        allowedProviderIds.has(key.providerResourceId)),
  );
}

function selectedProviderLabel(
  simulation: RouteSimulationResponse | null,
  providerResources: ProviderResource[],
) {
  if (!simulation?.selected_target) {
    return "No eligible provider";
  }

  return (
    providerResources.find(
      (provider) =>
        provider.provider_resource_id === simulation.selected_target,
    )?.name ?? simulation.selected_target
  );
}

async function loadControlPlaneData() {
  const [tenants, projects, providerResources, routePolicies, activeSnapshot] =
    await Promise.all([
      client.listTenants(),
      client.listProjects(),
      client.listProviderResources(),
      client.listRoutePolicies(),
      getActiveSnapshotOrNull(client),
    ]);

  return {
    activeSnapshot,
    projects,
    providerResources,
    routePolicies,
    tenants,
  };
}

const defaultConsoleDataService: ConsoleDataService = {
  async getOverview() {
    const {
      activeSnapshot,
      projects,
      providerResources,
      routePolicies,
      tenants,
    } = await loadControlPlaneData();
    const tenantId =
      getActiveTenantId() ?? activeSnapshot?.tenant_id ?? tenants[0]?.tenant_id;
    const tenant =
      tenants.find((candidate) => candidate.tenant_id === tenantId) ??
      tenants[0];

    if (!tenant) {
      throw new Error("tenant_not_found");
    }

    const tenantProjects = projects
      .filter((project) => project.tenant_id === tenant.tenant_id)
      .map(toProjectSummary);
    const tenantProviders = providerResources.filter(
      (provider) => provider.tenant_id === tenant.tenant_id,
    );
    const tenantRoutePolicies = routePolicies.filter(
      (policy) => policy.tenant_id === tenant.tenant_id,
    );
    const simulation = await getRouteSimulationOrNull(
      client,
      activeSnapshot?.tenant_id === tenant.tenant_id ? activeSnapshot : null,
      tenantRoutePolicies,
      tenantProviders,
    );

    return {
      activeProviders: tenantProviders.filter(
        (provider) => provider.status === "active",
      ).length,
      activeRoutes: tenantRoutePolicies.length,
      activeSnapshotId:
        activeSnapshot?.tenant_id === tenant.tenant_id
          ? activeSnapshot.config_snapshot_id
          : "No active snapshot",
      estimatedCostUsd: simulation?.estimated_cost.amount ?? "0.000000",
      projects: tenantProjects,
      selectedProvider: selectedProviderLabel(simulation, tenantProviders),
      tenantLabel: tenant.display_name,
      workspace: tenant.slug,
    };
  },

  async getUsageDashboard(range, groupBy, projectId, cursor) {
    const tenantId = getActiveTenantId();

    if (!tenantId) {
      throw new Error("tenant_not_found");
    }

    const window = rangeToWindow(range);
    const projects = (await client.listProjects())
      .filter((project) => project.tenant_id === tenantId)
      .map(toProjectSummary);
    const [summary, breakdown] = await Promise.all([
      client.getUsageSummary({
        project_id: projectId,
        tenant_id: tenantId,
        window_end: window.windowEnd,
        window_start: window.windowStart,
      }),
      client.getUsageBreakdown({
        cursor,
        group_by: groupBy,
        project_id: projectId,
        tenant_id: tenantId,
        window_end: window.windowEnd,
        window_start: window.windowStart,
      }),
    ]);

    return {
      activeProjectId: projectId,
      availableProjects: projects,
      billablePriceUsd: formatUsdAmount(summary.data.billable_price),
      breakdown: breakdown.data.map(toUsageBreakdownView),
      cachedInputTokens: summary.data.cached_input_tokens,
      eventCount: summary.data.event_count,
      groupBy,
      inputTokens: summary.data.input_tokens,
      nextCursor: breakdown.next_cursor,
      outputTokens: summary.data.output_tokens,
      providerCostUsd: formatUsdAmount(summary.data.provider_cost),
      rangeLabel: window.label,
      windowEnd: summary.data.window_end,
      windowStart: summary.data.window_start,
    };
  },

  async getBillingDashboard(range, projectId) {
    const tenantId = getActiveTenantId();

    if (!tenantId) {
      throw new Error("tenant_not_found");
    }

    const window = rangeToWindow(range);
    const projects = (await client.listProjects())
      .filter((project) => project.tenant_id === tenantId)
      .map(toProjectSummary);
    const [projection, exportJobs] = await Promise.all([
      client.getBalanceProjection({
        project_id: projectId,
        tenant_id: tenantId,
      }),
      client.listBillingExports({
        project_id: projectId,
        tenant_id: tenantId,
      }),
    ]);

    return {
      activeProjectId: projectId,
      availableProjects: projects,
      billableTotalUsd: formatUsdAmount(projection.data.billable_total),
      configuredBudgetUsd: formatUsdAmount(projection.data.configured_budget),
      exportJobs: exportJobs.data.map(toBillingExportJobView),
      lastProjectedAt: projection.data.last_projected_at,
      projectionLagSeconds: projection.data.projection_lag_seconds,
      providerCostTotalUsd: formatUsdAmount(
        projection.data.provider_cost_total,
      ),
      rangeLabel: window.label,
      remainingBudgetUsd: formatUsdAmount(projection.data.remaining_budget),
      thresholdStatus: projection.data.threshold_status,
    };
  },

  async queueBillingExport(range, projectId) {
    const tenantId = getActiveTenantId();

    if (!tenantId) {
      throw new Error("tenant_not_found");
    }

    const window = rangeToWindow(range);
    const exportJob = await client.createBillingExport({
      format: "csv",
      project_id: projectId,
      tenant_id: tenantId,
      window_end: window.windowEnd,
      window_start: window.windowStart,
    });

    return toBillingExportJobView(exportJob.data);
  },

  async getTenantDetail(tenantId) {
    const {
      activeSnapshot,
      projects,
      providerResources,
      routePolicies,
      tenants,
    } = await loadControlPlaneData();
    const tenant = tenants.find(
      (candidate) => candidate.tenant_id === tenantId,
    );

    if (!tenant) {
      throw new Error("tenant_not_found");
    }

    const tenantProjects = projects
      .filter((project) => project.tenant_id === tenantId)
      .map(toProjectSummary);
    const tenantProviders = providerResources.filter(
      (provider) => provider.tenant_id === tenantId,
    );
    const tenantRoutePolicies = routePolicies.filter(
      (policy) => policy.tenant_id === tenantId,
    );
    const simulation = await getRouteSimulationOrNull(
      client,
      activeSnapshot?.tenant_id === tenantId ? activeSnapshot : null,
      tenantRoutePolicies,
      tenantProviders,
    );

    return {
      activeConfigSnapshotId:
        activeSnapshot?.tenant_id === tenantId
          ? activeSnapshot.config_snapshot_id
          : undefined,
      displayName: tenant.display_name,
      estimatedCostUsd: simulation?.estimated_cost.amount,
      id: tenant.tenant_id,
      projects: tenantProjects,
      providers: tenantProviders,
      routePolicies: mapRoutePolicies(
        tenantRoutePolicies,
        tenantProviders,
        activeSnapshot,
      ),
      selectedProvider: selectedProviderLabel(simulation, tenantProviders),
      slug: tenant.slug,
      updatedAt: tenant.updated_at,
      version: tenant.version,
    };
  },

  async listProviderResources() {
    const providerResources = await client.listProviderResources();

    return filterByTenant(providerResources);
  },

  async listRoutePolicies() {
    const [providerResources, routePolicies, activeSnapshot] =
      await Promise.all([
        client.listProviderResources(),
        client.listRoutePolicies(),
        getActiveSnapshotOrNull(client),
      ]);
    const filteredProviders = filterByTenant(providerResources);
    const filteredPolicies = filterByTenant(routePolicies);

    return mapRoutePolicies(
      filteredPolicies,
      filteredProviders,
      activeSnapshot,
    );
  },

  async listRouteReceipts() {
    const [routeReceipts, providerResources] = await Promise.all([
      listRouteReceiptsFromControlPlane(),
      client.listProviderResources(),
    ]);
    const filteredReceipts = filterByTenant(routeReceipts);
    const providerById = providerLabelById(filterByTenant(providerResources));
    const recentReceipts = filteredReceipts
      .sort((left, right) => right.created_at.localeCompare(left.created_at))
      .slice(0, 10);
    const diagnosticsByReceiptId = new Map(
      await Promise.all(
        recentReceipts.map(
          async (receipt) =>
            [
              receipt.route_receipt_id,
              await getRouteReceiptDiagnosticsFromControlPlane(
                receipt.route_receipt_id,
              ),
            ] as const,
        ),
      ),
    );

    return recentReceipts.map((receipt) =>
      toRouteReceiptDiagnostic(
        receipt,
        providerById,
        diagnosticsByReceiptId.get(receipt.route_receipt_id),
      ),
    );
  },

  async listTenants() {
    const {
      activeSnapshot,
      projects,
      providerResources,
      routePolicies,
      tenants,
    } = await loadControlPlaneData();

    return tenants.map((tenant) => ({
      activeConfigSnapshotId:
        activeSnapshot?.tenant_id === tenant.tenant_id
          ? activeSnapshot.config_snapshot_id
          : undefined,
      displayName: tenant.display_name,
      id: tenant.tenant_id,
      projectCount: projects.filter(
        (project) => project.tenant_id === tenant.tenant_id,
      ).length,
      providerCount: providerResources.filter(
        (provider) => provider.tenant_id === tenant.tenant_id,
      ).length,
      routePolicyCount: routePolicies.filter(
        (policy) => policy.tenant_id === tenant.tenant_id,
      ).length,
      slug: tenant.slug,
      updatedAt: tenant.updated_at,
    }));
  },

  async listConfigSnapshots() {
    const snapshots = await listConfigSnapshotsFromControlPlane();
    const filteredSnapshots = filterByTenant(snapshots);

    return filteredSnapshots.sort((left, right) =>
      right.revision === left.revision
        ? right.configSnapshotId.localeCompare(left.configSnapshotId)
        : right.revision - left.revision,
    );
  },

  async activateConfigSnapshot(configSnapshotId) {
    const activated =
      await activateConfigSnapshotFromControlPlane(configSnapshotId);

    return activated;
  },

  async listApiKeys() {
    return listTenantApiKeysFromControlPlane();
  },

  async revokeApiKey(apiKeyId, version) {
    await revokeApiKeyFromControlPlane(apiKeyId, version);
  },
};

let consoleDataServiceOverride: ConsoleDataService | null = null;

export function getConsoleDataService() {
  return consoleDataServiceOverride ?? defaultConsoleDataService;
}

export function setConsoleDataServiceForTests(
  service: ConsoleDataService | null,
) {
  consoleDataServiceOverride = service;
}
