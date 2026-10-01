// Depth and culling are rasterizer state, so the shader stays the plain colored-triangle one.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct Params {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = params.proj * (params.view * input.position);
    output.color = input.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
