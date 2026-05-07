import { z } from 'zod'

const prefixedId = (prefix: string) =>
  z
    .string()
    .regex(new RegExp(`^${prefix}[A-Za-z0-9][A-Za-z0-9_-]*$`), `Expected id with prefix ${prefix}`)

const dateTimeSchema = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}T.+Z$/, 'Expected an RFC3339 UTC timestamp')

const slugSchema = z
  .string()
  .regex(/^[a-z0-9-]+$/, 'Expected a lowercase slug with digits or hyphens')

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

const tenantIdSchema = prefixedId('tenant_')
const configSnapshotIdSchema = prefixedId('cfgsnap_')
const routeReceiptIdSchema = prefixedId('routercpt_')
export const merchantShopIdSchema = prefixedId('mshop_')
export const cardProductIdSchema = prefixedId('cardprod_')
export const trialConnectionIdSchema = prefixedId('trialconn_')
export const relayEvaluationIdSchema = prefixedId('reval_')
export const replayCapsuleIdSchema = prefixedId('replay_')
export const merchantShopStatusSchema = z.enum(['draft', 'active', 'suspended'])
export const merchantFulfillmentModeSchema = z.enum(['auto_card_secret'])
export const cardProductStatusSchema = z.enum(['draft', 'active', 'sold_out'])
export const cardDeliveryKindSchema = z.enum(['direct_secret'])
export const trialConnectionStatusSchema = z.enum(['active', 'paused', 'needs_rotation'])
export const relayEvaluationRunnerModeSchema = z.enum(['simulated', 'replay'])
export const relayEvaluationVerdictSchema = z.enum(['healthy', 'warning', 'fail'])
export const relayCheckStatusSchema = z.enum(['pass', 'warning', 'fail', 'not_tested'])

export const merchantShopSchema = z.object({
  merchant_shop_id: merchantShopIdSchema,
  tenant_id: tenantIdSchema,
  slug: slugSchema,
  display_name: z.string().min(1),
  status: merchantShopStatusSchema,
  announcement: z.string().min(1).optional(),
  fulfillment_mode: merchantFulfillmentModeSchema,
  version: z.number().int().nonnegative(),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema
})

export const cardProductSchema = z.object({
  card_product_id: cardProductIdSchema,
  tenant_id: tenantIdSchema,
  merchant_shop_id: merchantShopIdSchema,
  title: z.string().min(1),
  description: z.string().min(1),
  status: cardProductStatusSchema,
  inventory_count: z.number().int().nonnegative(),
  face_value_usd: z.string().regex(/^\d+(\.\d+)?$/),
  retail_price_usd: z.string().regex(/^\d+(\.\d+)?$/),
  delivery_kind: cardDeliveryKindSchema,
  supports_trial: z.boolean(),
  version: z.number().int().nonnegative(),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema
})

export const trialConnectionSchema = z.object({
  trial_connection_id: trialConnectionIdSchema,
  tenant_id: tenantIdSchema,
  provider_label: z.string().min(1),
  endpoint_base_url: endpointBaseUrlSchema,
  api_key_masked: z.string().min(1),
  target_model: z.string().min(1),
  status: trialConnectionStatusSchema,
  notes: z.string().min(1).optional(),
  last_verified_at: dateTimeSchema.optional(),
  version: z.number().int().nonnegative(),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema
})

export const replayCapsuleSchema = z.object({
  replay_capsule_id: replayCapsuleIdSchema,
  request_id: z.string().min(1),
  trace_id: z.string().min(1),
  route_receipt_id: routeReceiptIdSchema,
  config_snapshot_id: configSnapshotIdSchema,
  redaction_tier: z.enum(['metadata_only', 'structured_redacted', 'full_payload_retention']),
  normalized_request_summary: z.object({
    protocol_family: z.string().min(1),
    model_alias: z.string().min(1),
    estimated_prompt_tokens: z.number().int().nonnegative()
  }),
  upstream_error_summary: z
    .object({
      code: z.string().min(1)
    })
    .optional()
})

export const relayEvaluationSchema = z.object({
  relay_evaluation_id: relayEvaluationIdSchema,
  tenant_id: tenantIdSchema,
  trial_connection_id: trialConnectionIdSchema,
  replay_capsule_id: replayCapsuleIdSchema,
  provider_label: z.string().min(1),
  endpoint_base_url: endpointBaseUrlSchema,
  target_model: z.string().min(1),
  runner_mode: relayEvaluationRunnerModeSchema,
  sample_request_count: z.number().int().nonnegative(),
  estimated_tokens_saved: z.number().int().nonnegative(),
  overall_score: z.number().int().min(0).max(100),
  verdict: relayEvaluationVerdictSchema,
  fingerprint_status: relayCheckStatusSchema,
  protocol_status: relayCheckStatusSchema,
  token_status: relayCheckStatusSchema,
  multimodal_status: relayCheckStatusSchema,
  detected_channel: z.string().min(1).optional(),
  summary: z.string().min(1),
  created_at: dateTimeSchema
})

export const merchantWorkspaceSchema = z.object({
  merchant_enabled: z.boolean(),
  tenant_id: tenantIdSchema,
  shops: z.array(merchantShopSchema),
  card_products: z.array(cardProductSchema),
  trial_connections: z.array(trialConnectionSchema),
  recent_evaluations: z.array(relayEvaluationSchema)
})

export const replayCapsuleResponseSchema = z.object({
  replay_capsule: replayCapsuleSchema
})

export const merchantWorkspaceResponseSchema = z.object({
  data: merchantWorkspaceSchema
})

export type MerchantShop = z.infer<typeof merchantShopSchema>
export type CardProduct = z.infer<typeof cardProductSchema>
export type TrialConnection = z.infer<typeof trialConnectionSchema>
export type ReplayCapsule = z.infer<typeof replayCapsuleSchema>
export type RelayEvaluation = z.infer<typeof relayEvaluationSchema>
export type MerchantWorkspace = z.infer<typeof merchantWorkspaceSchema>
export type ReplayCapsuleResponse = z.infer<typeof replayCapsuleResponseSchema>
export type MerchantWorkspaceResponse = z.infer<typeof merchantWorkspaceResponseSchema>
