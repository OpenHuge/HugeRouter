# QA, Release, and Documentation

[Back to Execution Workstreams](README.md)

Build contract tests, e2e test environments, release automation, and agent-facing implementation guidance.

## Task sequence

| Task ID | Phase | Title                                                                       | Depends On                |
| ------- | ----- | --------------------------------------------------------------------------- | ------------------------- |
| QAR-004 | PI-0  | Create agent-facing implementation guides and definition-of-done checklists | None                      |
| QAR-001 | PI-1  | Build protocol contract tests and golden fixtures                           | GWT-004, PAD-002, FND-006 |
| QAR-002 | PI-2  | Create end-to-end integration environment and seeded demo tenant            | CTL-005, GWT-005, MET-002 |
| QAR-003 | PI-2  | Implement release automation, versioning policy, and changelog generation   | FND-005                   |

## Detailed tasks

### QAR-004 — Create agent-facing implementation guides and definition-of-done checklists

**Phase:** PI-0  
**Estimated size:** S  
**Recommended owners:** technical-writing

**Depends on:** None

**Primary paths to touch:**

- `docs/execution`
- `README.md`

**Expected outputs:**

- execution docs
- task templates
- checklists

**Acceptance criteria:**

- agents can discover work without reading the whole spec
- task handoff template exists
- checklists align with CI and review requirements

**Implementation notes:**

- Golden fixtures should cover both success and failure behavior.
- Release automation should be reproducible and workspace-aware.
- Documentation should allow a new agent to start contributing quickly.

### QAR-001 — Build protocol contract tests and golden fixtures

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** qa, gateway

**Depends on:** GWT-004, PAD-002, FND-006

**Primary paths to touch:**

- `crates/testing-kit`
- `schemas/examples`
- `crates/protocol-openai`

**Expected outputs:**

- golden fixtures
- contract runner
- fixture update workflow

**Acceptance criteria:**

- protocol contract tests run in CI
- fixtures cover success and failure cases
- breaking fixture deltas require review

**Implementation notes:**

- Golden fixtures should cover both success and failure behavior.
- Release automation should be reproducible and workspace-aware.
- Documentation should allow a new agent to start contributing quickly.

### QAR-002 — Create end-to-end integration environment and seeded demo tenant

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** qa, platform

**Depends on:** CTL-005, GWT-005, MET-002

**Primary paths to touch:**

- `infra/docker`
- `infra/scripts`
- `apps/console-web`

**Expected outputs:**

- seed scripts
- e2e test environment
- reference demo scenario

**Acceptance criteria:**

- fresh environment can be seeded repeatably
- smoke tests cover UI + API + gateway flow
- demo tenant credentials are generated securely

**Implementation notes:**

- Golden fixtures should cover both success and failure behavior.
- Release automation should be reproducible and workspace-aware.
- Documentation should allow a new agent to start contributing quickly.

### QAR-003 — Implement release automation, versioning policy, and changelog generation

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** platform

**Depends on:** FND-005

**Primary paths to touch:**

- `.github/workflows`
- `docs/runbooks`
- `Cargo.toml`
- `package.json`

**Expected outputs:**

- release pipeline
- versioning policy
- generated changelogs

**Acceptance criteria:**

- tagging a release produces versioned artifacts
- workspace versions are consistent
- release notes are derived from merged changes

**Implementation notes:**

- Golden fixtures should cover both success and failure behavior.
- Release automation should be reproducible and workspace-aware.
- Documentation should allow a new agent to start contributing quickly.
