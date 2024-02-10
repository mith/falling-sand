use std::{borrow::Cow, time::Duration};

use bevy::{
    prelude::*,
    render::{
        render_resource::{BindGroupEntries, CachedPipelineState},
        Render,
    },
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

use bytemuck::cast_slice;
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    movement::{fall, flow},
    particle_grid::{Particle, ParticleAttributeStore, ParticleGrid},
    types::{Material, MaterialDensities, MaterialFlowing, MaterialStates, StateOfMatter},
};

#[derive(Default)]
pub struct FallingSandPlugin {
    pub settings: FallingSandSettings,
}

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FallingSandSet;

#[derive(Resource)]
pub struct FallingSandRng(pub StdRng);

impl Plugin for FallingSandPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<FallingSandImages>::default(),
            ExtractResourcePlugin::<FallingSandSettings>::default(),
        ))
        .insert_resource(Time::<Virtual>::from_max_delta(Duration::from_secs_f32(
            1. / 64.,
        )))
        .insert_resource(self.settings.clone())
        .insert_resource({
            MaterialDensities(enum_map! {
            Material::Air => 1,
            Material::Water => 1000,
            Material::Sand => 1500,
            Material::Bedrock => 10000,
            })
        })
        .insert_resource({
            MaterialStates(enum_map! {
            Material::Air => StateOfMatter::Gas,
            Material::Water => StateOfMatter::Liquid,
            Material::Sand => StateOfMatter::Liquid,
            Material::Bedrock => StateOfMatter::Solid,
            })
        })
        .insert_resource({
            MaterialFlowing(enum_map! {
            Material::Air => false,
            Material::Water => true,
            Material::Sand => false,
            Material::Bedrock => false,
            })
        })
        .insert_resource(FallingSandRng(StdRng::seed_from_u64(0)))
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (
                (clean_particles, fall, flow, grid_to_texture).chain(),
                draw_debug_gizmoz,
            ),
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
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

#[derive(Component)]
pub struct FallingSandGrid {
    pub particles: ParticleGrid,
    pub particle_dirty: ParticleAttributeStore<bool>,
}

impl FallingSandGrid {
    pub fn new(size: (usize, usize)) -> FallingSandGrid {
        let particle_grid = ParticleGrid::new(size);
        let size = particle_grid.array().len();
        FallingSandGrid {
            particles: particle_grid,
            particle_dirty: ParticleAttributeStore::new(size),
        }
    }

    pub fn size(&self) -> IVec2 {
        IVec2::new(
            self.particles.array().dim().0 as i32,
            self.particles.array().dim().1 as i32,
        )
    }

    pub fn swap_particles(&mut self, a: (i32, i32), b: (i32, i32)) {
        self.particles
            .array_mut()
            .swap((a.0 as usize, a.1 as usize), (b.0 as usize, b.1 as usize));
        // Mark the particles as dirty
        *self
            .particle_dirty
            .get_mut(self.get(a.0, a.1).unwrap().id)
            .unwrap() = true;
        *self
            .particle_dirty
            .get_mut(self.get(b.0, b.1).unwrap().id)
            .unwrap() = true;
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&Particle> {
        self.particles.array().get((x as usize, y as usize))
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Particle> {
        self.particles.array_mut().get_mut((x as usize, y as usize))
    }
}

pub fn clean_particles(mut grid_query: Query<&mut FallingSandGrid>) {
    for mut grid in grid_query.iter_mut() {
        for dirty in grid.particle_dirty.iter_mut() {
            *dirty = false;
        }
    }
}

#[derive(Component, Reflect)]
pub struct FallingSandSprite {
    pub materials_texture: Handle<Image>,
    pub color_map: Handle<Image>,
}

pub fn grid_to_texture(
    falling_sand: Query<(&FallingSandSprite, &FallingSandGrid)>,
    mut textures: ResMut<Assets<Image>>,
) {
    for (falling_sand, grid) in &falling_sand {
        if let Some(materials_texture) = textures.get_mut(&falling_sand.materials_texture) {
            materials_texture.data.copy_from_slice(cast_slice(
                grid.particles
                    .array()
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
            size: (100, 100),
            tile_size: 1,
        }
    }
}

#[derive(Resource, Clone, ExtractResource)]
struct FallingSandImages {
    pub grid_texture: Handle<Image>,
    pub color_map: Handle<Image>,
    pub color_texture: Handle<Image>,
}

#[derive(Resource)]
struct FallingSandImagesBindGroup(BindGroup);

#[derive(Resource)]
pub struct FallingSandPipeline {
    texture_bind_group_layout: BindGroupLayout,
    render_pipeline: CachedComputePipelineId,
}

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<FallingSandPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    falling_sand_images: Res<FallingSandImages>,
    render_device: Res<RenderDevice>,
) {
    let particle_grid = gpu_images.get(&falling_sand_images.grid_texture).unwrap();
    let color_map = gpu_images.get(&falling_sand_images.color_map).unwrap();
    let render_target = gpu_images.get(&falling_sand_images.color_texture).unwrap();

    let bind_group = render_device.create_bind_group(
        Some("grid_material_bind_group"),
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((
            &particle_grid.texture_view,
            &color_map.texture_view,
            &render_target.texture_view,
        )),
    );

    commands.insert_resource(FallingSandImagesBindGroup(bind_group));
}

impl FromWorld for FallingSandPipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("grid_material_bind_group_layout"),
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
                label: Some("render_grid_pipeline".into()),
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
        let texture_bind_group = &world.resource::<FallingSandImagesBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<FallingSandPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        match self.state {
            FallingSandState::Loading => {}
            FallingSandState::Render => {
                let render_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.render_pipeline)
                    .unwrap();
                pass.set_pipeline(render_pipeline);

                let size = (self.size.0 as u32, self.size.1 as u32);
                let workgroup_size = 10;
                pass.dispatch_workgroups(size.0 / workgroup_size, size.1 / workgroup_size, 1);
            }
        }

        Ok(())
    }
}

fn draw_debug_gizmoz(mut gizmos: Gizmos, falling_sand_settings: Res<FallingSandSettings>) {
    gizmos.rect_2d(
        Vec2::ZERO,
        0.,
        Vec2::new(
            falling_sand_settings.size.0 as f32,
            falling_sand_settings.size.1 as f32,
        ),
        Color::RED,
    );
    gizmos.circle_2d(Vec2::ZERO, 0.1, Color::BLACK);
}

pub fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    falling_sand_settings: Res<FallingSandSettings>,
) {
    let size = (
        falling_sand_settings.size.0 as u32,
        falling_sand_settings.size.1 as u32,
    );

    let mut falling_sand_grid = FallingSandGrid::new((size.0 as usize, size.1 as usize));

    falling_sand_grid.get_mut(0, 0).unwrap().material = Material::Sand;

    falling_sand_grid.get_mut(9, 0).unwrap().material = Material::Water;

    falling_sand_grid.get_mut(9, 9).unwrap().material = Material::Bedrock;

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
    grid_image.texture_descriptor.label = Some("grid_texture");

    grid_image.data.copy_from_slice(cast_slice(
        falling_sand_grid.particles.array().as_slice().unwrap(),
    ));

    // Create the color map texture
    let material_colors = vec![
        255u8, 255u8, 255u8, 255u8, // Air
        77, 77, 77, 255u8, // Bedrock
        244, 215, 21, 255u8, // Sand
        0, 0, 255, 255u8, // Water
    ];
    let mut color_map_image = Image::new(
        Extent3d {
            height: 1,
            width: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D1,
        material_colors,
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

    let scale = falling_sand_settings.tile_size;

    commands.spawn((
        Name::new("Falling Sand Grid"),
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new((size.0 * scale) as f32, (size.1 * scale) as f32)),
                ..default()
            },
            texture: color_image.clone(),
            transform: Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::PI / 2.0)),
            ..default()
        },
        FallingSandSprite {
            materials_texture: grid_texture.clone(),
            color_map: color_map_image.clone(),
        },
        falling_sand_grid,
    ));

    commands.insert_resource(FallingSandImages {
        grid_texture,
        color_map: color_map_image,
        color_texture: color_image,
    });
}
