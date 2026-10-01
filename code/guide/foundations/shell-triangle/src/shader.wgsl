// Same positions and color as chapter 04: relocation must not change the frame.
const POSITIONS: array<vec4<f32>, 3> = array(
    vec4<f32>(-0.75, -0.75, 0.5, 1.0),
    vec4<f32>(0.75, -0.75, 0.5, 1.0),
    vec4<f32>(0.0, 0.75, 0.5, 1.0),
);

const COLOR: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return POSITIONS[vertex_index];
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return COLOR;
}
