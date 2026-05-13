import {
  openingGrantCreateResponseSchema,
  openingGrantSchema,
  openingGrantsResponseSchema,
} from "@huge-router/ts-shared-schema";
import type {
  OpeningGrantCreateResult,
  OpeningGrantView,
} from "./types";

type RequestJson = <T>(
  path: string,
  parse: (payload: unknown) => T,
  init?: RequestInit,
) => Promise<T>;

export type OpeningGrantCreateInput = {
  configSnapshotId: string;
  credentialKind?: "api_key";
  expiresAt: string;
  granteeId: string;
  granteeKind: "user" | "workspace" | "agent";
  granteeLabel?: string;
  ownerAccountId: string;
  scopes: string[];
};

function toOpeningGrantView(
  grant: ReturnType<typeof openingGrantSchema.parse>,
): OpeningGrantView {
  return {
    budgetPolicyId: grant.budget_policy_id,
    canRevoke: grant.status === "active",
    configSnapshotId: grant.config_snapshot_id,
    createdAt: grant.created_at,
    credentialId: grant.credential_id,
    credentialKeyPrefix: grant.credential_key_prefix,
    credentialKind: grant.credential_kind,
    credentialLastFour: grant.credential_last_four,
    expiresAt: grant.expires_at,
    grantId: grant.grant_id,
    granteeId: grant.grantee_id,
    granteeKind: grant.grantee_kind,
    granteeLabel: grant.grantee_label,
    isActive: grant.status === "active",
    ownerAccountId: grant.owner_account_id,
    projectId: grant.project_id,
    providerResourceIds: grant.provider_resource_ids,
    routePolicyId: grant.route_policy_id,
    scopes: grant.scopes,
    status: grant.status,
    tenantId: grant.tenant_id,
    updatedAt: grant.updated_at,
    version: grant.version,
  };
}

const parseOpeningGrantRecord = (payload: unknown) =>
  toOpeningGrantView(openingGrantSchema.parse(payload));

const parseOpeningGrantList = (payload: unknown) =>
  openingGrantsResponseSchema.parse(payload).data.map(toOpeningGrantView);

function parseOpeningGrantCreateResponse(
  payload: unknown,
): OpeningGrantCreateResult {
  const response = openingGrantCreateResponseSchema.parse(payload);

  return {
    credentialId: response.credential.credential_id,
    grant: toOpeningGrantView(response.grant),
    keyPrefix: response.credential.key_prefix,
    lastFour: response.credential.last_four,
    plaintext: response.credential.plaintext,
  };
}

export function createOpeningGrantService({
  isRecoverableMissingEndpoint,
  requestControlPlaneJson,
}: {
  isRecoverableMissingEndpoint: (error: unknown) => boolean;
  requestControlPlaneJson: RequestJson;
}) {
  return {
    async listOpeningGrants() {
      try {
        return await requestControlPlaneJson(
          "/v1/opening-grants",
          parseOpeningGrantList,
          {
            headers: {
              Accept: "application/json",
            },
            method: "GET",
          },
        );
      } catch (error) {
        if (isRecoverableMissingEndpoint(error)) {
          return [];
        }

        throw error;
      }
    },
    createOpeningGrant(input: OpeningGrantCreateInput) {
      return requestControlPlaneJson(
        "/v1/opening-grants",
        parseOpeningGrantCreateResponse,
        {
          body: JSON.stringify({
            config_snapshot_id: input.configSnapshotId,
            credential_kind: input.credentialKind ?? "api_key",
            expires_at: input.expiresAt,
            grantee_id: input.granteeId,
            grantee_kind: input.granteeKind,
            grantee_label: input.granteeLabel,
            owner_account_id: input.ownerAccountId,
            scopes: input.scopes,
          }),
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          method: "POST",
        },
      );
    },
    revokeOpeningGrant(grantId: string, expectedVersion: number) {
      return requestControlPlaneJson(
        `/v1/opening-grants/${encodeURIComponent(grantId)}/revoke`,
        parseOpeningGrantRecord,
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
