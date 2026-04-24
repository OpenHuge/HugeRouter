# Support

HugeRouter is under active development. Use GitHub issues for reproducible bugs, feature requests, and scoped technical tasks.

## Before Opening An Issue

Run the local checks that match the affected area:

- Toolchain: `pnpm verify:toolchain`
- JavaScript: `pnpm lint`, `pnpm typecheck`, `pnpm test`
- Rust: `pnpm rust:check`, `pnpm rust:test`
- Runtime stack: `just stack-up`, `just stack-wait`, `just status`
- Contract drift: `pnpm generate` followed by the protocol artifact test

## What To Include

- component or route affected
- exact command or request
- expected and actual result
- relevant trace ID, route receipt ID, usage event ID, or log excerpt
- whether the issue reproduces on a fresh stack

Do not include provider API keys, cookies, tenant-private data, unredacted prompts, or production credentials.
