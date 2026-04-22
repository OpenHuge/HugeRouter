import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import {
  authProviderLinkSchema,
  authSessionSchema,
  authSessionResponseSchema,
  configSnapshotResponseSchema,
  contractDigest,
  emailLoginCompleteRequestSchema,
  emailLoginStartRequestSchema,
  gatewayAnthropicMessagesErrorSchema,
  gatewayAnthropicMessagesRequestSchema,
  gatewayAnthropicMessagesResponseSchema,
  gatewayGeminiGenerateContentErrorSchema,
  gatewayGeminiGenerateContentRequestSchema,
  gatewayGeminiGenerateContentResponseSchema,
  gatewayChatRequestSchema,
  gatewayChatResponseSchema,
  routeReceiptDiagnosticsResponseSchema,
  oauthCallbackRequestSchema,
  oauthLoginStartResponseSchema,
  oauthProviderSchema,
  projectsResponseSchema,
  providerResourcesResponseSchema,
  routePoliciesResponseSchema,
  routeSimulationRequestSchema,
  routeSimulationResponseSchema,
  tenantsResponseSchema,
  usageEventRecordedMessageSchema
} from './index.ts'

const readJson = (relativePath: string) =>
  JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), 'utf8')
  ) as Record<string, unknown>

void test('checked-in schema manifest stays in sync with generated package metadata', () => {
  const manifest = readJson('../../../schemas/jsonschema/contracts.manifest.json')

  assert.equal(manifest.contract_digest, contractDigest)
})

void test('control-plane examples conform to shared zod schemas', () => {
  const tenants = readJson('../../../schemas/examples/control-plane/tenants.response.json')
  const projects = readJson('../../../schemas/examples/control-plane/projects.response.json')
  const resources = readJson(
    '../../../schemas/examples/control-plane/provider-resources.response.json'
  )
  const routePolicies = readJson(
    '../../../schemas/examples/control-plane/route-policies.response.json'
  )
  const snapshot = readJson(
    '../../../schemas/examples/control-plane/config-snapshot.response.json'
  )
  const simulationRequest = readJson(
    '../../../schemas/examples/control-plane/route-simulation.request.json'
  )
  const simulationResponse = readJson(
    '../../../schemas/examples/control-plane/route-simulation.response.json'
  )
  const sessionResponse = readJson('../../../schemas/examples/auth/session-response.json')

  assert.equal(tenantsResponseSchema.parse(tenants).data[0]?.tenant_id, 'tenant_acme')
  assert.equal(projectsResponseSchema.parse(projects).data[0]?.project_id, 'proj_core')
  assert.equal(
    providerResourcesResponseSchema.parse(resources).data[0]?.provider_id,
    'openai'
  )
  assert.equal(
    routePoliciesResponseSchema.parse(routePolicies).data[0]?.route_policy_id,
    'routepol_default'
  )
  assert.equal(
    configSnapshotResponseSchema.parse(snapshot).config_snapshot.config_snapshot_id,
    'cfgsnap_default'
  )
  assert.equal(
    routeSimulationRequestSchema.parse(simulationRequest).protocol_family,
    'openai_chat'
  )
  assert.equal(
    routeSimulationResponseSchema.parse(simulationResponse).selected_target,
    'prvrsrc_openai_primary'
  )
  assert.equal(
    authSessionResponseSchema.parse(sessionResponse).session?.sessionId,
    'sess_123'
  )
})

void test('gateway and event examples conform to shared zod schemas', () => {
  const gatewayRequest = readJson('../../../schemas/examples/gateway/chat.request.json')
  const gatewayResponse = readJson('../../../schemas/examples/gateway/chat.response.json')
  const anthropicRequest = readJson('../../../schemas/examples/gateway/anthropic-messages.request.json')
  const anthropicResponse = readJson(
    '../../../schemas/examples/gateway/anthropic-messages.response.json'
  )
  const anthropicError = readJson(
    '../../../schemas/examples/gateway/anthropic-messages.error.response.json'
  )
  const geminiRequest = readJson(
    '../../../schemas/examples/gateway/gemini-generate-content.request.json'
  )
  const geminiResponse = readJson(
    '../../../schemas/examples/gateway/gemini-generate-content.response.json'
  )
  const geminiError = readJson(
    '../../../schemas/examples/gateway/gemini-generate-content.error.response.json'
  )
  const routeReceiptDiagnostics = readJson(
    '../../../schemas/examples/control-plane/route-receipt-diagnostics.response.json'
  )
  const usageMessage = readJson(
    '../../../schemas/examples/events/usage-event-recorded.message.json'
  )

  assert.equal(
    gatewayChatRequestSchema.parse(gatewayRequest).request.protocol_family,
    'openai_chat'
  )
  assert.equal(
    gatewayChatResponseSchema.parse(gatewayResponse).usage_event.phase,
    'final'
  )
  assert.equal(
    usageEventRecordedMessageSchema.parse(usageMessage).message_type,
    'usage_event.recorded'
  )
  assert.equal(
    gatewayAnthropicMessagesRequestSchema.parse(anthropicRequest).messages[0]?.role,
    'user'
  )
  assert.equal(
    gatewayAnthropicMessagesResponseSchema.parse(anthropicResponse).content[0]?.type,
    'text'
  )
  assert.equal(
    gatewayAnthropicMessagesErrorSchema.parse(anthropicError).error.code,
    'validation_failed'
  )
  assert.equal(
    gatewayGeminiGenerateContentRequestSchema.parse(geminiRequest).contents[0]?.role,
    'user'
  )
  assert.equal(
    gatewayGeminiGenerateContentResponseSchema.parse(geminiResponse).candidates[0]?.content?.role,
    'model'
  )
  assert.equal(
    gatewayGeminiGenerateContentErrorSchema.parse(geminiError).error.code,
    'validation_failed'
  )
  assert.equal(
    routeReceiptDiagnosticsResponseSchema.parse(routeReceiptDiagnostics).route_receipt.route_receipt_id,
    'routercpt_123'
  )
})

void test('oauth providers exclude email', () => {
  assert.equal(oauthProviderSchema.parse('github'), 'github')
  assert.throws(() => oauthProviderSchema.parse('email'))
})

void test('auth session payload parses a minimal real session', () => {
  const parsed = authSessionSchema.parse({
    sessionId: 'sess_123',
    state: 'active',
    user: {
      userId: 'user_123',
      primaryEmail: 'dev@example.com',
      displayName: 'Dev Operator',
      avatarUrl: 'https://example.com/avatar.png',
      createdAt: '2026-04-20T09:00:00Z',
      lastLoginAt: '2026-04-22T09:30:00Z'
    },
    activeTenantId: 'tenant_123',
    memberships: [
      {
        membershipId: 'tmemb_123',
        tenant: {
          id: 'tenant_123',
          slug: 'acme',
          displayName: 'Acme'
        },
        role: 'admin',
        status: 'active'
      }
    ],
    authenticatedBy: 'google',
    createdAt: '2026-04-22T09:30:00Z',
    expiresAt: '2026-04-29T09:30:00Z',
    lastAuthenticatedAt: '2026-04-22T09:30:00Z'
  })

  assert.equal(parsed.user.userId, 'user_123')
  assert.equal(parsed.memberships[0]?.tenant.displayName, 'Acme')
  assert.equal(parsed.authenticatedBy, 'google')
})

void test('auth provider links preserve provider subject and unlink flag', () => {
  const parsed = authProviderLinkSchema.parse({
    linkId: 'authlink_123',
    provider: 'wechat',
    providerSubject: 'wechat-openid-42',
    linkedAt: '2026-04-22T09:30:00Z',
    canUnlink: false
  })

  assert.equal(parsed.providerSubject, 'wechat-openid-42')
  assert.equal(parsed.canUnlink, false)
})

void test('email and oauth completion requests remain distinct shapes', () => {
  const emailParsed = emailLoginCompleteRequestSchema.parse({
    flowId: 'authflow_123',
    code: '123456'
  })
  const oauthParsed = oauthCallbackRequestSchema.parse({
    state: 'oauth-state-123',
    code: 'oauth-code-123',
    redirectUri: 'https://console.example.com/login/callback'
  })

  assert.equal(emailParsed.flowId, 'authflow_123')
  assert.ok(!('state' in emailParsed))
  assert.equal(oauthParsed.state, 'oauth-state-123')
  assert.ok(!('flowId' in oauthParsed))
})

void test('checked-in oauth example stays in sync with shared schema', () => {
  const example = readJson('../../../schemas/examples/auth/github-oauth-start-response.json')
  const parsed = oauthLoginStartResponseSchema.parse(example)

  assert.equal(parsed.provider, 'github')
  assert.match(parsed.authorizationUrl, /^https:\/\/github\.com\//)
})

void test('email login start request requires a workspace slug', () => {
  const parsed = emailLoginStartRequestSchema.parse({
    email: 'dev@example.com',
    workspaceSlug: 'platform-admin'
  })

  assert.equal(parsed.workspaceSlug, 'platform-admin')
})
