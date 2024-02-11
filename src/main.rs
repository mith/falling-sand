#[macro_use]
extern crate enum_map;

use bevy_inspector_egui::quick::WorldInspectorPlugin;
use cursor_world_position::CursorWorldPositionPlugin;
use draw_tool::DrawToolPlugin;

use pan_zoom_camera::{DragState, PanZoomCameraPlugin};

use crate::{draw_tool::ToolState, falling_sand::FallingSandPlugin, material::Material};
use bevy::prelude::*;
use time_control::TimeControlPlugin;

mod chunk;
mod cursor_world_position;
mod draw_tool;
mod falling_sand;
mod fire;
mod material;
mod movement;
mod pan_zoom_camera;
mod particle_grid;
mod react;
mod time_control;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        WorldInspectorPlugin::default(),
        CursorWorldPositionPlugin,
        PanZoomCameraPlugin,
        FallingSandPlugin::default(),
        DrawToolPlugin,
        TimeControlPlugin,
    ));

    app.add_systems(Startup, setup);
    app.insert_resource(ClearColor(Color::WHITE))
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
