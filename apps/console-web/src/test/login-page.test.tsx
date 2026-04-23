import { fireEvent, screen, waitFor } from '@testing-library/react'
import { renderWithProviders } from '@huge-router/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { type ConsoleAuthClient } from '../features/auth/auth-client'
import type { ProviderAvailability } from '../features/auth/auth-contract'
import { getDefaultProviderAvailability } from '../features/auth/auth-contract'
import { setAuthClientForTests } from '../features/auth/auth-queries'
import { LoginPage } from '../features/auth/LoginPage'
import { resetSessionForTests } from '../features/auth/session'

const redirectToExternalMock = vi.fn()

vi.mock('../features/auth/browser-navigation', () => ({
  redirectToExternal: (url: string) => {
    redirectToExternalMock(url)
  }
}))

function createAuthClientStub(
  overrides: Partial<ConsoleAuthClient> = {}
) {
  const completeAuthCallback: ConsoleAuthClient['completeAuthCallback'] = vi.fn()
  const completeEmailLoginMock: ConsoleAuthClient['completeEmailLogin'] = vi.fn(() => Promise.resolve({
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
  }))
  const getSession: ConsoleAuthClient['getSession'] = vi.fn(() => Promise.resolve({
    data: {
      state: {
        availableProviders: getDefaultProviderAvailability(),
        kind: 'anonymous' as const
      }
    },
    meta: {}
  }))
  const logout: ConsoleAuthClient['logout'] = vi.fn(() => Promise.resolve({
    data: {
      outcome: 'signed_out' as const
    },
    meta: {}
  }))
  const startEmailLoginMock: ConsoleAuthClient['startEmailLogin'] = vi.fn(() => Promise.resolve({
    data: {
      codeHint: 'Use local bootstrap verification code 111111.',
      email: 'ops@huge-router.dev',
      expiresAt: '2026-04-22T10:00:00Z',
      flowId: 'authflow_123',
      message: 'HugeRouter started an email login flow.',
      outcome: 'email_sent' as const
    },
    meta: {}
  }))
  const startProviderLoginMock: ConsoleAuthClient['startProviderLogin'] = vi.fn(() => Promise.resolve({
    data: {
      authorizationUrl: 'http://127.0.0.1:3000/login/callback?provider=github&state=oauth_state_123&code=mock-github-code',
      outcome: 'redirect' as const
    },
    meta: {}
  }))

  return {
    client: {
      completeAuthCallback,
      completeEmailLogin: completeEmailLoginMock,
      getSession,
      logout,
      startEmailLogin: startEmailLoginMock,
      startProviderLogin: startProviderLoginMock,
      ...overrides
    },
    completeEmailLoginMock,
    startEmailLoginMock,
    startProviderLoginMock
  }
}

function configuredProviders(
  providers: ProviderAvailability[]
) {
  return providers
}

describe('LoginPage', () => {
  beforeEach(() => {
    redirectToExternalMock.mockReset()
    resetSessionForTests()
  })

  it('renders email plus GitHub, Google, and WeChat entry points', () => {
    const { client } = createAuthClientStub()
    setAuthClientForTests(client)

    renderWithProviders(
      <LoginPage
        search={{}}
        sessionEnvelope={{
          state: {
            availableProviders: configuredProviders(getDefaultProviderAvailability()),
            kind: 'anonymous'
          }
        }}
      />
    )

    expect(screen.getByLabelText('Workspace')).toHaveValue('platform-admin')
    expect(screen.getByRole('button', { name: 'Continue with Email' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Continue with GitHub' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Continue with Google' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Continue with WeChat' })).toBeInTheDocument()
  })

  it('starts email login and then completes verification', async () => {
    const { client, completeEmailLoginMock, startEmailLoginMock } = createAuthClientStub()
    setAuthClientForTests(client)

    renderWithProviders(
      <LoginPage
        search={{ redirect: '/admin/tenants' }}
        sessionEnvelope={{
          state: {
            availableProviders: configuredProviders(getDefaultProviderAvailability()),
            kind: 'anonymous'
          }
        }}
      />
    )

    fireEvent.click(screen.getByRole('button', { name: 'Continue with Email' }))

    expect(await screen.findByText('Check your email')).toBeInTheDocument()
    expect(startEmailLoginMock).toHaveBeenCalledWith({
      email: 'ops@huge-router.dev',
      redirectTo: '/admin/tenants',
      workspaceSlug: 'platform-admin'
    })

    fireEvent.change(screen.getByLabelText('Verification code'), {
      target: { value: '111111' }
    })
    fireEvent.click(screen.getByRole('button', { name: 'Complete Email Sign-In' }))

    await waitFor(() => {
      expect(completeEmailLoginMock).toHaveBeenCalledWith({
        code: '111111',
        flowId: 'authflow_123'
      })
    })
  })

  it('starts provider sign-in for GitHub, Google, and WeChat', async () => {
    const { client, startProviderLoginMock } = createAuthClientStub()
    setAuthClientForTests(client)

    renderWithProviders(
      <LoginPage
        search={{ redirect: '/app/overview' }}
        sessionEnvelope={{
          state: {
            availableProviders: configuredProviders(getDefaultProviderAvailability()),
            kind: 'anonymous'
          }
        }}
      />
    )

    for (const [provider, label] of [
      ['github', 'GitHub'],
      ['google', 'Google'],
      ['wechat', 'WeChat']
    ] as const) {
      fireEvent.click(screen.getByRole('button', { name: `Continue with ${label}` }))

      await waitFor(() => {
        expect(startProviderLoginMock).toHaveBeenCalledWith(provider, {
          redirectTo: '/app/overview',
          workspaceSlug: 'platform-admin'
        })
      })
    }

    expect(redirectToExternalMock).toHaveBeenCalledTimes(3)
  })

  it('only renders configured providers and hides unconfigured entry points', () => {
    const { client } = createAuthClientStub()
    setAuthClientForTests(client)

    renderWithProviders(
      <LoginPage
        search={{}}
        sessionEnvelope={{
          state: {
            availableProviders: configuredProviders([
              {
                enabled: true,
                hidden: false,
                provider: 'github'
              }
            ]),
            kind: 'anonymous'
          }
        }}
      />
    )

    expect(screen.getByLabelText('Workspace')).toHaveValue('platform-admin')
    expect(screen.getByRole('button', { name: 'Continue with GitHub' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Continue with Email' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Continue with Google' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Continue with WeChat' })).not.toBeInTheDocument()
  })
})
