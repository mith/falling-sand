@group(0) @binding(0)
var source: texture_storage_2d<r32uint, read>;

@group(0) @binding(1)
var target: texture_storage_2d<r32uint, write>;

@compute @workgroup_size(10, 10, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let value = textureLoad(source, location).r;

    textureStore(target, location, vec4<u32>(value, 0u, 0u, 1u));
}