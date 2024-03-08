use bevy::{
    app::{App, FixedUpdate, Startup, Update},
    asset::AssetServer,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        query::Changed,
        schedule::{
            apply_deferred,
            common_conditions::{not, resource_exists},
            IntoSystemConfigs, SystemSet,
        },
        system::{Commands, Local, Query, Res, ResMut, Resource},
    },
    hierarchy::BuildChildren,
    input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput},
    math::{IVec2, Vec2},
    prelude::{default, Color},
    reflect::Reflect,
    text::TextStyle,
    time::{Time, Timer},
    ui::{
        node_bundles::{ButtonBundle, NodeBundle, TextBundle},
        Display, Interaction, JustifyContent, Style, UiRect, Val,
    },
    utils::HashMap,
};
use itertools::Itertools;
use line_drawing::Bresenham;

use crate::{
    chunk::Chunk,
    chunk_positions::{update_chunk_positions, ChunkPositions},
    cursor_world_position::CursorWorldPosition,
    falling_sand::{ChunkCreationParams, FallingSandSet, FallingSandSettings},
    falling_sand_grid::FallingSandGridQuery,
    hovering_ui::{HoveringUiSet, UiFocused},
    material::{Material, MaterialColor, MaterialIterator},
    util::tile_pos_to_chunk_pos,
};

pub struct DrawToolPlugin;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct DrawToolSet;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct DrawToolPickerSet;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct DrawToolUpdateSet;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct DrawToolFixedUpdateSet;

impl bevy::app::Plugin for DrawToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorTilePosition>()
            .add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    cursor_tile_position_system,
                    update_chunk_positions,
                    calculate_stroke,
                    apply_deferred,
                    spawn_chunk_under_stroke,
                    apply_deferred,
                    update_chunk_positions,
                )
                    .chain()
                    .run_if(not(resource_exists::<UiFocused>))
                    .before(HoveringUiSet)
                    .in_set(DrawToolUpdateSet)
                    .in_set(DrawToolSet),
            )
            .add_systems(
                FixedUpdate,
                draw_particles
                    .run_if(not(resource_exists::<UiFocused>))
                    .before(FallingSandSet)
                    .in_set(DrawToolFixedUpdateSet),
            )
            .add_systems(
                Update,
                (switch_tool_system, material_button_system)
                    .before(DrawToolUpdateSet)
                    .in_set(DrawToolPickerSet)
                    .in_set(DrawToolSet),
            );
    }
}

#[derive(Component)]
struct MaterialButton(Material);

fn setup_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    material_colors: Res<MaterialColor>,
) {
    commands
        .spawn((
            Interaction::default(),
            NodeBundle {
                style: Style {
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::all(Val::Px(2.)),
                    display: Display::Flex,
                    flex_direction: bevy::ui::FlexDirection::Column,
                    padding: UiRect::all(Val::Px(2.)),
                    margin: UiRect::all(Val::Px(2.)),
                    ..default()
                },
                border_color: (Color::GRAY * 1.8).into(),
                background_color: Color::WHITE.into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            for material in MaterialIterator::new() {
                let material_color = material_colors.0[material];

                let lightness = material_color.l();

                let text_color = if lightness > 0.5 {
                    Color::BLACK
                } else {
                    Color::WHITE
                };

                let border_color = if lightness > 0.5 {
                    material_color * 0.8
                } else {
                    material_color * 1.2
                };

                parent
                    .spawn((
                        MaterialButton(material),
                        ButtonBundle {
                            style: Style {
                                margin: UiRect::all(Val::Px(2.)),
                                padding: UiRect::all(Val::Px(2.)),
                                border: UiRect::all(Val::Px(2.)),
                                ..default()
                            },
                            border_color: border_color.into(),
                            background_color: material_color.into(),
                            ..default()
                        },
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            material.to_string(),
                            TextStyle {
                                font: asset_server.load("fonts/PublicPixel-z84yD.ttf"),
                                color: text_color,
                                ..default()
                            },
                        ));
                    });
            }
        });
}

fn material_button_system(
    interaction_query: Query<(&Interaction, &MaterialButton), Changed<Interaction>>,
    mut tool_state: ResMut<ToolState>,
) {
    for (interaction, material_button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            tool_state.draw_type = material_button.0;
        }
    }
}

#[derive(Resource)]
pub struct ToolState {
    pub draw_type: Material,
}

fn switch_tool_system(
    mut tool_state: ResMut<ToolState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let material_keys = HashMap::from_iter([
        (KeyCode::Digit1, Material::Sand),
        (KeyCode::Digit2, Material::Water),
        (KeyCode::Digit3, Material::Fire),
        (KeyCode::Digit4, Material::Wood),
        (KeyCode::Digit5, Material::Bedrock),
        (KeyCode::Digit6, Material::Oil),
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
    mouse_button_input: Res<ButtonInput<MouseButton>>,
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
            .map(|pos| tile_pos_to_chunk_pos(*pos))
            .unique()
            .filter(|pos| !chunk_positions.contains(*pos));
        chunk_creation_params.spawn_chunks(unspawned_stroke_chunk_positions);
    }
}

fn draw_particles(
    mut grid: FallingSandGridQuery,
    stroke_query: Query<(Entity, &Stroke)>,
    tool_state: Res<ToolState>,
    mut commands: Commands,
) {
    stroke_query.iter().for_each(|(entity, stroke)| {
        stroke.0.iter().for_each(|pos| {
            grid.set_particle(*pos, tool_state.draw_type);
        });

        commands.entity(entity).despawn();
    });
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
