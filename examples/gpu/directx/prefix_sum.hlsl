// DirectX 12 Wave Intrinsics Prefix Sum Compute Shader
// Target Profile: cs_6_6
// Compiled with: dxc.exe -T cs_6_6 -E main -O3 -Fo prefix_sum.dxil prefix_sum.hlsl

struct PrefixSumParams {
    uint element_count;
    uint block_offset;
    uint2 _pad;
};

ConstantBuffer<PrefixSumParams> g_Params : register(b0);
RWStructuredBuffer<uint> g_Data : register(u0);

[numthreads(32, 1, 1)]
void main(uint3 dispatchThreadId : SV_DispatchThreadID, uint groupIndex : SV_GroupIndex) {
    uint globalIndex = g_Params.block_offset + dispatchThreadId.x;

    uint value = 0;
    if (globalIndex < g_Params.element_count) {
        value = g_Data[globalIndex];
    }

    // Shader Model 6.0+ Wave Intrinsics
    // WavePrefixSum computes exclusive scan across active lanes in the wave (warp)
    uint prefixSum = WavePrefixSum(value);
    uint totalWaveSum = WaveActiveSum(value);

    if (globalIndex < g_Params.element_count) {
        // Store exclusive prefix sum into output buffer
        g_Data[globalIndex] = prefixSum;
    }
}
