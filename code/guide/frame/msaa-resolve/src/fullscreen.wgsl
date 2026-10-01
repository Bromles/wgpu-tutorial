// Plain single-sample copy; multisampling lives in the scene pass only.
@group(0) @binding(0) var FRAME: texture_2d<f32>;

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
fn fs_full(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // Pixel centers coincide with texel centers: truncation addresses (x, y).
    let texel = vec2<i32>(position.xy);
    return textureLoad(FRAME, texel, 0);
}
