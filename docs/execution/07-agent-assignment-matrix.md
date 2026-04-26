# Agent Assignment Matrix

[Back to Execution Index](README.md)

| Task ID | Suggested Agent Capability      | Primary Paths                                                             | Depends On                                           |
| ------- | ------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------- |
| FND-001 | backend-platform                | Cargo.toml, crates/\* ...                                                 | None                                                 |
| FND-002 | frontend-platform               | package.json, pnpm-workspace.yaml ...                                     | None                                                 |
| FND-003 | platform                        | infra/docker, infra/scripts ...                                           | None                                                 |
| FND-004 | backend-platform, security      | crates/config, crates/sdk-server ...                                      | FND-001                                              |
| FND-005 | platform                        | .github/workflows, turbo.json ...                                         | FND-001, FND-002                                     |
| FND-006 | platform, frontend-platform     | schemas/openapi, schemas/jsonschema ...                                   | FND-002                                              |
| GWT-001 | gateway                         | services/gateway-api, crates/sdk-server ...                               | FND-001, FND-003, FND-004                            |
| GWT-002 | gateway, security               | services/gateway-api, crates/authn-authz ...                              | CTL-001, SEC-001                                     |
| GWT-003 | gateway-architecture            | crates/protocol-ir, crates/core-domain ...                                | FND-001                                              |
| GWT-004 | gateway                         | services/gateway-api, crates/protocol-openai ...                          | GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001 |
| GWT-005 | gateway                         | services/gateway-api, crates/sdk-server ...                               | GWT-004                                              |
| GWT-006 | gateway                         | crates/protocol-anthropic, services/gateway-api                           | GWT-003, PAD-003, RTE-002                            |
| GWT-007 | gateway                         | crates/protocol-gemini, services/gateway-api                              | GWT-003, PAD-004, RTE-002                            |
| GWT-008 | realtime                        | services/realtime-gateway, crates/protocol-realtime ...                   | FND-001, OBS-001, RTE-004                            |
| CTL-001 | control-plane                   | services/control-plane-api, crates/sdk-server ...                         | FND-001, FND-004                                     |
| CTL-002 | control-plane                   | services/control-plane-api, crates/storage ...                            | CTL-001, DB-001, SEC-001                             |
| CTL-003 | control-plane                   | services/control-plane-api, crates/storage ...                            | CTL-001, PAD-001, RTE-001, SEC-003                   |
| CTL-004 | frontend-console                | apps/console-web, packages/ui-kit ...                                     | FND-002, FND-006                                     |
| CTL-005 | frontend-console                | apps/console-web/src/routes, packages/ts-api-client                       | CTL-002, CTL-004                                     |
| CTL-006 | frontend-console                | apps/console-web/src/routes/providers, apps/console-web/src/routes/routes | CTL-003, CTL-004, RTE-005                            |
| CTL-007 | frontend-console                | apps/console-web/src/routes/usage, apps/console-web/src/routes/billing    | MET-004, MET-005, CTL-004                            |
| DB-001  | data-platform                   | crates/storage, infra/docker ...                                          | FND-001, FND-003                                     |
| DB-002  | data-platform                   | crates/queue, services/\* ...                                             | FND-001, FND-003                                     |
| PAD-001 | gateway-architecture            | crates/provider-traits, crates/testing-kit ...                            | GWT-003, FND-001                                     |
| PAD-002 | provider-openai                 | crates/provider-openai                                                    | PAD-001, GWT-003                                     |
| PAD-003 | provider-anthropic              | crates/provider-anthropic                                                 | PAD-001, GWT-003                                     |
| PAD-004 | provider-gemini                 | crates/provider-gemini                                                    | PAD-001, GWT-003                                     |
| PAD-005 | provider-gateway                | crates/provider-gateway                                                   | PAD-001, RTE-004                                     |
| RTE-001 | routing                         | crates/routing-engine, crates/core-domain ...                             | GWT-003, DB-001                                      |
| RTE-002 | routing                         | crates/routing-engine, services/gateway-api                               | RTE-001, PAD-001                                     |
| RTE-003 | routing, sre                    | services/edge-probe, services/routing-worker ...                          | RTE-001, DB-002, OBS-001                             |
| RTE-004 | routing, sre                    | crates/routing-engine, services/gateway-api ...                           | RTE-002, RTE-003, OBS-001                            |
| RTE-005 | routing, control-plane          | services/control-plane-api, crates/routing-engine ...                     | RTE-002, RTE-003, CTL-001                            |
| MET-001 | billing-platform                | crates/metering, crates/ledger-models ...                                 | GWT-003, DB-002                                      |
| MET-002 | billing-platform                | services/ledger-worker, crates/ledger-models ...                          | MET-001, DB-001, DB-002                              |
| MET-003 | billing-platform                | crates/metering, crates/ledger-models ...                                 | MET-002, CTL-003                                     |
| MET-004 | billing-platform                | services/ledger-worker, crates/storage ...                                | MET-002, MET-003                                     |
| MET-005 | billing-platform, control-plane | services/control-plane-api, schemas/openapi                               | MET-004, CTL-001                                     |
| SEC-001 | security                        | crates/authn-authz, services/control-plane-api ...                        | FND-001, CTL-001                                     |
| SEC-002 | security, frontend-console      | apps/console-web, services/control-plane-api ...                          | CTL-004, SEC-001                                     |
| SEC-003 | security, control-plane         | crates/storage, services/control-plane-api ...                            | FND-004, DB-001                                      |
| SEC-004 | security, data-platform         | services/audit-worker, crates/queue ...                                   | DB-002, SEC-001, OBS-001                             |
| OBS-001 | sre                             | crates/telemetry, infra/monitoring ...                                    | FND-001, FND-003                                     |
| OBS-002 | sre                             | infra/monitoring, docs/runbooks                                           | OBS-001, GWT-001, CTL-001                            |
| OBS-003 | sre                             | infra/monitoring, services/notification-worker                            | OBS-002, SEC-004                                     |
| OBS-004 | sre, qa                         | crates/testing-kit, infra/scripts ...                                     | GWT-005, RTE-004, MET-002                            |
| QAR-001 | qa, gateway                     | crates/testing-kit, schemas/examples ...                                  | GWT-004, PAD-002, FND-006                            |
| QAR-002 | qa, platform                    | infra/docker, infra/scripts ...                                           | CTL-005, GWT-005, MET-002                            |
| QAR-003 | platform                        | .github/workflows, docs/runbooks ...                                      | FND-005                                              |
| QAR-004 | technical-writing               | docs/execution, README.md                                                 | None                                                 |
