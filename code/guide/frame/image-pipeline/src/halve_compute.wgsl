// Invocation (x, y) handles texel (x, y); the guard no-ops the 8x8 round-up.

@group(0) @binding(0) var INPUT: texture_2d<f32>;
@group(0) @binding(1) var RESULT: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(8, 8)
fn halve(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(INPUT);
    if (id.x < size.x && id.y < size.y) {
        let texel = vec2<i32>(id.xy);
        let color = textureLoad(INPUT, texel, 0);
        textureStore(RESULT, texel, vec4<f32>(color.rgb * 0.5, color.a));
    }
}
