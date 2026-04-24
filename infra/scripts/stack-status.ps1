param(
  [ValidateSet('core', 'runtime', 'full', 'observability')]
  [string]$Mode = $(if ($env:HUGE_ROUTER_STACK_MODE) { $env:HUGE_ROUTER_STACK_MODE } else { 'core' })
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ServiceList {
  param([string]$RequestedMode)

  switch ($RequestedMode) {
    'core' { return @('postgres', 'redis', 'nats') }
    'runtime' { return @('postgres', 'redis', 'nats', 'control-plane-api', 'gateway-api', 'ledger-worker', 'route-receipt-worker', 'routing-worker', 'edge-probe', 'audit-worker', 'notification-worker') }
    'observability' { return @('otel-collector', 'alertmanager', 'prometheus', 'grafana') }
    'full' { return @('postgres', 'redis', 'nats', 'control-plane-api', 'gateway-api', 'ledger-worker', 'route-receipt-worker', 'routing-worker', 'edge-probe', 'audit-worker', 'notification-worker', 'otel-collector', 'alertmanager', 'prometheus', 'grafana') }
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

$services = Get-ServiceList -RequestedMode $Mode
docker compose @composeArgs ps @services
Write-Host ''

foreach ($service in $services) {
  $containerId = (docker compose @composeArgs ps -q $service).Trim()
  if (-not $containerId) {
    Write-Host "${service}: not started"
    continue
  }

  $status = (docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' $containerId 2>$null).Trim()
  Write-Host "${service}: $status"
}
