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
pub mod audio;
pub mod assets;
pub mod dialogue;
pub mod persistence;

pub struct ElfGamePlugin;

impl Plugin for ElfGamePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<RapierPhysicsPlugin<()>>() {
            app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        }
        if !app.is_plugin_added::<RapierDebugRenderPlugin>() {
            app.add_plugins(RapierDebugRenderPlugin::default().disabled());
        }
        if !app.is_plugin_added::<bevy::pbr::wireframe::WireframePlugin>() {
            app.add_plugins(bevy::pbr::wireframe::WireframePlugin::default());
        }

        app.add_plugins((
            assets::AssetsPlugin,
            core::CorePlugin,
            player::PlayerPlugin,
            wrapping::WrappingPlugin,
            moose::MoosePlugin,
            physics_sandbox::PhysicsSandboxPlugin,
            workshop::WorkshopPlugin,
            diegetic_ui::DiegeticUiPlugin,
            ui::ElfUiPlugin,
            dialogue::ElfDialoguePlugin,
            audio::ElfAudioPlugin,
            persistence::PersistencePlugin,
        ));
    }
}

pub fn main() {
    audio::configure_alsa_environment();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Santa's Workshop: Elf Simulator".into(),
                        ..default()
                    }),
                    ..default()
                })
                .build()
                .disable::<bevy::audio::AudioPlugin>(),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(ElfGamePlugin)
        .run();
}
