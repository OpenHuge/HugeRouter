import { z } from 'zod'

export const authProviderSchema = z.enum(['email', 'github', 'google', 'wechat', 'oidc'])

export type AuthProvider = z.infer<typeof authProviderSchema>

export type LoginReason =
  | 'provider-disabled'
  | 'session-expired'
  | 'signed-out'
  | 'tenant-denied'
  | 'tenant-selection'

export type LoginSearch = {
  redirect?: string
  reason?: LoginReason
  provider?: AuthProvider
  message?: string
}

export type AuthCallbackSearch = {
  code?: string
  error?: string
  errorDescription?: string
  provider?: AuthProvider
  redirect?: string
  state?: string
}

export const providerAvailabilitySchema = z.object({
  provider: authProviderSchema,
  enabled: z.boolean().default(true),
  hidden: z.boolean().default(false),
  reason: z.string().optional()
})

export type ProviderAvailability = z.infer<typeof providerAvailabilitySchema>

const tenantMembershipSchema = z.object({
  role: z.string(),
  tenantId: z.string(),
  tenantName: z.string(),
  tenantSlug: z.string()
})

export type TenantMembership = z.infer<typeof tenantMembershipSchema>

const authUserSchema = z.object({
  avatarUrl: z.string().optional(),
  displayName: z.string(),
  email: z.string(),
  id: z.string(),
  isPlatformAdmin: z.boolean().default(false)
})

export type AuthUser = z.infer<typeof authUserSchema>

const authSessionSchema = z.object({
  activeTenant: tenantMembershipSchema.nullish(),
  expiresAt: z.string(),
  memberships: z.array(tenantMembershipSchema).default([]),
  sessionId: z.string(),
  user: authUserSchema
})

export type AuthSession = z.infer<typeof authSessionSchema>

const baseSessionStateShape = {
  availableProviders: z.array(providerAvailabilitySchema).default([])
}

export const anonymousAuthStateSchema = z.object({
  ...baseSessionStateShape,
  kind: z.literal('anonymous')
})

export const authenticatedAuthStateSchema = z.object({
  ...baseSessionStateShape,
  kind: z.literal('authenticated'),
  session: authSessionSchema
})

export const tenantDeniedAuthStateSchema = z.object({
  ...baseSessionStateShape,
  kind: z.literal('tenant_access_denied'),
  message: z.string(),
  tenantSlug: z.string().optional()
})

export const tenantSelectionAuthStateSchema = z.object({
  ...baseSessionStateShape,
  kind: z.literal('tenant_selection_required'),
  memberships: z.array(tenantMembershipSchema).default([]),
  message: z.string()
})

export const authSessionStateSchema = z.discriminatedUnion('kind', [
  anonymousAuthStateSchema,
  authenticatedAuthStateSchema,
  tenantDeniedAuthStateSchema,
  tenantSelectionAuthStateSchema
])

export type AuthSessionState = z.infer<typeof authSessionStateSchema>

export const authSessionEnvelopeSchema = z.object({
  requestId: z.string().optional(),
  state: authSessionStateSchema,
  traceId: z.string().optional()
})

export type AuthSessionEnvelope = z.infer<typeof authSessionEnvelopeSchema>

export const emailLoginInputSchema = z.object({
  email: z.string().trim().email(),
  redirectTo: z.string().optional(),
  workspaceSlug: z.string().trim().min(1)
})

export type EmailLoginInput = z.infer<typeof emailLoginInputSchema>

export const emailLoginStartResultSchema = z.object({
  email: z.string(),
  flowId: z.string().min(1),
  codeHint: z.string().optional(),
  expiresAt: z.string().optional(),
  message: z.string(),
  outcome: z.literal('email_sent')
})

export type EmailLoginStartResult = z.infer<typeof emailLoginStartResultSchema>

export const emailLoginCompleteInputSchema = z.object({
  code: z.string().min(1),
  flowId: z.string().min(1)
})

export type EmailLoginCompleteInput = z.infer<typeof emailLoginCompleteInputSchema>

export const emailLoginCompleteResultSchema = z.discriminatedUnion('outcome', [
  z.object({
    message: z.string(),
    outcome: z.literal('authenticated'),
    redirectTo: z.string().optional(),
    state: authenticatedAuthStateSchema
  }),
  z.object({
    message: z.string(),
    outcome: z.literal('failure')
  }),
  z.object({
    message: z.string(),
    outcome: z.literal('tenant_denied'),
    tenantSlug: z.string().optional()
  })
])

export type EmailLoginCompleteResult = z.infer<typeof emailLoginCompleteResultSchema>

export const providerLoginStartInputSchema = z.object({
  redirectTo: z.string().optional(),
  workspaceSlug: z.string().trim().min(1)
})

export type ProviderLoginStartInput = z.infer<typeof providerLoginStartInputSchema>

export const providerLoginStartResultSchema = z.object({
  authorizationUrl: z.string().min(1),
  outcome: z.literal('redirect')
})

export type ProviderLoginStartResult = z.infer<typeof providerLoginStartResultSchema>

export const authCallbackInputSchema = z.object({
  code: z.string().optional(),
  error: z.string().optional(),
  errorDescription: z.string().optional(),
  redirectTo: z.string().optional(),
  state: z.string().optional()
})

export type AuthCallbackInput = z.infer<typeof authCallbackInputSchema>

const callbackBaseSchema = z.object({
  message: z.string(),
  provider: authProviderSchema
})

export const authCallbackResultSchema = z.discriminatedUnion('outcome', [
  callbackBaseSchema.extend({
    outcome: z.literal('authenticated'),
    redirectTo: z.string().optional(),
    state: authenticatedAuthStateSchema
  }),
  callbackBaseSchema.extend({
    outcome: z.literal('cancelled')
  }),
  callbackBaseSchema.extend({
    outcome: z.literal('failure')
  }),
  callbackBaseSchema.extend({
    outcome: z.literal('provider_disabled')
  }),
  callbackBaseSchema.extend({
    outcome: z.literal('tenant_denied'),
    tenantSlug: z.string().optional()
  })
])

export type AuthCallbackResult = z.infer<typeof authCallbackResultSchema>

export const logoutResultSchema = z.object({
  outcome: z.literal('signed_out')
})

export type LogoutResult = z.infer<typeof logoutResultSchema>

export type RequestMeta = {
  requestId?: string
  traceId?: string
}

const authProviderLabels: Record<AuthProvider, string> = {
  email: 'Email',
  github: 'GitHub',
  google: 'Google',
  wechat: 'WeChat',
  oidc: 'Enterprise SSO'
}

export function getAuthProviderLabel(provider: AuthProvider) {
  return authProviderLabels[provider]
}

export function getDefaultProviderAvailability(): ProviderAvailability[] {
  return ['email', 'github', 'google', 'wechat'].map((provider) =>
    providerAvailabilitySchema.parse({
      enabled: true,
      hidden: false,
      provider
    })
  )
}

function getSearchString(value: unknown) {
  if (typeof value === 'string' && value.length > 0) {
    return value
  }

  if (Array.isArray(value)) {
    return typeof value[0] === 'string' && value[0].length > 0 ? value[0] : undefined
  }

  return undefined
}

export function parseLoginSearch(rawSearch: Record<string, unknown>): LoginSearch {
  const provider = getSearchString(rawSearch.provider)
  const reason = getSearchString(rawSearch.reason)

  return {
    message: getSearchString(rawSearch.message),
    provider: provider && authProviderSchema.safeParse(provider).success
      ? authProviderSchema.parse(provider)
      : undefined,
    reason:
      reason &&
      ['provider-disabled', 'session-expired', 'signed-out', 'tenant-denied', 'tenant-selection'].includes(reason)
        ? (reason as LoginReason)
        : undefined,
    redirect: getSearchString(rawSearch.redirect)
  }
}

export function parseAuthCallbackSearch(
  rawSearch: Record<string, unknown>
): AuthCallbackSearch {
  const provider = getSearchString(rawSearch.provider)

  return {
    code: getSearchString(rawSearch.code),
    error: getSearchString(rawSearch.error),
    errorDescription: getSearchString(rawSearch.error_description) ?? getSearchString(rawSearch.errorDescription),
    provider: provider && authProviderSchema.safeParse(provider).success
      ? authProviderSchema.parse(provider)
      : undefined,
    redirect: getSearchString(rawSearch.redirect),
    state: getSearchString(rawSearch.state)
  }
}
