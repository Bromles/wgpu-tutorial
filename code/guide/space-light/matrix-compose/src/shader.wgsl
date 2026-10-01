// The same asymmetric triangle, now transformed by one GPU-side mat4x4.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

// Column-major mat4x4; multiplication acts right-to-left on the vector.
@group(0) @binding(0) var<uniform> transform: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = transform * vec4<f32>(input.position.xy, 0.5, 1.0);
    output.color = input.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
