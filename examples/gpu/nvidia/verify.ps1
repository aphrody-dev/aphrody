# verify.ps1 - NVIDIA Shader & Pipeline Verification
$ErrorActionPreference = "Stop"

Write-Host "=== Validating sm89_matmul_warp.wgsl with gpui-wgpu ==="
$cli = if (Test-Path "$PSScriptRoot\..\..\..\target\debug\gpui-wgpu.exe") {
    "$PSScriptRoot\..\..\..\target\debug\gpui-wgpu.exe"
} else {
    "gpui-wgpu"
}

$shader = "$PSScriptRoot\sm89_matmul_warp.wgsl"
& $cli validate $shader
if ($LASTEXITCODE -ne 0) { throw "WGSL validation failed" }

Write-Host "=== Transpiling to HLSL for DirectX 12 ==="
& $cli transpile $shader --target hlsl
if ($LASTEXITCODE -ne 0) { throw "HLSL transpilation failed" }

Write-Host "=== Transpiling to SPIR-V for Vulkan ==="
& $cli transpile $shader --target spv
if ($LASTEXITCODE -ne 0) { throw "SPIR-V transpilation failed" }

if (Get-Command nvcc -ErrorAction SilentlyContinue) {
    Write-Host "=== NVIDIA CUDA Compiler (nvcc) Available ==="
    & nvcc --version
}

Write-Host "=== NVIDIA Shader Verification Succeeded! ==="
