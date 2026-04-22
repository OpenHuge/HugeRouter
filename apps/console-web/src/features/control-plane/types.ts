import type { ProviderResource } from '@huge-router/ts-shared-schema'

export type ProjectSummary = {
  id: string
  name: string
  slug: string
}

export type OverviewData = {
  activeProviders: number
  activeRoutes: number
  activeSnapshotId: string
  estimatedCostUsd: string
  projects: ProjectSummary[]
  selectedProvider: string
  tenantLabel: string
  workspace: string
}

export type RoutePolicyView = {
  id: string
  modelAlias: string
  name: string
  preferredRegions: string[]
  protocolFamily: string
  requiredCapabilities: string[]
  selectedProviders: string[]
}

export type TenantSummary = {
  activeConfigSnapshotId?: string
  displayName: string
  id: string
  projectCount: number
  providerCount: number
  routePolicyCount: number
  slug: string
  updatedAt: string
}

export type TenantDetail = {
  activeConfigSnapshotId?: string
  displayName: string
  estimatedCostUsd?: string
  id: string
  projects: ProjectSummary[]
  providers: ProviderResource[]
  routePolicies: RoutePolicyView[]
  selectedProvider: string
  slug: string
  updatedAt: string
  version: number
}
