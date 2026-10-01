// Per-draw tint selection via immediates: a value block in command state.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct Params {
    gain: f32,
};

struct Selection {
    index: u32,
};

@group(0) @binding(0) var<uniform> params: Params;

// Two known tints; the draw picks one by an immediate index.
@group(1) @binding(0) var<storage, read> TINTS: array<vec4<f32>, 2>;

// Written by set_immediates before each draw, read like any module-scope var.
var<immediate> selection: Selection;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.color = input.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tint = TINTS[selection.index];
    return vec4<f32>(input.color * tint.rgb * params.gain, 1.0);
}
