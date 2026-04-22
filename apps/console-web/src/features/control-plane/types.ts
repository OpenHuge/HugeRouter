import type { NormalizedError, ProviderResource } from '@huge-router/ts-shared-schema'

type NormalizedRouteError = NormalizedError

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

export type RouteReceiptDiagnosticFallbackView = {
  fromProviderResourceId: string
  toProviderResourceId: string
  fromProviderLabel: string
  toProviderLabel: string
  reason: string
}

export type RouteReceiptDiagnosticExcludedTargetView = {
  providerResourceId: string
  providerLabel: string
  reason: string
}

export type RouteReceiptDiagnosticView = {
  admissionResult: string
  configSnapshotId: string
  createdAt: string
  decisionTimeline: {
    message: string
    notes: string[]
    score?: number
    stage: string
    status: string
  }[]
  excludedTargets: RouteReceiptDiagnosticExcludedTargetView[]
  fallbackTransitions: RouteReceiptDiagnosticFallbackView[]
  metadata: Record<string, string>
  modelAlias: string
  normalizedError?: NormalizedRouteError
  policyChecks: {
    policyId: string
    reason?: string
    status: string
  }[]
  providerAttempts: {
    attempt: number
    finishedAt: string
    latencyMs: number
    providerLabel: string
    providerResourceId: string
    reason: string
    startedAt: string
    status: string
  }[]
  protocolFamily: string
  routeReceiptId: string
  requestId: string
  selectedTargetLabel: string
  selectedTargetResourceId: string
  selectedTargetReason: string | null
  traceId: string
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

export type ConfigSnapshotView = {
  budgetPolicyId: string
  configSnapshotId: string
  activatedAt?: string
  providerResourceIds: string[]
  routePolicyId: string
  revision: number
  projectId: string
  status: 'active' | 'draft' | 'superseded'
  tenantId: string
}

export type ApiKeyView = {
  apiKeyId: string
  canRevoke: boolean
  createdAt: string
  displayName: string
  isActive: boolean
  keyPrefix: string
  providerResourceId: string
  tenantId?: string
  updatedAt: string
  version: number
}
