use bevy::{
    app::{App, Plugin, Update},
    ecs::system::ResMut,
    time::{Time, Virtual},
};
use bevy_egui::{egui, EguiContexts};

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, time_control);
    }
}

fn time_control(mut egui_contexts: EguiContexts, mut time: ResMut<Time<Virtual>>) {
    egui::Window::new("Time").show(egui_contexts.ctx_mut(), |ui| {
        let pause_text = if time.is_paused() { "Resume" } else { "Pause" };
        if ui.button(pause_text).clicked() {
            if time.is_paused() {
                time.unpause();
            } else {
                time.pause();
            }
        }
    });
}
