import { beforeEach, describe, expect, it } from "vitest";
import { resetSessionForTests, signIn } from "../auth/session";
import { useControlPlaneFetchMock } from "../../test/control-plane-fetch";
import { loadRouteData } from "./loaders";
import {
  getConsoleDataService,
  setConsoleDataServiceForTests,
  type ConsoleDataService,
} from "./service";

describe("console data service", () => {
  useControlPlaneFetchMock();

  beforeEach(() => {
    resetSessionForTests();
    setConsoleDataServiceForTests(null);
  });

  it("loads overview data for the current tenant session", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const overview = await getConsoleDataService().getOverview();

    expect(overview.tenantLabel).toBe("Acme Retail");
    expect(overview.workspace).toBe("acme-retail");
    expect(overview.activeProviders).toBe(2);
    expect(overview.activeRoutes).toBe(2);
    expect(overview.activeSnapshotId).toBe("cfgsnap_gateway_v1");
    expect(overview.selectedProvider).toBe("OpenAI Primary");
    expect(overview.projects).toHaveLength(3);
    expect(overview.projects[0]).toEqual({
      id: "proj_core",
      name: "Core Gateway",
      slug: "core-gateway",
    });
  });

  it("falls back to the active snapshot tenant when the session is anonymous", async () => {
    const overview = await getConsoleDataService().getOverview();

    expect(overview.tenantLabel).toBe("Acme Retail");
    expect(overview.workspace).toBe("acme-retail");
  });

  it("filters route policies to the authenticated tenant", async () => {
    signIn({
      email: "tenant@northstar.dev",
      workspace: "northstar-labs",
    });

    const policies = await getConsoleDataService().listRoutePolicies();

    expect(policies).toHaveLength(1);
    expect(policies[0]?.id).toBe("routepol_northstar_research");
    expect(policies[0]?.name).toBe("Northstar Research");
  });

  it("returns all route policies for admin sessions", async () => {
    signIn({
      email: "admin@huge-router.dev",
      workspace: "platform-admin",
    });

    const policies = await getConsoleDataService().listRoutePolicies();

    expect(policies).toHaveLength(3);
  });

  it("loads recent route receipts with mapped provider labels", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const receipts = await getConsoleDataService().listRouteReceipts();

    expect(receipts).toHaveLength(1);
    expect(receipts[0]?.receiptId).toBe("routercpt_openai_primary_recent");
    expect(receipts[0]?.selectedTargetName).toBe("OpenAI Primary");
    expect(receipts[0]?.excludedTargets[0]?.provider_resource_id).toBe(
      "prvrsrc_openai_backup",
    );
    expect(receipts[0]?.fallbackTransitions[0]?.from_provider_resource_id).toBe(
      "prvrsrc_openai_backup",
    );
    expect(receipts[0]?.fallbackTransitions[0]?.to_provider_resource_id).toBe(
      "prvrsrc_openai_primary",
    );
  });

  it("loads config snapshots from control-plane endpoint", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const snapshots = await getConsoleDataService().listConfigSnapshots();

    expect(snapshots).toHaveLength(2);
    expect(snapshots[0]?.configSnapshotId).toBe("cfgsnap_gateway_v2");
    expect(snapshots[0]?.status).toBe("draft");
    expect(snapshots[1]?.tenantId).toBe("tenant_acme");
  });

  it("loads api keys for the active tenant session", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const keys = await getConsoleDataService().listApiKeys();

    expect(keys).toHaveLength(2);
    expect(keys[0]?.apiKeyId).toBe("key_acme_primary");
    expect(keys.every((key) => key.isActive)).toBe(true);
  });

  it("loads merchant workspace with replay-backed evaluations", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const workspace = await getConsoleDataService().getMerchantWorkspace();

    expect(workspace.merchantEnabled).toBe(true);
    expect(workspace.shops[0]?.merchantShopId).toBe("mshop_acme");
    expect(workspace.cardProducts[0]?.cardProductId).toBe(
      "cardprod_acme_trial",
    );
    expect(workspace.trialConnections[0]?.trialConnectionId).toBe(
      "trialconn_acme_relay",
    );
    expect(workspace.recentEvaluations[0]?.replayCapsuleId).toBe(
      "replay_acme_relay_eval",
    );
    expect(workspace.recentEvaluations[0]?.estimatedTokensSaved).toBe(2400);
  });

  it("loads a replay capsule detail for merchant review", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const replay = await getConsoleDataService().getReplayCapsule(
      "replay_acme_relay_eval",
    );

    expect(replay.replayCapsuleId).toBe("replay_acme_relay_eval");
    expect(replay.redactionTier).toBe("structured_redacted");
    expect(replay.normalizedRequestSummary.protocolFamily).toBe("openai_chat");
    expect(replay.upstreamErrorCode).toBe("provider_signature_mismatch");
  });

  it("revokes an API key without throwing", async () => {
    signIn({
      email: "admin@huge-router.dev",
      workspace: "platform-admin",
    });

    await expect(
      getConsoleDataService().revokeApiKey("key_acme_primary", 1),
    ).resolves.toBeUndefined();
  });

  it("loads tenant detail with tenant-specific providers and policies", async () => {
    const detail = await getConsoleDataService().getTenantDetail("tenant_acme");

    expect(detail.displayName).toBe("Acme Retail");
    expect(detail.activeConfigSnapshotId).toBe("cfgsnap_gateway_v1");
    expect(
      detail.providers.map((provider) => provider.provider_resource_id),
    ).toEqual(["prvrsrc_openai_primary", "prvrsrc_openai_backup"]);
    expect(detail.routePolicies).toHaveLength(2);
  });

  it("throws when tenant detail is requested for an unknown tenant", async () => {
    await expect(
      getConsoleDataService().getTenantDetail("tenant_missing"),
    ).rejects.toThrow("tenant_not_found");
  });

  it("allows tests to replace the service implementation", async () => {
    const override: ConsoleDataService = {
      getOverview() {
        return Promise.resolve({
          activeProviders: 1,
          activeRoutes: 1,
          activeSnapshotId: "cfgsnap_override",
          estimatedCostUsd: "0.000001",
          projects: [],
          selectedProvider: "Override Provider",
          tenantLabel: "Override Tenant",
          workspace: "override-workspace",
        });
      },
      listProjects() {
        return Promise.resolve([]);
      },
      getUsageDashboard() {
        return Promise.resolve({
          activeProjectId: undefined,
          availableProjects: [],
          billablePriceUsd: "0.000001",
          breakdown: [],
          cachedInputTokens: 0,
          eventCount: 0,
          groupBy: "provider",
          inputTokens: 0,
          nextCursor: undefined,
          outputTokens: 0,
          providerCostUsd: "0.000001",
          rangeLabel: "Last 30 days",
          windowEnd: "2026-04-30T00:00:00Z",
          windowStart: "2026-04-01T00:00:00Z",
        });
      },
      getBillingDashboard() {
        return Promise.resolve({
          activeProjectId: undefined,
          availableProjects: [],
          billableTotalUsd: "0.000001",
          canManageBillingExports: true,
          configuredBudgetUsd: "10.000000",
          exportJobs: [],
          lastProjectedAt: "2026-04-22T00:00:00Z",
          projectionLagSeconds: 0,
          providerCostTotalUsd: "0.000001",
          rangeLabel: "Last 30 days",
          remainingBudgetUsd: "9.999999",
          thresholdStatus: "ok",
        });
      },
      queueBillingExport() {
        return Promise.resolve({
          exportJobId: "export_override",
          format: "csv",
          requestedAt: "2026-04-22T00:00:00Z",
          status: "queued",
        });
      },
      createWechatPayPrepay() {
        return Promise.resolve({
          appId: "wx_app_override",
          channel: "native",
          codeQrSvg: "<svg />",
          codeUrl: "weixin://wxpay/bizpayurl?pr=override",
          mchid: "1900000001",
          outTradeNo: "hr_override",
        });
      },
      getWechatPaymentOrder() {
        return Promise.resolve({
          amountTotal: 10000,
          channel: "native",
          createdAt: "2026-04-22T00:00:00Z",
          currency: "CNY",
          expiresAt: "2026-04-22T00:30:00Z",
          outTradeNo: "hr_override",
          status: "pending",
          tenantId: "tenant_override",
          updatedAt: "2026-04-22T00:00:00Z",
        });
      },
      getRouteDiagnostics() {
        return Promise.reject(new Error("unused"));
      },
      getMerchantWorkspace() {
        return Promise.resolve({
          merchantEnabled: true,
          tenantId: "tenant_override",
          shops: [],
          cardProducts: [],
          trialConnections: [],
          recentEvaluations: [],
        });
      },
      getReplayCapsule() {
        return Promise.resolve({
          replayCapsuleId: "replay_override",
          requestId: "req_override",
          traceId: "trace_override",
          routeReceiptId: "routercpt_override",
          configSnapshotId: "cfgsnap_override",
          redactionTier: "structured_redacted",
          normalizedRequestSummary: {
            protocolFamily: "openai_chat",
            modelAlias: "claude-sonnet",
            estimatedPromptTokens: 256,
          },
        });
      },
      getTenantDetail() {
        return Promise.reject(new Error("unused"));
      },
      listProviderResources() {
        return Promise.resolve([]);
      },
      listRoutePolicies() {
        return Promise.resolve([]);
      },
      listRouteReceipts() {
        return Promise.resolve([]);
      },
      listTenants() {
        return Promise.resolve([]);
      },
      listConfigSnapshots() {
        return Promise.resolve([]);
      },
      createConfigSnapshot() {
        return Promise.resolve({
          budgetPolicyId: "budgetpol_default",
          configSnapshotId: "cfgsnap_override",
          providerResourceIds: [],
          routePolicyId: "route_policy_override",
          revision: 1,
          projectId: "proj_core",
          status: "draft",
          tenantId: "tenant_acme",
        });
      },
      activateConfigSnapshot() {
        return Promise.resolve({
          budgetPolicyId: "budgetpol_default",
          configSnapshotId: "cfgsnap_override",
          providerResourceIds: [],
          routePolicyId: "route_policy_override",
          revision: 1,
          projectId: "proj_core",
          status: "active",
          tenantId: "tenant_acme",
        });
      },
      createProviderResource() {
        return Promise.reject(new Error("unused"));
      },
      updateProviderResource() {
        return Promise.reject(new Error("unused"));
      },
      disableProviderResource() {
        return Promise.reject(new Error("unused"));
      },
      listCodexAuthAccounts() {
        return Promise.resolve([]);
      },
      uploadCodexAuthAccount() {
        return Promise.reject(new Error("unused"));
      },
      listOAuthSharingLeases() {
        return Promise.resolve([]);
      },
      upsertOAuthSharingLease() {
        return Promise.reject(new Error("unused"));
      },
      revokeOAuthSharingLease() {
        return Promise.reject(new Error("unused"));
      },
      listOAuthCarpools() {
        return Promise.resolve([]);
      },
      upsertOAuthCarpool() {
        return Promise.reject(new Error("unused"));
      },
      removeOAuthCarpool() {
        return Promise.reject(new Error("unused"));
      },
      readOAuthSharingUsage() {
        return Promise.resolve({
          auditEvents: [],
          rows: [],
        });
      },
      createRoutePolicy() {
        return Promise.reject(new Error("unused"));
      },
      updateRoutePolicy() {
        return Promise.reject(new Error("unused"));
      },
      disableRoutePolicy() {
        return Promise.reject(new Error("unused"));
      },
      listApiKeys() {
        return Promise.resolve([]);
      },
      createApiKey() {
        return Promise.resolve({
          apiKeyId: "key_override",
          displayName: "Override key",
          keyPrefix: "ak-ovr",
          providerResourceId: "prvrsrc_openai_primary",
          version: 1,
        });
      },
      revokeApiKey() {
        return Promise.resolve();
      },
      createMerchantShop() {
        return Promise.reject(new Error("unused"));
      },
      createCardProduct() {
        return Promise.reject(new Error("unused"));
      },
      createTrialConnection() {
        return Promise.reject(new Error("unused"));
      },
      runRelayEvaluation() {
        return Promise.reject(new Error("unused"));
      },
      downloadBillingExport() {
        return Promise.resolve("a,b\n1,2\n");
      },
    };

    setConsoleDataServiceForTests(override);

    expect(await getConsoleDataService().getOverview()).toEqual({
      activeProviders: 1,
      activeRoutes: 1,
      activeSnapshotId: "cfgsnap_override",
      estimatedCostUsd: "0.000001",
      projects: [],
      selectedProvider: "Override Provider",
      tenantLabel: "Override Tenant",
      workspace: "override-workspace",
    });
  });
});

describe("loadRouteData", () => {
  function createRequestError(
    status: number,
    code: string,
    message = `Request error ${status}`,
  ) {
    const error = new Error(message);

    (error as unknown as { code: string }).code = code;
    (error as unknown as { status: number }).status = status;

    return error;
  }

  it("returns success state when the loader resolves", async () => {
    await expect(loadRouteData(() => Promise.resolve("ok"))).resolves.toEqual({
      data: "ok",
      state: "success",
    });
  });

  it("returns error state when the loader throws", async () => {
    await expect(
      loadRouteData(() => Promise.reject(new Error("failed"))),
    ).resolves.toMatchObject({
      kind: "unknown",
      message: "failed",
      state: "error",
    });
  });

  it("classifies session-expired errors", async () => {
    await expect(
      loadRouteData(() =>
        Promise.reject(createRequestError(401, "session_expired")),
      ),
    ).resolves.toMatchObject({
      kind: "session-expired",
      state: "error",
    });
  });

  it("classifies access-denied errors", async () => {
    await expect(
      loadRouteData(() =>
        Promise.reject(createRequestError(403, "tenant_access_denied")),
      ),
    ).resolves.toMatchObject({
      kind: "access-denied",
      state: "error",
    });
  });

  it("classifies not-found errors", async () => {
    await expect(
      loadRouteData(() => Promise.reject(createRequestError(404, "not_found"))),
    ).resolves.toMatchObject({
      kind: "not-found",
      state: "error",
    });
  });
});
