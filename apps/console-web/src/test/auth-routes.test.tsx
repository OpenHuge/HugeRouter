import { fireEvent, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { type ConsoleAuthClient } from '../features/auth/auth-client'
import { getDefaultProviderAvailability } from '../features/auth/auth-contract'
import { setAuthClientForTests } from '../features/auth/auth-queries'
import { resetSessionForTests, signIn } from '../features/auth/session'
import { renderRoute } from './router-test-utils'

function createAuthClientStub(
  overrides: Partial<ConsoleAuthClient> = {}
): ConsoleAuthClient {
  return {
    completeAuthCallback: () => Promise.resolve({
      data: {
        message: 'GitHub sign-in completed.',
        outcome: 'authenticated' as const,
        provider: 'github',
        state: {
          availableProviders: getDefaultProviderAvailability(),
          kind: 'authenticated' as const,
          session: {
            activeTenant: null,
            expiresAt: '2099-01-01T00:00:00.000Z',
            memberships: [],
            sessionId: 'sess_platform_admin',
            user: {
              displayName: 'Platform Admin',
              email: 'admin@huge-router.dev',
              id: 'user_admin',
              isPlatformAdmin: true
            }
          }
        }
      },
      meta: {}
    }),
    completeEmailLogin: () => Promise.resolve({
      data: {
        message: 'Email verification completed.',
        outcome: 'authenticated' as const,
        state: {
          availableProviders: getDefaultProviderAvailability(),
          kind: 'authenticated' as const,
          session: {
            activeTenant: null,
            expiresAt: '2099-01-01T00:00:00.000Z',
            memberships: [],
            sessionId: 'sess_platform_admin',
            user: {
              displayName: 'Platform Admin',
              email: 'admin@huge-router.dev',
              id: 'user_admin',
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
        authorizationUrl: 'http://127.0.0.1:3000/login/callback?provider=github&state=opaque&code=oauth-code',
        outcome: 'redirect' as const
      },
      meta: {}
    }),
    ...overrides
  }
}

describe('auth routes', () => {
  beforeEach(() => {
    resetSessionForTests()
    setAuthClientForTests(createAuthClientStub())
    vi.restoreAllMocks()
  })

  it('redirects anonymous access to login and preserves the destination', async () => {
    const { history } = await renderRoute('/app/overview')

    expect(
      await screen.findByRole('heading', {
        name: 'Sign in'
      })
    ).toBeInTheDocument()
    expect(history.location.href).toContain('/login')
    expect(history.location.href).toContain('redirect=%2Fapp%2Foverview')
  })

  it('redirects authenticated users away from /login', async () => {
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

  it('completes callback sign-in and lands on the admin surface', async () => {
    await renderRoute('/login/callback?provider=github&code=oauth-code&state=opaque')

    expect(
      await screen.findByRole('heading', {
        name: 'Tenants'
      })
    ).toBeInTheDocument()
  })

  it('completes WeChat callback sign-in with the backend redirect target', async () => {
    const completeAuthCallback = vi.fn<ConsoleAuthClient['completeAuthCallback']>(() =>
      Promise.resolve({
        data: {
          message: 'WeChat sign-in completed.',
          outcome: 'authenticated' as const,
          provider: 'wechat',
          redirectTo: '/admin/tenants',
          state: {
            availableProviders: getDefaultProviderAvailability(),
            kind: 'authenticated' as const,
            session: {
              activeTenant: null,
              expiresAt: '2099-01-01T00:00:00.000Z',
              memberships: [],
              sessionId: 'sess_platform_admin',
              user: {
                displayName: 'Platform Admin',
                email: 'admin@huge-router.dev',
                id: 'user_admin',
                isPlatformAdmin: true
              }
            }
          }
        },
        meta: {}
      })
    )
    setAuthClientForTests(createAuthClientStub({ completeAuthCallback }))

    await renderRoute('/login/callback?provider=wechat&code=wechat-code&state=opaque')

    await waitFor(() => {
      expect(completeAuthCallback).toHaveBeenCalledWith('wechat', {
        code: 'wechat-code',
        error: undefined,
        errorDescription: undefined,
        redirectTo: undefined,
        state: 'opaque'
      })
    })
    expect(
      await screen.findByRole('heading', {
        name: 'Tenants'
      })
    ).toBeInTheDocument()
  })

  it('shows a provider cancellation state on callback failure', async () => {
    await renderRoute('/login/callback?provider=google&error=access_denied')

    expect(
      await screen.findByRole('heading', {
        name: 'Google sign-in was cancelled'
      })
    ).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Back to login' })).toBeInTheDocument()
  })

  it('signs out from authenticated shells and returns to login', async () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    await renderRoute('/admin/tenants')

    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Sign out'
      })
    )

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
