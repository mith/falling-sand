use std::mem::MaybeUninit;

use paste::paste;

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        system::{Commands, Query, SystemParam},
        world::Mut,
    },
    math::IVec2,
    utils::{hashbrown::HashMap, HashSet},
};

use crate::{
    chunk::Chunk,
    material::Material,
    particle_grid::{Particle, ParticleAttributeStore},
    util::positive_mod,
};

pub const CHUNK_SIZE: i32 = 100;

#[derive(Component)]
pub struct ChunkActive;

#[derive(Component)]
pub struct ChunkPosition(pub IVec2);

pub fn tile_pos_to_chunk_pos(x: i32, y: i32) -> IVec2 {
    let floor_div = |a: i32, b: i32| {
        if a < 0 && a % b != 0 {
            (a / b) - 1
        } else {
            a / b
        }
    };
    IVec2::new(floor_div(x, CHUNK_SIZE), floor_div(y, CHUNK_SIZE))
}

#[derive(SystemParam)]
pub struct FallingSandGridQuery<'w, 's> {
    commands: Commands<'w, 's>,
    chunks: Query<'w, 's, &'static mut Chunk>,
    active_chunks: Query<'w, 's, (&'static ChunkActive, &'static ChunkPosition)>,
    chunk_positions: Query<'w, 's, (Entity, &'static ChunkPosition)>,
}

impl<'w, 's> FallingSandGridQuery<'w, 's> {
    pub fn active_chunks(&self) -> HashSet<IVec2> {
        self.active_chunks.iter().map(|(_, pos)| pos.0).collect()
    }

    pub fn chunk_positions(&self) -> HashMap<IVec2, Entity> {
        self.chunk_positions
            .iter()
            .map(|(entity, position)| (position.0, entity))
            .collect()
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE, CHUNK_SIZE)
    }

    fn get_chunk(&mut self, x: i32, y: i32) -> &Chunk {
        let positions = self.chunk_positions();
        let chunk_entity = positions.get(&IVec2::new(x, y)).unwrap();
        self.chunks.get(*chunk_entity).unwrap()
    }

    fn get_chunk_mut(&mut self, x: i32, y: i32) -> Mut<Chunk> {
        let positions = self.chunk_positions();
        let chunk_entity = positions.get(&IVec2::new(x, y)).unwrap();
        self.chunks.get_mut(*chunk_entity).unwrap()
    }

    fn get_chunks_mut<const N: usize>(&mut self, chunks: &[IVec2; N]) -> [Mut<Chunk>; N] {
        let mut entities = [(); N].map(|_| MaybeUninit::uninit());

        for (i, pos) in chunks.iter().enumerate() {
            let chunk_positions = &self.chunk_positions();
            let chunk_entity = chunk_positions.get(pos).unwrap();
            entities[i] = MaybeUninit::new(*chunk_entity);
        }

        unsafe {
            self.chunks
                .get_many_mut(entities.map(|e| e.assume_init()))
                .unwrap()
        }
    }

    pub fn get_particle(&mut self, x: i32, y: i32) -> &Particle {
        let chunk_pos = tile_pos_to_chunk_pos(x, y);
        let chunk = self.get_chunk(chunk_pos.x, chunk_pos.y);
        chunk
            .get(positive_mod(x, CHUNK_SIZE), positive_mod(y, CHUNK_SIZE))
            .unwrap()
    }

    pub fn set_particle(&mut self, x: i32, y: i32, material: Material) {
        let chunk_pos = tile_pos_to_chunk_pos(x, y);
        self.get_chunk_mut(chunk_pos.x, chunk_pos.y).set(
            positive_mod(x, CHUNK_SIZE),
            positive_mod(y, CHUNK_SIZE),
            material,
        );
    }

    pub fn get_chunk_active(&mut self, x: i32, y: i32) -> bool {
        self.active_chunks().contains(&IVec2::new(x, y))
    }

    pub fn set_chunk_active(&mut self, x: i32, y: i32, active: bool) {
        let positions = self.chunk_positions();
        let chunk_entity = positions.get(&IVec2::new(x, y)).unwrap();
        if active {
            self.commands.entity(*chunk_entity).insert(ChunkActive);
        } else {
            self.commands.entity(*chunk_entity).remove::<ChunkActive>();
        }
    }
}

macro_rules! define_attributes_and_swap {
    ($($attr:ident: $type:ty),* $(,)?) => {
        pub struct ParticleAttributes {
            $(pub $attr: ParticleAttributeStore<$type>,)*
        }

        impl ParticleAttributes {
            pub fn new(size: usize) -> Self {
                ParticleAttributes {
                    $($attr: ParticleAttributeStore::new(size),)*
                }
            }
        }

        impl<'w, 's> FallingSandGridQuery<'w, 's> {
            pub fn swap_particles(&mut self, a: (i32, i32), b: (i32, i32)) {
                let chunk_a_pos = tile_pos_to_chunk_pos(a.0, a.1);
                let chunk_b_pos = tile_pos_to_chunk_pos(b.0, b.1);

                let particle_pos_a = (positive_mod(a.0, CHUNK_SIZE), positive_mod(a.1, CHUNK_SIZE));
                let particle_pos_b = (positive_mod(b.0, CHUNK_SIZE), positive_mod(b.1, CHUNK_SIZE));

                if chunk_a_pos == chunk_b_pos {
                    let mut chunk = self.get_chunk_mut(chunk_a_pos.x, chunk_a_pos.y);
                    chunk.swap_particles(
                        particle_pos_a,
                        particle_pos_b
                    );
                    // self.set_chunk_active(chunk_a_pos.x, chunk_a_pos.y, true);
                } else {
                    let [mut chunk_a, mut chunk_b] = self.get_chunks_mut(&[chunk_a_pos, chunk_b_pos]);

                    let particle_a_id = chunk_a.get(particle_pos_a.0, particle_pos_a.1).unwrap().id;
                    let particle_b_id = chunk_b.get(particle_pos_b.0, particle_pos_b.1).unwrap().id;

                    $(
                        std::mem::swap(
                            chunk_a.attributes_mut().$attr.get_mut(particle_a_id).unwrap(),
                            chunk_b.attributes_mut().$attr.get_mut(particle_b_id).unwrap(),
                        );
                    )*

                    // self.set_chunk_active(chunk_a_pos.x, chunk_a_pos.y, true);
                    // self.set_chunk_active(chunk_b_pos.x, chunk_b_pos.y, true);
                }
            }

            $(
                paste! {
                    pub fn [<get_ $attr>](&mut self, x: i32, y: i32) -> $type {
                        let chunk_pos = tile_pos_to_chunk_pos(x, y);
                        let chunk = self.get_chunk(chunk_pos.x, chunk_pos.y);
                        let particle = chunk.get(positive_mod(x, CHUNK_SIZE), positive_mod(y, CHUNK_SIZE)).unwrap();
                        *chunk.attributes().$attr.get(particle.id).unwrap()
                    }

                    pub fn [<set_ $attr>](&mut self, x: i32, y: i32, value: $type) {
                        let chunk_pos = tile_pos_to_chunk_pos(x, y);
                        let mut chunk = self.get_chunk_mut(chunk_pos.x, chunk_pos.y);
                        let particle = *chunk.get_mut(positive_mod(x, CHUNK_SIZE), positive_mod(y, CHUNK_SIZE)).unwrap();
                        chunk.attributes_mut().$attr.set(particle.id, value);
                    }
                }
            )*
        }
    };
}

define_attributes_and_swap! {
    dirty: bool,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tile_pos_to_chunk_pos() {
        assert_eq!(tile_pos_to_chunk_pos(0, 0), IVec2::new(0, 0));
        assert_eq!(tile_pos_to_chunk_pos(63, 63), IVec2::new(0, 0));
        assert_eq!(tile_pos_to_chunk_pos(64, 64), IVec2::new(1, 1));
        assert_eq!(tile_pos_to_chunk_pos(65, 65), IVec2::new(1, 1));
        assert_eq!(tile_pos_to_chunk_pos(0, -1), IVec2::new(0, -1));
    }
}
