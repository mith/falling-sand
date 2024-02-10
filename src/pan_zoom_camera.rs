use bevy::{input::mouse::MouseWheel, prelude::*, render::camera::Camera};

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
