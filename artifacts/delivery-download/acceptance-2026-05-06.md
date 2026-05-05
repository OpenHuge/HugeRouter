# Delivery Download Acceptance Evidence - 2026-05-06

## Task

- Plan: `OpenHuge-后端下载授权与取回-任务计划书-2026-05-06.md`
- Branch: `lab`
- Status: accepted

## Delivered Scope

- Added delivery download grant protocol responses with one-time plaintext token issuance.
- Added `delivery_download_grants` backend fact table with token hash, masked token projection, status, expiry, use count, and bound delivery facts.
- Download grant issuance starts from `activation_id` and rechecks activation, entitlement, active service segment, and artifact availability.
- Download retrieve accepts the token only from `Authorization: Bearer <token>`.
- Download retrieve hashes the supplied token and never reads token plaintext from storage.
- Download retrieve returns the stored encrypted `.hcbrowser` ciphertext only.
- Response headers include content type, disposition, length, no-store, nosniff, artifact sha256, and download grant id.
- Successful retrieve marks the grant `used` and increments `use_count`.
- Expired, revoked, already-used, invalid, cross-tenant, and entitlement-inactive requests are rejected.
- Concurrent use of the same token allows only one successful artifact response.
- Query-string token submission is explicitly covered as rejected.
- Current branch is lifecycle-aware: initial downloads bind the activation segment artifact; later continuation downloads bind the current active service segment artifact instead of an unqualified latest artifact.

## Out Of Scope

- No frontend download button or client save flow.
- No browser restore/import flow.
- No server deployment.
- No git commit or push performed.

## Verification

- `cargo fmt --all`: pass
- `cargo test -p control-plane-api delivery_download_grant --locked`: pass, 3 tests
- `cargo check -p control-plane-api -p protocol-ir --locked`: pass
- `cargo test -p control-plane-api -p protocol-ir --locked`: pass
  - `control-plane-api`: 82 tests
  - `protocol-ir`: 8 tests
- `cargo clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings`: pass
- `pnpm.CMD generate`: pass
  - Warning only: workspace requested Node `24.15.0`; local Node was `24.12.0`.

## Acceptance Tests Covered

- `delivery_download_grant_issues_once_and_retrieves_bound_ciphertext`
- `delivery_download_grant_rejects_missing_invalid_expired_revoked_and_concurrent_tokens`
- `delivery_download_grant_rechecks_entitlement_and_tenant_boundaries`
