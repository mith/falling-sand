@group(0) @binding(0)
var particle_grid: binding_array<texture_storage_2d<r32uint, read> >;

@group(0) @binding(1)
var color_map: texture_storage_1d<rgba8unorm, read>;

@group(0) @binding(2)
var color_texture: binding_array<texture_storage_2d<rgba8unorm, write> >;

fn extract_material(particle_value: u32) -> u32 {
    return (particle_value >> 10) & 0x3FF; // Shift right by 10 bits and mask with 0x3FF to get 10 bits representing the material
}

@compute @workgroup_size(8, 8, 1)
fn render_grid(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let index = i32(invocation_id.z);

    let particle_value = textureLoad(particle_grid[index], location).x; // Load the entire particle value
    let material = extract_material(particle_value); // Extract material from the particle value

    var color = textureLoad(color_map, i32(material)); // Use extracted material to load the color

    textureStore(color_texture[index], location, color);
}
