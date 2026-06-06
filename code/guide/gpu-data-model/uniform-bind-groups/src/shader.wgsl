struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

struct Uniforms {
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let t = uniforms.time;
    let r = input.color.r * (0.5 + 0.5 * sin(t));
    let g = input.color.g * (0.5 + 0.5 * sin(t + 2.094));
    let b = input.color.b * (0.5 + 0.5 * sin(t + 4.189));
    return vec4<f32>(r, g, b, 1.0);
}
