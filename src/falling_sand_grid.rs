use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockWriteGuard},
    thread::panicking,
};

use ndarray::{s, Array2, ArrayView2, ArrayViewMut2, AssignElem, MathCell};
use paste::paste;

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, With},
        system::{Commands, Query, Res, ResMut, Resource, SystemParam},
    },
    math::IVec2,
    utils::{HashMap, HashSet},
};
use smallvec::SmallVec;

use crate::{
    chunk::{Chunk, ChunkData},
    material::Material,
    particle_grid::{Particle, ParticleAttributeStore},
    process_chunks::PROCESSING_LIMIT,
    util::positive_mod,
};

const SHIFT: i32 = 6; // log2(CHUNK_SIZE)
pub const CHUNK_SIZE: i32 = 1 << SHIFT;

#[derive(Component)]
#[component(storage = "SparseSet")]
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

fn chunk_pos_set_index(pos: &IVec2) -> i32 {
    let positive_mod = |n: i32, m: i32| ((n % m) + m) % m;
    let x = positive_mod(pos.x, 3);
    let y = positive_mod(pos.y, 3);
    x + y * 3
}
#[derive(Resource)]
pub struct ChunkPositions {
    positions: Array2<Option<Entity>>,
    offset: IVec2,
}

impl Default for ChunkPositions {
    fn default() -> Self {
        ChunkPositions {
            positions: Array2::default((0, 0)),
            offset: IVec2::ZERO,
        }
    }
}

impl ChunkPositions {
    pub fn get_chunk_at(&self, position: IVec2) -> Option<Entity> {
        *self
            .positions
            .get((
                (position.x + self.offset.x) as usize,
                (position.y + self.offset.y) as usize,
            ))
            .unwrap_or(&None)
    }

    pub fn contains(&self, position: IVec2) -> bool {
        self.get_chunk_at(position).is_some()
    }
}

pub fn update_chunk_positions(
    mut chunk_positions: ResMut<ChunkPositions>,
    new_chunks: Query<(Entity, &ChunkPosition), Added<ChunkPosition>>,
    chunk_positions_query: Query<(Entity, &ChunkPosition)>,
) {
    if new_chunks.is_empty() {
        return;
    }

    // Get the max bounds of the chunks to determine the depth of the quadtree
    let (min_bounds, max_bounds) =
        chunk_positions_query
            .iter()
            .fold((IVec2::MAX, IVec2::MIN), |(min, max), (_, pos)| {
                (
                    IVec2::new(min.x.min(pos.0.x), min.y.min(pos.0.y)),
                    IVec2::new(max.x.max(pos.0.x), max.y.max(pos.0.y)),
                )
            });

    let offset = (min_bounds.x.min(0).abs(), min_bounds.y.min(0).abs()).into();
    chunk_positions.offset = offset;

    let size = max_bounds + offset + IVec2::ONE;

    chunk_positions.positions = Array2::default((size.x as usize, size.y as usize));

    for (entity, pos) in chunk_positions_query.iter() {
        let pos = pos.0 + offset;
        chunk_positions.positions[(pos.x as usize, pos.y as usize)] = Some(entity);
    }
}

#[derive(Resource)]
pub struct ChunkPositionsData {
    positions: Array2<Option<Chunk>>,
    offset: IVec2,
}

impl Default for ChunkPositionsData {
    fn default() -> Self {
        ChunkPositionsData {
            positions: Array2::default((0, 0)),
            offset: IVec2::ZERO,
        }
    }
}

impl ChunkPositionsData {
    pub fn get_chunk_at(&self, position: IVec2) -> Option<&Chunk> {
        let Some(chunk) = self
            .positions
            .get((
                (position.x + self.offset.x) as usize,
                (position.y + self.offset.y) as usize,
            ))
            .unwrap_or(&None)
        else {
            return None;
        };
        Some(chunk)
    }

    pub fn contains(&self, position: IVec2) -> bool {
        self.get_chunk_at(position).is_some()
    }
}

pub fn update_chunk_positions_data(
    mut chunk_positions: ResMut<ChunkPositionsData>,
    new_chunks: Query<(Entity, &ChunkPosition), Added<ChunkPosition>>,
    chunk_positions_query: Query<(&Chunk, &ChunkPosition)>,
) {
    if new_chunks.is_empty() {
        return;
    }

    // Get the max bounds of the chunks to determine the depth of the quadtree
    let (min_bounds, max_bounds) =
        chunk_positions_query
            .iter()
            .fold((IVec2::MAX, IVec2::MIN), |(min, max), (_, pos)| {
                (
                    IVec2::new(min.x.min(pos.0.x), min.y.min(pos.0.y)),
                    IVec2::new(max.x.max(pos.0.x), max.y.max(pos.0.y)),
                )
            });

    let offset = (min_bounds.x.min(0).abs(), min_bounds.y.min(0).abs()).into();
    chunk_positions.offset = offset;

    let size = max_bounds + offset + IVec2::ONE;

    chunk_positions.positions = Array2::default((size.x as usize, size.y as usize));

    for (chunk, pos) in chunk_positions_query.iter() {
        let pos = pos.0 + offset;
        chunk_positions.positions[(pos.x as usize, pos.y as usize)] = Some(chunk.clone());
    }
}

#[derive(Resource, Default)]
pub struct ActiveChunks {
    chunks: HashMap<IVec2, u8>,
    passes: [SmallVec<[IVec2; 8]>; 9],
}

impl ActiveChunks {
    pub fn contains(&self, pos: &IVec2) -> bool {
        self.chunks.contains_key(pos)
    }

    pub fn hash_map(&self) -> &HashMap<IVec2, u8> {
        &self.chunks
    }

    pub fn passes(&self) -> &[SmallVec<[IVec2; 8]>; 9] {
        &self.passes
    }

    pub fn iter(&self) -> impl Iterator<Item = &IVec2> {
        self.chunks.keys()
    }
}

pub fn update_active_chunks(
    mut active_chunks: ResMut<ActiveChunks>,
    active_chunks_query: Query<(&ChunkActive, &ChunkPosition)>,
) {
    active_chunks.chunks.clear();
    active_chunks.chunks.extend(
        active_chunks_query
            .iter()
            .map(|(_, pos)| (pos.0, chunk_pos_set_index(&pos.0) as u8)),
    );

    let ActiveChunks {
        ref mut passes,
        ref chunks,
    } = *active_chunks;
    for pass in passes.iter_mut() {
        pass.clear();
    }
    for (chunk_pos, &set_index) in chunks.iter() {
        if chunk_pos.x.abs() > PROCESSING_LIMIT || chunk_pos.y.abs() > PROCESSING_LIMIT {
            continue;
        }
        passes[set_index as usize].push(*chunk_pos);
    }
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

        impl<'a> ChunkNeighborhoodView<'a> {
            pub fn swap_particles(&mut self, a: IVec2, b: IVec2) {
                let chunk_a_pos = neighborhood_pos_to_chunk_pos(a);
                let chunk_b_pos = neighborhood_pos_to_chunk_pos(b);

                let particle_pos_a = neighborhood_pos_to_local_pos(a, chunk_a_pos);
                let particle_pos_b = neighborhood_pos_to_local_pos(b, chunk_b_pos);

                if chunk_a_pos == chunk_b_pos {
                    let chunk = self.get_chunk_at_chunk_pos_mut(chunk_a_pos).unwrap();
                    chunk.swap_particles(
                        particle_pos_a.into(),
                        particle_pos_b.into()
                    );
                } else {
                    let (chunk_a, chunk_b) = self.get_two_chunks_mut(chunk_a_pos, chunk_b_pos);

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
                        let (chunk_pos, chunk) = self.get_chunk_at_neighborhood_pos(position).unwrap();
                        let local_pos = neighborhood_pos_to_local_pos(position, chunk_pos);
                        let particle = chunk.get_particle(local_pos).unwrap();
                        *chunk.attributes().$attr.get(particle.id).unwrap()
                    }

                    pub fn [<set_ $attr>](&mut self, position: IVec2, value: $type) {
                        let (chunk_pos, chunk) = self.get_chunk_at_neighborhood_pos_mut(position).unwrap();
                        let local_pos = neighborhood_pos_to_local_pos(position, chunk_pos);
                        let particle = *chunk.get_particle_mut(local_pos).unwrap();
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

            for (i, chunk) in chunks.iter().enumerate() {
                let chunk = chunk.write().unwrap();
                chunks_uninit[i] = MaybeUninit::new(chunk);
            }

            unsafe { chunks_uninit.map(|p| p.assume_init()) }
        };

        ChunkNeighborhoodView { chunks }
    }

    fn center_chunk(&self) -> &ChunkData {
        self.chunks[4].deref()
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
        if chunk_pos_a == chunk_pos_b {
            panic!("Chunks are the same");
        }

        let mut first_index = chunk_pos_to_index(chunk_pos_a);
        let mut second_index = chunk_pos_to_index(chunk_pos_b);
        let mut flipped = false;

        if first_index > second_index {
            std::mem::swap(&mut first_index, &mut second_index);
            flipped = true;
        }

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

    pub fn get_particle_mut(&mut self, position: IVec2) -> &mut Particle {
        let (chunk_pos, chunk) = self.get_chunk_at_neighborhood_pos_mut(position).unwrap();
        let local_pos = neighborhood_pos_to_local_pos(position, chunk_pos);
        chunk.get_particle_mut(local_pos).unwrap()
    }

    pub fn set_particle(&mut self, position: IVec2, material: Material) {
        let (chunk_pos, chunk) = self.get_chunk_at_neighborhood_pos_mut(position).unwrap();
        let local_pos = neighborhood_pos_to_local_pos(position, chunk_pos);
        chunk.set_particle_material(local_pos, material);
    }
}

fn neighborhood_pos_to_chunk_pos(position: IVec2) -> IVec2 {
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
    use bevy::app::{App, Update};

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
    fn test_update_chunk_positions() {
        let mut app = App::new();

        app.add_systems(Update, update_chunk_positions);

        app.world.init_resource::<ChunkPositions>();
        let chunk_0_0 = app.world.spawn(ChunkPosition(IVec2::new(0, 0))).id();
        let chunk_1_0 = app.world.spawn(ChunkPosition(IVec2::new(1, 0))).id();
        let chunk_neg_1_0 = app.world.spawn(ChunkPosition(IVec2::new(-1, 0))).id();

        let chunk_10_10 = app.world.spawn(ChunkPosition(IVec2::new(10, 10))).id();
        let chunk_neg_10_10 = app.world.spawn(ChunkPosition(IVec2::new(-10, -10))).id();

        let chunk_100_100 = app.world.spawn(ChunkPosition(IVec2::new(100, 100))).id();
        let chunk_neg_100_100 = app.world.spawn(ChunkPosition(IVec2::new(-100, -100))).id();

        app.update();

        let chunk_positions = app.world.get_resource::<ChunkPositions>().unwrap();
        assert_eq!(
            chunk_positions.get_chunk_at(IVec2::new(0, 0)),
            Some(chunk_0_0),
            "Chunk 0, 0 not found"
        );
        assert_eq!(
            chunk_positions.get_chunk_at(IVec2::new(1, 0)),
            Some(chunk_1_0),
            "Chunk 1, 0 not found"
        );
        assert_eq!(
            chunk_positions.get_chunk_at(IVec2::new(-1, 0)),
            Some(chunk_neg_1_0),
            "Chunk -1, 0 not found"
        );

        assert_eq!(
            chunk_positions.get_chunk_at(IVec2::new(10, 10)),
            Some(chunk_10_10),
            "Chunk 10, 10 not found"
        );
        assert_eq!(
            chunk_positions.get_chunk_at(IVec2::new(-10, -10)),
            Some(chunk_neg_10_10),
            "Chunk -10, -10 not found"
        );

        assert_eq!(
            chunk_positions.get_chunk_at(IVec2::new(100, 100)),
            Some(chunk_100_100),
            "Chunk 100, 100 not found"
        );
        assert_eq!(
            chunk_positions.get_chunk_at(IVec2::new(-100, -100)),
            Some(chunk_neg_100_100),
            "Chunk -100, -100 not found"
        );
    }

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
