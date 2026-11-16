# DirectX 12 & HLSL Verification Script for gpui-os
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspaceRoot = (Resolve-Path "$scriptDir\..\..\..").Path
$wgpuCli = Join-Path $workspaceRoot "target\debug\gpui-wgpu.exe"

$dxc = "$env:USERPROFILE\.local\bin\dxc.exe"
if (-not (Test-Path $dxc)) {
    $dxc = "$env:USERPROFILE\.local\sdk\directx\dxc\bin\x64\dxc.exe"
}
$fxc = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\fxc.exe"

Write-Host "=========================================================="
Write-Host "   gpui-os DirectX 12 & HLSL Verification Suite"
Write-Host "=========================================================="
Write-Host "DXC Path: $dxc"
Write-Host "FXC Path: $fxc"
Write-Host "WGPU CLI: $wgpuCli"
Write-Host ""

# 1. Verify DXC version
Write-Host "[1/6] Querying DXC Version..."
$dxcVer = & $dxc --version
Write-Host "DXC Version: $dxcVer"
Write-Host ""

# 2. Compile Wave Intrinsics prefix_sum.hlsl (SM 6.6)
Write-Host "[2/6] Compiling prefix_sum.hlsl with DXC (SM 6.6)..."
$prefixSumHlsl = Join-Path $scriptDir "prefix_sum.hlsl"
$prefixSumDxil = Join-Path $scriptDir "prefix_sum.dxil"
& $dxc -T cs_6_6 -E main -O3 -Qstrip_reflect -Fo $prefixSumDxil $prefixSumHlsl
if (Test-Path $prefixSumDxil) {
    $size = (Get-Item $prefixSumDxil).Length
    Write-Host "Compiled prefix_sum.dxil successfully ($size bytes)"
} else {
    throw "Failed to compile prefix_sum.dxil"
}
Write-Host ""

# 3. Compile gpui_quad.hlsl with DXC (SM 6.6)
Write-Host "[3/6] Compiling gpui_quad.hlsl with DXC (SM 6.6)..."
$quadHlsl = Join-Path $scriptDir "gpui_quad.hlsl"
$quadVsDxil = Join-Path $scriptDir "gpui_quad_vs.dxil"
$quadPsDxil = Join-Path $scriptDir "gpui_quad_ps.dxil"

& $dxc -T vs_6_6 -E vs_main -O3 -Fo $quadVsDxil $quadHlsl
& $dxc -T ps_6_6 -E ps_main -O3 -Fo $quadPsDxil $quadHlsl
Write-Host "Compiled gpui_quad_vs.dxil ($((Get-Item $quadVsDxil).Length) bytes) and gpui_quad_ps.dxil ($((Get-Item $quadPsDxil).Length) bytes)"
Write-Host ""

# 4. Compile gpui_quad.hlsl with legacy FXC (SM 5.0) for backward compatibility
Write-Host "[4/6] Verifying gpui_quad.hlsl compatibility with legacy FXC (SM 5.0)..."
$quadVsDxbc = Join-Path $scriptDir "gpui_quad_vs.dxbc"
$quadPsDxbc = Join-Path $scriptDir "gpui_quad_ps.dxbc"

& $fxc /nologo /T vs_5_0 /E vs_main /Fo $quadVsDxbc $quadHlsl
& $fxc /nologo /T ps_5_0 /E ps_main /Fo $quadPsDxbc $quadHlsl
Write-Host "Compiled gpui_quad_vs.dxbc ($((Get-Item $quadVsDxbc).Length) bytes) and gpui_quad_ps.dxbc ($((Get-Item $quadPsDxbc).Length) bytes)"
Write-Host ""

# 5. Transpile wave_field.wgsl to HLSL and compile with DXC
Write-Host "[5/6] Transpiling wave_field.wgsl -> HLSL and compiling with DXC..."
$waveWgsl = Join-Path $scriptDir "wave_field.wgsl"
$waveHlsl = Join-Path $scriptDir "wave_field.hlsl"
$waveDxil = Join-Path $scriptDir "wave_field.dxil"

& $wgpuCli transpile $waveWgsl --target hlsl -o $waveHlsl
Write-Host "Transpiled to $waveHlsl ($((Get-Item $waveHlsl).Length) bytes)"

& $dxc -T cs_6_6 -E main -O3 -Fo $waveDxil $waveHlsl
Write-Host "Compiled wave_field.dxil ($((Get-Item $waveDxil).Length) bytes)"
Write-Host ""

# 6. Enumerate DirectX 12 Hardware Adapters via WGPU CLI
Write-Host "[6/6] Enumerating DirectX 12 Adapters..."
& $wgpuCli adapters --backends dx12
Write-Host ""

Write-Host "All DirectX 12 verifications PASSED successfully!"
