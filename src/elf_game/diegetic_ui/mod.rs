use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct DiegeticMonitor {
    pub display_id: String,
    pub current_text: String,
}

#[derive(Component, Debug, Clone)]
pub struct DiegeticSign {
    pub message: String,
}

pub struct DiegeticUiPlugin;

impl Plugin for DiegeticUiPlugin {
    fn build(&self, _app: &mut App) {}
}
