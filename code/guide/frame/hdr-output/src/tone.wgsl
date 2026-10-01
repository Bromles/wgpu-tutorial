// Tone mapper: exposure, then Reinhard; the sRGB surface encodes once on store.
struct ToneParams {
    exposure: f32,
    mode: u32,
};

@group(0) @binding(0) var<uniform> tone: ToneParams;
@group(0) @binding(1) var HDR: texture_2d<f32>;

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
    if (tone.mode == 1u) {
        // Clipping view: white wherever the frame exceeded 1.0.
        let peak = max(hdr.r, max(hdr.g, hdr.b));
        let clipped = select(0.0, 1.0, peak > 1.0);
        return vec4<f32>(vec3<f32>(clipped), 1.0);
    }
    // Reinhard compresses positives into [0, 1): 0 -> 0, 1 -> 0.5.
    let scaled = hdr.rgb * tone.exposure;
    let tone_mapped = scaled / (vec3<f32>(1.0) + scaled);
    // Linear SDR out: the sRGB surface encodes it once, on store.
    return vec4<f32>(tone_mapped, 1.0);
}
