param(
  [int]$TimeoutSeconds = 180
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$checks = @(
  'http://127.0.0.1:8222/healthz',
  'http://127.0.0.1:13133/',
  'http://127.0.0.1:9090/-/ready',
  'http://127.0.0.1:3001/api/health'
)

foreach ($url in $checks) {
  while ((Get-Date) -lt $deadline) {
    try {
      Invoke-WebRequest -Uri $url -UseBasicParsing | Out-Null
      break
    } catch {
      Start-Sleep -Seconds 2
    }
  }

  if ((Get-Date) -ge $deadline) {
    throw "Timed out waiting for $url"
  }
}
