import { createControlPlaneClient } from '@huge-router/ts-api-client'
import { getSessionSnapshot } from '../auth/session'
import {
  overviewDataSchema,
  providerResourceSchema,
  routePolicySchema,
  tenantDetailSchema,
  tenantSummarySchema,
  type OverviewData,
  type ProviderResource,
  type RoutePolicy,
  type TenantDetail,
  type TenantSummary
} from './types'

export type ConsoleDataService = {
  getOverview: () => Promise<OverviewData>
  getTenantDetail: (tenantId: string) => Promise<TenantDetail>
  listProviderResources: () => Promise<ProviderResource[]>
  listRoutePolicies: () => Promise<RoutePolicy[]>
  listTenants: () => Promise<TenantSummary[]>
}

const client = createControlPlaneClient()

const tenantSummaries = tenantSummarySchema.array().parse([
  {
    activeRoutePolicies: 2,
    displayName: 'Acme Retail',
    id: 'tenant_acme',
    monthlySpendUsd: 18420,
    plan: 'enterprise',
    projectCount: 3,
    slug: 'acme-retail',
    status: 'healthy'
  },
  {
    activeRoutePolicies: 1,
    displayName: 'Northstar Labs',
    id: 'tenant_northstar',
    monthlySpendUsd: 6420,
    plan: 'growth',
    projectCount: 2,
    slug: 'northstar-labs',
    status: 'needs_attention'
  }
])

const providerResources = providerResourceSchema.array().parse([
  {
    health: 'healthy',
    id: 'provider_openai_us_east',
    name: 'OpenAI US East Primary',
    provider: 'OpenAI',
    region: 'us-east-1',
    scope: 'shared',
    status: 'active'
  },
  {
    health: 'warning',
    id: 'provider_anthropic_us_west',
    name: 'Anthropic US West Burst',
    provider: 'Anthropic',
    region: 'us-west-2',
    scope: 'tenant',
    status: 'active'
  },
  {
    health: 'degraded',
    id: 'provider_google_eu',
    name: 'Google EU Reserved',
    provider: 'Google',
    region: 'europe-west4',
    scope: 'shared',
    status: 'quarantined'
  }
])

const routePolicies = routePolicySchema.array().parse([
  {
    id: 'routepol_acme_chat',
    modelAlias: 'reasoning-fast',
    name: 'Acme Interactive Chat',
    selectedProvider: 'OpenAI US East Primary',
    status: 'active',
    successRate: 0.992,
    tenantId: 'tenant_acme'
  },
  {
    id: 'routepol_acme_tools',
    modelAlias: 'tool-call-balanced',
    name: 'Acme Tooling Workflows',
    selectedProvider: 'Anthropic US West Burst',
    status: 'draft',
    successRate: 0.964,
    tenantId: 'tenant_acme'
  },
  {
    id: 'routepol_northstar_support',
    modelAlias: 'support-safe',
    name: 'Northstar Support Agent',
    selectedProvider: 'OpenAI US East Primary',
    status: 'active',
    successRate: 0.978,
    tenantId: 'tenant_northstar'
  }
])

const projectDirectory = {
  tenant_acme: [
    { id: 'proj_acme_ops', name: 'Retail Operations Control' },
    { id: 'proj_acme_routing', name: 'Checkout Routing' },
    { id: 'proj_acme_support', name: 'Store Support Agent' }
  ],
  tenant_northstar: [
    { id: 'proj_ns_qa', name: 'Research QA' },
    { id: 'proj_ns_lab', name: 'Experiment Triage' }
  ]
} as const

function normalizeClientProjectName(projectName: string, fallbackName: string) {
  if (projectName === 'Bootstrap Placeholder') {
    return fallbackName
  }

  return projectName
}

async function getProjectsForTenant(tenantId: keyof typeof projectDirectory) {
  const projects = await client.listProjects()

  return projectDirectory[tenantId].map((project, index) => ({
    id: project.id,
    name: normalizeClientProjectName(
      projects[index]?.display_name ?? project.name,
      project.name
    )
  }))
}

const defaultConsoleDataService: ConsoleDataService = {
  async getOverview() {
    const session = getSessionSnapshot()
    const tenantId =
      session.authState === 'authenticated' && session.tenantId
        ? session.tenantId
        : 'tenant_acme'
    const tenant = tenantSummaries.find((candidate) => candidate.id === tenantId) ?? tenantSummaries[0]
    const projects = await getProjectsForTenant(tenant.id as keyof typeof projectDirectory)
    const activeRoutes = routePolicies.filter((policy) => policy.tenantId === tenant.id).length
    const activeProviders = providerResources.filter((provider) => provider.status === 'active').length

    return overviewDataSchema.parse({
      activeProviders,
      activeRoutes,
      monthlySpendUsd: tenant.monthlySpendUsd,
      projects,
      tenantLabel: tenant.displayName,
      workspace: tenant.slug
    })
  },
  async getTenantDetail(tenantId) {
    const tenant = tenantSummaries.find((candidate) => candidate.id === tenantId)

    if (!tenant) {
      throw new Error('tenant_not_found')
    }

    const projects = await getProjectsForTenant(tenant.id as keyof typeof projectDirectory)
    const tenantProviders =
      tenant.id === 'tenant_acme'
        ? providerResources.filter((provider) => provider.id !== 'provider_google_eu')
        : providerResources.filter((provider) => provider.provider !== 'Anthropic')

    return tenantDetailSchema.parse({
      ...tenant,
      notes:
        tenant.id === 'tenant_acme'
          ? 'Primary checkout and support traffic with aggressive latency SLOs.'
          : 'Research tenant using smaller daily volumes with stricter provider quarantine review.',
      primaryRegion: tenant.id === 'tenant_acme' ? 'us-east-1' : 'us-west-2',
      projects,
      providers: tenantProviders,
      routePolicies: routePolicies.filter((policy) => policy.tenantId === tenant.id)
    })
  },
  listProviderResources() {
    return Promise.resolve(providerResources)
  },
  listRoutePolicies() {
    const session = getSessionSnapshot()

    if (session.authState === 'authenticated' && session.tenantId) {
      return Promise.resolve(routePolicies.filter((policy) => policy.tenantId === session.tenantId))
    }

    return Promise.resolve(routePolicies)
  },
  listTenants() {
    return Promise.resolve(tenantSummaries)
  }
}

let consoleDataServiceOverride: ConsoleDataService | null = null

export function getConsoleDataService() {
  return consoleDataServiceOverride ?? defaultConsoleDataService
}

export function setConsoleDataServiceForTests(service: ConsoleDataService | null) {
  consoleDataServiceOverride = service
}
