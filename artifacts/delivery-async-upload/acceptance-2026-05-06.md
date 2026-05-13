# Delivery Async Upload Acceptance - 2026-05-06

## Scope

- Added asynchronous production-operator delivery upload batches.
- `POST /v1/delivery-uploads` only stages encrypted account payloads and returns a queued batch.
- `/internal/delivery-uploads/{batch_id}/process` is the worker claim/process entrypoint protected by the internal bearer token.
- `GET /v1/delivery-uploads/{batch_id}` and `/v1/delivery-uploads/{batch_id}/items` expose redacted batch state and row-level results.

## Concurrency and Idempotency

- Upload batches persist in `delivery_upload_batches`.
- Row payloads persist in `delivery_upload_batch_items`.
- Database uniqueness covers `(tenant_id, project_id, source_file_sha256)` and `(tenant_id, project_id, idempotency_key)`.
- Worker processing only allows one `processing` batch per tenant/project scope.
- Request-time upload does not write `delivery_artifacts`; artifacts are created only by worker processing.

## Security and Redaction

- Public batch item responses expose `payload_sha256`, size, status, and errors only.
- Public responses do not expose `payload_base64`, ciphertext, or raw account payloads.
- Worker-created artifacts still reuse existing encrypted artifact storage and public artifact projections.

## Operations Visibility

- Delivery operations overview includes upload batch and upload item totals/status counts.
- Delivery operations timelines include staged and processed upload item events for each delivery.
- Failed/rejected upload items appear in the delivery operations exception queue with upload batch/item ids.

## Validation Evidence

- `cargo check -p control-plane-api -p protocol-ir --locked` passed.
- `cargo test -p control-plane-api delivery_upload --locked` passed: 2 tests.
- `cargo test -p control-plane-api -p protocol-ir --locked` passed: control-plane-api 86 tests, protocol-ir 8 tests.
- `cargo fmt --all -- --check` passed.
- `cargo clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings` passed.
- `pnpm.CMD generate` passed with only the existing Node version warning.
