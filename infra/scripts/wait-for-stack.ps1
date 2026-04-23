param(
  [ValidateSet('core', 'runtime', 'full', 'observability')]
  [string]$Mode = $(if ($env:HUGE_ROUTER_STACK_MODE) { $env:HUGE_ROUTER_STACK_MODE } else { 'core' }),
  [int]$TimeoutSeconds = $(if ($env:HUGE_ROUTER_STACK_TIMEOUT_SECONDS) { [int]$env:HUGE_ROUTER_STACK_TIMEOUT_SECONDS } else { 180 })
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ServiceList {
  param([string]$RequestedMode)

  switch ($RequestedMode) {
    'core' { return @('postgres', 'redis', 'nats') }
    'runtime' { return @('postgres', 'redis', 'nats', 'control-plane-api', 'gateway-api', 'ledger-worker', 'route-receipt-worker') }
    'observability' { return @('otel-collector', 'alertmanager', 'prometheus', 'grafana') }
    'full' { return @('postgres', 'redis', 'nats', 'control-plane-api', 'gateway-api', 'ledger-worker', 'route-receipt-worker', 'otel-collector', 'alertmanager', 'prometheus', 'grafana') }
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
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)

while ((Get-Date) -lt $deadline) {
  $allReady = $true

  foreach ($service in $services) {
    $containerId = (docker compose @composeArgs ps -q $service).Trim()

    if (-not $containerId) {
      $allReady = $false
      continue
    }

    $status = (docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' $containerId 2>$null).Trim()

    if ($status -ne 'healthy' -and $status -ne 'running') {
      $allReady = $false
    }
  }

  if ($allReady) {
    Write-Host "Stack mode $Mode is ready."
    exit 0
  }

  Start-Sleep -Seconds 2
}

Write-Error "Timed out waiting for stack mode $Mode after ${TimeoutSeconds}s."

foreach ($service in $services) {
  $containerId = (docker compose @composeArgs ps -q $service).Trim()

  if (-not $containerId) {
    Write-Error "- ${service}: not started"
    continue
  }

  $status = (docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' $containerId 2>$null).Trim()
  Write-Error "- ${service}: $status"
}
