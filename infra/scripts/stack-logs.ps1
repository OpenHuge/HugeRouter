param(
  [string]$Service = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$composeFile = (Resolve-Path (Join-Path $PSScriptRoot '..\docker\compose.yaml')).Path

if ($Service) {
  docker compose -f $composeFile logs -f $Service
} else {
  docker compose -f $composeFile logs -f
}

