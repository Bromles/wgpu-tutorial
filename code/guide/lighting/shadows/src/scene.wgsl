// Scene pass — renders cubes with shadow comparison
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) light_pos: vec3<f32>,
    @location(3) uv: vec2<f32>,
}

struct CameraUniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

struct LightUniforms {
    light_view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    ambient: f32,
}

@group(1) @binding(0)
var<uniform> light: LightUniforms;

@group(1) @binding(1)
var shadow_tex: texture_depth_2d;

@group(1) @binding(2)
var shadow_sampler: sampler_comparison;

@group(1) @binding(3)
var diffuse_tex: texture_2d<f32>;

@group(1) @binding(4)
var diffuse_sampler: sampler;

struct InstanceInput {
    @location(3) model_col0: vec4<f32>,
    @location(4) model_col1: vec4<f32>,
    @location(5) model_col2: vec4<f32>,
    @location(6) model_col3: vec4<f32>,
    @location(7) normal_col0: vec3<f32>,
    @location(8) normal_col1: vec3<f32>,
    @location(9) normal_col2: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(instance.model_col0, instance.model_col1, instance.model_col2, instance.model_col3);
    let normal_matrix = mat3x3<f32>(instance.normal_col0, instance.normal_col1, instance.normal_col2);
    var output: VertexOutput;
    let world_pos = model * vec4<f32>(input.position, 1.0);
    output.position = camera.view_proj * world_pos;
    output.normal = normal_matrix * input.normal;
    output.world_pos = world_pos.xyz;
    let light_clip = light.light_view_proj * world_pos;
    output.light_pos = light_clip.xyz / light_clip.w;
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(-light.light_dir);
    let diffuse = max(dot(normal, light_dir), 0.0);

    // Shadow comparison
    let light_coords = input.light_pos;
    let shadow_uv = vec3<f32>(
        light_coords.x * 0.5 + 0.5,
        1.0 - (light_coords.y * 0.5 + 0.5),
        light_coords.z
    );
    var shadow = 0.0;
    if (shadow_uv.x >= 0.0 && shadow_uv.x <= 1.0 &&
        shadow_uv.y >= 0.0 && shadow_uv.y <= 1.0) {
        shadow = textureSampleCompare(shadow_tex, shadow_sampler, shadow_uv.xy, shadow_uv.z);
    } else {
        shadow = 1.0;
    }

    let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);
    let intensity = light.ambient + diffuse * shadow * (1.0 - light.ambient);
    return vec4<f32>(tex_color.rgb * vec3<f32>(1.0, 0.95, 0.85) * intensity, 1.0);
}
