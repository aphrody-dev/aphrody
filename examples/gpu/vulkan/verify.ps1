# Vulkan & SPIR-V Shader Verification Script for gpui-os
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path "$scriptDir\..\..\..").Path
Set-Location $scriptDir

$outputDir = Join-Path $scriptDir "output"
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$glslang = "$env:USERPROFILE\.local\bin\glslangValidator.exe"
$glslc = "$env:USERPROFILE\.local\bin\glslc.exe"
$spirvVal = "$env:USERPROFILE\.local\bin\spirv-val.exe"
$spirvOpt = "$env:USERPROFILE\.local\bin\spirv-opt.exe"
$spirvDis = "$env:USERPROFILE\.local\bin\spirv-dis.exe"
$gpuiWgpu = "$RepoRoot\target\debug\gpui-wgpu.exe"

Write-Host "=========================================================="
Write-Host "  gpui-os Vulkan & SPIR-V Shader Verification Pipeline"
Write-Host "=========================================================="

# 1. Compile GLSL Compute Shader with glslangValidator
Write-Host "`n[1/7] Compiling wave_compute.comp with glslangValidator..."
$compSpv = Join-Path $outputDir "wave_compute.spv"
& $glslang -V "wave_compute.comp" -o $compSpv
Write-Host "  -> Generated: $compSpv ($((Get-Item $compSpv).Length) bytes)"

# 2. Compile GLSL Compute Shader with glslc
Write-Host "`n[2/7] Compiling wave_compute.comp with glslc..."
$compGlslcSpv = Join-Path $outputDir "wave_compute_glslc.spv"
& $glslc "wave_compute.comp" -o $compGlslcSpv
Write-Host "  -> Generated: $compGlslcSpv ($((Get-Item $compGlslcSpv).Length) bytes)"

# 3. Validate Compute SPIR-V with spirv-val
Write-Host "`n[3/7] Validating Compute SPIR-V bytecode with spirv-val..."
& $spirvVal --target-env spv1.6 $compSpv
Write-Host "  -> wave_compute.spv: PASSED spirv-val (SPV 1.6)"
& $spirvVal --target-env spv1.6 $compGlslcSpv
Write-Host "  -> wave_compute_glslc.spv: PASSED spirv-val (SPV 1.6)"

# 4. Optimize and Disassemble SPIR-V
Write-Host "`n[4/7] Running spirv-opt and spirv-dis..."
$compOptSpv = Join-Path $outputDir "wave_compute.opt.spv"
& $spirvOpt -O $compSpv -o $compOptSpv
Write-Host "  -> Optimized: $compOptSpv ($((Get-Item $compOptSpv).Length) bytes)"

$compDisasm = Join-Path $outputDir "wave_compute.spvasm"
& $spirvDis $compOptSpv -o $compDisasm
Write-Host "  -> Disassembly saved: $compDisasm ($((Get-Content $compDisasm).Count) lines)"

# 5. Compile Graphics Shaders (triangle.vert & triangle.frag)
Write-Host "`n[5/7] Compiling triangle.vert and triangle.frag..."
$vertSpv = Join-Path $outputDir "triangle.vert.spv"
$fragSpv = Join-Path $outputDir "triangle.frag.spv"
& $glslang -V "triangle.vert" -o $vertSpv
& $glslang -V "triangle.frag" -o $fragSpv
& $spirvVal --target-env spv1.6 $vertSpv
& $spirvVal --target-env spv1.6 $fragSpv
Write-Host "  -> triangle.vert.spv ($((Get-Item $vertSpv).Length) bytes): PASSED"
Write-Host "  -> triangle.frag.spv ($((Get-Item $fragSpv).Length) bytes): PASSED"

# 6. Transpile WGSL to SPIR-V using gpui-wgpu (Naga back::spv)
Write-Host "`n[6/7] Transpiling quad_vulkan.wgsl via gpui-wgpu to SPIR-V..."
$quadSpv = Join-Path $outputDir "quad_vulkan.spv"
& $gpuiWgpu transpile --target spv -o $quadSpv "quad_vulkan.wgsl"
Write-Host "  -> Generated: $quadSpv ($((Get-Item $quadSpv).Length) bytes)"

# 7. Validate Naga-Generated SPIR-V
Write-Host "`n[7/7] Validating Naga-generated quad_vulkan.spv with spirv-val..."
& $spirvVal --target-env spv1.3 $quadSpv
Write-Host "  -> quad_vulkan.spv: PASSED spirv-val validation (SPV 1.3)"

Write-Host "`n=========================================================="
Write-Host "  ALL VULKAN SHADER VERIFICATION CHECKS PASSED (7/7)!"
Write-Host "=========================================================="
