use bevy::{
    ecs::{
        entity::Entity,
        system::{Query, Res, SystemParam},
    },
    math::IVec2,
    utils::HashSet,
};
use ndarray::parallel::prelude::{IntoParallelIterator, ParallelIterator};

use crate::{
    chunk::Chunk,
    falling_sand_grid::{
        ActiveChunks, ChunkNeighborhoodView, ChunkPositions, ChunkPositionsData, CHUNK_SIZE,
    },
    sparse_grid_iterator::SparseGridIterator,
    util::chunk_neighbors,
};

const PROCESSING_LIMIT: i32 = 10;

#[derive(SystemParam)]
pub struct ChunksParam<'w> {
    active_chunks: Res<'w, ActiveChunks>,
    chunk_positions: Res<'w, ChunkPositions>,
    chunk_positions_data: Res<'w, ChunkPositionsData>,
}

impl ChunksParam<'_> {
    pub fn active_chunks(&self) -> &HashSet<IVec2> {
        self.active_chunks.hash_set()
    }

    pub fn get_chunk_entity_at(&self, chunk_position: IVec2) -> Option<Entity> {
        self.chunk_positions.get_chunk_at(chunk_position)
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE, CHUNK_SIZE)
    }

    pub fn get_chunk_at(&self, chunk_position: IVec2) -> &Chunk {
        self.chunk_positions_data
            .get_chunk_at(chunk_position)
            .unwrap()
    }

    pub fn get_chunks_at<const N: usize>(&self, chunk_positions: &[IVec2; N]) -> [&Chunk; N] {
        chunk_positions.map(|pos| self.chunk_positions_data.get_chunk_at(pos).unwrap())
    }

    pub fn chunk_exists(&self, position: IVec2) -> bool {
        self.chunk_positions.contains(position)
    }
}

pub fn process_chunks<F>(grid: &mut ChunksParam, operation: F)
where
    F: Fn(IVec2, &mut ChunkNeighborhoodView) + Sync,
{
    let sparse_iterator = SparseGridIterator::new(
        grid.active_chunks()
            .iter()
            .filter(|pos| pos.x.abs() < PROCESSING_LIMIT && pos.y.abs() < PROCESSING_LIMIT)
            .copied()
            .collect(),
    );

    sparse_iterator.for_each(|chunk_set| {
        chunk_set.into_par_iter().for_each(|&center_chunk_pos| {
            let neighbors_positions = chunk_neighbors(center_chunk_pos);

            let neighbor_chunks = neighbors_positions
                .iter()
                .map(|pos| grid.get_chunk_at(*pos));

            let neighbors = neighbors_positions
                .iter()
                .zip(neighbor_chunks)
                .map(|(pos, chunk)| (*pos, chunk));

            let center_chunk = (center_chunk_pos, grid.get_chunk_at(center_chunk_pos));

            let mut grid_view = ChunkNeighborhoodView::new(center_chunk, neighbors);

            operation(center_chunk_pos, &mut grid_view);
        });
    });
}
