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

struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct InstanceInput {
    @location(3) model_col0: vec4<f32>,
    @location(4) model_col1: vec4<f32>,
    @location(5) model_col2: vec4<f32>,
    @location(6) model_col3: vec4<f32>,
    @location(7) normal_col0: vec4<f32>,
    @location(8) normal_col1: vec4<f32>,
    @location(9) normal_col2: vec4<f32>,
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
        instance.normal_col0.xyz,
        instance.normal_col1.xyz,
        instance.normal_col2.xyz,
    );
    var output: VertexOutput;
    let world_pos = model * vec4<f32>(input.position, 1.0);
    output.position = uniforms.view_proj * world_pos;
    output.normal = normal_matrix * input.normal;
    output.world_pos = world_pos.xyz;
    output.uv = input.uv;
    return output;
}

struct LightUniforms {
    light_dir: vec3<f32>,
    ambient: f32,
    light_color: vec3<f32>,
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
    let light_dir = normalize(-light.light_dir);

    let diffuse = max(dot(normal, light_dir), 0.0);
    let intensity = light.ambient + diffuse * (1.0 - light.ambient);

    let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);
    let color = tex_color.rgb * light.light_color * intensity;

    return vec4<f32>(color, 1.0);
}
