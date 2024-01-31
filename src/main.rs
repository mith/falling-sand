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

use bevy::{
    input::mouse::MouseWheel, prelude::*, render::camera::Camera, tasks::IoTaskPool, utils::HashMap,
};
use falling_sand::{FallingSandGrid, FallingSandSet};

use crate::{
    falling_sand::{FallingSandPlugin, FallingSandSettings},
    types::{Material, ToolState},
};

mod cursor_world_position;
mod double_buffered;
mod falling_sand;
mod flow;
mod margolus;
mod particle_grid;
mod types;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        WorldInspectorPlugin::default(),
        CursorWorldPositionPlugin,
        FallingSandPlugin::default(),
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
        min_zoom: 0.1,
        max_zoom: 10.0,
    })
    .insert_resource(ClearColor(Color::WHITE))
    .insert_resource(ToolState {
        draw_type: Material::Sand,
    })
    .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Main camera"),
        Camera2dBundle::default(),
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
    camera_transforms: Query<(&GlobalTransform, &OrthographicProjection), With<Camera>>,
    tool_state: Res<ToolState>,
    falling_sand_settings: Res<FallingSandSettings>,
    cursor_world_position: Res<CursorWorldPosition>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }

    for (camera_transform, projection) in camera_transforms.iter() {
        for (mut grid, grid_transform) in grid_query.iter_mut() {
            let tile_position = get_tile_position_under_cursor(
                cursor_world_position.position().extend(0.),
                camera_transform,
                grid.0.even.dim(),
                falling_sand_settings.tile_size,
            );
            if tile_position.0 > 0 && tile_position.1 > 0 {
                if let Some(cell) = grid
                    .0
                    .source_mut()
                    .get_mut((tile_position.0 as usize, tile_position.1 as usize))
                {
                    if cell.material != Material::Bedrock {
                        cell.material = tool_state.draw_type;
                        cell.pressure = 1.0;
                    }
                }
            }
        }
    }
}

fn get_tile_position_under_cursor(
    cursor_position: Vec3,
    camera_transform: &GlobalTransform,
    grid_size: (usize, usize),
    tile_size: u32,
) -> (i32, i32) {
    let translation = camera_transform.transform_point(cursor_position);
    let point_x = translation.x / tile_size as f32;
    let point_y = translation.y / tile_size as f32;
    (
        point_x.floor() as i32 + (grid_size.0 / 2) as i32,
        -point_y.floor() as i32 + (grid_size.1 / 2) as i32,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_tile_position_under_cursor() {
        let camera_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let tilemap_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let grid_size = (10, 10);
        let tile_size = 2;
        let camera_scale = 1.;

        let cursor_position = Vec3::new(0., 0., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (5, 5));

        let cursor_position = Vec3::new(10., 10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (10, 0));

        let cursor_position = Vec3::new(-10., -10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (0, 10));
    }

    #[test]
    fn get_tile_position_under_cursor_translated_camera() {
        let camera_transform = GlobalTransform::from_translation(Vec3::new(10., 10., 0.));
        let tilemap_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let grid_size = (10, 10);
        let tile_size = 2;
        let camera_scale = 1.;

        let cursor_position = Vec3::new(0., 0., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (10, 0));

        let cursor_position = Vec3::new(10., 10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (15, -5));

        let cursor_position = Vec3::new(-10., -10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (5, 5));
    }

    #[test]
    fn get_tile_position_under_cursor_scaled_camera() {
        let camera_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let tilemap_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let grid_size = (10, 10);
        let tile_size = 2;
        let camera_scale = 2.;

        let cursor_position = Vec3::new(0., 0., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (5, 5));

        let cursor_position = Vec3::new(10., 10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (15, -5));

        let cursor_position = Vec3::new(-10., -10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (-5, 15));
    }

    #[test]
    fn get_tile_position_under_cursor_translated_tilemap() {
        let camera_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let tilemap_transform = GlobalTransform::from_translation(Vec3::new(10., 10., 0.));
        let grid_size = (10, 10);
        let tile_size = 2;
        let camera_scale = 1.;

        let cursor_position = Vec3::new(0., 0., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (0, 10));

        let cursor_position = Vec3::new(10., 10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (5, 5));

        let cursor_position = Vec3::new(-10., -10., 0.);
        let tile_position = get_tile_position_under_cursor(
            cursor_position,
            &camera_transform,
            camera_scale,
            &tilemap_transform,
            grid_size,
            tile_size,
        );
        assert_eq!(tile_position, (10, 0));
    }
}
