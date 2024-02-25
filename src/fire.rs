use bevy::{log::info_span, math::IVec2};
use rand::Rng;

use crate::{
    material::Material,
    process_chunks::{process_chunks_dense, ChunksParam},
};

pub fn fire_to_smoke(grid: ChunksParam) {
    process_chunks_dense(&grid, |chunk_pos, chunk| {
        let _ = chunk_pos;
        let span = info_span!("fire_to_smoke");
        let _guard = span.enter();
        let chunk_size = chunk.size();
        let max_y = chunk_size.y;
        for y in 0..max_y {
            let max_x = chunk_size.x;
            for x in 0..max_x {
                let particle_position = IVec2::new(x, y);
                let particle = *chunk.get_particle(particle_position).unwrap();
                if particle.material == Material::Fire && chunk.rng().gen_bool(0.1) {
                    chunk.set_particle_material(particle_position, Material::Smoke);
                }
            }
        }
    });
}
