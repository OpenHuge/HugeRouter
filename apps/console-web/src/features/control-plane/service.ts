import {
  createControlPlaneClient,
  ContractApiError,
  ControlPlaneClientError,
  type ControlPlaneClient,
} from "@huge-router/ts-api-client";
import {
  cardProductSchema,
  type BillingExportJob,
  configSnapshotSchema,
  type ConfigSnapshot,
  merchantShopSchema,
  merchantWorkspaceResponseSchema,
  type Project,
  type ProviderResource,
  providerResourceSchema,
  relayEvaluationSchema,
  replayCapsuleResponseSchema,
  type RoutePolicy,
  type RouteReceipt,
  routePolicySchema,
  routeReceiptSchema,
  type RouteSimulationResponse,
  trialConnectionSchema,
} from "@huge-router/ts-shared-schema";
import type { AuthSessionEnvelope } from "../auth/auth-contract";
import { authSessionQueryKey } from "../auth/auth-queries";
import { getQueryClient } from "../../lib/query-client";
import type {
  ApiKeyView,
  ApiKeyCreateResult,
  BillingExportJobView,
  BillingDashboardData,
  CardProductView,
  ConfigSnapshotView,
  MerchantShopView,
  MerchantWorkspaceData,
  OverviewData,
  ReplayCapsuleView,
  ProjectSummary,
  RelayEvaluationView,
  RouteDiagnosticsView,
  RoutePolicyView,
  RouteReceiptView,
  TenantDetail,
  TenantSummary,
  TrialConnectionView,
  UsageBreakdownView,
  UsageDashboardData,
} from "./types";

export type ConsoleDataService = {
  getOverview: () => Promise<OverviewData>;
  listProjects: () => Promise<ProjectSummary[]>;
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
  getRouteDiagnostics: (routePolicyId: string) => Promise<RouteDiagnosticsView>;
  getMerchantWorkspace: () => Promise<MerchantWorkspaceData>;
  getReplayCapsule: (replayCapsuleId: string) => Promise<ReplayCapsuleView>;
  getTenantDetail: (tenantId: string) => Promise<TenantDetail>;
  listProviderResources: () => Promise<ProviderResource[]>;
  listRoutePolicies: () => Promise<RoutePolicyView[]>;
  listRouteReceipts: () => Promise<RouteReceiptView[]>;
  listTenants: () => Promise<TenantSummary[]>;
  listConfigSnapshots: () => Promise<ConfigSnapshotView[]>;
  createConfigSnapshot: (
    snapshot: ConfigSnapshotMutationInput,
  ) => Promise<ConfigSnapshotView>;
  activateConfigSnapshot: (
    configSnapshotId: string,
  ) => Promise<ConfigSnapshotView>;
  createProviderResource: (
    providerResource: ProviderResourceMutationInput,
  ) => Promise<ProviderResource>;
  updateProviderResource: (
    providerResourceId: string,
    providerResource: ProviderResourceMutationInput,
    expectedVersion: number,
  ) => Promise<ProviderResource>;
  disableProviderResource: (
    providerResourceId: string,
    expectedVersion: number,
  ) => Promise<ProviderResource>;
  uploadCodexAuthAccount: (
    input: CodexAuthAccountUploadInput,
  ) => Promise<CodexAuthAccountView>;
  listCodexAuthAccounts: () => Promise<CodexAuthAccountView[]>;
  listOAuthSharingLeases: () => Promise<OAuthSharingLeaseView[]>;
  upsertOAuthSharingLease: (
    input: OAuthSharingLeaseMutationInput,
  ) => Promise<OAuthSharingLeaseView>;
  revokeOAuthSharingLease: (leaseId: string) => Promise<OAuthSharingLeaseView>;
  listOAuthCarpools: () => Promise<OAuthCarpoolView[]>;
  upsertOAuthCarpool: (
    input: OAuthCarpoolMutationInput,
  ) => Promise<OAuthCarpoolView>;
  removeOAuthCarpool: (carpoolId: string) => Promise<OAuthCarpoolView>;
  readOAuthSharingUsage: (
    workspaceId?: string,
  ) => Promise<OAuthSharingUsageView>;
  createRoutePolicy: (
    routePolicy: RoutePolicyMutationInput,
  ) => Promise<RoutePolicy>;
  updateRoutePolicy: (
    routePolicyId: string,
    routePolicy: RoutePolicyMutationInput,
    expectedVersion: number,
  ) => Promise<RoutePolicy>;
  disableRoutePolicy: (
    routePolicyId: string,
    expectedVersion: number,
  ) => Promise<RoutePolicy>;
  listApiKeys: () => Promise<ApiKeyView[]>;
  createApiKey: (input: {
    apiKey: string;
    displayName: string;
    providerResourceId: string;
  }) => Promise<ApiKeyCreateResult>;
  createMerchantShop: (input: {
    merchantShopId: string;
    slug: string;
    displayName: string;
    announcement?: string;
  }) => Promise<MerchantShopView>;
  createCardProduct: (input: {
    cardProductId: string;
    merchantShopId: string;
    title: string;
    description: string;
    inventoryCount: number;
    faceValueUsd: string;
    retailPriceUsd: string;
    supportsTrial: boolean;
  }) => Promise<CardProductView>;
  createTrialConnection: (input: {
    trialConnectionId: string;
    providerLabel: string;
    endpointBaseUrl: string;
    apiKey: string;
    targetModel: string;
    notes?: string;
  }) => Promise<TrialConnectionView>;
  runRelayEvaluation: (input: {
    trialConnectionId: string;
  }) => Promise<RelayEvaluationView>;
  revokeApiKey: (apiKeyId: string, version: number) => Promise<void>;
  downloadBillingExport: (exportJobId: string) => Promise<string>;
};

type ActionErrorKind =
  | "api-key-create"
  | "api-key-revoke"
  | "billing-export-download"
  | "billing-export-queue"
  | "card-product-create"
  | "merchant-shop-create"
  | "provider-create"
  | "provider-disable"
  | "provider-update"
  | "codex-auth-upload"
  | "relay-evaluation-create"
  | "replay-capsule-load"
  | "route-policy-create"
  | "route-policy-disable"
  | "route-policy-update"
  | "sharing"
  | "snapshot-activate"
  | "snapshot-create"
  | "trial-connection-create";

export type ProviderResourceMutationInput = {
  authKind: "api_key" | "oauth_client_credentials" | "session_broker";
  budgetPolicyId?: string;
  capabilities: {
    supportsJsonMode: boolean;
    supportsStreaming: boolean;
    supportsToolCalling: boolean;
  };
  credentialOwnerType: "platform" | "tenant" | "project" | "partner";
  createdAt?: string;
  deploymentScope: "shared" | "tenant_dedicated" | "project_dedicated";
  endpointBaseUrl: string;
  healthState: "healthy" | "degraded" | "quarantined" | "draining" | "disabled";
  name: string;
  projectId?: string;
  providerId: string;
  providerResourceId: string;
  provenanceClass:
    | "official_api"
    | "official_gateway"
    | "byo_customer_credential"
    | "dedicated_managed_account"
    | "shared_brokered_pool"
    | "unofficial_client_channel";
  region: string;
  status: "active" | "disabled" | "draining" | "quarantined" | "deleted";
  version?: number;
};

export type CodexAuthAccountUploadInput = {
  authJson: unknown;
  displayName: string;
  endpointBaseUrl?: string;
  projectId?: string;
  providerResourceId?: string;
  region?: string;
};

export type CodexAuthAccountView = {
  codexAccountId: string;
  displayName: string;
  encryptedAuthJsonKeyId: string;
  providerResourceId: string;
  status: string;
  authJsonSha256: string;
  createdAt: string;
  updatedAt: string;
};

export type OAuthSharingLeaseView = {
  leaseId: string;
  ownerWorkspaceId?: string;
  borrowerWorkspaceId: string;
  provider: "codex" | "gemini" | "claude_code";
  poolId: string;
  allowedAccountIds: string[];
  status: "pending" | "active" | "paused" | "expired" | "revoked";
  startsAt: string;
  expiresAt: string;
  maxConcurrentRuns: number;
  turnBudget?: number;
  policy: "fair_share" | "owner_priority" | "borrower_priority";
  createdAt: string;
  updatedAt: string;
};

export type OAuthSharingLeaseMutationInput = Omit<
  OAuthSharingLeaseView,
  "createdAt" | "updatedAt"
>;

export type OAuthCarpoolView = {
  carpoolId: string;
  provider: "codex" | "gemini" | "claude_code";
  name: string;
  memberWorkspaceIds: string[];
  poolIds: string[];
  strategy: "fair_share" | "weighted" | "cheapest_ready" | "fastest_ready";
  perMemberConcurrencyLimit?: number;
  perMemberTurnBudget?: number;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
};

export type OAuthCarpoolMutationInput = Omit<
  OAuthCarpoolView,
  "createdAt" | "updatedAt"
>;

export type OAuthSharingUsageView = {
  rows: {
    leaseId?: string;
    carpoolId?: string;
    workspaceId?: string;
    provider: string;
    accountId?: string;
    turns: number;
  }[];
  auditEvents: {
    eventType: string;
    reason: string;
    provider: string;
    createdAt: string;
  }[];
};

export type RoutePolicyMutationInput = {
  createdAt?: string;
  displayName: string;
  modelAlias: string;
  preferredRegions: string[];
  protocolFamily:
    | "openai_chat"
    | "openai_responses"
    | "openai_images"
    | "mcp_streamable_http"
    | "realtime_webrtc"
    | "anthropic_messages"
    | "gemini_generate_content";
  requiredCapabilities: string[];
  routePolicyId: string;
  updatedAt?: string;
  version?: number;
};

export type ConfigSnapshotMutationInput = {
  activatedAt?: string;
  budgetPolicyId: string;
  configSnapshotId: string;
  projectId: string;
  providerResourceIds: string[];
  revision: number;
  routePolicyId: string;
  status: "draft" | "active" | "superseded";
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

function arrayOrEmpty(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
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
  routeReceipts: RouteReceipt[] = [],
): RoutePolicyView[] {
  const providerNames = new Map(
    providerResources.map((provider) => [
      provider.provider_resource_id,
      provider.name,
    ]),
  );

  return routePolicies.map((policy) => ({
    lastFailureReason: latestRouteReceiptForPolicy(
      routeReceipts,
      policy.route_policy_id,
    )?.failure_reason,
    lastReceiptId: latestRouteReceiptForPolicy(
      routeReceipts,
      policy.route_policy_id,
    )?.route_receipt_id,
    lastReceiptOutcome: latestRouteReceiptForPolicy(
      routeReceipts,
      policy.route_policy_id,
    )?.admission_result,
    createdAt: policy.created_at,
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
    tenantId: policy.tenant_id,
    updatedAt: policy.updated_at,
    version: policy.version,
  }));
}

function routePolicyLabelById(routePolicies: RoutePolicy[]) {
  return new Map(
    routePolicies.map((policy) => [
      policy.route_policy_id,
      policy.display_name,
    ]),
  );
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

function latestRouteReceiptForPolicy(
  routeReceipts: RouteReceipt[],
  routePolicyId: string,
) {
  return routeReceipts
    .filter((receipt) => receipt.route_policy_id === routePolicyId)
    .sort((left, right) => right.created_at.localeCompare(left.created_at))[0];
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

function toRouteReceiptView(
  receipt: RouteReceipt,
  providerById: Map<string, string>,
  routePoliciesById: Map<string, string>,
): RouteReceiptView {
  return {
    admissionResult: receipt.admission_result,
    createdAt: receipt.created_at,
    excludedTargets: receipt.excluded_targets,
    failureReason: receipt.failure_reason,
    fallbackTransitions: receipt.fallback_transitions,
    modelAlias: receipt.model_alias,
    normalizedError: receipt.normalized_error,
    protocolFamily: receipt.protocol_family,
    receiptId: receipt.route_receipt_id,
    routeName:
      routePoliciesById.get(receipt.route_policy_id) ?? receipt.route_policy_id,
    routePolicyId: receipt.route_policy_id,
    scoreBreakdown: receipt.score_breakdown,
    selectedTargetId: receipt.selected_target ?? undefined,
    selectedTargetName: receipt.selected_target
      ? mapProviderLabel(receipt.selected_target, providerById)
      : undefined,
  };
}

function mapRouteReceipts(
  routeReceipts: RouteReceipt[],
  providerResources: ProviderResource[],
  routePolicies: RoutePolicy[],
): RouteReceiptView[] {
  const providerById = providerLabelById(providerResources);
  const routePoliciesById = routePolicyLabelById(routePolicies);

  return routeReceipts
    .slice()
    .sort((left, right) => right.created_at.localeCompare(left.created_at))
    .map((receipt) =>
      toRouteReceiptView(receipt, providerById, routePoliciesById),
    );
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

function toMerchantShopView(
  shop: ReturnType<typeof merchantShopSchema.parse>,
): MerchantShopView {
  return {
    announcement: shop.announcement,
    createdAt: shop.created_at,
    displayName: shop.display_name,
    fulfillmentMode: shop.fulfillment_mode,
    merchantShopId: shop.merchant_shop_id,
    slug: shop.slug,
    status: shop.status,
    updatedAt: shop.updated_at,
    version: shop.version,
  };
}

function toCardProductView(
  product: ReturnType<typeof cardProductSchema.parse>,
): CardProductView {
  return {
    cardProductId: product.card_product_id,
    createdAt: product.created_at,
    deliveryKind: product.delivery_kind,
    description: product.description,
    faceValueUsd: product.face_value_usd,
    inventoryCount: product.inventory_count,
    merchantShopId: product.merchant_shop_id,
    retailPriceUsd: product.retail_price_usd,
    status: product.status,
    supportsTrial: product.supports_trial,
    title: product.title,
    updatedAt: product.updated_at,
    version: product.version,
  };
}

function toTrialConnectionView(
  connection: ReturnType<typeof trialConnectionSchema.parse>,
): TrialConnectionView {
  return {
    apiKeyMasked: connection.api_key_masked,
    createdAt: connection.created_at,
    endpointBaseUrl: connection.endpoint_base_url,
    lastVerifiedAt: connection.last_verified_at,
    notes: connection.notes,
    providerLabel: connection.provider_label,
    status: connection.status,
    targetModel: connection.target_model,
    trialConnectionId: connection.trial_connection_id,
    updatedAt: connection.updated_at,
    version: connection.version,
  };
}

function toRelayEvaluationView(
  evaluation: ReturnType<typeof relayEvaluationSchema.parse>,
): RelayEvaluationView {
  return {
    createdAt: evaluation.created_at,
    detectedChannel: evaluation.detected_channel,
    endpointBaseUrl: evaluation.endpoint_base_url,
    estimatedTokensSaved: evaluation.estimated_tokens_saved,
    fingerprintStatus: evaluation.fingerprint_status,
    multimodalStatus: evaluation.multimodal_status,
    overallScore: evaluation.overall_score,
    protocolStatus: evaluation.protocol_status,
    providerLabel: evaluation.provider_label,
    relayEvaluationId: evaluation.relay_evaluation_id,
    replayCapsuleId: evaluation.replay_capsule_id,
    runnerMode: evaluation.runner_mode,
    sampleRequestCount: evaluation.sample_request_count,
    summary: evaluation.summary,
    targetModel: evaluation.target_model,
    tokenStatus: evaluation.token_status,
    trialConnectionId: evaluation.trial_connection_id,
    verdict: evaluation.verdict,
  };
}

function parseMerchantWorkspace(payload: unknown): MerchantWorkspaceData {
  const parsed = merchantWorkspaceResponseSchema.parse(payload).data;

  return {
    cardProducts: parsed.card_products.map(toCardProductView),
    merchantEnabled: parsed.merchant_enabled,
    recentEvaluations: parsed.recent_evaluations.map(toRelayEvaluationView),
    shops: parsed.shops.map(toMerchantShopView),
    tenantId: parsed.tenant_id,
    trialConnections: parsed.trial_connections.map(toTrialConnectionView),
  };
}

function parseReplayCapsule(payload: unknown): ReplayCapsuleView {
  const parsed = replayCapsuleResponseSchema.parse(payload).replay_capsule;

  return {
    configSnapshotId: parsed.config_snapshot_id,
    normalizedRequestSummary: {
      estimatedPromptTokens:
        parsed.normalized_request_summary.estimated_prompt_tokens,
      modelAlias: parsed.normalized_request_summary.model_alias,
      protocolFamily: parsed.normalized_request_summary.protocol_family,
    },
    redactionTier: parsed.redaction_tier,
    replayCapsuleId: parsed.replay_capsule_id,
    requestId: parsed.request_id,
    routeReceiptId: parsed.route_receipt_id,
    traceId: parsed.trace_id,
    upstreamErrorCode: parsed.upstream_error_summary?.code,
  };
}

function parseProviderResourceRecord(payload: unknown) {
  return providerResourceSchema.parse(payload);
}

function parseRoutePolicyRecord(payload: unknown) {
  return routePolicySchema.parse(payload);
}

function parseConfigSnapshotRecord(payload: unknown) {
  return toConfigSnapshotView(configSnapshotSchema.parse(payload));
}

function isContractApiError(value: unknown): value is ContractApiError {
  return value instanceof ContractApiError;
}

function getActionErrorCode(error: unknown) {
  if (isControlPlaneError(error)) {
    return error.code;
  }

  if (isContractApiError(error)) {
    return error.envelope.error.code;
  }

  return undefined;
}

function getActionErrorStatus(error: unknown) {
  if (isControlPlaneError(error) || isContractApiError(error)) {
    return error.status;
  }

  return undefined;
}

export function getControlPlaneActionErrorMessage(
  error: unknown,
  kind: ActionErrorKind,
) {
  const code = getActionErrorCode(error);
  const status = getActionErrorStatus(error);

  if (status === 401 || code === "auth_invalid") {
    return "Your console session expired. Sign in again and retry the action.";
  }

  if (status === 403) {
    return "This account is not allowed to perform that action.";
  }

  if (status === 404) {
    return kind === "billing-export-download"
      ? "That billing export is no longer available. Refresh the page and try again."
      : "That resource no longer exists. Refresh the page and try again.";
  }

  if (status === 409 || code?.endsWith("_version_conflict")) {
    return "This resource changed since you opened it. Refresh the page and try again.";
  }

  if (code === "route_policy_compatibility_invalid") {
    return "Use a supported protocol family and supported capability values only.";
  }

  if (
    code === "codex_auth_json_invalid" ||
    code === "codex_auth_json_too_large" ||
    code === "codex_auth_display_name_required" ||
    code === "provider_resource_invalid" ||
    code === "provider_resource_id_invalid" ||
    code === "route_policy_invalid" ||
    code === "route_policy_id_invalid" ||
    code === "config_snapshot_invalid" ||
    code === "provider_resource_id_invalid"
  ) {
    return "Review the form fields and submit again.";
  }

  if (kind === "billing-export-download") {
    return "The billing export could not be downloaded right now.";
  }

  if (kind === "replay-capsule-load") {
    return "The replay capsule could not be loaded right now.";
  }

  if (kind === "billing-export-queue") {
    return "The billing export could not be queued right now.";
  }

  if (kind === "provider-disable") {
    return "The provider resource could not be disabled.";
  }

  if (kind === "codex-auth-upload") {
    return "The Codex auth.json file could not be added to the account pool.";
  }

  if (kind === "route-policy-disable") {
    return "The route policy could not be disabled.";
  }

  if (kind === "api-key-revoke") {
    return "The API key could not be revoked.";
  }

  if (kind === "snapshot-activate") {
    return "The config snapshot could not be activated.";
  }

  return "The requested action could not be completed. Try again.";
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

async function createConfigSnapshotInControlPlane(snapshot: ConfigSnapshot) {
  return requestControlPlaneJson(
    "/v1/config-snapshots",
    parseConfigSnapshotRecord,
    {
      body: JSON.stringify(snapshot),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
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

async function createProviderResourceInControlPlane(
  providerResource: ProviderResource,
) {
  return requestControlPlaneJson(
    "/v1/provider-resources",
    parseProviderResourceRecord,
    {
      body: JSON.stringify(providerResource),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

async function updateProviderResourceInControlPlane(
  providerResourceId: string,
  providerResource: ProviderResource,
  expectedVersion: number,
) {
  return requestControlPlaneJson(
    `/v1/provider-resources/${encodeURIComponent(providerResourceId)}`,
    parseProviderResourceRecord,
    {
      body: JSON.stringify({
        ...providerResource,
        expected_version: expectedVersion,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "PUT",
    },
  );
}

async function disableProviderResourceInControlPlane(
  providerResourceId: string,
  expectedVersion: number,
) {
  return requestControlPlaneJson(
    `/v1/provider-resources/${encodeURIComponent(providerResourceId)}/disable`,
    parseProviderResourceRecord,
    {
      body: JSON.stringify({
        expected_version: expectedVersion,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

function toCodexAuthAccountView(payload: unknown): CodexAuthAccountView {
  if (!isRecord(payload)) {
    throw new ControlPlaneClientError(
      "Invalid Codex auth account payload.",
      500,
    );
  }

  return {
    authJsonSha256:
      stringOrUndefined(pickRecordValue(payload, ["auth_json_sha256"])) ?? "",
    codexAccountId:
      stringOrUndefined(pickRecordValue(payload, ["codex_account_id"])) ?? "",
    createdAt:
      stringOrUndefined(pickRecordValue(payload, ["created_at"])) ?? "",
    displayName:
      stringOrUndefined(pickRecordValue(payload, ["display_name"])) ?? "",
    encryptedAuthJsonKeyId:
      stringOrUndefined(
        pickRecordValue(payload, ["encrypted_auth_json_key_id"]),
      ) ?? "",
    providerResourceId:
      stringOrUndefined(pickRecordValue(payload, ["provider_resource_id"])) ??
      "",
    status: stringOrUndefined(pickRecordValue(payload, ["status"])) ?? "",
    updatedAt:
      stringOrUndefined(pickRecordValue(payload, ["updated_at"])) ?? "",
  };
}

function parseCodexAuthAccountList(payload: unknown) {
  if (!isRecord(payload) || !hasArray(payload.data)) {
    return [];
  }

  return payload.data.map(toCodexAuthAccountView);
}

function toOAuthSharingLeaseView(payload: unknown): OAuthSharingLeaseView {
  if (!isRecord(payload)) {
    throw new ControlPlaneClientError(
      "Invalid OAuth sharing lease payload.",
      500,
    );
  }

  const usageBudget = pickRecordValue(payload, ["usage_budget"]);
  const budget = isRecord(usageBudget) ? usageBudget : {};

  return {
    allowedAccountIds: arrayOrEmpty(
      pickRecordValue(payload, ["allowed_account_ids"]),
    ).map(String),
    borrowerWorkspaceId:
      stringOrUndefined(pickRecordValue(payload, ["borrower_workspace_id"])) ??
      "",
    createdAt:
      stringOrUndefined(pickRecordValue(payload, ["created_at"])) ?? "",
    expiresAt:
      stringOrUndefined(pickRecordValue(payload, ["expires_at"])) ?? "",
    leaseId: stringOrUndefined(pickRecordValue(payload, ["lease_id"])) ?? "",
    maxConcurrentRuns:
      numberOrUndefined(pickRecordValue(payload, ["max_concurrent_runs"])) ?? 1,
    ownerWorkspaceId: stringOrUndefined(
      pickRecordValue(payload, ["owner_workspace_id"]),
    ),
    policy:
      (stringOrUndefined(
        pickRecordValue(payload, ["policy"]),
      ) as OAuthSharingLeaseView["policy"]) ?? "fair_share",
    poolId: stringOrUndefined(pickRecordValue(payload, ["pool_id"])) ?? "",
    provider:
      (stringOrUndefined(
        pickRecordValue(payload, ["provider"]),
      ) as OAuthSharingLeaseView["provider"]) ?? "codex",
    startsAt: stringOrUndefined(pickRecordValue(payload, ["starts_at"])) ?? "",
    status:
      (stringOrUndefined(
        pickRecordValue(payload, ["status"]),
      ) as OAuthSharingLeaseView["status"]) ?? "pending",
    turnBudget: numberOrUndefined(pickRecordValue(budget, ["turns"])),
    updatedAt:
      stringOrUndefined(pickRecordValue(payload, ["updated_at"])) ?? "",
  };
}

function parseOAuthSharingLeaseList(payload: unknown) {
  if (!isRecord(payload) || !hasArray(payload.data)) {
    return [];
  }

  return payload.data.map(toOAuthSharingLeaseView);
}

function toOAuthCarpoolView(payload: unknown): OAuthCarpoolView {
  if (!isRecord(payload)) {
    throw new ControlPlaneClientError("Invalid OAuth carpool payload.", 500);
  }

  return {
    carpoolId:
      stringOrUndefined(pickRecordValue(payload, ["carpool_id"])) ?? "",
    createdAt:
      stringOrUndefined(pickRecordValue(payload, ["created_at"])) ?? "",
    enabled: Boolean(pickRecordValue(payload, ["enabled"])),
    memberWorkspaceIds: arrayOrEmpty(
      pickRecordValue(payload, ["member_workspace_ids"]),
    ).map(String),
    name: stringOrUndefined(pickRecordValue(payload, ["name"])) ?? "",
    perMemberConcurrencyLimit: numberOrUndefined(
      pickRecordValue(payload, ["per_member_concurrency_limit"]),
    ),
    perMemberTurnBudget: numberOrUndefined(
      pickRecordValue(payload, ["per_member_turn_budget"]),
    ),
    poolIds: arrayOrEmpty(pickRecordValue(payload, ["pool_ids"])).map(String),
    provider:
      (stringOrUndefined(
        pickRecordValue(payload, ["provider"]),
      ) as OAuthCarpoolView["provider"]) ?? "codex",
    strategy:
      (stringOrUndefined(
        pickRecordValue(payload, ["strategy"]),
      ) as OAuthCarpoolView["strategy"]) ?? "fair_share",
    updatedAt:
      stringOrUndefined(pickRecordValue(payload, ["updated_at"])) ?? "",
  };
}

function parseOAuthCarpoolList(payload: unknown) {
  if (!isRecord(payload) || !hasArray(payload.data)) {
    return [];
  }

  return payload.data.map(toOAuthCarpoolView);
}

function toOAuthSharingUsageView(payload: unknown): OAuthSharingUsageView {
  const rows = isRecord(payload) && hasArray(payload.data) ? payload.data : [];
  const auditEvents =
    isRecord(payload) && hasArray(payload.audit_events)
      ? payload.audit_events
      : [];

  return {
    auditEvents: auditEvents.map((event) => ({
      createdAt: isRecord(event)
        ? (stringOrUndefined(pickRecordValue(event, ["created_at"])) ?? "")
        : "",
      eventType: isRecord(event)
        ? (stringOrUndefined(pickRecordValue(event, ["event_type"])) ?? "")
        : "",
      provider: isRecord(event)
        ? (stringOrUndefined(pickRecordValue(event, ["provider"])) ?? "")
        : "",
      reason: isRecord(event)
        ? (stringOrUndefined(pickRecordValue(event, ["reason"])) ?? "")
        : "",
    })),
    rows: rows.map((row) => ({
      accountId: isRecord(row)
        ? stringOrUndefined(pickRecordValue(row, ["account_id"]))
        : undefined,
      carpoolId: isRecord(row)
        ? stringOrUndefined(pickRecordValue(row, ["carpool_id"]))
        : undefined,
      leaseId: isRecord(row)
        ? stringOrUndefined(pickRecordValue(row, ["lease_id"]))
        : undefined,
      provider: isRecord(row)
        ? (stringOrUndefined(pickRecordValue(row, ["provider"])) ?? "")
        : "",
      turns: isRecord(row)
        ? (numberOrUndefined(pickRecordValue(row, ["turns"])) ?? 0)
        : 0,
      workspaceId: isRecord(row)
        ? stringOrUndefined(pickRecordValue(row, ["workspace_id"]))
        : undefined,
    })),
  };
}

async function listCodexAuthAccountsFromControlPlane() {
  try {
    return await requestControlPlaneJson(
      "/v1/codex-auth-accounts",
      parseCodexAuthAccountList,
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

async function listOAuthSharingLeasesFromControlPlane() {
  try {
    return await requestControlPlaneJson(
      "/v1/oauth-sharing-leases",
      parseOAuthSharingLeaseList,
      {
        headers: { Accept: "application/json" },
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

async function upsertOAuthSharingLeaseInControlPlane(
  input: OAuthSharingLeaseMutationInput,
) {
  return requestControlPlaneJson(
    "/v1/oauth-sharing-leases",
    toOAuthSharingLeaseView,
    {
      body: JSON.stringify({
        allowed_account_ids:
          input.allowedAccountIds.length > 0
            ? input.allowedAccountIds
            : undefined,
        borrower_workspace_id: input.borrowerWorkspaceId,
        expires_at: input.expiresAt,
        lease_id: input.leaseId,
        max_concurrent_runs: input.maxConcurrentRuns,
        owner_workspace_id: input.ownerWorkspaceId,
        policy: input.policy,
        pool_id: input.poolId,
        provider: input.provider,
        starts_at: input.startsAt,
        status: input.status,
        usage_budget: {
          turns: input.turnBudget,
        },
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

async function revokeOAuthSharingLeaseInControlPlane(leaseId: string) {
  return requestControlPlaneJson(
    `/v1/oauth-sharing-leases/${encodeURIComponent(leaseId)}/revoke`,
    toOAuthSharingLeaseView,
    {
      headers: { Accept: "application/json" },
      method: "POST",
    },
  );
}

async function listOAuthCarpoolsFromControlPlane() {
  try {
    return await requestControlPlaneJson(
      "/v1/oauth-carpools",
      parseOAuthCarpoolList,
      {
        headers: { Accept: "application/json" },
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

async function upsertOAuthCarpoolInControlPlane(
  input: OAuthCarpoolMutationInput,
) {
  return requestControlPlaneJson("/v1/oauth-carpools", toOAuthCarpoolView, {
    body: JSON.stringify({
      carpool_id: input.carpoolId,
      enabled: input.enabled,
      member_workspace_ids: input.memberWorkspaceIds,
      name: input.name,
      per_member_concurrency_limit: input.perMemberConcurrencyLimit,
      per_member_turn_budget: input.perMemberTurnBudget,
      pool_ids: input.poolIds,
      provider: input.provider,
      strategy: input.strategy,
    }),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
}

async function removeOAuthCarpoolInControlPlane(carpoolId: string) {
  return requestControlPlaneJson(
    `/v1/oauth-carpools/${encodeURIComponent(carpoolId)}`,
    toOAuthCarpoolView,
    {
      headers: { Accept: "application/json" },
      method: "DELETE",
    },
  );
}

async function readOAuthSharingUsageFromControlPlane(workspaceId?: string) {
  const query = workspaceId
    ? `?workspace_id=${encodeURIComponent(workspaceId)}`
    : "";

  return requestControlPlaneJson(
    `/v1/oauth-sharing-usage${query}`,
    toOAuthSharingUsageView,
    {
      headers: { Accept: "application/json" },
      method: "GET",
    },
  );
}

async function uploadCodexAuthAccountInControlPlane(
  input: CodexAuthAccountUploadInput,
) {
  return requestControlPlaneJson(
    "/v1/codex-auth-accounts",
    toCodexAuthAccountView,
    {
      body: JSON.stringify({
        auth_json: input.authJson,
        display_name: input.displayName,
        endpoint_base_url: input.endpointBaseUrl,
        project_id: input.projectId,
        provider_resource_id: input.providerResourceId,
        region: input.region,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

async function createRoutePolicyInControlPlane(routePolicy: RoutePolicy) {
  return requestControlPlaneJson("/v1/route-policies", parseRoutePolicyRecord, {
    body: JSON.stringify(routePolicy),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
}

async function updateRoutePolicyInControlPlane(
  routePolicyId: string,
  routePolicy: RoutePolicy,
  expectedVersion: number,
) {
  return requestControlPlaneJson(
    `/v1/route-policies/${encodeURIComponent(routePolicyId)}`,
    parseRoutePolicyRecord,
    {
      body: JSON.stringify({
        ...routePolicy,
        expected_version: expectedVersion,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "PUT",
    },
  );
}

async function disableRoutePolicyInControlPlane(
  routePolicyId: string,
  expectedVersion: number,
) {
  return requestControlPlaneJson(
    `/v1/route-policies/${encodeURIComponent(routePolicyId)}/disable`,
    parseRoutePolicyRecord,
    {
      body: JSON.stringify({
        expected_version: expectedVersion,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
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

async function createApiKeyInControlPlane(input: {
  apiKey: string;
  displayName: string;
  providerResourceId: string;
}) {
  return requestControlPlaneJson("/v1/api-keys", parseApiKeyRecord, {
    body: JSON.stringify({
      api_key: input.apiKey,
      display_name: input.displayName,
      provider_resource_id: input.providerResourceId,
    }),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
}

async function getMerchantWorkspaceFromControlPlane() {
  return requestControlPlaneJson(
    "/v1/merchant/workspace",
    parseMerchantWorkspace,
    {
      headers: {
        Accept: "application/json",
      },
      method: "GET",
    },
  );
}

async function createMerchantShopInControlPlane(input: {
  merchantShopId: string;
  slug: string;
  displayName: string;
  announcement?: string;
}) {
  return requestControlPlaneJson(
    "/v1/merchant/shops",
    (payload) => toMerchantShopView(merchantShopSchema.parse(payload)),
    {
      body: JSON.stringify({
        announcement: input.announcement,
        display_name: input.displayName,
        merchant_shop_id: input.merchantShopId,
        slug: input.slug,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

async function createCardProductInControlPlane(input: {
  cardProductId: string;
  merchantShopId: string;
  title: string;
  description: string;
  inventoryCount: number;
  faceValueUsd: string;
  retailPriceUsd: string;
  supportsTrial: boolean;
}) {
  return requestControlPlaneJson(
    "/v1/merchant/card-products",
    (payload) => toCardProductView(cardProductSchema.parse(payload)),
    {
      body: JSON.stringify({
        card_product_id: input.cardProductId,
        description: input.description,
        face_value_usd: input.faceValueUsd,
        inventory_count: input.inventoryCount,
        merchant_shop_id: input.merchantShopId,
        retail_price_usd: input.retailPriceUsd,
        supports_trial: input.supportsTrial,
        title: input.title,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

async function createTrialConnectionInControlPlane(input: {
  trialConnectionId: string;
  providerLabel: string;
  endpointBaseUrl: string;
  apiKey: string;
  targetModel: string;
  notes?: string;
}) {
  return requestControlPlaneJson(
    "/v1/merchant/trial-connections",
    (payload) => toTrialConnectionView(trialConnectionSchema.parse(payload)),
    {
      body: JSON.stringify({
        api_key: input.apiKey,
        endpoint_base_url: input.endpointBaseUrl,
        notes: input.notes,
        provider_label: input.providerLabel,
        target_model: input.targetModel,
        trial_connection_id: input.trialConnectionId,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

async function createRelayEvaluationInControlPlane(input: {
  trialConnectionId: string;
}) {
  return requestControlPlaneJson(
    "/v1/merchant/evaluations",
    (payload) => toRelayEvaluationView(relayEvaluationSchema.parse(payload)),
    {
      body: JSON.stringify({
        trial_connection_id: input.trialConnectionId,
      }),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      method: "POST",
    },
  );
}

async function getReplayCapsuleFromControlPlane(replayCapsuleId: string) {
  return requestControlPlaneJson(
    `/v1/replay-capsules/${encodeURIComponent(replayCapsuleId)}`,
    parseReplayCapsule,
    {
      headers: {
        Accept: "application/json",
      },
      method: "GET",
    },
  );
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

async function getRouteDiagnosticsFromControlPlane(routePolicyId: string) {
  try {
    return await client.getRouteDiagnostics(routePolicyId);
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

async function downloadBillingExportFromControlPlane(exportJobId: string) {
  return client.downloadBillingExport(exportJobId);
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

function currentTimestamp() {
  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
}

function requireActiveTenantId() {
  const tenantId = getActiveTenantId();

  if (!tenantId) {
    throw new Error("tenant_not_found");
  }

  return tenantId;
}

function toProviderResourcePayload(
  input: ProviderResourceMutationInput,
): ProviderResource {
  return {
    auth_kind: input.authKind,
    budget_policy_id: input.budgetPolicyId,
    capabilities: {
      supports_json_mode: input.capabilities.supportsJsonMode,
      supports_realtime: false,
      supports_response_model_metadata: true,
      supports_streaming: input.capabilities.supportsStreaming,
      supports_tool_calling: input.capabilities.supportsToolCalling,
    },
    created_at: input.createdAt ?? currentTimestamp(),
    credential_owner_type: input.credentialOwnerType,
    deployment_scope: input.deploymentScope,
    endpoint_base_url: input.endpointBaseUrl,
    health_state: input.healthState,
    is_transit_gateway: false,
    name: input.name,
    project_id: input.projectId,
    provider_id: input.providerId,
    provider_resource_id: input.providerResourceId,
    provenance_class: input.provenanceClass,
    region: input.region,
    status: input.status,
    supported_protocol_families: ["openai_chat"],
    tenant_id: requireActiveTenantId(),
    updated_at: currentTimestamp(),
    version: input.version ?? 1,
  };
}

function toRoutePolicyPayload(input: RoutePolicyMutationInput): RoutePolicy {
  return {
    created_at: input.createdAt ?? currentTimestamp(),
    display_name: input.displayName,
    model_alias: input.modelAlias,
    preferred_regions: input.preferredRegions,
    protocol_family: input.protocolFamily,
    required_capabilities: input.requiredCapabilities,
    route_policy_id: input.routePolicyId,
    tenant_id: requireActiveTenantId(),
    updated_at: input.updatedAt ?? currentTimestamp(),
    version: input.version ?? 1,
  };
}

function toConfigSnapshotPayload(
  input: ConfigSnapshotMutationInput,
): ConfigSnapshot {
  return {
    activated_at: input.activatedAt,
    budget_policy_id: input.budgetPolicyId,
    config_snapshot_id: input.configSnapshotId,
    project_id: input.projectId,
    provider_resource_ids: input.providerResourceIds,
    revision: input.revision,
    route_policy_id: input.routePolicyId,
    status: input.status,
    tenant_id: requireActiveTenantId(),
  };
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

  async listProjects() {
    const tenantId = getActiveTenantId();
    const projects = await client.listProjects();
    const filteredProjects =
      tenantId && !isPlatformAdmin()
        ? projects.filter((project) => project.tenant_id === tenantId)
        : projects;

    return filteredProjects.map(toProjectSummary);
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

  async getRouteDiagnostics(routePolicyId) {
    const diagnostics =
      await getRouteDiagnosticsFromControlPlane(routePolicyId);

    if (!diagnostics) {
      throw new Error("route_policy_not_found");
    }

    return {
      activeSnapshotId: diagnostics.active_snapshot?.config_snapshot_id,
      activeSnapshotMatchesRoutePolicy:
        diagnostics.active_snapshot_matches_route_policy,
      diagnostics,
    };
  },

  async getMerchantWorkspace() {
    return getMerchantWorkspaceFromControlPlane();
  },

  async getReplayCapsule(replayCapsuleId) {
    return getReplayCapsuleFromControlPlane(replayCapsuleId);
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
    const tenantRouteReceipts = filterByTenant(
      await listRouteReceiptsFromControlPlane(),
    ).filter((receipt) => receipt.tenant_id === tenantId);
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
        tenantRouteReceipts,
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
    const [providerResources, routePolicies, activeSnapshot, routeReceipts] =
      await Promise.all([
        client.listProviderResources(),
        client.listRoutePolicies(),
        getActiveSnapshotOrNull(client),
        listRouteReceiptsFromControlPlane(),
      ]);
    const filteredProviders = filterByTenant(providerResources);
    const filteredPolicies = filterByTenant(routePolicies);
    const filteredReceipts = filterByTenant(routeReceipts);

    return mapRoutePolicies(
      filteredPolicies,
      filteredProviders,
      activeSnapshot,
      filteredReceipts,
    );
  },

  async listRouteReceipts() {
    const [routeReceipts, providerResources, routePolicies] = await Promise.all(
      [
        listRouteReceiptsFromControlPlane(),
        client.listProviderResources(),
        client.listRoutePolicies(),
      ],
    );
    const filteredReceipts = filterByTenant(routeReceipts).slice(0, 20);

    return mapRouteReceipts(
      filteredReceipts,
      filterByTenant(providerResources),
      filterByTenant(routePolicies),
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

  async createConfigSnapshot(snapshot) {
    return createConfigSnapshotInControlPlane(
      toConfigSnapshotPayload(snapshot),
    );
  },

  async activateConfigSnapshot(configSnapshotId) {
    const activated =
      await activateConfigSnapshotFromControlPlane(configSnapshotId);

    return activated;
  },

  async createProviderResource(providerResource) {
    return createProviderResourceInControlPlane(
      toProviderResourcePayload(providerResource),
    );
  },

  async updateProviderResource(
    providerResourceId,
    providerResource,
    expectedVersion,
  ) {
    return updateProviderResourceInControlPlane(
      providerResourceId,
      toProviderResourcePayload(providerResource),
      expectedVersion,
    );
  },

  async disableProviderResource(providerResourceId, expectedVersion) {
    return disableProviderResourceInControlPlane(
      providerResourceId,
      expectedVersion,
    );
  },

  async listCodexAuthAccounts() {
    return listCodexAuthAccountsFromControlPlane();
  },

  async uploadCodexAuthAccount(input) {
    return uploadCodexAuthAccountInControlPlane(input);
  },

  async listOAuthSharingLeases() {
    return listOAuthSharingLeasesFromControlPlane();
  },

  async upsertOAuthSharingLease(input) {
    return upsertOAuthSharingLeaseInControlPlane(input);
  },

  async revokeOAuthSharingLease(leaseId) {
    return revokeOAuthSharingLeaseInControlPlane(leaseId);
  },

  async listOAuthCarpools() {
    return listOAuthCarpoolsFromControlPlane();
  },

  async upsertOAuthCarpool(input) {
    return upsertOAuthCarpoolInControlPlane(input);
  },

  async removeOAuthCarpool(carpoolId) {
    return removeOAuthCarpoolInControlPlane(carpoolId);
  },

  async readOAuthSharingUsage(workspaceId) {
    return readOAuthSharingUsageFromControlPlane(workspaceId);
  },

  async createRoutePolicy(routePolicy) {
    return createRoutePolicyInControlPlane(toRoutePolicyPayload(routePolicy));
  },

  async updateRoutePolicy(routePolicyId, routePolicy, expectedVersion) {
    return updateRoutePolicyInControlPlane(
      routePolicyId,
      toRoutePolicyPayload(routePolicy),
      expectedVersion,
    );
  },

  async disableRoutePolicy(routePolicyId, expectedVersion) {
    return disableRoutePolicyInControlPlane(routePolicyId, expectedVersion);
  },

  async listApiKeys() {
    return listTenantApiKeysFromControlPlane();
  },

  async createApiKey(input) {
    const created = await createApiKeyInControlPlane(input);

    return {
      apiKeyId: created.apiKeyId,
      displayName: created.displayName,
      keyPrefix: created.keyPrefix,
      providerResourceId: created.providerResourceId,
      version: created.version,
    };
  },

  async revokeApiKey(apiKeyId, version) {
    await revokeApiKeyFromControlPlane(apiKeyId, version);
  },

  async createMerchantShop(input) {
    return createMerchantShopInControlPlane(input);
  },

  async createCardProduct(input) {
    return createCardProductInControlPlane(input);
  },

  async createTrialConnection(input) {
    return createTrialConnectionInControlPlane(input);
  },

  async runRelayEvaluation(input) {
    return createRelayEvaluationInControlPlane(input);
  },

  async downloadBillingExport(exportJobId) {
    return downloadBillingExportFromControlPlane(exportJobId);
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
