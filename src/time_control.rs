use bevy::{
    app::{App, FixedUpdate, Plugin, Update},
    ecs::{
        schedule::Stepping,
        system::{Res, ResMut},
    },
    input::{keyboard::KeyCode, ButtonInput},
    log::{debug, info},
};

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        let mut stepping = Stepping::default();
        stepping.add_schedule(FixedUpdate);
        app.add_systems(Update, handle_input)
            .insert_resource(stepping);
    }
}

fn handle_input(keyboard_input: Res<ButtonInput<KeyCode>>, mut stepping: ResMut<Stepping>) {
    if keyboard_input.just_pressed(KeyCode::Slash) {
        info!("{:#?}", stepping);
    }
    // grave key to toggle stepping mode for the FixedUpdate schedule
    if keyboard_input.just_pressed(KeyCode::Backquote) {
        if stepping.is_enabled() {
            stepping.disable();
            debug!("disabled stepping");
        } else {
            stepping.enable();
            debug!("enabled stepping");
        }
    }

    if !stepping.is_enabled() {
        return;
    }

    // space key will step the remainder of this frame
    if keyboard_input.just_pressed(KeyCode::Space) {
        debug!("continue");
        stepping.continue_frame();
    } else if keyboard_input.just_pressed(KeyCode::KeyS) {
        debug!("stepping frame");
        stepping.step_frame();
    }
}
