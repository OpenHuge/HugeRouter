# Data Storage and Events

[Back to Execution Workstreams](README.md)

Build the relational schema, migration strategy, repository layer, and asynchronous event bus foundation used by ledger, audit, and routing workers.

## Task sequence

| Task ID | Phase | Title                                                                 | Depends On       |
| ------- | ----- | --------------------------------------------------------------------- | ---------------- |
| DB-001  | PI-0  | Design relational schema, migrations, and repository primitives       | FND-001, FND-003 |
| DB-002  | PI-0  | Implement event bus abstractions and NATS-backed publishers/consumers | FND-001, FND-003 |

## Detailed tasks

### DB-001 — Design relational schema, migrations, and repository primitives

**Phase:** PI-0  
**Estimated size:** L  
**Recommended owners:** data-platform

**Depends on:** FND-001, FND-003

**Primary paths to touch:**

- `crates/storage`
- `infra/docker`
- `docs/architecture/data-storage-and-events.md`

**Expected outputs:**

- migration framework
- core tables
- repository abstractions

**Acceptance criteria:**

- fresh database bootstraps successfully
- rollback policy is documented
- repositories are integration-tested

**Implementation notes:**

- Repository abstractions should support transactional boundaries without hiding SQL realities.
- Version event schemas explicitly.
- Make retry and dead-letter behavior observable.

### DB-002 — Implement event bus abstractions and NATS-backed publishers/consumers

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** data-platform

**Depends on:** FND-001, FND-003

**Primary paths to touch:**

- `crates/queue`
- `services/*`
- `crates/sdk-server`

**Expected outputs:**

- event publishing API
- consumer worker harness
- subject naming conventions

**Acceptance criteria:**

- workers can consume and ack events
- message schemas are versioned
- dead-letter strategy is defined

**Implementation notes:**

- Repository abstractions should support transactional boundaries without hiding SQL realities.
- Version event schemas explicitly.
- Make retry and dead-letter behavior observable.
