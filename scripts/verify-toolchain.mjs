import { readFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const packageJson = JSON.parse(
  readFileSync(path.join(repoRoot, 'package.json'), 'utf8')
)
const rustToolchain = readFileSync(
  path.join(repoRoot, 'rust-toolchain.toml'),
  'utf8'
)

const expectedNode = packageJson.engines?.node
const expectedPnpm =
  packageJson.engines?.pnpm ?? packageJson.packageManager?.split('@')[1]
const expectedRust = rustToolchain.match(/^channel = "([^"]+)"$/m)?.[1]

if (!expectedNode || !expectedPnpm || !expectedRust) {
  console.error('Unable to resolve the expected Node, pnpm, or Rust versions.')
  process.exit(1)
}

function parseVersion(output) {
  const match = output.trim().match(/v?(\d+\.\d+\.\d+)/)
  return match?.[1] ?? output.trim()
}

function detect(command, args, label) {
  const result = spawnSync(command, args, { encoding: 'utf8' })
  if (result.error) {
    return {
      label,
      ok: false,
      actual: null,
      detail: `Missing ${label}.`
    }
  }

  if (result.status !== 0) {
    return {
      label,
      ok: false,
      actual: null,
      detail: (result.stderr || result.stdout || `Unable to read ${label}.`).trim()
    }
  }

  return {
    label,
    ok: true,
    actual: parseVersion(result.stdout)
  }
}

const checks = [
  {
    ...detect(process.execPath, ['--version'], 'Node.js'),
    expected: expectedNode,
    fix: `Install Node.js ${expectedNode} or reopen the devcontainer.`
  },
  {
    ...detect('pnpm', ['--version'], 'pnpm'),
    expected: expectedPnpm,
    fix: `Run "corepack enable && corepack prepare pnpm@${expectedPnpm} --activate".`
  },
  {
    ...detect('rustc', ['--version'], 'Rust'),
    expected: expectedRust,
    fix: `Run "rustup toolchain install ${expectedRust}" and "rustup component add clippy rustfmt".`
  }
]

const failures = checks.filter((check) => !check.ok || check.actual !== check.expected)

if (failures.length > 0) {
  console.error('Toolchain verification failed.')

  for (const failure of failures) {
    if (!failure.ok) {
      console.error(`- ${failure.label}: unavailable. ${failure.detail} ${failure.fix}`)
      continue
    }

    console.error(
      `- ${failure.label}: expected ${failure.expected}, found ${failure.actual}. ${failure.fix}`
    )
  }

  process.exit(1)
}

for (const check of checks) {
  console.log(`- ${check.label}: ${check.actual}`)
}

console.log('Toolchain verification passed.')
