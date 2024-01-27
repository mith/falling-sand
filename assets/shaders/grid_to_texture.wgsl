@group(0) @binding(0)
var particle_grid: texture_storage_2d<rg32uint, read>;

@group(0) @binding(1)
var color_map: texture_storage_1d<rgba8unorm, read>;

@group(0) @binding(2)
var color_texture: texture_storage_2d<rgba8unorm, write>;


@compute @workgroup_size(10, 10, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let particle_material = textureLoad(particle_grid, location).r;

    var color = textureLoad(color_map, i32(particle_material));

    textureStore(color_texture, location, color);
}