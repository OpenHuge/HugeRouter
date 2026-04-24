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

export type ControlPlaneMockState = {
  apiKeys: ApiKeyRecord[];
  billingExportJobs: BillingExportJobState[];
  billingExportPollCount: number;
  cardProducts: CardProductRecord[];
  configSnapshots: ConfigSnapshotRecord[];
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
    merchantShops: structuredClone(merchantShopsInitialState),
    providerResources: structuredClone(providerResourcesResponse.data),
    relayEvaluations: structuredClone(relayEvaluationsInitialState),
    replayCapsules: structuredClone(replayCapsulesInitialState),
    routePolicies: structuredClone(routePoliciesResponse.data),
    trialConnections: structuredClone(trialConnectionsInitialState),
  };
}
