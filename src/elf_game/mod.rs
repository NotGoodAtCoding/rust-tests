use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
};
use bevy_rapier3d::prelude::*;

pub mod core;
pub mod player;
pub mod wrapping;
pub mod moose;
pub mod physics_sandbox;
pub mod workshop;
pub mod diegetic_ui;
pub mod ui;

pub struct ElfGamePlugin;

impl Plugin for ElfGamePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<RapierPhysicsPlugin<()>>() {
            app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        }
        if !app.is_plugin_added::<RapierDebugRenderPlugin>() {
            app.add_plugins(RapierDebugRenderPlugin::default().disabled());
        }

        app.add_plugins((
            core::CorePlugin,
            player::PlayerPlugin,
            wrapping::WrappingPlugin,
            moose::MoosePlugin,
            physics_sandbox::PhysicsSandboxPlugin,
            workshop::WorkshopPlugin,
            diegetic_ui::DiegeticUiPlugin,
            ui::ElfUiPlugin,
        ));
    }
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Santa's Workshop: Elf Simulator".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(ElfGamePlugin)
        .run();
}
