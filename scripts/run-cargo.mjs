import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import path from 'node:path'

const cargoArgs = process.argv.slice(2)

if (cargoArgs.length === 0) {
  console.error('Usage: node ./scripts/run-cargo.mjs <cargo-args...>')
  process.exit(1)
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    stdio: 'inherit',
    shell: false,
    ...options
  })

  if (typeof result.status === 'number') {
    return result.status
  }

  return 1
}

function runOnWindows() {
  const vswhere = path.join(
    process.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)',
    'Microsoft Visual Studio',
    'Installer',
    'vswhere.exe'
  )

  if (!existsSync(vswhere)) {
    return run('cargo', cargoArgs)
  }

  const probe = spawnSync(
    vswhere,
    [
      '-latest',
      '-products',
      '*',
      '-requires',
      'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
      '-property',
      'installationPath'
    ],
    { encoding: 'utf8' }
  )

  const installPath = probe.stdout.trim()
  if (!installPath) {
    return run('cargo', cargoArgs)
  }

  const devCmd = path.join(installPath, 'Common7', 'Tools', 'VsDevCmd.bat')
  if (!existsSync(devCmd)) {
    return run('cargo', cargoArgs)
  }

  const cargoCommand = ['cargo', ...cargoArgs].join(' ')
  const command = `call "${devCmd}" -arch=x64 -host_arch=x64 && ${cargoCommand}`
  const cargoPathEntries = [
    process.env.HUGEROUTER_RUST_BIN,
    process.env.CARGO_HOME ? path.join(process.env.CARGO_HOME, 'bin') : null,
    path.join(process.env.USERPROFILE ?? '', '.cargo', 'bin')
  ].filter((entry) => entry && existsSync(entry))

  const result = spawnSync(command, {
    stdio: 'inherit',
    shell: true,
    env: {
      ...process.env,
      PATH: `${cargoPathEntries.join(';')};${process.env.PATH ?? ''}`
    }
  })

  if (typeof result.status === 'number') {
    return result.status
  }

  return 1
}

const status =
  process.platform === 'win32' ? runOnWindows() : run('cargo', cargoArgs)

process.exit(status)
