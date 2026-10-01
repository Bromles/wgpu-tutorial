// One invocation per slot; the guard no-ops the workgroup round-up overflow.

struct Params {
    count: u32,
};

@group(0) @binding(0) var<storage, read_write> VALUES: array<f32>;
@group(0) @binding(1) var<uniform> params: Params;

@compute
@workgroup_size(64)
fn fill(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < params.count) {
        VALUES[id.x] = f32(id.x) / f32(params.count - 1u);
    }
}
