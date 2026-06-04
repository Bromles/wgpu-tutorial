struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
    light_dir: vec3<f32>,
    ambient: f32,
    base_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var diffuse_tex: texture_2d<f32>;

@group(0) @binding(2)
var diffuse_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = uniforms.model * vec4<f32>(input.position, 1.0);
    output.position = uniforms.view_proj * world_pos;
    output.normal = uniforms.normal_matrix * input.normal;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(-uniforms.light_dir);
    let diffuse = max(dot(normal, light_dir), 0.0);
    let intensity = uniforms.ambient + diffuse * (1.0 - uniforms.ambient);

    let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);
    let color = uniforms.base_color.rgb * tex_color.rgb;
    return vec4<f32>(color * vec3<f32>(1.0, 0.95, 0.85) * intensity, 1.0);
}
