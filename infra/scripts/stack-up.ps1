Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$composeFile = (Resolve-Path (Join-Path $PSScriptRoot '..\docker\compose.yaml')).Path
docker compose -f $composeFile up -d --remove-orphans

