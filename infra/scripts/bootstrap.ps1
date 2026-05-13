param(
  [ValidateSet('core', 'runtime', 'full')]
  [string]$Mode = $(if ($env:HUGE_ROUTER_STACK_MODE) { $env:HUGE_ROUTER_STACK_MODE } else { 'runtime' })
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

& (Join-Path $PSScriptRoot 'migrate.ps1') -Mode $Mode

$composeFile = (Resolve-Path (Join-Path $PSScriptRoot '..\docker\compose.yaml')).Path
$bootstrapFile = (Resolve-Path (Join-Path $PSScriptRoot '..\sql\runtime-bootstrap.sql')).Path
$composeArgs = @('-f', $composeFile)

if ($Mode -in @('runtime', 'full')) {
  $composeArgs += @('--profile', 'runtime')
}
if ($Mode -eq 'full') {
  $composeArgs += @('--profile', 'observability')
}

$postgresUser = if ($env:POSTGRES_USER) { $env:POSTGRES_USER } else { 'huge_router' }
$postgresDb = if ($env:POSTGRES_DB) { $env:POSTGRES_DB } else { 'huge_router' }
$postgresPassword = if ($env:POSTGRES_PASSWORD) { $env:POSTGRES_PASSWORD } else { 'huge_router' }

Get-Content -Raw $bootstrapFile | docker compose @composeArgs exec -T -e "PGPASSWORD=$postgresPassword" postgres psql -h 127.0.0.1 -U $postgresUser -d $postgresDb -v ON_ERROR_STOP=1 -f -
if ($LASTEXITCODE -ne 0) {
  throw 'Failed to apply runtime bootstrap data.'
}

Write-Host "Applied runtime bootstrap data using stack mode $Mode."
