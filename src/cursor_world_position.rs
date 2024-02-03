use bevy::{
    app::{App, Plugin, Update},
    ecs::system::{Query, ResMut, Resource},
    gizmos::gizmos::Gizmos,
    math::Vec2,
    reflect::Reflect,
    render::{camera::Camera, color::Color},
    transform::components::GlobalTransform,
    window::Window,
};
use bevy_inspector_egui::quick::ResourceInspectorPlugin;

pub struct CursorWorldPositionPlugin;

impl Plugin for CursorWorldPositionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CursorWorldPosition>()
            .insert_resource(CursorWorldPosition(Vec2::ZERO))
            .add_plugins(ResourceInspectorPlugin::<CursorWorldPosition>::default())
            .add_systems(Update, update_cursor_world_position);
    }
}

#[derive(Resource, Reflect)]
pub struct CursorWorldPosition(Vec2);

impl CursorWorldPosition {
    pub fn position(&self) -> Vec2 {
        self.0
    }
}

fn update_cursor_world_position(
    window: Query<&Window>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    mut cursor_world_position_query: ResMut<CursorWorldPosition>,
    mut gizmos: Gizmos,
) {
    let Some(cursor_position) = window
        .get_single()
        .ok()
        .and_then(|window| window.cursor_position())
    else {
        return;
    };

    let Ok((camera_transform, camera)) = camera_query.get_single() else {
        return;
    };

    let Some(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    cursor_world_position_query.0 = point;

    gizmos.circle_2d(point, 0.5, Color::GREEN);
}
