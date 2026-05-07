import {
  providerResourceSchema,
  type ProviderResource,
} from "@huge-router/ts-shared-schema";

export type ProviderResourceMutationInput = {
  authKind: "api_key" | "oauth_client_credentials" | "session_broker";
  budgetPolicyId?: string;
  capabilities: {
    supportsJsonMode: boolean;
    supportsStreaming: boolean;
    supportsToolCalling: boolean;
  };
  credentialOwnerType: "platform" | "tenant" | "project" | "partner";
  createdAt?: string;
  deploymentScope: "shared" | "tenant_dedicated" | "project_dedicated";
  endpointBaseUrl: string;
  healthState: "healthy" | "degraded" | "quarantined" | "draining" | "disabled";
  name: string;
  projectId?: string;
  providerId: string;
  providerResourceId: string;
  provenanceClass:
    | "official_api"
    | "official_gateway"
    | "byo_customer_credential"
    | "dedicated_managed_account"
    | "shared_brokered_pool"
    | "unofficial_client_channel";
  region: string;
  status: "active" | "disabled" | "draining" | "quarantined" | "deleted";
  version?: number;
};

type ControlPlaneJsonRequester = <T>(
  path: string,
  parser: (payload: unknown) => T,
  init?: RequestInit,
) => Promise<T>;

export type ProviderResourcesService = {
  createProviderResource: (
    providerResource: ProviderResourceMutationInput,
  ) => Promise<ProviderResource>;
  updateProviderResource: (
    providerResourceId: string,
    providerResource: ProviderResourceMutationInput,
    expectedVersion: number,
  ) => Promise<ProviderResource>;
  disableProviderResource: (
    providerResourceId: string,
    expectedVersion: number,
  ) => Promise<ProviderResource>;
};

export type ProviderResourcesServiceDependencies = {
  getActiveTenantId: () => string | null | undefined;
  requestControlPlaneJson: ControlPlaneJsonRequester;
  timestamp: () => string;
};

function parseProviderResourceRecord(payload: unknown) {
  return providerResourceSchema.parse(payload);
}

function requireActiveTenantId(
  getActiveTenantId: () => string | null | undefined,
) {
  const tenantId = getActiveTenantId();

  if (!tenantId) {
    throw new Error("tenant_not_found");
  }

  return tenantId;
}

export function toProviderResourcePayload(
  input: ProviderResourceMutationInput,
  dependencies: Pick<
    ProviderResourcesServiceDependencies,
    "getActiveTenantId" | "timestamp"
  >,
): ProviderResource {
  return {
    auth_kind: input.authKind,
    budget_policy_id: input.budgetPolicyId,
    capabilities: {
      supports_json_mode: input.capabilities.supportsJsonMode,
      supports_realtime: false,
      supports_response_model_metadata: true,
      supports_streaming: input.capabilities.supportsStreaming,
      supports_tool_calling: input.capabilities.supportsToolCalling,
    },
    created_at: input.createdAt ?? dependencies.timestamp(),
    credential_owner_type: input.credentialOwnerType,
    deployment_scope: input.deploymentScope,
    endpoint_base_url: input.endpointBaseUrl,
    health_state: input.healthState,
    is_transit_gateway: false,
    name: input.name,
    project_id: input.projectId,
    provider_id: input.providerId,
    provider_resource_id: input.providerResourceId,
    provenance_class: input.provenanceClass,
    region: input.region,
    status: input.status,
    supported_protocol_families: ["openai_chat"],
    tenant_id: requireActiveTenantId(dependencies.getActiveTenantId),
    updated_at: dependencies.timestamp(),
    version: input.version ?? 1,
  };
}

export function createProviderResourcesService(
  dependencies: ProviderResourcesServiceDependencies,
): ProviderResourcesService {
  return {
    createProviderResource(providerResource) {
      return dependencies.requestControlPlaneJson(
        "/v1/provider-resources",
        parseProviderResourceRecord,
        {
          body: JSON.stringify(
            toProviderResourcePayload(providerResource, dependencies),
          ),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },

    updateProviderResource(
      providerResourceId,
      providerResource,
      expectedVersion,
    ) {
      return dependencies.requestControlPlaneJson(
        `/v1/provider-resources/${encodeURIComponent(providerResourceId)}`,
        parseProviderResourceRecord,
        {
          body: JSON.stringify({
            ...toProviderResourcePayload(providerResource, dependencies),
            expected_version: expectedVersion,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "PUT",
        },
      );
    },

    disableProviderResource(providerResourceId, expectedVersion) {
      return dependencies.requestControlPlaneJson(
        `/v1/provider-resources/${encodeURIComponent(providerResourceId)}/disable`,
        parseProviderResourceRecord,
        {
          body: JSON.stringify({
            expected_version: expectedVersion,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },
  };
}
