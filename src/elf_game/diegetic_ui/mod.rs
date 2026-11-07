use bevy::prelude::*;

pub mod monitor;

pub struct DiegeticUiPlugin;

impl Plugin for DiegeticUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, monitor::spawn_diegetic_screens_and_signs)
            .add_systems(Update, monitor::update_diegetic_monitor_text);
    }
}
