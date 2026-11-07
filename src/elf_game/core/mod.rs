use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    InWorkshop,
    Paused,
    GameOverMooseRampage,
}

#[derive(Resource, Debug, Clone)]
pub struct WorkshopQuota {
    pub presents_delivered: u32,
    pub target_quota: u32,
    pub tier: u32,
}

impl Default for WorkshopQuota {
    fn default() -> Self {
        Self {
            presents_delivered: 0,
            target_quota: 5,
            tier: 1,
        }
    }
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::state::app::StatesPlugin>() {
            app.add_plugins(bevy::state::app::StatesPlugin);
        }
        app.init_state::<GameState>()
            .init_resource::<WorkshopQuota>();
    }
}
