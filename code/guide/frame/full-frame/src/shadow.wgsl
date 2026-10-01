// Depth-only pass; no fragment stage.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct ObjectRecord {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

// The empty group-1 hole keeps indices aligned with the lit pipeline.
@group(0) @binding(0) var<uniform> light_view_proj: mat4x4<f32>;
@group(2) @binding(0) var<storage, read> objects: array<ObjectRecord>;

struct ObjectIndex {
    index: u32,
};

var<immediate> object: ObjectIndex;

@vertex
fn vs_shadow(input: VertexInput) -> @builtin(position) vec4<f32> {
    let world = objects[object.index].model * input.position;
    return light_view_proj * world;
}
