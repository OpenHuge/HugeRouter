import {
  createControlPlaneClient,
  type ControlPlaneClient
} from '@huge-router/ts-api-client'
import type {
  ConfigSnapshot,
  Project,
  ProviderResource,
  RoutePolicy,
  RouteSimulationResponse,
  Tenant
} from '@huge-router/ts-shared-schema'
import type { AuthSessionEnvelope } from '../auth/auth-contract'
import { authSessionQueryKey } from '../auth/auth-queries'
import { getQueryClient } from '../../lib/query-client'
import type {
  OverviewData,
  ProjectSummary,
  RoutePolicyView,
  TenantDetail,
  TenantSummary
} from './types'

export type ConsoleDataService = {
  getOverview: () => Promise<OverviewData>
  getTenantDetail: (tenantId: string) => Promise<TenantDetail>
  listProviderResources: () => Promise<ProviderResource[]>
  listRoutePolicies: () => Promise<RoutePolicyView[]>
  listTenants: () => Promise<TenantSummary[]>
}

const CONTROL_PLANE_BASE_URL = import.meta.env.VITE_CONTROL_PLANE_BASE_URL
  ? String(import.meta.env.VITE_CONTROL_PLANE_BASE_URL)
  : ''

const client = createControlPlaneClient({
  baseUrl: CONTROL_PLANE_BASE_URL,
  fetch: (input, init) => globalThis.fetch(input, init)
})

function getAuthEnvelope() {
  return getQueryClient().getQueryData<AuthSessionEnvelope>(authSessionQueryKey)
}

function getActiveTenantId() {
  const envelope = getAuthEnvelope()

  if (envelope?.state.kind !== 'authenticated') {
    return null
  }

  return envelope.state.session.activeTenant?.tenantId ?? null
}

function isPlatformAdmin() {
  const envelope = getAuthEnvelope()

  return envelope?.state.kind === 'authenticated'
    ? envelope.state.session.user.isPlatformAdmin
    : false
}

function filterByTenant<T extends { tenant_id: string }>(items: T[]) {
  const tenantId = getActiveTenantId()

  if (!tenantId || isPlatformAdmin()) {
    return items
  }

  return items.filter((item) => item.tenant_id === tenantId)
}

function toProjectSummary(project: Project): ProjectSummary {
  return {
    id: project.project_id,
    name: project.display_name,
    slug: project.slug
  }
}

function mapRoutePolicies(
  routePolicies: RoutePolicy[],
  providerResources: ProviderResource[],
  activeSnapshot: ConfigSnapshot | null
): RoutePolicyView[] {
  const providerNames = new Map(
    providerResources.map((provider) => [provider.provider_resource_id, provider.name])
  )

  return routePolicies.map((policy) => ({
    id: policy.route_policy_id,
    modelAlias: policy.model_alias,
    name: policy.display_name,
    preferredRegions: policy.preferred_regions,
    protocolFamily: policy.protocol_family,
    requiredCapabilities: policy.required_capabilities,
    selectedProviders:
      activeSnapshot?.route_policy_id === policy.route_policy_id
        ? activeSnapshot.provider_resource_ids
            .map((providerId) => providerNames.get(providerId) ?? providerId)
        : []
  }))
}

async function getActiveSnapshotOrNull(apiClient: ControlPlaneClient) {
  try {
    return await apiClient.getConfigSnapshot('active')
  } catch {
    return null
  }
}

async function getRouteSimulationOrNull(
  apiClient: ControlPlaneClient,
  snapshot: ConfigSnapshot | null,
  routePolicies: RoutePolicy[],
  providerResources: ProviderResource[]
) {
  if (!snapshot) {
    return null
  }

  const routePolicy = routePolicies.find(
    (policy) => policy.route_policy_id === snapshot.route_policy_id
  )

  if (!routePolicy) {
    return null
  }

  const region =
    providerResources.find(
      (provider) => provider.provider_resource_id === snapshot.provider_resource_ids[0]
    )?.region ?? routePolicy.preferred_regions[0] ?? 'us-east-1'

  try {
    return await apiClient.simulateRoute({
      credential_scope: 'cred_console',
      expected_max_output_tokens: 256,
      expected_prompt_tokens: 64,
      model_alias: routePolicy.model_alias,
      project_id: snapshot.project_id,
      protocol_family: routePolicy.protocol_family,
      region,
      required_capabilities: routePolicy.required_capabilities,
      tenant_id: snapshot.tenant_id,
      traffic_class: 'console_preview'
    })
  } catch {
    return null
  }
}

function selectedProviderLabel(
  simulation: RouteSimulationResponse | null,
  providerResources: ProviderResource[]
) {
  if (!simulation?.selected_target) {
    return 'No eligible provider'
  }

  return (
    providerResources.find(
      (provider) => provider.provider_resource_id === simulation.selected_target
    )?.name ?? simulation.selected_target
  )
}

async function loadControlPlaneData() {
  const [tenants, projects, providerResources, routePolicies, activeSnapshot] = await Promise.all([
    client.listTenants(),
    client.listProjects(),
    client.listProviderResources(),
    client.listRoutePolicies(),
    getActiveSnapshotOrNull(client)
  ])

  return {
    activeSnapshot,
    projects,
    providerResources,
    routePolicies,
    tenants
  }
}

const defaultConsoleDataService: ConsoleDataService = {
  async getOverview() {
    const { activeSnapshot, projects, providerResources, routePolicies, tenants } =
      await loadControlPlaneData()
    const tenantId = getActiveTenantId() ?? activeSnapshot?.tenant_id ?? tenants[0]?.tenant_id
    const tenant = tenants.find((candidate) => candidate.tenant_id === tenantId) ?? tenants[0]

    if (!tenant) {
      throw new Error('tenant_not_found')
    }

    const tenantProjects = projects
      .filter((project) => project.tenant_id === tenant.tenant_id)
      .map(toProjectSummary)
    const tenantProviders = providerResources.filter(
      (provider) => provider.tenant_id === tenant.tenant_id
    )
    const tenantRoutePolicies = routePolicies.filter(
      (policy) => policy.tenant_id === tenant.tenant_id
    )
    const simulation = await getRouteSimulationOrNull(
      client,
      activeSnapshot?.tenant_id === tenant.tenant_id ? activeSnapshot : null,
      tenantRoutePolicies,
      tenantProviders
    )

    return {
      activeProviders: tenantProviders.filter((provider) => provider.status === 'active').length,
      activeRoutes: tenantRoutePolicies.length,
      activeSnapshotId:
        activeSnapshot?.tenant_id === tenant.tenant_id
          ? activeSnapshot.config_snapshot_id
          : 'No active snapshot',
      estimatedCostUsd: simulation?.estimated_cost.amount ?? '0.000000',
      projects: tenantProjects,
      selectedProvider: selectedProviderLabel(simulation, tenantProviders),
      tenantLabel: tenant.display_name,
      workspace: tenant.slug
    }
  },

  async getTenantDetail(tenantId) {
    const { activeSnapshot, projects, providerResources, routePolicies, tenants } =
      await loadControlPlaneData()
    const tenant = tenants.find((candidate) => candidate.tenant_id === tenantId)

    if (!tenant) {
      throw new Error('tenant_not_found')
    }

    const tenantProjects = projects
      .filter((project) => project.tenant_id === tenantId)
      .map(toProjectSummary)
    const tenantProviders = providerResources.filter(
      (provider) => provider.tenant_id === tenantId
    )
    const tenantRoutePolicies = routePolicies.filter(
      (policy) => policy.tenant_id === tenantId
    )
    const simulation = await getRouteSimulationOrNull(
      client,
      activeSnapshot?.tenant_id === tenantId ? activeSnapshot : null,
      tenantRoutePolicies,
      tenantProviders
    )

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
      routePolicies: mapRoutePolicies(tenantRoutePolicies, tenantProviders, activeSnapshot),
      selectedProvider: selectedProviderLabel(simulation, tenantProviders),
      slug: tenant.slug,
      updatedAt: tenant.updated_at,
      version: tenant.version
    }
  },

  async listProviderResources() {
    const providerResources = await client.listProviderResources()

    return filterByTenant(providerResources)
  },

  async listRoutePolicies() {
    const [providerResources, routePolicies, activeSnapshot] = await Promise.all([
      client.listProviderResources(),
      client.listRoutePolicies(),
      getActiveSnapshotOrNull(client)
    ])
    const filteredProviders = filterByTenant(providerResources)
    const filteredPolicies = filterByTenant(routePolicies)

    return mapRoutePolicies(filteredPolicies, filteredProviders, activeSnapshot)
  },

  async listTenants() {
    const { activeSnapshot, projects, providerResources, routePolicies, tenants } =
      await loadControlPlaneData()

    return tenants.map((tenant) => ({
      activeConfigSnapshotId:
        activeSnapshot?.tenant_id === tenant.tenant_id
          ? activeSnapshot.config_snapshot_id
          : undefined,
      displayName: tenant.display_name,
      id: tenant.tenant_id,
      projectCount: projects.filter((project) => project.tenant_id === tenant.tenant_id).length,
      providerCount: providerResources.filter(
        (provider) => provider.tenant_id === tenant.tenant_id
      ).length,
      routePolicyCount: routePolicies.filter(
        (policy) => policy.tenant_id === tenant.tenant_id
      ).length,
      slug: tenant.slug,
      updatedAt: tenant.updated_at
    }))
  }
}

let consoleDataServiceOverride: ConsoleDataService | null = null

export function getConsoleDataService() {
  return consoleDataServiceOverride ?? defaultConsoleDataService
}

export function setConsoleDataServiceForTests(service: ConsoleDataService | null) {
  consoleDataServiceOverride = service
}
