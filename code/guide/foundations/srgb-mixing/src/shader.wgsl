// Diagnostic shader: white vertex colors multiplied by the per-draw tone.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct Tone {
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> tone: Tone;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.color = input.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color * tone.color.rgb, 1.0);
}
