use bevy::prelude::*;

pub mod hunger_dial;
pub mod monitor;
pub mod text_renderer;

pub use hunger_dial::{
    Gauge, GasPressureGauge, GaugeNeedle, GaugeWarningBeacon, PropaneRefillGauge,
    spawn_gas_pressure_gauge, spawn_gauge, spawn_propane_refill_gauge,
    update_gas_pressure_gauge_system, update_propane_refill_gauge_system, animate_gauge_system,
};

pub struct DiegeticUiPlugin;

impl Plugin for DiegeticUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                monitor::spawn_diegetic_screens_and_signs,
                hunger_dial::spawn_gas_pressure_gauge,
            ),
        )
        .add_systems(
            Update,
            (
                monitor::update_diegetic_monitor_text,
                monitor::animate_crt_scanlines,
                hunger_dial::update_gas_pressure_gauge_system,
                hunger_dial::update_propane_refill_gauge_system,
                hunger_dial::animate_gauge_system,
            ),
        );
    }
}
