struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct CameraUniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

struct InstanceInput {
    @location(3) model_col0: vec4<f32>,
    @location(4) model_col1: vec4<f32>,
    @location(5) model_col2: vec4<f32>,
    @location(6) model_col3: vec4<f32>,
    @location(7) normal_col0: vec3<f32>,
    @location(8) normal_col1: vec3<f32>,
    @location(9) normal_col2: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_col0,
        instance.model_col1,
        instance.model_col2,
        instance.model_col3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_col0,
        instance.normal_col1,
        instance.normal_col2,
    );
    var output: VertexOutput;
    let world_pos = model * vec4<f32>(input.position, 1.0);
    output.position = camera.view_proj * world_pos;
    output.normal = normal_matrix * input.normal;
    output.world_pos = world_pos.xyz;
    output.uv = input.uv;
    return output;
}

struct Light {
    direction: vec3<f32>,
    color: vec3<f32>,
}

struct LightUniforms {
    lights: array<Light, 3>,
    ambient: f32,
}

@group(1) @binding(0)
var<uniform> light: LightUniforms;

@group(1) @binding(1)
var diffuse_tex: texture_2d<f32>;

@group(1) @binding(2)
var diffuse_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);

    var total = vec3<f32>(0.0);
    for (var i = 0u; i < 3u; i++) {
        let light_dir = normalize(-light.lights[i].direction);
        let diffuse = max(dot(normal, light_dir), 0.0);
        total += light.lights[i].color * diffuse * tex_color.rgb;
    }

    let ambient = light.ambient * tex_color.rgb;
    return vec4<f32>(ambient + total, 1.0);
}
