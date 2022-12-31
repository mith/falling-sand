#[macro_use]
extern crate enum_map;

use bevy::render::camera::Camera;

use bevy::prelude::*;

use falling_sand::FallingSandPlugin;
use margolus::Margulos;

use types::{MaterialDensities, ToolState};

use crate::falling_sand::FallingSand;

use crate::{
    margolus::gravity_system,
    types::{Material, MaterialPhases, Phase},
};

mod falling_sand;
mod grid;
mod margolus;
mod types;

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                window: WindowDescriptor {
                    // mode: bevy::window::WindowMode::BorderlessFullscreen,
                    ..Default::default()
                },
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    );

    app.add_plugin(FallingSandPlugin);

    app.add_startup_system(setup)
        .add_system(gravity_system)
        .add_system(draw_tool_system)
        .add_system(switch_tool_system)
        .insert_resource(Margulos::default())
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

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
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
    let translation =
        camera_transform.translation() + cursor_position - tilemap_transform.translation();
    let point_x = translation.x / tile_size as f32;
    let point_y = translation.y / tile_size as f32;
    (
        point_x.floor() as i32 + (grid_size.0 / 2) as i32,
        -point_y.floor() as i32 + (grid_size.1 / 2) as i32,
    )
}
