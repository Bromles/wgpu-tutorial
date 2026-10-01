// Reinhard tone mapping with the exposure fixed at 1.0.
@group(0) @binding(0) var HDR: texture_2d<f32>;

const EXPOSURE = 1.0;

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
    let texel = vec2<i32>(position.xy);
    let hdr = textureLoad(HDR, texel, 0);
    let scaled = hdr.rgb * EXPOSURE;
    let tone_mapped = scaled / (vec3<f32>(1.0) + scaled);
    // Linear SDR out: the sRGB surface encodes it once, on store.
    return vec4<f32>(tone_mapped, 1.0);
}
