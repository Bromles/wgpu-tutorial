// One face from two tangents: positions through the model, normals through its inverse-transpose.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
};

// Matches the encase layout: two mat4x4 columns-first, 128 bytes total.
struct ModelParams {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> view_proj: mat4x4<f32>;
@group(0) @binding(1) var<uniform> model_params: ModelParams;

// Chapter 23 light rotated toward X so the face tilt stays visible; all values linear.
const LIGHT_DIR = vec3<f32>(0.70710678, 0.0, 0.70710678);
const ALBEDO = 0.5;
const AMBIENT = 0.1;
const INTENSITY = 0.6;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // Positions: view*projection after the model matrix.
    output.position = view_proj * model_params.model * input.position;
    // Normals: a direction through the inverse-transpose, which keeps it perpendicular.
    output.world_normal = (model_params.normal_matrix * vec4<f32>(input.normal.xyz, 0.0)).xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // The normal matrix scales the length away: restore unit length before the dot.
    let n = normalize(input.world_normal);
    let d = max(dot(n, LIGHT_DIR), 0.0);
    let color = vec3<f32>(ALBEDO * (AMBIENT + INTENSITY * d));
    return vec4<f32>(color, 1.0);
}
