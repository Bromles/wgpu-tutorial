// Shadow depth pass — renders depth from light's perspective
struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct InstanceInput {
    @location(1) model_col0: vec4<f32>,
    @location(2) model_col1: vec4<f32>,
    @location(3) model_col2: vec4<f32>,
    @location(4) model_col3: vec4<f32>,
};

struct LightUniforms {
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> light: LightUniforms;

@vertex
fn vs_main(input: VertexInput, instance: InstanceInput) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(instance.model_col0, instance.model_col1, instance.model_col2, instance.model_col3);
    return light.light_view_proj * model * vec4<f32>(input.position, 1.0);
}
