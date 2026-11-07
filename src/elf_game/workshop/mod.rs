use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageWriter};
use crate::elf_game::core::WorkshopQuota;

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
        app.add_message::<UnlockDoorMessage>()
            .add_systems(Update, check_workshop_progression);
    }
}
