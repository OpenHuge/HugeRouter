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

export const serviceNameSchema = z.string().min(1)
export const providerCapabilitySchema = z.object({
  supports_streaming: z.boolean(),
  supports_tool_calling: z.boolean(),
  supports_json_mode: z.boolean()
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
  endpoint_base_url: z.url().regex(/^https:\/\//, 'Expected an https endpoint'),
  auth_kind: z.enum(['api_key', 'oauth_client_credentials', 'session_broker']),
  health_state: z.enum(['healthy', 'degraded', 'quarantined', 'draining', 'disabled']),
  budget_policy_id: budgetPolicyIdSchema.optional(),
  capabilities: providerCapabilitySchema,
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
export type ChatRequest = z.infer<typeof chatRequestSchema>
export type GatewayChatRequest = z.infer<typeof gatewayChatRequestSchema>
export type GatewayChatResponse = z.infer<typeof gatewayChatResponseSchema>
export type RouteSimulationRequest = z.infer<typeof routeSimulationRequestSchema>
export type RouteSimulationResponse = z.infer<typeof routeSimulationResponseSchema>
