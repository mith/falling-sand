use std::mem::size_of;

use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::render_resource::{
    AsBindGroup, AsBindGroupError, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Extent3d, OwnedBindingResource,
    PreparedBindGroup, ShaderStages, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureViewDimension,
};
use bevy::render::texture::TextureFormatPixelInfo;
use bevy::sprite::{Material2d, MaterialMesh2dBundle};

use ndarray::{s, ArrayViewMut};

use crate::grid::Grid;
use crate::types::Material;

#[derive(Component)]
pub struct FallingSand {
    pub cells: Grid,
    pub scratch: Grid,
    pub texture: Handle<Image>,
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
            texture,
            color_map,
        }
    }

    pub fn new_from_board(board: &Grid, texture: Handle<Image>, color_map: Handle<Image>) -> Self {
        let width = board.nrows();
        let height = board.ncols();
        FallingSand {
            cells: board.clone(),
            scratch: Grid::new(width, height),
            texture,
            color_map,
        }
    }
}

pub fn grid_system(falling_sand: Query<&FallingSand>, mut textures: ResMut<Assets<Image>>) {
    for grid in &falling_sand {
        if let Some(texture) = textures.get_mut(&grid.texture) {
            // texture.data = grid
            //     .cells
            //     .t()
            //     .iter()
            //     .map(|cell| match *cell {
            //         Material::Bedrock => 0u8,
            //         Material::Air => 1u8,
            //         Material::Sand => 2u8,
            //         Material::Water => 3u8,
            //     })
            //     .collect();
            texture.data[0] = 1u8;
            texture.data[99] = 2u8;
            texture.data[100 * 100 - 100] = 2u8;
            texture.data[100 * 100 - 1] = 3u8;

            assert!(texture.data.len() == size_of::<u8>() * 100 * 100);

            if let Ok(mut texture_grid) =
                ArrayViewMut::from_shape((100, 100), texture.data.as_mut_slice())
            {
                debug!("Uploading grid state to texture");
                // texture_grid.slice_mut(s![30..60, 30..60]).fill(2u8);
                // texture_grid.fill(2u8);
            }
        }
    }
}

#[derive(TypeUuid, Debug, Clone)]
#[uuid = "f4da3862-80e5-49db-b593-f08c04e3cfdf"]
pub struct GridMaterial {
    grid_texture: Handle<Image>,
    color_map: Handle<Image>,
}

impl Material2d for GridMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/grid.wgsl".into()
    }
}

impl AsBindGroup for GridMaterial {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &bevy::render::render_resource::BindGroupLayout,
        render_device: &bevy::render::renderer::RenderDevice,
        images: &bevy::render::render_asset::RenderAssets<Image>,
        _fallback_image: &bevy::render::texture::FallbackImage,
    ) -> Result<
        bevy::render::render_resource::PreparedBindGroup<Self>,
        bevy::render::render_resource::AsBindGroupError,
    > {
        let grid_image = images
            .get(&self.grid_texture)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        let color_map_image = images
            .get(&self.color_map)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&grid_image.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&color_map_image.texture_view),
                },
            ],
            label: Some("grid_material_bind_group"),
            layout,
        });

        Ok(PreparedBindGroup {
            bind_group,
            bindings: vec![
                OwnedBindingResource::TextureView(grid_image.texture_view.clone()),
                OwnedBindingResource::TextureView(color_map_image.texture_view.clone()),
            ],
            data: (),
        })
    }

    fn bind_group_layout(
        render_device: &bevy::render::renderer::RenderDevice,
    ) -> bevy::render::render_resource::BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
            label: Some("grid_material_bind_group_layout"),
        })
    }
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GridMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut grid_image = Image::new_fill(
        Extent3d {
            height: 100,
            width: 100,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8],
        TextureFormat::R8Uint,
    );
    let pixel_info = grid_image.texture_descriptor.format.pixel_info();
    info!(
        "Pixel info: size {}, num_components {}",
        pixel_info.type_size, pixel_info.num_components
    );
    grid_image.texture_descriptor.usage = TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
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
        TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
    color_map_image.texture_descriptor.label = Some("color_map_texture");

    let grid_texture = images.add(grid_image);
    let color_map_image = images.add(color_map_image);
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
        transform: Transform::default().with_scale(Vec3::splat(100.)),
        material: materials.add(GridMaterial {
            grid_texture: grid_texture.clone(),
            color_map: color_map_image.clone(),
        }),
        ..default()
    });

    let board = {
        let mut grid = Grid::new(100, 100);
        info!("Setting initial grid state");
        grid.slice_mut(s![10..20, 1]).fill(Material::Sand);
        grid.slice_mut(s![0..99, 99]).fill(Material::Bedrock);
        grid
    };
    commands.spawn(FallingSand::new_from_board(
        &board,
        grid_texture,
        color_map_image,
    ));
}
