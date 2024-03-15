use std::sync::{Arc, RwLock};

use bevy::{
    ecs::{
        entity::Entity,
        system::{Query, Res, SystemParam},
    },
    math::IVec2,
};

use crate::{
    chunk::{Chunk, ChunkData},
    consts::CHUNK_SIZE,
    falling_sand::ChunkPositions,
    material::Material,
    util::{positive_mod, tile_pos_to_chunk_pos},
};

#[derive(SystemParam)]
pub struct FallingSandGridQuery<'w, 's> {
    chunks: Query<'w, 's, &'static Chunk>,
    chunk_positions: Res<'w, ChunkPositions>,
}

impl<'w, 's> FallingSandGridQuery<'w, 's> {
    fn get_chunk_entity_at(&self, position: IVec2) -> Option<Entity> {
        self.chunk_positions.get_at(position).copied()
    }

    fn get_chunk_data(&self, position: IVec2) -> Arc<RwLock<ChunkData>> {
        let chunk_entity = self.get_chunk_entity_at(position).unwrap();
        self.chunks.get(chunk_entity).unwrap().clone().0.clone()
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
}
