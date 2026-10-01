// One source at a position over the chapter 25 plane: distance attenuates, an optional cone masks.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // The world position is a varying: L and r depend on the fragment.
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

// Matches encase: matrix first, then each vec3 16-aligned with the next scalar in the tail; 112 bytes.
struct Params {
    view_proj: mat4x4<f32>,
    light_pos: vec3<f32>,
    power: f32,
    spot_dir: vec3<f32>,
    spot_cos_inner: f32,
    spot_cos_outer: f32,
    spot_on: u32,
};

@group(0) @binding(0) var<uniform> params: Params;

const AMBIENT = 0.0;
const ALBEDO = vec3<f32>(0.5, 0.5, 0.5);
const R_MIN = 0.1;

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
    // L points from the fragment toward the source; the clamp keeps the denominator above R_MIN^2.
    let to_light = params.light_pos - input.world_position;
    let r = length(to_light);
    let l = to_light / r;
    let attenuation = 1.0 / max(r * r, R_MIN * R_MIN);
    var contribution = params.power * attenuation * max(dot(n, l), 0.0);
    if (params.spot_on == 1u) {
        // d is the cosine between the cone axis and the direction to the fragment (-L).
        let d = dot(-l, params.spot_dir);
        // Linear fade in the cosines: 0 at the outer cone, 1 at the inner.
        let t = clamp(
            (d - params.spot_cos_outer) / (params.spot_cos_inner - params.spot_cos_outer),
            0.0,
            1.0,
        );
        contribution = contribution * t;
    }
    let color = ALBEDO * (AMBIENT + contribution);
    return vec4<f32>(color, 1.0);
}
