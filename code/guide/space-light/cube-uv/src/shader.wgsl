// Each face corner carries its UV; fragments read one texel per pixel via textureLoad.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var<uniform> view_proj: mat4x4<f32>;
@group(0) @binding(1) var FACE: texture_2d<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = view_proj * input.position;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // UV scaled to texel indices; textureLoad reads whole texels.
    let extent = vec2<i32>(textureDimensions(FACE));
    let texel = vec2<i32>(input.uv * vec2<f32>(extent));
    return textureLoad(FACE, texel, 0);
}
