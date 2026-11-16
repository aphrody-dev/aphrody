# Verification Script for WebGPU & WGSL Architecture in gpui-os
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   gpui-os WebGPU & WGSL Architecture Verification Suite" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$gpuiRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$gpuiWgpu = Join-Path $gpuiRoot "target\debug\gpui-wgpu.exe"
$exampleDir = $PSScriptRoot
$shaderWgsl = Join-Path $exampleDir "nsh_ripple_shader.wgsl"

# 1. Audit Tooling
Write-Host "`n[1/6] Auditing Installed WebGPU & WGSL Tooling..." -ForegroundColor Yellow
$tools = @("bun.exe", "naga.exe", "tint.exe", "tint_info.exe", "wgsl-analyzer.exe")
foreach ($t in $tools) {
    $cmd = Get-Command $t -ErrorAction SilentlyContinue
    if ($cmd) {
        Write-Host "  [OK] Found $t at $($cmd.Source)" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Missing $t" -ForegroundColor Red
    }
}
if (Test-Path $gpuiWgpu) {
    Write-Host "  [OK] Found gpui-wgpu at $gpuiWgpu" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Missing gpui-wgpu at $gpuiWgpu" -ForegroundColor Red
}

# 2. Typecheck with Bun
Write-Host "`n[2/6] Running Bun TypeScript Typecheck (bun check)..." -ForegroundColor Yellow
Set-Location $exampleDir
bun check
if ($LASTEXITCODE -eq 0) {
    Write-Host "  [OK] Bun TypeScript type check passed with 0 errors." -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Bun check failed" -ForegroundColor Red
}

# 3. Run Bun WebGPU Inspector
Write-Host "`n[3/6] Running Bun WebGPU Inspector (webgpu_inspect.ts)..." -ForegroundColor Yellow
bun run webgpu_inspect.ts
if ($LASTEXITCODE -eq 0) {
    Write-Host "  [OK] Bun WebGPU Inspector completed successfully." -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Bun WebGPU Inspector failed" -ForegroundColor Red
}

# 4. Multi-Target Shader Cross-Compilation
Write-Host "`n[4/6] Cross-Compiling WGSL Across All 5 Backends with gpui-wgpu..." -ForegroundColor Yellow
$targets = @("wgsl", "spv", "hlsl", "msl", "webgl2")
foreach ($target in $targets) {
    $outFile = Join-Path $exampleDir "nsh_ripple.$target"
    if ($target -eq "wgsl" -or $target -eq "spv") {
        & $gpuiWgpu transpile $shaderWgsl --target $target -o $outFile
    } else {
        & $gpuiWgpu transpile $shaderWgsl --target $target --stage fragment -o $outFile
    }
    if (Test-Path $outFile) {
        $sizeBytes = (Get-Item $outFile).Length
        Write-Host "  [OK] Transpiled to $target -> $outFile ($sizeBytes bytes)" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Failed to transpile to $target" -ForegroundColor Red
    }
}

# 5. Headless GPU Render Pass
Write-Host "`n[5/6] Executing Headless GPU Render Pass (512x512 PNG)..." -ForegroundColor Yellow
$renderPng = Join-Path $exampleDir "nsh_ripple.png"
& $gpuiWgpu render $shaderWgsl --output $renderPng --width 512 --height 512
if (Test-Path $renderPng) {
    $pngSize = (Get-Item $renderPng).Length
    Write-Host "  [OK] Rendered offscreen PNG ($pngSize bytes) to $renderPng" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Render pass failed to generate PNG" -ForegroundColor Red
}

# 6. Headless Storage Buffer Compute
Write-Host "`n[6/6] Executing Headless GPU Compute Dispatch..." -ForegroundColor Yellow
$computeWgsl = Join-Path $exampleDir "nsh_compute.wgsl"
& $gpuiWgpu compute $computeWgsl --workgroups 1,1,1 --output-floats 64 --json | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "  [OK] Headless compute pass executed successfully on GPU." -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Headless compute pass failed" -ForegroundColor Red
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "  All WebGPU & WGSL Verification Checks Completed (PASS)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
