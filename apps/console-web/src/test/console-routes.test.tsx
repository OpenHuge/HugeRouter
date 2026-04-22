import { screen } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { type ConsoleAuthClient } from '../features/auth/auth-client'
import { getDefaultProviderAvailability } from '../features/auth/auth-contract'
import { setAuthClientForTests } from '../features/auth/auth-queries'
import { resetSessionForTests, signIn } from '../features/auth/session'
import {
  getConsoleDataService,
  setConsoleDataServiceForTests,
  type ConsoleDataService
} from '../features/control-plane/service'
import { renderRoute, createDeferred } from './router-test-utils'

function createAuthClientStub(
  overrides: Partial<ConsoleAuthClient> = {}
): ConsoleAuthClient {
  return {
    completeAuthCallback: () => {
      throw new Error('not used in route tests')
    },
    completeEmailLogin: () => Promise.resolve({
      data: {
        message: 'Email verification completed.',
        outcome: 'authenticated' as const,
        state: {
          availableProviders: getDefaultProviderAvailability(),
          kind: 'authenticated' as const,
          session: {
            activeTenant: null,
            expiresAt: '2026-04-29T09:30:00Z',
            memberships: [],
            sessionId: 'sess_123',
            user: {
              displayName: 'Operations Admin',
              email: 'ops@huge-router.dev',
              id: 'user_123',
              isPlatformAdmin: true
            }
          }
        }
      },
      meta: {}
    }),
    getSession: () => Promise.resolve({
      data: {
        state: {
          availableProviders: getDefaultProviderAvailability(),
          kind: 'anonymous' as const
        }
      },
      meta: {}
    }),
    logout: () => Promise.resolve({
      data: {
        outcome: 'signed_out' as const
      },
      meta: {}
    }),
    startEmailLogin: () => Promise.resolve({
      data: {
        codeHint: 'Use local bootstrap verification code 111111.',
        email: 'ops@huge-router.dev',
        expiresAt: '2026-04-22T10:00:00Z',
        flowId: 'authflow_123',
        message: 'HugeRouter started an email login flow.',
        outcome: 'email_sent' as const
      },
      meta: {}
    }),
    startProviderLogin: () => Promise.resolve({
      data: {
        authorizationUrl: 'http://127.0.0.1:3000/login/callback?provider=github&state=oauth_state_123&code=mock-github-code',
        outcome: 'redirect' as const
      },
      meta: {}
    }),
    ...overrides
  }
}

describe('console routes', () => {
  beforeEach(() => {
    resetSessionForTests()
    setAuthClientForTests(createAuthClientStub())
    setConsoleDataServiceForTests(null)
  })

  it('redirects anonymous users away from tenant routes', async () => {
    await renderRoute('/app/overview')

    expect(
      await screen.findByRole('heading', {
        name: 'Sign in'
      })
    ).toBeInTheDocument()
  })

  it('redirects authenticated tenant users away from admin routes', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/admin/tenants')

    expect(
      await screen.findByRole('heading', {
        name: 'Overview'
      })
    ).toBeInTheDocument()
  })

  it('redirects authenticated admin users away from tenant routes', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/app/providers')

    expect(
      await screen.findByRole('heading', {
        name: 'Tenants'
      })
    ).toBeInTheDocument()
  })

  it('redirects authenticated sessions away from login', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/login')

    expect(
      await screen.findByRole('heading', {
        name: 'Tenants'
      })
    ).toBeInTheDocument()
  })

  it('renders overview success state for tenant sessions', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/app/overview')

    expect(
      await screen.findByRole('heading', {
        name: 'Overview'
      })
    ).toBeInTheDocument()
    expect(screen.getByText('Active providers')).toBeInTheDocument()
    expect(screen.getByText('Retail Operations Control')).toBeInTheDocument()
  })

  it('renders overview loading state while route data is pending', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const deferred = createDeferred<Awaited<ReturnType<ConsoleDataService['getOverview']>>>()
    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      getOverview: () => deferred.promise
    })

    await renderRoute('/app/overview', {
      waitForLoad: false
    })

    expect(await screen.findByLabelText('Loading overview')).toBeInTheDocument()

    deferred.resolve({
      activeProviders: 3,
      activeRoutes: 2,
      monthlySpendUsd: 18420,
      projects: [],
      tenantLabel: 'Acme Retail',
      workspace: 'acme-retail'
    })

    expect(
      await screen.findByRole('heading', {
        name: 'Overview'
      })
    ).toBeInTheDocument()
  })

  it('renders overview empty state when no projects are returned', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      async getOverview() {
        const data = await baseService.getOverview()

        return {
          ...data,
          projects: []
        }
      }
    })

    await renderRoute('/app/overview')

    expect(await screen.findByText('No projects')).toBeInTheDocument()
  })

  it('renders overview error state when the loader fails', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      getOverview() {
        return Promise.reject(new Error('network_failed'))
      }
    })

    await renderRoute('/app/overview')

    expect(await screen.findByText('Overview unavailable')).toBeInTheDocument()
  })

  it('renders providers success state for tenant sessions', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/app/providers')

    expect(
      await screen.findByRole('heading', {
        name: 'Providers'
      })
    ).toBeInTheDocument()
    expect(screen.getByText('OpenAI US East Primary')).toBeInTheDocument()
    expect(screen.getByText('Anthropic US West Burst')).toBeInTheDocument()
  })

  it('renders providers empty state', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listProviderResources() {
        return Promise.resolve([])
      }
    })

    await renderRoute('/app/providers')

    expect(await screen.findByText('No providers')).toBeInTheDocument()
  })

  it('renders providers error state', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listProviderResources() {
        return Promise.reject(new Error('provider_failure'))
      }
    })

    await renderRoute('/app/providers')

    expect(await screen.findByText('Providers unavailable')).toBeInTheDocument()
  })

  it('renders routes success state for tenant sessions', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/app/routes')

    expect(
      await screen.findByRole('heading', {
        name: 'Routes'
      })
    ).toBeInTheDocument()
    expect(screen.getByText('Acme Interactive Chat')).toBeInTheDocument()
    expect(screen.getByText('reasoning-fast')).toBeInTheDocument()
  })

  it('renders routes empty state', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listRoutePolicies() {
        return Promise.resolve([])
      }
    })

    await renderRoute('/app/routes')

    expect(await screen.findByText('No route policies')).toBeInTheDocument()
  })

  it('renders routes error state', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listRoutePolicies() {
        return Promise.reject(new Error('route_failure'))
      }
    })

    await renderRoute('/app/routes')

    expect(await screen.findByText('Routes unavailable')).toBeInTheDocument()
  })

  it('renders tenant inventory empty state', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listTenants() {
        return Promise.resolve([])
      }
    })

    await renderRoute('/admin/tenants')

    expect(await screen.findByText('No tenants')).toBeInTheDocument()
  })

  it('renders tenant inventory error state', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listTenants() {
        return Promise.reject(new Error('tenant_failure'))
      }
    })

    await renderRoute('/admin/tenants')

    expect(await screen.findByText('Tenants unavailable')).toBeInTheDocument()
  })

  it('links tenant inventory rows to the tenant detail route', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/admin/tenants')

    expect(
      await screen.findByRole('link', {
        name: 'Acme Retail'
      })
    ).toHaveAttribute('href', '/admin/tenants/tenant_acme')
  })

  it('renders tenant detail for a selected tenant', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/admin/tenants/tenant_acme')

    expect(
      await screen.findByRole('heading', {
        name: 'Acme Retail'
      })
    ).toBeInTheDocument()
    expect(screen.getByText('Workspace notes')).toBeInTheDocument()
    expect(screen.getByText('Acme Interactive Chat')).toBeInTheDocument()
  })

  it('renders tenant detail error state for unknown tenants', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/admin/tenants/tenant_missing')

    expect(await screen.findByText('Tenant unavailable')).toBeInTheDocument()
  })
})
