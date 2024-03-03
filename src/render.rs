use std::{borrow::Cow, num::NonZeroU32, process::exit};

use bevy::{
    app::{App, Plugin},
    asset::{AssetServer, Handle},
    ecs::{
        schedule::IntoSystemConfigs,
        system::{Query, Res, ResMut, Resource},
        world::{FromWorld, World},
    },
    render::{
        extract_resource::ExtractResource,
        render_asset::RenderAssets,
        render_graph::{self, RenderLabel},
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntry, BindingType,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, ShaderStages, StorageTextureAccess,
            TextureFormat, TextureViewDimension,
        },
        renderer::RenderDevice,
        settings::WgpuFeatures,
        texture::Image,
        ExtractSchedule, Render, RenderApp, RenderSet,
    },
};
use itertools::Itertools;
use tracing::{error, info, info_span};

use crate::falling_sand::FallingSandSettings;

use self::extract::ExtractedChunkUpdate;

pub mod extract;

pub struct FallingSandRenderPlugin;

impl Plugin for FallingSandRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(ExtractSchedule, extract::extract);

        render_app
            .init_resource::<FallingSandImagesBindGroups>()
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            );

        let mut render_graph = render_app.world.resource_mut::<render_graph::RenderGraph>();
        render_graph.add_node(FallingSandRenderLabel, FallingSandNode::default());
        render_graph.add_node_edge(
            FallingSandRenderLabel,
            bevy::render::graph::CameraDriverLabel,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<FallingSandPipeline>();

        let render_device = render_app.world.resource::<RenderDevice>();

        // Check if the device support the required feature. If not, exit the example.
        // In a real application, you should setup a fallback for the missing feature
        if !render_device
            .features()
            .contains(WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING)
        {
            error!(
                "Render device doesn't support feature \
SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING, \
which is required for texture binding arrays"
            );
            exit(1);
        }
    }
}

#[derive(Resource, Clone, ExtractResource, Default)]
pub struct FallingSandImages {
    pub color_map: Handle<Image>,
}
#[derive(Resource, Default)]
struct FallingSandImagesBindGroups(Vec<(u32, BindGroup)>);

#[derive(Resource)]
struct FallingSandPipeline {
    texture_bind_group_layout: BindGroupLayout,
    render_pipeline: CachedComputePipelineId,
}

const MAX_TEXTURE_COUNT: usize = 64;

fn prepare_bind_group(
    pipeline: Res<FallingSandPipeline>,
    image_assets: Res<RenderAssets<Image>>,
    falling_sand_images: Res<FallingSandImages>,
    extracted_chunks: Query<&ExtractedChunkUpdate>,
    mut falling_sand_imgages_bind_groups: ResMut<FallingSandImagesBindGroups>,
    render_device: Res<RenderDevice>,
) {
    let color_map_texture = &image_assets
        .get(falling_sand_images.color_map.clone())
        .unwrap()
        .texture_view;

    falling_sand_imgages_bind_groups.0.clear();

    for chunks in &extracted_chunks.iter().chunks(MAX_TEXTURE_COUNT) {
        let (grid_textures, color_textures): (Vec<_>, Vec<_>) = chunks.fold(
            (
                Vec::with_capacity(MAX_TEXTURE_COUNT),
                Vec::with_capacity(MAX_TEXTURE_COUNT),
            ),
            |(mut grid_textures, mut color_textures), images| {
                grid_textures.push(&*images.materials_texture.default_view);
                color_textures.push(&*images.color_texture);
                (grid_textures, color_textures)
            },
        );

        let bind_group = render_device.create_bind_group(
            "bindless_grid_material_bind_group",
            &pipeline.texture_bind_group_layout,
            &BindGroupEntries::sequential((
                &grid_textures[..],
                color_map_texture,
                &color_textures[..],
            )),
        );

        falling_sand_imgages_bind_groups
            .0
            .push((grid_textures.len() as u32, bind_group));
    }
}

impl FromWorld for FallingSandPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "bindless_grid_material_bind_group_layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadOnly,
                        format: TextureFormat::R32Uint,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: NonZeroU32::new(MAX_TEXTURE_COUNT as u32),
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
                    count: NonZeroU32::new(MAX_TEXTURE_COUNT as u32),
                },
            ],
        );
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
pub enum FallingSandState {
    #[default]
    Loading,
    Render,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct FallingSandRenderLabel;

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
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<FallingSandPipeline>();

        match self.state {
            FallingSandState::Loading => {}
            FallingSandState::Render => {
                let span = info_span!("dispatch_render_chunks");
                let _guard = span.enter();
                for (group_size, bind_group) in texture_bind_group.iter() {
                    let span = info_span!("dispatch_render_chunk");
                    let _guard = span.enter();
                    let mut pass = render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor::default());

                    pass.set_bind_group(0, bind_group, &[]);

                    let render_pipeline = pipeline_cache
                        .get_compute_pipeline(pipeline.render_pipeline)
                        .unwrap();
                    pass.set_pipeline(render_pipeline);

                    let size = (self.size.0 as u32, self.size.1 as u32);
                    let workgroup_size = 8;
                    pass.dispatch_workgroups(
                        size.0 / workgroup_size,
                        size.1 / workgroup_size,
                        *group_size,
                    );
                }
            }
        }

        Ok(())
    }
}
