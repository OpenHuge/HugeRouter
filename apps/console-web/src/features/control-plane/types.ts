import { z } from 'zod'

export const projectSummarySchema = z.object({
  id: z.string(),
  name: z.string()
})

export const providerResourceSchema = z.object({
  id: z.string(),
  name: z.string(),
  provider: z.string(),
  health: z.enum(['degraded', 'healthy', 'warning']),
  region: z.string(),
  scope: z.enum(['shared', 'tenant']),
  status: z.enum(['active', 'quarantined'])
})

export const routePolicySchema = z.object({
  id: z.string(),
  name: z.string(),
  modelAlias: z.string(),
  status: z.enum(['active', 'draft']),
  successRate: z.number(),
  selectedProvider: z.string(),
  tenantId: z.string()
})

export const tenantSummarySchema = z.object({
  id: z.string(),
  slug: z.string(),
  displayName: z.string(),
  activeRoutePolicies: z.number(),
  monthlySpendUsd: z.number(),
  plan: z.enum(['enterprise', 'growth']),
  projectCount: z.number(),
  status: z.enum(['healthy', 'needs_attention'])
})

export const tenantDetailSchema = tenantSummarySchema.extend({
  notes: z.string(),
  primaryRegion: z.string(),
  projects: z.array(projectSummarySchema),
  providers: z.array(providerResourceSchema),
  routePolicies: z.array(routePolicySchema)
})

export const overviewDataSchema = z.object({
  activeProviders: z.number(),
  activeRoutes: z.number(),
  monthlySpendUsd: z.number(),
  projects: z.array(projectSummarySchema),
  tenantLabel: z.string(),
  workspace: z.string()
})

export type OverviewData = z.infer<typeof overviewDataSchema>
export type ProjectSummary = z.infer<typeof projectSummarySchema>
export type ProviderResource = z.infer<typeof providerResourceSchema>
export type RoutePolicy = z.infer<typeof routePolicySchema>
export type TenantSummary = z.infer<typeof tenantSummarySchema>
export type TenantDetail = z.infer<typeof tenantDetailSchema>
