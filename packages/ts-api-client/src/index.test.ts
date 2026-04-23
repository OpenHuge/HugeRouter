import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import {
  ContractApiError,
  createControlPlaneClient,
  createGatewayClient,
} from "./index.ts";

const readJson = (relativePath: string) =>
  JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), "utf8"),
  ) as Record<string, unknown>;

const jsonResponse = (status: number, payload: unknown) =>
  new Response(JSON.stringify(payload), {
    status,
    headers: {
      "content-type": "application/json",
    },
  });

const resolveRequestUrl = (input: RequestInfo | URL) =>
  typeof input === "string"
    ? input
    : input instanceof URL
      ? input.toString()
      : input.url;

void test("generated operation metadata stays in sync with the schema manifest", () => {
  const manifest = readJson(
    "../../../schemas/jsonschema/contracts.manifest.json",
  );
  const client = createControlPlaneClient({
    baseUrl: "https://api.example.test",
  });

  assert.equal(manifest.contract_digest, client.contractDigest);
});

void test("control-plane client resolves the documented endpoints and parses responses", async () => {
  const tenantsResponse = readJson(
    "../../../schemas/examples/control-plane/tenants.response.json",
  );
  const projectsResponse = readJson(
    "../../../schemas/examples/control-plane/projects.response.json",
  );
  const providerResourcesResponse = readJson(
    "../../../schemas/examples/control-plane/provider-resources.response.json",
  );
  const routeDiagnosticsResponse = readJson(
    "../../../schemas/examples/control-plane/route-diagnostics.response.json",
  );
  const routeReceiptResponse = readJson(
    "../../../schemas/examples/control-plane/route-receipt.response.json",
  );
  const routeReceiptsResponse = readJson(
    "../../../schemas/examples/control-plane/route-receipts.response.json",
  );

  const calls: Array<{ url: string; method?: string }> = [];
  const fetchImpl = (input: RequestInfo | URL, init?: RequestInit) => {
    const url = resolveRequestUrl(input);
    calls.push({ url, method: init?.method });

    if (url.endsWith("/v1/tenants")) {
      return Promise.resolve(jsonResponse(200, tenantsResponse));
    }

    if (url.endsWith("/v1/projects")) {
      return Promise.resolve(jsonResponse(200, projectsResponse));
    }

    if (url.endsWith("/v1/provider-resources?capability=realtime")) {
      return Promise.resolve(jsonResponse(200, providerResourcesResponse));
    }

    if (url.endsWith("/v1/provider-resources")) {
      return Promise.resolve(jsonResponse(200, providerResourcesResponse));
    }

    if (
      url.endsWith(
        "/v1/route-receipts?route_policy_id=routepol_default&limit=5",
      )
    ) {
      return Promise.resolve(jsonResponse(200, routeReceiptsResponse));
    }

    if (url.endsWith("/v1/route-receipts/routercpt_123")) {
      return Promise.resolve(jsonResponse(200, routeReceiptResponse));
    }

    if (url.endsWith("/v1/route-diagnostics/routepol_default")) {
      return Promise.resolve(jsonResponse(200, routeDiagnosticsResponse));
    }
    return Promise.resolve(
      jsonResponse(
        404,
        readJson("../../../schemas/examples/gateway/error.response.json"),
      ),
    );
  };

  const client = createControlPlaneClient({
    baseUrl: "https://api.example.test",
    fetch: fetchImpl,
  });

  const tenants = await client.listTenants();
  const projects = await client.listProjects();
  const resources = await client.listProviderResources({
    capability: "realtime",
  });
  const receipts = await client.listRouteReceipts({
    limit: 5,
    routePolicyId: "routepol_default",
  });
  const receipt = await client.getRouteReceipt("routercpt_123");
  const diagnostics = await client.getRouteDiagnostics("routepol_default");

  assert.equal(tenants[0]?.tenant_id, "tenant_acme");
  assert.equal(projects[0]?.project_id, "proj_core");
  assert.equal(resources[0]?.provider_resource_id, "prvrsrc_openai_primary");
  assert.equal(receipts[0]?.route_receipt_id, "routercpt_123");
  assert.equal(receipt.route_receipt_id, "routercpt_123");
  assert.equal(diagnostics.route_policy.route_policy_id, "routepol_default");
  assert.deepEqual(calls, [
    {
      url: "https://api.example.test/v1/tenants",
      method: "GET",
    },
    {
      url: "https://api.example.test/v1/projects",
      method: "GET",
    },
    {
      url: "https://api.example.test/v1/provider-resources?capability=realtime",
      method: "GET",
    },
    {
      url: "https://api.example.test/v1/route-receipts?route_policy_id=routepol_default&limit=5",
      method: "GET",
    },
    {
      url: "https://api.example.test/v1/route-receipts/routercpt_123",
      method: "GET",
    },
    {
      url: "https://api.example.test/v1/route-diagnostics/routepol_default",
      method: "GET",
    },
  ]);
});

void test("gateway client validates requests and normalizes contract errors", async () => {
  const gatewayRequest = readJson(
    "../../../schemas/examples/gateway/chat.request.json",
  );
  const gatewayResponse = readJson(
    "../../../schemas/examples/gateway/chat.response.json",
  );
  const errorResponse = readJson(
    "../../../schemas/examples/gateway/error.response.json",
  );

  const client = createGatewayClient({
    baseUrl: "https://gateway.example.test",
    fetch: () => Promise.resolve(jsonResponse(200, gatewayResponse)),
  });

  const response = await client.createChatCompletion(gatewayRequest as never);
  assert.equal(response.provider_response_id, "resp_openai_123");

  const failingClient = createGatewayClient({
    baseUrl: "https://gateway.example.test",
    fetch: () => Promise.resolve(jsonResponse(422, errorResponse)),
  });

  await assert.rejects(
    () => failingClient.createChatCompletion(gatewayRequest as never),
    (error: unknown) => {
      assert.ok(error instanceof ContractApiError);
      assert.equal(error.status, 422);
      assert.equal(error.envelope.error.code, "validation_failed");
      return true;
    },
  );
});

void test("startEmailLogin posts the expected auth path and payload", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const client = createControlPlaneClient({
    baseUrl: "https://control-plane.example.com",
    fetch: (url, init) => {
      requests.push({ url: resolveRequestUrl(url), init });

      return Promise.resolve(
        Response.json({
          flowId: "authflow_123",
          verificationMode: "magic_link",
          expiresAt: "2026-04-22T10:00:00Z",
        }),
      );
    },
  });

  const response = await client.startEmailLogin({
    email: "dev@example.com",
    workspaceSlug: "platform-admin",
    redirectTo: "/login/check-email",
  });

  assert.equal(
    requests[0]?.url,
    "https://control-plane.example.com/api/control-plane/auth/email/start",
  );
  assert.equal(requests[0]?.init?.method, "POST");
  assert.equal(
    requests[0]?.init?.body,
    JSON.stringify({
      email: "dev@example.com",
      workspaceSlug: "platform-admin",
      redirectTo: "/login/check-email",
    }),
  );
  assert.equal(response.flowId, "authflow_123");
});

void test("startOAuthLogin uses provider-specific start endpoints", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const client = createControlPlaneClient({
    baseUrl: "",
    fetch: (url, init) => {
      requests.push({ url: resolveRequestUrl(url), init });

      return Promise.resolve(
        Response.json({
          provider: "github",
          authorizationUrl:
            "https://github.com/login/oauth/authorize?client_id=demo",
          state: "oauth_state_123",
          expiresAt: "2026-04-22T10:00:00Z",
        }),
      );
    },
  });

  const response = await client.startOAuthLogin("github", {
    workspaceSlug: "platform-admin",
    redirectTo: "/login/callback",
  });

  assert.equal(requests[0]?.url, "/api/control-plane/auth/oauth/github/start");
  assert.equal(requests[0]?.init?.method, "POST");
  assert.equal(response.provider, "github");
});

void test("completeOAuthLogin validates the response payload as a shared auth result", async () => {
  const client = createControlPlaneClient({
    baseUrl: "",
    fetch: () =>
      Promise.resolve(
        Response.json({
          session: {
            sessionId: "sess_123",
            state: "active",
            user: {
              userId: "user_123",
              primaryEmail: "dev@example.com",
              displayName: "Dev Operator",
              createdAt: "2026-04-20T09:00:00Z",
            },
            memberships: [],
            authenticatedBy: "github",
            createdAt: "2026-04-22T09:30:00Z",
            expiresAt: "2026-04-29T09:30:00Z",
            lastAuthenticatedAt: "2026-04-22T09:30:00Z",
          },
          links: [
            {
              linkId: "authlink_123",
              provider: "github",
              providerSubject: "github-user-42",
              linkedAt: "2026-04-22T09:30:00Z",
              canUnlink: true,
            },
          ],
        }),
      ),
  });

  const result = await client.completeOAuthLogin("github", {
    state: "oauth_state_123",
    code: "oauth_code_123",
    redirectUri: "https://console.example.com/login/callback",
  });

  assert.equal(result.session.sessionId, "sess_123");
  assert.equal(result.links[0]?.provider, "github");
});

void test("unlinkAuthProvider hits the provider-specific delete endpoint", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const client = createControlPlaneClient({
    baseUrl: "",
    fetch: (url, init) => {
      requests.push({ url: resolveRequestUrl(url), init });

      return Promise.resolve(
        Response.json({
          provider: "email",
          removed: false,
        }),
      );
    },
  });

  const response = await client.unlinkAuthProvider("email");

  assert.equal(requests[0]?.url, "/api/control-plane/auth/links/email");
  assert.equal(requests[0]?.init?.method, "DELETE");
  assert.equal(response.removed, false);
});
