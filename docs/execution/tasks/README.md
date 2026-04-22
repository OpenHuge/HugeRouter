# Task Catalog

[Back to Execution Index](../README.md)

This folder provides both a human-readable and a machine-readable view of the roadmap tasks.

## Files

- [Task Catalog (Markdown)](task-catalog.md)
- [Task Catalog (YAML)](task-catalog.yaml)
- [Phase 0 Foundation](phase-0-foundation.md)
- [Phase 1 Core Gateway](phase-1-core-gateway.md)
- [Phase 2 Expansion](phase-2-expansion.md)
- [Phase 3 Enterprise Hardening](phase-3-enterprise-hardening.md)

## How to use

For a human assignment flow:
1. read the phase file
2. select tasks with no unmet dependencies
3. confirm touched paths do not overlap with other active tasks
4. assign work using the handoff template in `../04-agent-operating-model.md`

For an automated orchestration flow:
1. parse `task-catalog.yaml`
2. filter tasks by `phase`, `stream`, or `deps`
3. assign tasks to agents with matching capability and free path ownership
