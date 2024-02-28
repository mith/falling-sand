use ndarray::Array2;

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        query::Added,
        system::{Query, ResMut, Resource},
    },
    math::IVec2,
};

use crate::chunk::Chunk;

#[derive(Component)]
pub struct ChunkPosition(pub IVec2);

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

#[cfg(test)]
mod test {
    use bevy::app::{App, Update};

    use super::*;
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
}
