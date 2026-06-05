struct Particle {
    pos: vec3<f32>,
    _pad0: f32,
    vel: vec3<f32>,
    life: f32,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

struct Params {
    dt: f32,
    gravity: f32,
}

@group(0) @binding(1)
var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&particles)) {
        return;
    }

    var p = particles[idx];
    p.life -= params.dt;
    if (p.life <= 0.0) {
        p.pos = vec3<f32>(0.0, 0.0, 0.0);
        p.vel = vec3<f32>(0.0, 0.0, 0.0);
        p.life = 0.0;
        particles[idx] = p;
        return;
    }

    p.vel.y -= params.gravity * params.dt;
    p.pos += p.vel * params.dt;
    particles[idx] = p;
}
