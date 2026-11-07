use bevy::prelude::*;

pub mod core;
pub mod player;
pub mod wrapping;
pub mod moose;
pub mod physics_sandbox;
pub mod workshop;
pub mod diegetic_ui;

pub struct ElfGamePlugin;

impl Plugin for ElfGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            core::CorePlugin,
            player::PlayerPlugin,
            wrapping::WrappingPlugin,
            moose::MoosePlugin,
            physics_sandbox::PhysicsSandboxPlugin,
            workshop::WorkshopPlugin,
            diegetic_ui::DiegeticUiPlugin,
        ));
    }
}
