// Flat-color panel; straight alpha, depth writes off.
struct VertexInput {
    @location(0) position: vec4<f32>,
};

// A mat4 binding sees exactly the first 64 bytes of the camera buffer.
@group(0) @binding(0) var<uniform> view_proj: mat4x4<f32>;

const PANEL_COLOR = vec4<f32>(1.0, 1.0, 1.0, 0.5);

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return view_proj * input.position;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return PANEL_COLOR;
}
