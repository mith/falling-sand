use bevy::{
    ecs::{
        entity::Entity,
        system::{Res, SystemParam},
    },
    log::info_span,
    math::IVec2,
    utils::{tracing::span, HashMap},
};
use ndarray::{
    parallel::prelude::{IntoParallelIterator, ParallelIterator},
    Array2,
};
use smallvec::SmallVec;

use crate::{
    chunk::{Chunk, ChunkData},
    falling_sand_grid::{
        ActiveChunks, ChunkNeighborhoodView, ChunkPositions, ChunkPositionsData, CHUNK_SIZE,
    },
    util::chunk_neighbors,
};

pub const PROCESSING_LIMIT: i32 = 100;

#[derive(SystemParam)]
pub struct ChunksParam<'w> {
    active_chunks: Res<'w, ActiveChunks>,
    chunk_positions: Res<'w, ChunkPositions>,
    chunk_positions_data: Res<'w, ChunkPositionsData>,
}

impl ChunksParam<'_> {
    pub fn active_chunks(&self) -> &ActiveChunks {
        &self.active_chunks
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

    pub fn get_neighborhood(&self, chunk_position: IVec2) -> Array2<&Chunk> {
        let neighborhood = Array2::from_shape_fn((3, 3), |(y, x)| {
            let pos = IVec2::new(x as i32 - 1, y as i32 - 1) + chunk_position;
            self.get_chunk_at(pos)
        });
        neighborhood
    }

    pub fn chunk_exists(&self, position: IVec2) -> bool {
        self.chunk_positions.contains(position)
    }
}

pub fn process_chunks<F>(grid: &ChunksParam, operation: F)
where
    F: Fn(IVec2, &mut ChunkNeighborhoodView) + Sync,
{
    let span = info_span!("process_chunks");
    let _guard = span.enter();
    grid.active_chunks().passes().iter().for_each(|chunk_set| {
        let span = info_span!("process_chunks_pass");
        let _guard = span.enter();
        chunk_set.into_par_iter().for_each(|&center_chunk_pos| {
            let span = info_span!("process_chunks_task");
            let _guard = span.enter();
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

pub fn process_chunks_dense<F>(grid: &ChunksParam, operation: F)
where
    F: Fn(IVec2, &mut ChunkData) + Sync,
{
    let span = info_span!("process_chunks_dense");
    let _guard = span.enter();
    let active_chunks = grid.active_chunks().iter().collect::<Vec<_>>();
    active_chunks.into_par_iter().for_each(|&chunk_position| {
        let span = info_span!("process_chunks_dense_task");
        let _guard = span.enter();
        let chunk = grid.get_chunk_at(chunk_position);
        let mut chunk_data = chunk.write().unwrap();
        operation(chunk_position, &mut chunk_data);
    });
}
