import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const mode = process.argv[2]

const pipelines = {
  lint: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript lint', ['pnpm', ['js:lint']]],
    ['Rust lint', ['pnpm', ['rust:lint']]]
  ],
  typecheck: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript typecheck', ['pnpm', ['js:typecheck']]],
    ['Rust check', ['pnpm', ['rust:check']]]
  ],
  test: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript test', ['pnpm', ['js:test']]],
    ['Rust test', ['pnpm', ['rust:test']]]
  ],
  build: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript build', ['pnpm', ['js:build']]],
    ['Rust check', ['pnpm', ['rust:check']]]
  ]
}

if (!mode || !(mode in pipelines)) {
  console.error('Usage: node ./scripts/run-quality-pipeline.mjs <lint|typecheck|test|build>')
  process.exit(1)
}

for (const [label, [command, args]] of pipelines[mode]) {
  console.log(`==> ${label}`)

  const result = spawnSync(command, args, {
    cwd: repoRoot,
    stdio: 'inherit',
    shell: false
  })

  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }

  console.log('')
}
