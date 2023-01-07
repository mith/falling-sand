#[macro_use]
extern crate enum_map;

use bevy::{input::mouse::MouseWheel, prelude::*, render::camera::Camera};
use bevy_inspector_egui::InspectorPlugin;
use falling_sand::FallingSandPhase;
use grid::FallingSandGrid;
use margolus::MargolusSettings;

use crate::{
    falling_sand::{FallingSandPlugin, FallingSandSettings},
    types::{Material, ToolState},
};

mod double_buffered;
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

    app.add_plugin(FallingSandPlugin::default())
        .add_plugin(InspectorPlugin::<MargolusSettings>::new());

    app.add_startup_system(setup)
        .add_system(draw_tool_system.after(FallingSandPhase))
        .add_system(switch_tool_system)
        .add_system(camera_zoom)
        .add_system(move_camera_mouse)
        .insert_resource(CameraSettings {
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
    commands.spawn((Camera2dBundle::default(), DragState::default()));
}

#[derive(Resource)]
pub struct CameraSettings {
    pub zoom_speed: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

fn camera_zoom(
    mut query: Query<(&mut Transform, &mut OrthographicProjection), With<Camera>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    camera_settings: Res<CameraSettings>,
    windows: Res<Windows>,
) {
    if let Some(window) = windows.get_primary() {
        for event in mouse_wheel_events.iter() {
            for (mut transform, mut ortho) in query.iter_mut() {
                if let Some(cursor_pos) = window.cursor_position() {
                    let old_scale = ortho.scale;
                    let mut zoom_change = ortho.scale * event.y * camera_settings.zoom_speed;
                    ortho.scale -= zoom_change;

                    if ortho.scale < camera_settings.min_zoom {
                        ortho.scale = camera_settings.min_zoom;
                        zoom_change = old_scale - ortho.scale;
                    }

                    // Move the camera toward the cursor position to keep the current object
                    // underneath it.
                    let from_center =
                        cursor_pos - Vec2::new(window.width() / 2., window.height() / 2.);

                    let scaled_move = from_center * event.y * zoom_change.abs();
                    transform.translation += Vec3::new(scaled_move.x, scaled_move.y, 0.);
                }
            }
        }
    }
}

#[derive(Default, Component)]
pub struct DragState {
    drag_start: Option<(Vec2, Vec3)>,
}

pub fn move_camera_mouse(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut query: Query<(&mut Transform, &mut OrthographicProjection, &mut DragState), With<Camera>>,
) {
    if let Some(window) = windows.get_primary() {
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
                    transform.translation = cam_start - Vec3::new(diff.x, diff.y, 0.) * ortho.scale;
                    transform.translation.z = z;
                }
            }
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
    mut grid_query: Query<(&mut FallingSandGrid, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    camera_transforms: Query<(&GlobalTransform, &OrthographicProjection), With<Camera>>,
    tool_state: Res<ToolState>,
    falling_sand_settings: Res<FallingSandSettings>,
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

    for (camera_transform, projection) in camera_transforms.iter() {
        for (mut grid, grid_transform) in grid_query.iter_mut() {
            let tile_position = get_tile_position_under_cursor(
                cursor_position,
                camera_transform,
                projection.scale,
                grid_transform,
                grid.size(),
                falling_sand_settings.tile_size,
            );
            if tile_position.0 > 0 && tile_position.1 > 0 {
                if let Some(cell) = grid
                    .0
                    .target_mut()
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
    camera_scale: f32,
    tilemap_transform: &GlobalTransform,
    grid_size: (usize, usize),
    tile_size: u32,
) -> (i32, i32) {
    let translation = camera_transform.transform_point(cursor_position * camera_scale);
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
