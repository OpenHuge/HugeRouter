import { z } from 'zod'

const dateTimeSchema = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}T.+Z$/, 'Expected an RFC3339 UTC timestamp')

const prefixedId = (prefix: string) =>
  z
    .string()
    .regex(new RegExp(`^${prefix}[A-Za-z0-9][A-Za-z0-9_-]*$`), `Expected id with prefix ${prefix}`)

const budgetPolicyIdSchema = prefixedId('budgetpol_')
const configSnapshotIdSchema = prefixedId('cfgsnap_')
const credentialIdSchema = prefixedId('cred_')
const projectIdSchema = prefixedId('proj_')
const providerResourceIdSchema = prefixedId('prvrsrc_')
const routePolicyIdSchema = prefixedId('routepol_')
const tenantIdSchema = prefixedId('tenant_')

export const openingGrantIdSchema = z.string().min(1)

export const ownerAccountIdSchema = z
  .string()
  .regex(/^[A-Za-z0-9_.-]{1,128}$/, 'Expected owner account id')

export const createOpeningGrantRequestSchema = z.object({
  config_snapshot_id: configSnapshotIdSchema,
  owner_account_id: ownerAccountIdSchema,
  grantee_kind: z.enum(['user', 'workspace', 'agent']),
  grantee_id: z.string().min(1),
  grantee_label: z.string().min(1).optional(),
  expires_at: dateTimeSchema,
  scopes: z.array(z.string().min(1)).default([]),
  credential_kind: z.literal('api_key').optional()
})

export const openingCredentialSchema = z.object({
  credential_kind: z.string().min(1),
  credential_id: credentialIdSchema,
  key_prefix: z.string().min(1),
  last_four: z.string().min(1),
  plaintext: z.string().min(1).optional()
})

export const openingGrantSchema = z.object({
  grant_id: openingGrantIdSchema,
  tenant_id: tenantIdSchema,
  project_id: projectIdSchema,
  owner_account_id: ownerAccountIdSchema,
  grantee_kind: z.string().min(1),
  grantee_id: z.string().min(1),
  grantee_label: z.string().min(1).optional(),
  config_snapshot_id: configSnapshotIdSchema,
  route_policy_id: routePolicyIdSchema,
  budget_policy_id: budgetPolicyIdSchema,
  provider_resource_ids: z.array(providerResourceIdSchema),
  credential_kind: z.string().min(1),
  credential_id: credentialIdSchema,
  credential_key_prefix: z.string().min(1),
  credential_last_four: z.string().min(1),
  scopes: z.array(z.string().min(1)),
  expires_at: dateTimeSchema,
  status: z.string().min(1),
  created_by: z.string().min(1),
  created_at: dateTimeSchema,
  updated_at: dateTimeSchema,
  version: z.number().int().nonnegative(),
  revoked_at: dateTimeSchema.optional(),
  revoked_by: z.string().min(1).optional()
})

export const openingGrantCreateResponseSchema = z.object({
  grant: openingGrantSchema,
  credential: openingCredentialSchema
})

export const openingGrantRevokeRequestSchema = z.object({
  expected_version: z.number().int().nonnegative()
})

export const openingGrantsResponseSchema = z.object({
  data: z.array(openingGrantSchema)
})

export type CreateOpeningGrantRequest = z.infer<typeof createOpeningGrantRequestSchema>
export type OpeningCredential = z.infer<typeof openingCredentialSchema>
export type OpeningGrant = z.infer<typeof openingGrantSchema>
export type OpeningGrantCreateResponse = z.infer<typeof openingGrantCreateResponseSchema>
export type OpeningGrantRevokeRequest = z.infer<typeof openingGrantRevokeRequestSchema>
export type OpeningGrantsResponse = z.infer<typeof openingGrantsResponseSchema>
