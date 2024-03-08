use bevy::{
    asset::Handle,
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::DespawnRecursiveExt,
    render::{
        render_asset::RenderAssets,
        render_resource::{
            Extent3d, ImageDataLayout, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages, TextureView,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, Image, TextureCache, TextureFormatPixelInfo},
        Extract,
    },
};

use bytemuck::cast_slice;
use itertools::Itertools;

use crate::{chunk::Chunk, consts::CHUNK_SIZE};

#[derive(Component)]
pub struct ExtractedChunkUpdate {
    pub materials_texture: CachedTexture,
    pub color_texture: TextureView,
}

pub fn extract(
    mut commands: Commands,
    chunk_query: Extract<Query<(&Chunk, &Handle<Image>)>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut texture_cache: ResMut<TextureCache>,
    extracted_chunks_query: Query<Entity, With<ExtractedChunkUpdate>>,
    images: Res<RenderAssets<Image>>,
) {
    for entity in extracted_chunks_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    let extracted_chunks = chunk_query
        .iter()
        .flat_map(|(chunk, chunk_image)| {
            if !chunk.read().unwrap().is_dirty() {
                return None;
            }

            let chunk_data = &chunk.read().unwrap();

            let descriptor = TextureDescriptor {
                label: Some("chunk_update_texture"),
                size: Extent3d {
                    width: CHUNK_SIZE as u32,
                    height: CHUNK_SIZE as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Uint,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[TextureFormat::R32Uint],
            };
            let format_size = descriptor.format.pixel_size();
            let material_grid_texture = texture_cache.get(&render_device, descriptor);

            render_queue.write_texture(
                material_grid_texture.texture.as_image_copy(),
                cast_slice(
                    chunk_data
                        .particles()
                        .array()
                        .as_slice()
                        .expect("Failed to get chunk as slice"),
                ),
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(CHUNK_SIZE as u32 * format_size as u32),
                    rows_per_image: None,
                },
                Extent3d {
                    width: CHUNK_SIZE as u32,
                    height: CHUNK_SIZE as u32,
                    depth_or_array_layers: 1,
                },
            );

            let color_texture_image = images.get(chunk_image).unwrap();
            let color_texture_view = color_texture_image.texture_view.clone();

            Some(ExtractedChunkUpdate {
                materials_texture: material_grid_texture,
                color_texture: color_texture_view,
            })
        })
        .collect_vec();

    commands.spawn_batch(extracted_chunks);
}
