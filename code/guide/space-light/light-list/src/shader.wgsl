// Fixed light pool in a read-only storage buffer; the fragment shader sums the first `count`.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

// Matches encase packing: pos+power, axis+spot flag; 32 bytes per light.
struct Light {
    pos: vec4<f32>,
    spot: vec4<f32>,
};

// Matches encase: matrix first, then the count scalar padded to the 16-byte tail; 80 bytes.
struct Params {
    view_proj: mat4x4<f32>,
    count: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> lights: array<Light>;

const AMBIENT = 0.0;
const ALBEDO = vec3<f32>(0.5, 0.5, 0.5);
const R_MIN = 0.1;
// Cone angles of chapter 26a.
const SPOT_COS_INNER = 0.9659258;
const SPOT_COS_OUTER = 0.8660254;

// The per-light formula of chapter 26a, factored into a function.
fn contribution(light: Light, p: vec3<f32>, n: vec3<f32>) -> f32 {
    let to_light = light.pos.xyz - p;
    let r = length(to_light);
    let l = to_light / r;
    let attenuation = 1.0 / max(r * r, R_MIN * R_MIN);
    var c = light.pos.w * attenuation * max(dot(n, l), 0.0);
    if (light.spot.w == 1.0) {
        let d = dot(-l, light.spot.xyz);
        let t = clamp(
            (d - SPOT_COS_OUTER) / (SPOT_COS_INNER - SPOT_COS_OUTER),
            0.0,
            1.0,
        );
        c = c * t;
    }
    return c;
}

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
    var sum = 0.0;
    // The bound comes from the uniform: only the first `count` records are read.
    for (var i = 0u; i < u32(params.count); i++) {
        sum += contribution(lights[i], input.world_position, n);
    }
    // Ambient belongs to the scene: added once, after the loop.
    let color = ALBEDO * (AMBIENT + sum);
    return vec4<f32>(color, 1.0);
}
