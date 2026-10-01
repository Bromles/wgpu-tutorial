// Lambert with shadow visibility into a linear HDR frame.
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

// Role: per pass. The camera chain plus the shadow switch of the H key.
struct PassParams {
    view_proj: mat4x4<f32>,
    shadows: u32,
};

// Role: per material. One vec4 = 16 bytes, no padding.
struct Material {
    albedo: vec4<f32>,
};

// Role: per object. Two mat4x4 = a 128-byte stride.
struct ObjectRecord {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> pass_params: PassParams;
@group(1) @binding(0) var<uniform> material: Material;
@group(2) @binding(0) var<storage, read> objects: array<ObjectRecord>;

// Role: per frame. The light as an orthographic camera, plus its map.
@group(3) @binding(0) var<uniform> light_view_proj: mat4x4<f32>;
@group(3) @binding(1) var SHADOW: texture_depth_2d;

// Role: per draw; written by set_immediates, read like any module-scope variable.
struct ObjectIndex {
    index: u32,
};

var<immediate> object: ObjectIndex;

// The chapter 23 light; the direction is the precomputed unit vector.
const AMBIENT = 0.1;
const INTENSITY = 0.6;
const LIGHT_DIR = vec3<f32>(-0.410365, 0.911922, 0.0);
const SHADOW_BIAS = 0.0005;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let record = objects[object.index];
    // P * V * M, right to left: model first, camera second.
    let world = record.model * input.position;
    var output: VertexOutput;
    output.position = pass_params.view_proj * world;
    output.world_position = world.xyz;
    // Normals ride the inverse transpose; w = 0 keeps translation out.
    output.world_normal = (record.normal_matrix * input.normal).xyz;
    return output;
}

/// One-tap shadow comparison: project, address a texel, compare; no PCF.
fn visibility(world_position: vec3<f32>) -> f32 {
    let light_clip = light_view_proj * vec4<f32>(world_position, 1.0);
    // Orthographic projection: w = 1, the divide would change nothing.
    let ndc = light_clip.xyz;
    // Y scale is negative: the viewport wrote row 0 for NDC y = +1.
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    // Outside the light frustum nothing was ever recorded: lit.
    if (uv.x < 0.0 || uv.x >= 1.0 || uv.y < 0.0 || uv.y >= 1.0) {
        return 1.0;
    }
    // Size comes from the texture.
    let map_size = vec2<f32>(textureDimensions(SHADOW));
    let texel = vec2<i32>(uv * map_size);
    let closest = textureLoad(SHADOW, texel, 0);
    // glam maps z to [0, 1] already; the bias on the stored depth
    // moves past depth acne.
    return select(1.0, 0.0, ndc.z > closest + SHADOW_BIAS);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.world_normal);
    let diffuse = max(dot(n, LIGHT_DIR), 0.0);
    var lit = 1.0;
    if (pass_params.shadows == 1u) {
        lit = visibility(input.world_position);
    }
    // Linear HDR out: the float target stores it, tone mapping comes later.
    let color = material.albedo.rgb * (AMBIENT + INTENSITY * diffuse * lit);
    return vec4<f32>(color, 1.0);
}
