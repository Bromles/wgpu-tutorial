// Same positions as before; the GPU interpolates per-vertex linear RGB across the triangle.
const POSITIONS: array<vec4<f32>, 3> = array(
    vec4<f32>(-0.75, -0.75, 0.5, 1.0),
    vec4<f32>(0.75, -0.75, 0.5, 1.0),
    vec4<f32>(0.0, 0.75, 0.5, 1.0),
);

const COLORS: array<vec3<f32>, 3> = array(
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, 0.0, 1.0),
);

// Chapter 04 fallback color for the solid mode.
const SOLID_COLOR: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);

// Compile-time switch: gradient vs the chapter 04 solid color; needs a rebuild to change.
const GRADIENT: bool = true;

// @location(0) is interpolated per fragment; @builtin(position) is consumed by rasterization.
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    output.position = POSITIONS[vertex_index];
    output.color = select(SOLID_COLOR.rgb, COLORS[vertex_index], GRADIENT);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
