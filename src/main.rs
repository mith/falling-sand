use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::camera::Camera;
use ndarray::{prelude::*, Zip};
use std::ops::{Deref, DerefMut};

#[derive(Debug, TypeUuid, Clone)]
#[uuid = "3e6c203c-76a0-4acc-a812-8d48ee685e61"]
struct Board(pub Array2<Cell>);

impl Board {
    pub fn new(width: usize, height: usize) -> Board {
        Board(Array2::from_elem((width, height), Cell::Air))
    }
}

impl Deref for Board {
    type Target = Array2<Cell>;
    fn deref(&self) -> &Array2<Cell> {
        &self.0
    }
}

impl DerefMut for Board {
    fn deref_mut(&mut self) -> &mut Array2<Cell> {
        &mut self.0
    }
}

#[derive(Default)]
struct BoardState {
    setup: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Cell {
    Bedrock,
    Air,
    Sand,
}

pub struct FallingSand {
    cells: Board,
    scratch: Board,
    texture: Handle<Texture>,
}

impl FallingSand {
    fn new(width: usize, height: usize, texture: Handle<Texture>) -> Self {
        FallingSand {
            cells: Board::new(width + 2, height + 2),
            scratch: Board::new(width, height),
            texture,
        }
    }

    fn new_from_board(board: &Board, texture: Handle<Texture>) -> Self {
        let width = board.nrows();
        let height = board.ncols();
        FallingSand {
            cells: board.clone(),
            scratch: Board::new(width - 2, height - 2),
            texture,
        }
    }

    fn iterate(&mut self) {
        self.scratch.view_mut().fill(Cell::Air);

        Zip::from(self.cells.windows((3, 3))).map_assign_into(
            &mut self.scratch.0,
            |neigh| unsafe {
                if *neigh.uget((1, 1)) == Cell::Air && *neigh.uget((1, 0)) == Cell::Sand
                    || *neigh.uget((1, 1)) == Cell::Sand && *neigh.uget((1, 2)) == Cell::Sand
                    || *neigh.uget((1, 1)) == Cell::Sand && *neigh.uget((1, 2)) == Cell::Bedrock
                {
                    Cell::Sand
                } else {
                    Cell::Air
                }
            },
        );

        self.cells.slice_mut(s![1..-1, 1..-1]).assign(&self.scratch);
    }
}

fn main() {
    let mut app = App::build();

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(DefaultPlugins);

    #[cfg(target_arch = "wasm32")]
    app.add_plugins(bevy_webgl2::DefaultPlugins);

    app.add_startup_system(setup.system())
        .insert_resource(BoardState::default())
        .add_system(grid_system.system())
        .add_system(setup_board.system())
        .add_system(draw_tool_system.system())
        .insert_resource(ClearColor(Color::WHITE))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    dbg!("setup camera");
}

fn setup_board(
    mut commands: Commands,
    mut state: ResMut<BoardState>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if state.setup {
        return;
    };
    dbg!("setting up the board");

    let a = {
        let mut a = Board::new(100, 100);
        a.slice_mut(s![10..20, 1]).fill(Cell::Sand);
        a.slice_mut(s![0..99, 99]).fill(Cell::Bedrock);
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

    state.setup = true;
}

fn grid_system(mut grid_query: Query<&mut FallingSand>, mut textures: ResMut<Assets<Texture>>) {
    for mut grid in grid_query.iter_mut() {
        grid.iterate();

        if let Some(texture) = textures.get_mut(&grid.texture) {
            texture.data = grid
                .cells
                .t()
                .iter()
                .flat_map(|cell| match *cell {
                    Cell::Sand => [244, 215, 21, 255u8],
                    Cell::Bedrock => [77, 77, 77, 255u8],
                    _ => [255u8, 255u8, 255u8, 255u8],
                })
                .collect();
        }
    }
}

fn draw_tool_system(
    windows: Res<Windows>,
    mut grid_query: Query<(&mut FallingSand, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    camera_transforms: Query<&GlobalTransform, With<Camera>>,
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
                if let Some(cell) = grid.cells.get_mut((tile_position.0 as usize, tile_position.1 as usize)) {
                    *cell = Cell::Sand;
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
