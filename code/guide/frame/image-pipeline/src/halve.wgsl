// Fragment fork: a fullscreen triangle halving every texel it rasterizes.

@group(0) @binding(0) var INPUT: texture_2d<f32>;

@vertex
fn vs_full(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[vertex_index], 0.5, 1.0);
}

@fragment
fn fs_halve(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(position.xy);
    let color = textureLoad(INPUT, texel, 0);
    // Linear light is halved; alpha passes through untouched.
    return vec4<f32>(color.rgb * 0.5, color.a);
}
