@group(0) @binding(0)
var input_tex: texture_2d<f32>;

@group(0) @binding(1)
var output_tex: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input_tex);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    var color = vec4<f32>(0.0);
    let radius = 4;
    for (var dy: i32 = -radius; dy <= radius; dy++) {
        for (var dx: i32 = -radius; dx <= radius; dx++) {
            let coord = clamp(
                vec2<i32>(i32(id.x) + dx, i32(id.y) + dy),
                vec2<i32>(0, 0),
                vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1),
            );
            color += textureLoad(input_tex, coord, 0);
        }
    }
    let total = f32((2 * radius + 1) * (2 * radius + 1));
    textureStore(output_tex, vec2<i32>(id.xy), color / total);
}
