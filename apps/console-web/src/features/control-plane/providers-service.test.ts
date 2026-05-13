import { describe, expect, it } from "vitest";
import {
  createProviderResourcesService,
  toProviderResourcePayload,
  type ProviderResourceMutationInput,
} from "./providers-service";

const providerInput: ProviderResourceMutationInput = {
  authKind: "api_key",
  budgetPolicyId: "budgetpol_default",
  capabilities: {
    supportsJsonMode: true,
    supportsStreaming: true,
    supportsToolCalling: false,
  },
  credentialOwnerType: "platform",
  deploymentScope: "shared",
  endpointBaseUrl: "https://provider.example.com",
  healthState: "healthy",
  name: "OpenAI Primary",
  projectId: "proj_core",
  providerId: "openai",
  providerResourceId: "prvrsrc_openai_primary",
  provenanceClass: "official_api",
  region: "global",
  status: "active",
};

const timestamp = "2026-05-02T00:00:00Z";

function parseJsonBody(init?: RequestInit) {
  if (typeof init?.body !== "string") {
    throw new Error("Expected a JSON string request body.");
  }

  return JSON.parse(init.body) as Record<string, unknown>;
}

describe("provider resources service", () => {
  it("builds provider resource payloads with tenant ownership and defaults", () => {
    expect(
      toProviderResourcePayload(providerInput, {
        getActiveTenantId: () => "tenant_acme",
        timestamp: () => timestamp,
      }),
    ).toEqual({
      auth_kind: "api_key",
      budget_policy_id: "budgetpol_default",
      capabilities: {
        supports_json_mode: true,
        supports_realtime: false,
        supports_response_model_metadata: true,
        supports_streaming: true,
        supports_tool_calling: false,
      },
      created_at: timestamp,
      credential_owner_type: "platform",
      deployment_scope: "shared",
      endpoint_base_url: "https://provider.example.com",
      health_state: "healthy",
      is_transit_gateway: false,
      name: "OpenAI Primary",
      project_id: "proj_core",
      provider_id: "openai",
      provider_resource_id: "prvrsrc_openai_primary",
      provenance_class: "official_api",
      region: "global",
      status: "active",
      supported_protocol_families: ["openai_chat"],
      tenant_id: "tenant_acme",
      updated_at: timestamp,
      version: 1,
    });
  });

  it("fails before issuing provider mutations without an active tenant", () => {
    expect(() =>
      toProviderResourcePayload(providerInput, {
        getActiveTenantId: () => undefined,
        timestamp: () => timestamp,
      }),
    ).toThrow("tenant_not_found");
  });

  it("sends provider create, update, and disable requests through the injected transport", async () => {
    const calls: { init?: RequestInit; path: string }[] = [];
    const responsePayload = toProviderResourcePayload(providerInput, {
      getActiveTenantId: () => "tenant_acme",
      timestamp: () => timestamp,
    });
    const service = createProviderResourcesService({
      getActiveTenantId: () => "tenant_acme",
      requestControlPlaneJson: async (path, parser, init) => {
        calls.push({ init, path });

        return parser(responsePayload);
      },
      timestamp: () => timestamp,
    });

    await service.createProviderResource(providerInput);
    await service.updateProviderResource(
      "prvrsrc_openai_primary",
      { ...providerInput, version: 3 },
      3,
    );
    await service.disableProviderResource("prvrsrc_openai_primary", 3);

    expect(calls.map((call) => [call.path, call.init?.method])).toEqual([
      ["/v1/provider-resources", "POST"],
      ["/v1/provider-resources/prvrsrc_openai_primary", "PUT"],
      ["/v1/provider-resources/prvrsrc_openai_primary/disable", "POST"],
    ]);
    expect(parseJsonBody(calls[1]?.init)).toMatchObject({
      expected_version: 3,
      provider_resource_id: "prvrsrc_openai_primary",
      tenant_id: "tenant_acme",
      version: 3,
    });
    expect(parseJsonBody(calls[2]?.init)).toEqual({
      expected_version: 3,
    });
  });
});
