use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        schedule::{apply_deferred, SystemSet},
        system::{Commands, Query, Resource},
    },
    ui::Interaction,
};

pub struct HoveringUiPlugin;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HoveringUiSet;

impl Plugin for HoveringUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (hovering_ui, apply_deferred));
    }
}

#[derive(Resource, Debug)]
pub struct UiFocused;

fn hovering_ui(mut commands: Commands, interaction_query: Query<&Interaction>) {
    let hovering = interaction_query
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Hovered | Interaction::Pressed));

    if hovering {
        commands.insert_resource(UiFocused);
    } else {
        commands.remove_resource::<UiFocused>();
    }
}
