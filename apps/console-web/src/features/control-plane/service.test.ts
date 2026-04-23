import { beforeEach, describe, expect, it } from 'vitest'
import { resetSessionForTests, signIn } from '../auth/session'
import { useControlPlaneFetchMock } from '../../test/control-plane-fetch'
import { loadRouteData } from './loaders'
import {
  getConsoleDataService,
  setConsoleDataServiceForTests,
  type ConsoleDataService
} from './service'

describe('console data service', () => {
  useControlPlaneFetchMock()

  beforeEach(() => {
    resetSessionForTests()
    setConsoleDataServiceForTests(null)
  })

  it('loads overview data for the current tenant session', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const overview = await getConsoleDataService().getOverview()

    expect(overview.tenantLabel).toBe('Acme Retail')
    expect(overview.workspace).toBe('acme-retail')
    expect(overview.activeProviders).toBe(3)
    expect(overview.activeRoutes).toBe(3)
    expect(overview.activeSnapshotId).toBe('cfgsnap_gateway_v1')
    expect(overview.selectedProvider).toBe('OpenAI Primary')
    expect(overview.projects).toHaveLength(3)
    expect(overview.projects[0]).toEqual({
      id: 'proj_core',
      name: 'Core Gateway',
      slug: 'core-gateway'
    })
  })

  it('falls back to the active snapshot tenant when the session is anonymous', async () => {
    const overview = await getConsoleDataService().getOverview()

    expect(overview.tenantLabel).toBe('Acme Retail')
    expect(overview.workspace).toBe('acme-retail')
  })

  it('filters route policies to the authenticated tenant', async () => {
    signIn({
      email: 'tenant@northstar.dev',
      workspace: 'northstar-labs'
    })

    const policies = await getConsoleDataService().listRoutePolicies()

    expect(policies).toHaveLength(1)
    expect(policies[0]?.id).toBe('routepol_northstar_research')
    expect(policies[0]?.name).toBe('Northstar Research')
  })

  it('returns all route policies for admin sessions', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    const policies = await getConsoleDataService().listRoutePolicies()

    expect(policies).toHaveLength(4)
  })

  it('loads tenant detail with tenant-specific providers and policies', async () => {
    const detail = await getConsoleDataService().getTenantDetail('tenant_acme')

    expect(detail.displayName).toBe('Acme Retail')
    expect(detail.activeConfigSnapshotId).toBe('cfgsnap_gateway_v1')
    expect(detail.providers.map((provider) => provider.provider_resource_id)).toEqual([
      'prvrsrc_openai_primary',
      'prvrsrc_openai_backup',
      'prvrsrc_transit_relay'
    ])
    expect(detail.routePolicies).toHaveLength(3)
  })

  it('loads operator-facing provider diagnostics with recent route signals', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const providers = await getConsoleDataService().listProviderResources()

    expect(providers[0]?.capabilities.streaming).toBe(true)
    expect(providers[0]?.protocolFamilies).toContain('openai_chat')
    expect(providers.find((provider) => provider.id === 'prvrsrc_transit_relay')?.isTransitGateway)
      .toBe(true)
    expect(providers[0]?.latestRoutingSignal?.routeName).toBe('Acme Realtime Agent')
  })

  it('loads route diagnostics drill-down data', async () => {
    const diagnostics = await getConsoleDataService().getRouteDiagnostics('routepol_acme_realtime')

    expect(diagnostics.activeSnapshotMatchesRoutePolicy).toBe(false)
    expect(diagnostics.diagnostics.targets).toHaveLength(3)
    expect(diagnostics.diagnostics.last_route_receipt?.failure_reason).toContain('realtime')
  })

  it('loads recent route receipts with failure reasons', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const receipts = await getConsoleDataService().listRouteReceipts()

    expect(receipts[0]?.receiptId).toBe('routercpt_acme_realtime')
    expect(receipts[0]?.failureReason).toContain('realtime_webrtc')
    expect(receipts[1]?.selectedTargetName).toBe('OpenAI Primary')
  })

  it('throws when tenant detail is requested for an unknown tenant', async () => {
    await expect(getConsoleDataService().getTenantDetail('tenant_missing')).rejects.toThrow(
      'tenant_not_found'
    )
  })

  it('allows tests to replace the service implementation', async () => {
    const override: ConsoleDataService = {
      getOverview() {
        return Promise.resolve({
          activeProviders: 1,
          activeRoutes: 1,
          activeSnapshotId: 'cfgsnap_override',
          estimatedCostUsd: '0.000001',
          projects: [],
          selectedProvider: 'Override Provider',
          tenantLabel: 'Override Tenant',
          workspace: 'override-workspace'
        })
      },
      getTenantDetail() {
        return Promise.reject(new Error('unused'))
      },
      getRouteDiagnostics() {
        return Promise.reject(new Error('unused'))
      },
      listProviderResources() {
        return Promise.resolve([])
      },
      listRoutePolicies() {
        return Promise.resolve([])
      },
      listRouteReceipts() {
        return Promise.resolve([])
      },
      listTenants() {
        return Promise.resolve([])
      }
    }

    setConsoleDataServiceForTests(override)

    expect(await getConsoleDataService().getOverview()).toEqual({
      activeProviders: 1,
      activeRoutes: 1,
      activeSnapshotId: 'cfgsnap_override',
      estimatedCostUsd: '0.000001',
      projects: [],
      selectedProvider: 'Override Provider',
      tenantLabel: 'Override Tenant',
      workspace: 'override-workspace'
    })
  })
})

describe('loadRouteData', () => {
  it('returns success state when the loader resolves', async () => {
    await expect(loadRouteData(() => Promise.resolve('ok'))).resolves.toEqual({
      data: 'ok',
      state: 'success'
    })
  })

  it('returns error state when the loader throws', async () => {
    await expect(
      loadRouteData(() => Promise.reject(new Error('failed')))
    ).resolves.toEqual({
      state: 'error'
    })
  })
})
