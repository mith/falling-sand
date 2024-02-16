use std::{
    borrow::Cow,
    hash::Hash,
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    render::{
        render_resource::{BindGroupEntries, CachedPipelineState},
        Render,
    },
    transform::commands,
    ui::update,
    utils::{HashMap, HashSet},
};

use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, Extent3d,
    PipelineCache, ShaderStages, StorageTextureAccess, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDimension,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::{render_graph, RenderApp, RenderSet};

use bevy_egui::egui::text;
use bytemuck::cast_slice;
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    chunk::{Chunk, ChunkData},
    falling_sand_grid::{
        update_chunk_positions, ChunkActive, ChunkPosition, ChunkPositions, CHUNK_SIZE,
    },
    fire::fire_to_smoke,
    material::MaterialIterator,
    material::{Material, MaterialColor, MaterialPlugin},
    movement::{fall, flow},
    process_chunks::ChunksParam,
    reactions::react,
    util::chunk_neighbors,
};

#[derive(Default)]
pub struct FallingSandPlugin {
    pub settings: FallingSandSettings,
}

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FallingSandSet;

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FallingSandPostSet;

#[derive(Resource)]
pub struct FallingSandRng(pub StdRng);

impl Plugin for FallingSandPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<FallingSandImages>::default(),
            ExtractResourcePlugin::<FallingSandSettings>::default(),
            ExtractResourcePlugin::<DirtyOrCreatedChunks>::default(),
            MaterialPlugin,
        ))
        .insert_resource(Time::<Virtual>::from_max_delta(Duration::from_secs_f32(
            1. / 64.,
        )))
        .insert_resource(self.settings.clone())
        .insert_resource(FallingSandRng(StdRng::seed_from_u64(0)))
        .init_resource::<ChunkPositions>()
        .init_resource::<DirtyChunks>()
        .init_resource::<DirtyOrCreatedChunks>()
        .init_resource::<FallingSandImages>()
        .add_systems(
            Startup,
            setup.before(FallingSandSet).before(FallingSandPostSet),
        )
        .add_systems(
            FixedUpdate,
            (
                (
                    update_chunk_positions,
                    clean_particles,
                    // fall,
                    // flow,
                    // clean_particles,
                    // react,
                    // fire_to_smoke,
                )
                    .in_set(FallingSandSet)
                    .chain(),
                // draw_debug_gizmoz,
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                apply_deferred,
                activate_dirty_chunks,
                apply_deferred,
                update_dirty_chunks,
                grid_to_texture,
                clean_chunks,
            )
                .chain()
                .in_set(FallingSandPostSet)
                .after(FallingSandSet),
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<FallingSandImagesBindGroups>()
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            );

        let mut render_graph = render_app.world.resource_mut::<render_graph::RenderGraph>();
        render_graph.add_node("falling_sand", FallingSandNode::default());
        render_graph.add_node_edge(
            "falling_sand",
            bevy::render::main_graph::node::CAMERA_DRIVER,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<FallingSandPipeline>();
    }
}

fn clean_particles(chunk_query: Query<&Chunk>) {
    chunk_query.par_iter().for_each(|grid| {
        for dirty in grid.write().unwrap().attributes_mut().dirty.iter_mut() {
            *dirty = false;
        }
    });
}

fn clean_chunks(chunk_query: Query<&Chunk>) {
    chunk_query.par_iter().for_each(|chunk| {
        chunk.write().unwrap().set_dirty(false);
    });
}

fn update_dirty_chunks(
    mut dirty_chunks: ResMut<DirtyChunks>,
    mut dirty_or_created_chunks: ResMut<DirtyOrCreatedChunks>,
    chunk_query: Query<(&Chunk, &ChunkPosition)>,
    mut seen_chunks: Local<HashSet<IVec2>>,
) {
    dirty_chunks.0.clear();
    for (chunk, chunk_position) in chunk_query.iter() {
        if chunk.read().unwrap().is_dirty() {
            dirty_chunks.0.insert(chunk_position.0);
        }

        if !seen_chunks.contains(&chunk_position.0) {
            dirty_or_created_chunks.0.insert(chunk_position.0);
            seen_chunks.insert(chunk_position.0);
        }
    }

    dirty_or_created_chunks.0.extend(dirty_chunks.0.iter());
}

fn activate_dirty_chunks(
    mut commands: Commands,
    dirty_chunks: ResMut<DirtyChunks>,
    mut chunk_creation_params: ChunkCreationParams,
    chunk_params: ChunksParam,
) {
    for position in dirty_chunks.0.iter() {
        let chunk = chunk_params
            .get_chunk_entity_at(position.x, position.y)
            .unwrap();

        commands.entity(chunk).insert(ChunkActive);

        // If there's no chunks in the neighborhood, create them
        for neighbor in chunk_neighbors(*position) {
            if !chunk_params.chunk_exists(neighbor) {
                chunk_creation_params.create_chunk(neighbor, false);
            }
        }
    }
}

#[derive(Component, Reflect)]
pub struct FallingSandSprite {
    pub materials_texture: Handle<Image>,
    pub color_map: Handle<Image>,
}

fn grid_to_texture(
    falling_sand: Query<(Entity, &FallingSandSprite, &Chunk, &ChunkPosition)>,
    mut textures: ResMut<Assets<Image>>,
    mut initialized_textures: Local<HashSet<Entity>>,
) {
    for (chunk_entity, falling_sand, chunk, position) in &falling_sand {
        if !chunk.read().unwrap().is_dirty() && initialized_textures.contains(&chunk_entity) {
            continue;
        }
        if !initialized_textures.contains(&chunk_entity) {
            initialized_textures.insert(chunk_entity);
        }
        debug!(chunk_position=?position.0, "Updating chunk texture");
        if let Some(materials_texture) = textures.get_mut(&falling_sand.materials_texture) {
            let chunk_data = &chunk.read().unwrap();
            let particle_grid = chunk_data.particles();
            let particle_array = particle_grid.array();
            materials_texture.data.copy_from_slice(cast_slice(
                particle_array
                    .as_slice()
                    .expect("Failed to get slice from grid"),
            ));
        }
    }
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

#[derive(Clone, Reflect)]
struct ChunkImages {
    pub grid_texture: Handle<Image>,
    pub color_map: Handle<Image>,
    pub color_texture: Handle<Image>,
}

#[derive(Resource, Clone, ExtractResource, Default, Reflect)]
struct FallingSandImages {
    chunk_images: HashMap<IVec2, ChunkImages>,
}

#[derive(Resource, Clone, Default, Reflect)]
struct DirtyChunks(HashSet<IVec2>);

#[derive(Resource, Clone, ExtractResource, Default)]
struct DirtyOrCreatedChunks(HashSet<IVec2>);

#[derive(Resource, Default)]
struct FallingSandImagesBindGroups(HashMap<IVec2, BindGroup>);

#[derive(Resource)]
struct FallingSandPipeline {
    texture_bind_group_layout: BindGroupLayout,
    render_pipeline: CachedComputePipelineId,
}

fn prepare_bind_group(
    pipeline: Res<FallingSandPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    falling_sand_images: Res<FallingSandImages>,
    mut falling_sand_imgages_bind_groups: ResMut<FallingSandImagesBindGroups>,
    render_device: Res<RenderDevice>,
) {
    for (position, images) in falling_sand_images.chunk_images.iter() {
        let particle_grid = gpu_images.get(&images.grid_texture).unwrap();
        let color_map = gpu_images.get(&images.color_map).unwrap();
        let render_target = gpu_images.get(&images.color_texture).unwrap();

        let bind_group = render_device.create_bind_group(
            Some(format!("grid_material_bind_group_{}", position).as_str()),
            &pipeline.texture_bind_group_layout,
            &BindGroupEntries::sequential((
                &particle_grid.texture_view,
                &color_map.texture_view,
                &render_target.texture_view,
            )),
        );

        falling_sand_imgages_bind_groups
            .0
            .insert(*position, bind_group);
    }
}

impl FromWorld for FallingSandPipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("chunk_material_bind_group_layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadOnly,
                                format: TextureFormat::Rg32Uint,
                                view_dimension: TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadOnly,
                                format: TextureFormat::Rgba8Unorm,
                                view_dimension: TextureViewDimension::D1,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::WriteOnly,
                                format: TextureFormat::Rgba8Unorm,
                                view_dimension: TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/grid_to_texture.wgsl");

        let pipeline_cache = world.resource_mut::<PipelineCache>();
        let render_grid_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("render_chunk_pipeline".into()),
                layout: vec![texture_bind_group_layout.clone()],
                push_constant_ranges: vec![],
                shader,
                shader_defs: vec![],
                entry_point: Cow::from("render_grid"),
            });

        FallingSandPipeline {
            texture_bind_group_layout,
            render_pipeline: render_grid_pipeline,
        }
    }
}

#[derive(Default)]
enum FallingSandState {
    #[default]
    Loading,
    Render,
}

#[derive(Default)]
struct FallingSandNode {
    state: FallingSandState,
    size: (usize, usize),
}

impl render_graph::Node for FallingSandNode {
    fn update(&mut self, world: &mut World) {
        let falling_sand_settings = world.resource::<FallingSandSettings>();

        self.size = falling_sand_settings.size;

        let pipeline = world.resource::<FallingSandPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        match self.state {
            FallingSandState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.render_pipeline)
                {
                    info!("Falling sand pipeline loaded");
                    self.state = FallingSandState::Render;
                }
            }
            FallingSandState::Render => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<FallingSandImagesBindGroups>().0;
        let dirty_chunks = world.resource::<DirtyOrCreatedChunks>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<FallingSandPipeline>();

        match self.state {
            FallingSandState::Loading => {}
            FallingSandState::Render => {
                for (position, bind_group) in texture_bind_group.iter() {
                    if !dirty_chunks.0.contains(position) {
                        continue;
                    }
                    let mut pass = render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor::default());

                    pass.set_bind_group(0, bind_group, &[]);

                    let render_pipeline = pipeline_cache
                        .get_compute_pipeline(pipeline.render_pipeline)
                        .unwrap();
                    pass.set_pipeline(render_pipeline);

                    let size = (self.size.0 as u32, self.size.1 as u32);
                    let workgroup_size = 10;
                    pass.dispatch_workgroups(size.0 / workgroup_size, size.1 / workgroup_size, 1);
                }
            }
        }

        Ok(())
    }
}

fn draw_debug_gizmoz(
    mut gizmos: Gizmos,
    falling_sand_settings: Res<FallingSandSettings>,
    chunk_positions: Query<&ChunkPosition>,
) {
    for position in &chunk_positions {
        let position =
            position.0.as_vec2() * CHUNK_SIZE as f32 * falling_sand_settings.tile_size as f32;
        gizmos.rect_2d(
            position,
            0.,
            Vec2::new(
                falling_sand_settings.size.0 as f32,
                falling_sand_settings.size.1 as f32,
            ),
            Color::RED,
        );
        gizmos.circle_2d(position, 0.1, Color::BLACK);
    }
}

#[derive(SystemParam)]
pub struct ChunkCreationParams<'w, 's> {
    commands: Commands<'w, 's>,
    images: ResMut<'w, Assets<Image>>,
    falling_sand_images: ResMut<'w, FallingSandImages>,
    falling_sand_settings: Res<'w, FallingSandSettings>,
    material_colors: Res<'w, MaterialColor>,
}

impl<'w, 's> ChunkCreationParams<'w, 's> {
    pub fn create_chunk(&mut self, position: IVec2, active: bool) {
        create_chunk(
            &mut self.commands,
            &mut self.images,
            &mut self.falling_sand_images,
            &self.falling_sand_settings,
            &self.material_colors,
            position,
            active,
        );
    }
}

fn setup(mut chunk_creation_params: ChunkCreationParams) {
    let radius = 10;
    for x in -radius..=radius {
        for y in -radius..=radius {
            chunk_creation_params.create_chunk(
                (x, y).into(),
                x.abs() <= (radius - 1) && y.abs() <= (radius - 1),
            );
        }
    }
}

fn create_chunk(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    falling_sand_images: &mut FallingSandImages,
    falling_sand_settings: &FallingSandSettings,
    material_colors: &MaterialColor,
    position: IVec2,
    active: bool,
) {
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
    let chunk = Chunk::new_with_material((size.0 as usize, size.1 as usize), material, rng);

    let (grid_texture, color_map_image, color_image) =
        create_chunk_images(size, &chunk.read().unwrap(), material_colors, images);

    falling_sand_images.chunk_images.insert(
        IVec2::new(x, y),
        ChunkImages {
            grid_texture: grid_texture.clone(),
            color_map: color_map_image.clone(),
            color_texture: color_image.clone(),
        },
    );

    let mut new_chunk = commands.spawn((
        Name::new("Chunk"),
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new((size.0 * scale) as f32, (size.1 * scale) as f32)),
                ..default()
            },
            texture: color_image,
            transform: Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::PI / 2.0))
                .with_translation(Vec3::new(
                    (x * size.0 as i32 * scale as i32) as f32,
                    (y * size.1 as i32 * scale as i32) as f32,
                    0.0,
                )),
            ..default()
        },
        FallingSandSprite {
            materials_texture: grid_texture,
            color_map: color_map_image,
        },
        chunk,
        ChunkPosition(IVec2::new(x, y)),
    ));

    if active {
        new_chunk.insert(ChunkActive);
    }
}

fn create_chunk_images(
    size: (u32, u32),
    falling_sand_grid: &ChunkData,
    material_colors: &MaterialColor,
    images: &mut Assets<Image>,
) -> (Handle<Image>, Handle<Image>, Handle<Image>) {
    // Create the particle grid texture
    let mut grid_image = Image::new_fill(
        Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 16],
        TextureFormat::Rg32Uint,
    );
    grid_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    grid_image.texture_descriptor.label = Some("chunk_texture");

    grid_image.data.copy_from_slice(cast_slice(
        falling_sand_grid.particles().array().as_slice().unwrap(),
    ));

    let material_colors_vec = MaterialIterator::new()
        .map(|m| material_colors.0[m])
        .flat_map(|c| [c[0], c[1], c[2], 255u8])
        .collect::<Vec<u8>>();

    let mut color_map_image = Image::new(
        Extent3d {
            height: 1,
            width: material_colors.0.len() as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D1,
        material_colors_vec,
        TextureFormat::Rgba8Unorm,
    );
    color_map_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    color_map_image.texture_descriptor.label = Some("color_map_texture");

    // Create the render target texture
    let mut render_target = Image::new_fill(
        Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    render_target.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    // Add the textures to the asset server and get the handles
    let grid_texture = images.add(grid_image);
    let color_map_image = images.add(color_map_image);
    let color_image = images.add(render_target);
    (grid_texture, color_map_image, color_image)
}
