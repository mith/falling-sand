use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockWriteGuard},
};

use bevy::{
    ecs::{
        entity::Entity,
        system::{Query, Res, SystemParam},
    },
    math::IVec2,
};

use crate::{
    chunk::{Chunk, ChunkData},
    chunk_positions::ChunkPositions,
    consts::{CHUNK_SIZE, SHIFT},
    material::Material,
    particle_attributes::swap_particles_between_chunks,
    particle_grid::Particle,
    util::{positive_mod, tile_pos_to_chunk_pos},
};

pub struct ChunkNeighborhoodView<'a> {
    chunks: [RwLockWriteGuard<'a, ChunkData>; 9],
}

impl<'a> ChunkNeighborhoodView<'a> {
    pub fn new(chunks: &'a [&Chunk]) -> ChunkNeighborhoodView<'a> {
        debug_assert_eq!(chunks.len(), 9, "Chunks must be of length 9");

        let chunks = {
            const ARRAY_REPEAT_VALUE: MaybeUninit<std::sync::RwLockWriteGuard<'_, ChunkData>> =
                MaybeUninit::uninit();

            let mut chunks_uninit: [MaybeUninit<RwLockWriteGuard<'a, ChunkData>>; 9] =
                [ARRAY_REPEAT_VALUE; 9];

            chunks_uninit
                .iter_mut()
                .zip(chunks.iter())
                .for_each(|(p, chunk)| {
                    let chunk = chunk.write().unwrap();
                    p.write(chunk);
                });

            unsafe { chunks_uninit.map(|p| p.assume_init()) }
        };

        ChunkNeighborhoodView { chunks }
    }

    pub fn center_chunk_mut(&mut self) -> &mut ChunkData {
        self.chunks[4].deref_mut()
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE, CHUNK_SIZE)
    }

    fn get_chunk_at_chunk_pos(&self, position: IVec2) -> Option<&ChunkData> {
        self.chunks
            .get(chunk_pos_to_index(position))
            .map(|chunk| chunk.deref())
    }

    fn get_chunk_at_chunk_pos_mut(&mut self, position: IVec2) -> Option<&mut ChunkData> {
        self.chunks
            .get_mut(chunk_pos_to_index(position))
            .map(|chunk| chunk.deref_mut())
    }

    fn get_chunk_at_neighborhood_pos(&self, position: IVec2) -> Option<(IVec2, &ChunkData)> {
        let chunk_pos = neighborhood_pos_to_chunk_pos(position);
        self.get_chunk_at_chunk_pos(chunk_pos)
            .map(|chunk| (chunk_pos, chunk))
    }

    fn get_chunk_at_neighborhood_pos_mut(
        &mut self,
        position: IVec2,
    ) -> Option<(IVec2, &mut ChunkData)> {
        let chunk_pos = neighborhood_pos_to_chunk_pos(position);
        self.get_chunk_at_chunk_pos_mut(chunk_pos)
            .map(|chunk| (chunk_pos, chunk))
    }

    pub fn get_two_chunks_mut(
        &mut self,
        chunk_pos_a: IVec2,
        chunk_pos_b: IVec2,
    ) -> (&mut ChunkData, &mut ChunkData) {
        debug_assert_ne!(chunk_pos_a, chunk_pos_b, "Chunks must be different");

        let mut first_index = chunk_pos_to_index(chunk_pos_a);
        let mut second_index = chunk_pos_to_index(chunk_pos_b);

        let flipped = if first_index > second_index {
            std::mem::swap(&mut first_index, &mut second_index);
            true
        } else {
            false
        };

        let (first_half, second_half) = self.chunks.split_at_mut(second_index);
        let chunk_a = &mut first_half[first_index];
        let chunk_b = &mut second_half[0];

        if flipped {
            (chunk_b, chunk_a)
        } else {
            (chunk_a, chunk_b)
        }
    }

    pub fn get_particle(&self, position: IVec2) -> &Particle {
        let (chunk_pos, chunk) = self.get_chunk_at_neighborhood_pos(position).unwrap();
        let local_pos = neighborhood_pos_to_local_pos(position, chunk_pos);
        chunk.get_particle(local_pos).unwrap()
    }

    pub fn set_particle(&mut self, position: IVec2, material: Material) {
        let (chunk_pos, chunk) = self.get_chunk_at_neighborhood_pos_mut(position).unwrap();
        let local_pos = neighborhood_pos_to_local_pos(position, chunk_pos);
        chunk.set_particle_material(local_pos, material);
    }

    pub fn swap_particles(&mut self, a: IVec2, b: IVec2) {
        let chunk_a_pos = neighborhood_pos_to_chunk_pos(a);
        let chunk_b_pos = neighborhood_pos_to_chunk_pos(b);

        let particle_pos_a = neighborhood_pos_to_local_pos(a, chunk_a_pos);
        let particle_pos_b = neighborhood_pos_to_local_pos(b, chunk_b_pos);

        if chunk_a_pos == chunk_b_pos {
            let chunk = self.get_chunk_at_chunk_pos_mut(chunk_a_pos).unwrap();
            chunk.swap_particles(particle_pos_a.into(), particle_pos_b.into());
        } else {
            let (chunk_a, chunk_b) = self.get_two_chunks_mut(chunk_a_pos, chunk_b_pos);

            swap_particles_between_chunks(chunk_a, particle_pos_a, chunk_b, particle_pos_b);
        }
    }
}

pub fn neighborhood_pos_to_chunk_pos(position: IVec2) -> IVec2 {
    let chunk_x = position.x >> SHIFT;
    let chunk_y = position.y >> SHIFT;

    IVec2::new(chunk_x, chunk_y)
}

fn neighborhood_pos_to_local_pos(position: IVec2, chunk_pos: IVec2) -> IVec2 {
    let IVec2 {
        x: chunk_x,
        y: chunk_y,
    } = chunk_pos;

    let local_x = position.x - (chunk_x << SHIFT);
    let local_y = position.y - (chunk_y << SHIFT);

    IVec2::new(local_x, local_y)
}

fn chunk_pos_to_index(pos: IVec2) -> usize {
    (pos.x + (pos.y) * 3) as usize
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_neighborhood_pos_to_chunk_pos() {
        assert_eq!(
            neighborhood_pos_to_chunk_pos(IVec2::new(0, 0)),
            IVec2::new(0, 0)
        );
        assert_eq!(
            neighborhood_pos_to_chunk_pos(IVec2::new(63, 63)),
            IVec2::new(0, 0)
        );
        assert_eq!(
            neighborhood_pos_to_chunk_pos(IVec2::new(64, 64)),
            IVec2::new(1, 1)
        );
        assert_eq!(
            neighborhood_pos_to_chunk_pos(IVec2::new(65, 65)),
            IVec2::new(1, 1)
        );
        assert_eq!(
            neighborhood_pos_to_chunk_pos(IVec2::new(128, 0)),
            IVec2::new(2, 0)
        );
        assert_eq!(
            neighborhood_pos_to_chunk_pos(IVec2::new(0, 128)),
            IVec2::new(0, 2)
        )
    }

    #[test]
    fn test_neighborhood_pos_to_local_pos() {
        assert_eq!(
            neighborhood_pos_to_local_pos(IVec2::new(0, 0), IVec2::new(0, 0)),
            IVec2::new(0, 0)
        );
        assert_eq!(
            neighborhood_pos_to_local_pos(IVec2::new(63, 63), IVec2::new(0, 0)),
            IVec2::new(63, 63)
        );
        assert_eq!(
            neighborhood_pos_to_local_pos(IVec2::new(64, 64), IVec2::new(1, 1)),
            IVec2::new(0, 0)
        );
        assert_eq!(
            neighborhood_pos_to_local_pos(IVec2::new(65, 65), IVec2::new(1, 1)),
            IVec2::new(1, 1)
        );
        assert_eq!(
            neighborhood_pos_to_local_pos(IVec2::new(128, 0), IVec2::new(2, 0)),
            IVec2::new(0, 0)
        );
        assert_eq!(
            neighborhood_pos_to_local_pos(IVec2::new(0, 128), IVec2::new(0, 2)),
            IVec2::new(0, 0)
        )
    }

    #[test]
    fn test_chunk_pos_to_index() {
        assert_eq!(chunk_pos_to_index(IVec2::new(0, 0)), 0);
        assert_eq!(chunk_pos_to_index(IVec2::new(1, 0)), 1);
        assert_eq!(chunk_pos_to_index(IVec2::new(2, 0)), 2);
        assert_eq!(chunk_pos_to_index(IVec2::new(0, 1)), 3);
        assert_eq!(chunk_pos_to_index(IVec2::new(1, 1)), 4);
        assert_eq!(chunk_pos_to_index(IVec2::new(2, 1)), 5);
        assert_eq!(chunk_pos_to_index(IVec2::new(0, 2)), 6);
        assert_eq!(chunk_pos_to_index(IVec2::new(1, 2)), 7);
        assert_eq!(chunk_pos_to_index(IVec2::new(2, 2)), 8);
    }
}
