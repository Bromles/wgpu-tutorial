// UV scaled by k and shifted by phase; the LOD clamp forces level 0 or 1.
struct VertexInput {
    @location(0) position: vec4<f32>,
    // UV binds at location 2; location 1 still carries the unused vertex color.
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Params {
    k: f32,
    phase: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var TEX: texture_2d<f32>;
@group(0) @binding(2) var SAMPLER: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv * params.k + vec2<f32>(params.phase, 0.0);
    return textureSample(TEX, SAMPLER, uv);
}
