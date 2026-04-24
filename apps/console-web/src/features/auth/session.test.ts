import { isRedirect } from '@tanstack/react-router'
import { beforeEach, describe, expect, it } from 'vitest'
import {
  clearSession,
  getDefaultRouteForSession,
  getSessionSnapshot,
  redirectAuthenticatedSession,
  resetSessionForTests,
  requireSession,
  signIn
} from './session'

describe('session', () => {
  beforeEach(() => {
    resetSessionForTests()
  })

  const captureRedirect = (callback: () => void) => {
    try {
      callback()
    } catch (error) {
      if (isRedirect(error)) {
        return error
      }

      throw error
    }

    throw new Error('expected redirect')
  }

  it('returns anonymous when there is no stored session', () => {
    expect(getSessionSnapshot()).toEqual({
      authState: 'anonymous'
    })
  })

  it('creates a normalized authenticated tenant session', () => {
    const result = signIn({
      email: 'Tenant@Acme.Dev ',
      workspace: '  ACME-RETAIL '
    })

    expect(result).toEqual({
      ok: true,
      session: {
        authState: 'authenticated',
        email: 'tenant@acme.dev',
        role: 'tenant',
        tenantId: 'tenant_acme',
        workspace: 'acme-retail'
      }
    })
    expect(getSessionSnapshot()).toEqual(result.session)
  })

  it('rejects unknown workspaces without storing a session', () => {
    const result = signIn({
      email: 'ops@example.com',
      workspace: 'unknown-workspace'
    })

    expect(result).toEqual({
      ok: false,
      message:
        'Use one of the seeded workspaces: platform-admin, acme-retail, or northstar-labs.'
    })
    expect(getSessionSnapshot()).toEqual({
      authState: 'anonymous'
    })
  })

  it('clears the stored session', () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    clearSession()

    expect(getSessionSnapshot()).toEqual({
      authState: 'anonymous'
    })
  })

  it('returns the correct default route for each auth state', () => {
    expect(
      getDefaultRouteForSession({
        authState: 'anonymous'
      })
    ).toBe('/login')
    expect(
      getDefaultRouteForSession({
        authState: 'authenticated',
        email: 'admin@huge-router.dev',
        role: 'admin',
        tenantId: null,
        workspace: 'platform-admin'
      })
    ).toBe('/admin/tenants')
    expect(
      getDefaultRouteForSession({
        authState: 'authenticated',
        email: 'tenant@acme.dev',
        role: 'tenant',
        tenantId: 'tenant_acme',
        workspace: 'acme-retail'
      })
    ).toBe('/app/overview')
  })

  it('redirects anonymous access to login and preserves redirect target', () => {
    const redirect = captureRedirect(() =>
      requireSession('tenant', '/app/providers')
    )

    expect(redirect.options.to).toBe('/login')
    expect(redirect.options.search).toEqual({
      redirect: '/app/providers'
    })
  })

  it('redirects authenticated users to the correct shell when role does not match', () => {
    signIn({
      email: 'tenant@acme.dev',
      workspace: 'acme-retail'
    })

    const redirect = captureRedirect(() => requireSession('admin'))

    expect(redirect.options.to).toBe('/app/overview')
  })

  it('redirects authenticated sessions away from login', () => {
    signIn({
      email: 'admin@huge-router.dev',
      workspace: 'platform-admin'
    })

    const redirect = captureRedirect(() => redirectAuthenticatedSession())

    expect(redirect.options.to).toBe('/admin/tenants')
  })
})
