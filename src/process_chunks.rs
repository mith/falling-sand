use bevy::{
    ecs::{
        entity::Entity,
        system::{Query, Res, SystemParam},
    },
    log::error,
    math::IVec2,
    utils::HashSet,
};
use itertools::Itertools;
use ndarray::parallel::prelude::{IntoParallelIterator, ParallelIterator};
use smallvec::SmallVec;

use crate::{
    chunk::{self, Chunk},
    falling_sand_grid::{
        ActiveChunks, ChunkActive, ChunkNeighborhoodView, ChunkPosition, ChunkPositions, CHUNK_SIZE,
    },
    sparse_grid_iterator::SparseGridIterator,
    util::{chunk_neighbors, chunk_pos_with_neighbor_positions},
};

const PROCESSING_LIMIT: i32 = 10;

#[derive(SystemParam)]
pub struct ChunksParam<'w, 's> {
    chunks: Query<'w, 's, &'static Chunk>,
    active_chunks: Res<'w, ActiveChunks>,
    chunk_positions: Res<'w, ChunkPositions>,
}

impl ChunksParam<'_, '_> {
    pub fn active_chunks(&self) -> &HashSet<IVec2> {
        self.active_chunks.hash_set()
    }

    pub fn get_chunk_entity_at(&self, chunk_position: IVec2) -> Option<&Entity> {
        self.chunk_positions.get_chunk_at(chunk_position)
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE, CHUNK_SIZE)
    }

    pub fn get_chunk_at(&self, chunk_position: IVec2) -> &Chunk {
        let chunk_entity = *self.get_chunk_entity_at(chunk_position).unwrap();
        self.chunks.get(chunk_entity).unwrap()
    }

    pub fn get_chunks_at<const N: usize>(&self, chunk_positions: &[IVec2; N]) -> [&Chunk; N] {
        let chunk_entities = chunk_positions.map(|pos| *self.get_chunk_entity_at(pos).unwrap());
        self.chunks.get_many(chunk_entities).unwrap()
    }

    pub fn chunk_exists(&self, position: IVec2) -> bool {
        self.chunk_positions.contains(position)
    }
}

pub fn process_chunks<F>(grid: &mut ChunksParam, operation: F)
where
    F: Fn(IVec2, &mut ChunkNeighborhoodView),
{
    for &chunk_pos in grid
        .active_chunks()
        .iter()
        .filter(|pos| pos.x.abs() < PROCESSING_LIMIT && pos.y.abs() < PROCESSING_LIMIT)
    {
        let chunk_neighborhood_pos = chunk_pos_with_neighbor_positions(chunk_pos);
        let chunks = chunk_neighborhood_pos.map(|pos| grid.get_chunk_at(pos));

        let chunks_pos = chunk_neighborhood_pos
            .iter()
            .zip(chunks.into_iter())
            .map(|(pos, chunk)| (*pos, &chunk.0))
            .collect::<Vec<_>>();

        let ([center_chunk], neighbors) = chunks_pos.split_at(1) else {
            unreachable!("Center chunk not found");
        };

        let mut grid_view = ChunkNeighborhoodView::new(center_chunk, neighbors);

        // Call the operation closure with the current chunk position and neighborhood view.
        operation(chunk_pos, &mut grid_view);
    }
}

pub fn process_chunks_parallel<F>(grid: &mut ChunksParam, operation: F)
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
        let chunks_with_neighbors: SmallVec<[_; 9]> = chunk_set
            .iter()
            .map(|center_chunk_pos| {
                let neighbors = chunk_neighbors(*center_chunk_pos);

                let center_pos_plus_neighbors = {
                    let mut arr = [IVec2::ZERO; 9];
                    arr[0] = *center_chunk_pos;
                    arr[1..].copy_from_slice(&neighbors);
                    arr
                };

                let chunks = grid.get_chunks_at(&center_pos_plus_neighbors);

                let chunks_with_pos: SmallVec<[(IVec2, _); 9]> = center_pos_plus_neighbors
                    .iter()
                    .zip(chunks)
                    .map(|(pos, chunk)| (*pos, &chunk.0))
                    .collect();

                (chunks_with_pos[0], chunks_with_pos[1..].to_vec())
            })
            .collect();

        chunks_with_neighbors
            .into_par_iter()
            .for_each(|(center, neighbors)| {
                let mut grid_view = ChunkNeighborhoodView::new(center, neighbors);
                operation(center.0, &mut grid_view);
            });
    });
}
