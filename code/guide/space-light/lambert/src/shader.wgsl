// Attributes: position w = 1 (a point), normal w = 0 (a direction); model is identity.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

// Matches the encase layout: vec3 fields align to 16 bytes.
struct LightParams {
    light_dir: vec3<f32>,
    albedo: vec3<f32>,
    ambient: f32,
    intensity: f32,
    show_normals: u32,
};

@group(0) @binding(0) var<uniform> view_proj: mat4x4<f32>;
@group(0) @binding(1) var<uniform> light: LightParams;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = view_proj * input.position;
    output.normal = input.normal.xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Interpolation shortens the varying normal; restore the unit length.
    let n = normalize(input.normal);
    if (light.show_normals == 1u) {
        // Diagnostic view: map the unit normal from [-1, 1] into [0, 1].
        return vec4<f32>(n * 0.5 + vec3<f32>(0.5), 1.0);
    }
    // L points toward the light; N and L both live in world space.
    let l = normalize(light.light_dir);
    let d = max(dot(n, l), 0.0);
    let color = light.albedo * (light.ambient + light.intensity * d);
    // Linear output; the sRGB target encodes it on store.
    return vec4<f32>(color, 1.0);
}
