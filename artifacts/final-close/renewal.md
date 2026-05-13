# Final Close Renewal Boundary Probe

Captured: 2026-05-03 17:20 Asia/Shanghai

Sandbox setup:

- Created opening grants through the active control-plane API.
- Forced the grants to expired state through Postgres fixture updates.
- Inserted sandbox WeChat order rows directly into Postgres.
- No production payment provider was called.

Paid order recovery:

- grant id: `opengrant_10036`
- order id: `wx_final_paid_1777799991933`
- expired resolve before renewal: HTTP `403`
- `POST /v1/billing/renewal-intents`: HTTP `200`
- status: `renewed`
- reason code: `payment_paid_grant_recovered`
- previous grant status: `expired`
- previous expires at: `2000-01-01T00:00:00Z`
- resolve after renewal: HTTP `200`, status `active`

Unpaid order boundary:

- grant id: `opengrant_10037`
- order id: `wx_final_unpaid_1777799991933`
- expired resolve before renewal: HTTP `403`
- `POST /v1/billing/renewal-intents`: HTTP `200`
- status: `renewal_blocked`
- reason code: `payment_not_paid`
- resolve after renewal: HTTP `403`, code `api_key_expired`

List:

- `GET /v1/billing/renewal-intents?tenant_id=tenant_acme&project_id=proj_core`: HTTP `200`
- both paid and unpaid renewal intents are present.

Prepay route:

- `POST /v1/billing/wechat-pay/prepay`: HTTP `500`, code `wechat_pay_not_configured`
- message: local `WECHAT_PAY_MERCHANT_PRIVATE_KEY` / `WECHAT_PAY_MERCHANT_PRIVATE_KEY_PATH` is not configured.

Interpretation:

- TR-CLOSE-07 passes for sandbox/manual renewal semantics.
- Real WeChat prepay remains environment-gated by merchant secrets and was not attempted.
