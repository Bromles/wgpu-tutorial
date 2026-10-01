// position w = 1 (a point), normal w = 0 (a direction).
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

// Matches encase: vec3s align to 16 bytes; 48 bytes total.
struct SceneParams {
    light_dir: vec3<f32>,
    albedo: vec3<f32>,
    ambient: f32,
    intensity: f32,
    shadows_on: u32,
};

@group(0) @binding(0) var<uniform> model: mat4x4<f32>;
@group(0) @binding(1) var<uniform> camera_view_proj: mat4x4<f32>;
@group(0) @binding(2) var<uniform> light_view_proj: mat4x4<f32>;
@group(0) @binding(3) var<uniform> scene: SceneParams;
@group(0) @binding(4) var SHADOW: texture_depth_2d;

// Zero for now; written out to show where a bias would enter.
const BIAS: f32 = 0.0;

// Depth-only pass; no fragment stage.
@vertex
fn vs_depth(input: VertexInput) -> @builtin(position) vec4<f32> {
    return light_view_proj * model * input.position;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let world = model * input.position;
    var output: VertexOutput;
    output.position = camera_view_proj * world;
    output.world_pos = world.xyz;
    output.normal = input.normal.xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Restore the unit length interpolation shortened.
    let n = normalize(input.normal);
    // L points from the surface toward the light.
    let l = normalize(scene.light_dir);
    let d = max(dot(n, l), 0.0);
    // The shadow scales only the direct term; ambient survives.
    let visibility = shadow_visibility(input.world_pos);
    let color = scene.albedo * (scene.ambient + scene.intensity * d * visibility);
    // Linear output; the sRGB target encodes it on store.
    return vec4<f32>(color, 1.0);
}

fn shadow_visibility(world_pos: vec3<f32>) -> f32 {
    if (scene.shadows_on == 0u) {
        return 1.0;
    }
    let clip = light_view_proj * vec4<f32>(world_pos, 1.0);
    // w = 1 under ortho; the guard stays for a perspective light.
    if (clip.w <= 0.0) {
        return 1.0;
    }
    let ndc = clip.xy / clip.w;
    // Y scale is negative: the viewport wrote row 0 for NDC y = +1.
    let uv = ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    // Outside the light frustum nothing was ever recorded: lit.
    if (uv.x < 0.0 || uv.x >= 1.0 || uv.y < 0.0 || uv.y >= 1.0) {
        return 1.0;
    }
    let size = vec2<f32>(textureDimensions(SHADOW));
    let texel = vec2<i32>(uv * size);
    let stored = textureLoad(SHADOW, texel, 0);
    let reference = clip.z / clip.w;
    // z is already in [0, 1]: no remap, a plain comparison.
    return select(1.0, 0.0, reference > stored + BIAS);
}
