import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { type ConsoleAuthClient } from "../features/auth/auth-client";
import { getDefaultProviderAvailability } from "../features/auth/auth-contract";
import { setAuthClientForTests } from "../features/auth/auth-queries";
import { resetSessionForTests, signIn } from "../features/auth/session";
import {
  getConsoleDataService,
  setConsoleDataServiceForTests,
  type ConsoleDataService,
} from "../features/control-plane/service";
import { useControlPlaneFetchMock } from "./control-plane-fetch";
import { createDeferred, renderRoute } from "./router-test-utils";

function createAuthClientStub(
  overrides: Partial<ConsoleAuthClient> = {},
): ConsoleAuthClient {
  return {
    completeAuthCallback: () => {
      throw new Error("not used in route tests");
    },
    completeEmailLogin: () =>
      Promise.resolve({
        data: {
          message: "Email verification completed.",
          outcome: "authenticated" as const,
          state: {
            availableProviders: getDefaultProviderAvailability(),
            kind: "authenticated" as const,
            session: {
              activeTenant: null,
              expiresAt: "2026-04-29T09:30:00Z",
              memberships: [],
              sessionId: "sess_123",
              user: {
                displayName: "Operations Admin",
                email: "ops@huge-router.dev",
                id: "user_123",
                isPlatformAdmin: true,
              },
            },
          },
        },
        meta: {},
      }),
    getSession: () =>
      Promise.resolve({
        data: {
          state: {
            availableProviders: getDefaultProviderAvailability(),
            kind: "anonymous" as const,
          },
        },
        meta: {},
      }),
    logout: () =>
      Promise.resolve({
        data: {
          outcome: "signed_out" as const,
        },
        meta: {},
      }),
    startEmailLogin: () =>
      Promise.resolve({
        data: {
          codeHint: "Use local bootstrap verification code 111111.",
          email: "ops@huge-router.dev",
          expiresAt: "2026-04-22T10:00:00Z",
          flowId: "authflow_123",
          message: "HugeRouter started an email login flow.",
          outcome: "email_sent" as const,
        },
        meta: {},
      }),
    startProviderLogin: () =>
      Promise.resolve({
        data: {
          authorizationUrl:
            "http://127.0.0.1:3000/login/callback?provider=github&state=oauth_state_123&code=mock-github-code",
          outcome: "redirect" as const,
        },
        meta: {},
      }),
    ...overrides,
  };
}

describe("console routes", () => {
  useControlPlaneFetchMock();

  beforeEach(() => {
    resetSessionForTests();
    setAuthClientForTests(createAuthClientStub());
    setConsoleDataServiceForTests(null);
  });

  it("redirects anonymous users away from tenant routes", async () => {
    await renderRoute("/app/overview");

    expect(
      await screen.findByRole("heading", {
        name: "Sign in",
      }),
    ).toBeInTheDocument();
  });

  it("redirects authenticated tenant users away from admin routes", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/admin/tenants");

    expect(
      await screen.findByRole("heading", {
        name: "Overview",
      }),
    ).toBeInTheDocument();
  });

  it("redirects authenticated admin users away from tenant routes", async () => {
    signIn({
      email: "admin@huge-router.dev",
      workspace: "platform-admin",
    });

    await renderRoute("/app/providers");

    expect(
      await screen.findByRole("heading", {
        name: "Tenants",
      }),
    ).toBeInTheDocument();
  });

  it("redirects authenticated sessions away from login", async () => {
    signIn({
      email: "admin@huge-router.dev",
      workspace: "platform-admin",
    });

    await renderRoute("/login");

    expect(
      await screen.findByRole("heading", {
        name: "Tenants",
      }),
    ).toBeInTheDocument();
  });

  it("renders overview success state for tenant sessions", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/overview");

    expect(
      await screen.findByRole("heading", {
        name: "Overview",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("Active providers")).toBeInTheDocument();
    expect(
      screen.getByText(/Selected provider: OpenAI Primary/),
    ).toBeInTheDocument();
  });

  it("renders overview loading state while route data is pending", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const deferred =
      createDeferred<Awaited<ReturnType<ConsoleDataService["getOverview"]>>>();
    const baseService = getConsoleDataService();

    setConsoleDataServiceForTests({
      ...baseService,
      getOverview: () => deferred.promise,
    });

    await renderRoute("/app/overview", {
      waitForLoad: false,
    });

    expect(
      await screen.findByLabelText("Loading overview"),
    ).toBeInTheDocument();

    deferred.resolve({
      activeProviders: 3,
      activeRoutes: 3,
      activeSnapshotId: "cfgsnap_gateway_v1",
      estimatedCostUsd: "0.000210",
      projects: [],
      selectedProvider: "OpenAI Primary",
      tenantLabel: "Acme Retail",
      workspace: "acme-retail",
    });

    expect(
      await screen.findByRole("heading", {
        name: "Overview",
      }),
    ).toBeInTheDocument();
  });

  it("renders overview empty state when no projects are returned", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    const baseService = getConsoleDataService();

    setConsoleDataServiceForTests({
      ...baseService,
      async getOverview() {
        const data = await baseService.getOverview();

        return {
          ...data,
          projects: [],
        };
      },
    });

    await renderRoute("/app/overview");

    expect(await screen.findByText("No projects")).toBeInTheDocument();
  });

  it("renders providers success state for tenant sessions", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/providers");

    expect(
      await screen.findByRole("heading", {
        name: "Providers",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("OpenAI Primary")).toBeInTheDocument();
    expect(screen.getByText("Realtime Transit Relay")).toBeInTheDocument();
    expect(
      screen.getAllByText("response-model-metadata").length,
    ).toBeGreaterThan(0);
  });

  it("renders routes success state for tenant sessions", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/routes");

    expect(
      await screen.findByRole("heading", {
        name: "Routes",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("Acme Reasoning Fast")).toBeInTheDocument();
    expect(screen.getAllByText("Inspect diagnostics").length).toBeGreaterThan(
      0,
    );
  });

  it("renders route diagnostics drill-down success state", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/route-diagnostics/routepol_acme_realtime");

    expect(
      await screen.findByRole("heading", {
        name: "Acme Realtime Agent",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("Target decision matrix")).toBeInTheDocument();
    expect(
      screen.getByText(/No healthy target satisfied realtime_webrtc/i),
    ).toBeInTheDocument();
  });

  it("renders route receipts success state for tenant sessions", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/receipts");

    expect(
      await screen.findByRole("heading", {
        name: "Route receipts",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("routercpt_acme_realtime")).toBeInTheDocument();
    expect(
      screen.getByText(/Excluded: capability_gap_realtime/),
    ).toBeInTheDocument();
  });

  it("renders tenant inventory and links to tenant detail", async () => {
    signIn({
      email: "admin@huge-router.dev",
      workspace: "platform-admin",
    });

    await renderRoute("/admin/tenants");

    expect(
      await screen.findByRole("heading", {
        name: "Tenants",
      }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("link", {
        name: "Acme Retail",
      }),
    ).toHaveAttribute("href", "/admin/tenants/tenant_acme");
  });

  it("renders tenant detail for a selected tenant", async () => {
    signIn({
      email: "admin@huge-router.dev",
      workspace: "platform-admin",
    });

    await renderRoute("/admin/tenants/tenant_acme");

    expect(
      await screen.findByRole("heading", {
        name: "Acme Retail",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("Provider resources")).toBeInTheDocument();
    expect(screen.getByText("Acme Reasoning Fast")).toBeInTheDocument();
  });

  it("renders tenant detail error state for unknown tenants", async () => {
    signIn({
      email: "admin@huge-router.dev",
      workspace: "platform-admin",
    });

    await renderRoute("/admin/tenants/tenant_missing");

    expect(await screen.findByText("Tenant unavailable")).toBeInTheDocument();
  });
});
