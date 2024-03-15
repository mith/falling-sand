use bevy::{
    ecs::{
        entity::Entity,
        system::{Res, SystemParam},
    },
    log::info_span,
    math::IVec2,
};

#[cfg(feature = "parallel")]
use ndarray::parallel::prelude::{IntoParallelIterator, ParallelIterator};

use ndarray::Array2;

use crate::{
    active_chunks::ActiveChunks,
    chunk::{Chunk, ChunkData},
    chunk_neighborhood_view::ChunkNeighborhoodView,
    falling_sand::{ChunkDataPositions, ChunkPositions},
};

pub const PROCESSING_LIMIT: i32 = 100;

#[derive(SystemParam)]
pub struct ChunksParam<'w> {
    active_chunks: Res<'w, ActiveChunks>,
    chunk_positions_data: Res<'w, ChunkDataPositions>,
}

impl ChunksParam<'_> {
    pub fn active_chunks(&self) -> &ActiveChunks {
        &self.active_chunks
    }

    pub fn get_chunk_at(&self, chunk_position: IVec2) -> &Chunk {
        self.chunk_positions_data.get_at(chunk_position).unwrap()
    }

    pub fn get_neighborhood(&self, chunk_position: IVec2) -> Array2<&Chunk> {
        let neighborhood = Array2::from_shape_fn((3, 3), |(y, x)| {
            let pos = IVec2::new(x as i32 - 1, y as i32 - 1) + chunk_position;
            self.get_chunk_at(pos)
        });
        neighborhood
    }
}

pub fn process_chunks_neighborhood<F>(grid: &ChunksParam, operation: F)
where
    F: Fn(IVec2, &mut ChunkNeighborhoodView) + Sync,
{
    let span = info_span!("process_chunks");
    let _guard = span.enter();
    grid.active_chunks().passes().iter().for_each(|chunk_set| {
        let span = info_span!("process_chunks_pass");
        let _guard = span.enter();

        #[cfg(feature = "parallel")]
        let iter = chunk_set.into_par_iter();
        #[cfg(not(feature = "parallel"))]
        let iter = chunk_set.iter();

        iter.for_each(|&center_chunk_pos| {
            let span = info_span!("process_chunks_task");
            let _guard = span.enter();
            let neighborhood = grid.get_neighborhood(center_chunk_pos);

            let mut grid_view = ChunkNeighborhoodView::new(neighborhood.as_slice().unwrap());

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

    #[cfg(feature = "parallel")]
    let iter = active_chunks.into_par_iter();
    #[cfg(not(feature = "parallel"))]
    let iter = active_chunks.iter().copied();

    iter.for_each(|&chunk_position| {
        let span = info_span!("process_chunks_dense_task");
        let _guard = span.enter();
        let chunk = grid.get_chunk_at(chunk_position);
        let mut chunk_data = chunk.write().unwrap();
        operation(chunk_position, &mut chunk_data);
    });
}
