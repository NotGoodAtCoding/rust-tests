use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader, MessageWriter};

pub mod toy_model;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MooseHungerStage {
    BehindShutters,    // 0..20
    PeekingThrough,    // 20..50
    AgitatedHuffing,   // 50..60
    WindowSlammed,     // 60 (waiting 3..12s before smash)
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
        }
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

#[derive(Message, Debug, Clone)]
pub struct MooseHuffEvent;

#[derive(Message, Debug, Clone)]
pub struct MooseWindowSlamEvent;

#[derive(Message, Debug, Clone)]
pub struct MooseBreachMessage;

#[derive(Message, Debug, Clone)]
pub struct FeedMooseMessage {
    pub snack_entity: Entity,
}

pub fn update_moose_hunger_system(
    time: Option<Res<Time>>,
    mut hunger: ResMut<MooseHunger>,
    mut slam_events: Option<MessageWriter<MooseWindowSlamEvent>>,
    mut breach_messages: Option<MessageWriter<MooseBreachMessage>>,
    mut huff_events: Option<MessageWriter<MooseHuffEvent>>,
) {
    let dt = time.map(|t| t.delta_secs()).unwrap_or(0.0);

    if hunger.is_breached {
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
        hunger.stage = MooseHungerStage::WindowSlammed;
        // Default deterministic midpoint for delay if random not seeded: 6.0 seconds
        hunger.breach_delay_remaining = Some(6.0);
        if let Some(ref mut writer) = slam_events {
            writer.write(MooseWindowSlamEvent);
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
            .add_message::<MooseHuffEvent>()
            .add_message::<MooseWindowSlamEvent>()
            .add_message::<FeedMooseMessage>()
            .add_systems(Startup, toy_model::spawn_mr_moose)
            .add_systems(
                Update,
                (
                    update_moose_hunger_system,
                    handle_moose_feeding,
                    toy_model::animate_mr_moose_peeking_system,
                ),
            );
    }
}
