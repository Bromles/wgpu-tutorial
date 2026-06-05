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

struct CameraUniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

struct LightUniforms {
    light_dir: vec3<f32>,
    ambient: f32,
    intensity: f32,
}

@group(1) @binding(0)
var<uniform> light: LightUniforms;

@group(1) @binding(1)
var diffuse_tex: texture_2d<f32>;

@group(1) @binding(2)
var diffuse_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.normal = input.normal;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(-light.light_dir);
    let diffuse = max(dot(normal, light_dir), 0.0);

    let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);
    let intensity = light.ambient + diffuse * light.intensity;
    return vec4<f32>(tex_color.rgb * vec3<f32>(1.0, 0.95, 0.85) * intensity, 1.0);
}
