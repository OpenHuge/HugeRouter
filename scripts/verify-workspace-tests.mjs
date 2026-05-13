import { existsSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '..'
)
const policy = JSON.parse(
  readFileSync(
    path.join(repoRoot, 'scripts', 'workspace-test-policy.json'),
    'utf8'
  )
)

const placeholderAllowed = policy.placeholderAllowed ?? {}
const noTestScriptAllowed = policy.noTestScriptAllowed ?? {}

function readWorkspacePackages(rootDir) {
  const baseDir = path.join(repoRoot, rootDir)

  return readdirSync(baseDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(baseDir, entry.name, 'package.json'))
    .filter((manifestPath) => existsSync(manifestPath))
}

function readPackageManifest(manifestPath) {
  const raw = readFileSync(manifestPath, 'utf8')
  const manifest = JSON.parse(raw)

  return {
    manifestPath,
    name: manifest.name,
    testScript: manifest.scripts?.test ?? null
  }
}

function isPlaceholderTest(script) {
  return (
    typeof script === 'string' &&
    script.includes('console.log') &&
    /(No tests|reserved for)/i.test(script)
  )
}

const manifests = [
  ...readWorkspacePackages('apps'),
  ...readWorkspacePackages('packages')
]
  .map(readPackageManifest)
  .sort((left, right) => left.name.localeCompare(right.name))

const meaningful = []
const placeholder = []
const exempt = []
const unexpectedPlaceholder = []
const unexpectedMissing = []

for (const manifest of manifests) {
  if (isPlaceholderTest(manifest.testScript)) {
    if (placeholderAllowed[manifest.name]) {
      placeholder.push(manifest)
      continue
    }

    unexpectedPlaceholder.push(manifest)
    continue
  }

  if (manifest.testScript === null) {
    if (noTestScriptAllowed[manifest.name]) {
      exempt.push(manifest)
      continue
    }

    unexpectedMissing.push(manifest)
    continue
  }

  meaningful.push(manifest)
}

console.log('Workspace test audit')
console.log('')

console.log('Meaningful test scripts:')
for (const manifest of meaningful) {
  console.log(`- ${manifest.name}: ${manifest.testScript}`)
}

console.log('')
console.log('Explicit placeholder tests (not counted as coverage):')
for (const manifest of placeholder) {
  console.log(`- ${manifest.name}: ${placeholderAllowed[manifest.name]}`)
}

console.log('')
console.log('Packages allowed to omit test scripts:')
for (const manifest of exempt) {
  console.log(`- ${manifest.name}: ${noTestScriptAllowed[manifest.name]}`)
}

if (unexpectedPlaceholder.length > 0 || unexpectedMissing.length > 0) {
  console.error('')
  console.error('Unexpected workspace test policy drift detected.')

  for (const manifest of unexpectedPlaceholder) {
    console.error(
      `- ${manifest.name}: placeholder test script must be converted to a real test or added to scripts/workspace-test-policy.json with a reason.`
    )
  }

  for (const manifest of unexpectedMissing) {
    console.error(
      `- ${manifest.name}: missing a test script and not listed in scripts/workspace-test-policy.json.`
    )
  }

  process.exit(1)
}

console.log('')
console.log('Workspace test policy check passed.')
