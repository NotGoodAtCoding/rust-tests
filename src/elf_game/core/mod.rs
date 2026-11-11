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

pub const WORLD_GRAVITY: Vec3 = Vec3::new(0.0, -18.0, 0.0);

/// Synchronizes world gravity in Rapier configuration so physics objects feel less floaty and more grounded.
pub fn sync_world_gravity_system(
    mut rapier_configs: Query<&mut bevy_rapier3d::prelude::RapierConfiguration>,
) {
    for mut config in &mut rapier_configs {
        if config.gravity.y > -15.0 {
            config.gravity = WORLD_GRAVITY;
        }
    }
}

/// Automatically ensures all dynamic physics objects have dampening applied so they do not feel floaty.
pub fn apply_physics_damping_system(
    mut commands: Commands,
    q_dynamic_bodies: Query<
        Entity,
        (
            With<bevy_rapier3d::prelude::RigidBody>,
            Without<bevy_rapier3d::prelude::Damping>,
            Without<crate::elf_game::player::controller::ElfPlayer>,
        ),
    >,
) {
    for entity in &q_dynamic_bodies {
        commands.entity(entity).insert(bevy_rapier3d::prelude::Damping {
            linear_damping: 0.8,
            angular_damping: 1.5,
        });
    }
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::state::app::StatesPlugin>() {
            app.add_plugins(bevy::state::app::StatesPlugin);
        }
        app.init_state::<GameState>()
            .init_resource::<WorkshopQuota>()
            .add_systems(Startup, sync_world_gravity_system)
            .add_systems(Update, (sync_world_gravity_system, apply_physics_damping_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_rapier3d::prelude::*;

    #[test]
    fn test_world_gravity_and_physics_damping() {
        let mut app = App::new();
        let rapier_entity = app.world_mut().spawn(RapierConfiguration::new(1.0)).id();
        app.add_systems(Update, (sync_world_gravity_system, apply_physics_damping_system));
        app.update();

        let config = app.world().get::<RapierConfiguration>(rapier_entity).unwrap();
        assert_eq!(config.gravity, WORLD_GRAVITY, "World gravity must be higher (-18.0 m/s^2)");

        let dynamic_cube = app.world_mut().spawn((
            RigidBody::Dynamic,
            Transform::from_xyz(0.0, 1.0, 0.0),
        )).id();
        app.update();
        let damping = app.world().get::<Damping>(dynamic_cube).expect("Dynamic physics objects must be given dampening");
        assert!(damping.linear_damping >= 0.5, "Dynamic object must have linear damping");
    }
}
