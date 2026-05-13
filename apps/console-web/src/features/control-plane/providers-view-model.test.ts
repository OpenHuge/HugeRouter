import type { ProviderResource } from "@huge-router/ts-shared-schema";
import { describe, expect, it } from "vitest";
import {
  buildCarpoolInput,
  buildCodexAuthUploadInput,
  buildProviderMutationInput,
  buildSharingLeaseInput,
  createEmptyCarpoolForm,
  createEmptyCodexAuthUpload,
  createEmptyProviderForm,
  createEmptySharingLeaseForm,
  latestSignalForProvider,
  providerCapabilityLabels,
  providerToFormState,
  splitCsv,
  validateCarpoolForm,
  validateCodexAuthUpload,
  validateProviderForm,
  validateSharingLeaseForm,
} from "./providers-view-model";
import type { RouteReceiptView } from "./types";

const provider: ProviderResource = {
  auth_kind: "api_key",
  budget_policy_id: "budgetpol_default",
  capabilities: {
    supports_json_mode: true,
    supports_realtime: false,
    supports_response_model_metadata: true,
    supports_streaming: true,
    supports_tool_calling: true,
  },
  created_at: "2026-05-01T00:00:00Z",
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
  updated_at: "2026-05-02T00:00:00Z",
  version: 3,
};

function receipt(overrides: Partial<RouteReceiptView>): RouteReceiptView {
  return {
    admissionResult: "accepted",
    createdAt: "2026-05-02T00:00:00Z",
    excludedTargets: [],
    fallbackTransitions: [],
    modelAlias: "gpt-5.1",
    protocolFamily: "openai_chat",
    receiptId: "rtrcp_001",
    routeName: "Default chat",
    routePolicyId: "routepol_default_chat",
    scoreBreakdown: {
      cost: 1,
      health: 1,
      latency: 1,
      trust: 1,
    },
    ...overrides,
  };
}

describe("providers view model", () => {
  it("maps provider resources to editable form state and mutation input", () => {
    const form = providerToFormState(provider);

    expect(form).toMatchObject({
      budgetPolicyId: "budgetpol_default",
      name: "OpenAI Primary",
      providerResourceId: "prvrsrc_openai_primary",
      supportsJsonMode: true,
      supportsStreaming: true,
      supportsToolCalling: true,
    });

    expect(
      buildProviderMutationInput(
        {
          ...form,
          budgetPolicyId: " ",
          endpointBaseUrl: " https://new-provider.example.com ",
          name: " New name ",
          projectId: "",
          providerId: " openai-compatible ",
          providerResourceId: " prvrsrc_openai_primary ",
          region: " us-east ",
        },
        provider,
      ),
    ).toEqual({
      authKind: "api_key",
      budgetPolicyId: undefined,
      capabilities: {
        supportsJsonMode: true,
        supportsStreaming: true,
        supportsToolCalling: true,
      },
      createdAt: "2026-05-01T00:00:00Z",
      credentialOwnerType: "platform",
      deploymentScope: "shared",
      endpointBaseUrl: "https://new-provider.example.com",
      healthState: "healthy",
      name: "New name",
      projectId: undefined,
      providerId: "openai-compatible",
      providerResourceId: "prvrsrc_openai_primary",
      provenanceClass: "official_api",
      region: "us-east",
      status: "active",
      version: 3,
    });
  });

  it("validates provider and Codex auth upload forms before service calls", () => {
    expect(
      validateProviderForm({
        ...createEmptyProviderForm(),
        budgetPolicyId: "bad_budget",
        endpointBaseUrl: "http://provider.example.com",
        name: "",
        providerId: "",
        providerResourceId: "bad-id",
        region: "",
      }),
    ).toMatchObject({
      budgetPolicyId: "Budget policy ids must start with budgetpol_.",
      endpointBaseUrl: "Use an HTTPS endpoint URL.",
      name: "Enter a provider resource name.",
      providerId: "Enter the upstream provider id.",
      providerResourceId:
        "Use an id that starts with prvrsrc_ and contains letters, digits, _ or -.",
      region: "Enter a deployment region.",
    });

    expect(
      validateCodexAuthUpload({
        ...createEmptyCodexAuthUpload(),
        displayName: "",
        endpointBaseUrl: "http://proxy.example.com",
        providerResourceId: "bad-id",
        region: "",
      }),
    ).toMatchObject({
      displayName: "Enter a display name.",
      endpointBaseUrl: "Use an HTTPS reverse proxy endpoint.",
      file: "Choose a Codex auth.json file.",
      providerResourceId: "Provider resource ids must start with prvrsrc_.",
      region: "Enter a region label.",
    });

    expect(
      buildCodexAuthUploadInput(
        {
          ...createEmptyCodexAuthUpload(),
          displayName: " Shared Codex ",
          endpointBaseUrl: " https://codex.example.com ",
          projectId: "",
          providerResourceId: " prvrsrc_codex ",
          region: " global ",
        },
        { refresh_token: "secret" },
      ),
    ).toEqual({
      authJson: { refresh_token: "secret" },
      displayName: "Shared Codex",
      endpointBaseUrl: "https://codex.example.com",
      projectId: undefined,
      providerResourceId: "prvrsrc_codex",
      region: "global",
    });
  });

  it("builds sharing lease and carpool inputs from normalized form state", () => {
    expect(splitCsv(" tenant_a, tenant_b ,, tenant_c ")).toEqual([
      "tenant_a",
      "tenant_b",
      "tenant_c",
    ]);

    expect(
      validateSharingLeaseForm({
        ...createEmptySharingLeaseForm(),
        borrowerWorkspaceId: "",
        expiresAt: "",
        leaseId: "",
        poolId: "",
      }),
    ).toMatchObject({
      borrowerWorkspaceId: "Enter a borrower workspace id.",
      expiresAt: "Enter an expiration timestamp.",
      leaseId: "Enter a lease id.",
      poolId: "Enter a pool id.",
    });

    expect(
      buildSharingLeaseInput(
        {
          ...createEmptySharingLeaseForm(),
          borrowerWorkspaceId: " tenant_borrower ",
          expiresAt: " 2027-05-02T00:00:00Z ",
          leaseId: " lease_001 ",
          poolId: " pool_001 ",
          turnBudget: 15,
        },
        "2026-05-02T00:00:00Z",
      ),
    ).toMatchObject({
      borrowerWorkspaceId: "tenant_borrower",
      expiresAt: "2027-05-02T00:00:00Z",
      leaseId: "lease_001",
      poolId: "pool_001",
      startsAt: "2026-05-02T00:00:00Z",
      turnBudget: 15,
    });

    expect(
      validateCarpoolForm({
        ...createEmptyCarpoolForm(),
        carpoolId: "",
        memberWorkspaceIds: " , ",
        name: "",
        poolIds: "",
      }),
    ).toMatchObject({
      carpoolId: "Enter a carpool id.",
      memberWorkspaceIds: "Enter at least one member workspace.",
      name: "Enter a carpool name.",
      poolIds: "Enter at least one pool id.",
    });

    expect(
      buildCarpoolInput({
        ...createEmptyCarpoolForm(),
        carpoolId: " carpool_001 ",
        memberWorkspaceIds: "tenant_a, tenant_b",
        name: " Shared pool ",
        poolIds: "pool_a, pool_b",
      }),
    ).toMatchObject({
      carpoolId: "carpool_001",
      memberWorkspaceIds: ["tenant_a", "tenant_b"],
      name: "Shared pool",
      poolIds: ["pool_a", "pool_b"],
    });
  });

  it("derives provider capability labels and latest traffic signals", () => {
    const routePolicyNames = new Map([["routepol_default_chat", "Default chat"]]);

    expect(providerCapabilityLabels(provider)).toEqual([
      "streaming",
      "tool_calling",
      "json_mode",
      "response_model_metadata",
    ]);

    expect(
      latestSignalForProvider(
        provider,
        [receipt({ selectedTargetId: provider.provider_resource_id })],
        routePolicyNames,
      ),
    ).toBe("Default chat: selected");

    expect(
      latestSignalForProvider(
        provider,
        [
          receipt({
            excludedTargets: [
              {
                provider_resource_id: provider.provider_resource_id,
                reason: "region mismatch",
                reason_code: "region_mismatch",
              },
            ],
          }),
        ],
        routePolicyNames,
      ),
    ).toBe("Default chat: region_mismatch");

    expect(latestSignalForProvider(provider, [], routePolicyNames)).toBeNull();
  });
});
