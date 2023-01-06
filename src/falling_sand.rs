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

use bevy_inspector_egui::Inspectable;
use bytemuck::cast_slice;
use ndarray::s;

use crate::grid::Grid;
use crate::margolus::{margolus_gravity, MargulosState};
use crate::types::{Material, MaterialDensities, MaterialPhases, Phase};

pub struct DoubleBuffered<T> {
    pub odd: T,
    pub even: T,
    target_is_odd: bool,
}

impl<T> DoubleBuffered<T> {
    pub fn new(odd: T, even: T) -> Self {
        DoubleBuffered {
            odd,
            even,
            target_is_odd: false,
        }
    }
    pub fn source(&self) -> &T {
        if self.target_is_odd {
            &self.even
        } else {
            &self.odd
        }
    }

    pub fn target(&self) -> &T {
        if self.target_is_odd {
            &self.odd
        } else {
            &self.even
        }
    }

    pub fn target_mut(&mut self) -> &mut T {
        if self.target_is_odd {
            &mut self.odd
        } else {
            &mut self.even
        }
    }

    pub fn source_and_target_mut(&mut self) -> (&T, &mut T) {
        if self.target_is_odd {
            (&self.even, &mut self.odd)
        } else {
            (&self.odd, &mut self.even)
        }
    }

    pub fn swap(&mut self) {
        self.target_is_odd = !self.target_is_odd;
    }
}

#[derive(Component)]
pub struct FallingSand {
    pub cells: DoubleBuffered<Grid>,
    pub materials_texture: Handle<Image>,
    pub color_map: Handle<Image>,
}

impl FallingSand {
    pub fn new_from_board(board: &Grid, texture: Handle<Image>, color_map: Handle<Image>) -> Self {
        FallingSand {
            cells: DoubleBuffered::new(board.clone(), board.clone()),
            materials_texture: texture,
            color_map,
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.cells.target().nrows(), self.cells.target().ncols())
    }
}

pub fn gravity_system(
    mut grid_query: Query<&mut FallingSand>,
    mut margolus: ResMut<MargulosState>,
    falling_sand_settings: Res<FallingSandSettings>,
) {
    for mut grid in grid_query.iter_mut() {
        grid.cells.swap();
        let (source, target) = {
            if margolus.odd_timestep {
                let (source, target) = grid.cells.source_and_target_mut();

                // Copy the border from the source to the target first
                match &falling_sand_settings.border_update_mode {
                    BorderUpdateMode::CopyEntireSource => {
                        target.assign(&source);
                    }
                    BorderUpdateMode::CopyBorder => {
                        target.slice_mut(s![0, ..]).assign(&source.slice(s![0, ..]));
                        target
                            .slice_mut(s![-1, ..])
                            .assign(&source.slice(s![-1, ..]));
                        target.slice_mut(s![.., 0]).assign(&source.slice(s![.., 0]));
                        target
                            .slice_mut(s![.., -1])
                            .assign(&source.slice(s![.., -1]));
                    }
                };
                (
                    source.slice(s![1..-1, 1..-1]),
                    target.slice_mut(s![1..-1, 1..-1]),
                )
            } else {
                let (source, target) = grid.cells.source_and_target_mut();
                (source.view(), target.view_mut())
            }
        };

        margolus_gravity(source, target, falling_sand_settings.parallel_gravity);
        margolus.odd_timestep = !margolus.odd_timestep;
    }
}

pub fn grid_to_texture(falling_sand: Query<&FallingSand>, mut textures: ResMut<Assets<Image>>) {
    for falling_sand in &falling_sand {
        if let Some(materials_texture) = textures.get_mut(&falling_sand.materials_texture) {
            materials_texture.data.copy_from_slice(cast_slice(
                falling_sand
                    .cells
                    .target()
                    .as_slice()
                    .expect("Failed to get slice from grid"),
            ));
        }
    }
}

pub struct FallingSandPlugin {
    pub settings: FallingSandSettings,
}

#[derive(SystemLabel)]
pub struct FallingSandPhase;

impl Plugin for FallingSandPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractResourcePlugin::<FallingSandImages>::default())
            .add_plugin(ExtractResourcePlugin::<FallingSandSettings>::default())
            .insert_resource(self.settings.clone())
            .init_resource::<MargulosState>()
            .insert_resource({
                MaterialDensities(enum_map! {
                Material::Air => 0,
                Material::Water => 1,
                Material::Sand => 2,
                Material::Bedrock => 3,
                })
            })
            .insert_resource({
                MaterialPhases(enum_map! {
                Material::Air => Phase::Gas,
                Material::Water => Phase::Liquid,
                Material::Sand => Phase::Liquid,
                Material::Bedrock => Phase::Solid,
                })
            })
            .add_startup_system(setup)
            .add_system_set(
                SystemSet::new()
                    .label(FallingSandPhase)
                    .with_system(gravity_system)
                    .with_system(grid_to_texture.after(gravity_system)),
            );

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

#[derive(Clone, Reflect, Inspectable)]
pub enum BorderUpdateMode {
    CopyEntireSource,
    CopyBorder,
}

#[derive(Resource, Clone, ExtractResource, Reflect, Inspectable)]
pub struct FallingSandSettings {
    pub size: (usize, usize),
    pub tile_size: u32,
    pub border_update_mode: BorderUpdateMode,
    pub parallel_gravity: bool,
}

impl Default for FallingSandSettings {
    fn default() -> Self {
        FallingSandSettings {
            size: (500, 500),
            tile_size: 2,
            border_update_mode: BorderUpdateMode::CopyBorder,
            parallel_gravity: true,
        }
    }
}

#[derive(Resource, Clone, ExtractResource)]
struct FallingSandImages {
    pub grid_texture: Handle<Image>,
    pub color_map: Handle<Image>,
    pub color_texture: Handle<Image>,
}

#[derive(Resource, Deref, DerefMut)]
struct FallingSandImagesBindGroup(BindGroup);

#[derive(Resource)]
pub struct FallingSandPipeline {
    texture_bind_group_layout: BindGroupLayout,
    render_pipeline: CachedComputePipelineId,
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
            render_pipeline: update_pipeline,
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
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<FallingSandImagesBindGroup>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<FallingSandPipeline>();

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        let update_pipeline = pipeline_cache
            .get_compute_pipeline(pipeline.render_pipeline)
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

    let board = {
        let mut grid = Grid::new(size.0 as usize, size.1 as usize);
        info!("Setting initial grid state");
        grid.slice_mut(s![10..20, 1]).fill(Material::Sand);
        grid.slice_mut(s![0..99, 99]).fill(Material::Bedrock);
        grid
    };

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

    grid_image.data.copy_from_slice(cast_slice(
        board.as_slice().expect("Failed to get grid data"),
    ));

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn double_buffered_swap() {
        let mut buffer = DoubleBuffered::new(1, 2);

        assert!(*buffer.source() == 1);
        assert!(*buffer.target() == 2);

        buffer.swap();

        assert!(*buffer.source() == 2);
        assert!(*buffer.target() == 1);
    }

    #[test]
    fn double_buffered_source_and_target_mut() {
        let mut buffer = DoubleBuffered::new(1, 2);

        buffer.swap();

        let (source, target) = buffer.source_and_target_mut();
        assert!(*source == 2);
        assert!(*target == 1);
    }
}
