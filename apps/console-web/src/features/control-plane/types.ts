import type {
  ProviderResource,
  RouteDiagnosticsResponse,
  RouteReceipt,
} from "@huge-router/ts-shared-schema";

export type ProjectSummary = {
  id: string;
  name: string;
  slug: string;
};

export type OverviewData = {
  activeProviders: number;
  activeRoutes: number;
  activeSnapshotId: string;
  estimatedCostUsd: string;
  projects: ProjectSummary[];
  selectedProvider: string;
  tenantLabel: string;
  workspace: string;
};

export type RoutePolicyView = {
  id: string;
  modelAlias: string;
  name: string;
  preferredRegions: string[];
  protocolFamily: string;
  requiredCapabilities: string[];
  selectedProviders: string[];
  lastReceiptOutcome?: string;
  lastFailureReason?: string;
  lastReceiptId?: string;
};

export type ProviderRouteSignal = {
  createdAt: string;
  outcome: "selected" | "excluded";
  reason: string;
  receiptId: string;
  routeName: string;
};

export type ProviderResourceView = {
  id: string;
  name: string;
  providerId: string;
  region: string;
  scope: string;
  status: string;
  healthState: string;
  healthMessage?: string;
  quarantineReason?: string;
  provenanceClass: string;
  protocolFamilies: string[];
  isTransitGateway: boolean;
  capabilities: {
    streaming: boolean;
    realtime: boolean;
    toolCalling: boolean;
    responseModelMetadata: boolean;
  };
  latestRoutingSignal?: ProviderRouteSignal;
};

export type RouteReceiptView = {
  receiptId: string;
  routePolicyId: string;
  routeName: string;
  modelAlias: string;
  protocolFamily: string;
  createdAt: string;
  admissionResult: string;
  selectedTargetName?: string;
  failureReason?: string;
  excludedTargets: RouteReceipt["excluded_targets"];
  normalizedError?: RouteReceipt["normalized_error"];
  fallbackTransitions: RouteReceipt["fallback_transitions"];
  scoreBreakdown: RouteReceipt["score_breakdown"];
};

export type RouteDiagnosticsView = {
  activeSnapshotId?: string;
  activeSnapshotMatchesRoutePolicy: boolean;
  diagnostics: RouteDiagnosticsResponse;
};

export type TenantSummary = {
  activeConfigSnapshotId?: string;
  displayName: string;
  id: string;
  projectCount: number;
  providerCount: number;
  routePolicyCount: number;
  slug: string;
  updatedAt: string;
};

export type TenantDetail = {
  activeConfigSnapshotId?: string;
  displayName: string;
  estimatedCostUsd?: string;
  id: string;
  projects: ProjectSummary[];
  providers: ProviderResource[];
  routePolicies: RoutePolicyView[];
  selectedProvider: string;
  slug: string;
  updatedAt: string;
  version: number;
};
