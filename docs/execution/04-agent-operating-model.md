# Agent Operating Model

[Back to Execution Index](README.md)

This program is designed for execution by multiple coding agents or sub-teams.

## Agent roles

### Interface owner
Owns high-contention contracts such as:
- protocol IR
- adapter traits
- storage repositories
- OpenAPI definitions
- RBAC policy models

### Feature implementer
Builds against stable interfaces inside a service, crate, or route group.

### Integrator
Merges generated artifacts, fixes cross-package type issues, and keeps CI green.

### Verifier
Writes or extends contract tests, integration tests, smoke tests, and performance baselines.

## How to assign work

A task is safe to assign to an agent when:

- the touched paths are mostly isolated
- dependencies are already merged or stubbed behind stable interfaces
- acceptance criteria can be validated locally or in CI
- ownership of boundary files is clear

## Suggested handoff template

Use this structure for every task assignment:

```text
Task ID:
Goal:
Dependencies already merged:
Primary paths to edit:
Out of scope:
Required tests:
Acceptance criteria:
```

## Review loop

Every completed task should include:

1. a concise change summary
2. affected paths
3. migrations or config changes
4. test evidence
5. follow-up tasks or known gaps

## When to stop and split work

Split a task into two or more child tasks if:

- the task touches both gateway hot path and control plane UI
- the task requires a migration plus a broad frontend workflow
- the task changes a shared contract and multiple adapters at once
- the task would likely exceed one focused pull request
