$ErrorActionPreference = 'Stop'
$runner = Join-Path $PSScriptRoot '..\verify.ts'
bun run $runner intel
exit $LASTEXITCODE
