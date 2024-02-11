use bevy::{
    ecs::component::Component,
    math::IVec2,
    utils::{hashbrown::HashMap, HashSet},
};

use crate::{chunk::Chunk, material::Material, particle_grid::Particle};

const CHUNK_SIZE: usize = 64;

#[derive(Component)]
pub struct FallingSandGrid {
    chunks: HashMap<IVec2, Chunk>,
    active_chunks: HashSet<IVec2>,
}

impl FallingSandGrid {
    pub fn new() -> FallingSandGrid {
        // Create a 3x3 grid of chunks with a single active chunk at the origin
        let chunks = HashMap::from([(IVec2::new(0, 0), Chunk::new((CHUNK_SIZE, CHUNK_SIZE)))]);
        FallingSandGrid {
            chunks,
            active_chunks: HashSet::from([IVec2::new(0, 0)]),
        }
    }

    pub fn active_chunks(&self) -> &HashSet<IVec2> {
        &self.active_chunks
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE as i32, CHUNK_SIZE as i32)
    }

    pub fn get_chunk(&mut self, x: i32, y: i32) -> &Chunk {
        if !self.chunks.contains_key(&IVec2::new(x, y)) {
            self.chunks
                .insert(IVec2::new(x, y), Chunk::new((CHUNK_SIZE, CHUNK_SIZE)));
        }
        self.chunks.get(&IVec2::new(x, y)).unwrap()
    }

    pub fn get_chunk_mut(&mut self, x: i32, y: i32) -> &mut Chunk {
        if !self.chunks.contains_key(&IVec2::new(x, y)) {
            self.chunks
                .insert(IVec2::new(x, y), Chunk::new((CHUNK_SIZE, CHUNK_SIZE)));
        }
        self.active_chunks.insert(IVec2::new(x, y));
        self.chunks.get_mut(&IVec2::new(x, y)).unwrap()
    }

    pub fn get_particle(&mut self, x: i32, y: i32) -> &Particle {
        self.get_chunk(x / CHUNK_SIZE as i32, y / CHUNK_SIZE as i32)
            .get(x % CHUNK_SIZE as i32, y % CHUNK_SIZE as i32)
            .unwrap()
    }

    pub fn set_particle(&mut self, x: i32, y: i32, material: Material) {
        let chunk_pos = IVec2::new(x / CHUNK_SIZE as i32, y / CHUNK_SIZE as i32);
        self.get_chunk_mut(chunk_pos.x, chunk_pos.y).set(
            x % CHUNK_SIZE as i32,
            y % CHUNK_SIZE as i32,
            material,
        );
        self.active_chunks.insert(chunk_pos);
    }

    pub fn swap_particles(&mut self, a: (i32, i32), b: (i32, i32)) {
        // If the particles are in the same chunk, we can just swap them
        if a.0 / CHUNK_SIZE as i32 == b.0 / CHUNK_SIZE as i32
            && a.1 / CHUNK_SIZE as i32 == b.1 / CHUNK_SIZE as i32
        {
            let chunk_pos = IVec2::new(a.0 / CHUNK_SIZE as i32, a.1 / CHUNK_SIZE as i32);
            let chunk = self.get_chunk_mut(chunk_pos.x, chunk_pos.y);
            chunk.swap_particles(
                (a.0 % CHUNK_SIZE as i32, a.1 % CHUNK_SIZE as i32),
                (b.0 % CHUNK_SIZE as i32, b.1 % CHUNK_SIZE as i32),
            );
            self.active_chunks.insert(chunk_pos);
        } else {
            // If the particles are in different chunks, we need to move them between chunks
            let particle_a = *self.get_particle(a.0, a.1);
            let particle_b = *self.get_particle(b.0, b.1);

            self.set_particle(a.0, a.0, particle_b.material);
            self.set_particle(b.0, b.0, particle_a.material);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_falling_sand_grid() {
        let mut grid = FallingSandGrid::new();
        assert_eq!(grid.get_particle(0, 0).material, Material::Air);
        grid.set_particle(0, 0, Material::Sand);
        assert_eq!(grid.get_particle(0, 0).material, Material::Sand);
        grid.swap_particles((0, 0), (1, 1));
        assert_eq!(grid.get_particle(0, 0).material, Material::Air);
        assert_eq!(grid.get_particle(1, 1).material, Material::Sand);
    }

    #[test]
    fn test_falling_sand_grid_swap_across_chunks() {
        let mut grid = FallingSandGrid::new();
        assert_eq!(grid.get_particle(0, 0).material, Material::Air);
        grid.set_particle(0, 0, Material::Sand);
        assert_eq!(grid.get_particle(0, 0).material, Material::Sand);
        grid.set_particle(CHUNK_SIZE as i32, CHUNK_SIZE as i32, Material::Water);
        assert_eq!(
            grid.get_particle(CHUNK_SIZE as i32, CHUNK_SIZE as i32)
                .material,
            Material::Water
        );
        grid.swap_particles((0, 0), (CHUNK_SIZE as i32, CHUNK_SIZE as i32));
        assert_eq!(grid.get_particle(0, 0).material, Material::Water);
        assert_eq!(
            grid.get_particle(CHUNK_SIZE as i32, CHUNK_SIZE as i32)
                .material,
            Material::Sand
        );
    }
}
