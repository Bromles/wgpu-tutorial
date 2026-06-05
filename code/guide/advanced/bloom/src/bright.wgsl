struct Params {
    threshold: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0)
var input_tex: texture_2d<f32>;

@group(0) @binding(1)
var output_tex: texture_storage_2d<rgba16float, write>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input_tex);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }
    let color = textureLoad(input_tex, vec2<i32>(id.xy), 0);
    let brightness = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    if (brightness > params.threshold) {
        textureStore(output_tex, vec2<i32>(id.xy), color);
    } else {
        textureStore(output_tex, vec2<i32>(id.xy), vec4<f32>(0.0));
    }
}
