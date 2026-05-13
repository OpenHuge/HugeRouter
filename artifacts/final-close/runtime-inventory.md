# Final Close Runtime Inventory

Captured: 2026-05-03 17:22 Asia/Shanghai

Branch:

- `lab...origin/lab`

Runtime dependencies:

- Postgres container: `huge-router-local-postgres-1`
- NATS: `nats://127.0.0.1:4222`
- Provider resources `prvrsrc_openai_primary` and `prvrsrc_openai_backup` were patched in Postgres to sandbox endpoint `http://127.0.0.1:18082/v1`.

Current binaries:

- `target/debug/control-plane-api.exe`: LastWriteTime `2026/5/3 17:09:08`
- `target/debug/gateway-api.exe`: LastWriteTime `2026/5/3 17:00:23`
- `target/debug/route-receipt-worker.exe`: LastWriteTime `2026/5/3 16:58:54`
- `target/debug/ledger-worker.exe`: LastWriteTime `2026/5/3 16:58:57`

Active listeners:

- `127.0.0.1:18081` -> `control-plane-api` PID `24784`, `/healthz` returned `200`
- `127.0.0.1:18080` -> `gateway-api` PID `13776`, `/ready` returned `200`
- `127.0.0.1:18082` -> mock OpenAI-compatible upstream PID `4448`

Workers:

- `route-receipt-worker` PID `33312`
- `ledger-worker` PID `33516`

Old duplicate `18080` and `18081` service processes were stopped before rebuild. The final active runtime has one listener per port.
