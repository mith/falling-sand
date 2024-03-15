use std::time::Duration;

use bevy::ecs::system::SystemParam;

use bevy::{prelude::*, render::render_asset::RenderAssetUsages, utils::HashSet};

use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use bytemuck::cast_slice;
use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};

use crate::spatial_store::SpatialStore;
use crate::{
    active_chunks::{gather_active_chunks, ActiveChunks, ChunkActive},
    chunk::{Chunk, ChunkData},
    consts::CHUNK_SIZE,
    fall::fall,
    fire::fire_to_smoke,
    flow::flow,
    material::{Material, MaterialColor, MaterialPlugin},
    process_chunks::ChunksParam,
    reactions::react,
    render::{FallingSandImages, FallingSandRenderPlugin},
    util::{chunk_neighbors, chunk_neighbors_n},
};

#[derive(Default)]
pub struct FallingSandPlugin {
    pub settings: FallingSandSettings,
}

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FallingSandSet;

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FallingSandCleanSet;

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
struct FallingSandPreSet;

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
struct FallingSandPhysicsSet;

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
struct FallingSandPostSet;

#[derive(Resource)]
pub struct FallingSandRng(pub StdRng);

impl Plugin for FallingSandPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<FallingSandImages>::default(),
            ExtractResourcePlugin::<FallingSandSettings>::default(),
            MaterialPlugin,
            FallingSandRenderPlugin,
        ))
        .register_type::<DirtyChunks>()
        .insert_resource(Time::<Virtual>::from_max_delta(Duration::from_secs_f32(
            1. / 64.,
        )))
        .insert_resource(self.settings.clone())
        .insert_resource(FallingSandRng(StdRng::seed_from_u64(0)))
        .init_resource::<ChunkPositions>()
        .init_resource::<ChunkDataPositions>()
        .init_resource::<ActiveChunks>()
        .init_resource::<FallingSandImages>()
        .init_resource::<ChunkDebug>()
        .add_systems(Startup, setup.before(FallingSandPreSet))
        .add_systems(
            FixedPreUpdate,
            (
                (
                    activate_or_deactivate_chunks,
                    apply_deferred,
                    (clean_chunks, spawn_chunks_around_active),
                )
                    .chain(),
                (gather_active_chunks,),
            )
                .in_set(FallingSandSet),
        )
        .add_systems(
            FixedUpdate,
            (
                fall,
                clean_particles,
                flow,
                clean_particles,
                react,
                fire_to_smoke,
            )
                .chain()
                .in_set(FallingSandSet)
                .in_set(FallingSandPhysicsSet),
        )
        .add_systems(
            Update,
            (
                toggle_chunk_debug,
                draw_chunk_debug_gizmos.run_if(chunk_debug_enabled),
            ),
        );
    }
}

fn clean_particles(chunk_query: Query<&Chunk>) {
    chunk_query.par_iter().for_each(|grid| {
        let grid = &mut grid.write().unwrap();
        if !grid.is_dirty() {
            return;
        }
        clean_particles_chunk(grid);
    });
}

fn clean_particles_chunk(grid: &mut ChunkData) {
    grid.particles_mut()
        .array_mut()
        .iter_mut()
        .for_each(|particle| {
            particle.set_dirty(false);
        });
}

fn clean_chunks(chunk_query: Query<&Chunk>) {
    chunk_query.par_iter().for_each(|chunk| {
        let chunk_data = &mut chunk.write().unwrap();
        if !chunk_data.is_dirty() {
            return;
        }

        clean_particles_chunk(chunk_data);
        chunk_data.set_dirty(false);
    });
}

fn activate_or_deactivate_chunks(mut commands: Commands, chunks_query: Query<(Entity, &Chunk)>) {
    for (entity, chunk) in chunks_query.iter() {
        if chunk.read().unwrap().is_dirty() {
            commands.entity(entity).insert(ChunkActive);
        } else {
            commands.entity(entity).remove::<ChunkActive>();
        }
    }
}

fn spawn_chunks_around_active(
    mut commands: Commands,
    mut chunk_creation_params: ChunkCreationParams,
    active_chunks_query: Query<&ChunkPosition, With<ChunkActive>>,
) {
    for position in &active_chunks_query {
        let chunk_neighbors_2 = chunk_neighbors_n(position.0, 2);
        let unspawned_neighbors = chunk_neighbors_2
            .iter()
            .filter(|&neighbor| !chunk_creation_params.chunk_positions.contains(*neighbor))
            .copied()
            .collect_vec();

        chunk_creation_params.spawn_chunks(unspawned_neighbors);

        for neighbor in chunk_neighbors(position.0)
            .iter()
            .filter_map(|&pos| chunk_creation_params.chunk_positions.get_at(pos))
        {
            commands.entity(*neighbor).insert(ChunkActive);
        }
    }
}

#[derive(Component, Reflect)]
pub struct ChunkParticleGridImage {
    pub materials_texture: Handle<Image>,
}

#[derive(Resource, Clone, ExtractResource, Reflect)]
pub struct FallingSandSettings {
    pub size: (usize, usize),
    pub tile_size: u32,
}

impl Default for FallingSandSettings {
    fn default() -> Self {
        FallingSandSettings {
            size: (CHUNK_SIZE as usize, CHUNK_SIZE as usize),
            tile_size: 1,
        }
    }
}

#[derive(Resource, Clone, Default, Reflect)]
struct DirtyChunks(HashSet<IVec2>);

#[derive(Resource, Default)]
struct ChunkDebug(bool);

fn chunk_debug_enabled(terrain_debug: Res<ChunkDebug>) -> bool {
    terrain_debug.0
}

const TERRAIN_DEBUG_TOGGLE_KEY: KeyCode = KeyCode::F3;

fn toggle_chunk_debug(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut terrain_debug: ResMut<ChunkDebug>,
) {
    if keyboard_input.just_pressed(TERRAIN_DEBUG_TOGGLE_KEY) {
        terrain_debug.0 = !terrain_debug.0;
    }
}

fn draw_chunk_debug_gizmos(
    mut gizmos: Gizmos,
    falling_sand_settings: Res<FallingSandSettings>,
    chunk_positions: Query<(&ChunkPosition, Option<&ChunkActive>)>,
) {
    for (position, chunk) in &chunk_positions {
        let position =
            position.0.as_vec2() * CHUNK_SIZE as f32 * falling_sand_settings.tile_size as f32;
        gizmos.rect_2d(
            position,
            0.,
            Vec2::new(
                falling_sand_settings.size.0 as f32,
                falling_sand_settings.size.1 as f32,
            ),
            if chunk.is_some() {
                Color::RED
            } else {
                Color::GREEN
            },
        );
    }
}

#[derive(Component)]
pub struct ChunkPosition(pub IVec2);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct ChunkPositions(SpatialStore<Entity>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct ChunkDataPositions(SpatialStore<Chunk>);

#[derive(SystemParam)]
pub struct ChunkCreationParams<'w, 's> {
    commands: Commands<'w, 's>,
    images: ResMut<'w, Assets<Image>>,
    falling_sand_settings: Res<'w, FallingSandSettings>,
    material_colors: Res<'w, MaterialColor>,
    pub chunk_positions: ResMut<'w, ChunkPositions>,
    pub chunk_data_positions: ResMut<'w, ChunkDataPositions>,
}

impl<'w, 's> ChunkCreationParams<'w, 's> {
    pub fn spawn_chunks(&mut self, positions: impl IntoIterator<Item = IVec2>) {
        let initial_material = Material::Air;
        positions.into_iter().for_each(|position| {
            let chunk_bundle = {
                let images: &mut Assets<Image> = &mut self.images;
                let falling_sand_settings: &FallingSandSettings = &self.falling_sand_settings;
                let material_colors: &MaterialColor = &self.material_colors;
                let IVec2 { x, y } = position;
                let size = (
                    falling_sand_settings.size.0 as u32,
                    falling_sand_settings.size.1 as u32,
                );
                let scale = falling_sand_settings.tile_size;

                let seed = 0u64
                    .wrapping_add(x as u64)
                    .wrapping_mul(31)
                    .wrapping_add(y as u64);
                let rng = StdRng::seed_from_u64(seed);
                let material = Material::Air;

                let chunk =
                    Chunk::new_with_material((size.0 as usize, size.1 as usize), material, rng);
                self.chunk_data_positions.add(position, chunk.clone());

                let initial_color = material_colors[initial_material];

                let (grid_texture, color_image) =
                    create_chunk_images(size, &chunk.read().unwrap(), images, initial_color);

                (
                    Name::new("Chunk"),
                    SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(
                                (size.0 * scale) as f32,
                                (size.1 * scale) as f32,
                            )),
                            ..default()
                        },
                        texture: color_image,
                        transform: Transform::from_rotation(Quat::from_rotation_z(
                            std::f32::consts::PI / 2.0,
                        ))
                        .with_translation(Vec3::new(
                            (x * size.0 as i32 * scale as i32) as f32,
                            (y * size.1 as i32 * scale as i32) as f32,
                            0.0,
                        )),
                        ..default()
                    },
                    ChunkParticleGridImage {
                        materials_texture: grid_texture,
                    },
                    chunk,
                    ChunkPosition(IVec2::new(x, y)),
                )
            };

            let chunk_entity = self.commands.spawn(chunk_bundle).id();
            self.chunk_positions.add(position, chunk_entity);
        });
    }
}

fn setup(
    mut chunk_creation_params: ChunkCreationParams,
    material_colors: Res<MaterialColor>,
    mut falling_sand_images: ResMut<FallingSandImages>,
) {
    let color_map_image = create_color_map_image(&material_colors);
    falling_sand_images.color_map = chunk_creation_params.images.add(color_map_image);
    let radius = 10;
    let chunk_positions = (-radius..=radius)
        .cartesian_product(-radius..=radius)
        .map(|(x, y)| (x, y).into());
    chunk_creation_params.spawn_chunks(chunk_positions);
}

fn create_chunk_images(
    size: (u32, u32),
    falling_sand_grid: &ChunkData,
    images: &mut Assets<Image>,
    initial_color: Color,
) -> (Handle<Image>, Handle<Image>) {
    // Create the particle grid texture
    let mut grid_image = Image::new_fill(
        Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::R32Uint,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    grid_image.texture_descriptor.usage = TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING;
    grid_image.texture_descriptor.label = Some("chunk_texture");

    grid_image.data.copy_from_slice(cast_slice(
        falling_sand_grid.particles().array().as_slice().unwrap(),
    ));

    // Create the render target texture
    let mut render_target = Image::new_fill(
        Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        cast_slice(&initial_color.as_linear_rgba_f32()),
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    render_target.texture_descriptor.usage =
        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    // Add the textures to the asset server and get the handles
    let grid_texture = images.add(grid_image);
    let color_image = images.add(render_target);
    (grid_texture, color_image)
}

fn create_color_map_image(material_colors: &MaterialColor) -> Image {
    let material_colors_vec = material_colors
        .values()
        .flat_map(|c| c.as_rgba_u8())
        .collect::<Vec<u8>>();

    let mut color_map_image = Image::new(
        Extent3d {
            height: 1,
            width: material_colors.0.len() as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D1,
        material_colors_vec,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    color_map_image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING;
    color_map_image.texture_descriptor.label = Some("color_map_texture");
    color_map_image
}
