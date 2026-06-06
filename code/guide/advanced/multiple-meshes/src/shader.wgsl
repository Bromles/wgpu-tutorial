struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct CameraUniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

struct MeshUniforms {
    model: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
    light_dir: vec3<f32>,
    ambient: f32,
    base_color: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> mesh: MeshUniforms;

@group(1) @binding(1)
var diffuse_tex: texture_2d<f32>;

@group(1) @binding(2)
var diffuse_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = mesh.model * vec4<f32>(input.position, 1.0);
    output.position = camera.view_proj * world_pos;
    output.world_pos = world_pos.xyz;
    output.normal = mesh.normal_matrix * input.normal;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(-mesh.light_dir);
    let diffuse = max(dot(normal, light_dir), 0.0);
    let intensity = mesh.ambient + diffuse * (1.0 - mesh.ambient);

    let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);
    let color = mesh.base_color.rgb * tex_color.rgb;
    return vec4<f32>(color * vec3<f32>(1.0, 0.95, 0.85) * intensity, 1.0);
}
