// One mesh, many objects: the vertex shader reads the record its draw selected via the immediate index.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

// Role: per frame. The camera chain plus the eye for the specular term.
struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
};

// Role: per material. Two vec4 = 32 bytes, no padding.
struct Material {
    albedo: vec4<f32>,
    specular: vec4<f32>,
};

// Role: per object. Two mat4x4 = a 128-byte stride.
struct ObjectRecord {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> material: Material;
@group(2) @binding(0) var<storage, read> objects: array<ObjectRecord>;

// Role: per draw. The record index, written by set_immediates before each draw.
struct ObjectIndex {
    index: u32,
};

var<immediate> object: ObjectIndex;

// The single light of chapter 25, kept as constants.
const LIGHT_DIR = vec3<f32>(0.0, 0.0, 1.0);
const AMBIENT = 0.1;
const INTENSITY = 0.6;
const SHININESS = 32.0;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let record = objects[object.index];
    // P * V * M, right to left: the model acts on the vertex first.
    let world = record.model * input.position;
    var output: VertexOutput;
    output.position = camera.view_proj * world;
    output.world_position = world.xyz;
    // Normals ride the inverse transpose (chapter 24); w = 0 keeps translation out.
    output.world_normal = (record.normal_matrix * input.normal).xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.world_normal);
    // V points from the fragment toward the eye; it varies across the plane.
    let v = normalize(camera.eye.xyz - input.world_position);
    let diffuse = max(dot(n, LIGHT_DIR), 0.0);
    var specular = 0.0;
    // One-sided: no specular unless the light and the eye face the normal.
    if (diffuse > 0.0 && dot(n, v) > 0.0) {
        let l_plus_v = LIGHT_DIR + v;
        // V exactly against L leaves no half vector; treat the highlight as absent.
        if (length(l_plus_v) > 0.0) {
            let h = normalize(l_plus_v);
            specular = pow(max(dot(n, h), 0.0), SHININESS);
        }
    }
    let color = material.albedo.rgb * (AMBIENT + INTENSITY * diffuse)
        + material.specular.rgb * (INTENSITY * specular);
    return vec4<f32>(color, 1.0);
}
