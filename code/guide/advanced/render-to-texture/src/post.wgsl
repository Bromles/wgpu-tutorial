// Post-process shader — renders fullscreen quad with offscreen texture
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

struct PostUniforms {
    mode: u32,
};

@group(0) @binding(0)
var scene_tex: texture_2d<f32>;

@group(0) @binding(1)
var scene_sampler: sampler;

@group(0) @binding(2)
var<uniform> post: PostUniforms;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(scene_tex, scene_sampler, input.uv);
    if (post.mode == 1u) {
        let gray = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        return vec4<f32>(vec3<f32>(gray), 1.0);
    }
    if (post.mode == 2u) {
        return vec4<f32>(1.0 - color.rgb, 1.0);
    }
    return color;
}
