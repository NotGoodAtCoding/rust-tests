use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader, MessageWriter};

#[derive(Resource, Debug, Clone)]
pub struct MooseHunger {
    pub current: f32,
    pub max: f32,
    pub drain_rate: f32, // Points per second
    pub is_breached: bool,
}

impl Default for MooseHunger {
    fn default() -> Self {
        Self {
            current: 90.0,
            max: 90.0,
            drain_rate: 1.0,
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
            nutrition_value: 45.0,
        }
    }
}

#[derive(Component, Debug)]
pub struct MooseFeedingTrough;

#[derive(Message, Debug, Clone)]
pub struct MooseBreachMessage;

#[derive(Message, Debug, Clone)]
pub struct FeedMooseMessage {
    pub snack_entity: Entity,
}

pub fn drain_moose_hunger(
    time: Res<Time>,
    mut hunger: ResMut<MooseHunger>,
    mut breach_messages: MessageWriter<MooseBreachMessage>,
) {
    if hunger.is_breached {
        return;
    }

    hunger.current -= hunger.drain_rate * time.delta_secs();
    if hunger.current <= 0.0 {
        hunger.current = 0.0;
        hunger.is_breached = true;
        breach_messages.write(MooseBreachMessage);
    }
}

pub fn handle_moose_feeding(
    mut feed_messages: MessageReader<FeedMooseMessage>,
    mut hunger: ResMut<MooseHunger>,
    snack_query: Query<&MooseSnack>,
    mut commands: Commands,
) {
    for msg in feed_messages.read() {
        if let Ok(snack) = snack_query.get(msg.snack_entity) {
            hunger.current = (hunger.current + snack.nutrition_value).min(hunger.max);
            commands.entity(msg.snack_entity).despawn();
        }
    }
}

pub struct MoosePlugin;

impl Plugin for MoosePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MooseHunger>()
            .add_message::<MooseBreachMessage>()
            .add_message::<FeedMooseMessage>()
            .add_systems(Update, (drain_moose_hunger, handle_moose_feeding));
    }
}
