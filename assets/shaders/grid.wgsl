
@group(1) @binding(0)
var grid_texture: texture_2d<u32>;

@group(1) @binding(1)
var color_map: texture_1d<f32>;

struct FragmentInput {
    #import bevy_sprite::mesh2d_vertex_output
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var scaled_uv = in.uv * vec2<f32>(1.0);
    var coords = vec2<i32>(round(scaled_uv).xy);
    var cell_material = textureLoad(grid_texture, coords, 0);
    var color = textureLoad(color_map, i32(cell_material.r), 0);
    return color;
}