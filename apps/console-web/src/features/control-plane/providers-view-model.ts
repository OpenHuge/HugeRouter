import type { ProviderResource } from "@huge-router/ts-shared-schema";
import type {
  CodexAuthAccountUploadInput,
  OAuthCarpoolMutationInput,
  OAuthSharingLeaseMutationInput,
} from "./service";
import type { ProviderResourceMutationInput } from "./providers-service";
import type { RouteReceiptView } from "./types";

export type ProviderFormState = {
  authKind: ProviderResourceMutationInput["authKind"];
  budgetPolicyId: string;
  credentialOwnerType: ProviderResourceMutationInput["credentialOwnerType"];
  deploymentScope: ProviderResourceMutationInput["deploymentScope"];
  endpointBaseUrl: string;
  healthState: ProviderResourceMutationInput["healthState"];
  name: string;
  projectId: string;
  providerId: string;
  providerResourceId: string;
  provenanceClass: ProviderResourceMutationInput["provenanceClass"];
  region: string;
  status: ProviderResourceMutationInput["status"];
  supportsJsonMode: boolean;
  supportsStreaming: boolean;
  supportsToolCalling: boolean;
};

export type ProviderFormErrors = Partial<Record<keyof ProviderFormState, string>>;

export type CodexAuthUploadState = {
  displayName: string;
  endpointBaseUrl: string;
  file: File | null;
  projectId: string;
  providerResourceId: string;
  region: string;
};

export type CodexAuthUploadErrors = Partial<Record<keyof CodexAuthUploadState, string>>;

export type SharingLeaseFormState = {
  borrowerWorkspaceId: string;
  expiresAt: string;
  leaseId: string;
  maxConcurrentRuns: number;
  policy: OAuthSharingLeaseMutationInput["policy"];
  poolId: string;
  provider: OAuthSharingLeaseMutationInput["provider"];
  status: OAuthSharingLeaseMutationInput["status"];
  turnBudget: number;
};

export type CarpoolFormState = {
  carpoolId: string;
  enabled: boolean;
  memberWorkspaceIds: string;
  name: string;
  perMemberConcurrencyLimit: number;
  perMemberTurnBudget: number;
  poolIds: string;
  provider: OAuthCarpoolMutationInput["provider"];
  strategy: OAuthCarpoolMutationInput["strategy"];
};

export type SharingFormErrors = Partial<
  Record<keyof SharingLeaseFormState | keyof CarpoolFormState, string>
>;

export const providerStatusOptions = [
  { label: "active", value: "active" },
  { label: "quarantined", value: "quarantined" },
  { label: "draining", value: "draining" },
  { label: "disabled", value: "disabled" },
  { label: "deleted", value: "deleted" },
] as const;

export const providerHealthOptions = [
  { label: "healthy", value: "healthy" },
  { label: "degraded", value: "degraded" },
  { label: "quarantined", value: "quarantined" },
  { label: "draining", value: "draining" },
  { label: "disabled", value: "disabled" },
] as const;

export const authKindOptions = [
  { label: "api_key", value: "api_key" },
  { label: "oauth_client_credentials", value: "oauth_client_credentials" },
  { label: "session_broker", value: "session_broker" },
] as const;

export const provenanceOptions = [
  { label: "official_api", value: "official_api" },
  { label: "official_gateway", value: "official_gateway" },
  { label: "byo_customer_credential", value: "byo_customer_credential" },
  { label: "dedicated_managed_account", value: "dedicated_managed_account" },
  { label: "shared_brokered_pool", value: "shared_brokered_pool" },
  { label: "unofficial_client_channel", value: "unofficial_client_channel" },
] as const;

export const credentialOwnerOptions = [
  { label: "platform", value: "platform" },
  { label: "tenant", value: "tenant" },
  { label: "project", value: "project" },
  { label: "partner", value: "partner" },
] as const;

export const deploymentScopeOptions = [
  { label: "shared", value: "shared" },
  { label: "tenant_dedicated", value: "tenant_dedicated" },
  { label: "project_dedicated", value: "project_dedicated" },
] as const;

export function createEmptyProviderForm(): ProviderFormState {
  return {
    authKind: "api_key",
    budgetPolicyId: "",
    credentialOwnerType: "platform",
    deploymentScope: "shared",
    endpointBaseUrl: "https://",
    healthState: "healthy",
    name: "",
    projectId: "",
    providerId: "",
    providerResourceId: "",
    provenanceClass: "official_api",
    region: "",
    status: "active",
    supportsJsonMode: true,
    supportsStreaming: true,
    supportsToolCalling: true,
  };
}

export function createEmptyCodexAuthUpload(): CodexAuthUploadState {
  return {
    displayName: "",
    endpointBaseUrl: "https://",
    file: null,
    projectId: "",
    providerResourceId: "",
    region: "global",
  };
}

export function defaultSharingExpiry() {
  return new Date(Date.now() + 1000 * 60 * 60 * 24 * 30).toISOString();
}

export function createEmptySharingLeaseForm(): SharingLeaseFormState {
  return {
    borrowerWorkspaceId: "tenant_acme",
    expiresAt: defaultSharingExpiry(),
    leaseId: "",
    maxConcurrentRuns: 1,
    policy: "fair_share",
    poolId: "",
    provider: "codex",
    status: "active",
    turnBudget: 20,
  };
}

export function createEmptyCarpoolForm(): CarpoolFormState {
  return {
    carpoolId: "",
    enabled: true,
    memberWorkspaceIds: "tenant_acme",
    name: "",
    perMemberConcurrencyLimit: 2,
    perMemberTurnBudget: 50,
    poolIds: "",
    provider: "codex",
    strategy: "fair_share",
  };
}

export function splitCsv(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function providerToFormState(provider: ProviderResource): ProviderFormState {
  return {
    authKind: provider.auth_kind,
    budgetPolicyId: provider.budget_policy_id ?? "",
    credentialOwnerType: provider.credential_owner_type,
    deploymentScope: provider.deployment_scope,
    endpointBaseUrl: provider.endpoint_base_url,
    healthState: provider.health_state,
    name: provider.name,
    projectId: provider.project_id ?? "",
    providerId: provider.provider_id,
    providerResourceId: provider.provider_resource_id,
    provenanceClass: provider.provenance_class,
    region: provider.region,
    status: provider.status,
    supportsJsonMode: provider.capabilities.supports_json_mode,
    supportsStreaming: provider.capabilities.supports_streaming,
    supportsToolCalling: provider.capabilities.supports_tool_calling,
  };
}

export function validateProviderForm(form: ProviderFormState) {
  const errors: ProviderFormErrors = {};

  if (!/^prvrsrc_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.providerResourceId)) {
    errors.providerResourceId =
      "Use an id that starts with prvrsrc_ and contains letters, digits, _ or -.";
  }

  if (!form.name.trim()) {
    errors.name = "Enter a provider resource name.";
  }

  if (!form.providerId.trim()) {
    errors.providerId = "Enter the upstream provider id.";
  }

  if (!form.region.trim()) {
    errors.region = "Enter a deployment region.";
  }

  if (!/^https:\/\/.+/.test(form.endpointBaseUrl.trim())) {
    errors.endpointBaseUrl = "Use an HTTPS endpoint URL.";
  }

  if (
    form.budgetPolicyId.trim() &&
    !/^budgetpol_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.budgetPolicyId)
  ) {
    errors.budgetPolicyId = "Budget policy ids must start with budgetpol_.";
  }

  return errors;
}

export function buildProviderMutationInput(
  form: ProviderFormState,
  editingProvider?: ProviderResource | null,
): ProviderResourceMutationInput {
  return {
    authKind: form.authKind,
    budgetPolicyId: form.budgetPolicyId.trim() || undefined,
    capabilities: {
      supportsJsonMode: form.supportsJsonMode,
      supportsStreaming: form.supportsStreaming,
      supportsToolCalling: form.supportsToolCalling,
    },
    credentialOwnerType: form.credentialOwnerType,
    createdAt: editingProvider?.created_at,
    deploymentScope: form.deploymentScope,
    endpointBaseUrl: form.endpointBaseUrl.trim(),
    healthState: form.healthState,
    name: form.name.trim(),
    projectId: form.projectId || undefined,
    providerId: form.providerId.trim(),
    providerResourceId: form.providerResourceId.trim(),
    provenanceClass: form.provenanceClass,
    region: form.region.trim(),
    status: form.status,
    version: editingProvider?.version,
  };
}

export function validateCodexAuthUpload(form: CodexAuthUploadState) {
  const errors: CodexAuthUploadErrors = {};

  if (!form.displayName.trim()) {
    errors.displayName = "Enter a display name.";
  }

  if (!form.file) {
    errors.file = "Choose a Codex auth.json file.";
  }

  if (
    form.providerResourceId.trim() &&
    !/^prvrsrc_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.providerResourceId.trim())
  ) {
    errors.providerResourceId = "Provider resource ids must start with prvrsrc_.";
  }

  if (!/^https:\/\/.+/.test(form.endpointBaseUrl.trim())) {
    errors.endpointBaseUrl = "Use an HTTPS reverse proxy endpoint.";
  }

  if (!form.region.trim()) {
    errors.region = "Enter a region label.";
  }

  return errors;
}

export function buildCodexAuthUploadInput(
  form: CodexAuthUploadState,
  authJson: unknown,
): CodexAuthAccountUploadInput {
  return {
    authJson,
    displayName: form.displayName.trim(),
    endpointBaseUrl: form.endpointBaseUrl.trim(),
    projectId: form.projectId || undefined,
    providerResourceId: form.providerResourceId.trim() || undefined,
    region: form.region.trim(),
  };
}

export function validateSharingLeaseForm(form: SharingLeaseFormState) {
  const errors: SharingFormErrors = {};

  if (!form.leaseId.trim()) {
    errors.leaseId = "Enter a lease id.";
  }

  if (!form.borrowerWorkspaceId.trim()) {
    errors.borrowerWorkspaceId = "Enter a borrower workspace id.";
  }

  if (!form.poolId.trim()) {
    errors.poolId = "Enter a pool id.";
  }

  if (!form.expiresAt.trim()) {
    errors.expiresAt = "Enter an expiration timestamp.";
  }

  return errors;
}

export function buildSharingLeaseInput(
  form: SharingLeaseFormState,
  startsAt = new Date().toISOString(),
): OAuthSharingLeaseMutationInput {
  return {
    allowedAccountIds: [],
    borrowerWorkspaceId: form.borrowerWorkspaceId.trim(),
    expiresAt: form.expiresAt.trim(),
    leaseId: form.leaseId.trim(),
    maxConcurrentRuns: form.maxConcurrentRuns,
    policy: form.policy,
    poolId: form.poolId.trim(),
    provider: form.provider,
    startsAt,
    status: form.status,
    turnBudget: form.turnBudget,
  };
}

export function validateCarpoolForm(form: CarpoolFormState) {
  const errors: SharingFormErrors = {};

  if (!form.carpoolId.trim()) {
    errors.carpoolId = "Enter a carpool id.";
  }

  if (!form.name.trim()) {
    errors.name = "Enter a carpool name.";
  }

  if (splitCsv(form.memberWorkspaceIds).length === 0) {
    errors.memberWorkspaceIds = "Enter at least one member workspace.";
  }

  if (splitCsv(form.poolIds).length === 0) {
    errors.poolIds = "Enter at least one pool id.";
  }

  return errors;
}

export function buildCarpoolInput(form: CarpoolFormState): OAuthCarpoolMutationInput {
  return {
    carpoolId: form.carpoolId.trim(),
    enabled: form.enabled,
    memberWorkspaceIds: splitCsv(form.memberWorkspaceIds),
    name: form.name.trim(),
    perMemberConcurrencyLimit: form.perMemberConcurrencyLimit,
    perMemberTurnBudget: form.perMemberTurnBudget,
    poolIds: splitCsv(form.poolIds),
    provider: form.provider,
    strategy: form.strategy,
  };
}

export function providerCapabilityLabels(provider: ProviderResource) {
  return [
    provider.capabilities.supports_streaming ? "streaming" : null,
    provider.capabilities.supports_tool_calling ? "tool_calling" : null,
    provider.capabilities.supports_json_mode ? "json_mode" : null,
    provider.capabilities.supports_realtime ? "realtime" : null,
    provider.capabilities.supports_response_model_metadata
      ? "response_model_metadata"
      : null,
  ].filter(Boolean) as string[];
}

export function latestSignalForProvider(
  provider: ProviderResource,
  routeReceipts: RouteReceiptView[],
  routePolicyNames: Map<string, string>,
) {
  const receipt = routeReceipts.find(
    (candidate) =>
      candidate.selectedTargetId === provider.provider_resource_id ||
      candidate.excludedTargets.some(
        (target) =>
          target.provider_resource_id === provider.provider_resource_id,
      ),
  );

  if (!receipt) {
    return null;
  }

  if (receipt.selectedTargetId === provider.provider_resource_id) {
    return `${routePolicyNames.get(receipt.routePolicyId) ?? receipt.routeName}: selected`;
  }

  return `${routePolicyNames.get(receipt.routePolicyId) ?? receipt.routeName}: ${receipt.excludedTargets[0]?.reason_code ?? "excluded"}`;
}
