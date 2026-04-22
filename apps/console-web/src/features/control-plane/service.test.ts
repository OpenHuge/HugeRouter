import { beforeEach, describe, expect, it } from 'vitest'
import { resetSessionForTests, signIn } from '../auth/session'
import { loadRouteData } from './loaders'
import {
  getConsoleDataService,
  setConsoleDataServiceForTests,
  type ConsoleDataService
} from './service'

describe('console data service', () => {
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
    expect(overview.activeProviders).toBe(2)
    expect(overview.activeRoutes).toBe(2)
    expect(overview.projects).toHaveLength(3)
    expect(overview.projects[0]).toEqual({
      id: 'proj_acme_ops',
      name: 'Retail Operations Control'
    })
  })

  it('falls back to the seeded tenant when the session is anonymous', async () => {
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
    expect(policies[0]?.tenantId).toBe('tenant_northstar')
    expect(policies[0]?.name).toBe('Northstar Support Agent')
  })

  it('returns all route policies for admin sessions', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    const policies = await getConsoleDataService().listRoutePolicies()

    expect(policies).toHaveLength(3)
  })

  it('loads tenant detail with tenant-specific providers and policies', async () => {
    const detail = await getConsoleDataService().getTenantDetail('tenant_acme')

    expect(detail.displayName).toBe('Acme Retail')
    expect(detail.primaryRegion).toBe('us-east-1')
    expect(detail.providers.map((provider) => provider.id)).not.toContain('provider_google_eu')
    expect(detail.routePolicies).toHaveLength(2)
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
          monthlySpendUsd: 1,
          projects: [],
          tenantLabel: 'Override Tenant',
          workspace: 'override-workspace'
        })
      },
      getTenantDetail() {
        return Promise.reject(new Error('unused'))
      },
      listProviderResources() {
        return Promise.resolve([])
      },
      listRoutePolicies() {
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
      monthlySpendUsd: 1,
      projects: [],
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
