@group(0) @binding(0)
var input_tex: texture_2d<f32>;

@group(0) @binding(1)
var output_tex: texture_storage_2d<rgba16float, write>;

struct Params {
    direction: vec2<f32>,
}

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input_tex);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    var result = textureLoad(input_tex, vec2<i32>(id.xy), 0) * weights[0];

    for (var i: i32 = 1; i < 5; i++) {
        let offset = params.direction * f32(i);
        let coord1 = clamp(vec2<i32>(vec2<f32>(f32(id.x), f32(id.y)) + offset),
                           vec2<i32>(0), vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1));
        let coord2 = clamp(vec2<i32>(vec2<f32>(f32(id.x), f32(id.y)) - offset),
                           vec2<i32>(0), vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1));
        result += textureLoad(input_tex, coord1, 0) * weights[i];
        result += textureLoad(input_tex, coord2, 0) * weights[i];
    }

    textureStore(output_tex, vec2<i32>(id.xy), result);
}
