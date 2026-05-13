# Final Close API Key Lifecycle Probe

Captured: 2026-05-03 17:18 Asia/Shanghai

Session:

- email: `ops@huge-router.dev`
- workspace: `platform-admin`
- session id: `sess_10002`

Create/list/resolve:

- `POST /v1/api-keys`: HTTP `200`
- created api key id: `ak_26c8457984e0d762`
- key prefix: `akp_fi...`
- active: `true`
- `GET /v1/api-keys`: HTTP `200`, created key present
- `POST /internal/gateway/api-keys/resolve`: HTTP `200`
- resolved status: `active`
- tenant/project: `tenant_acme` / `proj_core`

Gateway use before revoke:

- `POST /v1/chat/completions`: HTTP `200`
- content: `final-close mock ok`
- route receipt id: `routercpt_1007`
- selected target: `prvrsrc_openai_primary`

Revoke and fail-closed:

- `POST /v1/api-keys/ak_26c8457984e0d762/revoke`: HTTP `200`
- revoked active flag: `false`
- version: `2`
- internal resolve after revoke: HTTP `403`, code `api_key_revoked`
- gateway call after revoke: HTTP `401`

Interpretation:

- TR-CLOSE-04 passes in active runtime.
