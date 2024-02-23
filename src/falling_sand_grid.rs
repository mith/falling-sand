use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockWriteGuard},
};

use itertools::Itertools;
use paste::paste;

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, With},
        system::{Commands, Query, Res, ResMut, Resource, SystemParam},
    },
    math::{IVec2, Vec2},
    utils::{hashbrown::HashMap, HashSet},
};

use crate::{
    chunk::{Chunk, ChunkData},
    material::Material,
    particle_grid::{Particle, ParticleAttributeStore},
    util::positive_mod,
};

pub const CHUNK_SIZE: i32 = 64;

pub const CHUNK_LENGTH: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Component)]
pub struct ChunkActive;

#[derive(Component)]
pub struct ChunkPosition(pub IVec2);

pub fn tile_pos_to_chunk_pos(IVec2 { x, y }: IVec2) -> IVec2 {
    let floor_div = |a: i32, b: i32| {
        if a < 0 && a % b != 0 {
            (a / b) - 1
        } else {
            a / b
        }
    };
    IVec2::new(floor_div(x, CHUNK_SIZE), floor_div(y, CHUNK_SIZE))
}

#[derive(Resource, Default)]
pub struct ChunkPositions(HashMap<IVec2, Entity>);

impl ChunkPositions {
    pub fn get_chunk_at(&self, position: IVec2) -> Option<Entity> {
        self.0.get(&position).copied()
    }

    pub fn contains(&self, pos: &IVec2) -> bool {
        self.0.contains_key(pos)
    }
}

pub fn update_chunk_positions(
    mut chunk_positions: ResMut<ChunkPositions>,
    new_chunks: Query<(Entity, &ChunkPosition), Added<ChunkPosition>>,
) {
    new_chunks.iter().for_each(|(entity, position)| {
        chunk_positions.0.insert(position.0, entity);
    });
}

#[derive(SystemParam)]
pub struct FallingSandGridQuery<'w, 's> {
    commands: Commands<'w, 's>,
    chunks: Query<'w, 's, &'static Chunk>,
    active_chunks: Query<'w, 's, (&'static ChunkActive, &'static ChunkPosition)>,
    chunk_positions: Res<'w, ChunkPositions>,
}

impl<'w, 's> FallingSandGridQuery<'w, 's> {
    pub fn active_chunks(&self) -> HashSet<IVec2> {
        self.active_chunks.iter().map(|(_, pos)| pos.0).collect()
    }

    pub fn get_chunk_entity_at(&self, position: IVec2) -> Option<Entity> {
        self.chunk_positions.get_chunk_at(position)
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE, CHUNK_SIZE)
    }

    fn get_chunk_data(&self, position: IVec2) -> Arc<RwLock<ChunkData>> {
        let chunk_entity = self.get_chunk_entity_at(position).unwrap();
        self.chunks.get(chunk_entity).unwrap().clone().0.clone()
    }

    pub fn get_particle(&self, position: IVec2) -> Particle {
        let chunk_position = tile_pos_to_chunk_pos(position);
        let chunk = self.get_chunk_data(chunk_position);
        let read = chunk.read().unwrap();
        *(read
            .get_particle(IVec2::new(
                positive_mod(position.x, CHUNK_SIZE),
                positive_mod(position.y, CHUNK_SIZE),
            ))
            .unwrap())
    }

    pub fn set_particle(&mut self, position: IVec2, material: Material) {
        let chunk_position = tile_pos_to_chunk_pos(position);
        let chunk = self.get_chunk_data(chunk_position);
        let mut chunk_data = chunk.write().unwrap();
        chunk_data.set_particle_material(
            IVec2::new(
                positive_mod(position.x, CHUNK_SIZE),
                positive_mod(position.y, CHUNK_SIZE),
            ),
            material,
        );
    }

    pub fn get_chunk_active(&mut self, chunk_position: IVec2) -> bool {
        self.active_chunks().contains(&chunk_position)
    }

    pub fn set_chunk_active(&mut self, chunk_position: IVec2, active: bool) {
        let chunk_entity = self.get_chunk_entity_at(chunk_position).unwrap();
        if active {
            self.commands.entity(chunk_entity).insert(ChunkActive);
        } else {
            self.commands.entity(chunk_entity).remove::<ChunkActive>();
        }
    }
}

macro_rules! define_attributes_and_swap {
    ($($attr:ident: $type:ty),* $(,)?) => {
        #[derive(Debug)]
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
            pub fn swap_particles(&mut self, a: IVec2, b: IVec2) {
                let chunk_a_pos = tile_pos_to_chunk_pos(a);
                let chunk_b_pos = tile_pos_to_chunk_pos(b);

                let particle_pos_a = (positive_mod(a.x, CHUNK_SIZE), positive_mod(a.y, CHUNK_SIZE)).into();
                let particle_pos_b = (positive_mod(b.x, CHUNK_SIZE), positive_mod(b.y, CHUNK_SIZE)).into();

                if chunk_a_pos == chunk_b_pos {
                    let chunk = self.get_chunk_data(chunk_a_pos);
                    let chunk_lock = chunk.write();
                    let mut chunk = chunk_lock.unwrap();
                    chunk.swap_particles(
                        particle_pos_a,
                        particle_pos_b
                    );
                } else {
                    let chunk_a = self.get_chunk_data(chunk_a_pos);
                    let chunk_b = self.get_chunk_data(chunk_b_pos);

                    let chunk_a_lock = chunk_a.write();
                    let chunk_b_lock = chunk_b.write();

                    let mut chunk_a = chunk_a_lock.unwrap();
                    let mut chunk_b = chunk_b_lock.unwrap();

                    let particle_a_id = chunk_a.get_particle(particle_pos_a).unwrap().id;
                    let particle_b_id = chunk_b.get_particle(particle_pos_b).unwrap().id;

                    std::mem::swap(
                        &mut chunk_a.get_particle_mut(particle_pos_a).unwrap().material,
                        &mut chunk_b.get_particle_mut(particle_pos_b).unwrap().material
                    );

                    $(
                        std::mem::swap(
                            chunk_a.attributes_mut().$attr.get_mut(particle_a_id).unwrap(),
                            chunk_b.attributes_mut().$attr.get_mut(particle_b_id).unwrap(),
                        );
                    )*

                    chunk_a.attributes_mut().dirty.set(particle_a_id, true);
                    chunk_b.attributes_mut().dirty.set(particle_b_id, true);
                }
            }

            $(
                paste! {
                    pub fn [<get_ $attr>](&self, position: IVec2) -> $type {
                        let chunk_pos = tile_pos_to_chunk_pos(position);
                        let chunk_data = self.get_chunk_data(chunk_pos);
                        let chunk = chunk_data.read().unwrap();
                        let particle = *chunk.get_particle(
                                (
                                    positive_mod(position.x, CHUNK_SIZE),
                                    positive_mod(position.y, CHUNK_SIZE)
                                ).into()
                            ).unwrap();
                        *chunk.attributes().$attr.get(particle.id).unwrap()
                    }

                    pub fn [<set_ $attr>](&mut self, position: IVec2, value: $type) {
                        let chunk_pos = tile_pos_to_chunk_pos(position);
                        let chunk = self.get_chunk_data(chunk_pos);
                        let mut chunk_lock = chunk.write().unwrap();
                        let particle = *chunk_lock.get_particle_mut(
                                (
                                    positive_mod(position.x, CHUNK_SIZE),
                                    positive_mod(position.y, CHUNK_SIZE)
                                ).into()
                            ).unwrap();
                        chunk_lock.attributes_mut().$attr.set(particle.id, value);
                    }
                }
            )*
        }

        impl<'w> ChunkNeighborhoodView<'w> {
            pub fn swap_particles(&mut self, a: IVec2, b: IVec2) {
                let chunk_a_pos = tile_pos_to_chunk_pos(a);
                let chunk_b_pos = tile_pos_to_chunk_pos(b);

                let particle_pos_a = IVec2::new(positive_mod(a.x, CHUNK_SIZE), positive_mod(a.y, CHUNK_SIZE));
                let particle_pos_b = IVec2::new(positive_mod(b.x, CHUNK_SIZE), positive_mod(b.y, CHUNK_SIZE));

                if chunk_a_pos == chunk_b_pos {
                    let chunk = self.get_chunk_at_pos_mut(a).unwrap();
                    chunk.swap_particles(
                        particle_pos_a.into(),
                        particle_pos_b.into()
                    );
                } else {
                    let (chunk_a, chunk_b) = self.get_two_chunks_mut(chunk_a_pos, chunk_b_pos).unwrap();

                    let particle_a_id = chunk_a.get_particle(particle_pos_a).unwrap().id;
                    let particle_b_id = chunk_b.get_particle(particle_pos_b).unwrap().id;

                    std::mem::swap(
                        &mut chunk_a.get_particle_mut(particle_pos_a).unwrap().material,
                        &mut chunk_b.get_particle_mut(particle_pos_b).unwrap().material
                    );

                    $(
                        std::mem::swap(
                            chunk_a.attributes_mut().$attr.get_mut(particle_a_id).unwrap(),
                            chunk_b.attributes_mut().$attr.get_mut(particle_b_id).unwrap(),
                        );
                    )*

                    chunk_a.attributes_mut().dirty.set(particle_a_id, true);
                    chunk_b.attributes_mut().dirty.set(particle_b_id, true);
                }
            }

            $(
                paste! {
                    pub fn [<get_ $attr>](&self, position: IVec2) -> $type {
                        let chunk = self.get_chunk_at_pos(position).unwrap();
                        let particle = chunk.get_particle(
                                (
                                    positive_mod(position.x, CHUNK_SIZE),
                                    positive_mod(position.y, CHUNK_SIZE)
                                ).into()
                            ).unwrap();
                        *chunk.attributes().$attr.get(particle.id).unwrap()
                    }

                    pub fn [<set_ $attr>](&mut self, position: IVec2, value: $type) {
                        let mut chunk = self.get_chunk_at_pos_mut(position).unwrap();
                        let particle = *chunk.get_particle_mut(
                                (
                                    positive_mod(position.x, CHUNK_SIZE),
                                    positive_mod(position.y, CHUNK_SIZE)
                                ).into()
                            ).unwrap();
                        chunk.attributes_mut().$attr.set(particle.id, value);
                    }
                }
            )*
        }
    };
}

define_attributes_and_swap! {
    dirty: bool,
    velocity: IVec2,
}

pub struct ChunkNeighborhoodView<'a> {
    chunk_refs: Vec<(IVec2, RwLockWriteGuard<'a, ChunkData>)>,
}

impl ChunkNeighborhoodView<'_> {
    pub fn new<'a>(
        center_chunk: &'a (IVec2, Arc<RwLock<ChunkData>>),
        neighbors: &'a [(IVec2, Arc<RwLock<ChunkData>>)],
    ) -> ChunkNeighborhoodView<'a> {
        let mut chunk_refs = neighbors
            .iter()
            .map(|(pos, chunk)| (*pos, chunk.write().unwrap()))
            .collect_vec();
        chunk_refs.push((center_chunk.0, center_chunk.1.write().unwrap()));
        ChunkNeighborhoodView { chunk_refs }
    }

    pub fn center_chunk_mut(&mut self) -> &mut ChunkData {
        self.chunk_refs.last_mut().unwrap().1.deref_mut()
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE, CHUNK_SIZE)
    }

    pub fn get_chunk_at_pos(&self, position: IVec2) -> Option<&ChunkData> {
        let chunk_pos = tile_pos_to_chunk_pos(position);
        self.chunk_refs
            .iter()
            .find(|(pos, _)| *pos == chunk_pos)
            .map(|(_, chunk)| chunk.deref())
    }

    pub fn get_chunk_at_pos_mut(&mut self, position: IVec2) -> Option<&mut ChunkData> {
        let chunk_pos = tile_pos_to_chunk_pos(position);
        self.chunk_refs
            .iter_mut()
            .find(|(pos, _)| *pos == chunk_pos)
            .map(|(_, chunk)| chunk.deref_mut())
    }

    pub fn get_two_chunks_mut(
        &mut self,
        pos_a: IVec2,
        pos_b: IVec2,
    ) -> Option<(&mut ChunkData, &mut ChunkData)> {
        if pos_a == pos_b {
            return None; // Early return if positions are the same, as we can't borrow mutably twice.
        }

        let mut first_index = self.chunk_refs.iter().position(|(pos, _)| *pos == pos_a)?;

        let mut second_index = self.chunk_refs.iter().position(|(pos, _)| *pos == pos_b)?;

        let mut flipped = false;

        if first_index > second_index {
            std::mem::swap(&mut first_index, &mut second_index);
            flipped = true;
        }

        let (first_half, second_half) = self.chunk_refs.split_at_mut(second_index);
        let chunk_a = &mut first_half[first_index].1;
        let chunk_b = &mut second_half[0].1;

        if flipped {
            Some((chunk_b, chunk_a))
        } else {
            Some((chunk_a, chunk_b))
        }
    }

    pub fn get_particle(&self, position: IVec2) -> &Particle {
        let chunk = self.get_chunk_at_pos(position).unwrap();
        chunk
            .get_particle(
                (
                    positive_mod(position.x, CHUNK_SIZE),
                    positive_mod(position.y, CHUNK_SIZE),
                )
                    .into(),
            )
            .unwrap()
    }

    pub fn get_particle_mut(&mut self, position: IVec2) -> &mut Particle {
        let mut chunk = self.get_chunk_at_pos_mut(position).unwrap();
        chunk
            .get_particle_mut(
                (
                    positive_mod(position.x, CHUNK_SIZE),
                    positive_mod(position.y, CHUNK_SIZE),
                )
                    .into(),
            )
            .unwrap()
    }

    pub fn set_particle(&mut self, position: IVec2, material: Material) {
        let chunk = &mut self.get_chunk_at_pos_mut(position).unwrap();
        chunk.set_particle_material(
            (
                positive_mod(position.x, CHUNK_SIZE),
                positive_mod(position.y, CHUNK_SIZE),
            )
                .into(),
            material,
        );
    }
}

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn test_tile_pos_to_chunk_pos() {
        assert_eq!(tile_pos_to_chunk_pos((0, 0).into()), IVec2::new(0, 0));
        assert_eq!(tile_pos_to_chunk_pos((63, 63).into()), IVec2::new(0, 0));
        assert_eq!(tile_pos_to_chunk_pos((64, 64).into()), IVec2::new(1, 1));
        assert_eq!(tile_pos_to_chunk_pos((65, 65).into()), IVec2::new(1, 1));
        assert_eq!(tile_pos_to_chunk_pos((0, -1).into()), IVec2::new(0, -1));
    }

    #[test]
    fn test_chunk_neighorhood_view_get_chunk_at_pos() {
        let rng = StdRng::seed_from_u64(0);
        let mut chunk = Chunk::new((CHUNK_SIZE as usize, CHUNK_SIZE as usize), rng);
    }
}
