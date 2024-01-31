@group(0) @binding(0)
var particle_grid: texture_storage_2d<rgba32uint, read>;

@group(0) @binding(1)
var color_map: texture_storage_1d<rgba8unorm, read>;

@group(0) @binding(2)
var color_texture: texture_storage_2d<rgba8unorm, write>;


@compute @workgroup_size(10, 10, 1)
fn render_grid(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let particle_material = textureLoad(particle_grid, location).r;
    let particle_pressure = bitcast<f32>(textureLoad(particle_grid, location).g);

    var color = textureLoad(color_map, i32(particle_material));

    // make color darker based on pressure
    // pressure is in range [0, 10]
    var inverse_pressure_normalized = 1.0 - (particle_pressure / 10.0);
    color.r *= inverse_pressure_normalized;
    color.g *= inverse_pressure_normalized;
    color.b *= inverse_pressure_normalized;

    textureStore(color_texture, location, color);
}