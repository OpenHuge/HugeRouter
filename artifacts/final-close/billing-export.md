# Final Close Billing Export Probe

Captured: 2026-05-03 17:18 Asia/Shanghai

Create:

- `POST /v1/billing/exports`
- tenant/project: `tenant_acme` / `proj_core`
- window: `2026-05-01T00:00:00Z` to `2026-05-04T00:00:00Z`
- format: `csv`
- HTTP status: `202`
- export job id: `export_1777799909`
- initial status: `queued`

List/fetch/download:

- `GET /v1/billing/exports?tenant_id=tenant_acme&project_id=proj_core`: HTTP `200`, job present
- `GET /v1/billing/exports/export_1777799909`: HTTP `200`
- fetched job status: `completed`
- completed at: `2026-05-03T09:18:29.2280826Z`
- `GET /v1/billing/exports/export_1777799909/download`: HTTP `200`
- content-type: `text/csv`
- CSV header: `bucket,provider_id,model_alias,input_tokens,output_tokens,cached_input_tokens,provider_cost,billable_price`

Interpretation:

- TR-CLOSE-05 passes in active runtime.
