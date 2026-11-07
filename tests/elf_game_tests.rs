use std::collections::HashSet;
use bevy::prelude::*;
use rust_tests::elf_game::wrapping::{
    WrappingPlugin, Present, WrappingFeature, WrappingSpecification, WrappingTool, UseToolOnPresentMessage,
};
use rust_tests::elf_game::moose::{
    MoosePlugin, MooseHunger, MooseHungerStage, MooseSnack, FeedMooseMessage,
};
use rust_tests::elf_game::player::{
    BhopConfig, BhopWindow, ElfMovement, ElfHands, update_elf_physics,
};
use rust_tests::elf_game::workshop::{
    WorkshopPlugin,
};
use rust_tests::elf_game::core::{
    CorePlugin, WorkshopQuota,
};
use rust_tests::elf_game::physics_sandbox::{
    SoftBodyLattice,
};

#[test]
fn test_wrapping_attribute_specification_fulfillment() {
    let mut app = App::new();
    app.add_plugins(WrappingPlugin);

    // Specification requires PaperSized, PaperWrapped, TapedCorners, and DecorativeBow
    let mut required = HashSet::new();
    required.insert(WrappingFeature::PaperSized);
    required.insert(WrappingFeature::PaperWrapped);
    required.insert(WrappingFeature::TapedCorners);
    required.insert(WrappingFeature::DecorativeBow);

    let present_entity = app.world_mut().spawn((
        Present {
            toy_name: "Wooden Train".to_string(),
            ..default()
        },
        WrappingSpecification {
            required_features: required,
            forbidden_features: HashSet::new(),
        },
    )).id();

    // 1. Cut Paper (PaperSized)
    app.world_mut().write_message(UseToolOnPresentMessage {
        tool: WrappingTool::Scissors,
        present_entity,
    });
    app.update();
    let p = app.world().get::<Present>(present_entity).unwrap();
    assert!(p.features.contains(&WrappingFeature::PaperSized));
    assert!(!p.is_completed);

    // 2. Wrap Paper (PaperWrapped)
    app.world_mut().write_message(UseToolOnPresentMessage {
        tool: WrappingTool::WrappingPaperRoll,
        present_entity,
    });
    app.update();
    let p = app.world().get::<Present>(present_entity).unwrap();
    assert!(p.features.contains(&WrappingFeature::PaperWrapped));
    assert!(!p.is_completed);

    // 3. Tape Corners (TapedCorners)
    app.world_mut().write_message(UseToolOnPresentMessage {
        tool: WrappingTool::TapeDispenser,
        present_entity,
    });
    app.update();
    let p = app.world().get::<Present>(present_entity).unwrap();
    assert!(p.features.contains(&WrappingFeature::TapedCorners));
    assert!(!p.is_completed);

    // 4. Attach Decorative Bow (DecorativeBow) -> All required attributes present!
    app.world_mut().write_message(UseToolOnPresentMessage {
        tool: WrappingTool::DecorativeBowBox,
        present_entity,
    });
    app.update();
    let p = app.world().get::<Present>(present_entity).unwrap();
    assert!(p.features.contains(&WrappingFeature::DecorativeBow));
    assert!(p.is_completed, "Present should be completed when all specified attributes are met");
}

#[test]
fn test_forbidden_plasma_instant_wrap() {
    let mut app = App::new();
    app.add_plugins(WrappingPlugin);

    let present_entity = app.world_mut().spawn((
        Present {
            toy_name: "Giant Teddy".to_string(),
            quality_score: 1.0,
            ..default()
        },
        WrappingSpecification::default(),
    )).id();

    // Instantly seal with Forbidden Plasma Shrink
    app.world_mut().write_message(UseToolOnPresentMessage {
        tool: WrappingTool::ForbiddenPlasmaShrink,
        present_entity,
    });
    app.update();

    let p = app.world().get::<Present>(present_entity).unwrap();
    assert!(p.is_completed, "Forbidden plasma wrap fulfills required specifications");
    assert!(p.features.contains(&WrappingFeature::PlasmaShrinkSealed));
    assert_eq!(p.quality_score, 1.5, "Forbidden tool gives instant high quality boost");
}

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
fn test_workshop_progression_and_door_unlocks() {
    let mut app = App::new();
    app.add_plugins((CorePlugin, WorkshopPlugin));

    // Deliver enough presents to pass Tier 1 quota (target is 5)
    {
        let mut quota = app.world_mut().resource_mut::<WorkshopQuota>();
        quota.presents_delivered = 5;
    }

    app.update();

    let quota = app.world().resource::<WorkshopQuota>();
    assert_eq!(quota.tier, 2, "Progresses to Tier 2 (Storage Wing)");
    assert_eq!(quota.target_quota, 15);
}

#[test]
fn test_bhop_perfect_timing_and_speed_scaling() {
    let config = BhopConfig {
        window: BhopWindow::Sixtieth,
        max_boost_multiplier: 1.41421356, // sqrt(2)
    };
    let mut movement = ElfMovement::default();
    let dt = 1.0 / 60.0;
    let wish_dir = Vec3::new(0.0, 0.0, -1.0);

    // Hop 1: Perfect timing (landed dt ago <= 1/60s)
    movement.is_grounded = true;
    movement.time_since_landing = 0.0;
    update_elf_physics(dt, wish_dir, true, false, &config, &mut movement);
    assert_eq!(movement.consecutive_bhops, 1);
    assert!(!movement.is_grounded);

    // Hop 2: Perfect landing and fresh jump
    movement.is_grounded = true;
    movement.time_since_landing = 0.0;
    update_elf_physics(dt, wish_dir, true, false, &config, &mut movement);
    assert_eq!(movement.consecutive_bhops, 2);

    // Hop 3: Perfect landing and fresh jump -> Reaches sqrt(2) speed cap!
    movement.is_grounded = true;
    movement.time_since_landing = 0.0;
    update_elf_physics(dt, wish_dir, true, false, &config, &mut movement);
    assert_eq!(movement.consecutive_bhops, 3);
    let expected_boosted_speed = movement.base_speed * 1.41421356;
    assert!((movement.current_max_speed - expected_boosted_speed).abs() < 0.001);
}

#[test]
fn test_bhop_holding_space_does_not_bunnyhop() {
    let config = BhopConfig {
        window: BhopWindow::Sixtieth,
        max_boost_multiplier: 1.41421356,
    };
    let mut movement = ElfMovement {
        consecutive_bhops: 2,
        ..default()
    };
    let dt = 1.0 / 60.0;
    let wish_dir = Vec3::new(0.0, 0.0, -1.0);

    // Player lands while holding space (`jump_just_pressed = false`, `jump_held = true`)
    movement.is_grounded = true;
    movement.time_since_landing = 0.05; // Expired landing window
    update_elf_physics(dt, wish_dir, false, true, &config, &mut movement);

    // Must not trigger hop, and streak decays to 0
    assert_eq!(movement.consecutive_bhops, 0);
    assert_eq!(movement.current_max_speed, movement.base_speed);
}

#[test]
fn test_air_straining_vs_ground_acceleration() {
    let config = BhopConfig::default();
    let dt = 0.1;
    let wish_dir = Vec3::new(1.0, 0.0, 0.0);

    // Ground acceleration
    let mut ground_movement = ElfMovement::default();
    ground_movement.is_grounded = true;
    update_elf_physics(dt, wish_dir, false, false, &config, &mut ground_movement);
    let ground_delta_v = ground_movement.velocity.x;

    // Air straining acceleration
    let mut air_movement = ElfMovement::default();
    air_movement.is_grounded = false;
    update_elf_physics(dt, wish_dir, false, false, &config, &mut air_movement);
    let air_delta_v = air_movement.velocity.x;

    // Air straining must be significantly lower than ground acceleration (~12.5%)
    assert!(air_delta_v > 0.0, "Air straining should impart some acceleration");
    assert!(
        air_delta_v < ground_delta_v * 0.25,
        "Air straining ({air_delta_v}) must be much weaker than ground control ({ground_delta_v})"
    );
}

#[test]
fn test_soft_body_spring_oscillation_and_damping() {
    let mut lattice = SoftBodyLattice::new_cube(Vec3::splat(0.5), 50.0, 5.0);

    // Apply an impulse along X
    lattice.apply_impulse(Vec3::new(2.0, 0.0, 0.0));

    // Step physics forward 0.1s
    for _ in 0..10 {
        lattice.step(0.01);
    }

    // Points should have displaced along X
    let pt = &lattice.points[0];
    assert!(pt.position.x > pt.original_offset.x);

    // Over time, damping should dissipate the kinetic velocity back towards rest
    for _ in 0..200 {
        lattice.step(0.01);
    }
    let pt_rested = &lattice.points[0];
    assert!(
        (pt_rested.position.x - pt_rested.original_offset.x).abs() < 0.05,
        "Damped lattice should settle back near rest position"
    );
}

#[test]
fn test_dual_hands_tape_pulling_and_limits() {
    let mut hands = ElfHands::default();

    // Pull tape for 0.5s -> 0.125m
    assert_eq!(hands.update_tape_pull(0.5, true), None);
    assert!((hands.current_tape_length - 0.125).abs() < 0.001);

    // Release grab -> yields valid tape strand of 0.125m
    let tape_cut = hands.update_tape_pull(0.0, false);
    assert_eq!(tape_cut, Some(0.125));
    assert_eq!(hands.current_tape_length, 0.0);

    // Pull tape for too long (> 0.40m limit, e.g. 2.0s = 0.50m) -> tangles!
    hands.update_tape_pull(2.0, true);
    assert!(hands.tape_tangled, "Tape pulled beyond limit must tangle");

    // Releasing tangled tape yields None (ruined)
    let ruined_tape = hands.update_tape_pull(0.0, false);
    assert_eq!(ruined_tape, None);
}

#[test]
fn test_dual_hands_ribbon_shake_to_form_bow() {
    let mut hands = ElfHands::default();

    // Gentle movement: does not reach threshold
    assert!(!hands.add_shake_energy(0.4));
    assert!(!hands.add_shake_energy(0.4));

    // Violent shake impulse: crosses goal threshold (1.0) and creates a bow
    assert!(hands.add_shake_energy(0.3));
    assert_eq!(hands.ribbon_shake_energy, 0.0, "Energy resets after forming bow");
}

#[test]
fn test_bhop_timing_window_cycling_and_debug_options() {
    let mut config = BhopConfig::default();
    assert_eq!(config.window, BhopWindow::Sixtieth);
    assert_eq!(config.window.label(), "1/60s (Strict)");
    assert!((config.window.duration_secs() - 1.0 / 60.0).abs() < 0.0001);

    // Cycle to 1/30s
    config.window = config.window.next();
    assert_eq!(config.window, BhopWindow::Thirtieth);
    assert_eq!(config.window.label(), "1/30s (Medium)");
    assert!((config.window.duration_secs() - 1.0 / 30.0).abs() < 0.0001);

    // Cycle to 1/10s
    config.window = config.window.next();
    assert_eq!(config.window, BhopWindow::Tenth);
    assert_eq!(config.window.label(), "1/10s (Forgiving)");
    assert!((config.window.duration_secs() - 1.0 / 10.0).abs() < 0.0001);

    // Cycle back to 1/60s
    config.window = config.window.next();
    assert_eq!(config.window, BhopWindow::Sixtieth);
}

#[test]
fn test_conveyor_despawn_chute() {
    use rust_tests::elf_game::workshop::room::{ConveyorDespawnChute, conveyor_despawn_chute_system};

    let mut app = App::new();
    app.add_systems(Update, conveyor_despawn_chute_system);

    app.world_mut().spawn((
        ConveyorDespawnChute,
        Transform::from_xyz(9.6, 0.45, 3.0),
    ));

    // Present cube that reached the end of the conveyor
    let cube_entity = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(9.2, 0.5, 3.0),
    )).id();

    // Present cube still in the middle of the conveyor
    let safe_cube = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(0.0, 0.5, 3.0),
    )).id();

    app.update();

    assert!(app.world().get_entity(cube_entity).is_err(), "Present at end of belt should be despawned by chute");
    assert!(app.world().get_entity(safe_cube).is_ok(), "Present in middle of belt must remain active");
}

#[test]
fn test_moose_rampage_retreat_and_reset() {
    use rust_tests::elf_game::workshop::events::{MooseRampageTracker, moose_rampage_path_and_physics_impulse_system};
    use rust_tests::elf_game::moose::toy_model::MrMoose;

    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(MooseHunger {
        value: 60.0,
        stage: MooseHungerStage::BreachedRampage,
        is_breached: true,
        ..default()
    });
    app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

    // Moose with tracker at last waypoint index
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        MooseRampageTracker {
            current_waypoint_index: 8,
            speed: 8.5,
            waypoints: vec![Vec3::ZERO; 8],
        },
        Transform::from_xyz(-6.5, 1.8, -17.0),
    )).id();

    app.update();

    let hunger = app.world().resource::<MooseHunger>();
    assert_eq!(hunger.stage, MooseHungerStage::BehindShutters);
    assert_eq!(hunger.value, 0.0);
    assert!(!hunger.is_breached);
    assert!(app.world().get::<MooseRampageTracker>(moose_entity).is_none(), "Tracker should be removed on retreat");
}

#[test]
fn test_player_launched_on_contact_with_rampaging_moose() {
    use rust_tests::elf_game::workshop::events::{MooseRampageTracker, moose_rampage_path_and_physics_impulse_system};
    use rust_tests::elf_game::moose::toy_model::MrMoose;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs_f32(0.016));
    app.insert_resource(time);
    app.insert_resource(MooseHunger {
        value: 60.0,
        stage: MooseHungerStage::BreachedRampage,
        is_breached: true,
        ..default()
    });
    app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

    // Moose rampaging at (0.0, 1.4, -4.0) towards next waypoint (5.0, 1.4, 3.0)
    app.world_mut().spawn((
        MrMoose,
        MooseRampageTracker {
            current_waypoint_index: 0,
            speed: 8.5,
            waypoints: vec![Vec3::new(5.0, 1.4, 3.0)],
        },
        Transform::from_xyz(0.0, 1.4, -4.0),
    ));

    // Player standing within contact distance (0.5m from Moose)
    let player_entity = app.world_mut().spawn((
        ElfMovement {
            velocity: Vec3::ZERO,
            is_grounded: true,
            ..default()
        },
        Transform::from_xyz(0.5, 1.2, -4.0),
    )).id();

    app.update();

    let movement = app.world().get::<ElfMovement>(player_entity).unwrap();
    assert!(
        movement.velocity.y >= 10.0,
        "Player should be launched upwards with high vertical speed (>= 10.0), got {}",
        movement.velocity.y
    );
    assert!(!movement.is_grounded, "Player must become airborne upon being launched");
    let h_speed = Vec2::new(movement.velocity.x, movement.velocity.z).length();
    assert!(
        h_speed >= 5.0,
        "Player should have strong horizontal momentum, got {}",
        h_speed
    );
}

#[test]
fn test_moose_window_shutters_rotate_inward_on_peek() {
    use rust_tests::elf_game::moose::toy_model::{
        LeftWindowShutter, RightWindowShutter, MrMoose, MooseHead, animate_mr_moose_peeking_system,
    };

    let mut app = App::new();
    app.insert_resource(MooseHunger {
        value: 10.0,
        stage: MooseHungerStage::BehindShutters,
        is_breached: false,
        ..default()
    });
    app.add_systems(Update, animate_mr_moose_peeking_system);

    let left_shutter_entity = app.world_mut().spawn((
        LeftWindowShutter,
        Transform::from_xyz(-7.4, 2.9, -13.0),
    )).id();

    let right_shutter_entity = app.world_mut().spawn((
        RightWindowShutter,
        Transform::from_xyz(-5.6, 2.9, -13.0),
    )).id();

    let _moose_entity = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(-6.5, 1.8, -17.0),
    )).id();

    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    )).id();

    // 1. Stage 1: BehindShutters (hunger 10.0) -> Shutters completely closed (Quat::IDENTITY)
    app.update();
    {
        let left_tf = app.world().get::<Transform>(left_shutter_entity).unwrap();
        let right_tf = app.world().get::<Transform>(right_shutter_entity).unwrap();
        assert_eq!(left_tf.rotation, Quat::IDENTITY, "Left shutter must be shut at hunger 10");
        assert_eq!(right_tf.rotation, Quat::IDENTITY, "Right shutter must be shut at hunger 10");
    }

    // 2. Stage 2: PeekingThrough (hunger 35.0, progress = 0.5)
    // Shutters must rotate inward into the room (+Z direction) as head pokes forward
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 35.0;
        hunger.stage = MooseHungerStage::PeekingThrough;
    }
    app.update();
    {
        let left_tf = app.world().get::<Transform>(left_shutter_entity).unwrap();
        let right_tf = app.world().get::<Transform>(right_shutter_entity).unwrap();

        let (left_yaw, _, _) = left_tf.rotation.to_euler(EulerRot::YXZ);
        let (right_yaw, _, _) = right_tf.rotation.to_euler(EulerRot::YXZ);

        assert!(left_yaw < -0.3, "Left shutter rotates inward around Y with negative yaw, got {}", left_yaw);
        assert!(right_yaw > 0.3, "Right shutter rotates inward around Y with positive yaw, got {}", right_yaw);

        // Verify the shutter tips move into +Z (into the workshop interior from Z = -13.0)
        let left_tip_local = left_tf.rotation * Vec3::new(0.88, 0.0, 0.0);
        let right_tip_local = right_tf.rotation * Vec3::new(-0.88, 0.0, 0.0);
        assert!(
            left_tip_local.z > 0.2,
            "Left shutter tip must protrude into +Z interior of workshop, got {}",
            left_tip_local.z
        );
        assert!(
            right_tip_local.z > 0.2,
            "Right shutter tip must protrude into +Z interior of workshop, got {}",
            right_tip_local.z
        );

        let head_tf = app.world().get::<Transform>(head_entity).unwrap();
        assert!(head_tf.translation.z > 2.0, "Moose head pokes forward into the room");
    }

    // 3. Stage 4: WindowSlammed (hunger 60.0) -> Shutters slam shut
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
    }
    app.update();
    {
        let left_tf = app.world().get::<Transform>(left_shutter_entity).unwrap();
        let right_tf = app.world().get::<Transform>(right_shutter_entity).unwrap();
        assert_eq!(left_tf.rotation, Quat::IDENTITY, "Left shutter slams shut at hunger 60");
        assert_eq!(right_tf.rotation, Quat::IDENTITY, "Right shutter slams shut at hunger 60");
    }

    // 4. Stage 5: BreachedRampage -> Shutters blown wide open
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.stage = MooseHungerStage::BreachedRampage;
        hunger.is_breached = true;
    }
    app.update();
    {
        let left_tf = app.world().get::<Transform>(left_shutter_entity).unwrap();
        let right_tf = app.world().get::<Transform>(right_shutter_entity).unwrap();
        let (left_yaw, _, _) = left_tf.rotation.to_euler(EulerRot::YXZ);
        let (right_yaw, _, _) = right_tf.rotation.to_euler(EulerRot::YXZ);
        assert!(left_yaw <= -1.8, "Left shutter blown wide open during breach, got {}", left_yaw);
        assert!(right_yaw >= 1.8, "Right shutter blown wide open during breach, got {}", right_yaw);
    }
}
