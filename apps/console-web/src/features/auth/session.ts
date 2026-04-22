import { redirect } from '@tanstack/react-router'
import { getQueryClient, resetQueryClientForTests } from '../../lib/query-client'
import { resetRequestContextForTests } from '../../start'
import { setAuthClientForTests, storeAuthSession } from './auth-queries'
import type { AuthSessionEnvelope } from './auth-contract'

export type AuthRole = 'admin' | 'tenant'

export type AnonymousSession = {
  authState: 'anonymous'
}

export type AuthenticatedSession = {
  authState: 'authenticated'
  email: string
  role: AuthRole
  tenantId: string | null
  workspace: string
}

export type AppSession = AnonymousSession | AuthenticatedSession

export type SignInInput = {
  email: string
  workspace: string
}

const workspaceDirectory = {
  'acme-retail': {
    displayName: 'Acme Retail',
    role: 'tenant' as const,
    tenantId: 'tenant_acme'
  },
  'northstar-labs': {
    displayName: 'Northstar Labs',
    role: 'tenant' as const,
    tenantId: 'tenant_northstar'
  },
  'platform-admin': {
    displayName: 'Platform Admin',
    role: 'admin' as const,
    tenantId: 'tenant_platform'
  }
} as const

function normalizeWorkspace(workspace: string) {
  return workspace.trim().toLowerCase()
}

function createEnvelopeFromSession(session: AuthenticatedSession): AuthSessionEnvelope {
  const tenantName = workspaceDirectory[session.workspace as keyof typeof workspaceDirectory]?.displayName

  return {
    state: {
      availableProviders: [],
      kind: 'authenticated',
      session: {
        activeTenant:
          session.role === 'tenant'
            ? {
                role: 'admin',
                tenantId: session.tenantId ?? 'tenant_unknown',
                tenantName: tenantName ?? session.workspace,
                tenantSlug: session.workspace
              }
            : null,
        expiresAt: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
        memberships: [
          {
            role: 'admin',
            tenantId: session.tenantId ?? 'tenant_unknown',
            tenantName: tenantName ?? session.workspace,
            tenantSlug: session.workspace
          }
        ],
        sessionId: 'sess_seed',
        user: {
          displayName: tenantName ?? 'Seed User',
          email: session.email,
          id: 'user_seed',
          isPlatformAdmin: session.role === 'admin'
        }
      }
    }
  }
}

export function clearSession() {
  storeAuthSession(
    {
      state: {
        availableProviders: [],
        kind: 'anonymous'
      }
    },
    getQueryClient()
  )
}

export function getSessionSnapshot(): AppSession {
  const snapshot = getQueryClient().getQueryData<AuthSessionEnvelope>(['auth', 'session'])

  if (!snapshot || snapshot.state.kind !== 'authenticated') {
    return {
      authState: 'anonymous'
    }
  }

  return {
    authState: 'authenticated',
    email: snapshot.state.session.user.email,
    role: snapshot.state.session.user.isPlatformAdmin ? 'admin' : 'tenant',
    tenantId: snapshot.state.session.activeTenant?.tenantId ?? null,
    workspace:
      snapshot.state.session.activeTenant?.tenantSlug ??
      (snapshot.state.session.user.isPlatformAdmin ? 'platform-admin' : 'tenant')
  }
}

export function signIn({ email, workspace }: SignInInput) {
  const normalizedWorkspace = normalizeWorkspace(workspace)
  const access = workspaceDirectory[normalizedWorkspace as keyof typeof workspaceDirectory]

  if (!access) {
    return {
      ok: false as const,
      message:
        'Use one of the seeded workspaces: platform-admin, acme-retail, or northstar-labs.'
    }
  }

  const session: AuthenticatedSession = {
    authState: 'authenticated',
    email: email.trim().toLowerCase(),
    role: access.role,
    tenantId: access.tenantId,
    workspace: normalizedWorkspace
  }

  storeAuthSession(createEnvelopeFromSession(session), getQueryClient())

  return {
    ok: true as const,
    session
  }
}

export function getDefaultRouteForSession(session: AppSession) {
  if (session.authState !== 'authenticated') {
    return '/login'
  }

  return session.role === 'admin' ? '/admin/tenants' : '/app/overview'
}

function assertAuthenticatedSession(locationHref?: string) {
  const session = getSessionSnapshot()

  if (session.authState !== 'authenticated') {
    throw redirect({
      search: locationHref ? { redirect: locationHref } : undefined,
      to: '/login'
    }) as unknown as Error
  }

  return session
}

export function requireSession(role: AuthRole, locationHref?: string) {
  const session = assertAuthenticatedSession(locationHref)

  if (session.role !== role) {
    throw redirect({
      to: getDefaultRouteForSession(session)
    }) as unknown as Error
  }

  return session
}

export function redirectAuthenticatedSession() {
  const session = getSessionSnapshot()

  if (session.authState === 'authenticated') {
    throw redirect({
      to: getDefaultRouteForSession(session)
    }) as unknown as Error
  }
}

export function resetSessionForTests() {
  setAuthClientForTests(null)
  resetQueryClientForTests()
  resetRequestContextForTests()
}
