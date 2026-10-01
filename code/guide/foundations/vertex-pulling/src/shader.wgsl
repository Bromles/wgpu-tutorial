// Both entry points share the record layout: two vec4, stride 32, no padding.

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexData {
    position: vec4<f32>,
    color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct Params {
    tint: vec4<f32>,
    gain: f32,
};

@group(0) @binding(0) var<uniform> params: Params;

// The same bytes as the vertex buffer, exposed as a read-only storage array.
@group(1) @binding(0) var<storage, read> DATA: array<VertexData>;

@vertex
fn vs_fetch(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.color = input.color.rgb;
    return output;
}

@vertex
fn vs_pull(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // In indexed draws vertex_index is the index value itself.
    let data = DATA[vertex_index];
    var output: VertexOutput;
    output.position = data.position;
    output.color = data.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color * params.tint.rgb * params.gain, 1.0);
}
