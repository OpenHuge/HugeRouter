import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import {
  authProviderLinkSchema,
  authSessionSchema,
  authSessionResponseSchema,
  merchantWorkspaceResponseSchema,
  configSnapshotResponseSchema,
  contractDigest,
  emailLoginCompleteRequestSchema,
  emailLoginStartRequestSchema,
  routeReceiptDiagnosticsResponseSchema,
  oauthCallbackRequestSchema,
  oauthLoginStartResponseSchema,
  oauthProviderSchema,
  projectsResponseSchema,
  providerResourcesResponseSchema,
  replayCapsuleResponseSchema,
  routePoliciesResponseSchema,
  routeSimulationRequestSchema,
  routeSimulationResponseSchema,
  tenantsResponseSchema,
  usageEventRecordedMessageSchema,
} from "./index.ts";

const readJson = (relativePath: string) =>
  JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), "utf8"),
  ) as Record<string, unknown>;

void test("checked-in schema manifest stays in sync with generated package metadata", () => {
  const manifest = readJson(
    "../../../schemas/jsonschema/contracts.manifest.json",
  );

  assert.equal(manifest.contract_digest, contractDigest);
});

void test("control-plane examples conform to shared zod schemas", () => {
  const tenants = readJson(
    "../../../schemas/examples/control-plane/tenants.response.json",
  );
  const projects = readJson(
    "../../../schemas/examples/control-plane/projects.response.json",
  );
  const resources = readJson(
    "../../../schemas/examples/control-plane/provider-resources.response.json",
  );
  const routePolicies = readJson(
    "../../../schemas/examples/control-plane/route-policies.response.json",
  );
  const snapshot = readJson(
    "../../../schemas/examples/control-plane/config-snapshot.response.json",
  );
  const simulationRequest = readJson(
    "../../../schemas/examples/control-plane/route-simulation.request.json",
  );
  const simulationResponse = readJson(
    "../../../schemas/examples/control-plane/route-simulation.response.json",
  );
  const sessionResponse = readJson(
    "../../../schemas/examples/auth/session-response.json",
  );

  assert.equal(
    tenantsResponseSchema.parse(tenants).data[0]?.tenant_id,
    "tenant_acme",
  );
  assert.equal(
    projectsResponseSchema.parse(projects).data[0]?.project_id,
    "proj_core",
  );
  assert.equal(
    providerResourcesResponseSchema.parse(resources).data[0]?.provider_id,
    "openai",
  );
  assert.equal(
    routePoliciesResponseSchema.parse(routePolicies).data[0]?.route_policy_id,
    "routepol_default",
  );
  assert.equal(
    configSnapshotResponseSchema.parse(snapshot).config_snapshot
      .config_snapshot_id,
    "cfgsnap_default",
  );
  assert.equal(
    routeSimulationRequestSchema.parse(simulationRequest).protocol_family,
    "openai_chat",
  );
  assert.equal(
    routeSimulationResponseSchema.parse(simulationResponse).selected_target,
    "prvrsrc_openai_primary",
  );
  assert.equal(
    authSessionResponseSchema.parse(sessionResponse).session?.sessionId,
    "sess_123",
  );
});

void test("event and diagnostics examples conform to shared zod schemas", () => {
  const routeReceiptDiagnostics = readJson(
    "../../../schemas/examples/control-plane/route-receipt-diagnostics.response.json",
  );
  const usageMessage = readJson(
    "../../../schemas/examples/events/usage-event-recorded.message.json",
  );

  assert.equal(
    usageEventRecordedMessageSchema.parse(usageMessage).message_type,
    "usage_event.recorded",
  );
  assert.equal(
    routeReceiptDiagnosticsResponseSchema.parse(routeReceiptDiagnostics)
      .route_receipt.route_receipt_id,
    "routercpt_123",
  );
});

void test("oauth providers exclude email", () => {
  assert.equal(oauthProviderSchema.parse("github"), "github");
  assert.throws(() => oauthProviderSchema.parse("email"));
});

void test("auth session payload parses a minimal real session", () => {
  const parsed = authSessionSchema.parse({
    sessionId: "sess_123",
    state: "active",
    user: {
      userId: "user_123",
      primaryEmail: "dev@example.com",
      displayName: "Dev Operator",
      avatarUrl: "https://example.com/avatar.png",
      createdAt: "2026-04-20T09:00:00Z",
      lastLoginAt: "2026-04-22T09:30:00Z",
    },
    activeTenantId: "tenant_123",
    memberships: [
      {
        membershipId: "tmemb_123",
        tenant: {
          id: "tenant_123",
          slug: "acme",
          displayName: "Acme",
        },
        role: "admin",
        status: "active",
      },
    ],
    authenticatedBy: "google",
    createdAt: "2026-04-22T09:30:00Z",
    expiresAt: "2026-04-29T09:30:00Z",
    lastAuthenticatedAt: "2026-04-22T09:30:00Z",
  });

  assert.equal(parsed.user.userId, "user_123");
  assert.equal(parsed.memberships[0]?.tenant.displayName, "Acme");
  assert.equal(parsed.authenticatedBy, "google");
});

void test("auth provider links preserve provider subject and unlink flag", () => {
  const parsed = authProviderLinkSchema.parse({
    linkId: "authlink_123",
    provider: "wechat",
    providerSubject: "wechat-openid-42",
    linkedAt: "2026-04-22T09:30:00Z",
    canUnlink: false,
  });

  assert.equal(parsed.providerSubject, "wechat-openid-42");
  assert.equal(parsed.canUnlink, false);
});

void test("email and oauth completion requests remain distinct shapes", () => {
  const emailParsed = emailLoginCompleteRequestSchema.parse({
    flowId: "authflow_123",
    code: "123456",
  });
  const oauthParsed = oauthCallbackRequestSchema.parse({
    state: "oauth-state-123",
    code: "oauth-code-123",
    redirectUri: "https://console.example.com/login/callback",
  });

  assert.equal(emailParsed.flowId, "authflow_123");
  assert.ok(!("state" in emailParsed));
  assert.equal(oauthParsed.state, "oauth-state-123");
  assert.ok(!("flowId" in oauthParsed));
});

void test("checked-in oauth example stays in sync with shared schema", () => {
  const example = readJson(
    "../../../schemas/examples/auth/github-oauth-start-response.json",
  );
  const parsed = oauthLoginStartResponseSchema.parse(example);

  assert.equal(parsed.provider, "github");
  assert.match(parsed.authorizationUrl, /^https:\/\/github\.com\//);
});

void test("email login start request requires a workspace slug", () => {
  const parsed = emailLoginStartRequestSchema.parse({
    email: "dev@example.com",
    workspaceSlug: "platform-admin",
  });

  assert.equal(parsed.workspaceSlug, "platform-admin");
});

void test("merchant workspace payload accepts replay-linked evaluations", () => {
  const parsed = merchantWorkspaceResponseSchema.parse({
    data: {
      merchant_enabled: true,
      tenant_id: "tenant_acme",
      shops: [
        {
          merchant_shop_id: "mshop_acme",
          tenant_id: "tenant_acme",
          slug: "acme-small-shop",
          display_name: "Acme Small Shop",
          seller_alias: "acme-verified",
          identity_level: "l2_kyc",
          guarantee_deposit_usd: "250.00",
          dispute_rate_bps: 125,
          status: "active",
          announcement: "Fresh stock every day",
          fulfillment_mode: "auto_card_secret",
          version: 1,
          created_at: "2026-04-22T00:00:00Z",
          updated_at: "2026-04-22T00:00:00Z",
        },
      ],
      card_products: [
        {
          card_product_id: "cardprod_trial_pack",
          tenant_id: "tenant_acme",
          merchant_shop_id: "mshop_acme",
          title: "Trial Claude Pack",
          description: "Five low-cost test cards",
          status: "active",
          inventory_count: 42,
          face_value_usd: "1.00",
          retail_price_usd: "1.99",
          delivery_kind: "direct_secret",
          supports_trial: true,
          risk_tier: "green",
          review_status: "approved",
          escrow_mode: "platform_ledger",
          required_kyc_level: "l1_basic",
          evidence_requirement: "Replay capsule required before listing.",
          version: 1,
          created_at: "2026-04-22T00:00:00Z",
          updated_at: "2026-04-22T00:00:00Z",
        },
      ],
      recent_orders: [
        {
          trade_order_id: "tradeord_acme_trial_001",
          tenant_id: "tenant_acme",
          merchant_shop_id: "mshop_acme",
          card_product_id: "cardprod_trial_pack",
          buyer_alias: "buyer-l1-8291",
          seller_alias: "acme-verified",
          state: "escrow_funded",
          escrow_mode: "platform_ledger",
          evidence_state: "required",
          dispute_state: "none",
          order_amount_usd: "1.99",
          created_at: "2026-04-22T00:00:00Z",
          updated_at: "2026-04-22T00:00:00Z",
        },
      ],
      trial_connections: [
        {
          trial_connection_id: "trialconn_acme",
          tenant_id: "tenant_acme",
          provider_label: "Acme Relay",
          endpoint_base_url: "https://relay.example.com/v1",
          api_key_masked: "sk-test...1234",
          target_model: "claude-sonnet",
          status: "active",
          version: 1,
          created_at: "2026-04-22T00:00:00Z",
          updated_at: "2026-04-22T00:00:00Z",
        },
      ],
      recent_evaluations: [
        {
          relay_evaluation_id: "reval_acme",
          tenant_id: "tenant_acme",
          trial_connection_id: "trialconn_acme",
          replay_capsule_id: "replay_acme",
          provider_label: "Acme Relay",
          endpoint_base_url: "https://relay.example.com/v1",
          target_model: "claude-sonnet",
          runner_mode: "simulated",
          sample_request_count: 5,
          estimated_tokens_saved: 2400,
          overall_score: 82,
          verdict: "warning",
          fingerprint_status: "pass",
          protocol_status: "warning",
          token_status: "warning",
          multimodal_status: "not_tested",
          summary: "Replay capsule captured for support review.",
          created_at: "2026-04-22T00:00:00Z",
        },
      ],
    },
  });

  assert.equal(
    parsed.data.recent_evaluations[0]?.replay_capsule_id,
    "replay_acme",
  );
  assert.equal(parsed.data.card_products[0]?.delivery_kind, "direct_secret");
  assert.equal(parsed.data.recent_orders[0]?.state, "escrow_funded");
});

void test("replay capsule payload keeps redacted summary shape", () => {
  const parsed = replayCapsuleResponseSchema.parse({
    replay_capsule: {
      replay_capsule_id: "replay_eval_1",
      request_id: "req_eval_1",
      trace_id: "trace_eval_1",
      route_receipt_id: "routercpt_eval_1",
      config_snapshot_id: "cfgsnap_eval_1",
      redaction_tier: "structured_redacted",
      normalized_request_summary: {
        protocol_family: "openai_chat",
        model_alias: "claude-sonnet",
        estimated_prompt_tokens: 480,
      },
      upstream_error_summary: {
        code: "provider_signature_mismatch",
      },
    },
  });

  assert.equal(parsed.replay_capsule.redaction_tier, "structured_redacted");
  assert.equal(
    parsed.replay_capsule.normalized_request_summary.estimated_prompt_tokens,
    480,
  );
});
