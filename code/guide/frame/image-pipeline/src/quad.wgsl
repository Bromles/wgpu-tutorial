// UVs pick whole texels through textureLoad; each quadrant shows one color.
struct VertexInput {
    @location(0) position: vec4<f32>,
    // The UV lives in the second vertex buffer, shader_location 2.
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var TEX: texture_2d<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // UV in [0, 1] scaled to texel indices; the sRGB view decodes to linear.
    let extent = vec2<i32>(textureDimensions(TEX));
    let texel = vec2<i32>(input.uv * vec2<f32>(extent));
    return textureLoad(TEX, texel, 0);
}
