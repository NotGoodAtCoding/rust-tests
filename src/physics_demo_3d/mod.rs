pub mod platforms;
pub mod player;
pub mod scene;
pub mod ui;

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
};
use bevy_rapier3d::prelude::*;

pub const DEFAULT_RENDER_LAYER: usize = 0;
pub const VIEW_MODEL_RENDER_LAYER: usize = 1;

pub fn main() {
    App::new()
        .init_resource::<ui::MenuState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy 3D - Dynamic Platforms & Damped Physics Demo".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_observer(ui::on_button_activate)
        .add_observer(ui::on_slider_thumb_drag)
        .add_observer(ui::on_slider_thumb_over)
        .add_observer(ui::on_slider_thumb_out)
        .add_systems(
            Startup,
            (
                scene::setup_scene,
                scene::setup_physics_objects,
                player::spawn_player_viewmodel,
                ui::spawn_ui,
            ),
        )
        .add_systems(
            Update,
            (
                ui::toggle_menu,
                ui::handle_mouse_grab,
                ui::update_button_visuals,
                ui::update_menu_button_labels,
                ui::sync_hud_visibility,
                ui::sync_sensitivity_slider,
                ui::update_fps,
                (
                    platforms::update_moving_platforms,
                    player::player_look,
                    player::player_move,
                    player::player_push_spheres,
                )
                    .chain(),
                player::handle_pickup_interaction,
                player::update_held_sphere,
                scene::apply_low_velocity_damping,
                ui::update_hud_stats,
                player::viewmodel_bob,
                player::change_fov,
            ),
        )
        .run();
}
