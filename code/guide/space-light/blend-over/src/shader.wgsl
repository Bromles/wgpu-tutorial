// Diagnostic quads for the over operator; the pipeline picks the alpha representation.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct Params {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = params.view_proj * input.position;
    output.color = input.color;
    return output;
}

// Straight alpha: RGB untouched; SrcAlpha / OneMinusSrcAlpha weigh it at the merger.
@fragment
fn fs_straight(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

// Premultiplied alpha: shader weighs RGB by alpha; pipeline uses One / OneMinusSrcAlpha.
@fragment
fn fs_premultiplied(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color.rgb * input.color.a, input.color.a);
}
