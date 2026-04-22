import { beforeEach, describe, expect, it, vi } from "vitest";
import { createConsoleAuthClient } from "./auth-client";

const mockClient = vi.hoisted(() => ({
  completeOAuthLogin: vi.fn(),
  getAuthProviders: vi.fn(),
  getCurrentSession: vi.fn(),
}));

vi.mock("@huge-router/ts-api-client", () => {
  class ControlPlaneClientError extends Error {
    code?: string;
    meta: {
      requestId?: string;
      traceId?: string;
    };
    status: number;

    constructor(
      message: string,
      status: number,
      options: {
        code?: string;
        meta?: {
          requestId?: string;
          traceId?: string;
        };
      } = {},
    ) {
      super(message);
      this.code = options.code;
      this.meta = options.meta ?? {};
      this.name = "ControlPlaneClientError";
      this.status = status;
    }
  }

  return {
    ControlPlaneClientError,
    createControlPlaneClient: () => mockClient,
  };
});

describe("createConsoleAuthClient", () => {
  beforeEach(() => {
    mockClient.completeOAuthLogin.mockReset();
    mockClient.getAuthProviders.mockReset();
    mockClient.getCurrentSession.mockReset();
  });

  it("does not forward relative post-login redirects as OAuth redirect URIs", async () => {
    mockClient.completeOAuthLogin.mockResolvedValue({
      links: [],
      session: {
        activeTenantId: "tenant_acme",
        authenticatedBy: "github",
        createdAt: "2026-04-22T09:30:00Z",
        expiresAt: "2026-04-29T09:30:00Z",
        lastAuthenticatedAt: "2026-04-22T09:30:00Z",
        memberships: [
          {
            membershipId: "tmemb_acme",
            role: "admin",
            status: "active",
            tenant: {
              displayName: "Acme Retail",
              id: "tenant_acme",
              slug: "acme-retail",
            },
          },
        ],
        sessionId: "sess_123",
        state: "active",
        user: {
          createdAt: "2026-04-20T09:00:00Z",
          displayName: "Dev Operator",
          primaryEmail: "dev@example.com",
          userId: "user_123",
        },
      },
    });

    const client = createConsoleAuthClient();
    await client.completeAuthCallback("github", {
      code: "oauth-code",
      redirectTo: "/admin/tenants",
      state: "oauth_state_123",
    });

    expect(mockClient.completeOAuthLogin).toHaveBeenCalledWith("github", {
      code: "oauth-code",
      state: "oauth_state_123",
    });
  });

  it("treats the platform-admin workspace as a platform-admin session", async () => {
    mockClient.getCurrentSession.mockResolvedValue({
      session: {
        activeTenantId: "tenant_platform",
        authenticatedBy: "github",
        createdAt: "2026-04-22T09:30:00Z",
        expiresAt: "2026-04-29T09:30:00Z",
        lastAuthenticatedAt: "2026-04-22T09:30:00Z",
        memberships: [
          {
            membershipId: "tmemb_platform",
            role: "admin",
            status: "active",
            tenant: {
              displayName: "Platform Admin",
              id: "tenant_platform",
              slug: "platform-admin",
            },
          },
        ],
        sessionId: "sess_platform",
        state: "active",
        user: {
          createdAt: "2026-04-20T09:00:00Z",
          displayName: "Platform Admin",
          primaryEmail: "ops@huge-router.dev",
          userId: "user_ops",
        },
      },
    });
    mockClient.getAuthProviders.mockResolvedValue({
      providers: [
        {
          displayName: "Continue with GitHub",
          enabled: true,
          provider: "github",
          startPath: "/api/control-plane/auth/oauth/github/start",
        },
      ],
    });

    const client = createConsoleAuthClient();
    const result = await client.getSession();

    expect(result.data.state.kind).toBe("authenticated");
    if (result.data.state.kind !== "authenticated") {
      throw new Error("expected authenticated session");
    }
    expect(result.data.state.session.user.isPlatformAdmin).toBe(true);
  });

  it("supports oidc as an interactive provider", async () => {
    mockClient.completeOAuthLogin.mockResolvedValue({
      links: [],
      session: {
        activeTenantId: "tenant_platform",
        authenticatedBy: "oidc",
        createdAt: "2026-04-22T09:30:00Z",
        expiresAt: "2026-04-29T09:30:00Z",
        lastAuthenticatedAt: "2026-04-22T09:30:00Z",
        memberships: [
          {
            membershipId: "tmemb_platform",
            role: "admin",
            status: "active",
            tenant: {
              displayName: "Platform Admin",
              id: "tenant_platform",
              slug: "platform-admin",
            },
          },
        ],
        sessionId: "sess_oidc_platform",
        state: "active",
        user: {
          createdAt: "2026-04-20T09:00:00Z",
          displayName: "Enterprise Operator",
          primaryEmail: "enterprise@example.com",
          userId: "user_oidc",
        },
      },
    });

    const client = createConsoleAuthClient();
    const result = await client.completeAuthCallback("oidc", {
      code: "mock-oidc-code",
      state: "oauth_state_oidc",
    });

    expect(mockClient.completeOAuthLogin).toHaveBeenCalledWith("oidc", {
      code: "mock-oidc-code",
      state: "oauth_state_oidc",
    });
    expect(result.data.provider).toBe("oidc");
    expect(result.data.outcome).toBe("authenticated");
  });
});
