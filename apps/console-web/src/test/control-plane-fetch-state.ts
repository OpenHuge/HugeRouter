import {
  apiKeysInitialState,
  billingExportResponse,
  configSnapshotsResponse,
  providerResourcesResponse,
  routePoliciesResponse,
  type BillingExportJobState,
} from "./control-plane-fetch-fixtures";
import {
  cardProductsInitialState,
  merchantShopsInitialState,
  relayEvaluationsInitialState,
  replayCapsulesInitialState,
  trialConnectionsInitialState,
  type CardProductRecord,
  type MerchantShopRecord,
  type RelayEvaluationRecord,
  type ReplayCapsuleRecord,
  type TrialConnectionRecord,
} from "./control-plane-fetch-merchant-fixtures";

export type ProviderResourceRecord =
  (typeof providerResourcesResponse.data)[number];
export type RoutePolicyRecord = (typeof routePoliciesResponse.data)[number];
export type ConfigSnapshotRecord =
  (typeof configSnapshotsResponse.data)[number];
export type ApiKeyRecord = (typeof apiKeysInitialState)[number];
export type OAuthSharingLeaseRecord = {
  allowed_account_ids?: string[];
  borrower_workspace_id: string;
  created_at: string;
  expires_at: string;
  lease_id: string;
  max_concurrent_runs: number;
  policy: string;
  pool_id: string;
  provider: string;
  starts_at: string;
  status: string;
  updated_at: string;
  usage_budget?: {
    turns?: number;
  };
};
export type OAuthCarpoolRecord = {
  carpool_id: string;
  created_at: string;
  enabled: boolean;
  member_workspace_ids: string[];
  name: string;
  per_member_concurrency_limit?: number;
  per_member_turn_budget?: number;
  pool_ids: string[];
  provider: string;
  strategy: string;
  updated_at: string;
};
export type OAuthSharingAuditEventRecord = {
  audit_event_id: string;
  created_at: string;
  event_type: string;
  provider: string;
  reason: string;
};

export type ControlPlaneMockState = {
  apiKeys: ApiKeyRecord[];
  billingExportJobs: BillingExportJobState[];
  billingExportPollCount: number;
  cardProducts: CardProductRecord[];
  configSnapshots: ConfigSnapshotRecord[];
  oauthCarpools: OAuthCarpoolRecord[];
  oauthSharingAuditEvents: OAuthSharingAuditEventRecord[];
  oauthSharingLeases: OAuthSharingLeaseRecord[];
  merchantShops: MerchantShopRecord[];
  providerResources: ProviderResourceRecord[];
  relayEvaluations: RelayEvaluationRecord[];
  replayCapsules: ReplayCapsuleRecord[];
  routePolicies: RoutePolicyRecord[];
  trialConnections: TrialConnectionRecord[];
};

export function createInitialControlPlaneMockState(): ControlPlaneMockState {
  return {
    apiKeys: [...apiKeysInitialState],
    billingExportJobs: [structuredClone(billingExportResponse.data)],
    billingExportPollCount: 0,
    cardProducts: structuredClone(cardProductsInitialState),
    configSnapshots: structuredClone(configSnapshotsResponse.data),
    oauthCarpools: [
      {
        carpool_id: "carpool_codex_acme",
        created_at: "2026-04-22T00:00:00Z",
        enabled: true,
        member_workspace_ids: ["tenant_acme"],
        name: "Acme Codex Team Share",
        per_member_concurrency_limit: 2,
        per_member_turn_budget: 50,
        pool_ids: ["prvrsrc_codex_team"],
        provider: "codex",
        strategy: "fair_share",
        updated_at: "2026-04-22T00:00:00Z",
      },
    ],
    oauthSharingAuditEvents: [],
    oauthSharingLeases: [
      {
        borrower_workspace_id: "tenant_acme",
        created_at: "2026-04-22T00:00:00Z",
        expires_at: "2026-05-22T00:00:00Z",
        lease_id: "lease_codex_acme",
        max_concurrent_runs: 2,
        policy: "fair_share",
        pool_id: "prvrsrc_codex_team",
        provider: "codex",
        starts_at: "2026-04-22T00:00:00Z",
        status: "active",
        updated_at: "2026-04-22T00:00:00Z",
        usage_budget: {
          turns: 20,
        },
      },
    ],
    merchantShops: structuredClone(merchantShopsInitialState),
    providerResources: structuredClone(providerResourcesResponse.data),
    relayEvaluations: structuredClone(relayEvaluationsInitialState),
    replayCapsules: structuredClone(replayCapsulesInitialState),
    routePolicies: structuredClone(routePoliciesResponse.data),
    trialConnections: structuredClone(trialConnectionsInitialState),
  };
}
