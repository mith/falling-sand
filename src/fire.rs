use bevy::ecs::system::ResMut;
use rand::Rng;

use crate::{
    falling_sand::FallingSandRng, falling_sand_grid::FallingSandGridQuery, material::Material,
};

pub fn fire_to_smoke(mut grid: FallingSandGridQuery, mut rng: ResMut<FallingSandRng>) {
    for chunk_pos in grid.active_chunks() {
        let chunk_size = grid.chunk_size();
        let min_y = chunk_pos.y * chunk_size.y;
        let max_y = (chunk_pos.y + 1) * chunk_size.y;
        for y in min_y..max_y {
            let min_x = chunk_pos.x * chunk_size.x;
            let max_x = (chunk_pos.x + 1) * chunk_size.x;
            for x in min_x..max_x {
                let particle = grid.get_particle(x, y);
                if particle.material == Material::Fire && rng.0.gen_bool(0.1) {
                    grid.set_particle(x, y, Material::Smoke);
                }
            }
        }
    }
}
