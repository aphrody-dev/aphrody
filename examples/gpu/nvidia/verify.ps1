$ErrorActionPreference = 'Stop'
$runner = Join-Path $PSScriptRoot '..\verify.ts'
bun run $runner nvidia
exit $LASTEXITCODE
