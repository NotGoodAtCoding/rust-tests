use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    InWorkshop,
    Paused,
    GameOverMooseRampage,
}

/// Helper function determining whether game simulation, timers, and animations should be paused.
/// Returns true if the options/pause menu is open or the game state is `GameState::Paused`.
pub fn is_game_paused(
    menu_state: Option<&crate::elf_game::ui::MenuState>,
    game_state: Option<&State<GameState>>,
) -> bool {
    if let Some(menu) = menu_state {
        if menu.is_open {
            return true;
        }
    }
    if let Some(state) = game_state {
        if *state.get() == GameState::Paused {
            return true;
        }
    }
    false
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
