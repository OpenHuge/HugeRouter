param(
  [ValidateSet('core', 'runtime', 'full', 'observability')]
  [string]$Mode = $(if ($env:HUGE_ROUTER_STACK_MODE) { $env:HUGE_ROUTER_STACK_MODE } else { 'core' }),
  [string[]]$Service = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ServiceList {
  param([string]$RequestedMode)

  switch ($RequestedMode) {
    'core' { return @('postgres', 'redis', 'nats') }
    'runtime' { return @('postgres', 'redis', 'nats', 'control-plane-api', 'ledger-worker', 'route-receipt-worker', 'routing-worker', 'edge-probe', 'audit-worker', 'notification-worker') }
    'observability' { return @('otel-collector', 'alertmanager', 'prometheus', 'grafana') }
    'full' { return @('postgres', 'redis', 'nats', 'control-plane-api', 'ledger-worker', 'route-receipt-worker', 'routing-worker', 'edge-probe', 'audit-worker', 'notification-worker', 'otel-collector', 'alertmanager', 'prometheus', 'grafana') }
    default { throw "Unsupported stack mode: $RequestedMode" }
  }
}

$composeFile = (Resolve-Path (Join-Path $PSScriptRoot '..\docker\compose.yaml')).Path
$composeArgs = @('-f', $composeFile)

if ($Mode -in @('runtime', 'full')) {
  $composeArgs += @('--profile', 'runtime')
}
if ($Mode -in @('observability', 'full')) {
  $composeArgs += @('--profile', 'observability')
}

$serviceArgs =
  if ($Service.Count -gt 0) {
    $Service
  } else {
    Get-ServiceList -RequestedMode $Mode
  }

if ($Service.Count -gt 0 -or $Mode -in @('core', 'observability')) {
  docker compose @composeArgs up -d --remove-orphans @serviceArgs
  exit 0
}

docker compose @composeArgs up -d --remove-orphans postgres redis nats

if ($Mode -eq 'full') {
  docker compose @composeArgs up -d --remove-orphans otel-collector alertmanager prometheus grafana
}

if ($env:HUGE_ROUTER_STACK_SKIP_INIT -ne 'true') {
  & (Join-Path $PSScriptRoot 'migrate.ps1') -Mode $Mode
  & (Join-Path $PSScriptRoot 'bootstrap.ps1') -Mode $Mode
}

docker compose @composeArgs up -d --remove-orphans control-plane-api ledger-worker route-receipt-worker routing-worker edge-probe audit-worker notification-worker
