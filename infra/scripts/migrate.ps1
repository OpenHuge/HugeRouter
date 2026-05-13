param(
  [ValidateSet('core', 'runtime', 'full')]
  [string]$Mode = $(if ($env:HUGE_ROUTER_STACK_MODE) { $env:HUGE_ROUTER_STACK_MODE } else { 'runtime' })
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$composeFile = (Resolve-Path (Join-Path $PSScriptRoot '..\docker\compose.yaml')).Path
$schemaFile = (Resolve-Path (Join-Path $PSScriptRoot '..\sql\runtime-schema.sql')).Path
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

docker compose @composeArgs up -d postgres | Out-Null

$deadline = (Get-Date).AddSeconds($(if ($env:HUGE_ROUTER_STACK_TIMEOUT_SECONDS) { [int]$env:HUGE_ROUTER_STACK_TIMEOUT_SECONDS } else { 180 }))
while ((Get-Date) -lt $deadline) {
  docker compose @composeArgs exec -T postgres pg_isready -U $postgresUser -d $postgresDb *> $null
  if ($LASTEXITCODE -eq 0) {
    break
  }
  Start-Sleep -Seconds 2
}

if ((Get-Date) -ge $deadline) {
  throw 'Timed out waiting for postgres.'
}

Get-Content -Raw $schemaFile | docker compose @composeArgs exec -T -e "PGPASSWORD=$postgresPassword" postgres psql -h 127.0.0.1 -U $postgresUser -d $postgresDb -v ON_ERROR_STOP=1 -f -
if ($LASTEXITCODE -ne 0) {
  throw 'Failed to apply runtime schema.'
}

Write-Host "Applied runtime schema using stack mode $Mode."
