# Merchant Marketplace And Relay Evaluation Spec

[Back to Docs Index](../README.md)

## 1. Problem

HugeRouter currently models tenants, projects, provider resources, API keys, routes, usage, and billing, but it does not yet expose a merchant-facing workflow for:

- opening a small shop inside the console
- selling card-secret style products with automatic fulfillment
- attaching trial keys and relay providers for pre-sale quality checks
- showing buyers and operators whether a relay is trustworthy enough to buy from

This gap matters because AI relay commerce is increasingly bundled with lightweight storefronts, trial access, and third-party quality claims. If HugeRouter only manages routing and billing, merchants still need separate tools to sell, test, and explain their upstream quality.

## 2. External Reference Notes

As of **April 23, 2026**, `cctest.ai` emphasizes a narrow but important product idea: relay evaluation should not be a generic ping or benchmark page. Its public pages describe:

- black-box verification rather than self-reported merchant claims
- multiple probe requests covering fingerprint, protocol structure, signature/channel clues, and multimodal behavior
- explicit warnings around counterfeit models, protocol inconsistency, token abuse, and data leakage
- guidance to use a dedicated test key instead of a production key

Relevant sources:

- [cctest.ai home](https://cctest.ai/en)
- [cctest.ai FAQ](https://cctest.ai/en/faq)

Implication for HugeRouter:

- merchant tooling should couple sales with quality evidence
- trial-provider onboarding must be first-class, not an operator-only workaround
- evaluation results must explain risk, not just print a score
- the first implementation should distinguish real probe-based checks from simulated or manually confirmed checks

## 3. Product Positioning

HugeRouter should add a tenant-facing `Merchant Center` that sits beside existing route, provider, billing, and API-key surfaces.

The V1 product is not a full payment marketplace. It is a merchant operations slice that lets a tenant:

- open one or more storefronts
- publish card-secret products
- register trial relay endpoints and masked test keys
- run and retain relay quality evaluations before or during sale

## 4. Goals

- Let tenant admins become merchants without leaving HugeRouter.
- Make card-secret inventory and trial-provider configuration visible in one workspace.
- Attach relay quality evidence to merchant assets so storefront operations are not blind.
- Reuse HugeRouter tenancy, auth, typed schema, and control-plane patterns instead of inventing a parallel system.

## 5. Non-Goals For V1

- payment acquisition, settlement, refunds, invoicing, or payout ledgers
- buyer-facing public storefront pages
- full black-box detection parity with `cctest.ai`
- upstream signature reverse-engineering or forensic proof claims
- automatic secret inventory import from third-party发卡平台

## 6. Core Personas

### 6.1 Merchant Operator

Needs to open a shop, create sellable card products, attach trial channels, and understand whether an upstream relay is safe enough to list.

### 6.2 Platform Admin

Needs to inspect merchant readiness, risky relay verdicts, and whether a tenant is selling through degraded or suspicious channels.

### 6.3 Buyer Support / Ops

Needs quick visibility into what was sold, which trial provider was evaluated, and why a relay is marked healthy, warning, or failed.

## 7. V1 Functional Scope

### 7.1 Merchant Center

Add tenant route:

- `/app/merchant`

Primary sections:

- shop profile and status
- card-secret products
- trial provider connections
- recent relay evaluations

### 7.2 Shop Management

Merchant admins can:

- create a shop
- define slug, display name, announcement, and operating status
- declare fulfillment mode as `auto_card_secret`

### 7.3 Card-Secret Products

Merchant admins can:

- create products attached to a shop
- define title, description, inventory count, retail price, face value, and status
- mark products as trial-friendly or regular paid inventory

V1 stores product metadata only. Secret stock and payment fulfillment are intentionally out of scope.

### 7.4 Trial Provider Connections

Merchant admins can register a trial relay target with:

- provider label
- endpoint base URL
- masked key prefix only in returned payloads
- target model
- connection status
- optional notes

Guardrail:

- UI copy must tell merchants to use dedicated trial keys, not production keys

### 7.5 Relay Evaluation

Merchants can trigger an evaluation against a registered trial connection.

V1 evaluation dimensions:

- model fingerprint confidence
- protocol consistency
- stream/non-stream structure readiness
- token reasonability
- multimodal readiness
- channel hint or provenance hint when available

### 7.6 Replayable Test Record

Each merchant evaluation should produce a replayable support artifact so operators do not need to repeatedly spend live tokens for:

- support review
- merchant dispute handling
- regression comparison
- buyer-facing risk explanation

The artifact should be redacted-first and reuse HugeRouter's replay-capsule direction instead of inventing a merchant-only debug format.

V1 runner mode:

- `simulated`

This means the first implementation returns a deterministic preview result shaped like the future real evaluator contract. It must be labeled clearly in docs and UI.

## 8. Domain Additions

### 8.1 Merchant Shop

Attributes:

- `merchant_shop_id`
- `tenant_id`
- `slug`
- `display_name`
- `status` as `draft | active | suspended`
- `announcement` nullable
- `fulfillment_mode` as `auto_card_secret`
- `version`
- `created_at`
- `updated_at`

### 8.2 Card Product

Attributes:

- `card_product_id`
- `tenant_id`
- `merchant_shop_id`
- `title`
- `description`
- `status` as `draft | active | sold_out`
- `inventory_count`
- `face_value_usd`
- `retail_price_usd`
- `delivery_kind` as `direct_secret`
- `supports_trial`
- `version`
- `created_at`
- `updated_at`

### 8.3 Trial Provider Connection

Attributes:

- `trial_connection_id`
- `tenant_id`
- `provider_label`
- `endpoint_base_url`
- `api_key_masked`
- `target_model`
- `status` as `active | paused | needs_rotation`
- `notes` nullable
- `last_verified_at` nullable
- `version`
- `created_at`
- `updated_at`

### 8.4 Relay Evaluation

Attributes:

- `relay_evaluation_id`
- `tenant_id`
- `trial_connection_id`
- `replay_capsule_id`
- `provider_label`
- `endpoint_base_url`
- `target_model`
- `runner_mode`
- `sample_request_count`
- `estimated_tokens_saved`
- `overall_score`
- `verdict` as `healthy | warning | fail`
- `fingerprint_status`
- `protocol_status`
- `token_status`
- `multimodal_status`
- `detected_channel` nullable
- `summary`
- `created_at`

## 9. Control Plane API Additions

Suggested V1 endpoints:

```text
GET   /v1/merchant/workspace
POST  /v1/merchant/shops
POST  /v1/merchant/card-products
POST  /v1/merchant/trial-connections
POST  /v1/merchant/evaluations
GET   /v1/replay-capsules/:replayCapsuleId
```

Response contract for workspace:

- `merchant_enabled`
- `tenant_id`
- `shops`
- `card_products`
- `trial_connections`
- `recent_evaluations`

Authorization:

- any tenant member may read workspace data
- only tenant admins or owners may mutate merchant resources

## 10. Console UX

Merchant Center should surface:

- one-page operational overview instead of separate deep navigation first
- clear warning banner for dedicated trial keys
- product inventory table
- trial-connection list with masked credential presentation
- evaluation timeline with score, verdict, replay capsule id, and summary

V1 UX rule:

- every evaluation result must show `runner_mode`
- every evaluation result should show whether the displayed outcome came from a live run or replayed artifact

## 11. Delivery Slice For This Repository

The first implementation in this repo should ship:

- typed shared schemas for shop, card product, trial connection, relay evaluation, and workspace aggregate
- core-domain structs with basic validation
- replay-capsule recording for merchant evaluations with redacted summaries
- memory-backed control-plane endpoints for merchant workspace, replay lookup, and create actions
- tenant console route `/app/merchant`
- minimal tests for shared schema, control-plane behavior, and console service wiring

## 12. Follow-Ups

- secret stock ingestion and fulfillment ledger
- public storefront publishing
- order lifecycle and buyer access control
- probe-based real relay evaluator
- admin moderation and merchant risk rules
