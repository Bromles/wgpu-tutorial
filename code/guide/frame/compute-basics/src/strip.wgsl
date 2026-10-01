// The vertex stage pulls height and gray from storage; the CPU never reads back.

struct Params {
    count: u32,
};

// Strip frame: 257 cells across [-0.9, 0.9], growing up from y = -0.95.
const CELLS: f32 = 257.0;
const X0: f32 = -0.9;
const STRIP_WIDTH: f32 = 1.8;
const Y0: f32 = -0.95;
const STRIP_HEIGHT: f32 = 0.55;

struct VertexInput {
    @location(0) corner: vec2<f32>,
    @location(1) cell: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) value: f32,
};

@group(0) @binding(0) var<storage, read> VALUES: array<f32>;
@group(0) @binding(1) var<uniform> params: Params;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    // Slots at or past count can hold stale values from a previous, larger
    // count; select masks them so the surplus cells collapse to the baseline.
    let value = select(0.0, VALUES[input.cell], input.cell < params.count);
    let x = X0 + (f32(input.cell) + input.corner.x) * (STRIP_WIDTH / CELLS);
    let y = Y0 + input.corner.y * value * STRIP_HEIGHT;
    var output: VertexOutput;
    output.position = vec4<f32>(x, y, 0.5, 1.0);
    output.value = value;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(vec3<f32>(input.value), 1.0);
}
