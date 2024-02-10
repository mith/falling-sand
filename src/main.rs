#[macro_use]
extern crate enum_map;

use std::{fs::File, io::Write};

use bevy_inspector_egui::quick::WorldInspectorPlugin;
use cursor_world_position::{CursorWorldPosition, CursorWorldPositionPlugin};
use nix::{
    libc::{self},
    sys::wait::waitpid,
    unistd::{fork, write, ForkResult},
};

use crate::{
    falling_sand::{FallingSandPlugin, FallingSandSettings},
    types::{Material, ToolState},
};
use bevy::{
    input::mouse::MouseWheel, prelude::*, render::camera::Camera, tasks::IoTaskPool, utils::HashMap,
};
use falling_sand::{FallingSandGrid, FallingSandSet};
use time_control::TimeControlPlugin;

mod cursor_world_position;
mod falling_sand;
mod movement;
mod particle_grid;
mod time_control;
mod types;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        WorldInspectorPlugin::default(),
        CursorWorldPositionPlugin,
        FallingSandPlugin::default(),
        TimeControlPlugin,
    ));

    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            draw_tool_system.after(FallingSandSet),
            switch_tool_system,
            camera_zoom,
            move_camera_mouse,
            save_scene_fork,
        ),
    );
    app.insert_resource(CameraSettings {
        zoom_speed: 0.1,
        min_zoom: 0.01,
        max_zoom: 10.0,
    })
    .insert_resource(ClearColor(Color::WHITE))
    .insert_resource(ToolState {
        draw_type: Material::Sand,
    })
    .run();
}

fn setup(mut commands: Commands) {
    let mut camera2d_bundle = Camera2dBundle::default();
    camera2d_bundle.projection.scale = 0.1;
    commands.spawn((
        Name::new("Main camera"),
        camera2d_bundle,
        DragState::default(),
    ));
}

fn save_scene_fork(
    world: &World,
    type_registry: Res<AppTypeRegistry>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if !keyboard_input.just_pressed(KeyCode::F5) {
        return;
    }

    // Fork the process to save the scene in a child process.
    // The memory pages are marked as copy-on-write, so the child process
    // can save the gamestate while the main process is still running.
    // This way the game simulation is not paused while saving.
    // It works for Factorio, so it should work here too
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("Forked child save process with pid {}", child);
            // spawn a task to wait for the child process to finish
            IoTaskPool::get()
                .spawn(async move {
                    // wait for the child process to finish
                    waitpid(child, None).expect("Failed to wait for child process");
                    println!("Child save process finished");
                })
                .detach();
            println!("Continuing simulation");
        }
        Ok(ForkResult::Child) => {
            let scene = DynamicScene::from_world(world);
            let serialized_scene = scene.serialize_ron(&type_registry).unwrap();
            File::create("test_save.ron")
                .and_then(|mut file| file.write(serialized_scene.as_bytes()))
                .expect("Error while saving scene to file");
            write(libc::STDOUT_FILENO, "Saved scene to file\n".as_bytes()).ok();
            // exit the child process
            unsafe { libc::_exit(0) };
        }
        Err(e) => panic!("Failed to fork: {}", e),
    }
}

#[derive(Resource)]
pub struct CameraSettings {
    pub zoom_speed: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

fn camera_zoom(
    mut query: Query<&mut OrthographicProjection>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    camera_settings: Res<CameraSettings>,
) {
    for mut projection in &mut query {
        for event in mouse_wheel_events.read() {
            projection.scale -= projection.scale * event.y * camera_settings.zoom_speed;
            projection.scale = projection
                .scale
                .clamp(camera_settings.min_zoom, camera_settings.max_zoom);
        }
    }
}

#[derive(Default, Component)]
pub struct DragState {
    drag_start: Option<(Vec2, Vec3)>,
}

pub fn move_camera_mouse(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Query<&Window>,
    mut query: Query<(&mut Transform, &mut OrthographicProjection, &mut DragState), With<Camera>>,
) {
    if let Ok(window) = windows.get_single() {
        for (mut transform, ortho, mut state) in query.iter_mut() {
            if mouse_button_input.just_pressed(MouseButton::Middle) {
                if let Some(cursor_pos) = window.cursor_position() {
                    state.drag_start = Some((cursor_pos, transform.translation));
                }
            }

            if mouse_button_input.just_released(MouseButton::Middle) {
                state.drag_start = None;
            }

            if let Some((drag_start, cam_start)) = state.drag_start {
                if let Some(cursor) = window.cursor_position() {
                    let diff = cursor - drag_start;
                    let z = transform.translation.z;
                    transform.translation =
                        cam_start - Vec3::new(diff.x, -diff.y, 0.) * ortho.scale;
                    transform.translation.z = z;
                }
            }
        }
    }
}

fn switch_tool_system(mut tool_state: ResMut<ToolState>, keyboard_input: Res<Input<KeyCode>>) {
    let material_keys = HashMap::from_iter([
        (KeyCode::Key1, Material::Sand),
        (KeyCode::Key2, Material::Water),
    ]);
    if let Some(material) = keyboard_input
        .get_pressed()
        .find_map(|p| material_keys.get(p))
    {
        tool_state.draw_type = *material;
    }
}

fn draw_tool_system(
    mut grid_query: Query<(&mut FallingSandGrid, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    tool_state: Res<ToolState>,
    falling_sand_settings: Res<FallingSandSettings>,
    cursor_world_position: Res<CursorWorldPosition>,
) {
    if !mouse_button_input.just_pressed(MouseButton::Left) {
        return;
    }

    for (mut grid, _grid_transform) in grid_query.iter_mut() {
        let Some(tile_position) = get_tile_at_world_position(
            cursor_world_position.position(),
            grid.size(),
            falling_sand_settings.tile_size,
        ) else {
            continue;
        };
        if let Some(cell) = grid
            .particles
            .array_mut()
            .get_mut((tile_position.x as usize, tile_position.y as usize))
        {
            if cell.material != Material::Bedrock {
                cell.material = tool_state.draw_type;
            }
        }
    }
}

fn get_tile_at_world_position(
    cursor_position: Vec2,
    grid_size: IVec2,
    tile_size: u32,
) -> Option<IVec2> {
    let x = (cursor_position.x / tile_size as f32 + grid_size.x as f32 / 2.0) as i32;
    let y = (cursor_position.y / tile_size as f32 + grid_size.y as f32 / 2.0) as i32;
    if x >= 0 && x < grid_size.x && y >= 0 && y < grid_size.y {
        Some(IVec2::new(x, y))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_tile_position_under_cursor() {
        let grid_size = IVec2::new(10, 10);
        let tile_size = 1;

        let cursor_position = Vec2::new(0., 0.);
        let tile_position = get_tile_at_world_position(cursor_position, grid_size, tile_size);
        assert_eq!(tile_position, Some(IVec2::new(5, 5)));

        let cursor_position = Vec2::new(4.5, 4.5);
        let tile_position = get_tile_at_world_position(cursor_position, grid_size, tile_size);
        assert_eq!(tile_position, Some(IVec2::new(9, 9)));

        let cursor_position = Vec2::new(-4.5, 4.5);
        let tile_position = get_tile_at_world_position(cursor_position, grid_size, tile_size);
        assert_eq!(tile_position, Some(IVec2::new(0, 9)));
    }
}
