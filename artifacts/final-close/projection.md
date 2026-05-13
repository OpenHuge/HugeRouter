# Final Close Projection Probe

Captured: 2026-05-03 17:14 Asia/Shanghai

Receipt and diagnostics:

- route receipt id: `routercpt_1005`
- `GET /v1/route-receipts/routercpt_1005`: HTTP `200`
- `GET /v1/route-receipts/routercpt_1005/diagnostics`: HTTP `200`
- selected target: `prvrsrc_openai_primary`
- provider attempt: status `success`, reason `provider_success`, latency `3ms`
- diagnostics source message id: `msg_routercpt_1005`

Ledger:

- `ledger_b87e085ecd287e7a14501878e5ab9b2483cacdb51c5b42f1967fdcfdc680936f`
- usage event id: `usageevt_1005`
- route receipt id: `routercpt_1005`
- input tokens: `11`
- output tokens: `7`
- billable cost micros: `102`

Usage projection:

- `GET /v1/usage/summary?tenant_id=tenant_acme&project_id=proj_core`: HTTP `200`
- event count: `2`
- input tokens: `22`
- output tokens: `14`
- billable price: `USD 0.000204`
- `GET /v1/usage/breakdown?...&group_by=provider`: HTTP `200`
- provider bucket: `openai`

Source fix applied:

- `services/control-plane-api/src/store.rs` now casts Postgres `SUM(bigint)` aggregates back to `BIGINT` before Rust decodes them as `i64`.

Interpretation:

- TR-CLOSE-03 passes in active runtime.
