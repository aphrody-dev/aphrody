# verify.ps1 - Intel Shader & ISPC Pipeline Verification
$ErrorActionPreference = "Stop"

Write-Host "=== Validating xe_tiled_conv.wgsl with gpui-wgpu ==="
$cli = if (Test-Path "$PSScriptRoot\..\..\..\target\debug\gpui-wgpu.exe") {
    "$PSScriptRoot\..\..\..\target\debug\gpui-wgpu.exe"
} else {
    "gpui-wgpu"
}

$shader = "$PSScriptRoot\xe_tiled_conv.wgsl"
& $cli validate $shader
if ($LASTEXITCODE -ne 0) { throw "WGSL validation failed" }

Write-Host "=== Transpiling to HLSL for DirectX 12 ==="
& $cli transpile $shader --target hlsl
if ($LASTEXITCODE -ne 0) { throw "HLSL transpilation failed" }

Write-Host "=== Transpiling to SPIR-V for Vulkan ==="
& $cli transpile $shader --target spv
if ($LASTEXITCODE -ne 0) { throw "SPIR-V transpilation failed" }

$ispcCli = if (Get-Command ispc -ErrorAction SilentlyContinue) {
    "ispc"
} elseif (Test-Path "$HOME\.local\bin\ispc.exe") {
    "$HOME\.local\bin\ispc.exe"
} else {
    $null
}

if ($ispcCli) {
    Write-Host "=== Compiling vector_fma.ispc with Intel ISPC (AVX2 Target) ==="
    & $ispcCli --version
    $ispcSrc = "$PSScriptRoot\vector_fma.ispc"
    $objOut = "$PSScriptRoot\vector_fma.obj"
    $hOut = "$PSScriptRoot\vector_fma.h"
    & $ispcCli -O3 --target=avx2-i32x8 $ispcSrc -o $objOut -h $hOut
    if ($LASTEXITCODE -ne 0) { throw "ISPC compilation failed" }
    Write-Host "Successfully generated $objOut and $hOut"
}

Write-Host "=== Intel Shader & SIMD Verification Succeeded! ==="
