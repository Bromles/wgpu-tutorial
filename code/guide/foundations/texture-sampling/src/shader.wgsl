// The sampler maps UV to texel values; sRGB texels decode to linear before mixing.
struct VertexInput {
    @location(0) position: vec4<f32>,
    // UV lives in the second vertex buffer; the color attribute stays unused here.
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var TEX: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(TEX, SAMPLER, input.uv);
}
