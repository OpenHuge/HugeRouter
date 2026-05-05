# Final Close Recovery Probe

Captured: 2026-05-03 17:19 Asia/Shanghai

Revoked credential:

- API key `ak_26c8457984e0d762` resolved before revoke and could call gateway with HTTP `200`.
- Revoke returned HTTP `200`, `is_active=false`, `version=2`.
- Internal resolve after revoke returned HTTP `403`, code `api_key_revoked`.
- Gateway call after revoke returned HTTP `401`.

Expired opening grant:

- Paid renewal grant `opengrant_10036` was forced to expired in sandbox DB fixture.
- Internal resolve before renewal returned HTTP `403`.
- Paid renewal restored the grant to active and internal resolve returned HTTP `200`.

Unpaid opening grant:

- Unpaid renewal grant `opengrant_10037` was forced to expired in sandbox DB fixture.
- Unpaid renewal returned `renewal_blocked`.
- Internal resolve after unpaid renewal remained HTTP `403`, code `api_key_expired`.

Test coverage:

- `gateway-api route_token`: 6 tests passed, including expired/revoked route token handling.
- `gateway-api events_protocol`: 12 tests passed, including usage and receipt publish failure behavior.

Interpretation:

- TR-CLOSE-06 passes for revoked and expired/future-state recovery boundaries.
