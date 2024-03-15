#ifdef BINDLESS
@group(0) @binding(0)
var particle_grid: binding_array<texture_2d<u32> >;
#else
@group(0) @binding(0)
var particle_grid: texture_2d<u32>;
#endif

@group(0) @binding(1)
var color_map: texture_1d<f32>;

#ifdef BINDLESS
@group(0) @binding(2)
var color_texture: binding_array<texture_storage_2d<rgba32float, write> >;
#else
@group(0) @binding(2)
var color_texture: texture_storage_2d<rgba32float, write>;
#endif

fn extract_material(particle_value: u32) -> u32 {
    return (particle_value >> 10) & 0x3FF; // Shift right by 10 bits and mask with 0x3FF to get 10 bits representing the material
}

@compute @workgroup_size(8, 8, 1)
fn render_grid(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

#ifdef BINDLESS
    let index = i32(invocation_id.z);
    let particle_value = textureLoad(particle_grid[index], location, 0).x; // Load the entire particle value
#else
    let particle_value = textureLoad(particle_grid, location, 0).x; // Load the entire particle value
#endif

    let material = extract_material(particle_value); // Extract material from the particle value

    var color = textureLoad(color_map, i32(material), 0); // Use extracted material to load the color

#ifdef BINDLESS
    textureStore(color_texture[index], location, color);
#else
    textureStore(color_texture, location, color);
#endif
}
