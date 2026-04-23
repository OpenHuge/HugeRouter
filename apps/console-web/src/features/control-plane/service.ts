import {
  createControlPlaneClient,
  type ControlPlaneClient,
} from "@huge-router/ts-api-client";
import type {
  ConfigSnapshot,
  Project,
  ProviderResource,
  RouteDiagnosticsResponse,
  RoutePolicy,
  RouteReceipt,
  RouteSimulationResponse,
} from "@huge-router/ts-shared-schema";
import type { AuthSessionEnvelope } from "../auth/auth-contract";
import { authSessionQueryKey } from "../auth/auth-queries";
import { getQueryClient } from "../../lib/query-client";
import type {
  OverviewData,
  ProjectSummary,
  ProviderResourceView,
  RouteDiagnosticsView,
  RoutePolicyView,
  RouteReceiptView,
  TenantDetail,
  TenantSummary,
} from "./types";

export type ConsoleDataService = {
  getOverview: () => Promise<OverviewData>;
  getRouteDiagnostics: (routePolicyId: string) => Promise<RouteDiagnosticsView>;
  getTenantDetail: (tenantId: string) => Promise<TenantDetail>;
  listProviderResources: () => Promise<ProviderResourceView[]>;
  listRoutePolicies: () => Promise<RoutePolicyView[]>;
  listRouteReceipts: () => Promise<RouteReceiptView[]>;
  listTenants: () => Promise<TenantSummary[]>;
};

const CONTROL_PLANE_BASE_URL = import.meta.env.VITE_CONTROL_PLANE_BASE_URL
  ? String(import.meta.env.VITE_CONTROL_PLANE_BASE_URL)
  : "";

const client = createControlPlaneClient({
  baseUrl: CONTROL_PLANE_BASE_URL,
  fetch: (input, init) => globalThis.fetch(input, init),
});

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

function filterByTenant<T extends { tenant_id: string }>(items: T[]) {
  const tenantId = getActiveTenantId();

  if (!tenantId || isPlatformAdmin()) {
    return items;
  }

  return items.filter((item) => item.tenant_id === tenantId);
}

function toProjectSummary(project: Project): ProjectSummary {
  return {
    id: project.project_id,
    name: project.display_name,
    slug: project.slug,
  };
}

function providerNameMap(providerResources: ProviderResource[]) {
  return new Map(
    providerResources.map((provider) => [
      provider.provider_resource_id,
      provider.name,
    ]),
  );
}

function routePolicyNameMap(routePolicies: RoutePolicy[]) {
  return new Map(
    routePolicies.map((policy) => [
      policy.route_policy_id,
      policy.display_name,
    ]),
  );
}

function mapRoutePolicies(
  routePolicies: RoutePolicy[],
  providerResources: ProviderResource[],
  activeSnapshot: ConfigSnapshot | null,
  routeReceipts: RouteReceipt[],
): RoutePolicyView[] {
  const providerNames = providerNameMap(providerResources);

  return routePolicies.map((policy) => {
    const latestReceipt = routeReceipts
      .filter((receipt) => receipt.route_policy_id === policy.route_policy_id)
      .sort((left, right) =>
        right.created_at.localeCompare(left.created_at),
      )[0];

    return {
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
      lastReceiptId: latestReceipt?.route_receipt_id,
      lastReceiptOutcome: latestReceipt?.admission_result,
      lastFailureReason: latestReceipt?.failure_reason,
    };
  });
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

function latestSignalForProvider(
  provider: ProviderResource,
  routeReceipts: RouteReceipt[],
  policyNames: Map<string, string>,
) {
  const recentReceipt = routeReceipts
    .filter(
      (receipt) =>
        receipt.selected_target === provider.provider_resource_id ||
        receipt.excluded_targets.some(
          (target) =>
            target.provider_resource_id === provider.provider_resource_id,
        ),
    )
    .sort((left, right) => right.created_at.localeCompare(left.created_at))[0];

  if (!recentReceipt) {
    return undefined;
  }

  const excludedTarget = recentReceipt.excluded_targets.find(
    (target) => target.provider_resource_id === provider.provider_resource_id,
  );

  return {
    createdAt: recentReceipt.created_at,
    outcome:
      recentReceipt.selected_target === provider.provider_resource_id
        ? "selected"
        : "excluded",
    reason:
      recentReceipt.selected_target === provider.provider_resource_id
        ? "Selected by the latest matching route receipt."
        : (excludedTarget?.reason ??
          recentReceipt.failure_reason ??
          "Excluded by the route receipt."),
    receiptId: recentReceipt.route_receipt_id,
    routeName:
      policyNames.get(recentReceipt.route_policy_id) ??
      recentReceipt.route_policy_id,
  } satisfies ProviderResourceView["latestRoutingSignal"];
}

function mapProviderResources(
  providerResources: ProviderResource[],
  routeReceipts: RouteReceipt[],
  routePolicies: RoutePolicy[],
): ProviderResourceView[] {
  const policyNames = routePolicyNameMap(routePolicies);

  return providerResources.map((provider) => ({
    id: provider.provider_resource_id,
    name: provider.name,
    providerId: provider.provider_id,
    region: provider.region,
    scope: provider.deployment_scope,
    status: provider.status,
    healthState: provider.health_state,
    healthMessage: provider.health_message,
    quarantineReason: provider.quarantine_reason,
    provenanceClass: provider.provenance_class,
    protocolFamilies: provider.supported_protocol_families,
    isTransitGateway: provider.is_transit_gateway,
    capabilities: {
      streaming: provider.capabilities.supports_streaming,
      realtime: provider.capabilities.supports_realtime,
      toolCalling: provider.capabilities.supports_tool_calling,
      responseModelMetadata:
        provider.capabilities.supports_response_model_metadata,
    },
    latestRoutingSignal: latestSignalForProvider(
      provider,
      routeReceipts,
      policyNames,
    ),
  }));
}

function mapRouteReceipts(
  routeReceipts: RouteReceipt[],
  providerResources: ProviderResource[],
  routePolicies: RoutePolicy[],
): RouteReceiptView[] {
  const providerNames = providerNameMap(providerResources);
  const policyNames = routePolicyNameMap(routePolicies);

  return routeReceipts
    .slice()
    .sort((left, right) => right.created_at.localeCompare(left.created_at))
    .map((receipt) => ({
      receiptId: receipt.route_receipt_id,
      routePolicyId: receipt.route_policy_id,
      routeName:
        policyNames.get(receipt.route_policy_id) ?? receipt.route_policy_id,
      modelAlias: receipt.model_alias,
      protocolFamily: receipt.protocol_family,
      createdAt: receipt.created_at,
      admissionResult: receipt.admission_result,
      selectedTargetName: receipt.selected_target
        ? (providerNames.get(receipt.selected_target) ??
          receipt.selected_target)
        : undefined,
      failureReason: receipt.failure_reason,
      excludedTargets: receipt.excluded_targets,
      normalizedError: receipt.normalized_error,
      fallbackTransitions: receipt.fallback_transitions,
      scoreBreakdown: receipt.score_breakdown,
    }));
}

async function loadControlPlaneData() {
  const [
    tenants,
    projects,
    providerResources,
    routePolicies,
    routeReceipts,
    activeSnapshot,
  ] = await Promise.all([
    client.listTenants(),
    client.listProjects(),
    client.listProviderResources(),
    client.listRoutePolicies(),
    client.listRouteReceipts({ limit: 20 }),
    getActiveSnapshotOrNull(client),
  ]);

  return {
    activeSnapshot,
    projects,
    providerResources,
    routePolicies,
    routeReceipts,
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

  async getRouteDiagnostics(routePolicyId) {
    const diagnostics = await client.getRouteDiagnostics(routePolicyId);

    return {
      activeSnapshotId: diagnostics.active_snapshot?.config_snapshot_id,
      activeSnapshotMatchesRoutePolicy:
        diagnostics.active_snapshot_matches_route_policy,
      diagnostics,
    };
  },

  async getTenantDetail(tenantId) {
    const {
      activeSnapshot,
      projects,
      providerResources,
      routePolicies,
      routeReceipts,
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
        routeReceipts.filter((receipt) => receipt.tenant_id === tenantId),
      ),
      selectedProvider: selectedProviderLabel(simulation, tenantProviders),
      slug: tenant.slug,
      updatedAt: tenant.updated_at,
      version: tenant.version,
    };
  },

  async listProviderResources() {
    const [providerResources, routePolicies, routeReceipts] = await Promise.all(
      [
        client.listProviderResources(),
        client.listRoutePolicies(),
        client.listRouteReceipts({ limit: 20 }),
      ],
    );

    return mapProviderResources(
      filterByTenant(providerResources),
      filterByTenant(routeReceipts),
      filterByTenant(routePolicies),
    );
  },

  async listRoutePolicies() {
    const [providerResources, routePolicies, routeReceipts, activeSnapshot] =
      await Promise.all([
        client.listProviderResources(),
        client.listRoutePolicies(),
        client.listRouteReceipts({ limit: 20 }),
        getActiveSnapshotOrNull(client),
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
    const [providerResources, routePolicies, routeReceipts] = await Promise.all(
      [
        client.listProviderResources(),
        client.listRoutePolicies(),
        client.listRouteReceipts({ limit: 20 }),
      ],
    );

    return mapRouteReceipts(
      filterByTenant(routeReceipts),
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
