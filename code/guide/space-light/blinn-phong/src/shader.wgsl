// Flat XY plane, one directional light; the highlight comes from the half vector.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // The world position is a varying: V depends on where the fragment is.
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

struct Material {
    albedo: vec3<f32>,
    specular: vec3<f32>,
    shininess: f32,
};

// Matches encase: vec3s 16-aligned (eye 64, light_dir 80), Material at 96; 128 total.
struct Params {
    view_proj: mat4x4<f32>,
    eye: vec3<f32>,
    light_dir: vec3<f32>,
    material: Material,
};

@group(0) @binding(0) var<uniform> params: Params;

const AMBIENT = 0.1;
const INTENSITY = 0.6;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // Model is the identity: plane positions are world positions.
    output.position = params.view_proj * input.position;
    output.world_position = input.position.xyz;
    output.world_normal = input.normal.xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.world_normal);
    let l = params.light_dir;
    // V points from the fragment toward the eye; it varies across the plane.
    let v = normalize(params.eye - input.world_position);
    let diffuse = max(dot(n, l), 0.0);
    var spec_factor = 0.0;
    // One-sided: no specular unless the light and the eye face the normal.
    if (diffuse > 0.0 && dot(n, v) > 0.0) {
        let l_plus_v = l + v;
        // V exactly against L leaves no half vector; no highlight.
        if (length(l_plus_v) > 0.0) {
            let h = normalize(l_plus_v);
            spec_factor = pow(max(dot(n, h), 0.0), params.material.shininess);
        }
    }
    let color = params.material.albedo * (AMBIENT + INTENSITY * diffuse)
        + params.material.specular * (INTENSITY * spec_factor);
    return vec4<f32>(color, 1.0);
}
