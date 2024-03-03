#[macro_use]
extern crate enum_map;

use cursor_world_position::CursorWorldPositionPlugin;
use draw_tool::DrawToolPlugin;

use hovering_ui::HoveringUiPlugin;
use pan_zoom_camera::{DragState, PanZoomCameraPlugin};

use crate::{draw_tool::ToolState, falling_sand::FallingSandPlugin, material::Material};
use bevy::prelude::*;
use time_control::TimeControlPlugin;

mod active_chunks;
mod chunk;
mod chunk_neighborhood_view;
mod chunk_positions;
mod consts;
mod cursor_world_position;
mod draw_tool;
mod fall;
mod falling_sand;
mod falling_sand_grid;
mod fire;
mod flow;
mod hovering_ui;
mod material;
mod pan_zoom_camera;
mod particle_attributes;
mod particle_grid;
mod process_chunks;
mod reactions;
mod render;
mod time_control;
mod util;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        CursorWorldPositionPlugin,
        PanZoomCameraPlugin,
        FallingSandPlugin::default(),
        HoveringUiPlugin,
        DrawToolPlugin,
        TimeControlPlugin,
    ))
    .insert_resource(ToolState {
        draw_type: Material::Sand,
    })
    .add_systems(Startup, setup)
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
