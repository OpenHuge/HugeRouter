import { z } from 'zod'
import {
  ADMISSION_RESULTS,
  COMPATIBILITY_RULES,
  CONTRACT_DIGEST,
  CONTRACT_VERSION,
  PROTOCOL_FAMILIES,
  PROVIDER_RESOURCE_STATUSES,
  USAGE_PHASES
} from './generated/contract-meta.ts'
export { createOpeningGrantRequestSchema, openingCredentialSchema, openingGrantCreateResponseSchema, openingGrantIdSchema, openingGrantRevokeRequestSchema, openingGrantSchema, openingGrantsResponseSchema, ownerAccountIdSchema, type CreateOpeningGrantRequest, type OpeningCredential, type OpeningGrant, type OpeningGrantCreateResponse, type OpeningGrantRevokeRequest, type OpeningGrantsResponse } from './opening-grants.ts'
export { cardDeliveryKindSchema, cardProductIdSchema, cardProductSchema, cardProductStatusSchema, merchantFulfillmentModeSchema, merchantShopIdSchema, merchantShopSchema, merchantShopStatusSchema, merchantWorkspaceResponseSchema, merchantWorkspaceSchema, relayCheckStatusSchema, relayEvaluationIdSchema, relayEvaluationRunnerModeSchema, relayEvaluationSchema, relayEvaluationVerdictSchema, replayCapsuleIdSchema, replayCapsuleResponseSchema, replayCapsuleSchema, trialConnectionIdSchema, trialConnectionSchema, trialConnectionStatusSchema, type CardProduct, type MerchantShop, type MerchantWorkspace, type MerchantWorkspaceResponse, type RelayEvaluation, type ReplayCapsule, type ReplayCapsuleResponse, type TrialConnection } from './merchant.ts'
import { openingGrantIdSchema, ownerAccountIdSchema } from './opening-grants.ts'

const dateTimeSchema = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}T.+Z$/, 'Expected an RFC3339 UTC timestamp')

const slugSchema = z
  .string()
  .regex(/^[a-z0-9-]+$/, 'Expected a lowercase slug with digits or hyphens')

const prefixedId = (prefix: string) =>
  z
    .string()
    .regex(new RegExp(`^${prefix}[A-Za-z0-9][A-Za-z0-9_-]*$`), `Expected id with prefix ${prefix}`)

const endpointBaseUrlSchema = z.url().refine(
  (value) => {
    const parsed = new URL(value)
    if (parsed.protocol === 'https:') {
      return true
    }
    if (parsed.protocol !== 'http:') {
      return false
    }
    return ['localhost', '127.0.0.1', '[::1]'].includes(parsed.hostname)
  },
  { message: 'Expected an https endpoint or a local loopback http endpoint' }
)

export const contractVersion = CONTRACT_VERSION
export const contractDigest = CONTRACT_DIGEST
export const compatibilityRules = [...COMPATIBILITY_RULES]

export const protocolFamilySchema = z.enum(PROTOCOL_FAMILIES)
export const providerResourceStatusSchema = z.enum(PROVIDER_RESOURCE_STATUSES)
export const admissionResultSchema = z.enum(ADMISSION_RESULTS)
export const usagePhaseSchema = z.enum(USAGE_PHASES)

export const tenantIdSchema = prefixedId('tenant_')
export const projectIdSchema = prefixedId('proj_')
export const credentialIdSchema = prefixedId('cred_')
export const providerResourceIdSchema = prefixedId('prvrsrc_')
export const routePolicyIdSchema = prefixedId('routepol_')
export const budgetPolicyIdSchema = prefixedId('budgetpol_')
export const configSnapshotIdSchema = prefixedId('cfgsnap_')
export const routeReceiptIdSchema = prefixedId('routercpt_')
export const usageEventIdSchema = prefixedId('usageevt_')
export const ledgerEntryIdSchema = prefixedId('ledger_')
export const userIdSchema = prefixedId('user_')
export const tenantMembershipIdSchema = prefixedId('tmemb_')
export const authSessionIdSchema = prefixedId('sess_')
export const authProviderLinkIdSchema = prefixedId('authlink_')
export const authFlowIdSchema = prefixedId('authflow_')
export const serviceNameSchema = z.string().min(1)
export const providerCapabilitySchema = z.object({
  supports_streaming: z.boolean(),
  supports_tool_calling: z.boolean(),
  supports_json_mode: z.boolean(),
  supports_realtime: z.boolean(),
  supports_response_model_metadata: z.boolean()
})

export const tenantSchema = z.object({
  tenant_id: tenantIdSchema,
  slug: slugSchema,
  display_name: z.string().min(1),
  version: z.number().int().nonnegative(),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema
})

export const projectSchema = z.object({
  project_id: projectIdSchema,
  tenant_id: tenantIdSchema,
  slug: slugSchema,
  display_name: z.string().min(1),
  version: z.number().int().nonnegative(),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema
})

export const quotaReserveStrategySchema = z.enum([
  'none',
  'estimate_then_reserve',
  'fixed_reserve'
])

export const providerResourceSchema = z.object({
  provider_resource_id: providerResourceIdSchema,
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema.optional(),
  provider_id: z.string().min(1),
  name: z.string().min(1),
  status: providerResourceStatusSchema,
  provenance_class: z.enum([
    'official_api',
    'official_gateway',
    'byo_customer_credential',
    'dedicated_managed_account',
    'shared_brokered_pool',
    'unofficial_client_channel'
  ]),
  credential_owner_type: z.enum(['platform', 'tenant', 'project', 'partner']),
  deployment_scope: z.enum(['shared', 'tenant_dedicated', 'project_dedicated']),
  region: z.string().min(1),
  endpoint_base_url: endpointBaseUrlSchema,
  auth_kind: z.enum(['api_key', 'oauth_client_credentials', 'session_broker']),
  health_state: z.enum(['healthy', 'degraded', 'quarantined', 'draining', 'disabled']),
  health_message: z.string().min(1).optional(),
  quarantine_reason: z.string().min(1).optional(),
  budget_policy_id: budgetPolicyIdSchema.optional(),
  capabilities: providerCapabilitySchema,
  supported_protocol_families: z.array(protocolFamilySchema).min(1),
  is_transit_gateway: z.boolean(),
  version: z.number().int().nonnegative(),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema
})

export const routePolicySchema = z.object({
  route_policy_id: routePolicyIdSchema,
  tenant_id: tenantIdSchema,
  display_name: z.string().min(1),
  protocol_family: protocolFamilySchema,
  model_alias: z.string().min(1),
  required_capabilities: z.array(z.string().min(1)).min(1),
  preferred_regions: z.array(z.string().min(1)),
  version: z.number().int().nonnegative(),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema
})

export const configSnapshotSchema = z.object({
  config_snapshot_id: configSnapshotIdSchema,
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema,
  revision: z.number().int().nonnegative(),
  status: z.enum(['draft', 'active', 'superseded']),
  activated_at: dateTimeSchema.optional(),
  provider_resource_ids: z.array(providerResourceIdSchema),
  route_policy_id: routePolicyIdSchema,
  budget_policy_id: budgetPolicyIdSchema
})

export const monetaryAmountSchema = z.object({
  currency: z.string().min(1),
  amount: z.string().regex(/^-?\d+(\.\d+)?$/)
})

export const scoreBreakdownSchema = z.object({
  latency: z.number(),
  cost: z.number(),
  health: z.number(),
  trust: z.number()
})

export const excludedTargetSchema = z.object({
  provider_resource_id: providerResourceIdSchema,
  reason_code: z.string().min(1),
  reason: z.string().min(1)
})

export const fallbackTransitionSchema = z.object({
  from_provider_resource_id: providerResourceIdSchema,
  to_provider_resource_id: providerResourceIdSchema,
  reason: z.string().min(1)
})

export const validationIssueSchema = z.object({
  field: z.string().min(1),
  message: z.string().min(1)
})

export const normalizedErrorSchema = z.object({
  code: z.string().min(1),
  message: z.string().min(1),
  request_id: z.string().min(1),
  retryable: z.boolean(),
  upstream_code: z.string().min(1).optional(),
  upstream_status_code: z.number().int().positive().optional(),
  validation_issues: z.array(validationIssueSchema).default([]),
  details: z.record(z.string(), z.string()).default({})
})

export const errorEnvelopeSchema = z.object({
  error: normalizedErrorSchema
})

export const routeReceiptSchema = z.object({
  route_receipt_id: routeReceiptIdSchema,
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema,
  route_policy_id: routePolicyIdSchema,
  request_id: z.string().min(1),
  trace_id: z.string().min(1),
  protocol_family: protocolFamilySchema,
  model_alias: z.string().min(1),
  config_snapshot_id: configSnapshotIdSchema,
  admission_result: admissionResultSchema,
  selected_target: providerResourceIdSchema.optional(),
  excluded_targets: z.array(excludedTargetSchema),
  score_breakdown: scoreBreakdownSchema,
  fallback_transitions: z.array(fallbackTransitionSchema),
  normalized_error: normalizedErrorSchema.optional(),
  failure_reason: z.string().min(1).optional(),
  created_at: dateTimeSchema
})

export const usageMetricsSchema = z.object({
  input_tokens: z.number().int().nonnegative(),
  output_tokens: z.number().int().nonnegative(),
  cached_input_tokens: z.number().int().nonnegative()
})

export const usageEventSchema = z.object({
  usage_event_id: usageEventIdSchema,
  route_receipt_id: routeReceiptIdSchema,
  grant_id: openingGrantIdSchema.optional(),
  owner_account_id: ownerAccountIdSchema.optional(),
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema,
  provider_resource_id: providerResourceIdSchema,
  model_alias: z.string().min(1),
  phase: usagePhaseSchema,
  idempotency_key: z.string().min(1),
  usage: usageMetricsSchema,
  estimated_cost: monetaryAmountSchema,
  recorded_at: dateTimeSchema
})

export const requestEnvelopeSchema = z.object({
  protocol_family: protocolFamilySchema,
  source_service: serviceNameSchema,
  request_id: z.string().min(1),
  trace_id: z.string().min(1),
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema
})

export const chatMessageSchema = z.object({
  role: z.enum(['system', 'user', 'assistant', 'tool']),
  content: z.string().min(1)
})

export const toolDefinitionSchema = z.object({
  name: z.string().min(1),
  description: z.string().min(1)
})

export const chatRequestSchema = z.object({
  model_alias: z.string().min(1),
  messages: z.array(chatMessageSchema).min(1),
  required_capabilities: z.array(z.string().min(1)),
  expected_prompt_tokens: z.number().int().nonnegative(),
  max_output_tokens: z.number().int().positive(),
  temperature_milli: z.number().int().nonnegative(),
  tools: z.array(toolDefinitionSchema),
  conversation_id: z.string().min(1).optional()
})

export const gatewayChatRequestSchema = z.object({
  request: requestEnvelopeSchema,
  chat: chatRequestSchema
})

export const gatewayChatResponseSchema = z.object({
  route_receipt: routeReceiptSchema,
  usage_event: usageEventSchema,
  provider_response_id: z.string().min(1),
  output_text: z.string()
})

export const gatewayAnthropicMessageContentBlockSchema = z.object({
  type: z.literal('text'),
  text: z.string().min(1)
})

export const gatewayAnthropicMessageContentSchema = z.union([
  z.string().min(1),
  z.array(gatewayAnthropicMessageContentBlockSchema)
])

export const gatewayAnthropicMessageSchema = z.object({
  role: z.string().min(1),
  content: gatewayAnthropicMessageContentSchema
})

export const gatewayAnthropicMessagesRequestSchema = z.object({
  model: z.string().min(1),
  messages: z.array(gatewayAnthropicMessageSchema),
  max_tokens: z.number().int().nonnegative().optional(),
  system: z.string().min(1).optional(),
  stream: z.boolean().optional(),
  temperature: z.number().nonnegative().optional(),
  top_p: z.number().nonnegative().optional(),
  top_k: z.number().int().nonnegative().optional()
})

export const gatewayAnthropicUsageSchema = z.object({
  input_tokens: z.number().int().nonnegative(),
  output_tokens: z.number().int().nonnegative()
})

export const gatewayAnthropicMessagesResponseSchema = z.object({
  id: z.string().min(1).optional(),
  model: z.string().min(1).optional(),
  content: z.array(
    z.object({
      type: z.string().min(1),
      text: z.string().min(1)
    })
  ),
  stop_reason: z.string().min(1).optional(),
  usage: gatewayAnthropicUsageSchema.optional()
})

export const gatewayAnthropicMessagesErrorSchema = z.object({
  error: normalizedErrorSchema
})

export const gatewayGeminiPartSchema = z.object({
  text: z.string().min(1)
})

export const gatewayGeminiRoleSchema = z.enum(['user', 'assistant', 'model', 'system'])

export const gatewayGeminiContentSchema = z.object({
  role: gatewayGeminiRoleSchema,
  parts: z.array(gatewayGeminiPartSchema)
})

export const gatewayGeminiSystemInstructionSchema = z.object({
  parts: z.array(gatewayGeminiPartSchema)
})

export const gatewayGeminiGenerationConfigSchema = z.object({
  maxOutputTokens: z.number().int().nonnegative().optional(),
  temperature: z.number().optional()
})

export const gatewayGeminiGenerateContentRequestSchema = z.object({
  model: z.string().min(1),
  contents: z.array(gatewayGeminiContentSchema),
  tools: z.array(z.unknown()),
  stream: z.boolean().default(false),
  systemInstruction: gatewayGeminiSystemInstructionSchema.optional(),
  generationConfig: gatewayGeminiGenerationConfigSchema.optional()
})

export const gatewayGeminiCandidateSchema = z.object({
  content: gatewayGeminiContentSchema.optional(),
  finishReason: z.string().min(1).optional()
})

export const gatewayGeminiUsageMetadataSchema = z.object({
  promptTokenCount: z.number().int().nonnegative(),
  candidatesTokenCount: z.number().int().nonnegative(),
  totalTokenCount: z.number().int().nonnegative(),
  cachedContentTokenCount: z.number().int().nonnegative()
})

export const gatewayGeminiGenerateContentResponseSchema = z.object({
  responseId: z.string().min(1).optional(),
  candidates: z.array(gatewayGeminiCandidateSchema),
  usageMetadata: gatewayGeminiUsageMetadataSchema.optional(),
  modelVersion: z.string().min(1).optional()
})

export const gatewayGeminiGenerateContentErrorSchema = z.object({
  error: normalizedErrorSchema
})

export const routeReceiptDecisionTraceStepSchema = z.object({
  stage: z.string().min(1),
  status: z.string().min(1),
  message: z.string().min(1),
  score: z.number().optional(),
  notes: z.array(z.string().min(1))
})

export const routeReceiptPolicyCheckSchema = z.object({
  policy_id: routePolicyIdSchema,
  status: z.string().min(1),
  reason: z.string().min(1).optional()
})

export const routeReceiptProviderAttemptSchema = z.object({
  provider_resource_id: providerResourceIdSchema,
  attempt: z.number().int().nonnegative(),
  status: z.string().min(1),
  started_at: dateTimeSchema,
  finished_at: dateTimeSchema,
  latency_ms: z.number().int().nonnegative(),
  reason: z.string().min(1)
})

export const routeReceiptDiagnosticsResponseSchema = z.object({
  route_receipt: routeReceiptSchema,
  decision_timeline: z.array(routeReceiptDecisionTraceStepSchema),
  policy_checks: z.array(routeReceiptPolicyCheckSchema),
  provider_attempts: z.array(routeReceiptProviderAttemptSchema),
  metadata: z.record(z.string(), z.string())
})

export const routeDiagnosticDecisionSchema = z.enum([
  'selected',
  'eligible',
  'excluded'
])

export const routeReceiptSummarySchema = z.object({
  route_receipt_id: routeReceiptIdSchema,
  admission_result: admissionResultSchema,
  selected_target: providerResourceIdSchema.optional(),
  failure_reason: z.string().min(1).optional(),
  created_at: dateTimeSchema
})

export const routeDiagnosticTargetSchema = z.object({
  provider_resource: providerResourceSchema,
  decision: routeDiagnosticDecisionSchema,
  in_active_snapshot: z.boolean(),
  supports_protocol_family: z.boolean(),
  capability_gaps: z.array(z.string().min(1)),
  reason_code: z.string().min(1),
  reason: z.string().min(1),
  recent_receipt_id: routeReceiptIdSchema.optional(),
  recent_receipt_reason: z.string().min(1).optional()
})

export const routeDiagnosticsResponseSchema = z.object({
  route_policy: routePolicySchema,
  active_snapshot: configSnapshotSchema.optional(),
  active_snapshot_matches_route_policy: z.boolean(),
  last_route_receipt: routeReceiptSummarySchema.optional(),
  recent_receipts: z.array(routeReceiptSummarySchema),
  targets: z.array(routeDiagnosticTargetSchema)
})

export const eligibleCandidateSchema = z.object({
  provider_resource_id: providerResourceIdSchema,
  score_breakdown: scoreBreakdownSchema
})

export const routeSimulationRequestSchema = z.object({
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema,
  credential_scope: credentialIdSchema,
  protocol_family: protocolFamilySchema,
  model_alias: z.string().min(1),
  required_capabilities: z.array(z.string().min(1)),
  region: z.string().min(1),
  expected_prompt_tokens: z.number().int().nonnegative(),
  expected_max_output_tokens: z.number().int().nonnegative(),
  traffic_class: z.string().min(1)
})

export const routeSimulationResponseSchema = z.object({
  simulation_id: z.string().min(1),
  config_snapshot_id: configSnapshotIdSchema,
  admission_result: admissionResultSchema,
  eligible_candidates: z.array(eligibleCandidateSchema),
  excluded_candidates: z.array(excludedTargetSchema),
  selected_target: providerResourceIdSchema.optional(),
  estimated_cost: monetaryAmountSchema
})

export const tenantsResponseSchema = z.object({
  data: z.array(tenantSchema)
})

export const projectsResponseSchema = z.object({
  data: z.array(projectSchema)
})

export const providerResourcesResponseSchema = z.object({
  data: z.array(providerResourceSchema)
})

export const routePoliciesResponseSchema = z.object({
  data: z.array(routePolicySchema)
})

export const configSnapshotResponseSchema = z.object({
  config_snapshot: configSnapshotSchema
})

export const routeReceiptResponseSchema = z.object({
  route_receipt: routeReceiptSchema
})

export const routeReceiptsResponseSchema = z.object({
  data: z.array(routeReceiptSchema)
})

export const usageSummarySchema = z.object({
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema.optional(),
  owner_account_id: ownerAccountIdSchema.optional(),
  window_start: dateTimeSchema,
  window_end: dateTimeSchema,
  currency: z.string().min(1),
  event_count: z.number().int().nonnegative(),
  input_tokens: z.number().int().nonnegative(),
  output_tokens: z.number().int().nonnegative(),
  cached_input_tokens: z.number().int().nonnegative(),
  provider_cost: monetaryAmountSchema,
  billable_price: monetaryAmountSchema
})

export const usageSummaryResponseSchema = z.object({
  data: usageSummarySchema
})

export const usageBreakdownRowSchema = z.object({
  bucket: z.string().min(1),
  provider_id: z.string().min(1).optional(),
  model_alias: z.string().min(1).optional(),
  input_tokens: z.number().int().nonnegative(),
  output_tokens: z.number().int().nonnegative(),
  cached_input_tokens: z.number().int().nonnegative(),
  provider_cost: monetaryAmountSchema,
  billable_price: monetaryAmountSchema
})

export const usageBreakdownResponseSchema = z.object({
  data: z.array(usageBreakdownRowSchema),
  next_cursor: z.string().min(1).optional()
})

export const balanceProjectionSchema = z.object({
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema.optional(),
  owner_account_id: ownerAccountIdSchema.optional(),
  currency: z.string().min(1),
  provider_cost_total: monetaryAmountSchema,
  billable_total: monetaryAmountSchema,
  configured_budget: monetaryAmountSchema,
  remaining_budget: monetaryAmountSchema,
  threshold_status: z.string().min(1),
  last_projected_at: dateTimeSchema,
  projection_lag_seconds: z.number().int().nonnegative()
})

export const balanceProjectionResponseSchema = z.object({
  data: balanceProjectionSchema
})

export const pricingSimulationRequestSchema = z.object({
  provider_id: z.string().min(1),
  model_alias: z.string().min(1),
  usage: usageMetricsSchema,
  region: z.string().min(1).optional(),
  image_generation_units: z.number().int().nonnegative().optional(),
  audio_seconds: z.number().int().nonnegative().optional()
})

export const pricingSimulationLineItemSchema = z.object({
  dimension: z.string().min(1),
  units: z.number().int().nonnegative(),
  provider_cost: monetaryAmountSchema,
  billable_price: monetaryAmountSchema,
  rate_source: z.string().min(1)
})

export const pricingSimulationResponseSchema = z.object({
  catalog_id: z.string().min(1),
  catalog_version: z.number().int().nonnegative(),
  currency: z.string().min(1),
  provider_cost: monetaryAmountSchema,
  billable_price: monetaryAmountSchema,
  line_items: z.array(pricingSimulationLineItemSchema)
})

export const pricingCatalogEntrySchema = z.object({
  dimension: z.string().min(1),
  provider_id: z.string().min(1),
  model_alias: z.string().min(1).optional(),
  region: z.string().min(1).optional(),
  micros_per_unit: z.number().int(),
  unit_denominator: z.number().int().positive(),
  source: z.string().min(1)
})

export const pricingCatalogResponseSchema = z.object({
  catalog_id: z.string().min(1),
  catalog_version: z.number().int().nonnegative(),
  currency: z.string().min(1),
  entries: z.array(pricingCatalogEntrySchema)
})

export const billingExportRequestSchema = z.object({
  tenant_id: tenantIdSchema.optional(),
  project_id: projectIdSchema.optional(),
  window_start: dateTimeSchema,
  window_end: dateTimeSchema,
  format: z.string().min(1)
})

export const billingExportJobSchema = z.object({
  export_job_id: z.string().min(1),
  status: z.string().min(1),
  format: z.string().min(1),
  requested_at: dateTimeSchema,
  completed_at: dateTimeSchema.optional(),
  error_message: z.string().min(1).optional(),
  tenant_id: tenantIdSchema.optional(),
  project_id: projectIdSchema.optional()
})

export const billingExportJobResponseSchema = z.object({
  data: billingExportJobSchema
})

export const billingExportJobsResponseSchema = z.object({
  data: z.array(billingExportJobSchema)
})

export const usageEventRecordedMessageSchema = z.object({
  message_id: z.string().min(1),
  message_type: z.literal('usage_event.recorded'),
  schema_version: z.number().int().positive(),
  occurred_at: dateTimeSchema,
  producer: serviceNameSchema,
  trace_id: z.string().min(1).optional(),
  request_id: z.string().min(1).optional(),
  idempotency_key: z.string().min(1),
  payload: z.object({
    usage_event: usageEventSchema
  })
})

export const configSnapshotActivatedMessageSchema = z.object({
  message_id: z.string().min(1),
  message_type: z.literal('config_snapshot.activated'),
  schema_version: z.number().int().positive(),
  occurred_at: dateTimeSchema,
  producer: serviceNameSchema,
  trace_id: z.string().min(1).optional(),
  request_id: z.string().min(1).optional(),
  idempotency_key: z.string().min(1),
  payload: z.object({
    config_snapshot: configSnapshotSchema
  })
})

export const authProviderSchema = z.enum(['email', 'github', 'google', 'wechat', 'oidc'])
export const oauthProviderSchema = z.enum(['github', 'google', 'wechat', 'oidc'])
export const tenantMembershipRoleSchema = z.enum(['owner', 'admin', 'member'])
export const tenantMembershipStatusSchema = z.enum(['active', 'invited', 'suspended'])
export const authSessionStateSchema = z.enum(['active', 'revoked', 'expired'])
export const emailLoginVerificationModeSchema = z.enum(['magic_link', 'one_time_code'])

export const tenantSummarySchema = z.object({
  id: tenantIdSchema,
  slug: slugSchema,
  displayName: z.string().min(1)
})

export const userIdentitySchema = z.object({
  userId: userIdSchema,
  primaryEmail: z.email().optional(),
  displayName: z.string().min(1),
  avatarUrl: z.url().optional(),
  createdAt: dateTimeSchema,
  lastLoginAt: dateTimeSchema.optional()
})

export const tenantMembershipSchema = z.object({
  membershipId: tenantMembershipIdSchema,
  tenant: tenantSummarySchema,
  role: tenantMembershipRoleSchema,
  status: tenantMembershipStatusSchema
})

export const authProviderLinkSchema = z.object({
  linkId: authProviderLinkIdSchema,
  provider: authProviderSchema,
  providerSubject: z.string().min(1),
  email: z.email().optional(),
  linkedAt: dateTimeSchema,
  lastUsedAt: dateTimeSchema.optional(),
  canUnlink: z.boolean()
})

export const authSessionSchema = z.object({
  sessionId: authSessionIdSchema,
  state: authSessionStateSchema,
  user: userIdentitySchema,
  activeTenantId: tenantIdSchema.optional(),
  memberships: z.array(tenantMembershipSchema),
  authenticatedBy: authProviderSchema,
  createdAt: dateTimeSchema,
  expiresAt: dateTimeSchema,
  lastAuthenticatedAt: dateTimeSchema
})

export const authProviderAvailabilitySchema = z.object({
  provider: authProviderSchema,
  displayName: z.string().min(1),
  enabled: z.boolean(),
  startPath: z.string().startsWith('/'),
  reasonCode: z.string().min(1).optional()
})

export const emailLoginStartRequestSchema = z.object({
  email: z.email(),
  workspaceSlug: slugSchema,
  redirectTo: z.string().min(1).optional()
})

export const emailLoginStartResponseSchema = z.object({
  flowId: authFlowIdSchema,
  verificationMode: emailLoginVerificationModeSchema,
  expiresAt: dateTimeSchema,
  codeHint: z.string().min(1).optional()
})

export const emailLoginCompleteRequestSchema = z.object({
  flowId: authFlowIdSchema,
  code: z.string().min(1)
})

export const oauthLoginStartRequestSchema = z.object({
  workspaceSlug: slugSchema,
  redirectTo: z.string().min(1).optional()
})

export const oauthLoginStartResponseSchema = z.object({
  provider: oauthProviderSchema,
  authorizationUrl: z.url(),
  state: z.string().min(1),
  expiresAt: dateTimeSchema
})

export const oauthCallbackRequestSchema = z.object({
  state: z.string().min(1),
  code: z.string().min(1),
  redirectUri: z.url().optional()
})

export const authLoginResultSchema = z.object({
  session: authSessionSchema,
  links: z.array(authProviderLinkSchema),
  redirectTo: z.string().min(1).optional()
})

export const authSessionResponseSchema = z.object({
  session: authSessionSchema.optional().nullable()
})

export const authProviderLinksResponseSchema = z.object({
  links: z.array(authProviderLinkSchema)
})

export const authProvidersResponseSchema = z.object({
  providers: z.array(authProviderAvailabilitySchema)
})

export const logoutResponseSchema = z.object({
  outcome: z.literal('signed_out')
})

export const unlinkAuthProviderResponseSchema = z.object({
  provider: authProviderSchema,
  removed: z.boolean()
})

export type ProtocolFamily = z.infer<typeof protocolFamilySchema>
export type Tenant = z.infer<typeof tenantSchema>
export type Project = z.infer<typeof projectSchema>
export type ProviderResource = z.infer<typeof providerResourceSchema>
export type RoutePolicy = z.infer<typeof routePolicySchema>
export type ConfigSnapshot = z.infer<typeof configSnapshotSchema>
export type NormalizedError = z.infer<typeof normalizedErrorSchema>
export type ErrorEnvelope = z.infer<typeof errorEnvelopeSchema>
export type RouteReceipt = z.infer<typeof routeReceiptSchema>
export type UsageEvent = z.infer<typeof usageEventSchema>
export type UsageSummary = z.infer<typeof usageSummarySchema>
export type UsageSummaryResponse = z.infer<typeof usageSummaryResponseSchema>
export type UsageBreakdownRow = z.infer<typeof usageBreakdownRowSchema>
export type UsageBreakdownResponse = z.infer<typeof usageBreakdownResponseSchema>
export type BalanceProjection = z.infer<typeof balanceProjectionSchema>
export type BalanceProjectionResponse = z.infer<typeof balanceProjectionResponseSchema>
export type PricingSimulationRequest = z.infer<typeof pricingSimulationRequestSchema>
export type PricingSimulationLineItem = z.infer<typeof pricingSimulationLineItemSchema>
export type PricingSimulationResponse = z.infer<typeof pricingSimulationResponseSchema>
export type PricingCatalogEntry = z.infer<typeof pricingCatalogEntrySchema>
export type PricingCatalogResponse = z.infer<typeof pricingCatalogResponseSchema>
export type BillingExportRequest = z.infer<typeof billingExportRequestSchema>
export type BillingExportJob = z.infer<typeof billingExportJobSchema>
export type BillingExportJobResponse = z.infer<typeof billingExportJobResponseSchema>
export type BillingExportJobsResponse = z.infer<typeof billingExportJobsResponseSchema>
export type ChatRequest = z.infer<typeof chatRequestSchema>
export type GatewayChatRequest = z.infer<typeof gatewayChatRequestSchema>
export type GatewayChatResponse = z.infer<typeof gatewayChatResponseSchema>
export type RouteSimulationRequest = z.infer<typeof routeSimulationRequestSchema>
export type RouteSimulationResponse = z.infer<typeof routeSimulationResponseSchema>
export type GatewayAnthropicMessageContentBlock = z.infer<
  typeof gatewayAnthropicMessageContentBlockSchema
>
export type GatewayAnthropicMessageContent = z.infer<
  typeof gatewayAnthropicMessageContentSchema
>
export type GatewayAnthropicMessage = z.infer<typeof gatewayAnthropicMessageSchema>
export type GatewayAnthropicMessagesRequest = z.infer<
  typeof gatewayAnthropicMessagesRequestSchema
>
export type GatewayAnthropicUsage = z.infer<typeof gatewayAnthropicUsageSchema>
export type GatewayAnthropicMessagesResponse = z.infer<
  typeof gatewayAnthropicMessagesResponseSchema
>
export type GatewayAnthropicMessagesError = z.infer<typeof gatewayAnthropicMessagesErrorSchema>
export type GatewayGeminiPart = z.infer<typeof gatewayGeminiPartSchema>
export type GatewayGeminiContent = z.infer<typeof gatewayGeminiContentSchema>
export type GatewayGeminiSystemInstruction = z.infer<typeof gatewayGeminiSystemInstructionSchema>
export type GatewayGeminiGenerationConfig = z.infer<typeof gatewayGeminiGenerationConfigSchema>
export type GatewayGeminiCandidate = z.infer<typeof gatewayGeminiCandidateSchema>
export type GatewayGeminiUsageMetadata = z.infer<typeof gatewayGeminiUsageMetadataSchema>
export type GatewayGeminiGenerateContentRequest = z.infer<
  typeof gatewayGeminiGenerateContentRequestSchema
>
export type GatewayGeminiGenerateContentResponse = z.infer<
  typeof gatewayGeminiGenerateContentResponseSchema
>
export type GatewayGeminiGenerateContentError = z.infer<
  typeof gatewayGeminiGenerateContentErrorSchema
>
export type GatewayGeminiRole = z.infer<typeof gatewayGeminiRoleSchema>
export type RouteReceiptDecisionTraceStep = z.infer<typeof routeReceiptDecisionTraceStepSchema>
export type RouteReceiptPolicyCheck = z.infer<typeof routeReceiptPolicyCheckSchema>
export type RouteReceiptProviderAttempt = z.infer<typeof routeReceiptProviderAttemptSchema>
export type RouteReceiptDiagnosticsResponse = z.infer<
  typeof routeReceiptDiagnosticsResponseSchema
>
export type RouteDiagnosticDecision = z.infer<typeof routeDiagnosticDecisionSchema>
export type RouteReceiptSummary = z.infer<typeof routeReceiptSummarySchema>
export type RouteDiagnosticTarget = z.infer<typeof routeDiagnosticTargetSchema>
export type RouteDiagnosticsResponse = z.infer<typeof routeDiagnosticsResponseSchema>
export type RouteReceiptsResponse = z.infer<typeof routeReceiptsResponseSchema>
export type AuthProvider = z.infer<typeof authProviderSchema>
export type OAuthProvider = z.infer<typeof oauthProviderSchema>
export type TenantSummary = z.infer<typeof tenantSummarySchema>
export type UserIdentity = z.infer<typeof userIdentitySchema>
export type TenantMembership = z.infer<typeof tenantMembershipSchema>
export type AuthProviderLink = z.infer<typeof authProviderLinkSchema>
export type AuthSession = z.infer<typeof authSessionSchema>
export type AuthProviderAvailability = z.infer<typeof authProviderAvailabilitySchema>
export type EmailLoginStartRequest = z.infer<typeof emailLoginStartRequestSchema>
export type EmailLoginStartResponse = z.infer<typeof emailLoginStartResponseSchema>
export type EmailLoginCompleteRequest = z.infer<typeof emailLoginCompleteRequestSchema>
export type OAuthLoginStartRequest = z.infer<typeof oauthLoginStartRequestSchema>
export type OAuthLoginStartResponse = z.infer<typeof oauthLoginStartResponseSchema>
export type OAuthCallbackRequest = z.infer<typeof oauthCallbackRequestSchema>
export type AuthLoginResult = z.infer<typeof authLoginResultSchema>
export type AuthSessionResponse = z.infer<typeof authSessionResponseSchema>
export type AuthProviderLinksResponse = z.infer<typeof authProviderLinksResponseSchema>
export type AuthProvidersResponse = z.infer<typeof authProvidersResponseSchema>
export type LogoutResponse = z.infer<typeof logoutResponseSchema>
export type UnlinkAuthProviderResponse = z.infer<typeof unlinkAuthProviderResponseSchema>
