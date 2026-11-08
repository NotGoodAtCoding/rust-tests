use bevy::prelude::*;

pub mod hunger_dial;
pub mod monitor;
pub mod text_renderer;

pub struct DiegeticUiPlugin;

impl Plugin for DiegeticUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                monitor::spawn_diegetic_screens_and_signs,
                hunger_dial::spawn_moose_hunger_dial_and_scale,
            ),
        )
        .add_systems(
            Update,
            (
                monitor::update_diegetic_monitor_text,
                monitor::animate_crt_scanlines,
                hunger_dial::animate_moose_hunger_dial_system,
            ),
        );
    }
}
