use rand::Rng;

use crate::{
    material::Material,
    process_chunks::{process_chunks_parallel, ChunksParam},
};

pub fn fire_to_smoke(mut grid: ChunksParam) {
    process_chunks_parallel(&mut grid, |chunk_pos, grid| {
        let chunk_size = grid.chunk_size();
        let min_y = chunk_pos.y * chunk_size.y;
        let max_y = (chunk_pos.y + 1) * chunk_size.y;
        for y in min_y..max_y {
            let min_x = chunk_pos.x * chunk_size.x;
            let max_x = (chunk_pos.x + 1) * chunk_size.x;
            for x in min_x..max_x {
                let particle = grid.get_particle(x, y);
                if particle.material == Material::Fire
                    && grid.center_chunk_mut().rng().gen_bool(0.1)
                {
                    grid.set_particle(x, y, Material::Smoke);
                }
            }
        }
    });
}
