use std::path::{Path, PathBuf};
use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageWriter, MessageReader};
use serde::{Deserialize, Serialize};

use crate::elf_game::{
    core::{is_game_paused, GameState, WorkshopQuota},
    moose::{MooseHunger, MooseHungerStage},
    physics_sandbox::{
        ragdoll::{ChandelierPerchedElf, ElfAssistant, ElfJob, ElfJobHat, PhilosopherPipe},
    },
    player::{
        bunny_ears::BunnyEarsItem,
        controller::{CameraPivot, ElfPlayer},
        items::{HatType, WearableHatItem},
        ElfMovement,
    },
    workshop::{
        pantry::{MooseTreatJar, PantryRestockTracker},
        progression::{DoorState, WorkshopDoor},
    },
};

pub const DEFAULT_SAVE_PATH: &str = "workshop_save.ron";

/// Player statistics and session records tracked over play time.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerStatistics {
    pub total_gifts_wrapped: u32,
    pub presents_delivered: u32,
    pub quota_score: u32,
    pub best_bhop_streak: u32,
    pub elapsed_play_time: f32,
}

impl Default for PlayerStatistics {
    fn default() -> Self {
        Self {
            total_gifts_wrapped: 0,
            presents_delivered: 0,
            quota_score: 0,
            best_bhop_streak: 0,
            elapsed_play_time: 0.0,
        }
    }
}

/// Progression milestone flags for major workshop unlocks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProgressionMilestones {
    pub presents_delivered: u32,
    pub target_quota: u32,
    pub tier: u32,
    pub moose_awakened: bool,
    pub sparky_awakened: bool,
}

/// Simplified serializable representation of Mr. Moose's state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedMooseState {
    pub hunger_value: f32,
    pub stage: String,
    pub is_dormant: bool,
    pub is_breached: bool,
}

impl Default for SavedMooseState {
    fn default() -> Self {
        Self {
            hunger_value: 0.0,
            stage: "BehindShutters".to_string(),
            is_dormant: true,
            is_breached: false,
        }
    }
}

/// Simplified serializable door barrier state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedDoorState {
    pub door_id: String,
    pub required_tier: u32,
    pub is_open: bool,
    pub position: [f32; 3],
}

/// Simplified serializable Moose Treat Jar state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedTreatJar {
    pub position: [f32; 3],
    pub treat_count: u32,
    pub max_treats: u32,
}

/// Simplified serializable spawned hat prop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedHat {
    pub hat_type: String,
    pub position: [f32; 3],
}

/// Simplified serializable Pantry restock tracker state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SavedPantryTracker {
    pub restock_cycle_count: u32,
    pub total_restocked_jars: u32,
}

/// Saved player transform and look direction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedPlayerTransform {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

/// Complete lightweight persistent save snapshot of Santa's Workshop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkshopSaveData {
    pub version: u32,
    pub timestamp_secs: f64,
    pub milestones: ProgressionMilestones,
    pub player_stats: PlayerStatistics,
    pub player_transform: Option<SavedPlayerTransform>,
    pub moose_state: SavedMooseState,
    pub doors: Vec<SavedDoorState>,
    pub treat_jars: Vec<SavedTreatJar>,
    pub spawned_hats: Vec<SavedHat>,
    pub pantry_tracker: SavedPantryTracker,
}

impl Default for WorkshopSaveData {
    fn default() -> Self {
        Self {
            version: 1,
            timestamp_secs: 0.0,
            milestones: ProgressionMilestones::default(),
            player_stats: PlayerStatistics::default(),
            player_transform: None,
            moose_state: SavedMooseState::default(),
            doors: Vec::new(),
            treat_jars: Vec::new(),
            spawned_hats: Vec::new(),
            pantry_tracker: SavedPantryTracker::default(),
        }
    }
}

/// Request message to quick-save the current workshop state.
#[derive(Message, Debug, Clone)]
pub struct QuickSaveRequest {
    pub destination_path: Option<PathBuf>,
}

impl Default for QuickSaveRequest {
    fn default() -> Self {
        Self { destination_path: None }
    }
}

/// Request message to quick-load the workshop state from disk.
#[derive(Message, Debug, Clone)]
pub struct QuickLoadRequest {
    pub source_path: Option<PathBuf>,
}

impl Default for QuickLoadRequest {
    fn default() -> Self {
        Self { source_path: None }
    }
}

/// Notification message fired after a save operation finishes.
#[derive(Message, Debug, Clone)]
pub struct SaveCompletedEvent {
    pub success: bool,
    pub path: PathBuf,
    pub message: String,
}

/// Notification message fired after a load operation finishes.
#[derive(Message, Debug, Clone)]
pub struct LoadCompletedEvent {
    pub success: bool,
    pub path: PathBuf,
    pub message: String,
}

/// Serializes save data to human-readable RON format.
pub fn serialize_save_data(data: &WorkshopSaveData) -> Result<String, ron::Error> {
    let pretty = ron::ser::PrettyConfig::default()
        .depth_limit(4)
        .separate_tuple_members(true);
    ron::ser::to_string_pretty(data, pretty)
}

/// Deserializes save data from a RON string.
pub fn deserialize_save_data(ron_str: &str) -> Result<WorkshopSaveData, ron::error::SpannedError> {
    ron::de::from_str(ron_str)
}

/// Captures a lightweight snapshot of the current workshop state from the Bevy World.
pub fn create_save_snapshot(world: &mut World) -> WorkshopSaveData {
    let mut data = WorkshopSaveData {
        version: 1,
        timestamp_secs: world
            .get_resource::<Time>()
            .map(|t| t.elapsed_secs_f64())
            .unwrap_or(0.0),
        ..default()
    };

    // 1. Quota & progression milestones
    if let Some(quota) = world.get_resource::<WorkshopQuota>() {
        data.milestones.presents_delivered = quota.presents_delivered;
        data.milestones.target_quota = quota.target_quota;
        data.milestones.tier = quota.tier;
    }

    // 2. Player statistics
    if let Some(stats) = world.get_resource::<PlayerStatistics>() {
        data.player_stats = stats.clone();
    }

    // 3. Moose State
    if let Some(moose) = world.get_resource::<MooseHunger>() {
        let stage_str = match moose.stage {
            MooseHungerStage::BehindShutters => "BehindShutters",
            MooseHungerStage::PeekingThrough => "PeekingThrough",
            MooseHungerStage::AgitatedHuffing => "AgitatedHuffing",
            MooseHungerStage::WindowSlammed => "WindowSlammed",
            MooseHungerStage::DeliberatingLateTreat => "DeliberatingLateTreat",
            MooseHungerStage::EatingTreat => "EatingTreat",
            MooseHungerStage::BreachedRampage => "BreachedRampage",
        };
        data.moose_state = SavedMooseState {
            hunger_value: moose.value,
            stage: stage_str.to_string(),
            is_dormant: moose.is_dormant,
            is_breached: moose.is_breached,
        };
        data.milestones.moose_awakened = !moose.is_dormant;
    }

    // 4. Sparky easter egg perched elf state
    let mut sparky_query = world.query::<(&ElfAssistant, &ChandelierPerchedElf)>();
    for (elf, perched) in sparky_query.iter(world) {
        if elf.name == "Elf Sparky" {
            data.milestones.sparky_awakened = perched.is_awakened;
            break;
        }
    }

    // 5. Doors state
    let mut door_query = world.query::<(&WorkshopDoor, &Transform)>();
    for (door, tf) in door_query.iter(world) {
        data.doors.push(SavedDoorState {
            door_id: door.door_id.clone(),
            required_tier: door.required_tier,
            is_open: matches!(door.state, DoorState::Open),
            position: [tf.translation.x, tf.translation.y, tf.translation.z],
        });
    }

    // 6. Treat Jars
    let mut jar_query = world.query::<(&MooseTreatJar, &Transform)>();
    for (jar, tf) in jar_query.iter(world) {
        data.treat_jars.push(SavedTreatJar {
            position: [tf.translation.x, tf.translation.y, tf.translation.z],
            treat_count: jar.treat_count,
            max_treats: jar.max_treats,
        });
    }

    // 7. Spawned hats
    let mut hat_query = world.query::<(Entity, &Transform, Option<&WearableHatItem>, Option<&ElfJobHat>, Option<&BunnyEarsItem>, Option<&PhilosopherPipe>)>();
    for (_e, tf, hat_item, job_hat, bunny, pipe) in hat_query.iter(world) {
        let hat_type_str = if let Some(hat) = hat_item {
            match &hat.hat_type {
                HatType::BunnyEars => "BunnyEars".to_string(),
                HatType::PhilosopherPipe => "PhilosopherPipe".to_string(),
                HatType::Job(job) => match job {
                    ElfJob::MooseFeeder => "MooseFeeder".to_string(),
                    ElfJob::PresentWrapper => "PresentWrapper".to_string(),
                    ElfJob::WorkshopMaintenance => "WorkshopMaintenance".to_string(),
                },
                HatType::Custom(c) => c.clone(),
            }
        } else if let Some(job) = job_hat {
            match job.job {
                ElfJob::MooseFeeder => "MooseFeeder".to_string(),
                ElfJob::PresentWrapper => "PresentWrapper".to_string(),
                ElfJob::WorkshopMaintenance => "WorkshopMaintenance".to_string(),
            }
        } else if bunny.is_some() {
            "BunnyEars".to_string()
        } else if pipe.is_some() {
            "PhilosopherPipe".to_string()
        } else {
            continue;
        };

        data.spawned_hats.push(SavedHat {
            hat_type: hat_type_str,
            position: [tf.translation.x, tf.translation.y, tf.translation.z],
        });
    }

    // 8. Pantry Restock Tracker
    if let Some(pantry) = world.get_resource::<PantryRestockTracker>() {
        data.pantry_tracker = SavedPantryTracker {
            restock_cycle_count: pantry.restock_cycle_count,
            total_restocked_jars: pantry.total_restocked_jars,
        };
    }

    // 9. Player transform
    let mut player_query = world.query_filtered::<&Transform, With<ElfPlayer>>();
    let mut pivot_query = world.query::<&CameraPivot>();
    let pitch = pivot_query.iter(world).next().map(|p| p.pitch).unwrap_or(0.0);
    for tf in player_query.iter(world) {
        let (yaw, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
        data.player_transform = Some(SavedPlayerTransform {
            position: [tf.translation.x, tf.translation.y, tf.translation.z],
            yaw,
            pitch,
        });
        break;
    }

    data
}

/// Applies a loaded save snapshot to the active Bevy World.
pub fn apply_save_snapshot(world: &mut World, save_data: &WorkshopSaveData) {
    // 1. Quota & Progression
    if let Some(mut quota) = world.get_resource_mut::<WorkshopQuota>() {
        quota.presents_delivered = save_data.milestones.presents_delivered;
        quota.target_quota = save_data.milestones.target_quota;
        quota.tier = save_data.milestones.tier;
    }

    // 2. Player statistics
    if let Some(mut stats) = world.get_resource_mut::<PlayerStatistics>() {
        *stats = save_data.player_stats.clone();
    }

    // 3. Moose State
    if let Some(mut moose) = world.get_resource_mut::<MooseHunger>() {
        moose.value = save_data.moose_state.hunger_value;
        moose.is_dormant = save_data.moose_state.is_dormant;
        moose.is_breached = save_data.moose_state.is_breached;
        moose.stage = match save_data.moose_state.stage.as_str() {
            "PeekingThrough" => MooseHungerStage::PeekingThrough,
            "AgitatedHuffing" => MooseHungerStage::AgitatedHuffing,
            "WindowSlammed" => MooseHungerStage::WindowSlammed,
            "DeliberatingLateTreat" => MooseHungerStage::DeliberatingLateTreat,
            "EatingTreat" => MooseHungerStage::EatingTreat,
            "BreachedRampage" => MooseHungerStage::BreachedRampage,
            _ => MooseHungerStage::BehindShutters,
        };
        if save_data.milestones.moose_awakened {
            moose.is_dormant = false;
        }
    }

    // 4. Sparky perched elf awakening
    if save_data.milestones.sparky_awakened {
        let mut sparky_query = world.query::<(&mut ElfAssistant, &mut ChandelierPerchedElf)>();
        for (elf, mut perched) in sparky_query.iter_mut(world) {
            if elf.name == "Elf Sparky" {
                perched.is_awakened = true;
                break;
            }
        }
    }

    // 5. Door barrier state
    let mut door_query = world.query::<(&mut WorkshopDoor, &mut Transform)>();
    for (mut door, mut tf) in door_query.iter_mut(world) {
        if let Some(saved) = save_data.doors.iter().find(|d| d.door_id == door.door_id) {
            if saved.is_open {
                door.state = DoorState::Open;
                tf.translation = door.open_position;
            } else {
                door.state = DoorState::Locked;
                tf.translation = door.closed_position;
            }
        }
    }

    // 6. Treat Jars
    let mut jar_query = world.query::<(&mut MooseTreatJar, &mut Transform)>();
    let mut jar_iter = jar_query.iter_mut(world);
    for saved in &save_data.treat_jars {
        if let Some((mut jar, mut tf)) = jar_iter.next() {
            jar.treat_count = saved.treat_count;
            jar.max_treats = saved.max_treats;
            tf.translation = Vec3::from_array(saved.position);
        }
    }

    // 7. Pantry Tracker
    if let Some(mut pantry) = world.get_resource_mut::<PantryRestockTracker>() {
        pantry.restock_cycle_count = save_data.pantry_tracker.restock_cycle_count;
        pantry.total_restocked_jars = save_data.pantry_tracker.total_restocked_jars;
    }

    // 8. Player transform
    if let Some(ref saved_player) = save_data.player_transform {
        let mut player_query = world.query_filtered::<&mut Transform, With<ElfPlayer>>();
        for mut tf in player_query.iter_mut(world) {
            tf.translation = Vec3::from_array(saved_player.position);
            tf.rotation = Quat::from_rotation_y(saved_player.yaw);
            break;
        }
        let mut pivot_query = world.query::<&mut CameraPivot>();
        for mut pivot in pivot_query.iter_mut(world) {
            pivot.pitch = saved_player.pitch;
            break;
        }
    }
}

/// Saves the workshop state snapshot to a file path.
pub fn save_workshop_to_file(world: &mut World, file_path: &Path) -> Result<(), std::io::Error> {
    let snapshot = create_save_snapshot(world);
    let ron_str = serialize_save_data(&snapshot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(file_path, ron_str)
}

/// Loads the workshop state snapshot from a file path and applies it to the world.
pub fn load_workshop_from_file(
    world: &mut World,
    file_path: &Path,
) -> Result<WorkshopSaveData, std::io::Error> {
    let ron_str = std::fs::read_to_string(file_path)?;
    let snapshot = deserialize_save_data(&ron_str)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    apply_save_snapshot(world, &snapshot);
    Ok(snapshot)
}

/// System to handle Quick-Save (<kbd>F5</kbd>) and Quick-Load (<kbd>F9</kbd>) keyboard inputs.
pub fn quick_save_load_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut save_requests: MessageWriter<QuickSaveRequest>,
    mut load_requests: MessageWriter<QuickLoadRequest>,
) {
    if keyboard.just_pressed(KeyCode::F5) {
        save_requests.write(QuickSaveRequest::default());
    }
    if keyboard.just_pressed(KeyCode::F9) {
        load_requests.write(QuickLoadRequest::default());
    }
}

/// System that executes QuickSaveRequest messages.
pub fn process_quick_save_requests(
    mut commands: Commands,
    mut requests: MessageReader<QuickSaveRequest>,
) {
    for req in requests.read() {
        let path = req
            .destination_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));

        commands.queue(move |world: &mut World| {
            let result = save_workshop_to_file(world, &path);
            let (success, message) = match result {
                Ok(_) => (true, format!("Successfully saved to {:?}", path)),
                Err(err) => (false, format!("Save failed: {:?}", err)),
            };
            world.write_message(SaveCompletedEvent {
                success,
                path,
                message,
            });
        });
    }
}

/// System that executes QuickLoadRequest messages.
pub fn process_quick_load_requests(
    mut commands: Commands,
    mut requests: MessageReader<QuickLoadRequest>,
) {
    for req in requests.read() {
        let path = req
            .source_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));

        commands.queue(move |world: &mut World| {
            let result = load_workshop_from_file(world, &path);
            let (success, message) = match result {
                Ok(_) => (true, format!("Successfully loaded from {:?}", path)),
                Err(err) => (false, format!("Load failed: {:?}", err)),
            };
            world.write_message(LoadCompletedEvent {
                success,
                path,
                message,
            });
        });
    }
}

/// Continuously tracks player statistics during active play time.
pub fn track_player_statistics_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<GameState>>>,
    quota: Option<Res<WorkshopQuota>>,
    q_player: Query<&ElfMovement, With<ElfPlayer>>,
    stats: Option<ResMut<PlayerStatistics>>,
) {
    let Some(mut stats) = stats else { return };

    if !is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        if let Some(t) = time {
            stats.elapsed_play_time += t.delta_secs();
        }
    }

    if let Some(q) = quota {
        stats.presents_delivered = q.presents_delivered;
        stats.quota_score = q.presents_delivered * 100;
    }

    for movement in &q_player {
        if movement.consecutive_bhops > stats.best_bhop_streak {
            stats.best_bhop_streak = movement.consecutive_bhops;
        }
    }
}

/// Persistence plugin wiring save/load resources, messages, and systems.
pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStatistics>()
            .add_message::<QuickSaveRequest>()
            .add_message::<QuickLoadRequest>()
            .add_message::<SaveCompletedEvent>()
            .add_message::<LoadCompletedEvent>()
            .add_systems(
                Update,
                (
                    quick_save_load_input_system,
                    process_quick_save_requests,
                    process_quick_load_requests,
                    track_player_statistics_system,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf_game::{
        core::WorkshopQuota,
        moose::{MooseHunger, MooseHungerStage},
        physics_sandbox::ragdoll::{ChandelierPerchedElf, ElfAssistant},
        player::ElfMovement,
        workshop::{
            pantry::{MooseTreatJar, PantryRestockTracker},
            progression::{DoorState, WorkshopDoor},
        },
    };

    #[test]
    fn test_save_load_serialization_roundtrip() {
        let original = WorkshopSaveData {
            version: 1,
            timestamp_secs: 123.456,
            milestones: ProgressionMilestones {
                presents_delivered: 8,
                target_quota: 10,
                tier: 2,
                moose_awakened: true,
                sparky_awakened: true,
            },
            player_stats: PlayerStatistics {
                total_gifts_wrapped: 12,
                presents_delivered: 8,
                quota_score: 800,
                best_bhop_streak: 5,
                elapsed_play_time: 142.0,
            },
            player_transform: Some(SavedPlayerTransform {
                position: [1.5, 1.2, -3.2],
                yaw: 0.785,
                pitch: -0.25,
            }),
            moose_state: SavedMooseState {
                hunger_value: 42.0,
                stage: "PeekingThrough".to_string(),
                is_dormant: false,
                is_breached: false,
            },
            doors: vec![SavedDoorState {
                door_id: "StorageWingDoor".to_string(),
                required_tier: 2,
                is_open: true,
                position: [13.0, 0.0, 0.0],
            }],
            treat_jars: vec![SavedTreatJar {
                position: [-10.0, 0.98, 12.6],
                treat_count: 3,
                max_treats: 4,
            }],
            spawned_hats: vec![SavedHat {
                hat_type: "MooseFeeder".to_string(),
                position: [-4.5, 1.3, 12.0],
            }],
            pantry_tracker: SavedPantryTracker {
                restock_cycle_count: 2,
                total_restocked_jars: 3,
            },
        };

        let serialized = serialize_save_data(&original).expect("Serialization must succeed");
        assert!(serialized.contains("milestones"));
        assert!(serialized.contains("StorageWingDoor"));

        let deserialized: WorkshopSaveData = deserialize_save_data(&serialized).expect("Deserialization must succeed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_create_and_apply_save_snapshot() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.insert_resource(WorkshopQuota {
            presents_delivered: 7,
            target_quota: 10,
            tier: 2,
        });
        app.insert_resource(PlayerStatistics {
            total_gifts_wrapped: 9,
            presents_delivered: 7,
            quota_score: 700,
            best_bhop_streak: 4,
            elapsed_play_time: 65.0,
        });
        app.insert_resource(MooseHunger {
            value: 35.0,
            stage: MooseHungerStage::PeekingThrough,
            is_dormant: false,
            ..default()
        });
        app.insert_resource(PantryRestockTracker {
            restock_cycle_count: 3,
            total_restocked_jars: 5,
            ..default()
        });

        let _player_e = app.world_mut().spawn((
            ElfPlayer,
            Transform::from_xyz(2.0, 1.2, -4.0),
        )).id();

        let door_e = app.world_mut().spawn((
            WorkshopDoor {
                door_id: "StorageWingDoor".to_string(),
                required_tier: 2,
                state: DoorState::Open,
                closed_position: Vec3::new(13.0, 0.0, 0.0),
                open_position: Vec3::new(13.0, 4.0, 0.0),
            },
            Transform::from_xyz(13.0, 4.0, 0.0),
        )).id();

        let jar_e = app.world_mut().spawn((
            MooseTreatJar {
                treat_count: 2,
                max_treats: 4,
            },
            Transform::from_xyz(-10.0, 0.98, 12.6),
        )).id();

        let sparky_e = app.world_mut().spawn((
            ElfAssistant::new(0, "Elf Sparky", Vec::new()),
            ChandelierPerchedElf {
                is_awakened: true,
                ..default()
            },
        )).id();

        // Capture snapshot
        let snapshot = create_save_snapshot(app.world_mut());
        assert_eq!(snapshot.milestones.presents_delivered, 7);
        assert_eq!(snapshot.milestones.tier, 2);
        assert!(snapshot.milestones.moose_awakened);
        assert!(snapshot.milestones.sparky_awakened);
        assert_eq!(snapshot.moose_state.hunger_value, 35.0);
        assert_eq!(snapshot.doors.len(), 1);
        assert!(snapshot.doors[0].is_open);

        // Now corrupt/reset world state
        {
            let mut quota = app.world_mut().resource_mut::<WorkshopQuota>();
            quota.presents_delivered = 0;
            quota.tier = 1;
            let mut stats = app.world_mut().resource_mut::<PlayerStatistics>();
            *stats = PlayerStatistics::default();
            let mut moose = app.world_mut().resource_mut::<MooseHunger>();
            moose.value = 0.0;
            moose.is_dormant = true;
            moose.stage = MooseHungerStage::BehindShutters;
            let mut pantry = app.world_mut().resource_mut::<PantryRestockTracker>();
            pantry.restock_cycle_count = 0;

            let closed_pos = {
                let mut door = app.world_mut().get_mut::<WorkshopDoor>(door_e).unwrap();
                door.state = DoorState::Locked;
                door.closed_position
            };
            let mut door_tf = app.world_mut().get_mut::<Transform>(door_e).unwrap();
            door_tf.translation = closed_pos;

            let mut jar = app.world_mut().get_mut::<MooseTreatJar>(jar_e).unwrap();
            jar.treat_count = 0;

            let mut sparky = app.world_mut().get_mut::<ChandelierPerchedElf>(sparky_e).unwrap();
            sparky.is_awakened = false;
        }

        // Apply snapshot back
        apply_save_snapshot(app.world_mut(), &snapshot);

        // Verify all restored
        let quota = app.world().resource::<WorkshopQuota>();
        assert_eq!(quota.presents_delivered, 7);
        assert_eq!(quota.tier, 2);

        let stats = app.world().resource::<PlayerStatistics>();
        assert_eq!(stats.quota_score, 700);
        assert_eq!(stats.best_bhop_streak, 4);

        let moose = app.world().resource::<MooseHunger>();
        assert_eq!(moose.value, 35.0);
        assert_eq!(moose.stage, MooseHungerStage::PeekingThrough);
        assert!(!moose.is_dormant);

        let door = app.world().get::<WorkshopDoor>(door_e).unwrap();
        assert_eq!(door.state, DoorState::Open);
        let door_tf = app.world().get::<Transform>(door_e).unwrap();
        assert_eq!(door_tf.translation, Vec3::new(13.0, 4.0, 0.0));

        let jar = app.world().get::<MooseTreatJar>(jar_e).unwrap();
        assert_eq!(jar.treat_count, 2);

        let sparky = app.world().get::<ChandelierPerchedElf>(sparky_e).unwrap();
        assert!(sparky.is_awakened);
    }

    #[test]
    fn test_quicksave_quickload_file_roundtrip() {
        let test_file = std::path::PathBuf::from(format!("/tmp/workshop_save_test_{}.ron", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));

        let mut app = App::new();
        app.insert_resource(WorkshopQuota {
            presents_delivered: 15,
            target_quota: 20,
            tier: 3,
        });
        app.insert_resource(PlayerStatistics {
            total_gifts_wrapped: 22,
            presents_delivered: 15,
            quota_score: 1500,
            best_bhop_streak: 7,
            elapsed_play_time: 320.0,
        });

        // Save to file
        save_workshop_to_file(app.world_mut(), &test_file).expect("File save should succeed");
        assert!(test_file.exists(), "Save file must be written to disk");

        // Mutate world
        {
            let mut q = app.world_mut().resource_mut::<WorkshopQuota>();
            q.presents_delivered = 0;
            q.tier = 1;
        }

        // Load from file
        let loaded_data = load_workshop_from_file(app.world_mut(), &test_file).expect("File load should succeed");
        assert_eq!(loaded_data.milestones.presents_delivered, 15);
        assert_eq!(loaded_data.milestones.tier, 3);

        let quota = app.world().resource::<WorkshopQuota>();
        assert_eq!(quota.presents_delivered, 15);
        assert_eq!(quota.tier, 3);

        // Clean up temp file
        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_player_statistics_tracking_and_bhop_streak() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<PlayerStatistics>();
        app.insert_resource(WorkshopQuota {
            presents_delivered: 4,
            target_quota: 5,
            tier: 1,
        });

        let player_e = app.world_mut().spawn((
            ElfPlayer,
            ElfMovement {
                consecutive_bhops: 6,
                ..default()
            },
        )).id();

        app.add_systems(Update, track_player_statistics_system);
        app.update();

        let stats = app.world().resource::<PlayerStatistics>();
        assert_eq!(stats.presents_delivered, 4);
        assert_eq!(stats.quota_score, 400);
        assert_eq!(stats.best_bhop_streak, 6);

        // Further hop streak
        {
            let mut movement = app.world_mut().get_mut::<ElfMovement>(player_e).unwrap();
            movement.consecutive_bhops = 9;
        }
        app.update();

        let stats = app.world().resource::<PlayerStatistics>();
        assert_eq!(stats.best_bhop_streak, 9);
    }
}
