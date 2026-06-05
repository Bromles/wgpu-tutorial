struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0)
var scene_tex: texture_2d<f32>;

@group(0) @binding(1)
var blur_tex: texture_2d<f32>;

@group(0) @binding(2)
var tex_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[idx], 0.0, 1.0);
    output.uv = uvs[idx];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (input.uv.x < 0.5) {
        return textureSample(scene_tex, tex_sampler, input.uv);
    } else {
        return textureSample(blur_tex, tex_sampler, input.uv);
    }
}
