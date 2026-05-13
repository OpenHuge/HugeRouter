import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const mode = process.argv[2]

function pnpmInvocation(scriptName) {
  if (process.platform === 'win32') {
    return ['cmd.exe', ['/d', '/s', '/c', 'pnpm.cmd', scriptName]]
  }

  return ['pnpm', [scriptName]]
}

const pipelines = {
  lint: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript lint', pnpmInvocation('js:lint')],
    ['Rust lint', pnpmInvocation('rust:lint')]
  ],
  typecheck: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript typecheck', pnpmInvocation('js:typecheck')],
    ['Rust check', pnpmInvocation('rust:check')]
  ],
  test: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript test', pnpmInvocation('js:test')],
    ['Rust test', pnpmInvocation('rust:test')]
  ],
  build: [
    ['Verify toolchain', [process.execPath, ['./scripts/verify-toolchain.mjs']]],
    ['JavaScript build', pnpmInvocation('js:build')],
    ['Rust check', pnpmInvocation('rust:check')]
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
