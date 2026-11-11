use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader, MessageWriter};

pub mod toy_model;
pub use toy_model::{MrMoose, MooseHead, MoosePart};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MooseHungerStage {
    BehindShutters,    // 0..20
    PeekingThrough,    // 20..50
    AgitatedHuffing,   // 50..60
    WindowSlammed,     // 60 (waiting 3..12s before smash)
    DeliberatingLateTreat, // Peeking back in after window was shut, deciding accept/reject (1-2s)
    EatingTreat,       // Reaching down into trough to eat (gentle or fast)
    BreachedRampage,   // Smashed through window and moving on path
}

#[derive(Resource, Debug, Clone)]
pub struct MooseHunger {
    pub value: f32,          // 0.0 to 60.0, increments 1.0/sec
    pub max_value: f32,      // 60.0
    pub increment_rate: f32, // 1.0 per sec
    pub stage: MooseHungerStage,
    pub breach_delay_remaining: Option<f32>, // 3.0 to 12.0s
    pub huff_sound_timer: f32,
    pub is_breached: bool,
    pub is_dormant: bool,

    // Deliberation and eating animation tracking:
    pub deliberation_timer: f32,
    pub deliberation_duration: f32,
    pub eating_timer: f32,
    pub eating_duration: f32,
    pub is_eating_fast: bool,
    pub active_snack_entity: Option<Entity>,
    pub late_treat_decision_override: Option<bool>, // Some(true) = accept, Some(false) = reject, None = default
    pub late_treat_rejected: bool,
}

impl Default for MooseHunger {
    fn default() -> Self {
        Self {
            value: 0.0,
            max_value: 60.0,
            increment_rate: 1.0,
            stage: MooseHungerStage::BehindShutters,
            breach_delay_remaining: None,
            huff_sound_timer: 0.0,
            is_breached: false,
            is_dormant: false,
            deliberation_timer: 0.0,
            deliberation_duration: 1.5,
            eating_timer: 0.0,
            eating_duration: 2.2,
            is_eating_fast: false,
            active_snack_entity: None,
            late_treat_decision_override: None,
            late_treat_rejected: false,
        }
    }
}

impl MooseHunger {
    /// Constructs a dormant MooseHunger instance where Mr. Moose starts sleeping
    /// peacefully behind closed shutters until awakened by the 1st submitted present.
    pub fn new_dormant() -> Self {
        Self {
            is_dormant: true,
            ..default()
        }
    }

    /// Awakens Mr. Moose from dormancy, initiating hunger timer accumulation (+1/s).
    pub fn awaken(&mut self) {
        self.is_dormant = false;
    }
}

#[derive(Component, Debug, Clone)]
pub struct MooseSnack {
    pub nutrition_value: f32,
}

impl Default for MooseSnack {
    fn default() -> Self {
        Self {
            nutrition_value: 30.0,
        }
    }
}

#[derive(Component, Debug)]
pub struct MooseFeedingTrough;

/// Tag component for the feeding bucket on the right side of the stable double door
#[derive(Component, Debug, Clone)]
pub struct MooseFeedingBucket;

/// Tag component for the festive cookie plate on the table next to the stable door
#[derive(Component, Debug, Clone)]
pub struct MooseCookiePlate;

#[derive(Message, Debug, Clone)]
pub struct MooseHuffEvent;

#[derive(Message, Debug, Clone)]
pub struct MooseWindowSlamEvent;

#[derive(Message, Debug, Clone)]
pub struct MooseBreachMessage;

/// Fired when Mr. Moose is awakened from initial dormancy by the first submitted present
#[derive(Message, Debug, Clone)]
pub struct MooseAwakenedMessage;

#[derive(Message, Debug, Clone)]
pub struct FeedMooseMessage {
    pub snack_entity: Entity,
}

pub fn update_moose_hunger_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut hunger: ResMut<MooseHunger>,
    mut slam_events: Option<MessageWriter<MooseWindowSlamEvent>>,
    mut breach_messages: Option<MessageWriter<MooseBreachMessage>>,
    mut huff_events: Option<MessageWriter<MooseHuffEvent>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    if hunger.is_dormant {
        return;
    }

    let dt = time.map(|t| t.delta_secs()).unwrap_or(0.0);

    if hunger.is_breached {
        return;
    }

    // Deliberation or active eating pauses hunger accumulation and breach countdown!
    if hunger.stage == MooseHungerStage::DeliberatingLateTreat
        || hunger.stage == MooseHungerStage::EatingTreat
    {
        return;
    }

    if let Some(ref mut delay) = hunger.breach_delay_remaining {
        *delay -= dt;
        if *delay <= 0.0 {
            hunger.breach_delay_remaining = None;
            hunger.is_breached = true;
            hunger.stage = MooseHungerStage::BreachedRampage;
            if let Some(ref mut writer) = breach_messages {
                writer.write(MooseBreachMessage);
            }
        }
        return;
    }

    // Increment hunger value
    hunger.value = (hunger.value + hunger.increment_rate * dt).min(hunger.max_value);

    // Evaluate stages
    if hunger.value < 20.0 {
        hunger.stage = MooseHungerStage::BehindShutters;
    } else if hunger.value < 50.0 {
        hunger.stage = MooseHungerStage::PeekingThrough;
    } else if hunger.value < 60.0 {
        hunger.stage = MooseHungerStage::AgitatedHuffing;
        hunger.huff_sound_timer += dt;
        if hunger.huff_sound_timer >= 2.0 {
            hunger.huff_sound_timer = 0.0;
            if let Some(ref mut writer) = huff_events {
                writer.write(MooseHuffEvent);
            }
        }
    } else {
        // Reached 60! Slam window and start 3-12s breach delay
        let was_already_slammed = hunger.stage == MooseHungerStage::WindowSlammed;
        hunger.stage = MooseHungerStage::WindowSlammed;
        if hunger.breach_delay_remaining.is_none() {
            // Default deterministic midpoint for delay if random not seeded: 6.0 seconds
            hunger.breach_delay_remaining = Some(6.0);
        }
        if !was_already_slammed {
            if let Some(ref mut writer) = slam_events {
                writer.write(MooseWindowSlamEvent);
            }
        }
    }
}

pub fn handle_moose_feeding(
    feed_messages: Option<MessageReader<FeedMooseMessage>>,
    mut hunger: ResMut<MooseHunger>,
    snack_query: Query<&MooseSnack>,
    mut commands: Commands,
) {
    let Some(mut messages) = feed_messages else {
        return;
    };

    for msg in messages.read() {
        if let Ok(snack) = snack_query.get(msg.snack_entity) {
            // Feeding reduces hunger value back towards 0
            hunger.value = (hunger.value - snack.nutrition_value).max(0.0);
            hunger.breach_delay_remaining = None;
            hunger.is_breached = false;

            if hunger.value < 20.0 {
                hunger.stage = MooseHungerStage::BehindShutters;
            } else if hunger.value < 50.0 {
                hunger.stage = MooseHungerStage::PeekingThrough;
            } else {
                hunger.stage = MooseHungerStage::AgitatedHuffing;
            }

            commands.entity(msg.snack_entity).despawn();
        }
    }
}

pub struct MoosePlugin;

impl Plugin for MoosePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MooseHunger>()
            .add_message::<MooseBreachMessage>()
            .add_message::<MooseAwakenedMessage>()
            .add_message::<MooseHuffEvent>()
            .add_message::<MooseWindowSlamEvent>()
            .add_message::<FeedMooseMessage>()
            .add_systems(Startup, toy_model::spawn_mr_moose)
            .add_systems(
                Update,
                (
                    toy_model::moose_feeding_trough_system,
                    update_moose_hunger_system,
                    handle_moose_feeding,
                    toy_model::animate_mr_moose_peeking_system,
                    toy_model::moose_eating_and_deliberation_system,
                    toy_model::moose_scarf_physics_system,
                ).chain(),
            );
    }
}
