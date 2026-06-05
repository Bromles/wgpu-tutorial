struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let x = f32(i32(idx) - 1) / 2.0;
    let y = f32(i32(idx & 1u) * 2 - 1) / 2.0;
    let color = vec3<f32>(f32(idx == 0u), f32(idx == 1u), f32(idx == 2u));

    var output: VertexOutput;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
