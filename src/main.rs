#[macro_use]
extern crate enum_map;

use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::camera::Camera;
use enum_map::EnumMap;
use ndarray::{prelude::*, Zip};
use std::ops::{Deref, DerefMut};

#[derive(Debug, TypeUuid, Clone)]
#[uuid = "3e6c203c-76a0-4acc-a812-8d48ee685e61"]
struct Board(pub Array2<Material>);

impl Board {
    pub fn new(width: usize, height: usize) -> Board {
        Board(Array2::from_elem((width, height), Material::Air))
    }
}

impl Deref for Board {
    type Target = Array2<Material>;
    fn deref(&self) -> &Array2<Material> {
        &self.0
    }
}

impl DerefMut for Board {
    fn deref_mut(&mut self) -> &mut Array2<Material> {
        &mut self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
enum Material {
    Bedrock,
    Air,
    Sand,
    Water,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
enum Phase {
    Solid,
    Liquid,
    Gas,
}

struct ToolState {
    draw_type: Material,
}

struct MaterialDensities(EnumMap<Material, u32>);
struct MaterialPhases(EnumMap<Material, Phase>);

pub struct FallingSand {
    cells: Board,
    scratch: Board,
    texture: Handle<Texture>,
    odd_timestep: bool,
}

impl FallingSand {
    fn new(width: usize, height: usize, texture: Handle<Texture>) -> Self {
        FallingSand {
            cells: Board::new(width, height),
            scratch: Board::new(width, height),
            texture,
            odd_timestep: false,
        }
    }

    fn new_from_board(board: &Board, texture: Handle<Texture>) -> Self {
        let width = board.nrows();
        let height = board.ncols();
        FallingSand {
            cells: board.clone(),
            scratch: Board::new(width, height),
            texture,
            odd_timestep: false,
        }
    }

    fn gravity(
        &mut self,
        material_densities: &MaterialDensities,
        material_phases: &MaterialPhases,
    ) {
        // TODO move this function to gravity_system, without tripping up the borrow checker
        let (source, target) = {
            if !self.odd_timestep {
                (self.cells.view(), self.scratch.view_mut())
            } else {
                (
                    self.cells.slice(s![1..-1, 1..-1]),
                    self.scratch.slice_mut(s![1..-1, 1..-1]),
                )
            }
        };

        // Method from: https://ir.cwi.nl/pub/4545

        Zip::from(target.reversed_axes().exact_chunks_mut((2, 2)))
            .and(source.reversed_axes().exact_chunks((2, 2)))
            .for_each(|mut s, neigh| {
                if neigh.iter().all(|material| *material == Material::Air) {
                    s.assign(&neigh);
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Air, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Air],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Air, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Air, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Sand],
                        [Material::Air, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Air, Material::Sand],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Sand, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Sand, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Air, Material::Sand],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Sand],
                        [Material::Air, Material::Sand],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Air, Material::Sand],
                        [Material::Sand, Material::Sand],
                    ]));
                } else if neigh
                    == arr2(&[
                        [Material::Sand, Material::Sand],
                        [Material::Sand, Material::Air],
                    ])
                {
                    s.assign(&arr2(&[
                        [Material::Sand, Material::Air],
                        [Material::Sand, Material::Sand],
                    ]));
                } else {
                    s.assign(&neigh);
                }
            });
        self.cells.assign(&self.scratch);
        self.odd_timestep = !self.odd_timestep;
    }
}

fn main() {
    let mut app = App::build();

    app.insert_resource(WindowDescriptor {
        mode: bevy::window::WindowMode::BorderlessFullscreen,
        ..Default::default()
    });

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(DefaultPlugins);

    #[cfg(target_arch = "wasm32")]
    app.add_plugins(bevy_webgl2::DefaultPlugins);

    app.add_startup_system(setup.system())
        .add_system(gravity_system.system())
        .add_system(grid_system.system())
        .add_system(draw_tool_system.system())
        .add_system(switch_tool_system.system())
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
        .insert_resource(ClearColor(Color::WHITE))
        .insert_resource(ToolState {
            draw_type: Material::Sand,
        })
        .run();
}

fn setup(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    dbg!("setting up the board");

    let a = {
        let mut a = Board::new(100, 100);
        a.slice_mut(s![10..20, 1]).fill(Material::Sand);
        a.slice_mut(s![0..99, 99]).fill(Material::Bedrock);
        a
    };

    let width = a.nrows();
    let height = a.ncols();

    let texture = textures.add(Texture::new_fill(
        bevy::render::texture::Extent3d::new(height as u32, width as u32, 1u32),
        bevy::render::texture::TextureDimension::D2,
        &[0u8, 0u8, 0u8, 255u8],
        bevy::render::texture::TextureFormat::Rgba8UnormSrgb,
    ));

    let material = ColorMaterial::texture(texture.clone());
    //
    let scale = 8.0;

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(material),
            sprite: Sprite {
                size: Vec2::new(width as f32, height as f32),
                ..Default::default()
            },
            transform: Transform::from_scale(Vec3::new(scale, scale, 1.0)),
            ..Default::default()
        })
        .insert(FallingSand::new_from_board(&a, texture));
}

fn gravity_system(
    mut grid_query: Query<&mut FallingSand>,
    material_densities: Res<MaterialDensities>,
    material_phases: Res<MaterialPhases>,
) {
    for mut grid in grid_query.iter_mut() {
        grid.gravity(&material_densities, &material_phases);
    }
}

fn grid_system(grid_query: Query<&FallingSand>, mut textures: ResMut<Assets<Texture>>) {
    for grid in grid_query.iter() {
        if let Some(texture) = textures.get_mut(&grid.texture) {
            texture.data = grid
                .cells
                .t()
                .iter()
                .flat_map(|cell| match *cell {
                    Material::Sand => [244, 215, 21, 255u8],
                    Material::Water => [255, 0, 0, 255u8],
                    Material::Bedrock => [77, 77, 77, 255u8],
                    _ => [255u8, 255u8, 255u8, 255u8],
                })
                .collect();
        }
    }
}

fn switch_tool_system(mut tool_state: ResMut<ToolState>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.pressed(KeyCode::Key1) {
        tool_state.draw_type = Material::Sand;
    }
    if keyboard_input.pressed(KeyCode::Key2) {
        tool_state.draw_type = Material::Water;
    }
}

fn draw_tool_system(
    windows: Res<Windows>,
    mut grid_query: Query<(&mut FallingSand, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    camera_transforms: Query<&GlobalTransform, With<Camera>>,
    tool_state: Res<ToolState>,
) {
    let maybe_window: Option<Vec3> = windows.get_primary().and_then(|window| {
        window.cursor_position().map(|cursor_position| {
            Vec3::new(
                cursor_position.x - window.width() / 2.0,
                cursor_position.y - window.height() / 2.0,
                0.0,
            )
        })
    });
    let cursor_position = if let Some(window) = maybe_window {
        window
    } else {
        return;
    };

    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }

    for camera_transform in camera_transforms.iter() {
        for (mut grid, grid_transform) in grid_query.iter_mut() {
            let tile_position = get_tile_position_under_cursor(
                cursor_position,
                camera_transform,
                grid_transform,
                (grid.cells.nrows(), grid.cells.ncols()),
                8,
            );
            if tile_position.0 > 0 && tile_position.1 > 0 {
                if let Some(cell) = grid
                    .cells
                    .get_mut((tile_position.0 as usize, tile_position.1 as usize))
                {
                    if *cell != Material::Bedrock {
                        *cell = tool_state.draw_type;
                    }
                }
            }
        }
    }
}

fn get_tile_position_under_cursor(
    cursor_position: Vec3,
    camera_transform: &GlobalTransform,
    tilemap_transform: &GlobalTransform,
    grid_size: (usize, usize),
    tile_size: u32,
) -> (i32, i32) {
    let translation = (camera_transform.mul_vec3(cursor_position)) - tilemap_transform.translation;
    let point_x = translation.x / tile_size as f32;
    let point_y = translation.y / tile_size as f32;
    (
        point_x.floor() as i32 + (grid_size.0 / 2) as i32,
        -point_y.floor() as i32 + (grid_size.1 / 2) as i32,
    )
}
