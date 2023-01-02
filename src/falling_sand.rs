use std::borrow::Cow;

use bevy::prelude::*;

use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, CachedComputePipelineId,
    ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, PipelineCache, ShaderStages,
    StorageTextureAccess, TextureDimension, TextureFormat, TextureUsages, TextureViewDimension,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::TextureFormatPixelInfo;
use bevy::render::{render_graph, RenderApp, RenderStage};

use bytemuck::cast_slice;
use ndarray::s;

use crate::grid::Grid;
use crate::types::Material;

#[derive(Component)]
pub struct FallingSand {
    pub cells: Grid,
    pub scratch: Grid,
    pub materials_texture: Handle<Image>,
    pub color_map: Handle<Image>,
}

impl FallingSand {
    pub fn new(
        width: usize,
        height: usize,
        texture: Handle<Image>,
        color_map: Handle<Image>,
    ) -> Self {
        FallingSand {
            cells: Grid::new(width, height),
            scratch: Grid::new(width, height),
            materials_texture: texture,
            color_map,
        }
    }

    pub fn new_from_board(board: &Grid, texture: Handle<Image>, color_map: Handle<Image>) -> Self {
        let width = board.nrows();
        let height = board.ncols();
        FallingSand {
            cells: board.clone(),
            scratch: Grid::new(width, height),
            materials_texture: texture,
            color_map,
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.cells.nrows(), self.cells.ncols())
    }
}

pub fn grid_system(falling_sand: Query<&FallingSand>, mut textures: ResMut<Assets<Image>>) {
    for falling_sand in &falling_sand {
        if let Some(materials_texture) = textures.get_mut(&falling_sand.materials_texture) {
            materials_texture.data.copy_from_slice(cast_slice(
                falling_sand
                    .cells
                    .as_slice()
                    .expect("Failed to get slice from grid"),
            ));
        }
    }
}

pub struct FallingSandPlugin;

impl Plugin for FallingSandPlugin {
    fn build(&self, app: &mut App) {
        let settings = FallingSandSettings {
            size: (200, 200),
            tile_size: 4,
        };
        app.add_plugin(ExtractResourcePlugin::<FallingSandImages>::default())
            .add_plugin(ExtractResourcePlugin::<FallingSandSettings>::default())
            .insert_resource(settings)
            .add_startup_system(setup)
            .add_system(grid_system);

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<FallingSandPipeline>()
            .add_system_to_stage(RenderStage::Queue, queue_bind_group);

        let mut render_graph = render_app.world.resource_mut::<render_graph::RenderGraph>();
        render_graph.add_node("falling_sand", FallingSandNode::default());
        render_graph
            .add_node_edge(
                "falling_sand",
                bevy::render::main_graph::node::CAMERA_DRIVER,
            )
            .expect("Failed to add falling_sand node to render graph");
    }
}

#[derive(Resource, Clone, ExtractResource)]
pub struct FallingSandSettings {
    pub size: (usize, usize),
    pub tile_size: u32,
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
    update_pipeline: CachedComputePipelineId,
}

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<FallingSandPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    falling_sand_images: Res<FallingSandImages>,
    render_device: Res<RenderDevice>,
) {
    let grid_view = &gpu_images[&falling_sand_images.grid_texture];
    let color_map_view = &gpu_images[&falling_sand_images.color_map];
    let color_view = &gpu_images[&falling_sand_images.color_texture];

    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        layout: &pipeline.texture_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&grid_view.texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&color_map_view.texture_view),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&color_view.texture_view),
            },
        ],
        label: Some("grid_material_bind_group"),
    });

    commands.insert_resource(FallingSandImagesBindGroup(bind_group));
}

impl FromWorld for FallingSandPipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadOnly,
                                format: TextureFormat::R32Uint,
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

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            layout: Some(vec![texture_bind_group_layout.clone()]),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("update"),
            label: None,
        });

        FallingSandPipeline {
            texture_bind_group_layout,
            update_pipeline,
        }
    }
}

#[derive(Default)]
struct FallingSandNode {
    size: (usize, usize),
}

impl render_graph::Node for FallingSandNode {
    fn run(
        &self,
        graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<FallingSandImagesBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<FallingSandPipeline>();

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        let update_pipeline = pipeline_cache
            .get_compute_pipeline(pipeline.update_pipeline)
            .unwrap();
        pass.set_pipeline(update_pipeline);

        let size = (self.size.0 as u32, self.size.1 as u32);
        let workgroup_size = 10;
        pass.dispatch_workgroups(size.0 / workgroup_size, size.1 / workgroup_size, 1);

        Ok(())
    }

    fn update(&mut self, _world: &mut World) {
        let falling_sand_settings = _world.resource::<FallingSandSettings>();

        self.size = falling_sand_settings.size;
    }
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
    let mut grid_image = Image::new_fill(
        Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::R32Uint,
    );
    let pixel_info = grid_image.texture_descriptor.format.pixel_info();
    info!(
        "Pixel info: size {}, num_components {}",
        pixel_info.type_size, pixel_info.num_components
    );
    grid_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    grid_image.texture_descriptor.label = Some("grid_texture");

    let material_colors = vec![
        255u8, 255u8, 255u8, 255u8, // Air
        77, 77, 77, 255u8, // Bedrock
        244, 215, 21, 255u8, // Sand
        255, 0, 0, 255u8, // Water
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

    let mut color_image = Image::new_fill(
        Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    color_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    let grid_texture = images.add(grid_image);
    let color_map_image = images.add(color_map_image);
    let color_image = images.add(color_image);
    let scale = falling_sand_settings.tile_size;

    let board = {
        let mut grid = Grid::new(size.0 as usize, size.1 as usize);
        info!("Setting initial grid state");
        grid.slice_mut(s![10..20, 1]).fill(Material::Sand);
        grid.slice_mut(s![0..99, 99]).fill(Material::Bedrock);
        grid
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new((size.0 * scale) as f32, (size.1 * scale) as f32)),
                // flip_x: false,
                flip_y: true,
                ..default()
            },
            texture: color_image.clone(),
            transform: Transform::from_rotation(Quat::from_rotation_z(-std::f32::consts::PI / 2.0)),
            ..default()
        },
        FallingSand::new_from_board(&board, grid_texture.clone(), color_map_image.clone()),
    ));

    commands.insert_resource(FallingSandImages {
        grid_texture,
        color_map: color_map_image,
        color_texture: color_image,
    });
}
