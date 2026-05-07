import { describe, expect, it } from "vitest";
import {
  buildOpeningGrantCreateInput,
  countActiveOpeningGrants,
  createOpeningGrantFormState,
  summarizeOpeningGrantOwners,
  validateOpeningGrantForm,
  type ExistingOpeningGrant,
} from "./opening-grants-view-model";
import type { ConfigSnapshotView } from "./types";

const activeSnapshot: ConfigSnapshotView = {
  budgetPolicyId: "budgetpol_default",
  configSnapshotId: "cfgsnap_gateway_v1",
  projectId: "proj_core",
  providerResourceIds: ["prvrsrc_openai_primary"],
  revision: 1,
  routePolicyId: "routepol_openai_chat_default",
  status: "active",
  tenantId: "tenant_acme",
};

function grant(
  ownerAccountId: string,
  granteeId: string,
  overrides: Partial<ExistingOpeningGrant> = {},
): ExistingOpeningGrant {
  return {
    expiresAt: "2027-05-22T00:00:00Z",
    granteeId,
    granteeKind: "user",
    isActive: true,
    ownerAccountId,
    projectId: "proj_core",
    ...overrides,
  };
}

describe("opening grant view model", () => {
  it("counts active child keys per owner and project", () => {
    const grants = [
      grant("acct_a", "user_1"),
      grant("acct_a", "user_2", { projectId: "proj_other" }),
      grant("acct_b", "user_3"),
      grant("acct_a", "user_4", { isActive: false }),
    ];

    expect(
      countActiveOpeningGrants(grants, {
        ownerAccountId: "acct_a",
        projectId: "proj_core",
      }),
    ).toBe(1);
    expect(summarizeOpeningGrantOwners(grants)[0]).toMatchObject({
      activeCount: 2,
      ownerAccountId: "acct_a",
      totalCount: 3,
    });
  });

  it("validates duplicate grantees and active limit in the selected owner scope", () => {
    const form = {
      ...createOpeningGrantFormState([activeSnapshot]),
      expiresAt: "2027-05-22T00:00:00Z",
      granteeId: "user_1",
      ownerAccountId: "acct_a",
    };

    expect(
      validateOpeningGrantForm(form, {
        existingGrants: [grant("acct_a", "user_1")],
        snapshots: [activeSnapshot],
      }).granteeId,
    ).toBe("This grantee already has an active child key for this owner account.");

    const fullOwnerGrants = Array.from({ length: 8 }, (_, index) =>
      grant("acct_a", `user_${index}`),
    );

    expect(
      validateOpeningGrantForm(
        {
          ...form,
          granteeId: "user_9",
        },
        {
          existingGrants: fullOwnerGrants,
          snapshots: [activeSnapshot],
        },
      ).ownerAccountId,
    ).toBe("This owner account already has 8 active child keys.");
  });

  it("builds the create payload with trimmed optional fields", () => {
    const form = {
      ...createOpeningGrantFormState([activeSnapshot]),
      expiresAt: " 2027-05-22T00:00:00Z ",
      granteeId: " user_store_001 ",
      granteeLabel: " ",
      ownerAccountId: " acct_acme_owner ",
      scopes: " route:codex, provider:hugerouter-commercial ",
    };

    expect(buildOpeningGrantCreateInput(form)).toEqual({
      configSnapshotId: "cfgsnap_gateway_v1",
      expiresAt: "2027-05-22T00:00:00Z",
      granteeId: "user_store_001",
      granteeKind: "user",
      granteeLabel: undefined,
      ownerAccountId: "acct_acme_owner",
      scopes: ["route:codex", "provider:hugerouter-commercial"],
    });
  });
});
