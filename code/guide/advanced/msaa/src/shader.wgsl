struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    ambient: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct InstanceInput {
    @location(2) model_col0: vec4<f32>,
    @location(3) model_col1: vec4<f32>,
    @location(4) model_col2: vec4<f32>,
    @location(5) model_col3: vec4<f32>,
    @location(6) normal_col0: vec4<f32>,
    @location(7) normal_col1: vec4<f32>,
    @location(8) normal_col2: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_col0, instance.model_col1, instance.model_col2, instance.model_col3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_col0.xyz, instance.normal_col1.xyz, instance.normal_col2.xyz,
    );
    var output: VertexOutput;
    output.position = uniforms.view_proj * model * vec4<f32>(input.position, 1.0);
    output.normal = normal_matrix * input.normal;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(-uniforms.light_dir);
    let diffuse = max(dot(normal, light_dir), 0.0);
    let intensity = uniforms.ambient + diffuse * (1.0 - uniforms.ambient);
    let base_color = vec3<f32>(0.85, 0.85, 0.85);
    return vec4<f32>(base_color * vec3<f32>(1.0, 0.95, 0.85) * intensity, 1.0);
}
