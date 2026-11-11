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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;
    use crate::elf_game::{
        core::{GameState, WorkshopQuota},
        ui::MenuState,
        workshop::room::{TubeTravellingLump, tube_travelling_lump_system},
    };

    #[test]
    fn test_moose_hunger_progression_stages() {
        let mut app = App::new();
        app.add_plugins(MoosePlugin);

        // Initial state: hunger = 0.0 -> BehindShutters
        {
            let hunger = app.world().resource::<MooseHunger>();
            assert_eq!(hunger.value, 0.0);
            assert_eq!(hunger.stage, MooseHungerStage::BehindShutters);
        }

        // Advance 30 seconds -> PeekingThrough (20..50)
        {
            let mut time = Time::<()>::default();
            time.advance_by(std::time::Duration::from_secs(30));
            app.insert_resource(time);
        }
        app.update();
        {
            let hunger = app.world().resource::<MooseHunger>();
            assert_eq!(hunger.stage, MooseHungerStage::PeekingThrough);
            assert_eq!(hunger.value, 30.0);
        }

        // Advance another 25 seconds -> AgitatedHuffing (50..60)
        {
            let mut time = Time::<()>::default();
            time.advance_by(std::time::Duration::from_secs(25));
            app.insert_resource(time);
        }
        app.update();
        {
            let hunger = app.world().resource::<MooseHunger>();
            assert_eq!(hunger.stage, MooseHungerStage::AgitatedHuffing);
            assert_eq!(hunger.value, 55.0);
        }

        // Advance 10 seconds -> WindowSlammed (60.0 reached, starts breach countdown)
        {
            let mut time = Time::<()>::default();
            time.advance_by(std::time::Duration::from_secs(10));
            app.insert_resource(time);
        }
        app.update();
        {
            let hunger = app.world().resource::<MooseHunger>();
            assert_eq!(hunger.stage, MooseHungerStage::WindowSlammed);
            assert_eq!(hunger.value, 60.0);
            assert!(hunger.breach_delay_remaining.is_some());
        }

        // Advance remaining breach delay (e.g. 7s) -> Smashes through into BreachedRampage
        {
            let mut time = Time::<()>::default();
            time.advance_by(std::time::Duration::from_secs(7));
            app.insert_resource(time);
        }
        app.update();
        {
            let hunger = app.world().resource::<MooseHunger>();
            assert!(hunger.is_breached);
            assert_eq!(hunger.stage, MooseHungerStage::BreachedRampage);
        }
    }

    #[test]
    fn test_moose_hunger_feeding_reduces_value() {
        let mut app = App::new();
        app.add_plugins(MoosePlugin);

        // Set hunger to 45.0 (Peeking stage)
        {
            let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
            hunger.value = 45.0;
            hunger.stage = MooseHungerStage::PeekingThrough;
        }

        // Spawn snack (30 nutrition)
        let snack_entity = app.world_mut().spawn(MooseSnack {
            nutrition_value: 30.0,
        }).id();

        // Feed moose
        app.world_mut().write_message(FeedMooseMessage { snack_entity });
        app.update();

        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.value, 15.0, "Hunger reduced by 30 to 15.0");
        assert_eq!(hunger.stage, MooseHungerStage::BehindShutters, "Moose retreats behind shutters");
        assert!(app.world().get_entity(snack_entity).is_err(), "Snack consumed");
    }

    #[test]
    fn test_pause_button_and_menu_pauses_hunger_meter_and_timers() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);

        app.insert_resource(MenuState::default());
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.insert_resource(MooseHunger {
            value: 10.0,
            increment_rate: 2.0,
            stage: MooseHungerStage::BehindShutters,
            ..default()
        });
        app.add_systems(Update, update_moose_hunger_system);

        // 1. Unpaused: hunger accumulates with dt
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
        app.update();

        let hunger_unpaused = app.world().resource::<MooseHunger>().value;
        assert!(
            hunger_unpaused > 10.0,
            "Moose hunger must increase while game is unpaused, got {}",
            hunger_unpaused
        );

        // 2. Pause via MenuState::is_open = true
        app.world_mut().resource_mut::<MenuState>().is_open = true;

        let snapshot_hunger = hunger_unpaused;
        for _ in 0..10 {
            app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
            app.update();
        }

        let hunger_paused_menu = app.world().resource::<MooseHunger>().value;
        assert_eq!(
            hunger_paused_menu, snapshot_hunger,
            "Moose hunger must remain strictly frozen while menu is open"
        );

        // 3. Pause via GameState::Paused
        {
            let mut menu = app.world_mut().resource_mut::<MenuState>();
            menu.is_open = false;
            let mut next_state = app.world_mut().resource_mut::<NextState<GameState>>();
            next_state.set(GameState::Paused);
        }
        app.update(); // Apply state transition

        for _ in 0..10 {
            app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
            app.update();
        }

        let hunger_paused_state = app.world().resource::<MooseHunger>().value;
        assert_eq!(
            hunger_paused_state, snapshot_hunger,
            "Moose hunger must remain strictly frozen while GameState is Paused"
        );

        // 4. Unpause: resuming allows hunger to accumulate again
        {
            let mut next_state = app.world_mut().resource_mut::<NextState<GameState>>();
            next_state.set(GameState::InWorkshop);
        }
        app.update();

        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
        app.update();

        let hunger_resumed = app.world().resource::<MooseHunger>().value;
        assert!(
            hunger_resumed > snapshot_hunger,
            "Moose hunger must resume accumulating after unpausing, got {}",
            hunger_resumed
        );
    }

    #[test]
    fn test_moose_initial_dormancy_and_first_present_wakeup() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);

        // Insert dormant MooseHunger
        app.insert_resource(MooseHunger::new_dormant());
        app.insert_resource(WorkshopQuota::default());
        app.add_message::<MooseAwakenedMessage>();

        app.add_systems(Update, (update_moose_hunger_system, tube_travelling_lump_system));

        // 1. Advance time while Mr. Moose is dormant: hunger remains 0.0, stage remains BehindShutters
        for _ in 0..100 {
            app.update();
        }

        let hunger = app.world().resource::<MooseHunger>();
        assert!(hunger.is_dormant, "Mr. Moose must remain dormant initially");
        assert_eq!(hunger.value, 0.0, "Hunger must stay at 0.0 while dormant");
        assert_eq!(hunger.stage, MooseHungerStage::BehindShutters);

        // 2. Dispatch a present through the pneumatic tube (progress = 0.99)
        app.world_mut().spawn(TubeTravellingLump {
            progress: 0.99,
            speed: 1.0,
            mass: 1.5,
            size: 0.3,
        });

        app.update();

        // 3. Delivery completed! Verify Mr. Moose has awakened
        let updated_hunger = app.world().resource::<MooseHunger>();
        assert!(!updated_hunger.is_dormant, "Mr. Moose must awaken when first present is delivered");

        // Verify MooseAwakenedMessage was emitted
        let events = app.world().resource::<Messages<MooseAwakenedMessage>>();
        assert_eq!(events.len(), 1, "Must emit MooseAwakenedMessage on wakeup");

        // 4. Now that Mr. Moose is awake, ticking time increments hunger normally (+1/s)
        let initial_val = app.world().resource::<MooseHunger>().value;
        for _ in 0..50 {
            app.update();
        }
        let final_val = app.world().resource::<MooseHunger>().value;
        assert!(
            final_val > initial_val,
            "Hunger must accumulate after awakening: was {}, now {}",
            initial_val,
            final_val
        );
    }
}
