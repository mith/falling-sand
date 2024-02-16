use bevy::{
    app::{App, Update},
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        schedule::{apply_deferred, IntoSystemConfigs},
        system::{Commands, Local, Query, Res, ResMut, Resource},
    },
    input::{keyboard::KeyCode, mouse::MouseButton, Input},
    log::info,
    math::{IVec2, Vec2},
    reflect::Reflect,
    time::{Time, Timer},
    utils::HashMap,
};
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use itertools::Itertools;
use line_drawing::Bresenham;

use crate::{
    chunk::Chunk,
    cursor_world_position::CursorWorldPosition,
    falling_sand::{ChunkCreationParams, FallingSandPostSet, FallingSandSet, FallingSandSettings},
    falling_sand_grid::{
        tile_pos_to_chunk_pos, update_chunk_positions, ChunkPositions, FallingSandGridQuery,
    },
    material::Material,
};

pub struct DrawToolPlugin;

impl bevy::app::Plugin for DrawToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorTilePosition>()
            .add_plugins(ResourceInspectorPlugin::<CursorTilePosition>::default())
            .add_systems(
                Update,
                (
                    cursor_tile_position_system,
                    update_chunk_positions,
                    switch_tool_system,
                    calculate_stroke,
                    apply_deferred,
                    spawn_chunk_under_stroke,
                    apply_deferred,
                    update_chunk_positions,
                    draw_particles,
                )
                    .chain()
                    .after(FallingSandSet)
                    .before(FallingSandPostSet),
            );
    }
}

#[derive(Resource)]
pub struct ToolState {
    pub draw_type: Material,
}

fn switch_tool_system(mut tool_state: ResMut<ToolState>, keyboard_input: Res<Input<KeyCode>>) {
    let material_keys = HashMap::from_iter([
        (KeyCode::Key1, Material::Sand),
        (KeyCode::Key2, Material::Water),
        (KeyCode::Key3, Material::Fire),
        (KeyCode::Key4, Material::Wood),
        (KeyCode::Key5, Material::Bedrock),
        (KeyCode::Key6, Material::Oil),
    ]);
    if let Some(material) = keyboard_input
        .get_pressed()
        .find_map(|p| material_keys.get(p))
    {
        tool_state.draw_type = *material;
    }
}

struct DrawTimer(Timer);

impl Default for DrawTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.04, bevy::time::TimerMode::Repeating))
    }
}

#[derive(Default)]
struct LastDrawPosition(Option<IVec2>);

#[derive(Component, Debug, Reflect)]
struct Stroke(Vec<IVec2>);

fn calculate_stroke(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    cursor_tile_position: Res<CursorTilePosition>,
    mut timer: Local<DrawTimer>,
    time: Res<Time>,
    mut last_draw_position: Local<LastDrawPosition>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        last_draw_position.0 = None;
        return;
    }

    let current_tile_pos = cursor_tile_position.0;

    if timer.0.tick(time.delta()).just_finished() || cursor_tile_position.is_changed() {
        let start_pos = last_draw_position.0.unwrap_or(current_tile_pos);
        if start_pos != current_tile_pos {
            let bresenham = Bresenham::new(start_pos.into(), current_tile_pos.into());
            commands.spawn(Stroke(bresenham.map(Into::into).collect()));
        } else {
            commands.spawn(Stroke(vec![current_tile_pos]));
        }
        last_draw_position.0 = Some(current_tile_pos);
    }
}

#[derive(Resource, Default, Reflect)]
struct CursorTilePosition(pub IVec2);

fn cursor_tile_position_system(
    cursor_world_position: Res<CursorWorldPosition>,
    falling_sand_settings: Res<FallingSandSettings>,
    mut cursor_tile_position: ResMut<CursorTilePosition>,
    grid_query: Query<&Chunk>,
) {
    for grid in &grid_query {
        let tile_position = get_tile_at_world_position(
            cursor_world_position.position(),
            grid.0.read().unwrap().size(),
            falling_sand_settings.tile_size,
        );

        if tile_position != cursor_tile_position.0 {
            cursor_tile_position.0 = tile_position;
        }
    }
}

fn get_tile_at_world_position(world_position: Vec2, grid_size: IVec2, tile_size: u32) -> IVec2 {
    let x = (world_position.x / tile_size as f32 + grid_size.x as f32 / 2.0) as i32;
    let y = (world_position.y / tile_size as f32 + grid_size.y as f32 / 2.0) as i32;
    IVec2::new(x, y)
}

fn spawn_chunk_under_stroke(
    mut chunk_creation_params: ChunkCreationParams,
    chunk_positions: Res<ChunkPositions>,
    stroke_query: Query<&Stroke>,
) {
    for stroke in stroke_query.iter() {
        let unspawned_stroke_chunk_positions = stroke
            .0
            .iter()
            .map(|pos| tile_pos_to_chunk_pos(pos.x, pos.y))
            .unique()
            .filter(|pos| !chunk_positions.contains(pos));
        chunk_creation_params.spawn_chunks(unspawned_stroke_chunk_positions);
    }
}

fn draw_particles(
    mut grid: FallingSandGridQuery,
    stroke_query: Query<(Entity, &Stroke)>,
    tool_state: Res<ToolState>,
    mut commands: Commands,
) {
    for (entity, stroke) in stroke_query.iter() {
        for pos in &stroke.0 {
            grid.set_particle(pos.x, pos.y, tool_state.draw_type);
        }
        commands.entity(entity).despawn();
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
        assert_eq!(tile_position, IVec2::new(5, 5));

        let cursor_position = Vec2::new(4.5, 4.5);
        let tile_position = get_tile_at_world_position(cursor_position, grid_size, tile_size);
        assert_eq!(tile_position, IVec2::new(9, 9));

        let cursor_position = Vec2::new(-4.5, 4.5);
        let tile_position = get_tile_at_world_position(cursor_position, grid_size, tile_size);
        assert_eq!(tile_position, IVec2::new(0, 9));
    }
}
