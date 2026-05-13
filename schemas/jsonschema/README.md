# JSON Schema Contracts

The JSON Schema files in this directory are generated from the Rust contract
types and committed so backend, frontend, and docs tooling can share the same
shapes.

Refresh them with:

```sh
pnpm generate
```

`contracts.manifest.json` is the synchronization anchor for the pipeline. Its
digest is embedded into the generated TypeScript package metadata so drift shows
up in tests immediately.
