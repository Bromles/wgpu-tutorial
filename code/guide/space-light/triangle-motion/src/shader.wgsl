// An asymmetric flat triangle moves in the plane: X to the right, Y up.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct Params {
    translate: vec2<f32>,
    scale_angle: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    // Scale, rotate the scaled offset, then translate; cos/sin give the images of the unit axes.
    let local = input.position.xy * params.scale_angle.x;
    let angle = params.scale_angle.y;
    let c = cos(angle);
    let s = sin(angle);
    let rotated = vec2<f32>(c * local.x - s * local.y, s * local.x + c * local.y);
    var output: VertexOutput;
    output.position = vec4<f32>(rotated + params.translate, 0.5, 1.0);
    output.color = input.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
