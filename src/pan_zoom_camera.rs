use bevy::{input::mouse::MouseWheel, prelude::*, render::camera::Camera, window::PrimaryWindow};

pub struct PanZoomCameraPlugin;

impl Plugin for PanZoomCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_systems(Update, (camera_zoom, move_camera_mouse));
    }
}

#[derive(Resource)]
pub struct CameraSettings {
    pub zoom_speed: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        CameraSettings {
            zoom_speed: 0.1,
            min_zoom: 0.01,
            max_zoom: 10.0,
        }
    }
}

fn camera_zoom(
    mut camera_query: Query<(&mut Transform, &mut OrthographicProjection)>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_settings: Res<CameraSettings>,
) {
    if keyboard_input.pressed(KeyCode::ControlLeft) {
        return;
    }
    let Ok(primary_window) = window_query.get_single() else {
        return;
    };
    let Some(cursor_position) = primary_window.cursor_position() else {
        return;
    };

    for event in mouse_wheel_events.read() {
        for (mut transform, mut ortho) in camera_query.iter_mut() {
            let old_scale = ortho.scale;
            let mut zoom_change = ortho.scale * event.y.clamp(-1., 1.) * camera_settings.zoom_speed;
            ortho.scale -= zoom_change;

            if ortho.scale < camera_settings.min_zoom {
                ortho.scale = camera_settings.min_zoom;
                zoom_change = old_scale - ortho.scale;
            } else if ortho.scale > camera_settings.max_zoom {
                ortho.scale = camera_settings.max_zoom;
                zoom_change = old_scale - ortho.scale;
            }

            // Move the camera toward the cursor position to keep the current object
            // underneath it.
            let from_center = cursor_position
                - Vec2::new(primary_window.width() / 2., primary_window.height() / 2.);

            let scaled_move = from_center * event.y.clamp(-1., 1.) * zoom_change.abs();
            transform.translation += Vec3::new(scaled_move.x, -scaled_move.y, 0.);
        }
    }
}

#[derive(Default, Component)]
pub struct DragState {
    drag_start: Option<(Vec2, Vec3)>,
}

pub fn move_camera_mouse(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut camera_query: Query<
        (&mut Transform, &mut OrthographicProjection, &mut DragState),
        With<Camera>,
    >,
) {
    if let Ok(window) = windows.get_single() {
        for (mut transform, ortho, mut state) in camera_query.iter_mut() {
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
