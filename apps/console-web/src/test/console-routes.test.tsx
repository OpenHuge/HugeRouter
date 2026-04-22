import { fireEvent, screen, waitFor, within } from '@testing-library/react'
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
import { useControlPlaneFetchMock } from './control-plane-fetch'
import { createDeferred, renderRoute } from './router-test-utils'

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

function createRequestError(status: number, code: string, message: string) {
  const error = new Error(message)

  ;(error as unknown as { code: string }).code = code
  ;(error as unknown as { status: number }).status = status

  return error
}

describe('console routes', () => {
  useControlPlaneFetchMock()

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
    expect(screen.getByText(/Selected provider: OpenAI Primary/)).toBeInTheDocument()
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
      activeProviders: 2,
      activeRoutes: 2,
      activeSnapshotId: 'cfgsnap_gateway_v1',
      estimatedCostUsd: '0.000210',
      projects: [],
      selectedProvider: 'OpenAI Primary',
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

  it('renders session-expired state when CP session is denied', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      getOverview: () => Promise.reject(createRequestError(401, 'session_expired', 'Session invalid'))
    })

    await renderRoute('/app/overview')

    expect(await screen.findByRole('heading', { name: 'Overview' })).toBeInTheDocument()
    expect(
      screen.getByText('The console session is no longer valid. Sign in to continue.')
    ).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Sign in again' })).toHaveAttribute('href', '/login?reason=session-expired')
  })

  it('renders access-denied state when CP returns a permission error', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      getOverview: () =>
        Promise.reject(createRequestError(403, 'tenant_access_denied', 'Tenant denied'))
    })

    await renderRoute('/app/overview')

    expect(await screen.findByText('The active account does not have access to this control-plane resource.')).toBeInTheDocument()
  })

  it('renders snapshots success state for tenant sessions', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/app/snapshots')

    expect(await screen.findByRole('heading', { name: 'Snapshots' })).toBeInTheDocument()
    expect(screen.getByText('cfgsnap_gateway_v2')).toBeInTheDocument()
  })

  it('renders snapshots loading state while data is pending', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const deferred = createDeferred<Awaited<ReturnType<ConsoleDataService['listConfigSnapshots']>>>()
    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listConfigSnapshots: () => deferred.promise
    })

    await renderRoute('/app/snapshots', { waitForLoad: false })

    expect(await screen.findByLabelText('Loading snapshots')).toBeInTheDocument()

    deferred.resolve([
      {
        budgetPolicyId: 'budgetpol_default',
        configSnapshotId: 'cfgsnap_test',
        providerResourceIds: ['prvrsrc_openai_primary'],
        routePolicyId: 'routepol_openai_chat_default',
        revision: 5,
        projectId: 'proj_core',
        status: 'draft',
        tenantId: 'tenant_acme'
      }
    ])

    expect(await screen.findByText('cfgsnap_test')).toBeInTheDocument()
  })

  it('renders snapshots empty state when none are returned', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listConfigSnapshots: () => Promise.resolve([])
    })

    await renderRoute('/app/snapshots')

    expect(await screen.findByText('No snapshots')).toBeInTheDocument()
  })

  it('renders API keys success state for tenant sessions', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/app/api-keys')

    expect(await screen.findByRole('heading', { name: 'API Keys' })).toBeInTheDocument()
    expect(screen.getByText('Acme Primary Key')).toBeInTheDocument()
    expect(screen.getByText('Acme Secondary Key')).toBeInTheDocument()
  })

  it('renders api-keys loading state while data is pending', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const deferred = createDeferred<Awaited<ReturnType<ConsoleDataService['listApiKeys']>>>()
    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listApiKeys: () => deferred.promise
    })

    await renderRoute('/app/api-keys', {
      waitForLoad: false
    })

    expect(await screen.findByLabelText('Loading API keys')).toBeInTheDocument()

    deferred.resolve([
      {
        apiKeyId: 'key_test',
        canRevoke: true,
        createdAt: '2026-04-01T00:00:00Z',
        displayName: 'Test Key',
        isActive: true,
        keyPrefix: 'ak-test',
        providerResourceId: 'prvrsrc_openai_primary',
        tenantId: 'tenant_acme',
        updatedAt: '2026-04-01T00:00:00Z',
        version: 1
      }
    ])

    expect(await screen.findByText('Test Key')).toBeInTheDocument()
  })

  it('renders api-keys empty state when no keys are returned', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listApiKeys: () => Promise.resolve([])
    })

    await renderRoute('/app/api-keys')

    expect(await screen.findByText('No API keys')).toBeInTheDocument()
  })

  it('shows revoked status after API key revocation action', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/app/api-keys')

    const primaryRow = screen.getByRole('row', { name: /Acme Primary Key/ })
    const primaryButton = within(primaryRow).getByRole('button', { name: 'Revoke' })

    fireEvent.click(primaryButton)

    await waitFor(() => {
      const row = screen.getByRole('row', { name: /Acme Primary Key/ })
      expect(within(row).getByText('Revoked')).toBeInTheDocument()
      expect(within(row).getByRole('button', { name: 'Revoke' })).toBeDisabled()
    })
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
    expect(screen.getByText('OpenAI Primary')).toBeInTheDocument()
    expect(screen.getByText('OpenAI Backup')).toBeInTheDocument()
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
    expect(screen.getByText('Acme Reasoning Fast')).toBeInTheDocument()
    expect(screen.getByText('OpenAI Primary, OpenAI Backup')).toBeInTheDocument()
  })

  it('renders protocol-aware route policy groups and route receipts diagnostics', async () => {
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
    expect(screen.getAllByText('OpenAI Chat').length).toBeGreaterThan(0)
    expect(screen.getByText('Recent route receipts')).toBeInTheDocument()
    expect(screen.getByText('routercpt_openai_primary_recent')).toBeInTheDocument()
    expect(screen.getByText('OpenAI Primary')).toBeInTheDocument()
    expect(screen.getByText('OpenAI Backup (provider_region_mismatch)')).toBeInTheDocument()
    expect(screen.getByText('OpenAI Backup → OpenAI Primary (replayed_after_transient_timeout)')).toBeInTheDocument()
  })

  it('renders route diagnostics empty state when no route receipts are available', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const baseService = getConsoleDataService()

    setConsoleDataServiceForTests({
      ...baseService,
      listRouteReceipts: () => Promise.resolve([])
    })

    await renderRoute('/app/routes')

    expect(await screen.findByText('No route receipts')).toBeInTheDocument()
  })

  it('renders tenant inventory and links to tenant detail', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/admin/tenants')

    expect(
      await screen.findByRole('heading', {
        name: 'Tenants'
      })
    ).toBeInTheDocument()
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
    expect(screen.getByText('Provider resources')).toBeInTheDocument()
    expect(screen.getByText('Acme Reasoning Fast')).toBeInTheDocument()
  })

  it('renders tenant detail error state for unknown tenants', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/admin/tenants/tenant_missing')

    expect(await screen.findByText('Tenant unavailable')).toBeInTheDocument()
  })

  it('signs out from app shell and returns to login screen', async () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    await renderRoute('/app/overview')

    fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }))

    await waitFor(() => {
      expect(
        screen.getByRole('heading', {
          name: 'Sign in'
        })
      ).toBeInTheDocument()
    })
    expect(screen.getByText('Signed out')).toBeInTheDocument()
  })
})
