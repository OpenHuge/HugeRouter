# ku0.com - Ku

[Back to Docs Index](../README.md)

## Trusted AI Resource Library Phase 1

Phase 1 narrows the main product line into `ku0.com`, a trusted AI resource library rather than a generic marketplace or full public gateway product.

The product promise is:

- counterparties can trade or evaluate AI resources without exposing full identity to each other
- the platform keeps seller identity tier, order evidence, relay quality artifacts, and dispute-ready records as the system of record
- trust comes from review, quality evidence, escrow state, and disclosure, not from screenshots, group size, or self-reported claims

## Product Positioning

`ku0.com` groups the first product line into three resource libraries:

- `Account Library`
  Covers account recharge, purchase, and pooled inventory or wholesale-style supply. In the current implementation this is represented by verified seller profiles, reviewed listings, and escrow-ready order records.
- `Relay Library`
  Covers relay endpoint onboarding, quality inspection, unified gateway access decisions, and replay-backed evidence. This is the most complete part of the current Phase 1 implementation.
- `Info Library`
  Covers disclosure, operator notes, risk warnings, quality summaries, and discussion around AI resources. In the current implementation this exists only as seller announcement and quality-evidence surfaces; a dedicated info feed is still pending.

## Current Implementation Snapshot

As of April 25, 2026, the main branch already includes:

- verified seller profiles with alias, identity level, guarantee deposit, and dispute rate
- reviewed AI resource listings with risk tier, escrow mode, review status, and required KYC level
- escrow order records with buyer or seller aliases, evidence state, and dispute state
- relay trial connections
- replay-backed relay quality evaluations
- replay capsule detail views for support and dispute review

Still not implemented in the main branch:

- dedicated account recharge workflow
- batch inventory operations or pool wholesale controls
- unified external gateway access for third-party buyers
- dedicated info posts, disclosure threads, or public discussion surfaces
- public storefront or buyer-facing search pages

## Phase 1 Scope

Phase 1 should ship a credible private operations console for trusted AI resources:

- `Account Library`
  Verified vendors, reviewed listings, account or access SKU metadata, escrow orders, and dispute-ready evidence.
- `Relay Library`
  Trial connection onboarding, replay-backed quality inspection, relay trust summaries, and operator review artifacts.
- `Info Library`
  Structured disclosures and operator-facing summaries derived from listing announcements, evaluation summaries, and replay evidence. A standalone feed UI is not required for the first cut.

## Product Rules

- buyers and sellers see aliases, trust tier, escrow state, evidence state, and dispute state instead of raw identity data
- only reviewed listings can produce protected escrow orders
- dedicated trial keys are required for relay quality inspection
- the platform never treats chat logs as the source of truth for transactions
- trust signals come from verified orders and replay-backed evidence, not follower counts or forwarded screenshots
- the info library is disclosure-first and audit-friendly, not an anonymous rumor board

## Phase 1 Data Model

- `MerchantShop`
  Acts as the verified account-library vendor profile and carries `seller_alias`, `identity_level`, `guarantee_deposit_usd`, and `dispute_rate_bps`
- `CardProduct`
  Acts as the reviewed account or access listing and carries `risk_tier`, `review_status`, `escrow_mode`, `required_kyc_level`, and `evidence_requirement`
- `TradeOrder`
  Represents the escrow state machine with buyer or seller aliases, order state, escrow mode, evidence state, dispute state, and amount
- `TrialConnection`
  Represents a relay-library source registered only for quality inspection or controlled access testing
- `RelayEvaluation` and `ReplayCapsule`
  Represent the replay-backed evidence layer used to explain relay trust, support review, and dispute handling

## Implementation Order

1. Align the spec and UI around `Account Library`, `Relay Library`, and `Info Library`
2. Keep Phase 1 implementation centered on vendor profiles, listings, escrow records, and relay evidence
3. Add structured info-library objects only after the current account and relay workflows are stable
