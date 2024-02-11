use bevy::ecs::system::{Query, ResMut};
use rand::Rng;

use crate::{chunk::Chunk, falling_sand::FallingSandRng, material::Material};

pub fn fire_to_smoke(mut grid_query: Query<&mut Chunk>, mut rng: ResMut<FallingSandRng>) {
    for mut grid in grid_query.iter_mut() {
        for x in 0..grid.size().x {
            for y in 0..grid.size().y {
                let particle = grid.get(x, y).unwrap();
                if particle.material == Material::Fire && rng.0.gen_bool(0.1) {
                    grid.set(x, y, Material::Smoke);
                }
            }
        }
    }
}
