struct Particle {
    pos: vec3<f32>,
    vel: vec3<f32>,
    life: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read> particles: array<Particle>;

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: CameraUniforms;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VertexOutput {
    let p = particles[iid];
    let quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.05, -0.05),
        vec2<f32>( 0.05, -0.05),
        vec2<f32>( 0.05,  0.05),
        vec2<f32>(-0.05, -0.05),
        vec2<f32>( 0.05,  0.05),
        vec2<f32>(-0.05,  0.05),
    );
    let offset = quad_pos[vid % 6u];
    let world_pos = p.pos
        + camera.camera_right.xyz * offset.x
        + camera.camera_up.xyz * offset.y;

    var output: VertexOutput;
    output.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    let t = clamp(p.life / 3.0, 0.0, 1.0);
    output.color = vec4<f32>(1.0, 0.6 * t + 0.2, 0.1, t);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
