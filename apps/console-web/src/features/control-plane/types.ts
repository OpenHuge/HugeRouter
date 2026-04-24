import type {
  NormalizedError,
  ProviderResource,
  RouteDiagnosticsResponse,
  RouteReceipt,
} from "@huge-router/ts-shared-schema";

type NormalizedRouteError = NormalizedError;

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
  createdAt: string;
  id: string;
  lastFailureReason?: string;
  lastReceiptId?: string;
  lastReceiptOutcome?: string;
  modelAlias: string;
  name: string;
  preferredRegions: string[];
  protocolFamily: string;
  requiredCapabilities: string[];
  selectedProviders: string[];
  tenantId: string;
  updatedAt: string;
  version: number;
};

export type RouteReceiptDiagnosticFallbackView = {
  fromProviderResourceId: string;
  toProviderResourceId: string;
  fromProviderLabel: string;
  toProviderLabel: string;
  reason: string;
};

export type RouteReceiptDiagnosticExcludedTargetView = {
  providerResourceId: string;
  providerLabel: string;
  reason: string;
};

export type RouteReceiptDiagnosticView = {
  admissionResult: string;
  configSnapshotId: string;
  createdAt: string;
  decisionTimeline: {
    message: string;
    notes: string[];
    score?: number;
    stage: string;
    status: string;
  }[];
  excludedTargets: RouteReceiptDiagnosticExcludedTargetView[];
  fallbackTransitions: RouteReceiptDiagnosticFallbackView[];
  metadata: Record<string, string>;
  modelAlias: string;
  normalizedError?: NormalizedRouteError;
  policyChecks: {
    policyId: string;
    reason?: string;
    status: string;
  }[];
  providerAttempts: {
    attempt: number;
    finishedAt: string;
    latencyMs: number;
    providerLabel: string;
    providerResourceId: string;
    reason: string;
    startedAt: string;
    status: string;
  }[];
  protocolFamily: string;
  routeReceiptId: string;
  requestId: string;
  selectedTargetLabel: string;
  selectedTargetResourceId: string;
  selectedTargetReason: string | null;
  traceId: string;
};

export type RouteReceiptView = {
  receiptId: string;
  routePolicyId: string;
  routeName: string;
  modelAlias: string;
  protocolFamily: string;
  createdAt: string;
  admissionResult: string;
  selectedTargetId?: string;
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

export type ConfigSnapshotView = {
  budgetPolicyId: string;
  configSnapshotId: string;
  activatedAt?: string;
  providerResourceIds: string[];
  routePolicyId: string;
  revision: number;
  projectId: string;
  status: "active" | "draft" | "superseded";
  tenantId: string;
};

export type ApiKeyView = {
  apiKeyId: string;
  canRevoke: boolean;
  createdAt: string;
  displayName: string;
  isActive: boolean;
  keyPrefix: string;
  providerResourceId: string;
  tenantId?: string;
  updatedAt: string;
  version: number;
};

export type ApiKeyCreateResult = {
  apiKeyId: string;
  displayName: string;
  keyPrefix: string;
  providerResourceId: string;
  version: number;
};

export type UsageBreakdownView = {
  billablePriceUsd: string;
  bucket: string;
  cachedInputTokens: number;
  inputTokens: number;
  modelAlias?: string;
  outputTokens: number;
  providerCostUsd: string;
  providerId?: string;
};

export type UsageDashboardData = {
  activeProjectId?: string;
  availableProjects: ProjectSummary[];
  breakdown: UsageBreakdownView[];
  eventCount: number;
  groupBy: "provider" | "model" | "day";
  inputTokens: number;
  outputTokens: number;
  cachedInputTokens: number;
  nextCursor?: string;
  providerCostUsd: string;
  billablePriceUsd: string;
  rangeLabel: string;
  windowStart: string;
  windowEnd: string;
};

export type BillingExportJobView = {
  completedAt?: string;
  errorMessage?: string;
  exportJobId: string;
  format: string;
  projectId?: string;
  requestedAt: string;
  status: string;
  tenantId?: string;
};

export type BillingDashboardData = {
  activeProjectId?: string;
  availableProjects: ProjectSummary[];
  billableTotalUsd: string;
  configuredBudgetUsd: string;
  exportJobs: BillingExportJobView[];
  lastProjectedAt: string;
  projectionLagSeconds: number;
  providerCostTotalUsd: string;
  rangeLabel: string;
  remainingBudgetUsd: string;
  thresholdStatus: string;
};

export type MerchantShopView = {
  merchantShopId: string;
  slug: string;
  displayName: string;
  status: "draft" | "active" | "suspended";
  announcement?: string;
  fulfillmentMode: "auto_card_secret";
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type CardProductView = {
  cardProductId: string;
  merchantShopId: string;
  title: string;
  description: string;
  status: "draft" | "active" | "sold_out";
  inventoryCount: number;
  faceValueUsd: string;
  retailPriceUsd: string;
  deliveryKind: "direct_secret";
  supportsTrial: boolean;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type TrialConnectionView = {
  trialConnectionId: string;
  providerLabel: string;
  endpointBaseUrl: string;
  apiKeyMasked: string;
  targetModel: string;
  status: "active" | "paused" | "needs_rotation";
  notes?: string;
  lastVerifiedAt?: string;
  createdAt: string;
  updatedAt: string;
  version: number;
};

export type RelayEvaluationView = {
  relayEvaluationId: string;
  trialConnectionId: string;
  replayCapsuleId: string;
  providerLabel: string;
  endpointBaseUrl: string;
  targetModel: string;
  runnerMode: "simulated" | "replay";
  sampleRequestCount: number;
  estimatedTokensSaved: number;
  overallScore: number;
  verdict: "healthy" | "warning" | "fail";
  fingerprintStatus: "pass" | "warning" | "fail" | "not_tested";
  protocolStatus: "pass" | "warning" | "fail" | "not_tested";
  tokenStatus: "pass" | "warning" | "fail" | "not_tested";
  multimodalStatus: "pass" | "warning" | "fail" | "not_tested";
  detectedChannel?: string;
  summary: string;
  createdAt: string;
};

export type MerchantWorkspaceData = {
  merchantEnabled: boolean;
  tenantId: string;
  shops: MerchantShopView[];
  cardProducts: CardProductView[];
  trialConnections: TrialConnectionView[];
  recentEvaluations: RelayEvaluationView[];
};

export type ReplayCapsuleView = {
  replayCapsuleId: string;
  requestId: string;
  traceId: string;
  routeReceiptId: string;
  configSnapshotId: string;
  redactionTier:
    | "metadata_only"
    | "structured_redacted"
    | "full_payload_retention";
  normalizedRequestSummary: {
    protocolFamily: string;
    modelAlias: string;
    estimatedPromptTokens: number;
  };
  upstreamErrorCode?: string;
};
