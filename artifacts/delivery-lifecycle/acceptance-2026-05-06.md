# Delivery Lifecycle Acceptance Evidence - 2026-05-06

## Task

- Plan: `OpenHuge-后端权益生命周期与服务期截断续接策略-任务计划书-2026-05-06.md`
- Branch: `lab`
- Status: accepted

## Delivered Scope

- Delivery entitlement now keeps activation-time service window fields: `service_starts_at`, `service_ends_at`, and `version`.
- Delivery activation starts customer service at redemption time, not at preparation time.
- Delivery artifacts can carry `carrier_valid_until`, which is used to model upstream carrier expiry.
- Lifecycle reconciliation creates and maintains service segments for the same entitlement.
- Segment `effective_until` is clamped to the customer entitlement end time, so carrier rollover does not cut customer service early.
- Reconciliation can attach a prepared continuation artifact when the current carrier segment is expiring or has ended.
- Reconciliation fail-closes to `needs_manual_supply` when no usable continuation artifact exists.
- Entitlement extension extends the existing entitlement and writes lifecycle audit events, without minting a new redemption code.
- Download grant issue and consume paths require an active service segment and reject stale segment artifacts.
- Protocol/OpenAPI/schema/TypeScript generated artifacts were regenerated.

## Out Of Scope

- No frontend behavior change.
- No payment workflow change.
- No deploy or production data migration execution.
- No git commit or push performed.

## Verification

- `cargo fmt --all`: pass
- `cargo check -p control-plane-api -p protocol-ir --locked`: pass
- `cargo test -p control-plane-api delivery_lifecycle --locked`: pass, 3 tests
- `pnpm.CMD generate`: pass
  - Warning only: workspace requested Node `24.15.0`; local Node was `24.12.0`.
- `cargo test -p control-plane-api -p protocol-ir --locked`: pass
  - `control-plane-api`: 82 tests
  - `protocol-ir`: 8 tests
- `cargo clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings`: pass

## Acceptance Tests Added

- `delivery_lifecycle_creates_segment_and_continuation_without_cutting_customer_service`
- `delivery_lifecycle_fail_closes_when_continuation_supply_is_missing`
- `delivery_lifecycle_extends_same_entitlement_and_expiry_blocks_download`
