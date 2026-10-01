// Blinn-Phong into a float target: values above 1.0 survive until tone mapping.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // The world position is a varying: V depends on the fragment.
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

struct Material {
    albedo: vec3<f32>,
    specular: vec3<f32>,
    shininess: f32,
};

// Direction plus intensity in one 16-byte slot.
struct Light {
    light_dir: vec3<f32>,
    intensity: f32,
};

// Matches the encase layout: each vec3 aligned to 16 bytes, 144 total.
struct Params {
    view_proj: mat4x4<f32>,
    eye: vec3<f32>,
    light: Light,
    material: Material,
};

@group(0) @binding(0) var<uniform> params: Params;

const AMBIENT = 0.1;

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
    let l = params.light.light_dir;
    // V varies across the plane even though N and L never move.
    let v = normalize(params.eye - input.world_position);
    let diffuse = max(dot(n, l), 0.0);
    var spec_factor = 0.0;
    // One-sided: no specular unless the light and the eye face the normal.
    if (diffuse > 0.0 && dot(n, v) > 0.0) {
        let l_plus_v = l + v;
        // V exactly against L leaves no half vector: highlight absent there.
        if (length(l_plus_v) > 0.0) {
            let h = normalize(l_plus_v);
            spec_factor = pow(max(dot(n, h), 0.0), params.material.shininess);
        }
    }
    let color = params.material.albedo * (AMBIENT + params.light.intensity * diffuse)
        + params.material.specular * (params.light.intensity * spec_factor);
    // Linear HDR out: values above 1.0 stored as they are.
    return vec4<f32>(color, 1.0);
}
