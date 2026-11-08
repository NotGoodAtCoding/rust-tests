use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageWriter};
use crate::elf_game::core::WorkshopQuota;

pub mod room;
pub mod events;
pub mod progression;

pub use room::VacuumTractorBeamCone;
pub use progression::*;

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
            .add_message::<UnlockDoorMessage>()
            .add_message::<progression::PlasmaExplosionEvent>()
            .add_message::<progression::TriggerPlasmaWrapperMessage>()
            .add_message::<progression::FireRibbonCannonMessage>()
            .add_message::<progression::ToggleOverclockerMessage>()
            .add_systems(Startup, room::setup_workshop_scene)
            .add_systems(
                Update,
                (
                    (check_workshop_progression, progression::handle_door_unlock_messages).chain(),
                    progression::animate_workshop_doors_system,
                    progression::motorized_turntable_system,
                    progression::plasma_shrink_wrapper_system,
                    progression::ribbon_cannon_system,
                    progression::candy_cane_overclocker_system,
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
