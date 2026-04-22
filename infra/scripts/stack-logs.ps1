param(
  [ValidateSet('core', 'full', 'observability')]
  [string]$Mode = $(if ($env:HUGE_ROUTER_STACK_MODE) { $env:HUGE_ROUTER_STACK_MODE } else { 'core' }),
  [string[]]$Service = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ServiceList {
  param([string]$RequestedMode)

  switch ($RequestedMode) {
    'core' { return @('postgres', 'redis', 'nats') }
    'observability' { return @('otel-collector', 'prometheus', 'grafana') }
    'full' { return @('postgres', 'redis', 'nats', 'otel-collector', 'prometheus', 'grafana') }
    default { throw "Unsupported stack mode: $RequestedMode" }
  }
}

$composeFile = (Resolve-Path (Join-Path $PSScriptRoot '..\docker\compose.yaml')).Path
$composeArgs = @('-f', $composeFile)

if ($Mode -ne 'core') {
  $composeArgs += @('--profile', 'observability')
}

$serviceArgs =
  if ($Service.Count -gt 0) {
    $Service
  } else {
    Get-ServiceList -RequestedMode $Mode
  }

if ($serviceArgs.Count -gt 0) {
  docker compose @composeArgs logs -f --tail 200 @serviceArgs
} else {
  docker compose @composeArgs logs -f --tail 200
}
