use bevy::{
    ecs::{
        component::Component,
        system::{Query, ResMut, Resource},
    },
    math::IVec2,
    utils::HashMap,
};
use rand::seq::SliceRandom;
use smallvec::SmallVec;

use crate::{
    falling_sand::{ChunkPosition, FallingSandRng},
    process_chunks::PROCESSING_LIMIT,
};

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ChunkActive;

#[derive(Resource, Default)]
pub struct ActiveChunks {
    chunks: HashMap<IVec2, u8>,
    passes: [SmallVec<[IVec2; 8]>; 9],
}

impl ActiveChunks {
    pub fn passes(&self) -> &[SmallVec<[IVec2; 8]>; 9] {
        &self.passes
    }

    pub fn iter(&self) -> impl Iterator<Item = &IVec2> {
        self.chunks.keys()
    }
}

fn chunk_pos_pass_index(pos: &IVec2) -> i32 {
    let positive_mod = |n: i32, m: i32| ((n % m) + m) % m;
    let x = positive_mod(pos.x, 3);
    let y = positive_mod(pos.y, 3);
    x + y * 3
}

pub fn gather_active_chunks(
    mut active_chunks: ResMut<ActiveChunks>,
    active_chunks_query: Query<(&ChunkActive, &ChunkPosition)>,
    mut rng: ResMut<FallingSandRng>,
) {
    let ActiveChunks {
        ref mut passes,
        ref mut chunks,
    } = *active_chunks;

    chunks.clear();
    chunks.extend(
        active_chunks_query
            .iter()
            .map(|(_, pos)| (pos.0, chunk_pos_pass_index(&pos.0) as u8)),
    );

    for pass in passes.iter_mut() {
        pass.clear();
    }
    for (chunk_pos, &set_index) in chunks.iter() {
        if chunk_pos.x.abs() > PROCESSING_LIMIT || chunk_pos.y.abs() > PROCESSING_LIMIT {
            continue;
        }
        passes[set_index as usize].push(*chunk_pos);
    }

    // Shuffle the passes around
    active_chunks.passes.shuffle(&mut rng.0);
}
