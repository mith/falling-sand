use bevy::{
    app::{App, Update},
    ecs::{
        change_detection::DetectChanges,
        schedule::IntoSystemConfigs,
        system::{Local, Query, Res, ResMut, Resource},
    },
    input::{keyboard::KeyCode, mouse::MouseButton, Input},
    math::{IVec2, Vec2},
    time::{Time, Timer},
    transform::components::GlobalTransform,
    utils::HashMap,
};

use crate::{
    cursor_world_position::CursorWorldPosition,
    falling_sand::{FallingSandGrid, FallingSandSet, FallingSandSettings},
    material::Material,
};

pub struct DrawToolPlugin;

impl bevy::app::Plugin for DrawToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorTilePosition>().add_systems(
            Update,
            (
                cursor_tile_position_system,
                switch_tool_system,
                draw_tool_system,
            )
                .chain()
                .after(FallingSandSet),
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

fn draw_tool_system(
    mut grid_query: Query<(&mut FallingSandGrid, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    tool_state: Res<ToolState>,
    cursor_tile_position: Res<CursorTilePosition>,
    mut timer: Local<DrawTimer>,
    time: Res<Time>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }

    if timer.0.tick(time.delta()).just_finished() || cursor_tile_position.is_changed() {
        for (mut grid, _grid_transform) in grid_query.iter_mut() {
            if let Some(cell) = cursor_tile_position.0.and_then(|ctp| {
                grid.particles
                    .array_mut()
                    .get_mut((ctp.x as usize, ctp.y as usize))
            }) {
                if cell.material != Material::Bedrock {
                    cell.material = tool_state.draw_type;
                }
            }
        }
    }
}

#[derive(Resource, Default)]
struct CursorTilePosition(Option<IVec2>);

fn cursor_tile_position_system(
    cursor_world_position: Res<CursorWorldPosition>,
    falling_sand_settings: Res<FallingSandSettings>,
    mut cursor_tile_position: ResMut<CursorTilePosition>,
    grid_query: Query<&FallingSandGrid>,
) {
    for grid in &grid_query {
        let tile_position = get_tile_at_world_position(
            cursor_world_position.position(),
            grid.size(),
            falling_sand_settings.tile_size,
        );

        if tile_position != cursor_tile_position.0 {
            cursor_tile_position.0 = tile_position;
        }
    }
}

fn get_tile_at_world_position(
    world_position: Vec2,
    grid_size: IVec2,
    tile_size: u32,
) -> Option<IVec2> {
    let x = (world_position.x / tile_size as f32 + grid_size.x as f32 / 2.0) as i32;
    let y = (world_position.y / tile_size as f32 + grid_size.y as f32 / 2.0) as i32;
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
