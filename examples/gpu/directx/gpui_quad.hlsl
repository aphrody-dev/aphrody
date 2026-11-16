// DirectX 12 GPUI Quad Rendering Vertex & Pixel Shaders
// Profiles: vs_6_6 (vs_main), ps_6_6 (ps_main)
// Also compatible with legacy SM 5.0 (fxc.exe -T vs_5_0 / ps_5_0)

cbuffer QuadConstants : register(b0) {
    float4 u_color;         // offset 0  (16 bytes)
    float4 u_border_color;  // offset 16 (16 bytes)
    float2 u_viewport_size; // offset 32 (8 bytes)
    float2 u_quad_origin;   // offset 40 (8 bytes)
    float2 u_quad_size;     // offset 48 (8 bytes)
    float  u_border_width;  // offset 56 (4 bytes)
    float  u_corner_radius; // offset 60 (4 bytes)
    float4 _pad[12];        // Pad struct to 256 bytes for D3D12 constant buffer placement
};

struct VSInput {
    float2 unit_pos : POSITION;
    float2 uv       : TEXCOORD0;
};

struct PSInput {
    float4 pos      : SV_POSITION;
    float2 local_pos: TEXCOORD0;
    float2 uv       : TEXCOORD1;
};

PSInput vs_main(VSInput input) {
    PSInput output;

    // Calculate pixel coordinates from unit quad [0..1]
    float2 pixel_pos = u_quad_origin + (input.unit_pos * u_quad_size);

    // Transform pixel coordinates to Normalized Device Coordinates [-1..1]
    float2 ndc_pos;
    ndc_pos.x = (pixel_pos.x / u_viewport_size.x) * 2.0f - 1.0f;
    ndc_pos.y = 1.0f - (pixel_pos.y / u_viewport_size.y) * 2.0f; // Flip Y for D3D NDC

    output.pos = float4(ndc_pos, 0.0f, 1.0f);
    output.local_pos = input.unit_pos * u_quad_size;
    output.uv = input.uv;
    return output;
}

Texture2D g_Texture : register(t0);
SamplerState g_Sampler : register(s0);

float4 ps_main(PSInput input) : SV_TARGET {
    // Distance field for rounded corners
    float2 half_size = u_quad_size * 0.5f;
    float2 center_dist = abs(input.local_pos - half_size) - (half_size - u_corner_radius);
    float outside_dist = length(max(center_dist, 0.0f));
    float dist = outside_dist - u_corner_radius;

    // Smoothstep anti-aliasing on corner boundary
    float alpha = 1.0f - smoothstep(-0.5f, 0.5f, dist);

    // Border vs interior mask
    float interior_mask = step(dist, -u_border_width);
    float4 final_color = lerp(u_border_color, u_color, interior_mask);

    final_color.a *= alpha;
    return final_color;
}
