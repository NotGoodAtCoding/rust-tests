use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageWriter};
use crate::elf_game::core::WorkshopQuota;

pub mod room;
pub mod events;
pub mod progression;
pub mod pantry;
pub mod stove;
pub mod stable_door;
pub mod baking;

pub use room::VacuumTractorBeamCone;
pub use progression::*;
pub use pantry::*;
pub use stove::*;
pub use stable_door::*;
pub use baking::*;

#[derive(Message, Debug, Clone)]
pub struct UnlockDoorMessage {
    pub door_id: String,
    pub tier: u32,
}

pub fn check_workshop_progression(
    mut quota: ResMut<WorkshopQuota>,
    mut unlock_messages: MessageWriter<UnlockDoorMessage>,
) {
    if quota.presents_delivered >= quota.target_quota {
        if quota.tier == 1 {
            quota.tier = 2;
            quota.target_quota += 10;
            unlock_messages.write(UnlockDoorMessage {
                door_id: "StorageWing".to_string(),
                tier: 2,
            });
        } else if quota.tier == 2 {
            quota.tier = 3;
            quota.target_quota += 15;
            unlock_messages.write(UnlockDoorMessage {
                door_id: "ForbiddenBackWorkshop".to_string(),
                tier: 3,
            });
        }
    }
}

pub struct WorkshopPlugin;

impl Plugin for WorkshopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<room::ConveyorSpawnerTimer>()
            .init_resource::<room::CandleLightingConfig>()
            .init_resource::<stove::WorkshopTemperature>()
            .init_resource::<stove::WorkshopGasLine>()
            .add_message::<UnlockDoorMessage>()
            .add_message::<progression::PlasmaExplosionEvent>()
            .add_message::<progression::TriggerPlasmaWrapperMessage>()
            .add_message::<progression::FireRibbonCannonMessage>()
            .add_message::<progression::ToggleOverclockerMessage>()
            .add_message::<pantry::PantryRestockEvent>()
            .add_message::<pantry::PantryDoorInteractMessage>()
            .add_message::<stove::FieryExplosionEvent>()
            .add_message::<baking::CookieBakedEvent>()
            .add_systems(Startup, (room::setup_workshop_scene, stove::spawn_oven_stove_and_propane_scene, baking::spawn_baking_scene))
            .add_systems(
                Update,
                (
                    (check_workshop_progression, progression::handle_door_unlock_messages).chain(),
                    progression::animate_workshop_doors_system,
                    progression::motorized_turntable_system,
                    progression::plasma_shrink_wrapper_system,
                    progression::ribbon_cannon_system,
                    progression::candy_cane_overclocker_system,
                    pantry::pantry_door_spring_system,
                    pantry::pantry_door_restock_cycle_system,
                    pantry::pantry_door_interaction_system,
                    stable_door::stable_door_system,
                    stable_door::stable_door_interaction_system,
                    baking::ingredient_baking_sheet_assembly_system,
                    (baking::stove_baking_system, baking::baking_sheet_cookie_harvest_system).chain(),
                    baking::pantry_ingredient_restock_system,
                ),
            )
            .add_systems(
                Update,
                (
                    stove::workshop_temperature_system,
                    stove::propane_gas_flow_system,
                    stove::propane_platform_activation_area_system,
                    stove::stove_interaction_system,
                    stove::gas_hose_chain_physics_system,
                    stove::gas_hose_interaction_system,
                    stove::appliance_visual_indicators_system,
                    stove::propane_tank_refill_and_holder_system,
                    stove::gas_leak_flame_explosion_system,
                    stove::explosion_flash_fade_system,
                ),
            )
            .add_systems(
                Update,
                (
                    room::conveyor_cube_spawner_system,
                    room::conveyor_belt_physics_system,
                    room::conveyor_despawn_chute_system,
                    room::chandelier_candle_flicker_system,
                    room::chandelier_physics_interaction_system,
                    room::candle_particle_emitter_system,
                    room::candle_particle_system,
                    room::present_sending_platform_system,
                    room::tube_travelling_lump_system,
                    events::monitor_present_delivery,
                    events::handle_moose_window_breach,
                    events::moose_rampage_path_and_physics_impulse_system,
                ),
            );
    }
}
