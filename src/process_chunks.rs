use bevy::{
    ecs::{
        entity::Entity,
        system::{Query, Res, SystemParam},
    },
    log::error,
    math::IVec2,
};
use ndarray::parallel::prelude::{IntoParallelIterator, ParallelIterator};

use crate::{
    chunk::{self, Chunk},
    falling_sand_grid::{
        ChunkActive, ChunkNeighborhoodView, ChunkPosition, ChunkPositions, CHUNK_SIZE,
    },
    sparse_grid_iterator::SparseGridIterator,
};

#[derive(SystemParam)]
pub struct ChunksParam<'w, 's> {
    chunks: Query<'w, 's, &'static Chunk>,
    active_chunks: Query<'w, 's, (&'static ChunkActive, &'static ChunkPosition)>,
    chunk_positions: Res<'w, ChunkPositions>,
}

impl ChunksParam<'_, '_> {
    pub fn active_chunks(&self) -> Vec<IVec2> {
        self.active_chunks.iter().map(|(_, pos)| pos.0).collect()
    }

    pub fn get_chunk_entity_at(&self, x: i32, y: i32) -> Option<Entity> {
        self.chunk_positions.get_chunk_at(x, y)
    }

    pub fn chunk_size(&self) -> IVec2 {
        IVec2::new(CHUNK_SIZE, CHUNK_SIZE)
    }

    pub fn get_chunk(&self, x: i32, y: i32) -> &Chunk {
        let chunk_entity = self.get_chunk_entity_at(x, y).unwrap();
        self.chunks.get(chunk_entity).unwrap()
    }
}

pub fn process_chunks<F>(grid: &mut ChunksParam, operation: F)
where
    F: Fn(IVec2, &mut ChunkNeighborhoodView),
{
    for chunk_pos in grid.active_chunks() {
        let chunk_neighborhood_pos = chunk_pos_with_neighbor_positions(chunk_pos);
        let chunks = chunk_neighborhood_pos.map(|pos| grid.get_chunk(pos.x, pos.y));

        let chunks_pos = chunk_neighborhood_pos
            .iter()
            .zip(chunks.into_iter())
            .map(|(pos, chunk)| (*pos, chunk.0.clone()))
            .collect::<Vec<_>>();

        let ([center_chunk], neighbors) = chunks_pos.split_at(1) else {
            unreachable!("Center chunk not found");
        };

        let mut grid_view = ChunkNeighborhoodView::new(center_chunk, neighbors);

        // Call the operation closure with the current chunk position and neighborhood view.
        operation(chunk_pos, &mut grid_view);
    }
}

fn chunk_pos_with_neighbor_positions(chunk_pos: IVec2) -> [IVec2; 9] {
    [
        chunk_pos,
        IVec2::new(chunk_pos.x - 1, chunk_pos.y - 1),
        IVec2::new(chunk_pos.x, chunk_pos.y - 1),
        IVec2::new(chunk_pos.x + 1, chunk_pos.y - 1),
        IVec2::new(chunk_pos.x - 1, chunk_pos.y),
        IVec2::new(chunk_pos.x + 1, chunk_pos.y),
        IVec2::new(chunk_pos.x - 1, chunk_pos.y + 1),
        IVec2::new(chunk_pos.x, chunk_pos.y + 1),
        IVec2::new(chunk_pos.x + 1, chunk_pos.y + 1),
    ]
}

pub fn process_chunks_parallel<F>(grid: &mut ChunksParam, operation: F)
where
    F: Fn(IVec2, &mut ChunkNeighborhoodView) + Sync,
{
    let sparse_iterator = SparseGridIterator::new(grid.active_chunks());

    sparse_iterator.for_each(|chunk_set| {
        let chunks_with_neighbors = chunk_set
            .iter()
            .map(|center_pos| {
                let neighbors = chunk_neighbors(*center_pos)
                    .iter()
                    .map(|pos| (*pos, grid.get_chunk(pos.x, pos.y).0.clone()))
                    .collect::<Vec<_>>();
                (
                    (
                        *center_pos,
                        grid.get_chunk(center_pos.x, center_pos.y).0.clone(),
                    ),
                    neighbors,
                )
            })
            .collect::<Vec<_>>();

        chunks_with_neighbors
            .into_par_iter()
            .for_each(|(center, neighbors)| {
                let mut grid_view = ChunkNeighborhoodView::new(&center, &neighbors);
                operation(center.0, &mut grid_view);
            });
    });
}

fn chunk_neighbors(chunk_position: IVec2) -> [IVec2; 8] {
    [
        IVec2::new(chunk_position.x - 1, chunk_position.y - 1),
        IVec2::new(chunk_position.x, chunk_position.y - 1),
        IVec2::new(chunk_position.x + 1, chunk_position.y - 1),
        IVec2::new(chunk_position.x - 1, chunk_position.y),
        IVec2::new(chunk_position.x + 1, chunk_position.y),
        IVec2::new(chunk_position.x - 1, chunk_position.y + 1),
        IVec2::new(chunk_position.x, chunk_position.y + 1),
        IVec2::new(chunk_position.x + 1, chunk_position.y + 1),
    ]
}
