use bevy::{
    app::{App, Plugin, Update},
    ecs::system::{Res, ResMut},
    input::{keyboard::KeyCode, Input},
    time::{Time, Virtual},
};
use bevy_egui::{
    egui::{self},
    EguiContexts,
};

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, time_control);
    }
}

fn time_control(
    mut egui_contexts: EguiContexts,
    mut time: ResMut<Time<Virtual>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    egui::Window::new("Time").show(egui_contexts.ctx_mut(), |ui| {
        let pause_text = if time.is_paused() { "Resume" } else { "Pause" };
        if ui.button(pause_text).clicked() || keyboard_input.just_pressed(KeyCode::Space) {
            if time.is_paused() {
                time.unpause();
            } else {
                time.pause();
            }
        }
    });
}
