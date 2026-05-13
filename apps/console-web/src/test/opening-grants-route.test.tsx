import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { getDefaultProviderAvailability } from "../features/auth/auth-contract";
import { setAuthClientForTests } from "../features/auth/auth-queries";
import { resetSessionForTests, signIn } from "../features/auth/session";
import { setConsoleDataServiceForTests } from "../features/control-plane/service";
import { useControlPlaneFetchMock } from "./control-plane-fetch";
import { renderRoute } from "./router-test-utils";

function setupRouteTest() {
  resetSessionForTests();
  setAuthClientForTests({
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
      Promise.resolve({ data: { outcome: "signed_out" as const }, meta: {} }),
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
  });
  setConsoleDataServiceForTests(null);
}

describe("opening grants route panel", () => {
  useControlPlaneFetchMock();

  beforeEach(setupRouteTest);

  it("renders child key owner summary and inventory", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/api-keys");

    expect(await screen.findByText("Child keys")).toBeInTheDocument();
    expect(screen.getByText("acct_acme_owner")).toBeInTheDocument();
    expect(screen.getByText("acct_acme_owner: 1/8 active")).toBeInTheDocument();
  });

  it("shows revoked status after child key revocation action", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/api-keys");

    const row = screen.getByRole("row", { name: /user_store_001/ });
    fireEvent.click(within(row).getByRole("button", { name: "Revoke" }));

    await waitFor(() => {
      const revokedRow = screen.getByRole("row", { name: /user_store_001/ });
      expect(within(revokedRow).getByText("revoked")).toBeInTheDocument();
      expect(
        within(revokedRow).getByRole("button", { name: "Revoke" }),
      ).toBeDisabled();
    });
  });
});
