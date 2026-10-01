// View and projection multiply separately so the projection can switch; quads read texels via textureLoad.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct TexturedOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct FlatOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct Params {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var QUAD: texture_2d<f32>;

@vertex
fn vs_textured(input: VertexInput) -> TexturedOutput {
    var output: TexturedOutput;
    output.position = params.proj * (params.view * input.position);
    output.uv = input.uv;
    return output;
}

@vertex
fn vs_flat(input: VertexInput) -> FlatOutput {
    var output: FlatOutput;
    output.position = params.proj * (params.view * input.position);
    output.color = input.color.rgb;
    return output;
}

@fragment
fn fs_textured(input: TexturedOutput) -> @location(0) vec4<f32> {
    let extent = vec2<i32>(textureDimensions(QUAD));
    let texel = vec2<i32>(input.uv * vec2<f32>(extent));
    return textureLoad(QUAD, texel, 0);
}

@fragment
fn fs_flat(input: FlatOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
