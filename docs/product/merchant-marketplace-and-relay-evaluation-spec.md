# AI Resource Library And Relay Evidence Spec

[Back to Docs Index](../README.md)

## 1. Problem

HugeRouter already has strong control-plane primitives for tenancy, provider resources, API keys, routes, usage, and billing, but the main branch has shifted away from being a pure route-operator console.

The current gap is product framing:

- the codebase now exposes seller profiles, listings, escrow-ready orders, trial relay connections, and replay-backed evaluations
- the product spec still describes a narrow merchant center for card-secret style goods
- the user-facing positioning now needs to reflect `ku0.com`, a trusted AI resource library with account, relay, and information surfaces

Without this spec update, implementation will drift between old merchant vocabulary and the newer resource-library model.

## 2. Product Positioning

Phase 1 should be positioned as:

`ku0.com - Ku`

`Trusted AI Resource Library`

The first product line is organized into three libraries:

- `Account Library`
  Covers account recharge, purchase, and inventory-style supply. Phase 1 implements the control-plane side of this through vendor profiles, listings, and escrow orders.
- `Relay Library`
  Covers relay quality inspection, trust summaries, and future unified gateway access. Phase 1 implements this through trial connections, evaluations, and replay capsules.
- `Info Library`
  Covers disclosure, announcements, operator notes, and risk communication around AI resources. Phase 1 implements only the minimum disclosure layer, not a full content product.

## 3. Phase 1 Goals

- make the existing main-branch marketplace code read as a trusted AI resource library rather than a generic merchant tool
- preserve the current trust model: platform-visible seller identity, reviewed listings, escrow-ready orders, and replay-backed quality evidence
- keep relay quality evidence first-class because it is the most defensible trust surface in the current implementation
- scope the info library to structured disclosure and audit support instead of launching a social or forum product too early

## 4. Non-Goals

- anonymous peer-to-peer chat trading
- public buyer storefronts
- free-form account resale without review or escrow
- rumor-style community feeds without operator accountability
- external payment settlement, payout, or refund rails
- general-purpose public gateway runtime inside the main branch

## 5. Current Implementation Boundary

The current codebase already supports:

- verified seller profiles
- reviewed AI resource listings
- escrow-ready order records
- relay trial connection onboarding
- replay-backed relay quality evaluation
- replay capsule detail inspection

The current codebase does not yet support:

- dedicated recharge flows
- pool wholesale operations
- dedicated info posts or disclosure feed objects
- public search, discovery, or buyer self-service views
- production unified external gateway access for third parties

Phase 1 spec and implementation should stay inside that boundary.

## 6. Functional Scope

### 6.1 Account Library

Tenant admins can:

- open a verified vendor profile
- create reviewed account or access listings
- inspect listing trust metadata
- see escrow-ready order history with evidence and dispute state

Current technical mapping:

- `MerchantShop` is the vendor profile
- `CardProduct` is the account or access listing
- `TradeOrder` is the protected order record

### 6.2 Relay Library

Tenant admins can:

- register relay endpoints with dedicated trial credentials
- run replay-backed quality evaluation
- inspect verdict, score, quality dimensions, and replay evidence

Current technical mapping:

- `TrialConnection` is the relay source registration
- `RelayEvaluation` is the quality result
- `ReplayCapsule` is the redacted support artifact

### 6.3 Info Library

Phase 1 supports operator-facing disclosure only:

- seller announcement text
- listing trust metadata
- evaluation summary text
- replay-backed evidence references

The first implementation should not add a standalone forum or newsfeed. Instead, it should make disclosure visible inside the existing library views.

## 7. Trust Model

- counterparties are pseudonymous to each other but auditable by the platform
- only reviewed listings may be associated with protected orders
- relay trust must be supported by replay-backed evidence rather than self-report
- dispute handling must use order state and evidence records, not screenshots as the primary source
- info-library content must be attributable to a vendor profile, evaluation artifact, or operator workflow

## 8. Control Plane Contract

The Phase 1 workspace contract should continue to expose:

- `merchant_enabled`
- `tenant_id`
- `shops`
- `card_products`
- `recent_orders`
- `trial_connections`
- `recent_evaluations`

This contract is already the correct Phase 1 backbone for the resource-library product.

## 9. Console UX Direction

The `/app/merchant` route should be treated as the `ku0.com` resource-library workspace.

The UI should make three things obvious:

- `Account Library`
  Vendor profile, reviewed listings, and escrow orders are the live account-supply surface.
- `Relay Library`
  Trial connections, evaluations, and replay capsules are the live relay-inspection surface.
- `Info Library`
  Disclosure and risk communication exist today as announcement and evidence surfaces, with a richer content model deferred.

## 10. Implementation Sequence

1. Update product docs and UX copy to use the resource-library framing
2. Keep extending the current seller, listing, order, and relay evidence objects instead of introducing a second parallel domain
3. Add explicit info-library domain objects only after the account and relay workflows are stable and internally coherent
