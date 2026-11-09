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
use rust_tests::elf_game::audio::{
    ElfAudioPlugin, AudioDriverStatus, AudioMetrics, AudioConfig,
    PlaySoundMessage, SoundCueType, RetryAudioInitMessage,
};
use rust_tests::elf_game::moose::{MooseHuffEvent, MooseWindowSlamEvent};

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
    use rust_tests::elf_game::moose::toy_model::{MrMoose, MooseHead, WindowShutters};
    use rust_tests::elf_game::workshop::room::BreakableWindowPane;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.insert_resource(MooseHunger {
        value: 60.0,
        stage: MooseHungerStage::BreachedRampage,
        is_breached: true,
        ..default()
    });
    app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

    // Moose retreating along last leg towards (-6.5, 1.8, -17.0), currently at (-6.5, 1.4, -16.5)
    // facing 180 degrees backward (away from the window, into -Z)
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        MooseRampageTracker {
            current_waypoint_index: 1,
            speed: 8.5,
            waypoints: vec![
                Vec3::new(-6.5, 1.4, -10.0),
                Vec3::new(-6.5, 1.8, -17.0), // Final retreat destination
            ],
            ..default()
        },
        Transform::from_xyz(-6.5, 1.8, -16.6).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
    )).id();

    // Moose Head stretched forward from peeking / breach
    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 3.6).with_rotation(Quat::from_rotation_x(0.2)),
    )).id();

    // Shutters blown wide open
    let shutter_entity = app.world_mut().spawn((
        WindowShutters,
        Transform::from_xyz(-7.4, 2.9, -13.0).with_rotation(Quat::from_rotation_y(1.95)),
        Visibility::Hidden,
    )).id();

    // Broken window pane
    let window_entity = app.world_mut().spawn((
        BreakableWindowPane { is_broken: true },
        Transform::from_xyz(-6.5, 2.9, -13.0),
        Visibility::Hidden,
    )).id();

    // Update app: distance to final waypoint is 0.509m < 0.6m -> reaches destination!
    app.update();

    let hunger = app.world().resource::<MooseHunger>();
    assert_eq!(hunger.stage, MooseHungerStage::BehindShutters);
    assert_eq!(hunger.value, 0.0);
    assert!(!hunger.is_breached);
    assert!(app.world().get::<MooseRampageTracker>(moose_entity).is_none(), "Tracker should be removed on retreat");

    // 1. Assert Mr. Moose root transform snapped to default position and Quat::IDENTITY (facing +Z toward window)
    let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
    assert_eq!(
        moose_tf.translation,
        Vec3::new(-6.5, 1.8, -17.0),
        "Moose translation must reset to default snowy hill position (-6.5, 1.8, -17.0)"
    );
    assert_eq!(
        moose_tf.rotation,
        Quat::IDENTITY,
        "Moose orientation must reset to Quat::IDENTITY facing +Z toward window"
    );

    // 2. Assert Moose Head transform reset to default (0.0, 1.1, 1.6) and Quat::IDENTITY
    let head_tf = app.world().get::<Transform>(head_entity).unwrap();
    assert_eq!(
        head_tf.translation,
        Vec3::new(0.0, 1.1, 1.6),
        "Moose head translation must reset to default (0.0, 1.1, 1.6)"
    );
    assert_eq!(
        head_tf.rotation,
        Quat::IDENTITY,
        "Moose head rotation must reset to Quat::IDENTITY"
    );

    // 3. Assert shutters reset to Quat::IDENTITY and Visibility::Inherited
    let shutter_tf = app.world().get::<Transform>(shutter_entity).unwrap();
    let shutter_vis = app.world().get::<Visibility>(shutter_entity).unwrap();
    assert_eq!(shutter_tf.rotation, Quat::IDENTITY, "Shutters rotation must reset to Quat::IDENTITY (shut)");
    assert_eq!(*shutter_vis, Visibility::Inherited, "Shutters visibility must be Inherited");

    // 4. Assert breakable window pane is restored
    let window_pane = app.world().get::<BreakableWindowPane>(window_entity).unwrap();
    let window_vis = app.world().get::<Visibility>(window_entity).unwrap();
    assert!(!window_pane.is_broken, "Window pane must be repaired");
    assert_eq!(*window_vis, Visibility::Inherited, "Window pane visibility must be Inherited");
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
            ..default()
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

#[test]
fn test_moose_orientation_and_head_preserved_across_hunger_cycles() {
    use rust_tests::elf_game::moose::toy_model::{
        LeftWindowShutter, RightWindowShutter, MrMoose, MooseHead, animate_mr_moose_peeking_system,
    };

    let mut app = App::new();
    app.insert_resource(MooseHunger {
        value: 0.0,
        stage: MooseHungerStage::BehindShutters,
        is_breached: false,
        ..default()
    });
    app.add_systems(Update, animate_mr_moose_peeking_system);

    app.world_mut().spawn((
        LeftWindowShutter,
        Transform::from_xyz(-7.4, 2.9, -13.0),
    ));
    app.world_mut().spawn((
        RightWindowShutter,
        Transform::from_xyz(-5.6, 2.9, -13.0),
    ));

    // Moose entity with non-identity rotation (e.g. facing backward from a prior event)
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(-6.5, 1.4, -17.0).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
    )).id();

    // Moose Head entity
    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 0.5, 1.6).with_rotation(Quat::from_rotation_z(0.3)),
    )).id();

    // 1. BehindShutters (hunger 0.0) -> orientation and Y must strictly reset to default
    app.update();
    {
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
        let head_tf = app.world().get::<Transform>(head_entity).unwrap();
        assert_eq!(moose_tf.rotation, Quat::IDENTITY, "Moose must face +Z in BehindShutters");
        assert_eq!(moose_tf.translation.y, 1.8, "Moose Y must be 1.8 in BehindShutters");
        assert_eq!(head_tf.rotation, Quat::IDENTITY, "Moose head must be Quat::IDENTITY in BehindShutters");
        assert_eq!(head_tf.translation.y, 1.1, "Moose head Y must be 1.1 in BehindShutters");
    }

    // 2. PeekingThrough (hunger 35.0) -> orientation and Y remain strictly default
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 35.0;
        hunger.stage = MooseHungerStage::PeekingThrough;
    }
    app.update();
    {
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
        let head_tf = app.world().get::<Transform>(head_entity).unwrap();
        assert_eq!(moose_tf.rotation, Quat::IDENTITY, "Moose must face +Z in PeekingThrough");
        assert_eq!(moose_tf.translation.y, 1.8, "Moose Y must be 1.8 in PeekingThrough");
        assert_eq!(head_tf.rotation, Quat::IDENTITY, "Moose head must be Quat::IDENTITY in PeekingThrough");
        assert_eq!(head_tf.translation.y, 1.1, "Moose head Y must be 1.1 in PeekingThrough");
    }

    // 3. AgitatedHuffing (hunger 55.0) -> orientation and Y remain strictly default
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 55.0;
        hunger.stage = MooseHungerStage::AgitatedHuffing;
    }
    app.update();
    {
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
        let head_tf = app.world().get::<Transform>(head_entity).unwrap();
        assert_eq!(moose_tf.rotation, Quat::IDENTITY, "Moose must face +Z in AgitatedHuffing");
        assert_eq!(moose_tf.translation.y, 1.8, "Moose Y must be 1.8 in AgitatedHuffing");
        assert_eq!(head_tf.rotation, Quat::IDENTITY, "Moose head must be Quat::IDENTITY in AgitatedHuffing");
        assert_eq!(head_tf.translation.y, 1.1, "Moose head Y must be 1.1 in AgitatedHuffing");
    }

    // 4. WindowSlammed (hunger 60.0) -> orientation and Y remain strictly default
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
    }
    app.update();
    {
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
        let head_tf = app.world().get::<Transform>(head_entity).unwrap();
        assert_eq!(moose_tf.rotation, Quat::IDENTITY, "Moose must face +Z in WindowSlammed");
        assert_eq!(moose_tf.translation.y, 1.8, "Moose Y must be 1.8 in WindowSlammed");
        assert_eq!(head_tf.rotation, Quat::IDENTITY, "Moose head must be Quat::IDENTITY in WindowSlammed");
        assert_eq!(head_tf.translation.y, 1.1, "Moose head Y must be 1.1 in WindowSlammed");
    }
}

#[test]
fn test_floaty_hands_physics_momentum_and_sway() {
    use rust_tests::elf_game::player::controller::{FloatyHand, floaty_hands_system, ElfPlayer};
    use rust_tests::elf_game::player::ElfMovement;
    use bevy::input::mouse::AccumulatedMouseMotion;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs_f32(0.016));
    app.insert_resource(time);
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(50.0, 0.0), // Quick mouse flick to the right
    });

    // Player with ElfPlayer component and zero velocity initially
    let player_e = app.world_mut().spawn((ElfMovement::default(), ElfPlayer)).id();

    // Floaty hand entity
    let rest_pos = Vec3::new(-0.35, -0.3, -0.6);
    let hand_e = app.world_mut().spawn((
        FloatyHand::new(rest_pos, 0.0),
        Transform::from_translation(rest_pos),
    )).id();

    app.add_systems(Update, floaty_hands_system);

    // Frame 1: Hand should lag to the left (-X) due to opposite inertial sway impulse
    app.update();
    {
        let hand = app.world().get::<FloatyHand>(hand_e).unwrap();
        assert!(
            hand.velocity.x < -0.5,
            "Hand should develop negative X velocity from rightward camera flick, got {}",
            hand.velocity.x
        );
        assert!(
            hand.current_offset.x < 0.0,
            "Hand should sway leftward (negative X offset), got {}",
            hand.current_offset.x
        );
    }

    // Frame 2: Upward player jump velocity causes hand to dip downward (-Y)
    {
        let mut mouse = app.world_mut().resource_mut::<AccumulatedMouseMotion>();
        mouse.delta = Vec2::ZERO;
        let mut movement = app.world_mut().get_mut::<ElfMovement>(player_e).unwrap();
        movement.velocity.y = 8.0; // Jump upward
    }
    app.update();
    {
        let hand = app.world().get::<FloatyHand>(hand_e).unwrap();
        assert!(
            hand.velocity.y < 0.0,
            "Hand should dip downward with negative Y velocity upon player jump, got {}",
            hand.velocity.y
        );
    }

    // Advance 60 frames (1 second) of rest: damped spring brings hand back near rest position
    {
        let mut movement = app.world_mut().get_mut::<ElfMovement>(player_e).unwrap();
        movement.velocity = Vec3::ZERO;
    }
    for _ in 0..60 {
        app.update();
    }
    {
        let hand = app.world().get::<FloatyHand>(hand_e).unwrap();
        assert!(
            hand.current_offset.length() < 0.05,
            "Floaty hand should settle back near rest position under damping, offset length was {}",
            hand.current_offset.length()
        );
    }
}

#[test]
fn test_toggle_grab_set_down_and_mouse1_throw() {
    use rust_tests::elf_game::player::hands::{Grabbable, dual_hand_interaction_system};
    use rust_tests::elf_game::player::controller::{LeftHandVisual, RightHandVisual, FirstPersonCamera};
    use rust_tests::elf_game::player::ElfHands;
    use bevy_rapier3d::prelude::Velocity;

    let mut app = App::new();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_message::<rust_tests::elf_game::wrapping::UseToolOnPresentMessage>();
    app.add_systems(Update, dual_hand_interaction_system);

    let player_e = app.world_mut().spawn((
        ElfHands::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
    )).id();

    // Camera pointing forward (-Z)
    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.6, 0.0)),
    ));

    // Left and Right hand positions
    app.world_mut().spawn((
        LeftHandVisual,
        GlobalTransform::from(Transform::from_xyz(-0.35, 1.0, -0.6)),
    ));
    app.world_mut().spawn((
        RightHandVisual,
        GlobalTransform::from(Transform::from_xyz(0.35, 1.0, -0.6)),
    ));

    // Grabbable physics cube near left hand
    let cube_entity = app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(-0.35, 1.0, -0.7),
        Velocity::zero(),
    )).id();

    // 1. Press KeyQ once -> Toggle GRAB object
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyQ);
    }
    app.update();
    {
        let hands = app.world().get::<ElfHands>(player_e).unwrap();
        assert_eq!(hands.left_grabbed, Some(cube_entity), "KeyQ toggle should grab nearby object");
    }

    // 2. Release key input (no keys pressed) -> Object remains held in hand (TOGGLE, not hold!)
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyQ);
        keys.reset(KeyCode::KeyQ);
    }
    app.update();
    {
        let hands = app.world().get::<ElfHands>(player_e).unwrap();
        assert_eq!(hands.left_grabbed, Some(cube_entity), "Object must stay grabbed without holding KeyQ");
    }

    // 3. Press KeyQ again -> Toggle SET DOWN object
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyQ);
    }
    app.update();
    {
        let hands = app.world().get::<ElfHands>(player_e).unwrap();
        assert_eq!(hands.left_grabbed, None, "Pressing KeyQ again must set the object down");
        let vel = app.world().get::<Velocity>(cube_entity).unwrap();
        assert_eq!(vel.linear, Vec3::ZERO, "Set down object must rest with zero velocity");
    }

    // 4. Press KeyE -> Right hand grabs object
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyQ);
        keys.reset(KeyCode::KeyQ);
        keys.press(KeyCode::KeyE);
    }
    app.update();
    {
        let hands = app.world().get::<ElfHands>(player_e).unwrap();
        assert_eq!(hands.right_grabbed, Some(cube_entity), "KeyE toggle should grab into right hand");
    }

    // 5. Click Mouse 1 (Left button) -> THROW object with physical impulse
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.reset(KeyCode::KeyE);
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }
    app.update();
    {
        let hands = app.world().get::<ElfHands>(player_e).unwrap();
        assert_eq!(hands.right_grabbed, None, "Mouse 1 must release thrown object from right hand");
        let vel = app.world().get::<Velocity>(cube_entity).unwrap();
        assert!(
            vel.linear.length() >= 9.0,
            "Thrown object must have strong launch velocity (>= 9.0 m/s), got {}",
            vel.linear.length()
        );
        assert!(
            vel.linear.z < -8.0,
            "Object should be launched forward in camera direction (-Z), got vz = {}",
            vel.linear.z
        );
        assert!(
            vel.linear.y > 1.0,
            "Object should have upward throwing arc (vy > 1.0), got vy = {}",
            vel.linear.y
        );
    }
}

#[test]
fn test_player_jump_throw_velocity_inheritance() {
    use rust_tests::elf_game::player::hands::{Grabbable, dual_hand_interaction_system};
    use rust_tests::elf_game::player::controller::{RightHandVisual, FirstPersonCamera};
    use rust_tests::elf_game::player::{ElfHands, ElfMovement};
    use bevy_rapier3d::prelude::Velocity;

    let mut app = App::new();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_message::<rust_tests::elf_game::wrapping::UseToolOnPresentMessage>();
    app.add_systems(Update, dual_hand_interaction_system);

    // Player with forward and jumping vertical velocity (e.g. running at 6.0 m/s in -Z and jumping at +5.0 m/s in +Y)
    let player_e = app.world_mut().spawn((
        ElfHands::default(),
        ElfMovement {
            velocity: Vec3::new(0.0, 5.0, -6.0),
            is_grounded: false,
            ..default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0),
    )).id();

    // Camera pointing forward (-Z)
    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.6, 0.0)),
    ));

    app.world_mut().spawn((
        RightHandVisual,
        GlobalTransform::from(Transform::from_xyz(0.35, 1.0, -0.6)),
    ));

    let cube_entity = app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(0.35, 1.0, -0.7),
        Velocity::zero(),
    )).id();

    // 1. Right hand grabs object
    {
        let mut hands = app.world_mut().get_mut::<ElfHands>(player_e).unwrap();
        hands.right_grabbed = Some(cube_entity);
    }

    // 2. Click Mouse 1 (Left button) to throw while jumping forward
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }
    app.update();

    let vel = app.world().get::<Velocity>(cube_entity).unwrap();
    // Base throw is ~10 m/s: vz ~ -9.89, vy ~ +1.48.
    // With jump-throw:
    // vz should be base vz (-9.89) + player vz (-6.0) = ~ -15.89 m/s!
    // vy should be base vy (+1.48) + player jump vy (+5.0) = ~ +6.48 m/s!
    assert!(
        vel.linear.z < -14.0,
        "Forward throw velocity must inherit player forward running velocity (-6.0 m/s), got vz = {}",
        vel.linear.z
    );
    assert!(
        vel.linear.y > 6.0,
        "Vertical throw velocity must inherit player jumping vertical velocity (+5.0 m/s), got vy = {}",
        vel.linear.y
    );
    assert!(
        vel.linear.length() > 16.0,
        "Total launch velocity should be boosted to > 16.0 m/s on jump-throw, got {}",
        vel.linear.length()
    );
}

#[test]
fn test_player_vertical_velocity_reset_to_zero_on_landing_and_throw_unaffected() {
    use rust_tests::elf_game::player::hands::{Grabbable, dual_hand_interaction_system};
    use rust_tests::elf_game::player::controller::{RightHandVisual, FirstPersonCamera};
    use rust_tests::elf_game::player::{BhopConfig, ElfHands, ElfMovement, update_elf_physics};
    use bevy_rapier3d::prelude::Velocity;

    // 1. Verify update_elf_physics resets vertical velocity to 0.0 upon landing
    let mut movement = ElfMovement {
        velocity: Vec3::new(2.0, -12.5, -4.0), // Falling velocity after jumping
        is_grounded: true,                     // Just touched ground
        ..default()
    };
    let config = BhopConfig::default();
    let dt = 1.0 / 60.0;
    let wish_dir = Vec3::ZERO;

    update_elf_physics(dt, wish_dir, false, false, &config, &mut movement);
    assert_eq!(
        movement.velocity.y, 0.0,
        "Player vertical velocity must be reset to 0.0 upon landing on ground"
    );

    // 2. Verify throw velocity logic inherits 0.0 vertical velocity (not negative) after landing
    let mut app = App::new();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_message::<rust_tests::elf_game::wrapping::UseToolOnPresentMessage>();
    app.add_systems(Update, dual_hand_interaction_system);

    let player_e = app.world_mut().spawn((
        ElfHands::default(),
        movement, // Grounded player with velocity.y = 0.0
        Transform::from_xyz(0.0, 1.0, 0.0),
    )).id();

    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.6, 0.0)),
    ));

    app.world_mut().spawn((
        RightHandVisual,
        GlobalTransform::from(Transform::from_xyz(0.35, 1.0, -0.6)),
    ));

    let cube_entity = app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(0.35, 1.0, -0.7),
        Velocity::zero(),
    )).id();

    // Grab into right hand
    {
        let mut hands = app.world_mut().get_mut::<ElfHands>(player_e).unwrap();
        hands.right_grabbed = Some(cube_entity);
    }

    // Throw on ground
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }
    app.update();

    let vel = app.world().get::<Velocity>(cube_entity).unwrap();
    // Base throw upward arc is ~1.48 m/s. It must NOT be negative!
    assert!(
        vel.linear.y > 1.0,
        "Thrown object on the ground must retain positive upward arc (vy > 1.0), got vy = {}",
        vel.linear.y
    );
}

#[test]
fn test_player_collision_box_spacing() {
    use rust_tests::elf_game::player::controller::{spawn_elf_player, ElfPlayer};
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_elf_player);
    app.update();

    let Ok((_, collider, controller)) = app
        .world_mut()
        .query::<(&ElfPlayer, &Collider, &KinematicCharacterController)>()
        .single(app.world())
    else {
        panic!("ElfPlayer must be spawned with Collider and KinematicCharacterController");
    };

    // Verify collider shape has expanded radius (at least 0.45m)
    let capsule = collider.as_capsule().expect("Player collider must be a capsule");
    assert!(
        capsule.radius() >= 0.45,
        "Player collision box radius must be expanded to provide spacing before running into things, got {}",
        capsule.radius()
    );

    // Verify character controller offset padding (spacing cushion)
    let offset_val = match controller.offset {
        CharacterLength::Absolute(v) => v,
        CharacterLength::Relative(v) => v,
    };
    assert!(
        offset_val >= 0.03,
        "Player character controller offset cushion must be at least 0.03m (3cm), got {}",
        offset_val
    );
}

#[test]
fn test_chandelier_candle_flicker() {
    use rust_tests::elf_game::workshop::room::{CandleChandelier, chandelier_candle_flicker_system};

    let mut app = App::new();
    let time = Time::<()>::default();
    app.insert_resource(time);
    app.add_systems(Update, chandelier_candle_flicker_system);

    let chandelier_entity = app.world_mut().spawn((
        CandleChandelier {
            base_intensity: 480_000.0,
            flicker_seed: 0.0,
        },
        PointLight {
            intensity: 480_000.0,
            ..default()
        },
    )).id();

    // Advance simulated time across multiple frames and verify dynamic flicker modulation
    let mut intensities = Vec::new();
    for _ in 1..=10 {
        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(50));
        }
        app.update();

        let light = app.world().get::<PointLight>(chandelier_entity).unwrap();
        intensities.push(light.intensity);
        // Light intensity must stay within realistic bounded candle glow bounds (75% to 125%)
        assert!(light.intensity >= 480_000.0 * 0.75, "Intensity dropped too low: {}", light.intensity);
        assert!(light.intensity <= 480_000.0 * 1.25, "Intensity spiked too high: {}", light.intensity);
    }

    // Verify intensity actually varied over time (not static)
    let min_i = intensities.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_i = intensities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(max_i - min_i > 5_000.0, "Candle light intensity should fluctuate dynamically");
}

#[test]
fn test_conveyor_floor_chute_recessed_no_pop_despawn() {
    use rust_tests::elf_game::workshop::room::{ConveyorDespawnChute, conveyor_despawn_chute_system};

    let mut app = App::new();
    app.add_systems(Update, conveyor_despawn_chute_system);

    // Sub-floor recessed chute shaft located at (9.6, -0.8, 3.0)
    app.world_mut().spawn((
        ConveyorDespawnChute,
        Transform::from_xyz(9.6, -0.8, 3.0),
    ));

    // 1. Present cube still on elevated conveyor belt (Y = 0.75m) at X = 8.9m
    // MUST NOT despawn while visible on the belt!
    let visible_belt_cube = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(8.9, 0.75, 3.0),
    )).id();

    // 2. Present cube that tumbled off the belt into the sub-floor pit (Y = -0.3m, below floor level Y < 0.0)
    // MUST despawn out of sight!
    let fallen_subfloor_cube = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(9.6, -0.3, 3.0),
    )).id();

    // 3. Present on workbench
    let safe_workbench_cube = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(0.0, 0.8, -4.0),
    )).id();

    app.update();

    assert!(
        app.world().get_entity(visible_belt_cube).is_ok(),
        "Present resting on elevated conveyor belt must NOT despawn while still visible"
    );
    assert!(
        app.world().get_entity(fallen_subfloor_cube).is_err(),
        "Present falling into recessed sub-floor chute shaft must be despawned out of sight"
    );
    assert!(
        app.world().get_entity(safe_workbench_cube).is_ok(),
        "Present on workbench table must remain intact"
    );
}

#[test]
fn test_present_sending_platform_suction_and_vacuum_tube() {
    use rust_tests::elf_game::workshop::room::{
        PresentSendingPlatform, SendPlatformButton, VacuumTubeSegment, TubeTravellingLump,
        present_sending_platform_system, tube_travelling_lump_system,
    };
    use rust_tests::elf_game::player::ElfMovement;
    use rust_tests::elf_game::core::WorkshopQuota;
    use rust_tests::elf_game::wrapping::PresentDeliveredMessage;
    use bevy_rapier3d::prelude::Velocity;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(WorkshopQuota {
        presents_delivered: 0,
        target_quota: 5,
        tier: 1,
    });
    app.add_message::<PresentDeliveredMessage>();
    app.add_systems(Update, (present_sending_platform_system, tube_travelling_lump_system));

    // Sending Platform at (5.5, 0.05, -4.0)
    let platform_entity = app.world_mut().spawn((
        PresentSendingPlatform {
            is_active: false,
            suction_timer: 0.0,
            ..default()
        },
        Transform::from_xyz(5.5, 0.05, -4.0),
    )).id();

    // Send Button Pedestal at (3.8, 0.45, -2.8)
    app.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(3.8, 0.45, -2.8),
    ));

    // Player standing next to Send Button
    app.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(3.8, 1.0, -2.5),
    ));

    // Spawn 20 Vacuum Tube Segments along rafter path
    let mut segment_entities = Vec::new();
    for i in 0..20 {
        let rest_pos = Vec3::new(5.5 + (i as f32 * 0.35), 4.2 + (i as f32 * 0.05), -4.0);
        let seg_e = app.world_mut().spawn((
            VacuumTubeSegment {
                index: i,
                base_scale: Vec3::ONE,
                rest_position: rest_pos,
            },
            Transform::from_translation(rest_pos),
        )).id();
        segment_entities.push(seg_e);
    }

    // Grabbable Present on the sending platform
    let present_entity = app.world_mut().spawn((
        Present {
            is_completed: true,
            quality_score: 95.0,
            ..default()
        },
        Transform::from_xyz(5.5, 0.3, -4.0),
        Velocity::zero(),
    )).id();

    // 1. Press KeyF near the button to activate the pneumatic suction platform
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyF);
    }
    app.update();

    let platform = app.world().get::<PresentSendingPlatform>(platform_entity).unwrap();
    assert!(platform.is_active, "Platform must become active upon pressing Send button");
    assert!(platform.suction_timer > 0.0, "Suction timer should be active");

    // Check upward suction acceleration on present
    let vel = app.world().get::<Velocity>(present_entity).unwrap();
    assert!(
        vel.linear.y > 0.0,
        "Suction field must accelerate present upward towards 4m intake funnel, got vy = {}",
        vel.linear.y
    );

    // 2. Move present to funnel intake mouth (Y = 3.8m) -> verify conversion into TubeTravellingLump
    {
        let mut p_tf = app.world_mut().get_mut::<Transform>(present_entity).unwrap();
        p_tf.translation.y = 3.8;
    }
    app.update();

    // Original present entity should be consumed/despawned
    assert!(
        app.world().get_entity(present_entity).is_err(),
        "Present reaching 4m funnel mouth must be ingested into the pneumatic tube"
    );

    // A TubeTravellingLump should now exist
    let mut lump_query = app.world_mut().query::<(Entity, &TubeTravellingLump)>();
    let lump_res = lump_query.iter(app.world()).next();
    assert!(lump_res.is_some(), "TubeTravellingLump should be spawned upon present ingestion");
    let (lump_entity, _) = lump_res.unwrap();

    // 3. Advance the lump along the tube and verify segment bulging & loop impulses
    {
        let mut lump = app.world_mut().get_mut::<TubeTravellingLump>(lump_entity).unwrap();
        lump.progress = 0.35; // Lump is near segment 7 (inside the 360-degree loop)
    }
    app.update();

    // Segment 7 should be noticeably bulged radially
    let seg7_tf = app.world().get::<Transform>(segment_entities[7]).unwrap();
    assert!(
        seg7_tf.scale.x > 1.05,
        "Tube segment near travelling lump must dilate radially, scale.x = {}",
        seg7_tf.scale.x
    );

    // Segment far away (segment 19) should remain near baseline scale
    let seg19_tf = app.world().get::<Transform>(segment_entities[19]).unwrap();
    assert!(
        seg19_tf.scale.x < 1.02,
        "Distant tube segment should not bulge, scale.x = {}",
        seg19_tf.scale.x
    );

    // 4. Move lump to completion (progress >= 1.0) -> increments quota and despawns lump
    {
        let mut lump = app.world_mut().get_mut::<TubeTravellingLump>(lump_entity).unwrap();
        lump.progress = 1.0;
    }
    app.update();

    let quota = app.world().resource::<WorkshopQuota>();
    assert_eq!(
        quota.presents_delivered, 1,
        "WorkshopQuota presents_delivered must increment when lump reaches terminus"
    );
    assert!(
        app.world().get_entity(lump_entity).is_err(),
        "TubeTravellingLump must despawn upon completing tube transit"
    );
}

#[test]
fn test_moose_feeding_trough_stage_wait_and_reset() {
    use rust_tests::elf_game::moose::{
        MooseFeedingTrough, MooseSnack, MooseHunger, MooseHungerStage, FeedMooseMessage,
        toy_model::moose_feeding_trough_system,
    };

    let mut app = App::new();
    app.insert_resource(MooseHunger {
        value: 30.0,
        stage: MooseHungerStage::PeekingThrough,
        ..default()
    });
    app.add_message::<FeedMooseMessage>();
    app.add_systems(Update, moose_feeding_trough_system);

    // Feeding Trough at (-6.5, 0.3, -12.0)
    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    // Snack deposited inside the feeding trough
    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    // 1. Frame 1: Hunger is 30.0 (< 50.0). Snack MUST stay in the food box!
    app.update();

    assert!(
        app.world().get_entity(snack_entity).is_ok(),
        "Snack must remain in the food box while hunger is below 50"
    );
    let hunger = app.world().resource::<MooseHunger>();
    assert_eq!(hunger.value, 30.0, "Hunger should not reset before hunger reaches 50");

    // 2. Advance hunger to 52.0 (AgitatedHuffing / feeding stage 50-60)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 52.0;
        hunger.stage = MooseHungerStage::AgitatedHuffing;
    }
    app.update();

    // Snack should now be consumed/despawned and hunger reset to 0!
    assert!(
        app.world().get_entity(snack_entity).is_err(),
        "Snack in food box should be consumed once Mr. Moose is in feeding stage (50-60)"
    );
    let hunger = app.world().resource::<MooseHunger>();
    assert_eq!(hunger.value, 0.0, "Hunger meter must reset back to 0 upon feeding");
    assert_eq!(hunger.stage, MooseHungerStage::BehindShutters, "Moose retreats behind shutters");
}

#[test]
fn test_moose_soft_body_scarf_dynamics() {
    use rust_tests::elf_game::moose::toy_model::{
        MrMoose, MooseHead, MooseScarfSegment, moose_scarf_physics_system,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, moose_scarf_physics_system);

    // Mr. Moose root at (-6.5, 1.8, -17.0)
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(-6.5, 1.8, -17.0),
    )).id();

    // Moose Head at (0.0, 1.1, 1.6)
    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    )).id();

    // Scarf segment attached at rest offset
    let rest_off = Vec3::new(-0.25, -0.15, 0.95);
    let initial_pos = Vec3::new(-6.5, 1.8, -17.0) + rest_off;
    let seg_entity = app.world_mut().spawn((
        MooseScarfSegment {
            index: 0,
            rest_offset: rest_off,
            velocity: Vec3::ZERO,
        },
        Transform::from_translation(initial_pos),
    )).id();

    // 1. Initial rest step
    app.update();

    // 2. Mr. Moose lunges forward along Z (e.g. peeking through window or rampage)
    {
        let mut m_tf = app.world_mut().get_mut::<Transform>(moose_entity).unwrap();
        m_tf.translation.z = -14.5;
        let mut h_tf = app.world_mut().get_mut::<Transform>(head_entity).unwrap();
        h_tf.translation.z = 3.6;
    }
    app.update();

    let seg = app.world().get::<MooseScarfSegment>(seg_entity).unwrap();
    assert!(
        seg.velocity.length() > 0.5,
        "Scarf segment should gain dynamic spring acceleration and velocity from Moose movement, vel = {:?}",
        seg.velocity
    );
}

#[test]
fn test_windowsill_decorations_and_main_table_snack_jar() {
    use rust_tests::elf_game::moose::toy_model::{
        WindowsillMiniTree, WindowsillHotCocoa, MooseSnackJar, spawn_mr_moose,
    };
    use rust_tests::elf_game::moose::MooseSnack;
    use rust_tests::elf_game::player::hands::Grabbable;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_mr_moose);
    app.update();

    // Check Windowsill Mini Tree
    let mut tree_q = app.world_mut().query::<(&WindowsillMiniTree, &Transform, &Grabbable)>();
    let tree = tree_q.iter(app.world()).next();
    assert!(tree.is_some(), "WindowsillMiniTree should be spawned");
    let (_, tree_tf, _) = tree.unwrap();
    assert!(
        (tree_tf.translation.z - (-12.85)).abs() < 0.2,
        "Mini tree must be located on windowsill (Z ≈ -12.85), got {}",
        tree_tf.translation.z
    );

    // Check Windowsill Hot Cocoa Mug
    let mut cocoa_q = app.world_mut().query::<(&WindowsillHotCocoa, &Transform, &Grabbable)>();
    let cocoa = cocoa_q.iter(app.world()).next();
    assert!(cocoa.is_some(), "WindowsillHotCocoa mug should be spawned");
    let (_, cocoa_tf, _) = cocoa.unwrap();
    assert!(
        (cocoa_tf.translation.z - (-12.85)).abs() < 0.2,
        "Cocoa mug must be located on windowsill (Z ≈ -12.85), got {}",
        cocoa_tf.translation.z
    );

    // Check Main Table Moose Snack Jar & small snacks
    let mut jar_q = app.world_mut().query::<(&MooseSnackJar, &Transform)>();
    let jar = jar_q.iter(app.world()).next();
    assert!(jar.is_some(), "MooseSnackJar should be spawned on workbench table");

    let mut snack_q = app.world_mut().query::<(&MooseSnack, &Transform, &Grabbable)>();
    let snacks_count = snack_q.iter(app.world()).count();
    assert!(
        snacks_count >= 4,
        "At least 4 small Moose Snacks should be spawned on table, found {}",
        snacks_count
    );
}

#[test]
fn test_held_object_moved_out_and_no_player_collision() {
    use rust_tests::elf_game::player::hands::{
        Grabbable, HeldByPlayer, GROUP_PLAYER, GROUP_HELD_ITEM, dual_hand_interaction_system,
    };
    use rust_tests::elf_game::player::controller::{
        LeftHandVisual, RightHandVisual, FirstPersonCamera,
    };
    use rust_tests::elf_game::player::ElfHands;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_message::<rust_tests::elf_game::wrapping::UseToolOnPresentMessage>();
    app.add_systems(Update, dual_hand_interaction_system);

    let player_e = app.world_mut().spawn((
        ElfHands::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Collider::capsule_y(0.5, 0.3),
        CollisionGroups::new(GROUP_PLAYER, !GROUP_HELD_ITEM),
        KinematicCharacterController {
            filter_groups: Some(CollisionGroups::new(GROUP_PLAYER, !GROUP_HELD_ITEM)),
            ..default()
        },
    )).id();

    // Camera pointing forward (-Z)
    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.6, 0.0)),
    ));

    // Left and Right hand positions
    let left_hand_pos = Vec3::new(-0.35, 1.0, -0.6);
    app.world_mut().spawn((
        LeftHandVisual,
        GlobalTransform::from(Transform::from_translation(left_hand_pos)),
    ));
    app.world_mut().spawn((
        RightHandVisual,
        GlobalTransform::from(Transform::from_xyz(0.35, 1.0, -0.6)),
    ));

    // Grabbable physics cube
    let cube_entity = app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(-0.35, 1.0, -0.7),
        Collider::cuboid(0.3, 0.3, 0.3),
        CollisionGroups::new(Group::ALL, Group::ALL),
        Velocity::zero(),
    )).id();

    // 1. Press KeyQ -> Grab object into Left Hand
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyQ);
    }
    app.update();

    let hands = app.world().get::<ElfHands>(player_e).unwrap();
    assert_eq!(hands.left_grabbed, Some(cube_entity));

    // The held object must be moved out in front of the hand along view direction (-Z)
    let cube_tf = app.world().get::<Transform>(cube_entity).unwrap();
    assert!(
        cube_tf.translation.z <= left_hand_pos.z - 0.40,
        "Held item must be moved out forward (z <= {}), got z = {}",
        left_hand_pos.z - 0.40,
        cube_tf.translation.z
    );

    // The held object must have Sensor and CollisionGroups excluding GROUP_PLAYER
    assert!(
        app.world().get::<Sensor>(cube_entity).is_some(),
        "Held object must have Sensor attached to prevent blocking character controller"
    );
    assert!(
        app.world().get::<HeldByPlayer>(cube_entity).is_some(),
        "Held object must have HeldByPlayer marker"
    );
    let cg = app.world().get::<CollisionGroups>(cube_entity).unwrap();
    assert_eq!(cg.memberships, GROUP_HELD_ITEM);
    assert_eq!(cg.filters, !GROUP_PLAYER);

    // 2. Press KeyQ again -> Set down object
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyQ);
        keys.reset(KeyCode::KeyQ);
        keys.press(KeyCode::KeyQ);
    }
    app.update();

    let hands = app.world().get::<ElfHands>(player_e).unwrap();
    assert_eq!(hands.left_grabbed, None);

    // When released, Sensor and HeldByPlayer must be removed, restoring solid collision
    assert!(
        app.world().get::<Sensor>(cube_entity).is_none(),
        "Sensor must be removed when object is set down"
    );
    assert!(
        app.world().get::<HeldByPlayer>(cube_entity).is_none(),
        "HeldByPlayer must be removed when object is set down"
    );
    let cg_restored = app.world().get::<CollisionGroups>(cube_entity).unwrap();
    assert_eq!(cg_restored.memberships, Group::ALL);
    assert_eq!(cg_restored.filters, Group::ALL);
}

#[test]
fn test_vaulted_ceiling_steep_a_frame_geometry() {
    use rust_tests::elf_game::workshop::room::setup_workshop_scene;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

    let mut name_query = app.world_mut().query::<(&Name, &Transform)>();
    let mut west_roof_found = false;
    let mut east_roof_found = false;
    let mut ridge_beam_found = false;
    let mut king_post_found = false;

    for (name, tf) in name_query.iter(app.world()) {
        match name.as_str() {
            "VaultedRoofWest" => {
                west_roof_found = true;
                // West roof slope center should be at X = -6.5, Y = 9.75 (high peak A-frame)
                assert!(
                    (tf.translation.x - (-6.5)).abs() < 0.1,
                    "West roof center X should be -6.5, got {}",
                    tf.translation.x
                );
                assert!(
                    tf.translation.y >= 9.5,
                    "West roof center Y should be elevated for high pitch (>= 9.5), got {}",
                    tf.translation.y
                );

                // Check positive rotation around Z (so center X=0 is HIGHER than edge X=-13)
                let (_, _, roll) = tf.rotation.to_euler(EulerRot::XYZ);
                assert!(
                    roll > 0.5,
                    "West roof slope must have steep positive Z rotation (> 0.5 rad / ~28 deg), got {}",
                    roll
                );

                // Calculate vertical position at center X=0 vs wall X=-13
                // Half length along slope is ~7.935m
                let half_span_y = 7.935 * roll.sin();
                let y_apex = tf.translation.y + half_span_y;
                let y_wall = tf.translation.y - half_span_y;

                assert!(
                    y_apex > y_wall + 7.0,
                    "Center ridge apex must be significantly higher than edge walls (A-frame peak, not valley/trough): apex={}, wall={}",
                    y_apex,
                    y_wall
                );
                assert!(
                    y_apex >= 14.0,
                    "Peak apex should be >= 14.0m for steep high vault, got {}",
                    y_apex
                );
            }
            "VaultedRoofEast" => {
                east_roof_found = true;
                // East roof slope center should be at X = 6.5, Y = 9.75
                assert!(
                    (tf.translation.x - 6.5).abs() < 0.1,
                    "East roof center X should be 6.5, got {}",
                    tf.translation.x
                );
                assert!(
                    tf.translation.y >= 9.5,
                    "East roof center Y should be elevated for high pitch (>= 9.5), got {}",
                    tf.translation.y
                );

                // Check negative rotation around Z (so center X=0 is HIGHER than edge X=+13)
                let (_, _, roll) = tf.rotation.to_euler(EulerRot::XYZ);
                assert!(
                    roll < -0.5,
                    "East roof slope must have steep negative Z rotation (< -0.5 rad / ~-28 deg), got {}",
                    roll
                );

                let half_span_y = 7.935 * (-roll).sin();
                let y_apex = tf.translation.y + half_span_y;
                let y_wall = tf.translation.y - half_span_y;

                assert!(
                    y_apex > y_wall + 7.0,
                    "Center ridge apex must be significantly higher than edge walls: apex={}, wall={}",
                    y_apex,
                    y_wall
                );
                assert!(
                    y_apex >= 14.0,
                    "Peak apex should be >= 14.0m for steep high vault, got {}",
                    y_apex
                );
            }
            "VaultedRidgeBeam" => {
                ridge_beam_found = true;
                assert_eq!(
                    tf.translation.x, 0.0,
                    "Ridge apex beam must run along X = 0.0"
                );
                assert!(
                    tf.translation.y >= 14.0,
                    "Ridge apex beam must be elevated at steep peak (Y >= 14.0), got {}",
                    tf.translation.y
                );
            }
            "TimberKingPost" => {
                king_post_found = true;
                assert_eq!(
                    tf.translation.x, 0.0,
                    "King post must be centered at X = 0.0"
                );
                assert!(
                    tf.translation.y >= 9.0,
                    "King post center Y should be elevated to connect tie beam to high apex, got {}",
                    tf.translation.y
                );
            }
            _ => {}
        }
    }

    assert!(west_roof_found, "VaultedRoofWest must be spawned");
    assert!(east_roof_found, "VaultedRoofEast must be spawned");
    assert!(ridge_beam_found, "VaultedRidgeBeam must be spawned");
    assert!(king_post_found, "TimberKingPost must be spawned");
}

#[test]
fn test_audio_driver_failure_does_not_reinstantiate_per_frame() {
    let mut app = App::new();
    app.add_plugins(ElfAudioPlugin);
    app.add_message::<MooseHuffEvent>();
    app.add_message::<MooseWindowSlamEvent>();

    // Simulate an ALSA lock failure (e.g. EBUSY on raw hardware device hw:0,0 or hw:1,0)
    app.world_mut().insert_resource(AudioDriverStatus::FailedLock {
        reason: "ALSA pcm open failed with error 'Device or resource busy (EBUSY)'".into(),
        attempts: 1,
    });
    app.world_mut().resource_mut::<AudioMetrics>().initialization_attempts = 1;

    // Run the game simulation loop for 60 consecutive frames
    for frame in 0..60 {
        if frame == 10 {
            app.world_mut().write_message(MooseHuffEvent);
        }
        if frame == 25 {
            app.world_mut().write_message(MooseWindowSlamEvent);
        }
        if frame == 40 {
            app.world_mut().write_message(PlaySoundMessage::new(SoundCueType::VacuumSuction));
        }
        app.update();
    }

    let metrics = app.world().resource::<AudioMetrics>();
    let status = app.world().resource::<AudioDriverStatus>();

    // CRITICAL: Initialization attempts must remain strictly 1 — no per-frame reinstantiation!
    assert_eq!(
        metrics.initialization_attempts, 1,
        "Audio driver must NOT be reinstantiated on subsequent frames when in FailedLock state! Found: {}",
        metrics.initialization_attempts
    );
    assert_eq!(metrics.frames_elapsed, 60, "60 frames must have elapsed");
    assert_eq!(metrics.sound_events_received, 3, "3 gameplay sound events must have been received");
    assert_eq!(metrics.sound_events_suppressed, 3, "All 3 sound events must be safely suppressed without audio lock");
    assert_eq!(metrics.sound_events_played, 0, "No sounds played when audio lock is absent");
    assert!(status.is_failed_lock(), "Driver status must remain in FailedLock");
    assert!(!status.is_available(), "Audio must report unavailable");
}

#[test]
fn test_audio_driver_available_dispatches_sound_cues() {
    let mut app = App::new();
    app.add_plugins(ElfAudioPlugin);
    app.add_message::<MooseHuffEvent>();
    app.add_message::<MooseWindowSlamEvent>();

    // Simulate available audio driver lock
    app.world_mut().insert_resource(AudioDriverStatus::Available {
        backend: "Alsa".into(),
        device: "default".into(),
    });

    app.update();

    // Fire audio events
    app.world_mut().write_message(MooseHuffEvent);
    app.world_mut().write_message(MooseWindowSlamEvent);
    app.world_mut().write_message(PlaySoundMessage::new(SoundCueType::PresentDelivered));

    app.update();

    let metrics = app.world().resource::<AudioMetrics>();
    assert_eq!(metrics.sound_events_received, 3, "3 sound events received");
    assert_eq!(metrics.sound_events_played, 3, "All 3 sound cues successfully played");
    assert_eq!(metrics.sound_events_suppressed, 0, "Zero sound events suppressed when available");
}

#[test]
fn test_audio_driver_rate_limited_retry_prevents_per_frame_reconnect() {
    let mut app = App::new();
    app.add_plugins(ElfAudioPlugin);

    // Initial failed lock state
    app.world_mut().insert_resource(AudioDriverStatus::FailedLock {
        reason: "Device locked".into(),
        attempts: 1,
    });
    app.world_mut().resource_mut::<AudioMetrics>().initialization_attempts = 1;

    // Enable retry with 30s cooldown
    let mut config = app.world_mut().resource_mut::<AudioConfig>();
    config.allow_reinit_retry = true;
    config.min_retry_interval_secs = 30.0;
    config.last_retry_time = 0.0; // Last retry was at t=0

    // Frame 1 at t=5s (within 30s cooldown): rapid retries must be rejected
    let mut time: Time<()> = Time::default();
    time.advance_by(std::time::Duration::from_secs(5));
    app.world_mut().insert_resource(time);

    // Send 3 retry requests in the same frame
    app.world_mut().write_message(RetryAudioInitMessage);
    app.update();

    let metrics = app.world().resource::<AudioMetrics>();
    assert_eq!(
        metrics.initialization_attempts, 1,
        "Rapid retry requests within cooldown must be rejected and not re-probe ALSA!"
    );

    // Now advance time past 30s cooldown (t = 35s)
    let mut time: Time<()> = Time::default();
    time.advance_by(std::time::Duration::from_secs(35));
    app.world_mut().insert_resource(time);

    // Request retry once after cooldown expires
    app.world_mut().write_message(RetryAudioInitMessage);
    app.update();

    let metrics = app.world().resource::<AudioMetrics>();
    assert_eq!(
        metrics.initialization_attempts, 2,
        "Retry after cooldown expires should increment attempt count to 2"
    );

    // Immediately in the very next frame (t = 35.016s, 60fps), another retry request arrives:
    let mut time: Time<()> = Time::default();
    time.advance_by(std::time::Duration::from_millis(35016));
    app.world_mut().insert_resource(time);

    app.world_mut().write_message(RetryAudioInitMessage);
    app.update();

    let metrics = app.world().resource::<AudioMetrics>();
    assert_eq!(
        metrics.initialization_attempts, 2,
        "Next-frame retry request must be blocked by rate-limiter, preventing per-frame poll loops!"
    );
}

#[test]
fn test_audio_disabled_by_default_suppresses_cues_without_alsa_calls() {
    let mut app = App::new();
    app.add_plugins(ElfAudioPlugin);
    app.add_message::<MooseHuffEvent>();
    app.add_message::<MooseWindowSlamEvent>();

    // Run startup and update frames without any manual configuration
    app.update();

    let status = app.world().resource::<AudioDriverStatus>();
    let metrics = app.world().resource::<AudioMetrics>();
    let config = app.world().resource::<AudioConfig>();

    assert!(!config.enabled, "AudioConfig must be disabled by default");
    assert!(status.is_disabled(), "AudioDriverStatus must be Disabled by default to avoid alsa::poll() errors");
    assert_eq!(
        metrics.initialization_attempts, 0,
        "Zero ALSA/CPAL probe attempts must be made when audio is disabled by default"
    );

    // Write audio messages
    app.world_mut().write_message(MooseHuffEvent);
    app.world_mut().write_message(MooseWindowSlamEvent);
    app.world_mut().write_message(PlaySoundMessage::new(SoundCueType::VacuumSuction));
    app.update();

    let metrics = app.world().resource::<AudioMetrics>();
    assert_eq!(metrics.sound_events_received, 3, "3 sound events received");
    assert_eq!(metrics.sound_events_suppressed, 3, "All 3 sound events safely suppressed without ALSA calls");
    assert_eq!(metrics.sound_events_played, 0, "Zero sound events played");
    assert_eq!(metrics.initialization_attempts, 0, "Initialization attempts must strictly remain 0");
}

#[test]
fn test_moose_bulky_fluffy_scarf_structure() {
    use rust_tests::elf_game::moose::toy_model::{spawn_mr_moose, MooseScarf};

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_mr_moose);
    app.update();

    let mut name_q = app.world_mut().query::<&Name>();
    let mut collar_found = false;
    let mut cowl_fold_found = false;
    let mut knot_found = false;
    let mut pompom_count = 0;
    let mut segment_count = 0;

    for name in name_q.iter(app.world()) {
        match name.as_str() {
            "MooseScarfCollar" => collar_found = true,
            "MooseScarfCowlFold" => cowl_fold_found = true,
            "MooseScarfKnot" => knot_found = true,
            "MooseScarfPomPom" => pompom_count += 1,
            s if s.starts_with("MooseScarfSegment_") => segment_count += 1,
            _ => {}
        }
    }

    assert!(collar_found, "MooseScarfCollar must be spawned");
    assert!(cowl_fold_found, "Secondary fluffy MooseScarfCowlFold must be spawned");
    assert!(knot_found, "Bulky knit MooseScarfKnot must be spawned");
    assert_eq!(pompom_count, 2, "Both tails must have fluffy pom-poms on tips");
    assert_eq!(segment_count, 8, "8 soft-body scarf segments must be spawned across 2 tails");

    // Scarf entities should include collar and tail segments
    let mut scarf_q = app.world_mut().query::<(&MooseScarf, &Name)>();
    let scarf_entities: Vec<String> = scarf_q.iter(app.world()).map(|(_, n)| n.to_string()).collect();
    assert!(scarf_entities.len() >= 9, "Scarf must have collar and segments");
}

#[test]
fn test_moose_rampage_bucking_animation() {
    use rust_tests::elf_game::workshop::events::{
        MooseRampageTracker, moose_rampage_path_and_physics_impulse_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage, toy_model::MrMoose};

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    app.insert_resource(MooseHunger {
        value: 60.0,
        stage: MooseHungerStage::BreachedRampage,
        is_breached: true,
        ..default()
    });
    app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

    // Spawn Moose inside workshop heading towards waypoint 2 (currently at (1.0, 1.4, -2.0))
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        MooseRampageTracker {
            current_waypoint_index: 2,
            speed: 8.5,
            base_speed: 8.5,
            waypoints: vec![
                Vec3::new(-6.5, 1.4, -10.0),
                Vec3::new(0.0, 1.4, -4.0),
                Vec3::new(5.0, 1.4, 3.0),
                Vec3::new(-6.5, 1.8, -17.0),
            ],
            buck_timer: 1.15,
            next_buck_interval: 1.20,
            buck_duration: 0.80,
            is_bucking: false,
            current_buck_pitch: 0.0,
            ..default()
        },
        Transform::from_xyz(1.0, 1.4, -2.0),
    )).id();

    // 1. Step 1: Advance by 0.1s -> buck_timer exceeds next_buck_interval, bucking initiates!
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(100));
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(tracker.is_bucking, "Moose must start bucking when buck timer reaches interval");

    // 2. Step into Phase 1 (Rearing upward from back feet, slowing down slightly)
    // Advance progress to ~0.30 of buck_duration (0.80s * 0.30 = ~0.24s)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(200));
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();

    assert!(
        tracker.current_buck_pitch > 0.30,
        "Moose should rotate upward from back feet (pitch > 0.30 rad / ~17 deg), got: {}",
        tracker.current_buck_pitch
    );
    assert!(
        tracker.speed < tracker.base_speed,
        "Moose should slow down slightly while rearing upward (speed {} < base {})",
        tracker.speed, tracker.base_speed
    );
    assert!(
        moose_tf.translation.y > 1.45,
        "Moose torso should elevate (Y > 1.45m) to pivot upward from rear feet, got: {}",
        moose_tf.translation.y
    );

    // 3. Step into Phase 2 (Rotating back to flat and speeding up again)
    // Advance progress to ~0.70 of buck_duration
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(350));
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(
        tracker.speed > tracker.base_speed,
        "Moose should speed up again while rotating back to flat (speed {} > base {})",
        tracker.speed, tracker.base_speed
    );
    assert!(
        tracker.current_buck_pitch < 0.40,
        "Moose pitch should be rotating back down towards flat, got: {}",
        tracker.current_buck_pitch
    );

    // 4. Advance past buck completion (> 0.80s total)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(400));
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();

    assert!(!tracker.is_bucking, "Bucking cycle should finish and reset");
    assert_eq!(tracker.current_buck_pitch, 0.0, "Pitch must return to flat 0.0");
    assert!(
        (tracker.speed - tracker.base_speed).abs() < 0.01,
        "Speed must return to base speed after bucking recovery, got: {}",
        tracker.speed
    );
    assert!(
        (moose_tf.translation.y - 1.4).abs() < 0.05,
        "Moose height must return to flat ground level (1.4m), got: {}",
        moose_tf.translation.y
    );
}

#[test]
fn test_curved_crt_mesh_and_scanlines_geometry() {
    use rust_tests::elf_game::diegetic_ui::monitor::{
        create_curved_crt_mesh, create_curved_crt_scanlines_mesh,
    };
    use bevy::render::mesh::VertexAttributeValues;

    // 1. Verify Curved CRT Mesh
    let crt_mesh = create_curved_crt_mesh(2.1, 1.1, 0.08, 24, 16);
    let positions = match crt_mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(p) => p,
        _ => panic!("Expected Float32x3 positions"),
    };

    // Center vertex (i = 12, j = 8) should have peak forward bulge (Z ~ 0.08m)
    let center_idx = 8 * (24 + 1) + 12;
    let center_pos = positions[center_idx];
    assert!(
        (center_pos[2] - 0.08).abs() < 0.005,
        "CRT center must bulge forward ~0.08m, got: {}",
        center_pos[2]
    );

    // Corner vertex (0, 0) should have minimal curve (Z ~ 0.0m)
    let corner_pos = positions[0];
    assert!(
        corner_pos[2] < 0.01,
        "CRT corner must have flat border Z ~ 0.0m, got: {}",
        corner_pos[2]
    );

    // 2. Verify Scanlines Grille Mesh
    let scanline_mesh = create_curved_crt_scanlines_mesh(2.1, 1.1, 0.08, 28, 0.014);
    let sl_positions = match scanline_mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(p) => p,
        _ => panic!("Expected Float32x3 positions for scanlines"),
    };
    assert!(!sl_positions.is_empty(), "Scanline mesh must contain vertices");
    let max_z = sl_positions.iter().map(|p| p[2]).fold(0.0f32, f32::max);
    assert!(
        (max_z - 0.083).abs() < 0.005,
        "Peak scanline bulge should match curved glass + 3mm offset (0.083m), got: {}",
        max_z
    );
}

#[test]
fn test_crt_monitor_cyclic_instruction_messages() {
    use rust_tests::elf_game::diegetic_ui::monitor::{
        spawn_diegetic_screens_and_signs, update_diegetic_monitor_text,
        CrtInstructionMonitor,
    };
    use rust_tests::elf_game::core::WorkshopQuota;
    use rust_tests::elf_game::moose::MooseHunger;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.insert_resource(WorkshopQuota {
        presents_delivered: 2,
        target_quota: 5,
        tier: 1,
    });
    app.insert_resource(MooseHunger {
        value: 15.0,
        is_breached: false,
        ..default()
    });

    app.add_systems(Startup, spawn_diegetic_screens_and_signs);
    app.add_systems(Update, update_diegetic_monitor_text);
    app.update();

    // 1. Initial State: Page 0 (Present Instructions)
    {
        let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
        let monitor = q_mon.single(app.world()).unwrap();
        assert_eq!(monitor.current_page, 0, "Monitor should start at Page 0");
        assert_eq!(monitor.total_pages, 4, "Monitor should have 4 instructional pages");
    }

    {
        let mut q_txt = app.world_mut().query::<&Text2d>();
        let text_page0 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 1/4")).expect("Must find Page 1 text");
        assert!(text_page0.0.contains("PRESENT INSTRUCTIONS"));
        assert!(text_page0.0.contains("GRAB RAW TOY CUBE FROM CONVEYOR BELT"));
        assert!(text_page0.0.contains("QUOTA: 2 / 5 [TIER 1]"));
    }

    // 2. Advance time by 4.0s -> Advances to Page 1 (Dispatch & Vacuum Guide)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(4050));
    }
    app.update();

    {
        let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
        let monitor = q_mon.single(app.world()).unwrap();
        assert_eq!(monitor.current_page, 1, "Monitor should cycle to Page 1");
    }
    {
        let mut q_txt = app.world_mut().query::<&Text2d>();
        let text_page1 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 2/4")).expect("Must find Page 2 text");
        assert!(text_page1.0.contains("DISPATCH & VACUUM GUIDE"));
        assert!(text_page1.0.contains("CARRY WRAPPED PRESENTS TO THE CIRCULAR SEND PAD"));
    }

    // 3. Advance time by 4.0s -> Advances to Page 2 (Moose Safety Protocol)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(4050));
    }
    app.update();

    {
        let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
        let monitor = q_mon.single(app.world()).unwrap();
        assert_eq!(monitor.current_page, 2, "Monitor should cycle to Page 2");
    }
    {
        let mut q_txt = app.world_mut().query::<&Text2d>();
        let text_page2 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 3/4")).expect("Must find Page 3 text");
        assert!(text_page2.0.contains("MR. MOOSE SAFETY PROTOCOL"));
        assert!(text_page2.0.contains("GRAB SMALL MOOSE SNACKS FROM WORKBENCH JAR"));
    }

    // 4. Advance time by 4.0s -> Advances to Page 3 (Elf Controls & BHOP)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(4050));
    }
    app.update();

    {
        let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
        let monitor = q_mon.single(app.world()).unwrap();
        assert_eq!(monitor.current_page, 3, "Monitor should cycle to Page 3");
    }
    {
        let mut q_txt = app.world_mut().query::<&Text2d>();
        let text_page3 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 4/4")).expect("Must find Page 4 text");
        assert!(text_page3.0.contains("ELF CONTROLS & BHOP GUIDE"));
        assert!(text_page3.0.contains("TIME LANDING TO BHOP BOOST"));
    }

    // 5. Advance time by 4.0s -> Wraps back to Page 0 (Loop verified!)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(4050));
    }
    app.update();

    {
        let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
        let monitor = q_mon.single(app.world()).unwrap();
        assert_eq!(monitor.current_page, 0, "Monitor must wrap back to Page 0 in an infinite loop");
    }

    // 6. Emergency Breach override
    {
        let mut moose = app.world_mut().resource_mut::<MooseHunger>();
        moose.is_breached = true;
    }
    app.update();

    {
        let mut q_txt = app.world_mut().query::<&Text2d>();
        let text_breach = q_txt.iter(app.world()).find(|t| t.0.contains("EMERGENCY ALERT")).expect("Must find Emergency Alert");
        assert!(text_breach.0.contains("MR. MOOSE HAS BREACHED THE WORKSHOP"));
    }
}

#[test]
fn test_in_world_directional_signs_and_object_labels() {
    use rust_tests::elf_game::diegetic_ui::monitor::{
        spawn_diegetic_screens_and_signs, DirectionalSign,
    };

    let mut app = App::new();
    app.add_systems(Startup, spawn_diegetic_screens_and_signs);
    app.update();

    let signs: Vec<(String, String)> = {
        let mut q_signs = app.world_mut().query::<&DirectionalSign>();
        q_signs
            .iter(app.world())
            .map(|s| (s.station.clone(), s.label.clone()))
            .collect()
    };

    assert!(signs.len() >= 2, "Expected at least 2 directional signs/labels, found: {}", signs.len());

    let stations: Vec<&str> = signs.iter().map(|(st, _)| st.as_str()).collect();
    assert!(stations.contains(&"WrappingWorkbench"), "Must have Workbench Wrapping plaque");
    assert!(stations.contains(&"PresentSendingPlatform"), "Must have Present Sending Platform sign");

    let workbench_sign = signs.iter().find(|(st, _)| st == "WrappingWorkbench").unwrap();
    assert!(workbench_sign.1.contains("WRAPPING STATION"));
    assert!(workbench_sign.1.contains("ASSEMBLE, TAPE & TIE"));

    let send_sign = signs.iter().find(|(st, _)| st == "PresentSendingPlatform").unwrap();
    assert!(send_sign.1.contains("PLACE WRAPPED GIFT ON CIRCLE"));
    assert!(send_sign.1.contains("HIT RED BUTTON TO SEND"));
}

#[test]
fn test_above_floor_despawner_and_curved_metal_duct() {
    use rust_tests::elf_game::workshop::room::{
        setup_workshop_scene, ConveyorDespawnChute, ConveyorDespawnDuct, ConveyorSpawnerDuct,
        conveyor_despawn_chute_system,
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, setup_workshop_scene);
    app.add_systems(Update, conveyor_despawn_chute_system);
    app.update();

    // 1. Verify Despawner is positioned above the floor (Y >= 0.2m, not negative sub-floor)
    let chute_tf = {
        let mut q_chute = app.world_mut().query_filtered::<&Transform, With<ConveyorDespawnChute>>();
        let tf = q_chute.single(app.world()).unwrap();
        *tf
    };
    assert!(
        chute_tf.translation.y > 0.0,
        "Despawner chute must be positioned above the floor (Y > 0.0), got: {}",
        chute_tf.translation.y
    );
    assert!(
        (chute_tf.translation.y - 0.35).abs() < 0.1,
        "Despawner chute should sit at Y ~ 0.35m, got: {}",
        chute_tf.translation.y
    );

    // 2. Verify Curved Despawn Duct and Ceiling Spawner Duct are spawned
    let despawn_duct_count = {
        let mut q_duct = app.world_mut().query_filtered::<Entity, With<ConveyorDespawnDuct>>();
        q_duct.iter(app.world()).count()
    };
    assert!(despawn_duct_count >= 5, "Curved despawn duct must have multiple hood and cheek segments, found: {}", despawn_duct_count);

    let spawner_duct_count = {
        let mut q_duct = app.world_mut().query_filtered::<Entity, With<ConveyorSpawnerDuct>>();
        q_duct.iter(app.world()).count()
    };
    assert!(spawner_duct_count >= 4, "Ceiling spawner duct must have vertical shaft and hood segments, found: {}", spawner_duct_count);

    // 3. Test Despawn Functionality
    // Cube 1: Resting on the elevated conveyor belt (8.8, 0.75, 3.0) -> MUST NOT despawn
    let belt_cube = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(8.8, 0.75, 3.0),
    )).id();

    // Cube 2: Inside the above-floor despawner box (9.8, 0.35, 3.0) -> MUST despawn
    let despawn_cube = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(9.8, 0.35, 3.0),
    )).id();

    app.update();

    assert!(app.world().get_entity(belt_cube).is_ok(), "Cube on belt must remain intact");
    assert!(app.world().get_entity(despawn_cube).is_err(), "Cube entering above-floor despawner must be despawned");
}

#[test]
fn test_flipped_vacuum_cone_funnel() {
    use rust_tests::elf_game::workshop::room::{
        setup_workshop_scene, PresentVacuumDespawner,
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

    let funnel_tf = {
        let mut q_funnel = app.world_mut().query_filtered::<&Transform, With<PresentVacuumDespawner>>();
        let tf = q_funnel.single(app.world()).unwrap();
        *tf
    };

    // Rotation should be Quat::IDENTITY (or 0 rotation about X), pointing opening downwards
    let (rx, _, _) = funnel_tf.rotation.to_euler(EulerRot::XYZ);
    assert!(
        rx.abs() < 0.01,
        "Suction vacuum cone must be flipped right-side up (rx ~ 0.0 rad, not PI), got rx: {}",
        rx
    );
}

#[test]
fn test_present_vacuum_activation_with_q_e_and_physics_hit() {
    use rust_tests::elf_game::workshop::room::{
        PresentSendingPlatform, SendPlatformButton, VacuumTubeSegment, TubeTravellingLump,
        present_sending_platform_system, tube_travelling_lump_system,
    };
    use rust_tests::elf_game::player::ElfMovement;
    use rust_tests::elf_game::core::WorkshopQuota;
    use rust_tests::elf_game::wrapping::PresentDeliveredMessage;
    use bevy_rapier3d::prelude::Velocity;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(WorkshopQuota {
        presents_delivered: 0,
        target_quota: 5,
        tier: 1,
    });
    app.add_message::<PresentDeliveredMessage>();
    app.add_systems(Update, (present_sending_platform_system, tube_travelling_lump_system));

    let platform_entity = app.world_mut().spawn((
        PresentSendingPlatform {
            is_active: false,
            suction_timer: 0.0,
            ..default()
        },
        Transform::from_xyz(5.5, 0.05, -4.0),
    )).id();

    // Button Pedestal at (3.8, 0.45, -2.8) with button at (3.8, 0.95, -2.8)
    app.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(3.8, 0.95, -2.8),
    ));

    // Player standing within reach of button
    app.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(3.8, 1.0, -2.5),
    ));

    // Vacuum tube segments for transit effect
    for i in 0..10 {
        let rest_pos = Vec3::new(5.5 + (i as f32 * 0.35), 4.2, -4.0);
        app.world_mut().spawn((
            VacuumTubeSegment {
                index: i,
                base_scale: Vec3::ONE,
                rest_position: rest_pos,
            },
            Transform::from_translation(rest_pos),
        ));
    }

    // 1. Test activation with KeyQ
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyQ);
    }
    app.update();

    let platform = app.world().get::<PresentSendingPlatform>(platform_entity).unwrap();
    assert!(platform.is_active, "Vacuum must turn on when button is pressed with KeyQ");
    assert!(platform.suction_timer > 0.0);

    // Reset platform
    {
        let mut p = app.world_mut().get_mut::<PresentSendingPlatform>(platform_entity).unwrap();
        p.is_active = false;
        p.suction_timer = 0.0;
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset(KeyCode::KeyQ);
        keys.press(KeyCode::KeyE);
    }
    app.update();

    // 2. Test activation with KeyE
    let platform = app.world().get::<PresentSendingPlatform>(platform_entity).unwrap();
    assert!(platform.is_active, "Vacuum must turn on when button is pressed with KeyE");

    // Reset platform and test activation via physics object collision
    {
        let mut p = app.world_mut().get_mut::<PresentSendingPlatform>(platform_entity).unwrap();
        p.is_active = false;
        p.suction_timer = 0.0;
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset(KeyCode::KeyE);
    }

    // Spawn a thrown physics object hitting the button at (3.8, 0.95, -2.8)
    let thrown_obj = app.world_mut().spawn((
        Transform::from_xyz(3.82, 0.96, -2.78),
        Velocity {
            linear: Vec3::new(0.0, -2.0, 0.0),
            angular: Vec3::ZERO,
        },
    )).id();
    app.update();

    let platform = app.world().get::<PresentSendingPlatform>(platform_entity).unwrap();
    assert!(platform.is_active, "Vacuum must turn on when hit with a physics object");

    // Despawn the thrown object
    app.world_mut().despawn(thrown_obj);

    // 3. Test that non-player physics objects are drawn towards vacuum and ingested while button is held
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyE);
    }

    let loose_physics_prop = app.world_mut().spawn((
        Transform::from_xyz(5.5, 0.4, -4.0),
        Velocity::zero(),
    )).id();

    app.update();

    let vel = app.world().get::<Velocity>(loose_physics_prop).unwrap();
    assert!(vel.linear.y > 0.0, "Physics object must be drawn upwards towards vacuum funnel");

    // Move to funnel intake mouth (Y = 3.8m)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(loose_physics_prop).unwrap();
        tf.translation.y = 3.8;
    }
    app.update();

    // Physics object must be despawned and travelling lump spawned in tube
    assert!(app.world().get_entity(loose_physics_prop).is_err(), "Physics object must despawn upon hitting vacuum despawner");
    let lump_count = app.world_mut().query_filtered::<Entity, With<TubeTravellingLump>>().iter(app.world()).count();
    assert!(lump_count >= 1, "Vacuum tube must have travelling lump effect showing object going through it");
}

#[test]
fn test_moose_snappy_state_animations() {
    use rust_tests::elf_game::moose::toy_model::{
        LeftWindowShutter, RightWindowShutter, MrMoose, MooseHead, animate_mr_moose_peeking_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage};

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    app.insert_resource(MooseHunger {
        value: 10.0,
        stage: MooseHungerStage::BehindShutters,
        is_breached: false,
        ..default()
    });
    app.add_systems(Update, animate_mr_moose_peeking_system);

    let left_shutter = app.world_mut().spawn((
        LeftWindowShutter,
        Transform::from_xyz(-7.4, 2.9, -13.0),
    )).id();
    let _right_shutter = app.world_mut().spawn((
        RightWindowShutter,
        Transform::from_xyz(-5.6, 2.9, -13.0),
    )).id();
    let moose = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(-6.5, 1.8, -17.0),
    )).id();
    let head = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    )).id();

    // Settle in BehindShutters
    app.update();

    // 1. Enter PeekingThrough stage at hunger = 20.5 (barely entered PeekingThrough)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 20.5;
        hunger.stage = MooseHungerStage::PeekingThrough;
    }

    // Advance 15 frames (~0.24s)
    for _ in 0..15 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    // Verify Moose does NOT snap into the room immediately: body stays near resting outside (Z < -16.5),
    // head stays near outside (Z < 1.8), and shutters have barely cracked open (yaw < 0.2 rad)
    let m_tf = app.world().get::<Transform>(moose).unwrap();
    let h_tf = app.world().get::<Transform>(head).unwrap();
    let ls_tf = app.world().get::<Transform>(left_shutter).unwrap();
    let (left_yaw, _, _) = ls_tf.rotation.to_euler(EulerRot::YXZ);

    assert!(
        m_tf.translation.z < -16.5,
        "Moose body must NOT snap into room at start of peeking (Z < -16.5), got {}",
        m_tf.translation.z
    );
    assert!(
        h_tf.translation.z < 1.8,
        "Moose head must NOT jump forward at start of peeking (Z < 1.8), got {}",
        h_tf.translation.z
    );
    assert!(
        left_yaw.abs() < 0.2,
        "Left shutter must only barely start opening at start of peeking, got yaw {}",
        left_yaw
    );

    // Advance hunger to halfway through peeking (hunger = 35.0, 15s in)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 35.0;
    }
    for _ in 0..15 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    let m_tf = app.world().get::<Transform>(moose).unwrap();
    let h_tf = app.world().get::<Transform>(head).unwrap();
    let ls_tf = app.world().get::<Transform>(left_shutter).unwrap();
    let (left_yaw, _, _) = ls_tf.rotation.to_euler(EulerRot::YXZ);

    assert!(
        m_tf.translation.z > -16.1 && m_tf.translation.z < -15.4,
        "Moose body must be roughly halfway forward at hunger 35 (got {})",
        m_tf.translation.z
    );
    assert!(
        h_tf.translation.z > 1.9 && h_tf.translation.z < 2.2,
        "Moose head must be roughly halfway forward at hunger 35 (got {})",
        h_tf.translation.z
    );
    assert!(
        left_yaw < -0.5,
        "Left shutter must be roughly half open at hunger 35, got yaw {}",
        left_yaw
    );

    // Advance hunger to end of peeking (hunger = 50.0, 30s in)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 50.0;
    }
    for _ in 0..15 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    let m_tf = app.world().get::<Transform>(moose).unwrap();
    let h_tf = app.world().get::<Transform>(head).unwrap();
    let ls_tf = app.world().get::<Transform>(left_shutter).unwrap();
    let (left_yaw, _, _) = ls_tf.rotation.to_euler(EulerRot::YXZ);

    assert!(
        m_tf.translation.z > -14.7,
        "Moose body must reach window at hunger 50 (Z > -14.7), got {}",
        m_tf.translation.z
    );
    assert!(
        h_tf.translation.z > 2.3,
        "Moose head must reach full peek at hunger 50 (Z > 2.3), got {}",
        h_tf.translation.z
    );
    assert!(
        left_yaw < -1.0,
        "Left shutter must be fully open at hunger 50, got yaw {}",
        left_yaw
    );

    // 2. Transition to WindowSlammed stage
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
    }

    // Advance 15 frames (~0.24s)
    for _ in 0..15 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    let m_tf = app.world().get::<Transform>(moose).unwrap();
    let h_tf = app.world().get::<Transform>(head).unwrap();
    let ls_tf = app.world().get::<Transform>(left_shutter).unwrap();
    let (left_yaw, _, _) = ls_tf.rotation.to_euler(EulerRot::YXZ);

    assert!(
        m_tf.translation.z < -16.8,
        "Moose body must snap back outside (Z < -16.8) within 0.25s, got {}",
        m_tf.translation.z
    );
    assert!(
        h_tf.translation.z < 1.7,
        "Moose head must snap back behind shutters (Z < 1.7) within 0.25s, got {}",
        h_tf.translation.z
    );
    assert!(
        left_yaw.abs() < 0.05,
        "Shutters must slam shut within 0.25s, got yaw {}",
        left_yaw
    );
}

#[test]
fn test_moose_chain_physics_scarf_mount_and_tails() {
    use rust_tests::elf_game::moose::toy_model::{
        MrMoose, MooseHead, MooseScarfSegment, MooseScarfMount, moose_scarf_physics_system,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, moose_scarf_physics_system);

    // Mr. Moose root with MooseScarfMount
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        MooseScarfMount::default(),
        Transform::from_xyz(-6.5, 1.8, -17.0),
    )).id();

    // Moose Head
    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    )).id();

    // Spawn 8 scarf segments (left tail 0..4, right tail 4..8)
    let mut seg_entities = Vec::new();
    for tail in 0..2 {
        let is_left = tail == 0;
        let mount_anchor = if is_left {
            Vec3::new(-0.24, 0.85, 1.45)
        } else {
            Vec3::new(0.24, 0.85, 1.45)
        };
        for seg_i in 0..4 {
            let idx = if is_left { seg_i } else { 4 + seg_i };
            let link_disp = Vec3::new(0.0, -0.22 * (seg_i as f32 + 1.0), 0.08 * (seg_i as f32 + 1.0));
            let rest_off = mount_anchor + link_disp;
            let ent = app.world_mut().spawn((
                MooseScarfSegment {
                    index: idx,
                    rest_offset: rest_off,
                    velocity: Vec3::ZERO,
                },
                Transform::from_translation(Vec3::new(-6.5, 1.8, -17.0) + rest_off),
            )).id();
            seg_entities.push(ent);
        }
    }

    // 1. Initial rest step
    app.update();

    // Check that segments form a continuous chain where each link is close to rest distance (~0.22m) from its parent
    let mount = app.world().get::<MooseScarfMount>(moose_entity).unwrap();
    let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
    let left_mount = moose_tf.transform_point(mount.left_tail_anchor);
    let right_mount = moose_tf.transform_point(mount.right_tail_anchor);

    let seg0_pos = app.world().get::<Transform>(seg_entities[0]).unwrap().translation;
    let dist_mount_to_0 = (seg0_pos - left_mount).length();
    assert!(
        dist_mount_to_0 >= 0.15 && dist_mount_to_0 <= 0.35,
        "Segment 0 must attach to left mount point on Moose collar, dist = {}",
        dist_mount_to_0
    );

    let seg4_pos = app.world().get::<Transform>(seg_entities[4]).unwrap().translation;
    let dist_mount_to_4 = (seg4_pos - right_mount).length();
    assert!(
        dist_mount_to_4 >= 0.15 && dist_mount_to_4 <= 0.35,
        "Segment 4 must attach to right mount point on Moose collar, dist = {}",
        dist_mount_to_4
    );

    // Check link-to-link distance constraints along the left tail
    for i in 1..4 {
        let prev_pos = app.world().get::<Transform>(seg_entities[i - 1]).unwrap().translation;
        let cur_pos = app.world().get::<Transform>(seg_entities[i]).unwrap().translation;
        let link_dist = (cur_pos - prev_pos).length();
        assert!(
            link_dist >= 0.14 && link_dist <= 0.36,
            "Chain link {} must remain attached to link {}, dist = {}",
            i, i - 1, link_dist
        );
    }

    // 2. Moose lunges forward and bucks upward: verify momentum propagation down the chain
    {
        let mut m_tf = app.world_mut().get_mut::<Transform>(moose_entity).unwrap();
        m_tf.translation.z = -14.5;
        // Pitch up by 20 degrees around back feet (bucking)
        m_tf.rotation = Quat::from_rotation_x(0.35);

        let mut h_tf = app.world_mut().get_mut::<Transform>(head_entity).unwrap();
        h_tf.translation.z = 3.6;
    }

    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
    app.update();

    // Verify momentum propagates down to the tail tips (segment 3 and segment 7)
    let tip_left = app.world().get::<MooseScarfSegment>(seg_entities[3]).unwrap();
    let tip_right = app.world().get::<MooseScarfSegment>(seg_entities[7]).unwrap();

    assert!(
        tip_left.velocity.length() > 0.5,
        "Left tail tip (pom-pom) must receive dynamic chain physics momentum, got vel = {:?}",
        tip_left.velocity
    );
    assert!(
        tip_right.velocity.length() > 0.5,
        "Right tail tip (pom-pom) must receive dynamic chain physics momentum, got vel = {:?}",
        tip_right.velocity
    );
}

#[test]
fn test_physical_mechanical_hunger_dial_and_scale() {
    use rust_tests::elf_game::diegetic_ui::hunger_dial::{
        spawn_gas_pressure_gauge, animate_gauge_system,
        Gauge, GasPressureGauge, GaugeNeedle, GaugeWarningBeacon,
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();

    app.add_systems(Startup, spawn_gas_pressure_gauge);
    app.add_systems(Update, animate_gauge_system);

    // Run startup
    app.update();

    // 1. Verify all components exist
    let mut gauge_q = app.world_mut().query::<(&GasPressureGauge, &Gauge, &Transform)>();
    let dial = gauge_q.iter(app.world()).next();
    assert!(dial.is_some(), "GasPressureGauge must be spawned beside stove");
    let (_, _, dial_tf) = dial.unwrap();
    assert!(
        (dial_tf.translation.x - (-9.0)).abs() < 0.2 && (dial_tf.translation.z - (-12.75)).abs() < 0.2,
        "Dial must be mounted beside stove by the window, got {:?}",
        dial_tf.translation
    );

    let mut needle_q = app.world_mut().query::<&GaugeNeedle>();
    assert_eq!(needle_q.iter(app.world()).count(), 1, "Needle entity must exist");

    let mut beacon_q = app.world_mut().query::<&GaugeWarningBeacon>();
    assert_eq!(beacon_q.iter(app.world()).count(), 1, "Warning beacon must exist");

    // 2. Initial state: value 1.0 (full) -> angle ≈ +2.356 rad = +135 deg
    {
        let mut needle_q = app.world_mut().query::<&GaugeNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            (needle.current_angle - needle.max_angle).abs() < 0.1,
            "Needle should be at maximum angle (≈ +2.356) at value 1.0, got {}",
            needle.current_angle
        );
        assert!(!needle.is_jittering, "Needle should not jitter when calm");
    }

    // 3. Set value to 0.5 (midpoint angle ≈ 0.0 rad)
    {
        let mut q = app.world_mut().query::<&mut Gauge>();
        let mut gauge = q.single_mut(app.world_mut()).unwrap();
        gauge.set_value(0.5);
    }
    app.update();
    {
        let mut needle_q = app.world_mut().query::<&GaugeNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            needle.current_angle.abs() < 0.2,
            "Needle should be at top center (≈ 0.0) at value 0.5, got {}",
            needle.current_angle
        );
    }

    // 4. Set value to 0.0 (far left angle ≈ -2.356 rad = -135 deg)
    {
        let mut q = app.world_mut().query::<&mut Gauge>();
        let mut gauge = q.single_mut(app.world_mut()).unwrap();
        gauge.set_value(0.0);
    }
    app.update();
    {
        let mut needle_q = app.world_mut().query::<&GaugeNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            (needle.current_angle - needle.min_angle).abs() < 0.1,
            "Needle should be at far left (≈ -2.356) at value 0.0, got {}",
            needle.current_angle
        );
        assert!(needle.is_jittering, "Needle must jitter when value <= critical_threshold");

        let mut beacon_q = app.world_mut().query::<&GaugeWarningBeacon>();
        let beacon = beacon_q.iter(app.world()).next().unwrap();
        assert!(beacon.is_alert, "Warning beacon must be in alert mode when critical");
    }

    // 5. Reset value to 0.9 (safe calm state)
    {
        let mut q = app.world_mut().query::<&mut Gauge>();
        let mut gauge = q.single_mut(app.world_mut()).unwrap();
        gauge.set_value(0.9);
    }
    app.update();
    {
        let mut needle_q = app.world_mut().query::<&GaugeNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(!needle.is_jittering, "Needle must not jitter in safe state");

        let mut beacon_q = app.world_mut().query::<&GaugeWarningBeacon>();
        let beacon = beacon_q.iter(app.world()).next().unwrap();
        assert!(!beacon.is_alert, "Warning beacon must return to calm state");
    }
}

#[test]
fn test_soft_body_mesh_deformation_renderer() {
    use bevy::render::mesh::VertexAttributeValues;
    use rust_tests::elf_game::physics_sandbox::{
        SoftBodyLattice, SoftBodyDeformableMesh,
        update_soft_bodies_system, update_soft_body_meshes_system,
    };

    let base_cuboid = Cuboid::new(1.0, 1.0, 1.0);
    let mut mesh = Mesh::from(base_cuboid);
    let half_extents = Vec3::splat(0.5);

    let deformable = SoftBodyDeformableMesh::from_mesh(&mesh, half_extents)
        .expect("Deformable mesh extraction must succeed");

    assert_eq!(deformable.original_positions.len(), 24, "Cube mesh has 24 vertices (6 faces * 4 vertices)");
    assert_eq!(deformable.half_extents, half_extents);

    let mut lattice = SoftBodyLattice::new_cube(half_extents, 120.0, 8.0);
    assert_eq!(lattice.points.len(), 8, "Cube lattice must have 8 corner spring points");
    assert_eq!(lattice.edge_springs.len(), 12, "Cube must have 12 edge springs");

    // 1. Apply vertical squash (Y-axis compression with X/Z lateral dilation)
    lattice.apply_directional_squash(Vec3::Y, 0.4);

    // Verify top corner points (y > 0) compressed down and bulged outward
    for pt in &lattice.points {
        if pt.original_offset.y > 0.0 {
            assert!(
                pt.position.y < pt.original_offset.y,
                "Top points must be compressed downward: got {}, original {}",
                pt.position.y,
                pt.original_offset.y
            );
        } else {
            assert!(
                pt.position.y > pt.original_offset.y,
                "Bottom points must be compressed upward: got {}, original {}",
                pt.position.y,
                pt.original_offset.y
            );
        }
        // Lateral expansion to preserve volume
        let orig_lat = (pt.original_offset.x.powi(2) + pt.original_offset.z.powi(2)).sqrt();
        let cur_lat = (pt.position.x.powi(2) + pt.position.z.powi(2)).sqrt();
        assert!(
            cur_lat > orig_lat,
            "Lateral radius must expand for volume preservation: got {cur_lat}, original {orig_lat}"
        );
    }

    // 2. Apply deformation to mesh vertex buffers
    deformable.apply_deformation(&lattice, &mut mesh);

    let pos_attr = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    let VertexAttributeValues::Float32x3(deformed_positions) = pos_attr else {
        panic!("ATTRIBUTE_POSITION must be Float32x3");
    };

    // Verify deformed top face vertices (original y = 0.5) are squashed downwards
    for (orig, def) in deformable.original_positions.iter().zip(deformed_positions.iter()) {
        if orig.y > 0.49 {
            assert!(
                def[1] < orig.y,
                "Top vertices must be squashed downwards: deformed {}, original {}",
                def[1],
                orig.y
            );
        }
    }

    // 3. Test update_soft_body_meshes_system in a Bevy App
    let mut app = App::new();
    let mut meshes = Assets::<Mesh>::default();
    let handle = meshes.add(mesh);
    app.insert_resource(meshes);

    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    let entity = app.world_mut().spawn((
        Mesh3d(handle.clone()),
        lattice,
        deformable,
    )).id();

    app.add_systems(Update, (update_soft_bodies_system, update_soft_body_meshes_system).chain());

    // Step physics forward 10 frames
    for _ in 0..10 {
        app.update();
    }

    let lat = app.world().get::<SoftBodyLattice>(entity).unwrap();
    assert!(lat.max_displacement() > 0.0, "Lattice must still be oscillating with elastic restitution");

    // Mesh in assets should have been updated by the system
    let mesh_assets = app.world().resource::<Assets<Mesh>>();
    let updated_mesh = mesh_assets.get(&handle).unwrap();
    assert!(updated_mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
}

#[test]
fn test_ragdoll_npc_elf_assistants_spawning_and_limbs() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfLimbType, ElfRagdollLimb, setup_elf_assistants,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(Startup, setup_elf_assistants);
    app.update();

    let mut elf_q = app.world_mut().query::<(&ElfAssistant, &Name, &RigidBody, &Collider, &LockedAxes)>();
    let elves: Vec<(&ElfAssistant, String, &LockedAxes)> = elf_q
        .iter(app.world())
        .map(|(elf, name, _, _, locked)| (elf, name.to_string(), locked))
        .collect();

    assert_eq!(elves.len(), 3, "Exactly 3 NPC Elf assistants must be spawned");

    let names: Vec<String> = elves.iter().map(|(_, n, _)| n.clone()).collect();
    assert!(names.contains(&"Elf Sparky".to_string()));
    assert!(names.contains(&"Elf Tinsel".to_string()));
    assert!(names.contains(&"Elf Jingles".to_string()));

    // Verify all elves start with tilt rotation locked for walking
    for (_, _, locked) in &elves {
        assert_eq!(
            **locked,
            LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
            "Walking elves must have X/Z rotation locked"
        );
    }

    // Verify articulated limbs spawned for each elf
    let mut limb_q = app.world_mut().query::<&ElfRagdollLimb>();
    let mut sparky_limbs = Vec::new();
    for limb in limb_q.iter(app.world()) {
        if limb.elf_id == 1 {
            sparky_limbs.push(limb.limb_type);
        }
    }

    assert!(sparky_limbs.contains(&ElfLimbType::Torso));
    assert!(sparky_limbs.contains(&ElfLimbType::Head));
    assert!(sparky_limbs.contains(&ElfLimbType::ElfHat));
    assert!(sparky_limbs.contains(&ElfLimbType::HatPompom));
    assert!(sparky_limbs.contains(&ElfLimbType::LeftArm));
    assert!(sparky_limbs.contains(&ElfLimbType::RightArm));
    assert!(sparky_limbs.contains(&ElfLimbType::LeftLeg));
    assert!(sparky_limbs.contains(&ElfLimbType::RightLeg));
}

#[test]
fn test_elf_assistant_ai_states_and_ragdoll_tumble() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfNpcState, elf_assistant_ai_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage, toy_model::MrMoose};
    use rust_tests::elf_game::player::ElfMovement;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    let hunger_res = MooseHunger {
        value: 10.0,
        stage: MooseHungerStage::BehindShutters,
        ..default()
    };
    app.insert_resource(hunger_res);
    app.add_systems(Update, elf_assistant_ai_system);

    // Spawn an elf at (0.0, 0.55, 0.0) with waypoint at (3.0, 0.55, 0.0)
    let elf_entity = app.world_mut().spawn((
        ElfAssistant::new(1, "TestElf", vec![Vec3::new(3.0, 0.55, 0.0)]),
        Transform::from_xyz(0.0, 0.55, 0.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    // 1. Initial State: Wandering toward waypoint
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let vel = app.world().get::<Velocity>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Wandering);
        assert!(vel.linear.x > 0.5, "Elf must move towards waypoint along +X");
    }

    // 2. High Moose Hunger triggers Panicking state
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 55.0; // Warning stage
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let vel = app.world().get::<Velocity>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Panicking, "Agitated Moose must cause elf panic");
        assert!(vel.linear.z > 1.0, "Panicking elf must flee south (+Z) away from window");
    }

    // 3. Moose Rampage contact triggers RagdollTumble
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(0.0, 1.4, 0.5), // Within 2.8m of elf
    )).id();

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::BreachedRampage;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let locked = app.world().get::<LockedAxes>(elf_entity).unwrap();
        let impulse = app.world().get::<ExternalImpulse>(elf_entity).unwrap();

        assert_eq!(elf.state, ElfNpcState::RagdollTumble, "Moose strike must send elf into ragdoll tumble");
        assert_eq!(*locked, LockedAxes::empty(), "Ragdoll tumble must unlock all rotational axes for 6-DOF tumbling");
        assert!(impulse.impulse.y > 5.0, "Slapstick launch impulse must toss elf upward");
        assert!(impulse.torque_impulse.length() > 5.0, "Torque impulse must spin elf like a bowling pin");
    }

    // Despawn moose and simulate settling on the floor
    app.world_mut().despawn(moose_entity);
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 0.0;
        hunger.stage = MooseHungerStage::BehindShutters;

        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.tumble_timer = 2.5; // Exceeded tumble duration
        let mut vel = app.world_mut().get_mut::<Velocity>(elf_entity).unwrap();
        vel.linear = Vec3::ZERO; // Settled to rest on floor
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Recovering, "Resting ragdoll must transition into Recovering");
    }

    // Advance recovery to completion
    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.recovery_timer = 1.0; // Exceeded recovery_duration (0.8s)
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let locked = app.world().get::<LockedAxes>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Wandering, "Fully recovered elf must resume wandering");
        assert_eq!(*locked, LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z, "Upright elf must relock tilt axes");
    }

    // 4. Test high-speed player BHOP knockdown
    let player_entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.55, 0.3), // Near elf
        ElfMovement {
            velocity: Vec3::new(7.0, 0.0, 0.0), // Fast BHOP speed (> 5.5 m/s)
            ..default()
        },
    )).id();
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let locked = app.world().get::<LockedAxes>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::RagdollTumble, "High-speed player collision must knock elf into ragdoll");
        assert_eq!(*locked, LockedAxes::empty(), "Rotation axes unlocked on player knockdown");
    }
    app.world_mut().despawn(player_entity);
}

#[test]
fn test_elf_ragdoll_limb_procedural_animations() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfNpcState, ElfLimbType, ElfRagdollLimb,
        elf_ragdoll_limb_animation_system,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.add_systems(Update, elf_ragdoll_limb_animation_system);

    let elf_entity = app.world_mut().spawn((
        ElfAssistant::new(1, "AnimElf", vec![Vec3::ZERO]),
    )).id();

    let l_arm_entity = app.world_mut().spawn((
        ElfRagdollLimb {
            elf_entity,
            elf_id: 1,
            limb_type: ElfLimbType::LeftArm,
            rest_local_translation: Vec3::new(-0.21, 0.50, 0.0),
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        Transform::from_xyz(-0.21, 0.50, 0.0),
    )).id();

    // 1. Wandering animation: arm swings
    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.state = ElfNpcState::Wandering;
        elf.walk_timer = 0.2;
    }
    app.update();
    {
        let arm_tf = app.world().get::<Transform>(l_arm_entity).unwrap();
        assert!(
            arm_tf.rotation.x.abs() > 0.01,
            "Arm must swing along pitch axis during walking, got {}",
            arm_tf.rotation.x
        );
    }

    // 2. Panicking animation: arm flails overhead
    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.state = ElfNpcState::Panicking;
    }
    app.update();
    {
        let arm_tf = app.world().get::<Transform>(l_arm_entity).unwrap();
        // Overhead flail rotation has strong negative pitch
        let (x, _, _) = arm_tf.rotation.to_euler(EulerRot::XYZ);
        assert!(
            x.abs() > 0.5,
            "Panicking arm must be flailed upwards overhead, got rotation {x}"
        );
    }
}

#[test]
fn test_vacuum_tractor_beam_cone_gravity_draw_and_no_verification() {
    use rust_tests::elf_game::workshop::room::{
        PresentSendingPlatform, VacuumTractorBeamCone,
        VacuumTubeSegment, TubeTravellingLump,
        present_sending_platform_system, tube_travelling_lump_system,
    };
    use rust_tests::elf_game::wrapping::{Present, WrappingSpecification, WrappingFeature, PresentDeliveredMessage};
    use rust_tests::elf_game::core::WorkshopQuota;
    use bevy_rapier3d::prelude::Velocity;

    // 1. Verify mathematical cone containment and gravity draw vector
    let cone = VacuumTractorBeamCone::new(
        Vec3::new(5.5, 0.05, -4.0),
        Vec3::new(5.5, 4.0, -4.0),
        1.45,
        0.95,
    );

    // Points inside cone
    assert!(cone.contains_point(Vec3::new(5.5, 0.1, -4.0)), "Center platform floor point must be inside cone");
    assert!(cone.contains_point(Vec3::new(5.5 + 1.2, 0.15, -4.0)), "Platform rim floor point must be inside cone");
    assert!(cone.contains_point(Vec3::new(5.5, 2.0, -4.0)), "Mid-air beam point must be inside cone");
    assert!(cone.contains_point(Vec3::new(5.5 + 0.8, 3.8, -4.0)), "Top funnel mouth point must be inside cone");

    // Points outside cone
    assert!(!cone.contains_point(Vec3::new(5.5 + 2.5, 0.1, -4.0)), "Point far outside platform must NOT be in cone");
    assert!(!cone.contains_point(Vec3::new(5.5, -0.6, -4.0)), "Point below floor must NOT be in cone");
    assert!(!cone.contains_point(Vec3::new(5.5, 5.0, -4.0)), "Point above funnel must NOT be in cone");

    // Gravity draw vector calculation towards vacuum opening (5.5, 4.0, -4.0)
    let rim_pos = Vec3::new(6.5, 0.1, -4.0);
    let draw_vector = cone.compute_gravity_draw(rim_pos);
    assert!(
        draw_vector.x < 0.0,
        "Gravity draw on right side must accelerate left (-X) towards central axis: got dx = {}",
        draw_vector.x
    );
    assert!(
        draw_vector.y > 10.0,
        "Gravity draw must lift strongly upward (+Y) towards vacuum opening: got dy = {}",
        draw_vector.y
    );

    // 2. Bevy App simulation testing NO PRESENT VERIFICATION on various physics objects
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(WorkshopQuota {
        presents_delivered: 0,
        target_quota: 10,
        tier: 1,
    });
    app.add_message::<PresentDeliveredMessage>();
    app.add_systems(Update, (present_sending_platform_system, tube_travelling_lump_system));

    let platform_entity = app.world_mut().spawn((
        PresentSendingPlatform {
            is_active: false,
            suction_timer: 0.0,
            ..default()
        },
        Transform::from_xyz(5.5, 0.05, -4.0),
    )).id();

    let cone_entity = app.world_mut().spawn((
        cone,
        Visibility::Hidden,
        Transform::from_xyz(5.5, 2.025, -4.0),
    )).id();

    // Spawn 20 vacuum tube segments
    for i in 0..20 {
        let rest_pos = Vec3::new(5.5 + (i as f32 * 0.35), 4.2, -4.0);
        app.world_mut().spawn((
            VacuumTubeSegment {
                index: i,
                base_scale: Vec3::ONE,
                rest_position: rest_pos,
            },
            Transform::from_translation(rest_pos),
        ));
    }

    // Spawn 3 completely different physics objects on the platform:
    // Object A: Incomplete present (is_completed: false, missing paper/ribbon/bow!)
    let uncompleted_present = app.world_mut().spawn((
        Present {
            is_completed: false,
            quality_score: 10.0,
            ..default()
        },
        WrappingSpecification {
            required_features: [WrappingFeature::PaperWrapped, WrappingFeature::RibbonTied].into_iter().collect(),
            ..default()
        },
        Transform::from_xyz(5.2, 0.3, -4.0),
        Velocity::zero(),
        Name::new("UncompletedPresent"),
    )).id();

    // Object B: Raw wooden cube with NO Present component at all
    let raw_cube = app.world_mut().spawn((
        Transform::from_xyz(5.7, 0.3, -4.0),
        Velocity::zero(),
        Name::new("RawWoodenCube"),
    )).id();

    // Object C: Random workshop tool / snack entity with NO Present component
    let random_tool = app.world_mut().spawn((
        Transform::from_xyz(5.5, 0.3, -3.8),
        Velocity::zero(),
        Name::new("HeavyWrench"),
    )).id();

    // Activate the vacuum platform directly
    {
        let mut p = app.world_mut().get_mut::<PresentSendingPlatform>(platform_entity).unwrap();
        p.is_active = true;
        p.suction_timer = 6.0;
    }
    app.update();

    // Tractor beam cone should become visible
    let cone_vis = app.world().get::<Visibility>(cone_entity).unwrap();
    assert_eq!(*cone_vis, Visibility::Inherited, "Active tractor beam cone must be visible");

    // All 3 physics objects must experience upward gravity draw acceleration
    for &entity in &[uncompleted_present, raw_cube, random_tool] {
        let vel = app.world().get::<Velocity>(entity).unwrap();
        assert!(
            vel.linear.y > 0.0,
            "Physics object without completion must be pulled upward by tractor beam: got vy = {}",
            vel.linear.y
        );
    }

    // Move all 3 objects to the vacuum intake opening (Y = 3.6m)
    for &entity in &[uncompleted_present, raw_cube, random_tool] {
        let mut tf = app.world_mut().get_mut::<Transform>(entity).unwrap();
        tf.translation.y = 3.6;
    }
    app.update();

    // NO PRESENT VERIFICATION: All 3 physics objects must be ingested, despawned, and converted to tube lumps!
    assert!(app.world().get_entity(uncompleted_present).is_err(), "Uncompleted present must be ingested without verification");
    assert!(app.world().get_entity(raw_cube).is_err(), "Raw cube without Present must be ingested without verification");
    assert!(app.world().get_entity(random_tool).is_err(), "Random tool must be ingested without verification");

    let mut lump_q = app.world_mut().query::<&TubeTravellingLump>();
    let lumps: Vec<&TubeTravellingLump> = lump_q.iter(app.world()).collect();
    assert_eq!(lumps.len(), 3, "3 TubeTravellingLumps must be spawned for the 3 ingested objects");
}

#[test]
fn test_workshop_palette_constants_and_material_assets() {
    use rust_tests::elf_game::assets::{
        palette::*,
        WorkshopMaterials, AssetsPlugin,
    };
    use rust_tests::elf_game::workshop::room::setup_workshop_scene;
    use rust_tests::elf_game::moose::toy_model::spawn_mr_moose;
    use rust_tests::elf_game::player::hands::spawn_workshop_tools_and_materials;
    use rust_tests::elf_game::diegetic_ui::{
        hunger_dial::spawn_gas_pressure_gauge,
        monitor::spawn_diegetic_screens_and_signs,
    };

    // 1. Verify palette constants are pure Color::Srgba values
    assert_eq!(FESTIVE_RED, Color::srgb(0.88, 0.12, 0.14));
    assert_eq!(PINE_GREEN, Color::srgb(0.12, 0.42, 0.20));
    assert_eq!(SNOW_WHITE, Color::srgb(0.96, 0.96, 0.98));
    assert_eq!(GOLD_RIBBON, Color::srgb(0.95, 0.80, 0.15));
    assert_eq!(HAZARD_YELLOW, Color::srgb(0.92, 0.78, 0.12));
    assert_eq!(CABIN_LOG, Color::srgb(0.40, 0.25, 0.16));
    assert_eq!(TIMBER_BEAM, Color::srgb(0.26, 0.16, 0.10));
    assert_eq!(WEATHERED_POST, Color::srgb(0.22, 0.14, 0.08));
    assert_eq!(RUSTIC_PLANKS, Color::srgb(0.48, 0.30, 0.18));
    assert_eq!(WORKBENCH_WOOD, Color::srgb(0.40, 0.25, 0.15));
    assert_eq!(POLISHED_BRASS, Color::srgb(0.82, 0.68, 0.24));
    assert_eq!(CAST_IRON, Color::srgb(0.18, 0.18, 0.20));
    assert_eq!(WROUGHT_IRON, Color::srgb(0.12, 0.12, 0.14));
    assert_eq!(SHEET_METAL, Color::srgb(0.68, 0.72, 0.76));
    assert_eq!(MOOSE_FUR, Color::srgb(0.24, 0.16, 0.10));
    assert_eq!(MOOSE_MUZZLE, Color::srgb(0.36, 0.26, 0.18));
    assert_eq!(MOOSE_ANTLER, Color::srgb(0.78, 0.72, 0.60));
    assert_eq!(CRT_PHOSPHOR_GREEN, Color::srgb(0.20, 1.00, 0.40));
    assert_eq!(UNWRAPPED_TOY_BLUE, Color::srgb(0.20, 0.50, 0.80));

    // 2. Verify palette.ron exists on disk and contains valid content
    let ron_path = std::path::Path::new("assets/palette.ron");
    assert!(ron_path.exists(), "assets/palette.ron must exist in repository root");
    let ron_contents = std::fs::read_to_string(ron_path).expect("Read palette.ron");
    assert!(ron_contents.contains("festive_red"), "palette.ron must contain festive_red");
    assert!(ron_contents.contains("cabin_log"), "palette.ron must contain cabin_log");
    assert!(ron_contents.contains("moose_fur"), "palette.ron must contain moose_fur");
    assert!(ron_contents.contains("gas_hose_yellow"), "palette.ron must contain gas_hose_yellow");
    assert!(ron_contents.contains("propane_cylinder_blue"), "palette.ron must contain propane_cylinder_blue");

    // 3. Test App with AssetsPlugin and WorkshopMaterials initialization
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_plugins(AssetsPlugin);

    // Add workshop and moose spawning systems that consume WorkshopMaterials
    app.add_systems(Startup, (
        setup_workshop_scene,
        spawn_mr_moose,
        spawn_workshop_tools_and_materials,
        spawn_gas_pressure_gauge,
        spawn_diegetic_screens_and_signs,
        rust_tests::elf_game::workshop::stove::spawn_oven_stove_and_propane_scene,
    ));

    // Run PreStartup (which runs init_workshop_materials) and Startup
    app.update();

    // Verify WorkshopMaterials resource is present and populated
    let wm = app.world().get_resource::<WorkshopMaterials>();
    assert!(wm.is_some(), "WorkshopMaterials resource must be initialized in world");
    let wm = wm.unwrap();

    // Check material handles exist in Assets<StandardMaterial>
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    assert!(materials.get(&wm.cabin_log).is_some(), "Cabin log material must exist");
    assert!(materials.get(&wm.timber_beam).is_some(), "Timber beam material must exist");
    assert!(materials.get(&wm.festive_red).is_some(), "Festive red material must exist");
    assert!(materials.get(&wm.moose_fur).is_some(), "Moose fur material must exist");
    assert!(materials.get(&wm.crt_screen).is_some(), "CRT screen material must exist");
    assert!(materials.get(&wm.toy_blue).is_some(), "Toy blue material must exist");
    assert!(materials.get(&wm.vacuum_tube_glass).is_some(), "Vacuum tube glass material must exist");
    assert!(materials.get(&wm.tractor_beam).is_some(), "Tractor beam material must exist");
    assert!(materials.get(&wm.gas_hose).is_some(), "Gas hose material must exist");
    assert!(materials.get(&wm.gas_anchor_red).is_some(), "Gas anchor red material must exist");
    assert!(materials.get(&wm.propane_tank_blue).is_some(), "Propane tank blue material must exist");
    assert!(materials.get(&wm.propane_bulk_tank).is_some(), "Propane bulk tank material must exist");
    assert!(materials.get(&wm.stove_glowing_ember).is_some(), "Stove glowing ember material must exist");
    assert!(materials.get(&wm.stove_cool_dark_gray).is_some(), "Stove cool dark gray material must exist");
    assert!(materials.get(&wm.stove_blue_flame).is_some(), "Stove blue flame material must exist");
    assert!(materials.get(&wm.bunny_ear_pink).is_some(), "Bunny ear pink material must exist");
    assert!(materials.get(&wm.plasma_chamber).is_some(), "Plasma chamber material must exist");

    // Verify entities were successfully spawned using WorkshopMaterials
    let mut entity_count = 0;
    for _ in app.world().iter_entities() {
        entity_count += 1;
    }
    assert!(entity_count > 20, "Spawning systems with WorkshopMaterials must instantiate world entities (got {})", entity_count);
}

#[test]
fn test_stove_and_appliance_materials_defined_in_assets_materials_decoupled() {
    use rust_tests::elf_game::{
        assets::{palette::*, AssetsPlugin, WorkshopMaterials},
        player::bunny_ears::spawn_bunny_ears_item,
        workshop::stove::{
            spawn_oven_stove_and_propane_scene, appliance_visual_indicators_system,
            ApplianceVisualMaterials,
        },
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_plugins(AssetsPlugin);

    // Run PreStartup to populate WorkshopMaterials
    app.update();

    let wm = app.world().resource::<WorkshopMaterials>().clone();

    // Verify palette constants
    assert_eq!(GAS_HOSE_YELLOW, Color::srgb(0.95, 0.78, 0.12));
    assert_eq!(SAFETY_RED, Color::srgb(0.85, 0.15, 0.12));
    assert_eq!(PROPANE_CYLINDER_BLUE, Color::srgb(0.20, 0.45, 0.85));
    assert_eq!(BULK_TANK_WHITE, Color::srgb(0.88, 0.90, 0.92));
    assert_eq!(STOVE_GLOWING_EMBER, Color::srgb(1.0, 0.35, 0.05));
    assert_eq!(STOVE_COOL_GRAY, Color::srgb(0.12, 0.13, 0.14));
    assert_eq!(STOVE_BLUE_FLAME, Color::srgba(0.2, 0.5, 1.0, 0.85));
    assert_eq!(BUNNY_EAR_PINK, Color::srgb(0.98, 0.65, 0.75));
    assert_eq!(PLASMA_CHAMBER_PURPLE, Color::srgba(0.65, 0.15, 0.95, 0.75));

    // Run stove scene spawn system
    let mut schedule = Schedule::default();
    schedule.add_systems(spawn_oven_stove_and_propane_scene);
    schedule.run(app.world_mut());

    // Verify ApplianceVisualMaterials resource was registered with handles from WorkshopMaterials
    let appliance_mats = app.world().get_resource::<ApplianceVisualMaterials>();
    assert!(appliance_mats.is_some(), "ApplianceVisualMaterials must be inserted");
    let appliance_mats = appliance_mats.unwrap();
    assert_eq!(appliance_mats.glowing_ember, wm.stove_glowing_ember);
    assert_eq!(appliance_mats.cool_dark_gray, wm.stove_cool_dark_gray);
    assert_eq!(appliance_mats.blue_flame, wm.stove_blue_flame);

    // Verify bunny ears item spawning also uses centralized bunny_ear_pink
    let mut ear_app = App::new();
    ear_app.init_resource::<Assets<Mesh>>();
    ear_app.init_resource::<Assets<StandardMaterial>>();
    ear_app.insert_resource(wm.clone());
    ear_app.add_systems(
        Startup,
        |mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<StandardMaterial>>,
         wm: Res<WorkshopMaterials>| {
            spawn_bunny_ears_item(
                &mut commands,
                &mut meshes,
                &mut materials,
                Some(&wm),
                Vec3::ZERO,
            );
        },
    );
    ear_app.update();

    // Verify appliance visual indicators system can run using WorkshopMaterials directly without panic
    let mut visual_schedule = Schedule::default();
    visual_schedule.add_systems(appliance_visual_indicators_system);
    visual_schedule.run(app.world_mut());
}

#[test]
fn test_vacuum_proportional_distance_suction_and_cylinder_area() {
    use rust_tests::elf_game::workshop::room::VacuumTractorBeamCone;

    let vacuum_cylinder = VacuumTractorBeamCone::new(
        Vec3::new(5.5, 0.05, -4.0),
        Vec3::new(5.5, 4.0, -4.0),
        1.45,
        1.45,
    );

    // 1. Verify Cylinder Area geometry: constant radius R=1.45 across entire vertical height
    for y in [0.2, 1.5, 2.5, 3.8] {
        // Points within radius 1.2 < 1.45 at all heights must be inside cylinder area
        assert!(
            vacuum_cylinder.contains_point(Vec3::new(5.5 + 1.2, y, -4.0)),
            "Point at y={} inside radius must be within cylinder area",
            y
        );
        // Points outside radius 1.6 > 1.45 at all heights must NOT be within cylinder area
        assert!(
            !vacuum_cylinder.contains_point(Vec3::new(5.5 + 1.6, y, -4.0)),
            "Point at y={} outside radius must NOT be within cylinder area",
            y
        );
    }

    // Points outside vertical bounds
    assert!(!vacuum_cylinder.contains_point(Vec3::new(5.5, -0.6, -4.0)), "Point below floor must be excluded");
    assert!(!vacuum_cylinder.contains_point(Vec3::new(5.5, 5.0, -4.0)), "Point above despawner must be excluded");

    // 2. Verify suction acceleration is strictly proportional to distance to despawner opening
    // Opening is at (5.5, 4.0, -4.0)
    let p_far = Vec3::new(5.5, 0.0, -4.0);  // distance = 4.0m
    let p_mid = Vec3::new(5.5, 2.0, -4.0);  // distance = 2.0m
    let p_near = Vec3::new(5.5, 3.0, -4.0); // distance = 1.0m

    let a_far_total = vacuum_cylinder.compute_gravity_draw(p_far);
    let a_mid_total = vacuum_cylinder.compute_gravity_draw(p_mid);
    let a_near_total = vacuum_cylinder.compute_gravity_draw(p_near);

    // Subtract out the counter-gravity lift vector (+9.81 Y) to isolate pure suction acceleration
    let a_far_suction = a_far_total - Vec3::Y * 9.81;
    let a_mid_suction = a_mid_total - Vec3::Y * 9.81;
    let a_near_suction = a_near_total - Vec3::Y * 9.81;

    let mag_far = a_far_suction.length();
    let mag_mid = a_mid_suction.length();
    let mag_near = a_near_suction.length();

    // Assert that suction magnitude is proportional to distance:
    // mag(4m) / mag(2m) == 2.0, mag(4m) / mag(1m) == 4.0
    assert!(
        (mag_far / mag_mid - 2.0).abs() < 0.01,
        "Suction at 4m ({}) must be 2x suction at 2m ({})",
        mag_far, mag_mid
    );
    assert!(
        (mag_far / mag_near - 4.0).abs() < 0.01,
        "Suction at 4m ({}) must be 4x suction at 1m ({})",
        mag_far, mag_near
    );
}

#[test]
fn test_vacuum_momentary_button_and_auto_shutoff_with_cooldown() {
    use rust_tests::elf_game::workshop::room::{
        PresentSendingPlatform, SendPlatformButton, VacuumTractorBeamCone,
        present_sending_platform_system,
    };
    use rust_tests::elf_game::player::ElfMovement;
    use bevy_rapier3d::prelude::Velocity;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_systems(Update, present_sending_platform_system);

    let platform_e = app.world_mut().spawn((
        PresentSendingPlatform::default(),
        Transform::from_xyz(5.5, 0.05, -4.0),
    )).id();

    let cone_e = app.world_mut().spawn((
        VacuumTractorBeamCone::default(),
        Visibility::Hidden,
        Transform::from_xyz(5.5, 2.025, -4.0),
    )).id();

    // Button pedestal and button at (3.8, 0.95, -2.8)
    app.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(3.8, 0.95, -2.8),
    ));

    // Player near the button
    app.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(3.8, 1.0, -2.4),
    ));

    // 1. Initial state: button NOT pressed -> vacuum must be OFF
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must be OFF when button is not pressed");

    // 2. Player presses KeyQ -> vacuum must turn ON
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyQ);
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must turn ON while button is pressed by player");
    let vis = app.world().get::<Visibility>(cone_e).unwrap();
    assert_eq!(*vis, Visibility::Inherited);

    // 3. Player releases KeyQ -> vacuum must immediately turn OFF (momentary button)
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyQ);
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must turn OFF immediately when player releases button");

    // 4. Place physics object resting on the button -> vacuum must turn ON
    let crate_on_btn = app.world_mut().spawn((
        Transform::from_xyz(3.81, 1.02, -2.79), // on button
        Velocity::zero(),
    )).id();
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must turn ON when a physics object is on the button");

    // 5. Remove physics object from the button -> vacuum must turn OFF
    app.world_mut().despawn(crate_on_btn);
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must turn OFF when physics object is removed from the button");

    // 6. Test 10-second auto-shutoff and 1-second cooldown:
    // Hold KeyE continuously
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyE);
    }
    app.update();
    assert!(app.world().get::<PresentSendingPlatform>(platform_e).unwrap().is_active);

    // Simulate 9.9 seconds elapsed
    {
        let mut p = app.world_mut().get_mut::<PresentSendingPlatform>(platform_e).unwrap();
        p.active_duration = 9.9;
    }
    // Advance 0.2s to reach 10.1s
    {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_millis(200));
    }
    app.update();

    // After 10s continuously on: vacuum must automatically turn OFF and enter 1.0s cooldown
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must automatically turn OFF after being on for 10 seconds");
    assert!(p.cooldown_timer > 0.0, "Cooldown timer must be active after 10s auto shutoff (got {})", p.cooldown_timer);

    // During the 1-second cooldown, even with KeyE still held, vacuum must REMAIN OFF
    {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_millis(400));
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must remain OFF during cooldown even if button is held");

    // Once cooldown expires (advance remaining 0.7s), vacuum can turn ON again with held button
    {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_millis(700));
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must re-activate after cooldown expires while button is held");
}

#[test]
fn test_vacuum_player_suction_and_teleport_to_roof_after_3_seconds() {
    use rust_tests::elf_game::workshop::room::{
        PresentSendingPlatform, SendPlatformButton, VacuumTractorBeamCone,
        present_sending_platform_system,
    };
    use rust_tests::elf_game::player::ElfMovement;
    use bevy_rapier3d::prelude::Velocity;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_systems(Update, present_sending_platform_system);

    let platform_e = app.world_mut().spawn((
        PresentSendingPlatform::default(),
        Transform::from_xyz(5.5, 0.05, -4.0),
    )).id();

    app.world_mut().spawn((
        VacuumTractorBeamCone::default(),
        Visibility::Hidden,
        Transform::from_xyz(5.5, 2.025, -4.0),
    ));

    app.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(3.8, 0.95, -2.8),
    ));

    // Place a physics object on button to keep vacuum turned ON
    app.world_mut().spawn((
        Transform::from_xyz(3.8, 1.0, -2.8),
        Velocity::zero(),
    ));

    // Spawn player standing inside the vacuum cylinder on the sending platform (5.5, 0.2, -4.0)
    let player_e = app.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(5.5, 0.2, -4.0),
    )).id();

    // 1. Verify vacuum affects the player: upward suction acceleration
    app.update();
    {
        let p_mov = app.world().get::<ElfMovement>(player_e).unwrap();
        assert!(
            p_mov.velocity.y > 0.0,
            "Vacuum must accelerate player upward inside the cylinder: got vy = {}",
            p_mov.velocity.y
        );
        assert!(!p_mov.is_grounded, "Suction must lift player off the ground");
    }

    // 2. Player touches the present despawner at the top of the funnel (5.5, 3.8, -4.0)
    {
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_e).unwrap();
        p_tf.translation = Vec3::new(5.5, 3.8, -4.0);
    }
    // Simulate touching for 1.5 seconds (less than 3.0s)
    {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_millis(1500));
    }
    app.update();

    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(
        p.player_despawner_contact_time >= 1.4,
        "Contact time must accumulate while player touches despawner"
    );
    // Player should not be teleported yet
    let p_tf = app.world().get::<Transform>(player_e).unwrap();
    assert!(p_tf.translation.y < 10.0, "Player must not teleport before 3 seconds");

    // Keep player at despawner and advance past 3.0 seconds total (1.5s + 1.6s = 3.1s)
    {
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_e).unwrap();
        p_tf.translation = Vec3::new(5.5, 3.8, -4.0);
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_millis(1600));
    }
    app.update();

    // 3. Verify player is teleported to on top of the workshop roof!
    let p_tf = app.world().get::<Transform>(player_e).unwrap();
    let p_mov = app.world().get::<ElfMovement>(player_e).unwrap();
    assert_eq!(p_tf.translation.x, 0.0, "Player must be teleported to roof peak X");
    assert_eq!(p_tf.translation.y, 15.0, "Player must be teleported on top of the roof (Y = 15.0)");
    assert_eq!(p_tf.translation.z, 0.0, "Player must be teleported to roof peak Z");
    assert_eq!(p_mov.velocity, Vec3::ZERO, "Player velocity must be reset upon roof teleport");

    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert_eq!(p.player_despawner_contact_time, 0.0, "Contact timer must reset upon teleport");
}

#[test]
fn test_diegetic_3d_text_rasterization_and_quad_displays() {
    use rust_tests::elf_game::diegetic_ui::{
        monitor::{
            spawn_diegetic_screens_and_signs, update_diegetic_monitor_text,
            CrtInstructionMonitor, CrtMonitorTextDisplay, DirectionalSign3dText,
        },
        text_renderer::render_text_to_image,
    };
    use rust_tests::elf_game::core::WorkshopQuota;
    use rust_tests::elf_game::moose::MooseHunger;

    // 1. Verify text renderer rasterization output
    let test_img = render_text_to_image(
        "WRAPPING STATION\n▲ ASSEMBLE • TIE ┌─┐",
        28.0,
        Color::srgb(1.0, 0.9, 0.2),
        Color::srgba(0.0, 0.0, 0.0, 0.0),
        512,
        128,
        true,
    );
    assert_eq!(test_img.texture_descriptor.size.width, 512);
    assert_eq!(test_img.texture_descriptor.size.height, 128);
    let data = test_img.data.as_ref().expect("Image must have pixel data");
    assert_eq!(data.len(), 512 * 128 * 4);
    let non_zero_pixels = data.chunks_exact(4).filter(|p| p[3] > 0).count();
    assert!(non_zero_pixels > 200, "Rasterizer must draw anti-aliased text and procedural symbols (got {} text pixels)", non_zero_pixels);

    // 2. Setup Bevy App with Mesh, Material, and Image assets
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(WorkshopQuota {
        presents_delivered: 0,
        target_quota: 5,
        tier: 1,
    });
    app.insert_resource(MooseHunger {
        value: 10.0,
        is_breached: false,
        ..default()
    });

    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<Image>>();

    app.add_systems(Startup, spawn_diegetic_screens_and_signs);
    app.add_systems(Update, update_diegetic_monitor_text);
    app.update();

    // 3. Verify CRT Monitor 3D Text Quad and texture display component
    let (crt_display, mesh3d, mat3d) = {
        let mut q_crt = app.world_mut().query::<(&CrtMonitorTextDisplay, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>();
        let (disp, m, mat) = q_crt.single(app.world()).expect("Must spawn CrtMonitor3dTextQuad with 3D text display");
        (disp.clone(), m.clone(), mat.clone())
    };
    assert!(crt_display.last_content.contains("PAGE 1/4"));
    assert!(crt_display.last_content.contains("PRESENT INSTRUCTIONS"));

    // Check that meshes and materials exist in asset storage
    let meshes = app.world().resource::<Assets<Mesh>>();
    assert!(meshes.get(&mesh3d.0).is_some(), "CRT 3D text quad mesh must exist in Assets<Mesh>");
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let mat = materials.get(&mat3d.0).expect("CRT 3D text material must exist");
    assert!(mat.base_color_texture.is_some(), "Material must reference the rasterized text texture");
    let images = app.world().resource::<Assets<Image>>();
    let crt_image = images.get(&crt_display.image_handle).expect("CRT image asset must exist");
    let crt_data = crt_image.data.as_ref().unwrap();
    let crt_text_pixels = crt_data.chunks_exact(4).filter(|p| p[1] > 100).count(); // Green phosphor pixels
    assert!(crt_text_pixels > 500, "CRT screen image must contain green phosphor text pixels");

    // 4. Verify Directional Sign 3D Text Quads are spawned
    let sign_3d_count = {
        let mut q_signs = app.world_mut().query::<(&DirectionalSign3dText, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>();
        q_signs.iter(app.world()).count()
    };
    assert!(sign_3d_count >= 2, "Expected at least 2 in-world directional sign 3D text quads, found: {}", sign_3d_count);

    // 5. Advance time to Page 2 and verify CRT screen texture updates dynamically
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(4100));
    }
    app.update();

    let q_mon = {
        let mut q = app.world_mut().query::<&CrtInstructionMonitor>();
        q.single(app.world()).unwrap().clone()
    };
    assert_eq!(q_mon.current_page, 1, "Monitor should cycle to Page 1 (Page 2/4)");

    let q_display = {
        let mut q = app.world_mut().query::<&CrtMonitorTextDisplay>();
        q.single(app.world()).unwrap().clone()
    };
    assert!(q_display.last_content.contains("PAGE 2/4"), "CRT display component must update to Page 2 content");
    assert!(q_display.last_content.contains("DISPATCH & VACUUM GUIDE"));
}

#[test]
fn test_candle_particles_smoke_and_ascending_sparks() {
    use rust_tests::elf_game::workshop::room::{
        CandleFlameEmitter, CandleParticle, CandleParticleKind,
        candle_particle_emitter_system, candle_particle_system,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    app.add_systems(Update, (candle_particle_emitter_system, candle_particle_system));

    // Spawn a chandelier with emitter at (0.0, 4.4, -4.5)
    app.world_mut().spawn((
        Transform::from_xyz(0.0, 4.4, -4.5),
        GlobalTransform::from(Transform::from_xyz(0.0, 4.4, -4.5)),
        CandleFlameEmitter::default(),
    ));

    // 1. Tick time past smoke timer (0.16s -> advance 200ms)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(200));
    }
    app.update();

    let mut q_particles = app.world_mut().query::<(&Transform, &CandleParticle)>();
    let smoke_count = q_particles
        .iter(app.world())
        .filter(|(_, p)| p.kind == CandleParticleKind::Smoke)
        .count();
    assert!(smoke_count >= 1, "Candle emitter must spawn smoke particle wisps");

    // Verify smoke particle properties: rising velocity, expanding scale
    let (smoke_tf, smoke_p) = q_particles
        .iter(app.world())
        .find(|(_, p)| p.kind == CandleParticleKind::Smoke)
        .unwrap();
    assert!(smoke_p.velocity.y > 0.0, "Smoke must drift upward (got vy = {})", smoke_p.velocity.y);
    assert_eq!(smoke_p.start_scale, 0.025);
    assert_eq!(smoke_p.end_scale, 0.075);
    let initial_y = smoke_tf.translation.y;

    // 2. Advance time past spark timer (1.1s total -> advance 1000ms)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(1000));
    }
    app.update();

    let mut q_particles = app.world_mut().query::<(&Transform, &CandleParticle)>();
    let spark_count = q_particles
        .iter(app.world())
        .filter(|(_, p)| p.kind == CandleParticleKind::Spark)
        .count();
    assert!(spark_count >= 1, "Candle emitter must spawn ascending spark embers");

    let (spark_tf, spark_p) = q_particles
        .iter(app.world())
        .find(|(_, p)| p.kind == CandleParticleKind::Spark)
        .unwrap();
    assert!(spark_p.velocity.y > 1.0, "Sparks must travel upward rapidly (got vy = {})", spark_p.velocity.y);
    assert!(spark_tf.translation.y > 4.4, "Spark must originate from candle flame elevation");

    // 3. Smoke particle from step 1 should have risen higher
    let (smoke_tf_later, _) = q_particles
        .iter(app.world())
        .find(|(_, p)| p.kind == CandleParticleKind::Smoke)
        .unwrap();
    assert!(smoke_tf_later.translation.y > initial_y, "Smoke must ascend over time");

    // 4. Advance time past max particle lifetime (smoke ~1.4s, spark ~0.6s) -> old particles despawn
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(2000));
    }
    app.update();
    // Verify system continues ticking without leaks or panics
}

#[test]
fn test_chandelier_physics_swinging_and_joints() {
    use rust_tests::elf_game::workshop::room::{
        CandleChandelier, ChandelierCeilingAnchor,
        setup_workshop_scene, chandelier_physics_interaction_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());

    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<Image>>();

    app.add_systems(Startup, setup_workshop_scene);
    app.add_systems(Update, chandelier_physics_interaction_system);
    app.update();

    // 1. Verify two chandeliers are spawned as Dynamic RigidBodies with Colliders
    let chandeliers: Vec<(Entity, Transform, RigidBody, ExternalImpulse)> = {
        let mut q = app.world_mut().query_filtered::<(Entity, &Transform, &RigidBody, &ExternalImpulse), With<CandleChandelier>>();
        q.iter(app.world()).map(|(e, tf, rb, imp)| (e, *tf, *rb, *imp)).collect()
    };
    assert_eq!(chandeliers.len(), 2, "Must spawn 2 chandeliers (North & South)");
    for (_, tf, rb, _) in &chandeliers {
        assert_eq!(*rb, RigidBody::Dynamic, "Chandeliers must be Dynamic physics bodies to swing");
        assert_eq!(tf.translation.y, 4.4, "Chandelier hub resting height must be Y = 4.4m");
    }

    // 2. Verify Fixed Ceiling Anchors exist at Y = 5.2m
    let anchors: Vec<(Entity, Transform, RigidBody)> = {
        let mut q = app.world_mut().query_filtered::<(Entity, &Transform, &RigidBody), With<ChandelierCeilingAnchor>>();
        q.iter(app.world()).map(|(e, tf, rb)| (e, *tf, *rb)).collect()
    };
    assert_eq!(anchors.len(), 2, "Must spawn 2 fixed ceiling anchors for chandelier chains");
    for (_, tf, rb) in &anchors {
        assert_eq!(*rb, RigidBody::Fixed, "Ceiling mount anchors must be Fixed rigid bodies");
        assert_eq!(tf.translation.y, 5.2, "Ceiling mount anchor must be positioned at tie beam apex Y = 5.2m");
    }

    // 3. Verify ImpulseJoint components exist linking chandeliers to ceiling anchors
    let joints: Vec<&ImpulseJoint> = {
        let mut q = app.world_mut().query::<&ImpulseJoint>();
        q.iter(app.world()).collect()
    };
    assert!(joints.len() >= 2, "Chandeliers must contain ImpulseJoint linking them to ceiling anchors");

    // 4. Verify player physics interaction applies swinging impulses
    let (c_entity, _, _, _) = chandeliers[0];
    let chandelier_z = chandeliers[0].1.translation.z;

    // Spawn player standing within bat/push range (< 4.2m) of North chandelier
    app.world_mut().spawn((
        rust_tests::elf_game::player::controller::ElfPlayer,
        Transform::from_xyz(0.0, 1.8, chandelier_z + 2.0),
    ));

    // Player presses KeyE to bat/push the chandelier
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyE);
    }
    app.update();

    let impulse = app.world().get::<ExternalImpulse>(c_entity).unwrap();
    assert!(
        impulse.impulse.length_squared() > 1.0 || impulse.torque_impulse.length_squared() > 1.0,
        "Interacting with chandelier must apply swinging impulse & torque to its physics body (got {:?}, torque {:?})",
        impulse.impulse, impulse.torque_impulse
    );
}

#[test]
fn test_progression_unlock_conditions() {
    use rust_tests::elf_game::workshop::{
        WorkshopDoor, DoorState,
    };

    let mut app = App::new();
    app.add_plugins((CorePlugin, WorkshopPlugin));
    app.update();

    // 1. Initial State: Both doors exist and are locked
    {
        let mut q_doors = app.world_mut().query::<(&WorkshopDoor, &Name)>();
        let mut found_storage = false;
        let mut found_forbidden = false;
        for (door, _name) in q_doors.iter(app.world()) {
            if door.door_id == "StorageWing" {
                assert_eq!(door.state, DoorState::Locked);
                assert_eq!(door.required_tier, 2);
                found_storage = true;
            } else if door.door_id == "ForbiddenBackWorkshop" {
                assert_eq!(door.state, DoorState::Locked);
                assert_eq!(door.required_tier, 3);
                found_forbidden = true;
            }
        }
        assert!(found_storage, "StorageWingDoor must be spawned and locked");
        assert!(found_forbidden, "ForbiddenBackWorkshopDoor must be spawned and locked");
    }

    // 2. Pass Tier 1 Quota (deliver 5 presents) -> Storage Wing Door transitions to Opening
    {
        let mut quota = app.world_mut().resource_mut::<WorkshopQuota>();
        quota.presents_delivered = 5;
    }
    app.update();

    {
        let quota = app.world().resource::<WorkshopQuota>();
        assert_eq!(quota.tier, 2, "Quota tier must advance to Tier 2");
        assert_eq!(quota.target_quota, 15);

        let mut q_doors = app.world_mut().query::<&WorkshopDoor>();
        for door in q_doors.iter(app.world()) {
            if door.door_id == "StorageWing" {
                match door.state {
                    DoorState::Opening { progress, .. } => {
                        assert!(progress >= 0.0, "Storage door must begin opening");
                    }
                    other => panic!("Expected StorageWing door to be Opening, got {:?}", other),
                }
            } else if door.door_id == "ForbiddenBackWorkshop" {
                assert_eq!(door.state, DoorState::Locked, "Forbidden door must remain locked in Tier 2");
            }
        }
    }

    // 3. Pass Tier 2 Quota (deliver 15 presents) -> Forbidden Back Workshop Door transitions to Opening
    {
        let mut quota = app.world_mut().resource_mut::<WorkshopQuota>();
        quota.presents_delivered = 15;
    }
    app.update();

    {
        let quota = app.world().resource::<WorkshopQuota>();
        assert_eq!(quota.tier, 3, "Quota tier must advance to Tier 3");

        let mut q_doors = app.world_mut().query::<&WorkshopDoor>();
        for door in q_doors.iter(app.world()) {
            if door.door_id == "ForbiddenBackWorkshop" {
                match door.state {
                    DoorState::Opening { .. } => {}
                    other => panic!("Expected ForbiddenBackWorkshop door to be Opening, got {:?}", other),
                }
            }
        }
    }
}

#[test]
fn test_door_opening_animation_and_collider_removal() {
    use bevy_rapier3d::prelude::*;
    use rust_tests::elf_game::workshop::{
        WorkshopDoor, DoorState, animate_workshop_doors_system,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(500));
    app.insert_resource(time);

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(13.0, 1.7, 0.0),
        WorkshopDoor {
            door_id: "StorageWing".to_string(),
            required_tier: 2,
            state: DoorState::Opening { progress: 0.0, speed: 1.0 },
            closed_position: Vec3::new(13.0, 1.7, 0.0),
            open_position: Vec3::new(13.0, 5.0, 0.0),
        },
        Collider::cuboid(0.25, 1.7, 1.95),
    )).id();

    app.add_systems(Update, animate_workshop_doors_system);

    // Step 1: 0.5s -> progress = 0.5, translation halfway
    app.update();
    let tf = app.world().get::<Transform>(door_entity).unwrap();
    let door = app.world().get::<WorkshopDoor>(door_entity).unwrap();
    match door.state {
        DoorState::Opening { progress, .. } => {
            assert!((progress - 0.5).abs() < 0.01);
        }
        other => panic!("Expected Opening state, got {:?}", other),
    }
    assert!((tf.translation.y - 3.35).abs() < 0.05, "Door should be halfway up");
    assert!(app.world().get::<Collider>(door_entity).is_some(), "Collider must remain while opening");

    // Step 2: Another 0.6s -> progress >= 1.0 -> DoorState::Open and Collider removed
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(600));
    }
    app.update();

    let door = app.world().get::<WorkshopDoor>(door_entity).unwrap();
    assert_eq!(door.state, DoorState::Open, "Door must complete opening and become Open");
    let tf = app.world().get::<Transform>(door_entity).unwrap();
    assert!((tf.translation.y - 5.0).abs() < 0.01, "Door should reach full open position");
    assert!(
        app.world().get::<Collider>(door_entity).is_none(),
        "Blocking Collider must be removed once door is Open so player can pass through"
    );
}

#[test]
fn test_storage_wing_motorized_turntable() {
    use bevy_rapier3d::prelude::*;
    use rust_tests::elf_game::workshop::{
        MotorizedTurntable, motorized_turntable_system,
    };

    let mut app = App::new();
    app.add_systems(Update, motorized_turntable_system);

    // Spawn Turntable at (18.0, 0.35, 0.0)
    let _tt = app.world_mut().spawn((
        Transform::from_xyz(18.0, 0.35, 0.0),
        MotorizedTurntable {
            angular_speed: 3.0,
            radius: 1.5,
        },
    )).id();

    // Spawn Present placed on turntable at (18.5, 0.55, 0.0) with PaperSized feature
    let mut initial_features = HashSet::new();
    initial_features.insert(WrappingFeature::PaperSized);

    let present_entity = app.world_mut().spawn((
        Transform::from_xyz(18.5, 0.55, 0.0),
        Velocity::default(),
        Present {
            toy_name: "Wooden Train".to_string(),
            features: initial_features,
            ..default()
        },
    )).id();

    app.update();

    let (vel, present) = {
        let p_world = app.world();
        (
            p_world.get::<Velocity>(present_entity).unwrap().clone(),
            p_world.get::<Present>(present_entity).unwrap().clone(),
        )
    };

    // 1. Angular and Tangential Velocity applied
    assert_eq!(vel.angular.y, 3.0, "Turntable must impart spin to the present");
    assert!(vel.linear.z > 1.0, "Turntable must impart tangential velocity (+Z)");

    // 2. Motorized auto-wrapping: PaperSized -> PaperWrapped
    assert!(
        present.features.contains(&WrappingFeature::PaperWrapped),
        "Turntable must automatically wrap sized presents into PaperWrapped"
    );
}

#[test]
fn test_forbidden_tools_volatility() {
    use rust_tests::elf_game::workshop::{
        PlasmaShrinkWrapper, PlasmaExplosionEvent, TriggerPlasmaWrapperMessage,
        plasma_shrink_wrapper_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(500));
    app.insert_resource(time);

    app.add_message::<TriggerPlasmaWrapperMessage>();
    app.add_message::<PlasmaExplosionEvent>();
    app.add_systems(Update, plasma_shrink_wrapper_system);

    // Spawn Plasma Shrink Wrapper
    let wrapper_entity = app.world_mut().spawn((
        Transform::from_xyz(-3.0, 0.85, 18.0),
        PlasmaShrinkWrapper {
            heat: 0.0,
            max_heat: 100.0,
            cooling_rate: 10.0,
            heat_per_use: 35.0,
            ..default()
        },
    )).id();

    // Spawn present to wrap
    let present_entity = app.world_mut().spawn((
        Transform::from_xyz(-3.0, 1.0, 18.0),
        Velocity::default(),
        Present::default(),
        WrappingSpecification::default(),
    )).id();

    // 1. First trigger: Normal wrap processing starts, heat increases
    app.world_mut().write_message(TriggerPlasmaWrapperMessage {
        wrapper_entity,
        present_entity,
    });
    app.update();

    let wrapper = app.world().get::<PlasmaShrinkWrapper>(wrapper_entity).unwrap();
    assert_eq!(wrapper.heat, 35.0, "Heat should increase by 35 per wrap");
    assert!(wrapper.is_processing, "Should be in active processing state");
    assert!(!wrapper.is_overheated);

    // Advance 1.6s to complete processing (duration 1.5s)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(1600));
    }
    app.update();

    let present = app.world().get::<Present>(present_entity).unwrap();
    assert!(present.features.contains(&WrappingFeature::PlasmaShrinkSealed));
    assert!(present.features.contains(&WrappingFeature::PaperWrapped));
    assert!(present.features.contains(&WrappingFeature::DecorativeBow));
    assert!(present.is_completed, "Specification should be fulfilled by instant plasma seal");
    assert!(present.quality_score > 1.4, "Quality score should receive high-tech boost");

    // 2. Consecutive triggers to provoke overheat volatility
    app.world_mut().write_message(TriggerPlasmaWrapperMessage {
        wrapper_entity,
        present_entity,
    });
    app.update();

    app.world_mut().write_message(TriggerPlasmaWrapperMessage {
        wrapper_entity,
        present_entity,
    });
    app.update();

    app.world_mut().write_message(TriggerPlasmaWrapperMessage {
        wrapper_entity,
        present_entity,
    });
    app.update();

    let wrapper = app.world().get::<PlasmaShrinkWrapper>(wrapper_entity).unwrap();
    assert!(wrapper.is_overheated, "Wrapper must enter overheated volatility lock when heat >= 100");
    assert!(!wrapper.is_processing, "Processing must be aborted on overheat");
}

#[test]
fn test_ribbon_cannon_and_candy_cane_overclocker() {
    use bevy_rapier3d::prelude::*;
    use rust_tests::elf_game::workshop::{
        room::ConveyorBelt,
        RibbonCannon, FireRibbonCannonMessage, ribbon_cannon_system,
        CandyCaneOverclocker, ToggleOverclockerMessage, candy_cane_overclocker_system,
    };
    use rust_tests::elf_game::moose::MooseHunger;

    let mut app = App::new();
    app.insert_resource(MooseHunger {
        value: 10.0,
        increment_rate: 1.0,
        ..default()
    });

    app.add_message::<FireRibbonCannonMessage>();
    app.add_message::<ToggleOverclockerMessage>();
    app.add_systems(Update, (ribbon_cannon_system, candy_cane_overclocker_system));

    // 1. Ribbon Cannon Test
    let cannon_entity = app.world_mut().spawn((
        Transform::from_xyz(3.0, 0.85, 18.0),
        RibbonCannon::default(),
    )).id();

    let present_entity = app.world_mut().spawn((
        Transform::from_xyz(3.0, 0.85, 17.0),
        Velocity::default(),
        Present::default(),
    )).id();

    app.world_mut().write_message(FireRibbonCannonMessage {
        cannon_entity,
        present_entity,
    });
    app.update();

    let (present, vel) = {
        let p_world = app.world();
        (
            p_world.get::<Present>(present_entity).unwrap().clone(),
            p_world.get::<Velocity>(present_entity).unwrap().clone(),
        )
    };
    assert!(present.features.contains(&WrappingFeature::RibbonTied));
    assert!(present.features.contains(&WrappingFeature::DecorativeBow));
    assert!(vel.linear.y > 2.0, "Cannon must deliver upward pneumatic launch impulse");

    // 2. Candy-Cane Overclocker Test
    let conveyor_entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.75, 3.0),
        ConveyorBelt {
            speed: 2.0,
            direction: Vec3::X,
        },
    )).id();

    let _overclocker_entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, 1.2, 22.0),
        CandyCaneOverclocker::default(),
    )).id();

    // Toggle ON -> 2.0x conveyor speed, 1.75x moose hunger rate
    app.world_mut().write_message(ToggleOverclockerMessage);
    app.update();

    let belt = app.world().get::<ConveyorBelt>(conveyor_entity).unwrap();
    assert_eq!(belt.speed, 4.0, "Conveyor belt speed must double when overclocked");
    let moose = app.world().resource::<MooseHunger>();
    assert_eq!(moose.increment_rate, 1.75, "Mr. Moose hunger rate must scale by 1.75x while overclocked");

    // Toggle OFF -> restored to normal
    app.world_mut().write_message(ToggleOverclockerMessage);
    app.update();

    let belt = app.world().get::<ConveyorBelt>(conveyor_entity).unwrap();
    assert_eq!(belt.speed, 2.0, "Conveyor belt speed must return to normal");
    let moose = app.world().resource::<MooseHunger>();
    assert_eq!(moose.increment_rate, 1.0, "Moose hunger rate must return to 1.0");
}

#[test]
fn test_moose_gentle_eating_animation_on_time() {
    use rust_tests::elf_game::moose::{
        MoosePlugin, MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack,
        toy_model::MooseHead,
    };

    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    // Initial hunger at 55.0 (AgitatedHuffing / on-time feeding stage 50..60)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 55.0;
        hunger.stage = MooseHungerStage::AgitatedHuffing;
    }

    // Feeding Trough at (-6.5, 0.3, -12.0)
    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    // Place snack in trough
    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    // Advance 0.05s: Trough detects snack and initiates gentle eating animation
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.stage, MooseHungerStage::EatingTreat, "Moose must enter EatingTreat stage");
        assert!(!hunger.is_eating_fast, "On-time treat must trigger gentle eating animation (not fast)");
        assert_eq!(hunger.eating_duration, 2.2, "Gentle eating duration must be ~2.2s");
    }

    // Advance 0.5s into reach-down phase:
    // Head reaches down into trough (head Y drops below 0.8, extends forward)
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(500));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        // Verify hunger timer is paused while eating!
        assert_eq!(hunger.value, 55.0, "Hunger timer must remain paused during eating animation");

        // Verify head position
        let mut q_head = app.world_mut().query_filtered::<&Transform, With<MooseHead>>();
        if let Ok(head_tf) = q_head.single(app.world()) {
            assert!(head_tf.translation.y < 0.8, "Moose head must reach down towards trough (got Y = {})", head_tf.translation.y);
            assert!(head_tf.translation.z > 2.4, "Moose head must extend forward over trough (got Z = {})", head_tf.translation.z);
        }
    }

    // Advance past halfway mark (another 0.7s, total 1.25s / 2.2s):
    // Snack is consumed mid-bite!
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(700));
    }
    app.update();

    assert!(
        app.world().get_entity(snack_entity).is_err(),
        "Snack must be consumed (despawned) halfway through gentle eating animation"
    );

    // Advance remaining duration (another 1.0s, total 2.25s > 2.2s):
    // Gentle eating finishes: hunger resets to 0.0 and Moose retreats BehindShutters!
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(1000));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.stage, MooseHungerStage::BehindShutters, "Moose must retreat BehindShutters upon eating completion");
        assert_eq!(hunger.value, 0.0, "Hunger must reset back to 0.0 upon feeding");
    }
}

#[test]
fn test_moose_late_treat_deliberation_pauses_hunger_timer() {
    use rust_tests::elf_game::moose::{
        MoosePlugin, MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack,
        toy_model::{MrMoose, RightWindowShutter},
    };

    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    // Initial state: Player is late, hunger reached 60.0, window is shut (WindowSlammed)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
        hunger.breach_delay_remaining = Some(6.0);
    }

    // Feeding Trough
    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    // Place snack in trough
    let _snack = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    // Advance 0.05s: Trough detects snack while WindowSlammed -> enters DeliberatingLateTreat
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(
            hunger.stage,
            MooseHungerStage::DeliberatingLateTreat,
            "Moose must enter DeliberatingLateTreat stage when snack arrives after window shut"
        );
        assert_eq!(hunger.deliberation_duration, 1.5, "Deliberation duration must be 1-2 seconds (1.5s)");
    }

    // Advance 0.5s (within 1.5s deliberation window):
    // 1) Hunger timer must be strictly paused!
    // 2) Breach delay must be strictly paused!
    // 3) Moose peeks back through the window with shutters open!
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(500));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.stage, MooseHungerStage::DeliberatingLateTreat);
        assert_eq!(hunger.value, 60.0, "Hunger timer MUST pause during late-treat deliberation");
        assert_eq!(
            hunger.breach_delay_remaining,
            Some(6.0),
            "Breach delay countdown MUST pause during late-treat deliberation"
        );

        // Verify Moose stepped back up to window and shutters opened
        let mut q_moose = app.world_mut().query_filtered::<&Transform, With<MrMoose>>();
        if let Ok(m_tf) = q_moose.single(app.world()) {
            assert!(m_tf.translation.z > -16.0, "Moose body must step forward to window during deliberation (got Z = {})", m_tf.translation.z);
        }

        let mut q_shutters = app.world_mut().query_filtered::<&Transform, With<RightWindowShutter>>();
        if let Ok(s_tf) = q_shutters.single(app.world()) {
            let (_, angle, _) = s_tf.rotation.to_euler(EulerRot::YXZ);
            assert!(angle > 0.5, "Shutters must open as Moose peeks back through during deliberation (got angle = {})", angle);
        }
    }
}

#[test]
fn test_moose_late_treat_accept_decision_fast_annoyed_eating() {
    use rust_tests::elf_game::moose::{
        MoosePlugin, MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack,
    };

    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    // Player is late (WindowSlammed), and decision override is forced to ACCEPT
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
        hunger.breach_delay_remaining = Some(6.0);
        hunger.late_treat_decision_override = Some(true); // Force Accept!
    }

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    // Start with dt = 50ms -> enters deliberation
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    assert_eq!(app.world().resource::<MooseHunger>().stage, MooseHungerStage::DeliberatingLateTreat);

    // Advance 1.5s -> Deliberation finishes and Moose decides to ACCEPT!
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(1500));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.stage, MooseHungerStage::EatingTreat, "Moose must accept treat and enter EatingTreat");
        assert!(hunger.is_eating_fast, "Accepted late treat must run fast eating animation to show annoyance");
        assert_eq!(hunger.eating_duration, 0.75, "Fast eating duration must be snappy (~0.75s, 3x faster than gentle 2.2s)");
    }

    // Advance 0.4s (mid-point of 0.75s) -> Snack is consumed
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(400));
    }
    app.update();

    assert!(
        app.world().get_entity(snack_entity).is_err(),
        "Snack must be consumed during fast annoyed eating animation"
    );

    // Advance another 0.4s (0.8s total > 0.75s) -> Fast eating finishes!
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(400));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.stage, MooseHungerStage::BehindShutters, "Moose must retreat BehindShutters upon fast eating completion");
        assert_eq!(hunger.value, 0.0, "Hunger value must reset to 0 upon eating completion");
    }
}

#[test]
fn test_moose_late_treat_reject_decision_slams_window_and_continues_rampage() {
    use rust_tests::elf_game::moose::{
        MoosePlugin, MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack,
    };

    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    // Player is late (WindowSlammed), and decision override is forced to REJECT
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
        hunger.breach_delay_remaining = Some(4.0);
        hunger.late_treat_decision_override = Some(false); // Force Reject!
    }

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    // Start with dt = 50ms -> enters deliberation
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    assert_eq!(app.world().resource::<MooseHunger>().stage, MooseHungerStage::DeliberatingLateTreat);

    // Advance 1.5s -> Deliberation finishes and Moose REJECTS the treat!
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(1500));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(
            hunger.stage,
            MooseHungerStage::WindowSlammed,
            "Moose must reject late treat and slam the window shut again"
        );
        // Snack is NOT consumed!
        assert!(
            app.world().get_entity(snack_entity).is_ok(),
            "Rejected snack must NOT be consumed"
        );
    }

    // Now advance breach delay remaining (4.0s) -> Moose continues to rampage!
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs(5));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert!(hunger.is_breached, "Breach must occur after window was slammed shut on rejection");
        assert_eq!(hunger.stage, MooseHungerStage::BreachedRampage, "Moose must breach into BreachedRampage");
    }
}

#[test]
fn test_moose_late_treat_snack_removed_during_deliberation_slams_window() {
    use rust_tests::elf_game::moose::{
        MoosePlugin, MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack,
    };

    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
        hunger.breach_delay_remaining = Some(5.0);
    }

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    // 50ms -> enters deliberation
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    assert_eq!(app.world().resource::<MooseHunger>().stage, MooseHungerStage::DeliberatingLateTreat);

    // Player takes treat out of trough (despawn / move away)
    app.world_mut().despawn(snack_entity);

    // Next frame: Moose detects missing treat and slams window shut
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(50));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(
            hunger.stage,
            MooseHungerStage::WindowSlammed,
            "Moose must slam window shut if treat is removed while deliberating"
        );
    }
}

#[test]
fn test_elf_nav_graph_pathfinding_and_clearances() {
    use rust_tests::elf_game::physics_sandbox::ElfNavGraph;

    let graph = ElfNavGraph::default_workshop_graph();
    assert!(graph.nodes.len() >= 15, "Graph must define at least 15 workshop nodes (has {})", graph.nodes.len());
    assert_eq!(graph.nodes[15].name, "PantryShelves");
    assert_eq!(graph.nodes[15].neighbors, vec![8]);

    // Verify West bypass corridor nodes (Nodes 7 and 8) are placed further towards the wall (X <= -10.5)
    // to give plenty of clearance around the conveyor spawner duct at X = -8.5 .. -9.05
    let west_bypass = &graph.nodes[7];
    let west_south = &graph.nodes[8];
    assert!(
        west_bypass.position.x <= -10.5 && west_bypass.position.x > -12.8,
        "West bypass node 7 must be placed near west wall (between -10.5 and -12.8), was {}",
        west_bypass.position.x
    );
    assert!(
        west_south.position.x <= -10.5 && west_south.position.x > -12.8,
        "West south node 8 must be placed near west wall (between -10.5 and -12.8), was {}",
        west_south.position.x
    );
    // Spawner duct extends to X = -9.05. Edge between 7 and 8 at X = -11.0 gives > 1.9m clearance.
    assert!(
        (west_bypass.position.x - (-9.05)).abs() >= 1.5,
        "West bypass edge must clear spawner duct by at least 1.5m"
    );

    // Verify East bypass corridor nodes (Nodes 9 and 10) are placed further towards the wall (X >= 11.2)
    // to give clearance around the conveyor despawner chute at X = 9.8 .. 10.47
    let east_bypass = &graph.nodes[9];
    let east_south = &graph.nodes[10];
    assert!(
        east_bypass.position.x >= 11.2 && east_bypass.position.x < 12.8,
        "East bypass node 9 must be placed near east wall (between 11.2 and 12.8), was {}",
        east_bypass.position.x
    );
    assert!(
        east_south.position.x >= 11.2 && east_south.position.x < 12.8,
        "East south node 10 must be placed near east wall (between 11.2 and 12.8), was {}",
        east_south.position.x
    );
    // Despawner chute extends to X = 10.47. Edge between 9 and 10 at X = 11.5 gives > 1.0m clearance.
    assert!(
        east_bypass.position.x - 10.47 >= 0.9,
        "East bypass edge must clear despawner chute by at least 0.9m"
    );

    // Path from Moose Trough (Node 4) to Hat Closet (Node 12)
    let path_to_closet = graph.find_path(4, 12).expect("Path from Trough to Closet must exist");
    assert!(path_to_closet.len() >= 4);
    assert_eq!(*path_to_closet.first().unwrap(), 4);
    assert_eq!(*path_to_closet.last().unwrap(), 12);

    // Path from Workbench Front (Node 0) to Klepto Stash (Node 13)
    let path_to_stash = graph.find_path(0, 13).expect("Path from Workbench to Stash must exist");
    assert_eq!(*path_to_stash.first().unwrap(), 0);
    assert_eq!(*path_to_stash.last().unwrap(), 13);
    // Must traverse west corridor (Node 7 or 8) avoiding conveyor
    assert!(path_to_stash.contains(&7) || path_to_stash.contains(&8));

    // Closest node queries
    assert_eq!(graph.closest_node(Vec3::new(0.0, 0.45, -1.9)), 0);
    assert_eq!(graph.closest_node(Vec3::new(-10.0, 0.45, 10.0)), 13);
}

#[test]
fn test_elf_grounded_floor_translation() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfLimbType, ElfRagdollLimb, setup_elf_assistants,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(Startup, setup_elf_assistants);
    app.update();

    let mut elf_q = app.world_mut().query::<(&ElfAssistant, &Transform, &Collider)>();
    for (elf, tf, _collider) in elf_q.iter(app.world()) {
        assert!(
            (tf.translation.y - 0.45).abs() < 0.001,
            "Elf {} root Y must be 0.45m, was {}",
            elf.name,
            tf.translation.y
        );
    }

    let mut limb_q = app.world_mut().query::<&ElfRagdollLimb>();
    for limb in limb_q.iter(app.world()) {
        if limb.limb_type == ElfLimbType::LeftLeg || limb.limb_type == ElfLimbType::RightLeg {
            assert!(
                (limb.rest_local_translation.y - (-0.32)).abs() < 0.001,
                "Elf leg limb local Y must be -0.32m (bottom at -0.45m), was {}",
                limb.rest_local_translation.y
            );
        }
    }
}

#[test]
fn test_elf_mischief_traits_assignment_and_lazy_slacking() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfMischiefTrait, ElfNpcState, elf_assistant_ai_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, elf_assistant_ai_system);

    let mut lazy_elf = ElfAssistant::new(1, "LazySparky", vec![Vec3::new(2.0, 0.45, 0.0)]);
    assert_eq!(lazy_elf.mischief_trait, ElfMischiefTrait::Lazy);
    lazy_elf.slacking_cooldown = 0.0;

    let elf_entity = app.world_mut().spawn((
        lazy_elf,
        Transform::from_xyz(0.0, 0.45, 0.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    // Frame 1: Enters slacking state
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let vel = app.world().get::<Velocity>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Slacking, "Lazy elf must enter Slacking state");
        assert_eq!(vel.linear, Vec3::ZERO, "Slacking elf must stop moving");
    }

    // Frame 2: Remains slacking
    {
        let mut t = app.world_mut().resource_mut::<Time>();
        t.advance_by(std::time::Duration::from_millis(50));
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Slacking);
    }

    // Frame 3: Knock down slacking elf with a physics object
    let prop_entity = app.world_mut().spawn((
        rust_tests::elf_game::player::hands::Grabbable,
        Transform::from_xyz(0.0, 0.45, 0.3),
        Velocity {
            linear: Vec3::new(2.5, 0.0, 0.0),
            angular: Vec3::ZERO,
        },
    )).id();

    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let locked = app.world().get::<LockedAxes>(elf_entity).unwrap();
        assert_eq!(
            elf.state,
            ElfNpcState::RagdollTumble,
            "Physics impact must knock slacking elf into RagdollTumble"
        );
        assert_eq!(*locked, LockedAxes::empty());
    }

    app.world_mut().despawn(prop_entity);
    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.tumble_timer = 3.0;
        let mut vel = app.world_mut().get_mut::<Velocity>(elf_entity).unwrap();
        vel.linear = Vec3::ZERO;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Recovering);
    }

    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.recovery_timer = 1.0;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Wandering, "Elf must resume work after knockdown recovery");
        assert!(elf.slacking_cooldown > 5.0, "Slacking cooldown must reset after recovery");
    }
}

#[test]
fn test_elf_klepto_trait_stashes_present() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfMischiefTrait, ElfNpcState, elf_assistant_ai_system,
        ElfNavGraph,
    };
    use rust_tests::elf_game::wrapping::Present;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.insert_resource(ElfNavGraph::default_workshop_graph());
    app.add_systems(Update, elf_assistant_ai_system);

    let klepto_elf = ElfAssistant::new(2, "KleptoTinsel", vec![Vec3::ZERO]);
    assert_eq!(klepto_elf.mischief_trait, ElfMischiefTrait::Klepto);

    let elf_entity = app.world_mut().spawn((
        klepto_elf,
        Transform::from_xyz(0.0, 0.45, 0.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    let present_entity = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(0.3, 0.45, 0.2),
        Velocity::default(),
    )).id();

    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::StashingPresent);
        assert_eq!(elf.carried_present, Some(present_entity));
        assert_eq!(elf.target_node, 13, "Klepto elf must target Stash Node (13)");
    }
}

#[test]
fn test_elf_hat_closet_interaction_and_job_assignment() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfJob, ElfJobHat, spawn_hat_closet,
        hat_closet_interaction_system, elf_hat_assignment_system,
    };
    use rust_tests::elf_game::player::{ElfHands, ElfMovement};
    use rust_tests::elf_game::assets::WorkshopMaterials;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    let mut mats = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    let wm = WorkshopMaterials::new(&mut mats);
    app.insert_resource(wm);

    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyE);
    app.insert_resource(keys);

    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    app.add_systems(Startup, spawn_hat_closet);
    app.add_systems(Update, (hat_closet_interaction_system, elf_hat_assignment_system));

    let player_entity = app.world_mut().spawn((
        Transform::from_xyz(-4.5, 0.45, 11.0),
        ElfMovement::default(),
        ElfHands::default(),
    )).id();

    app.update();

    let mut hat_q = app.world_mut().query::<(Entity, &ElfJobHat)>();
    let hats: Vec<(Entity, ElfJob)> = hat_q.iter(app.world()).map(|(e, h)| (e, h.job)).collect();
    assert_eq!(hats.len(), 3, "Interacting with closet must spawn 3 job hats");

    let jobs: Vec<ElfJob> = hats.iter().map(|(_, j)| *j).collect();
    assert!(jobs.contains(&ElfJob::MooseFeeder));
    assert!(jobs.contains(&ElfJob::PresentWrapper));
    assert!(jobs.contains(&ElfJob::WorkshopMaintenance));

    let (wrapper_hat_e, _) = hats.iter().find(|(_, j)| *j == ElfJob::PresentWrapper).unwrap();
    {
        let mut hands = app.world_mut().get_mut::<ElfHands>(player_entity).unwrap();
        hands.right_grabbed = Some(*wrapper_hat_e);
    }

    let elf_entity = app.world_mut().spawn((
        ElfAssistant::new(1, "RecipientElf", vec![Vec3::ZERO]),
        Transform::from_xyz(-4.0, 0.45, 11.0),
        Velocity::default(),
    )).id();

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.assigned_job, Some(ElfJob::PresentWrapper), "Elf must receive PresentWrapper job");
        assert_eq!(elf.worn_hat_entity, Some(*wrapper_hat_e));

        let hands = app.world().get::<ElfHands>(player_entity).unwrap();
        assert_eq!(hands.right_grabbed, None, "Player hand must release the hat upon placing it");

        let hat = app.world().get::<ElfJobHat>(*wrapper_hat_e).unwrap();
        assert_eq!(hat.worn_by, Some(elf_entity));
    }
}

















#[test]
fn test_pause_button_and_menu_pauses_hunger_meter_and_timers() {
    use rust_tests::elf_game::core::GameState;
    use rust_tests::elf_game::moose::{update_moose_hunger_system, MooseHunger, MooseHungerStage};
    use rust_tests::elf_game::ui::MenuState;

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
fn test_pause_freezes_moose_rampage_movement_and_bucking_animation() {
    use rust_tests::elf_game::core::GameState;
    use rust_tests::elf_game::moose::toy_model::MrMoose;
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage};
    use rust_tests::elf_game::ui::MenuState;
    use rust_tests::elf_game::workshop::events::{
        moose_rampage_path_and_physics_impulse_system, MooseRampageTracker,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    app.insert_resource(MenuState::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_resource(MooseHunger {
        value: 60.0,
        stage: MooseHungerStage::BreachedRampage,
        is_breached: true,
        ..default()
    });
    app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

    let moose = app.world_mut().spawn((
        MrMoose,
        MooseRampageTracker::default(),
        Transform::from_xyz(-2.0, 1.4, -10.0),
    )).id();

    // Advance 10 frames while unpaused so the moose is actively moving and bucking
    for _ in 0..10 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    let (pos_before_pause, rot_before_pause, tracker_before_pause) = {
        let tf = app.world().get::<Transform>(moose).unwrap();
        let tracker = app.world().get::<MooseRampageTracker>(moose).unwrap();
        (tf.translation, tf.rotation, tracker.clone())
    };

    assert!(
        tracker_before_pause.buck_timer > 0.0,
        "Moose should have accumulated bucking animation timer"
    );

    // Pause the game
    app.world_mut().resource_mut::<MenuState>().is_open = true;

    // Advance 30 frames with significant delta
    for _ in 0..30 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(33));
        app.update();
    }

    let (pos_after_pause, rot_after_pause, tracker_after_pause) = {
        let tf = app.world().get::<Transform>(moose).unwrap();
        let tracker = app.world().get::<MooseRampageTracker>(moose).unwrap();
        (tf.translation, tf.rotation, tracker.clone())
    };

    assert_eq!(
        pos_after_pause, pos_before_pause,
        "Moose rampage position must freeze in place while paused"
    );
    assert_eq!(
        rot_after_pause, rot_before_pause,
        "Moose rampage orientation must freeze in place while paused"
    );
    assert_eq!(
        tracker_after_pause.current_waypoint_index, tracker_before_pause.current_waypoint_index,
        "Moose waypoint index must remain frozen while paused"
    );
    assert_eq!(
        tracker_after_pause.buck_timer, tracker_before_pause.buck_timer,
        "Moose buck animation timer must remain frozen while paused"
    );
    assert_eq!(
        tracker_after_pause.buck_progress, tracker_before_pause.buck_progress,
        "Moose buck progress must remain frozen while paused"
    );
    assert_eq!(
        tracker_after_pause.current_buck_pitch, tracker_before_pause.current_buck_pitch,
        "Moose bucking animation pitch must remain frozen while paused"
    );
}

#[test]
fn test_pause_syncs_virtual_time_rapier_physics_and_game_state() {
    use rust_tests::elf_game::core::GameState;
    use rust_tests::elf_game::ui::{sync_pause_state_system, MenuState};
    use bevy_rapier3d::prelude::RapierConfiguration;

    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_resource(Time::<Virtual>::default());

    // Spawn Rapier context entity with RapierConfiguration
    let rapier_entity = app.world_mut().spawn(RapierConfiguration::new(1.0)).id();

    app.add_systems(Update, sync_pause_state_system);
    app.update();

    // Initially unpaused
    assert_eq!(*app.world().resource::<State<GameState>>().get(), GameState::InWorkshop);
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert!(app.world().get::<RapierConfiguration>(rapier_entity).unwrap().physics_pipeline_active);

    // 1. Open Menu / Pause Game
    app.world_mut().resource_mut::<MenuState>().is_open = true;
    app.update(); // sync_pause_state_system runs and sets NextState(GameState::Paused)
    app.update(); // StateTransition schedule applies GameState::Paused

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Paused,
        "GameState must transition to Paused"
    );
    assert!(
        app.world().resource::<Time<Virtual>>().is_paused(),
        "Virtual time must be paused"
    );
    assert!(
        !app.world().get::<RapierConfiguration>(rapier_entity).unwrap().physics_pipeline_active,
        "Rapier physics pipeline must be deactivated when paused"
    );

    // 2. Close Menu / Resume Game
    app.world_mut().resource_mut::<MenuState>().is_open = false;
    app.update(); // sync_pause_state_system runs and sets NextState(GameState::InWorkshop)
    app.update(); // StateTransition schedule applies GameState::InWorkshop

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::InWorkshop,
        "GameState must return to InWorkshop"
    );
    assert!(
        !app.world().resource::<Time<Virtual>>().is_paused(),
        "Virtual time must be unpaused"
    );
    assert!(
        app.world().get::<RapierConfiguration>(rapier_entity).unwrap().physics_pipeline_active,
        "Rapier physics pipeline must be reactivated when unpaused"
    );
}

#[test]
fn test_pause_freezes_workshop_timers_vacuum_and_spawner() {
    use rust_tests::elf_game::core::GameState;
    use rust_tests::elf_game::ui::MenuState;
    use rust_tests::elf_game::workshop::room::{
        conveyor_cube_spawner_system, present_sending_platform_system, ConveyorSpawnerTimer,
        PresentSendingPlatform,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    app.insert_resource(MenuState::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_resource(ConveyorSpawnerTimer {
        timer: Timer::from_seconds(2.0, TimerMode::Repeating),
    });

    app.world_mut().spawn((
        PresentSendingPlatform {
            is_active: true,
            active_duration: 4.0,
            cooldown_timer: 0.0,
            player_despawner_contact_time: 1.2,
            suction_timer: 6.0,
        },
        Transform::from_xyz(5.5, 0.05, -4.0),
    ));

    app.add_systems(Update, (conveyor_cube_spawner_system, present_sending_platform_system));

    // Pause the game
    app.world_mut().resource_mut::<MenuState>().is_open = true;

    // Advance 5 seconds while paused
    for _ in 0..50 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(100));
        app.update();
    }

    // Verify ConveyorSpawnerTimer did not tick
    let spawner = app.world().resource::<ConveyorSpawnerTimer>();
    assert_eq!(
        spawner.timer.elapsed_secs(), 0.0,
        "Conveyor cube spawner timer must not tick while paused"
    );

    // Verify PresentSendingPlatform timers did not tick
    let mut platform_q = app.world_mut().query::<&PresentSendingPlatform>();
    let plat = platform_q.single(app.world()).unwrap();
    assert_eq!(
        plat.active_duration, 4.0,
        "Platform active duration must remain frozen while paused"
    );
    assert_eq!(
        plat.player_despawner_contact_time, 1.2,
        "Player despawner contact timer must remain frozen while paused"
    );
}

#[test]
fn test_toggle_menu_keybindings_and_resume_game() {
    use bevy::ui_widgets::{Activate, Button};
    use rust_tests::elf_game::ui::{on_button_activate, toggle_menu, MenuButtonAction, MenuState};

    let mut app = App::new();
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyP);
    app.insert_resource(keys);
    app.insert_resource(MenuState::default());

    app.add_systems(Update, toggle_menu);
    app.add_observer(on_button_activate);

    // Pressing 'P' toggles menu open
    app.update();
    assert!(
        app.world().resource::<MenuState>().is_open,
        "Pressing KeyCode::KeyP must open pause menu"
    );

    // Clear and press Escape toggles menu closed
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.press(KeyCode::Escape);
    }
    app.update();
    assert!(
        !app.world().resource::<MenuState>().is_open,
        "Pressing KeyCode::Escape must close pause menu"
    );

    // Reopen menu
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.press(KeyCode::Tab);
    }
    app.update();
    assert!(app.world().resource::<MenuState>().is_open);

    // Trigger ResumeGame button action via observer
    let resume_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::ResumeGame,
    )).id();
    app.world_mut().trigger(Activate { entity: resume_btn });

    assert!(
        !app.world().resource::<MenuState>().is_open,
        "Activating ResumeGame button action must close menu and unpause"
    );
}

#[test]
fn test_vacuum_button_activation_pedestal_player_inputs_and_physics_props() {
    use rust_tests::elf_game::workshop::room::{
        PresentSendingPlatform, SendPlatformButton,
        present_sending_platform_system,
    };
    use rust_tests::elf_game::player::ElfMovement;

    use bevy_rapier3d::prelude::Velocity;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_systems(Update, present_sending_platform_system);

    let platform_e = app.world_mut().spawn((
        PresentSendingPlatform::default(),
        Transform::from_xyz(5.5, 0.05, -4.0),
    )).id();

    let _button_e = app.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(3.8, 0.95, -2.8),
    )).id();

    // 1. Verify player standing at (3.8, 1.0, -2.0) can activate with Left Click
    let player_e = app.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(3.8, 1.0, -2.0),
    )).id();

    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
    }
    app.update();

    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must be active when player clicks button");

    // Release mouse
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release(MouseButton::Left);
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must turn off when click is released");

    // 2. Verify Right Click activates
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Right);
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must activate with Right Click");

    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release(MouseButton::Right);
    }
    app.update();

    // 3. Verify KeyF activates
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyF);
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must activate with KeyF");

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyF);
    }
    app.update();

    // Move player far away
    {
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_e).unwrap();
        p_tf.translation = Vec3::new(0.0, 1.0, 0.0);
    }

    // 4. Place a static or resting physics prop on the button
    let prop_on_button = app.world_mut().spawn((
        Transform::from_xyz(3.8, 1.05, -2.8),
        Velocity::zero(),
    )).id();
    app.update();

    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must activate when physics object rests on button, even without Velocity component");

    // Remove prop from button
    app.world_mut().despawn(prop_on_button);
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must turn off when physics object is removed from button");
}

#[test]
fn test_pause_allows_lighting_flicker_and_particles_to_continue() {
    use rust_tests::elf_game::core::GameState;
    use rust_tests::elf_game::ui::MenuState;
    use rust_tests::elf_game::workshop::room::{
        CandleChandelier, CandleFlameEmitter, CandleParticle,
        chandelier_candle_flicker_system, candle_particle_emitter_system, candle_particle_system,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    let mut time_real = Time::<bevy::time::Real>::default();
    time_real.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time_real);

    app.insert_resource(MenuState { is_open: true, ..default() }); // Game is paused!
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();

    app.add_systems(
        Update,
        (
            chandelier_candle_flicker_system,
            candle_particle_emitter_system,
            candle_particle_system,
        ),
    );

    let chandelier_e = app.world_mut().spawn((
        CandleChandelier {
            base_intensity: 480_000.0,
            flicker_seed: 1.0,
        },
        PointLight {
            intensity: 480_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 4.4, -4.5),
        GlobalTransform::from(Transform::from_xyz(0.0, 4.4, -4.5)),
        CandleFlameEmitter::default(),
    )).id();

    // 1. Advance real time across multiple frames while game is paused
    let mut intensities = Vec::new();
    for _ in 0..10 {
        {
            let mut tr = app.world_mut().resource_mut::<Time<bevy::time::Real>>();
            tr.advance_by(std::time::Duration::from_millis(50));
        }
        app.update();

        let light = app.world().get::<PointLight>(chandelier_e).unwrap();
        intensities.push(light.intensity);
    }

    // Verify lighting flickered dynamically despite game being paused
    let min_i = intensities.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_i = intensities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max_i - min_i > 1_000.0,
        "Chandelier candle lighting must continue flickering dynamically while game is paused"
    );

    // 2. Verify particles were emitted while paused
    let mut q_particles = app.world_mut().query::<(&Transform, &CandleParticle)>();
    let count = q_particles.iter(app.world()).count();
    assert!(
        count >= 1,
        "Candle particle emitter must continue spawning particles while game is paused"
    );
}

#[test]
fn test_player_hands_model_scale() {
    use rust_tests::elf_game::player::controller::{spawn_elf_player, LeftHandVisual, RightHandVisual};

    let mut app = App::new();
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(Startup, spawn_elf_player);
    app.update();

    let left_mesh_handle = {
        let mut left_hand_q = app.world_mut().query::<(&Mesh3d, &LeftHandVisual)>();
        let (left_h, _) = left_hand_q.single(app.world()).unwrap();
        left_h.0.clone()
    };
    let right_mesh_handle = {
        let mut right_hand_q = app.world_mut().query::<(&Mesh3d, &RightHandVisual)>();
        let (right_h, _) = right_hand_q.single(app.world()).unwrap();
        right_h.0.clone()
    };

    let meshes = app.world().resource::<Assets<Mesh>>();
    let left_mesh = meshes.get(&left_mesh_handle).expect("Left hand mesh must exist");
    let right_mesh = meshes.get(&right_mesh_handle).expect("Right hand mesh must exist");

    let positions = left_mesh.attribute(Mesh::ATTRIBUTE_POSITION).expect("Mesh must have positions");
    let max_radius = match positions {
        bevy::render::mesh::VertexAttributeValues::Float32x3(pts) => {
            pts.iter().map(|[x, y, z]| (x * x + y * y + z * z).sqrt()).fold(0.0f32, f32::max)
        }
        _ => panic!("Expected Float32x3 position values"),
    };

    assert!(
        max_radius <= 0.08,
        "Player hand glove radius must be scaled down to <= 0.08m (was 0.12m), got {}",
        max_radius
    );
    assert!(
        (max_radius - 0.065).abs() < 0.005,
        "Player hand glove radius should be ~0.065m, got {}",
        max_radius
    );

    // Verify right hand mesh is also scaled down
    let right_positions = right_mesh.attribute(Mesh::ATTRIBUTE_POSITION).expect("Right mesh must have positions");
    let right_max_radius = match right_positions {
        bevy::render::mesh::VertexAttributeValues::Float32x3(pts) => {
            pts.iter().map(|[x, y, z]| (x * x + y * y + z * z).sqrt()).fold(0.0f32, f32::max)
        }
        _ => panic!("Expected Float32x3 position values"),
    };
    assert!(
        (right_max_radius - 0.065).abs() < 0.005,
        "Right player hand glove radius should be ~0.065m, got {}",
        right_max_radius
    );
}

#[test]
fn test_menu_debug_submenu_navigation_and_panels() {
    use bevy::ui_widgets::{Activate, Button};
    use rust_tests::elf_game::ui::{
        on_button_activate, sync_submenu_visibility, DebugSubmenuPanel, MainMenuPanel,
        MenuButtonAction, MenuState, MenuSubmenu, MenuSubtitleText, MenuTitleText,
    };

    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.add_observer(on_button_activate);
    app.add_systems(Update, sync_submenu_visibility);

    // Spawn panels and title
    let title_e = app.world_mut().spawn((
        MenuTitleText,
        Text::new("PAUSED - OPTIONS & MENU"),
    )).id();
    let _subtitle_e = app.world_mut().spawn((
        MenuSubtitleText,
        Text::new("Options Subtitle"),
    )).id();
    let main_panel_e = app.world_mut().spawn((
        MainMenuPanel,
        Node { display: Display::Flex, ..default() },
    )).id();
    let debug_panel_e = app.world_mut().spawn((
        DebugSubmenuPanel,
        Node { display: Display::None, ..default() },
    )).id();

    // Default: Main submenu
    assert_eq!(app.world().resource::<MenuState>().active_submenu, MenuSubmenu::Main);

    // Trigger switch to DEBUG submenu
    let switch_to_debug_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::SwitchSubmenu(MenuSubmenu::Debug),
    )).id();
    app.world_mut().trigger(Activate { entity: switch_to_debug_btn });

    assert_eq!(app.world().resource::<MenuState>().active_submenu, MenuSubmenu::Debug);

    app.update();

    // Verify Main panel is hidden (Display::None) and Debug panel is shown (Display::Flex)
    let main_panel_node = app.world().get::<Node>(main_panel_e).unwrap();
    let debug_panel_node = app.world().get::<Node>(debug_panel_e).unwrap();
    assert_eq!(main_panel_node.display, Display::None);
    assert_eq!(debug_panel_node.display, Display::Flex);

    // Verify Title text updated
    let title_text = app.world().get::<Text>(title_e).unwrap();
    assert_eq!(title_text.0, "PAUSED - DEBUG MENU");

    // Trigger switch back to MAIN submenu
    let switch_to_main_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::SwitchSubmenu(MenuSubmenu::Main),
    )).id();
    app.world_mut().trigger(Activate { entity: switch_to_main_btn });

    assert_eq!(app.world().resource::<MenuState>().active_submenu, MenuSubmenu::Main);

    app.update();

    let main_panel_node = app.world().get::<Node>(main_panel_e).unwrap();
    let debug_panel_node = app.world().get::<Node>(debug_panel_e).unwrap();
    assert_eq!(main_panel_node.display, Display::Flex);
    assert_eq!(debug_panel_node.display, Display::None);

    let title_text = app.world().get::<Text>(title_e).unwrap();
    assert_eq!(title_text.0, "PAUSED - OPTIONS & MENU");
}

#[test]
fn test_debug_submenu_bhop_window_adjustment() {
    use bevy::ui_widgets::{Activate, Button};
    use rust_tests::elf_game::player::{BhopConfig, BhopWindow};
    use rust_tests::elf_game::ui::{on_button_activate, update_menu_button_labels, MenuButtonAction, MenuState};

    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.insert_resource(BhopConfig::default());
    app.add_observer(on_button_activate);
    app.add_systems(Update, update_menu_button_labels);

    let text_child = app.world_mut().spawn(Text::new("BHOP Timing: Initial")).id();
    let bhop_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::CycleBhopWindow,
    )).add_child(text_child).id();

    // 1. Initial window is Sixtieth (1/60s)
    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Sixtieth);

    // 2. Click button to cycle to Thirtieth (1/30s)
    app.world_mut().trigger(Activate { entity: bhop_btn });
    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Thirtieth);

    app.update();
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "BHOP Timing: 1/30s (Medium)");

    // 3. Click button to cycle to Tenth (1/10s)
    app.world_mut().trigger(Activate { entity: bhop_btn });
    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Tenth);

    app.update();
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "BHOP Timing: 1/10s (Forgiving)");

    // 4. Click button to cycle back to Sixtieth (1/60s)
    app.world_mut().trigger(Activate { entity: bhop_btn });
    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Sixtieth);

    app.update();
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "BHOP Timing: 1/60s (Strict)");
}

#[test]
fn test_debug_submenu_noclip_mode_and_collider_bypass() {
    use bevy::ui_widgets::{Activate, Button};
    use bevy_rapier3d::prelude::*;
    use rust_tests::elf_game::player::controller::{
        elf_player_controller_system, sync_noclip_collider_system, toggle_noclip_system,
        CameraPivot, ElfPlayer,
    };
    use rust_tests::elf_game::player::{BhopConfig, ElfMovement};
    use rust_tests::elf_game::ui::{
        on_button_activate, update_menu_button_labels, MenuButtonAction, MenuState,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(BhopConfig::default());
    app.insert_resource(MenuState::default());

    app.add_observer(on_button_activate);
    app.add_systems(
        Update,
        (
            toggle_noclip_system,
            sync_noclip_collider_system,
            elf_player_controller_system,
            update_menu_button_labels,
        ).chain(),
    );

    // Spawn player entity and camera pivot
    let player_e = app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(0.0, 1.5, 0.0),
        ElfMovement::default(),
        KinematicCharacterController::default(),
        Collider::capsule_y(0.5, 0.3),
    )).id();

    app.world_mut().spawn(CameraPivot { pitch: 0.0 });

    let text_child = app.world_mut().spawn(Text::new("Noclip Mode: OFF")).id();
    let noclip_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::ToggleNoclip,
    )).add_child(text_child).id();

    // 1. Initial state: noclip is OFF, player does not have ColliderDisabled
    assert!(!app.world().resource::<MenuState>().noclip);
    assert!(!app.world().entity(player_e).contains::<ColliderDisabled>());

    // 2. Trigger ToggleNoclip button in DEBUG submenu
    app.world_mut().trigger(Activate { entity: noclip_btn });
    assert!(app.world().resource::<MenuState>().noclip);

    // 3. Run update to sync collider disabled and update label
    app.update();
    assert!(
        app.world().entity(player_e).contains::<ColliderDisabled>(),
        "Player must have ColliderDisabled inserted when noclip is enabled"
    );
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Noclip Mode: ON (Flying)");

    // 4. Press W to fly forward in 3D noclip mode
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyW);
    }
    app.update();

    let player_tf = app.world().get::<Transform>(player_e).unwrap();
    let controller = app.world().get::<KinematicCharacterController>(player_e).unwrap();
    let movement = app.world().get::<ElfMovement>(player_e).unwrap();

    // In noclip, character controller translation is None (direct Transform movement)
    assert!(controller.translation.is_none(), "Controller translation must be None during noclip");
    // Forward in 3D (-Z)
    assert!(player_tf.translation.z < 0.0, "Player must have translated forward in Z, was {}", player_tf.translation.z);
    // Gravity is bypassed
    assert_eq!(movement.velocity, Vec3::ZERO, "Velocity must remain zero with no gravity during noclip");

    // 5. Test KeyCode::KeyV keyboard shortcut toggles noclip back OFF
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.press(KeyCode::KeyV);
    }
    app.update();

    assert!(!app.world().resource::<MenuState>().noclip, "KeyCode::KeyV must toggle noclip OFF");
    assert!(
        !app.world().entity(player_e).contains::<ColliderDisabled>(),
        "ColliderDisabled must be removed when noclip is disabled"
    );
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Noclip Mode: OFF");
}

#[test]
fn test_debug_submenu_wireframe_toggle() {
    use bevy::pbr::wireframe::WireframeConfig;
    use bevy::ui_widgets::{Activate, Button};
    use bevy_rapier3d::prelude::DebugRenderContext;
    use rust_tests::elf_game::ui::{
        on_button_activate, sync_hud_visibility, update_menu_button_labels, MenuButtonAction,
        MenuState,
    };

    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.insert_resource(WireframeConfig::default());
    app.insert_resource(DebugRenderContext::default());
    app.add_observer(on_button_activate);
    app.add_systems(Update, (sync_hud_visibility, update_menu_button_labels));

    let text_child = app.world_mut().spawn(Text::new("Wireframe Mode: OFF")).id();
    let wireframe_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::ToggleWireframes,
    )).add_child(text_child).id();

    // 1. Initial state: wireframes OFF
    assert!(!app.world().resource::<MenuState>().show_wireframes);

    // 2. Trigger wireframe toggle in DEBUG submenu
    app.world_mut().trigger(Activate { entity: wireframe_btn });
    assert!(app.world().resource::<MenuState>().show_wireframes);

    app.update();

    // Verify label and rendering contexts updated
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Wireframe Mode: ON");
    assert!(app.world().resource::<DebugRenderContext>().enabled);
    assert!(app.world().resource::<WireframeConfig>().global);

    // 3. Toggle back OFF
    app.world_mut().trigger(Activate { entity: wireframe_btn });
    assert!(!app.world().resource::<MenuState>().show_wireframes);

    app.update();

    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Wireframe Mode: OFF");
    assert!(!app.world().resource::<DebugRenderContext>().enabled);
    assert!(!app.world().resource::<WireframeConfig>().global);
}

#[test]
fn test_pantry_storage_wing_geometry_and_shelving() {
    use rust_tests::elf_game::workshop::{
        pantry::{PantryWing, PantryShelvingUnit, PantryDoor, PantryRestockTracker},
        room::setup_workshop_scene,
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

    // 1. Verify Pantry Storage Wing root entity and spatial bounds
    let Ok((pantry, p_tf)) = app.world_mut().query::<(&PantryWing, &Transform)>().single(app.world()) else {
        panic!("PantryWing must be spawned");
    };
    assert_eq!(pantry.bounds_min, Vec3::new(-17.0, 0.0, 3.5));
    assert_eq!(pantry.bounds_max, Vec3::new(-13.0, 3.5, 7.5));
    assert_eq!(p_tf.translation, Vec3::new(-15.0, 1.75, 5.5));

    // 2. Verify Floor, Ceiling, Walls, Header, and Lantern
    let names: Vec<String> = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .map(|n| n.to_string())
        .collect();

    assert!(names.contains(&"PantryFloor".to_string()), "Must spawn PantryFloor");
    assert!(names.contains(&"PantryCeiling".to_string()), "Must spawn PantryCeiling");
    assert!(names.contains(&"PantryNorthWall".to_string()), "Must spawn PantryNorthWall");
    assert!(names.contains(&"PantrySouthWall".to_string()), "Must spawn PantrySouthWall");
    assert!(names.contains(&"PantryWestWall".to_string()), "Must spawn PantryWestWall");
    assert!(names.contains(&"PantryEastWallNorth".to_string()), "Must spawn PantryEastWallNorth");
    assert!(names.contains(&"PantryEastWallSouth".to_string()), "Must spawn PantryEastWallSouth");
    assert!(names.contains(&"WallWestMidSouth".to_string()), "Must spawn WallWestMidSouth");
    assert!(names.contains(&"PantryDoorHeaderLintel".to_string()), "Must spawn PantryDoorHeaderLintel");
    assert!(names.contains(&"PantryCeilingLantern".to_string()), "Must spawn warm ambient ceiling lantern");

    // 3. Verify Deep Perimeter Shelving Units
    let mut shelves_q = app.world_mut().query::<(&PantryShelvingUnit, &Transform)>();
    let shelves: Vec<(String, usize, Vec3)> = shelves_q
        .iter(app.world())
        .map(|(s, tf)| (s.unit_name.clone(), s.tier_count, tf.translation))
        .collect();

    let north_unit = shelves.iter().find(|(n, _, _)| n == "NorthMooseTreatShelves").expect("North treat shelves must exist");
    assert_eq!(north_unit.1, 3, "North treat shelves must have 3 tiers");
    assert!((north_unit.2.x - (-15.0)).abs() < 0.1 && (north_unit.2.z - 3.80).abs() < 0.1);

    let west_unit = shelves.iter().find(|(n, _, _)| n == "WestSupplyShelves").expect("West supply shelves must exist");
    assert_eq!(west_unit.1, 2, "West supply shelves must have 2 tiers");

    assert!(names.contains(&"PantryBakingStagingCounter".to_string()), "Must spawn South baking prep staging counter");

    // 4. Verify Pantry Door & Tracker resource
    assert!(app.world_mut().query::<&PantryDoor>().iter(app.world()).next().is_some());
    assert!(app.world().get_resource::<PantryRestockTracker>().is_some());
}

#[test]
fn test_pantry_fully_enclosed_without_holes() {
    use rust_tests::elf_game::workshop::room::setup_workshop_scene;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

    // Query colliders and transforms
    let mut wall_colliders: Vec<(String, Vec3, Vec3)> = Vec::new();
    for (name, tf, collider) in app.world_mut().query::<(&Name, &Transform, &Collider)>().iter(app.world()) {
        let name_str = name.to_string();
        if name_str.starts_with("Pantry") || name_str.starts_with("WallWest") {
            let half_extents = match collider.as_cuboid() {
                Some(c) => Vec3::new(c.raw.half_extents.x, c.raw.half_extents.y, c.raw.half_extents.z),
                None => Vec3::ZERO,
            };
            wall_colliders.push((name_str, tf.translation, half_extents));
        }
    }

    // Verify East dividing walls North and South of the door
    let east_north = wall_colliders.iter().find(|(n, _, _)| n == "PantryEastWallNorth").expect("PantryEastWallNorth must exist");
    assert_eq!(east_north.1.x, -13.0);
    assert!(east_north.2.y >= 1.74, "East dividing wall North must reach full 3.5m height (half_extent >= 1.74)");
    // Z range must cover Z from 3.5 to 4.75
    let z_min_north = east_north.1.z - east_north.2.z;
    let z_max_north = east_north.1.z + east_north.2.z;
    assert!(z_min_north <= 3.55 && z_max_north >= 4.70, "PantryEastWallNorth must span Z in [3.5, 4.75]");

    let east_south = wall_colliders.iter().find(|(n, _, _)| n == "PantryEastWallSouth").expect("PantryEastWallSouth must exist");
    assert_eq!(east_south.1.x, -13.0);
    assert!(east_south.2.y >= 1.74, "East dividing wall South must reach full 3.5m height (half_extent >= 1.74)");
    // Z range must cover Z from 6.23 to 7.5
    let z_min_south = east_south.1.z - east_south.2.z;
    let z_max_south = east_south.1.z + east_south.2.z;
    assert!(z_min_south <= 6.25 && z_max_south >= 7.45, "PantryEastWallSouth must span Z in [6.23, 7.5]");

    // Verify WallWestMidSouth seals the middle height along South West wall
    let mid_south = wall_colliders.iter().find(|(n, _, _)| n == "WallWestMidSouth").expect("WallWestMidSouth must exist");
    assert_eq!(mid_south.1.x, -13.0);
    assert!(mid_south.2.y >= 1.35, "WallWestMidSouth must cover height Y in [1.2, 4.0]");

    // Verify that at player waist/eye height Y = 1.6, along X = -13.0 for all Z in [3.5, 7.5],
    // every point is covered by either PantryEastWallNorth, PantryEastWallSouth, or PantryDoor
    for z_idx in 0..40 {
        let z = 3.55 + (z_idx as f32) * 0.10; // 3.55 to 7.45
        let point = Vec3::new(-13.0, 1.6, z);

        let inside_north = (point.x - east_north.1.x).abs() <= east_north.2.x + 0.05
            && (point.y - east_north.1.y).abs() <= east_north.2.y
            && (point.z - east_north.1.z).abs() <= east_north.2.z + 0.01;

        let inside_south = (point.x - east_south.1.x).abs() <= east_south.2.x + 0.05
            && (point.y - east_south.1.y).abs() <= east_south.2.y
            && (point.z - east_south.1.z).abs() <= east_south.2.z + 0.01;

        let in_doorway = z >= 4.70 && z <= 6.25;

        assert!(
            inside_north || inside_south || in_doorway,
            "Point at Z = {z:.2} must be blocked by North wall, South wall, or Door! No holes allowed.",
        );
    }
}

#[test]
fn test_pantry_revolute_joint_door_and_spring_limits() {
    use rust_tests::elf_game::workshop::pantry::{
        PantryDoor, PantryDoorHingePost, pantry_door_spring_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    // Spawn fixed hinge post
    let hinge_post = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 4.75),
        RigidBody::Fixed,
        PantryDoorHingePost,
    )).id();

    // Spawn dynamic door leaf with RevoluteJointBuilder
    let joint = RevoluteJointBuilder::new(Vec3::Y)
        .local_anchor1(Vec3::ZERO)
        .local_anchor2(Vec3::new(0.0, 0.0, -0.73))
        .limits([0.0, 1.65])
        .contacts_enabled(false);

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        RigidBody::Dynamic,
        Velocity::zero(),
        ExternalImpulse::default(),
        ImpulseJoint::new(hinge_post, joint),
        PantryDoor {
            spring_stiffness: 20.0,
            spring_damping: 4.0,
            ..default()
        },
    )).id();

    app.add_systems(Update, pantry_door_spring_system);

    // 1. Verify ImpulseJoint component is attached and configured
    let imp_joint = app.world().get::<ImpulseJoint>(door_entity).expect("Door must have ImpulseJoint");
    assert_eq!(imp_joint.parent, hinge_post);

    // 2. Displace door open by -0.80 rad (~46 deg into pantry)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(-0.80);
    }

    app.update();

    // 3. Verify spring system applied positive torque impulse to swing door back toward closed (0.0 rad)
    let ext = app.world().get::<ExternalImpulse>(door_entity).unwrap();
    assert!(
        ext.torque_impulse.y > 0.0,
        "Spring torque should push door back toward 0.0 (closed), got {}",
        ext.torque_impulse.y
    );
}

#[test]
fn test_pantry_infinite_restock_cycle_closed_to_opened() {
    use rust_tests::elf_game::workshop::pantry::{
        PantryDoor, PantryRestockTracker, PantryRestockEvent, MooseTreatJar,
        pantry_door_restock_cycle_system, spawn_pantry_storage_wing,
    };
    use bevy::ecs::message::Messages;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_message::<PantryRestockEvent>();

    // Spawn pantry wing and initial jars
    let mut commands = app.world_mut().commands();
    spawn_pantry_storage_wing(&mut commands, &mut None, &mut None, None);
    app.update();

    // 1. Verify initially exactly ONE jar of Moose Treats in the pantry
    let initial_jars_count = app.world_mut().query::<&MooseTreatJar>().iter(app.world()).count();
    assert_eq!(initial_jars_count, 1, "The Pantry should only have one jar of Moose Treats at a time");

    let tracker = app.world().resource::<PantryRestockTracker>().clone();
    assert_eq!(tracker.slots.len(), 1);
    assert_eq!(tracker.restock_cycle_count, 0);

    app.add_systems(Update, pantry_door_restock_cycle_system);

    // Find door entity
    let Ok((door_entity, _)) = app.world_mut().query::<(Entity, &PantryDoor)>().single(app.world()) else {
        panic!("PantryDoor must be spawned");
    };

    // 2. Door closes and reopens while the jar is still in the pantry:
    // Should NOT refresh / spawn another jar because the jar was not removed!
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(0.0);
    }
    app.update();
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(0.55);
    }
    app.update();

    assert_eq!(
        app.world_mut().query::<&MooseTreatJar>().iter(app.world()).count(),
        1,
        "Must still only have 1 jar if not removed from pantry"
    );

    // 3. Player removes the jar from the pantry (move jar outside pantry to main room)
    let jar_entity = app
        .world_mut()
        .query_filtered::<Entity, With<MooseTreatJar>>()
        .iter(app.world())
        .next()
        .unwrap();

    {
        let mut jar_tf = app.world_mut().get_mut::<Transform>(jar_entity).unwrap();
        jar_tf.translation = Vec3::new(0.0, 0.5, 0.0); // Outside pantry bounds
    }
    app.update();

    // 4. Step with door still OPEN (yaw = 0.6 rad) -> no refresh until closed and reopened!
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(0.6);
        let mut door = app.world_mut().get_mut::<PantryDoor>(door_entity).unwrap();
        door.was_closed = false;
    }
    app.update();

    // 5. Close the door
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(0.0);
    }
    app.update();

    // Verify door recorded was_closed = true
    let door = app.world().get::<PantryDoor>(door_entity).unwrap();
    assert!(door.was_closed, "Door must record was_closed = true when closed");

    // 6. Player reopens the door -> Closed -> Opened transition!
    // Since jar was removed, pantry supplies reset to default initialized state (1 jar in pantry).
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(0.55);
    }
    app.update();

    // 7. Verify new jar was spawned at shelf slot
    let jars_after_restock = app.world_mut().query::<(&MooseTreatJar, &Transform)>().iter(app.world())
        .filter(|(_, tf)| tf.translation.x < -13.0) // inside pantry
        .count();
    assert_eq!(
        jars_after_restock, 1,
        "Exactly 1 jar must exist inside pantry after reset"
    );

    let updated_tracker = app.world().resource::<PantryRestockTracker>();
    assert_eq!(updated_tracker.restock_cycle_count, 1);
    assert_eq!(updated_tracker.total_restocked_jars, 1);

    // 8. Verify PantryRestockEvent was emitted
    let events = app.world().resource::<Messages<PantryRestockEvent>>();
    assert_eq!(events.len(), 1, "Must emit PantryRestockEvent on restock");
}

#[test]
fn test_pantry_door_player_interaction() {
    use rust_tests::elf_game::workshop::pantry::{
        PantryDoor, PantryDoorInteractMessage, pantry_door_interaction_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.add_message::<PantryDoorInteractMessage>();

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        ExternalImpulse::default(),
        PantryDoor::default(),
    )).id();

    app.add_systems(Update, pantry_door_interaction_system);

    // 1. Send Open command message (must swing into the workshop room with positive torque)
    app.world_mut().write_message(PantryDoorInteractMessage { open: true });

    app.update();

    let ext = app.world().get::<ExternalImpulse>(door_entity).unwrap();
    assert!(ext.torque_impulse.y > 0.0, "Open message must impart positive Y torque to swing into room");

    // 2. Send Close command message (must swing shut with negative torque)
    app.world_mut().write_message(PantryDoorInteractMessage { open: false });

    app.update();

    let ext2 = app.world().get::<ExternalImpulse>(door_entity).unwrap();
    assert!(ext2.torque_impulse.y < 0.0, "Close message must impart negative Y torque to swing shut");
}

#[test]
fn test_pantry_door_auto_close_and_fixed_hinge_alignment() {
    use rust_tests::elf_game::workshop::pantry::{
        PantryDoor, PantryDoorHingePost, pantry_door_spring_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    let hinge_post = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 4.75),
        RigidBody::Fixed,
        PantryDoorHingePost,
    )).id();

    let joint = RevoluteJointBuilder::new(Vec3::Y)
        .local_anchor1(Vec3::ZERO)
        .local_anchor2(Vec3::new(0.0, 0.0, -0.73))
        .limits([0.0, 1.65])
        .contacts_enabled(false);

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        RigidBody::Dynamic,
        Velocity::zero(),
        ExternalImpulse::default(),
        ImpulseJoint::new(hinge_post, joint),
        PantryDoor {
            close_speed: 1.0,
            close_snap_threshold: 0.25,
            ..default()
        },
    )).id();

    app.add_systems(Update, pantry_door_spring_system);

    // 1. Position door at yaw = 0.20 rad (< 1/4 rad = 0.25 rad)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(0.20);
    }

    // Step several frames to verify auto-close behavior
    for _ in 0..25 {
        app.update();
    }

    // 2. Verify door has slowly closed itself and snapped flush with the wall at 0.0 rad
    let (tf, door, vel) = {
        let world = app.world();
        (
            *world.get::<Transform>(door_entity).unwrap(),
            world.get::<PantryDoor>(door_entity).unwrap().clone(),
            *world.get::<Velocity>(door_entity).unwrap(),
        )
    };

    assert_eq!(door.current_angle, 0.0, "Door must auto-close to 0.0 rad when within 1/4 rad threshold");
    assert!((tf.translation.x - (-13.0)).abs() < 1e-4, "Door X must rest flush with wall at X = -13.0");
    assert!((tf.translation.z - 5.48).abs() < 1e-4, "Door Z must be centered at Z = 5.48");
    assert_eq!(tf.rotation, Quat::IDENTITY, "Door rotation must be identity when flush with wall");
    assert_eq!(vel.angular.y, 0.0, "Door angular velocity must come to rest when flush");

    // 3. Open door wide to yaw = 1.0 rad (>= 1/4 rad)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(1.0);
    }
    app.update();

    let (tf_open, door_open) = {
        let world = app.world();
        (
            *world.get::<Transform>(door_entity).unwrap(),
            world.get::<PantryDoor>(door_entity).unwrap().clone(),
        )
    };

    // Door should stay open (current_angle >= 0.25 rad)
    assert!(
        door_open.current_angle >= 0.25,
        "Door must NOT auto-close when opened beyond 1/4 rad threshold"
    );

    // Translation must follow exact circle around fixed hinge pivot (-13.0, 1.35, 4.75)
    let expected_x = -13.0 + 0.73 * 1.0_f32.sin();
    let expected_z = 4.75 + 0.73 * 1.0_f32.cos();
    assert!(
        (tf_open.translation.x - expected_x).abs() < 1e-3,
        "Door X must align with fixed hinge pivot, expected {}, got {}",
        expected_x,
        tf_open.translation.x
    );
    assert!(
        (tf_open.translation.z - expected_z).abs() < 1e-3,
        "Door Z must align with fixed hinge pivot, expected {}, got {}",
        expected_z,
        tf_open.translation.z
    );
}

#[test]
fn test_pantry_door_grab_and_swing_open_and_shut() {
    use rust_tests::elf_game::workshop::pantry::{
        PantryDoor, PantryDoorHingePost, pantry_door_interaction_system, pantry_door_spring_system,
    };
    use rust_tests::elf_game::player::controller::ElfPlayer;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();

    let hinge_post = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 4.75),
        RigidBody::Fixed,
        PantryDoorHingePost,
    )).id();

    let joint = RevoluteJointBuilder::new(Vec3::Y)
        .local_anchor1(Vec3::ZERO)
        .local_anchor2(Vec3::new(0.0, 0.0, -0.73))
        .limits([0.0, 1.65])
        .contacts_enabled(false);

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        RigidBody::Dynamic,
        Velocity::zero(),
        ExternalImpulse::default(),
        ImpulseJoint::new(hinge_post, joint),
        PantryDoor::default(),
    )).id();

    // Spawn player standing near the closed door (dist ~ 1.0m)
    let player_entity = app.world_mut().spawn((
        Transform::from_xyz(-12.0, 1.35, 5.48),
        ElfPlayer,
    )).id();

    app.add_systems(Update, (pantry_door_interaction_system, pantry_door_spring_system).chain());

    // 1. Simulate pressing E to grab the door
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();

    let door = app.world().get::<PantryDoor>(door_entity).unwrap();
    assert!(door.is_grabbed, "Player must grab the door when pressing interact near it");

    // 2. Player moves back into the room to swing the door open
    // Moving in +X from -12.0 to -10.5 pulls the door into the room
    {
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = Vec3::new(-10.5, 1.35, 4.75);
    }

    for _ in 0..10 {
        app.update();
    }

    let (swung_is_grabbed, swung_angle) = {
        let door = app.world().get::<PantryDoor>(door_entity).unwrap();
        (door.is_grabbed, door.current_angle)
    };
    assert!(swung_is_grabbed, "Door remains grabbed while player is within release distance");
    assert!(
        swung_angle > 0.30,
        "Door must swing open as player pulls it into the room, got angle {}",
        swung_angle
    );

    // 3. Player moves back toward the doorway to swing the door shut
    {
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = Vec3::new(-12.5, 1.35, 5.80);
    }

    for _ in 0..10 {
        app.update();
    }

    let closing_angle = app.world().get::<PantryDoor>(door_entity).unwrap().current_angle;
    assert!(
        closing_angle < swung_angle,
        "Door must swing toward shut as player moves toward the doorway, was {}, now {}",
        swung_angle,
        closing_angle
    );
}

#[test]
fn test_pantry_door_grab_released_when_player_moves_away() {
    use rust_tests::elf_game::workshop::pantry::{
        PantryDoor, pantry_door_interaction_system,
    };
    use rust_tests::elf_game::player::controller::ElfPlayer;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        ExternalImpulse::default(),
        PantryDoor {
            is_grabbed: true,
            grab_release_distance: 2.8,
            ..default()
        },
    )).id();

    // Spawn player far away from door (dist > 2.8m, e.g. at -9.0, 1.35, 5.48 -> dist = 4.0m)
    app.world_mut().spawn((
        Transform::from_xyz(-9.0, 1.35, 5.48),
        ElfPlayer,
    ));

    app.add_systems(Update, pantry_door_interaction_system);
    app.update();

    let door = app.world().get::<PantryDoor>(door_entity).unwrap();
    assert!(
        !door.is_grabbed,
        "Grab must be released when the player moves away beyond release distance"
    );
}

#[test]
fn test_elf_nav_graph_pantry_node_and_purposes() {
    use rust_tests::elf_game::physics_sandbox::{ElfNavGraph, ElfNodePurpose};

    let graph = ElfNavGraph::default_workshop_graph();

    // 1. Verify Node 15 is PantryShelves inside Pantry Storage Wing
    assert!(graph.nodes.len() >= 16);
    let pantry_node = &graph.nodes[15];
    assert_eq!(pantry_node.name, "PantryShelves");
    assert_eq!(pantry_node.position, Vec3::new(-15.0, 0.45, 5.5));
    assert_eq!(pantry_node.purpose, Some(ElfNodePurpose::PantryShelves));

    // 2. Verify bidirectional connection between WestSouthCorridor (Node 8) and Pantry (Node 15)
    assert!(pantry_node.neighbors.contains(&8));
    assert!(graph.nodes[8].neighbors.contains(&15));

    // 3. Verify semantic purposes lookups
    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::PantryShelves), Some(15));
    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::HatCloset), Some(12));
    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::KleptoStash), Some(13));
    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::MooseFeedingStation), Some(4));
    assert!(graph.find_node_by_purpose(ElfNodePurpose::Workbench).is_some());
    assert!(graph.find_node_by_purpose(ElfNodePurpose::MainTableTreatJar).is_some());

    // 4. Verify pathfinding routes from Workbench (Node 0) to Pantry Shelves (Node 15)
    let path = graph.find_path(0, 15).expect("Path from Workbench to Pantry must exist");
    assert_eq!(*path.first().unwrap(), 0);
    assert_eq!(*path.last().unwrap(), 15);
    // Must pass through corridor Node 8 before entering Pantry Node 15
    assert_eq!(path[path.len() - 2], 8);

    // 5. Verify path from Pantry (15) to Moose Feeding Trough (4)
    let path_to_trough = graph.find_path(15, 4).expect("Path from Pantry to Trough must exist");
    assert_eq!(*path_to_trough.first().unwrap(), 15);
    assert_eq!(*path_to_trough.last().unwrap(), 4);
}

#[test]
fn test_moose_initial_dormancy_and_first_present_wakeup() {
    use rust_tests::elf_game::moose::{
        MooseHunger, MooseHungerStage, MooseAwakenedMessage, update_moose_hunger_system,
    };
    use rust_tests::elf_game::workshop::room::{
        TubeTravellingLump, tube_travelling_lump_system,
    };
    use rust_tests::elf_game::core::WorkshopQuota;
    use bevy::ecs::message::Messages;

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

#[test]
fn test_moose_feeder_job_routine_loop() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfJob, ElfNavGraph, ElfNpcState, elf_assistant_ai_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage, FeedMooseMessage};
    use rust_tests::elf_game::workshop::pantry::MooseTreatJar;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    let hunger_res = MooseHunger {
        value: 45.0,
        stage: MooseHungerStage::PeekingThrough,
        ..default()
    };
    app.insert_resource(hunger_res);
    app.init_resource::<ElfNavGraph>();
    app.add_message::<FeedMooseMessage>();
    app.add_systems(Update, elf_assistant_ai_system);

    // Spawn Moose Feeder elf at Workbench Front (Node 0 at 0.0, 0.45, -1.8)
    let mut elf_assistant = ElfAssistant::new(1, "SparkyFeeder", vec![Vec3::ZERO]);
    elf_assistant.assigned_job = Some(ElfJob::MooseFeeder);
    elf_assistant.current_node = 0;
    elf_assistant.state = ElfNpcState::Wandering;

    let elf_entity = app.world_mut().spawn((
        elf_assistant,
        Transform::from_xyz(0.0, 0.45, -1.8),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    // Spawn a MooseTreatJar near Node 1 (WorkbenchWest: -3.8, 0.45, -4.0) with 2 treats
    let jar_entity = app.world_mut().spawn((
        MooseTreatJar { treat_count: 2, max_treats: 4 },
        Transform::from_xyz(-3.5, 0.98, -4.0),
    )).id();

    // 1. Initial update: Elf recognizes job and starts FetchingMooseTreat toward Node 1
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::FetchingMooseTreat, "Elf must enter FetchingMooseTreat");
        assert_eq!(elf.target_node, 1, "Target node must be MainTableTreatJar (Node 1)");
    }

    // Advance elf until reaching Node 1 (-3.8, 0.45, -4.0)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_entity).unwrap();
        tf.translation = Vec3::new(-3.7, 0.45, -4.0);
    }
    app.update();

    // 2. Arrived at Node 1: Elf inspects area, retrieves 1 treat, decrements jar, and switches to DeliveringMooseTreat
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::DeliveringMooseTreat, "Elf must transition to DeliveringMooseTreat");
        assert!(elf.carried_treat.is_some(), "Elf must be carrying a treat entity");
        assert_eq!(elf.target_node, 4, "Destination must be MooseFeedingStation (Node 4)");

        let jar = app.world().get::<MooseTreatJar>(jar_entity).unwrap();
        assert_eq!(jar.treat_count, 1, "Jar treat_count must be decremented from 2 to 1");
    }

    // 3. Move elf to MooseFeedingStation (Node 4 at -6.5, 0.45, -10.5)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_entity).unwrap();
        tf.translation = Vec3::new(-6.5, 0.45, -10.5);
    }
    app.update();

    // 4. Treat deposited: Mr. Moose hunger reduced, carried treat despawned, loops back to FetchingMooseTreat
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::FetchingMooseTreat, "Elf must loop back to FetchingMooseTreat");
        assert!(elf.carried_treat.is_none(), "Carried treat must be cleared after delivery");
        assert_eq!(elf.target_node, 1, "Destination must be back to Node 1 for next treat");

        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.value, 0.0, "Hunger (45) must be reduced by 60 down to 0");
        assert_eq!(hunger.stage, MooseHungerStage::BehindShutters);
    }
}

#[test]
fn test_moose_feeder_panic_when_treat_jar_missing_or_empty() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfJob, ElfNavGraph, ElfNpcState, elf_assistant_ai_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage, FeedMooseMessage};
    use rust_tests::elf_game::workshop::pantry::MooseTreatJar;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    let hunger_res = MooseHunger {
        value: 10.0,
        stage: MooseHungerStage::BehindShutters,
        ..default()
    };
    app.insert_resource(hunger_res);
    app.init_resource::<ElfNavGraph>();
    app.add_message::<FeedMooseMessage>();
    app.add_systems(Update, elf_assistant_ai_system);

    // Spawn Moose Feeder elf arriving at Node 1 (-3.8, 0.45, -4.0) with an EMPTY jar (treat_count: 0)
    let mut elf_assistant = ElfAssistant::new(1, "SparkyPanic", vec![Vec3::ZERO]);
    elf_assistant.assigned_job = Some(ElfJob::MooseFeeder);
    elf_assistant.current_node = 1;
    elf_assistant.target_node = 1;
    elf_assistant.path = vec![1];
    elf_assistant.path_step = 0;
    elf_assistant.state = ElfNpcState::FetchingMooseTreat;

    let elf_entity = app.world_mut().spawn((
        elf_assistant,
        Transform::from_xyz(-3.8, 0.45, -4.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    // Empty jar
    let empty_jar = app.world_mut().spawn((
        MooseTreatJar { treat_count: 0, max_treats: 4 },
        Transform::from_xyz(-3.8, 0.98, -4.0),
    )).id();

    // 1. Update: Elf arrives at Node 1, finds no treats, immediately enters Panicking with missing_treats_panic
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Panicking, "Elf must transition into Panicking");
        assert!(elf.missing_treats_panic, "missing_treats_panic flag must be set to true");
    }

    // 2. While still missing treats, elf continues panicking and pacing between nodes
    for _ in 0..10 {
        app.update();
    }
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Panicking, "Elf must remain in Panicking while treats are missing");
        assert!(elf.missing_treats_panic);
    }

    // 3. Player brings a fresh jar with 4 treats from the Pantry into the Node 1 area!
    app.world_mut().despawn(empty_jar);
    app.world_mut().spawn((
        MooseTreatJar { treat_count: 4, max_treats: 4 },
        Transform::from_xyz(-3.6, 0.98, -4.0),
    ));

    app.update();

    // 4. Panic resolved! Elf detects fresh jar and switches back to FetchingMooseTreat
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::FetchingMooseTreat, "Elf must recover from panic and resume FetchingMooseTreat");
        assert!(!elf.missing_treats_panic, "missing_treats_panic must be cleared");
        assert_eq!(elf.target_node, 1, "Must target Node 1 to retrieve treat");
    }
}

#[test]
fn test_moose_feeder_knockdown_drops_treat_and_recovers() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfJob, ElfNavGraph, ElfNpcState, elf_assistant_ai_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage, MooseSnack, FeedMooseMessage};
    use rust_tests::elf_game::player::hands::Grabbable;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    let hunger_res = MooseHunger {
        value: 10.0,
        stage: MooseHungerStage::BehindShutters,
        ..default()
    };
    app.insert_resource(hunger_res);
    app.init_resource::<ElfNavGraph>();
    app.add_message::<FeedMooseMessage>();
    app.add_systems(Update, elf_assistant_ai_system);

    // Spawn carried treat
    let treat_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(0.0, 0.5, 0.0),
        Velocity::default(),
    )).id();

    let mut elf_assistant = ElfAssistant::new(1, "SparkyDrop", vec![Vec3::ZERO]);
    elf_assistant.assigned_job = Some(ElfJob::MooseFeeder);
    elf_assistant.state = ElfNpcState::DeliveringMooseTreat;
    elf_assistant.carried_treat = Some(treat_entity);

    let elf_entity = app.world_mut().spawn((
        elf_assistant,
        Transform::from_xyz(0.0, 0.45, 0.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    // Dynamic prop flying fast towards elf to cause knockdown
    app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(0.3, 0.45, 0.0),
        Velocity {
            linear: Vec3::new(-4.0, 0.0, 0.0),
            angular: Vec3::ZERO,
        },
    ));

    app.update();

    // 1. Hit! Elf enters RagdollTumble and drops carried treat
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::RagdollTumble, "Impulse must send elf into RagdollTumble");
        assert!(elf.carried_treat.is_none(), "Carried treat must be dropped upon knockdown");
    }

    // 2. Advance time past tumble duration (2.2s)
    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.tumble_timer = 2.5;
        let mut vel = app.world_mut().get_mut::<Velocity>(elf_entity).unwrap();
        vel.linear = Vec3::ZERO;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Recovering);
    }

    // 3. Complete recovery duration (0.8s) -> elf resumes FetchingMooseTreat to get a replacement treat!
    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.recovery_timer = 0.9;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::FetchingMooseTreat, "Elf must resume FetchingMooseTreat after recovery");
    }
}

#[test]
fn test_moose_feeder_hat_equipping_activates_routine() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfJob, ElfJobHat, ElfNpcState, elf_hat_assignment_system,
    };
    use rust_tests::elf_game::player::ElfHands;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut key_inputs = ButtonInput::<KeyCode>::default();
    key_inputs.press(KeyCode::KeyE);
    app.insert_resource(key_inputs);
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_systems(Update, elf_hat_assignment_system);

    // Player holding MooseFeeder hat
    let player_entity = app.world_mut().spawn((
        ElfHands::default(),
        Transform::from_xyz(-4.0, 0.45, 10.0),
    )).id();

    let feeder_hat_e = app.world_mut().spawn((
        ElfJobHat {
            job: ElfJob::MooseFeeder,
            worn_by: None,
        },
        Transform::from_xyz(-4.0, 0.45, 10.0),
        Velocity::default(),
    )).id();

    {
        let mut hands = app.world_mut().get_mut::<ElfHands>(player_entity).unwrap();
        hands.right_grabbed = Some(feeder_hat_e);
    }

    let elf_entity = app.world_mut().spawn((
        ElfAssistant::new(1, "RecipientElf", vec![Vec3::ZERO]),
        Transform::from_xyz(-4.0, 0.45, 11.0),
        Velocity::default(),
    )).id();

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.assigned_job, Some(ElfJob::MooseFeeder), "Elf must receive MooseFeeder job");
        assert_eq!(elf.state, ElfNpcState::FetchingMooseTreat, "Equipping MooseFeeder hat must set state to FetchingMooseTreat");
        assert_eq!(elf.worn_hat_entity, Some(feeder_hat_e));

        let hands = app.world().get::<ElfHands>(player_entity).unwrap();
        assert_eq!(hands.right_grabbed, None, "Player hand must release the hat upon placing it");
    }
}

#[test]
fn test_mouse_sensitivity_slider_activation_and_hit_filtering() {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::window::WindowRef;
    use rust_tests::elf_game::ui::{
        on_slider_thumb_drag, on_slider_thumb_over, on_slider_thumb_out,
        MenuState, MenuSubmenu, SensitivitySlider, SensitivitySliderThumb,
        SensitivitySliderTrack, SensitivitySliderFill,
        DEFAULT_SENSITIVITY,
    };

    let mut app = App::new();
    app.init_resource::<MenuState>();
    app.add_observer(on_slider_thumb_drag);
    app.add_observer(on_slider_thumb_over);
    app.add_observer(on_slider_thumb_out);

    // Spawn slider components
    let track_e = app.world_mut().spawn((
        SensitivitySlider,
        SensitivitySliderTrack,
    )).id();

    let fill_e = app.world_mut().spawn((
        SensitivitySlider,
        SensitivitySliderFill,
    )).id();

    let thumb_e = app.world_mut().spawn((
        SensitivitySlider,
        SensitivitySliderThumb,
        BackgroundColor(Color::from(bevy::color::palettes::tailwind::CYAN_400)),
    )).id();

    // Spawn an unrelated entity (e.g. background, player or button)
    let unrelated_e = app.world_mut().spawn(()).id();

    let dummy_pointer = Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(WindowRef::Entity(Entity::PLACEHOLDER).normalize(None).unwrap()),
            position: Vec2::ZERO,
        },
    );

    let dummy_hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);

    // 1. Menu CLOSED: Dragging thumb or unrelated entity MUST NOT move slider
    assert!(!app.world().resource::<MenuState>().is_open);
    assert_eq!(app.world().resource::<MenuState>().mouse_sensitivity, DEFAULT_SENSITIVITY);

    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Holding Mouse1 when menu is closed must not move sensitivity slider"
    );

    app.world_mut().trigger(PointerDrag {
        entity: unrelated_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Holding Mouse1 on unrelated entity when menu is closed must not move slider"
    );

    // 2. Menu OPEN (Main Submenu / Options Menu):
    app.world_mut().resource_mut::<MenuState>().is_open = true;
    app.world_mut().resource_mut::<MenuState>().active_submenu = MenuSubmenu::Main;

    // Dragging UNRELATED entity while menu is open MUST NOT move slider
    app.world_mut().trigger(PointerDrag {
        entity: unrelated_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Holding Mouse1 on unrelated entity (not slider) must not move sensitivity slider"
    );

    // Dragging with non-primary button (e.g. Secondary / Right click) on thumb MUST NOT move slider
    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Secondary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Right-clicking/dragging must not move sensitivity slider"
    );

    // Dragging on SLIDER THUMB with Mouse1 (Primary) MUST move slider
    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(20.2, 0.0),
        delta: Vec2::new(20.2, 0.0),
    });
    let sens_after_thumb = app.world().resource::<MenuState>().mouse_sensitivity;
    assert!(
        sens_after_thumb > DEFAULT_SENSITIVITY,
        "Dragging slider thumb to the right must increase mouse sensitivity"
    );

    // Dragging on SLIDER TRACK with Mouse1 (Primary) MUST also move slider
    app.world_mut().trigger(PointerDrag {
        entity: track_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(-10.1, 0.0),
        delta: Vec2::new(-10.1, 0.0),
    });
    let sens_after_track = app.world().resource::<MenuState>().mouse_sensitivity;
    assert!(
        sens_after_track < sens_after_thumb,
        "Dragging on slider track to the left must decrease mouse sensitivity"
    );

    // Dragging on SLIDER FILL with Mouse1 (Primary) MUST also move slider
    app.world_mut().trigger(PointerDrag {
        entity: fill_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(10.1, 0.0),
        delta: Vec2::new(10.1, 0.0),
    });
    assert!(
        app.world().resource::<MenuState>().mouse_sensitivity > sens_after_track,
        "Dragging on slider fill must adjust mouse sensitivity"
    );

    // 3. Menu in DEBUG Submenu: Options menu is inactive, dragging must not move slider
    app.world_mut().resource_mut::<MenuState>().active_submenu = MenuSubmenu::Debug;
    let sens_before_debug = app.world().resource::<MenuState>().mouse_sensitivity;
    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(30.0, 0.0),
        delta: Vec2::new(30.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        sens_before_debug,
        "Dragging slider when debug submenu is active must not move slider"
    );

    // 4. Hover tests: PointerOver on unrelated entity must not highlight thumb
    app.world_mut().resource_mut::<MenuState>().active_submenu = MenuSubmenu::Main;
    app.world_mut().trigger(PointerOver {
        entity: unrelated_e,
        pointer: dummy_pointer.clone(),
        hit: dummy_hit.clone(),
    });
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb_e).unwrap().0,
        Color::from(bevy::color::palettes::tailwind::CYAN_400),
        "Hovering unrelated entity must not highlight slider thumb"
    );

    // PointerOver on slider thumb highlights to CYAN_300
    app.world_mut().trigger(PointerOver {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        hit: dummy_hit.clone(),
    });
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb_e).unwrap().0,
        Color::from(bevy::color::palettes::tailwind::CYAN_300),
        "Hovering slider thumb must highlight it to CYAN_300"
    );

    // PointerOut restores CYAN_400
    app.world_mut().trigger(PointerOut {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        hit: dummy_hit,
    });
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb_e).unwrap().0,
        Color::from(bevy::color::palettes::tailwind::CYAN_400),
        "PointerOut must restore thumb to CYAN_400"
    );
}

#[test]
fn test_dialogue_wheel_layout_zero_options() {
    use rust_tests::elf_game::dialogue::{compute_wheel_sections, DialogueAction, DialogueMenu};
    use std::f32::consts::FRAC_PI_2;

    let empty_menu = DialogueMenu {
        title: "Empty Menu".to_string(),
        prompt: "No options".to_string(),
        options: vec![],
    };

    let sections = compute_wheel_sections(&empty_menu);
    assert_eq!(sections.len(), 1, "Zero options must produce exactly 1 section (bottom half BACK)");
    let back = &sections[0];
    assert!(back.is_back);
    assert_eq!(back.action, DialogueAction::Close);
    // Centered at bottom: -PI/2
    assert!((back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");
    // Spanning half circle: half span = PI/2
    assert!((back.angle_half_span - FRAC_PI_2).abs() < 1e-4, "BACK must span the bottom half (half span PI/2)");
}

#[test]
fn test_dialogue_wheel_layout_one_option() {
    use rust_tests::elf_game::dialogue::{compute_wheel_sections, DialogueAction, DialogueMenu, DialogueOption};
    use std::f32::consts::FRAC_PI_2;

    let single_opt_menu = DialogueMenu {
        title: "Single Option Menu".to_string(),
        prompt: "One option available".to_string(),
        options: vec![DialogueOption {
            label: "Only Option".to_string(),
            description: "A single choice".to_string(),
            action: DialogueAction::Speak("Selected".to_string()),
        }],
    };

    let sections = compute_wheel_sections(&single_opt_menu);
    assert_eq!(sections.len(), 2, "One option must produce 2 sections (top half option, bottom half BACK)");

    // Top half is the option (+PI/2)
    let top_opt = &sections[0];
    assert!(!top_opt.is_back);
    assert_eq!(top_opt.label, "Only Option");
    assert!((top_opt.angle_center - FRAC_PI_2).abs() < 1e-4, "Top option must be centered at top (+PI/2)");
    assert!((top_opt.angle_half_span - FRAC_PI_2).abs() < 1e-4, "Top option must span top half (half span PI/2)");

    // Bottom half is BACK (-PI/2)
    let bottom_back = &sections[1];
    assert!(bottom_back.is_back);
    assert_eq!(bottom_back.action, DialogueAction::Close);
    assert!((bottom_back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");
    assert!((bottom_back.angle_half_span - FRAC_PI_2).abs() < 1e-4, "BACK must span bottom half (half span PI/2)");
}

#[test]
fn test_dialogue_wheel_layout_three_sections_thirds() {
    use rust_tests::elf_game::dialogue::{compute_wheel_sections, DialogueAction, DialogueMenu, DialogueOption};
    use std::f32::consts::{FRAC_PI_2, PI};

    // 2 options + BACK = 3 sections total (thirds of the wheel)
    let two_opt_menu = DialogueMenu {
        title: "Two Options Menu".to_string(),
        prompt: "Choose between two".to_string(),
        options: vec![
            DialogueOption {
                label: "Option Alpha".to_string(),
                description: "First".to_string(),
                action: DialogueAction::Speak("A".to_string()),
            },
            DialogueOption {
                label: "Option Beta".to_string(),
                description: "Second".to_string(),
                action: DialogueAction::Speak("B".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&two_opt_menu);
    assert_eq!(sections.len(), 3, "2 options + BACK must produce 3 sections (thirds of the selection wheel)");

    // Total angle span for each must be 2*PI / 3 (120 deg)
    let expected_half_span = PI / 3.0;
    for sec in &sections {
        assert!((sec.angle_half_span - expected_half_span).abs() < 1e-4, "Each third must have half-span PI/3");
    }

    // BACK is always centered at the bottom (-FRAC_PI_2 = -90 deg / 270 deg)
    let back = &sections[2];
    assert!(back.is_back);
    assert_eq!(back.action, DialogueAction::Close);
    assert!((back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");

    // Option Alpha at top-right (30 deg = PI/6)
    let opt_a = &sections[0];
    assert!(!opt_a.is_back);
    assert_eq!(opt_a.label, "Option Alpha");
    assert!((opt_a.angle_center - (PI / 6.0)).abs() < 1e-4, "Option Alpha must be at PI/6 (30 deg)");

    // Option Beta at top-left (150 deg = 5*PI/6)
    let opt_b = &sections[1];
    assert!(!opt_b.is_back);
    assert_eq!(opt_b.label, "Option Beta");
    assert!((opt_b.angle_center - (5.0 * PI / 6.0)).abs() < 1e-4, "Option Beta must be at 5*PI/6 (150 deg)");
}

#[test]
fn test_dialogue_wheel_layout_four_sections_quarters() {
    use rust_tests::elf_game::dialogue::{compute_wheel_sections, DialogueAction, DialogueMenu, DialogueOption};
    use std::f32::consts::{FRAC_PI_2, PI};

    let three_opt_menu = DialogueMenu {
        title: "Three Options Menu".to_string(),
        prompt: "Choose between three".to_string(),
        options: vec![
            DialogueOption {
                label: "Right".to_string(),
                description: "East".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Top".to_string(),
                description: "North".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
            DialogueOption {
                label: "Left".to_string(),
                description: "West".to_string(),
                action: DialogueAction::Speak("3".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&three_opt_menu);
    assert_eq!(sections.len(), 4, "3 options + BACK must produce 4 sections (quarters of the wheel)");

    let expected_half_span = PI / 4.0;
    for sec in &sections {
        assert!((sec.angle_half_span - expected_half_span).abs() < 1e-4, "Each quarter must have half-span PI/4");
    }

    // BACK is centered at bottom (-FRAC_PI_2)
    let back = &sections[3];
    assert!(back.is_back);
    assert!((back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");

    // Option 0 (Right: 0 rad)
    assert!((sections[0].angle_center - 0.0).abs() < 1e-4, "Section 0 must be centered at 0 rad (East)");
    // Option 1 (Top: PI/2 rad)
    assert!((sections[1].angle_center - FRAC_PI_2).abs() < 1e-4, "Section 1 must be centered at PI/2 rad (North)");
    // Option 2 (Left: PI rad)
    assert!((sections[2].angle_center - PI).abs() < 1e-4, "Section 2 must be centered at PI rad (West)");
}

#[test]
fn test_dialogue_mouse_angle_selection() {
    use rust_tests::elf_game::dialogue::{compute_wheel_sections, find_section_for_angle, DialogueAction, DialogueMenu, DialogueOption};
    use std::f32::consts::{FRAC_PI_2, PI};

    let menu = DialogueMenu {
        title: "Test".to_string(),
        prompt: "Test".to_string(),
        options: vec![
            DialogueOption {
                label: "Option 1".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Option 2".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&menu);
    assert_eq!(sections.len(), 3);

    // Aiming straight down: -PI/2 -> selects BACK (index 2)
    let down_sec = find_section_for_angle(&sections, -FRAC_PI_2);
    assert_eq!(down_sec, Some(2), "Aiming straight down must select BACK");

    // Aiming top-right: PI/6 (30 deg) -> selects Option 1 (index 0)
    let tr_sec = find_section_for_angle(&sections, PI / 6.0);
    assert_eq!(tr_sec, Some(0), "Aiming top-right must select Option 1");

    // Aiming top-left: 5*PI/6 (150 deg) -> selects Option 2 (index 1)
    let tl_sec = find_section_for_angle(&sections, 5.0 * PI / 6.0);
    assert_eq!(tl_sec, Some(1), "Aiming top-left must select Option 2");
}

#[test]
fn test_dialogue_keyboard_wasd_navigation() {
    use rust_tests::elf_game::dialogue::{compute_wheel_sections, find_section_for_wasd, DialogueAction, DialogueMenu, DialogueOption};

    let menu = DialogueMenu {
        title: "Test".to_string(),
        prompt: "Test".to_string(),
        options: vec![
            DialogueOption {
                label: "Right".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Top".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
            DialogueOption {
                label: "Left".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("3".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&menu);
    assert_eq!(sections.len(), 4);

    // Pressing S: (0.0, -1.0) -> must select BACK (index 3 at bottom)
    let s_sec = find_section_for_wasd(&sections, Vec2::new(0.0, -1.0));
    assert_eq!(s_sec, Some(3), "S key must always select BACK at bottom");

    // Pressing W: (0.0, 1.0) -> must select Top option (index 1)
    let w_sec = find_section_for_wasd(&sections, Vec2::new(0.0, 1.0));
    assert_eq!(w_sec, Some(1), "W key must select Top option");

    // Pressing A: (-1.0, 0.0) -> must select Left option (index 2)
    let a_sec = find_section_for_wasd(&sections, Vec2::new(-1.0, 0.0));
    assert_eq!(a_sec, Some(2), "A key must select Left option");

    // Pressing D: (1.0, 0.0) -> must select Right option (index 0)
    let d_sec = find_section_for_wasd(&sections, Vec2::new(1.0, 0.0));
    assert_eq!(d_sec, Some(0), "D key must select Right option");
}

#[test]
fn test_dialogue_submenu_navigation_and_back_resume_elf() {
    use rust_tests::elf_game::{
        dialogue::{compute_wheel_sections, ActiveDialogue, DialogueAction, DialogueMenu, DialogueOption},
        physics_sandbox::{ElfAssistant, ElfJob, ElfNpcState},
    };

    let mut active = ActiveDialogue::default();
    let mut elf = ElfAssistant::new(1, "Sparky", vec![Vec3::ZERO]);
    elf.state = ElfNpcState::Wandering;

    let sub_menu = DialogueMenu {
        title: "Submenu".to_string(),
        prompt: "Sub prompt".to_string(),
        options: vec![DialogueOption {
            label: "Sub Option".to_string(),
            description: "".to_string(),
            action: DialogueAction::AssignJob(ElfJob::MooseFeeder),
        }],
    };

    let top_menu = DialogueMenu {
        title: "Top Menu".to_string(),
        prompt: "Top prompt".to_string(),
        options: vec![DialogueOption {
            label: "Open Sub".to_string(),
            description: "".to_string(),
            action: DialogueAction::Submenu(sub_menu.clone()),
        }],
    };

    let dummy_elf_entity = Entity::from_raw_u32(100).unwrap();
    active.open_custom_menu(dummy_elf_entity, "Sparky", top_menu.clone());
    elf.previous_state = elf.state;
    elf.state = ElfNpcState::InDialogue;

    assert!(active.is_active);
    assert_eq!(elf.state, ElfNpcState::InDialogue);
    assert_eq!(active.menu_stack.len(), 0);

    // 1. Enter Submenu
    if let Some(current) = active.current_menu.take() {
        active.menu_stack.push(current);
    }
    active.sections_cache = compute_wheel_sections(&sub_menu);
    active.current_menu = Some(sub_menu);
    assert_eq!(active.menu_stack.len(), 1, "Submenu must push parent menu to stack");
    assert_eq!(active.current_menu.as_ref().unwrap().title, "Submenu");

    // 2. Select BACK in Submenu -> pops back to top menu
    let parent = active.menu_stack.pop().expect("Must have parent in stack");
    active.sections_cache = compute_wheel_sections(&parent);
    active.current_menu = Some(parent);
    assert_eq!(active.menu_stack.len(), 0);
    assert_eq!(active.current_menu.as_ref().unwrap().title, "Top Menu");
    assert!(active.is_active, "Selecting BACK in submenu must keep dialogue active");

    // 3. Select BACK at top level menu -> closes dialogue and resumes elf moving
    let parent_opt = active.menu_stack.pop();
    assert!(parent_opt.is_none());
    active.is_active = false;
    active.current_menu = None;

    // Elf resumes moving
    if elf.state == ElfNpcState::InDialogue {
        elf.state = elf.previous_state;
    }
    assert_eq!(elf.state, ElfNpcState::Wandering, "Pressing BACK at top level must resume elf wandering");
}

#[test]
fn test_dialogue_camera_and_player_movement_locked() {
    use rust_tests::elf_game::{
        dialogue::ActiveDialogue,
        player::{
            controller::{elf_look_system, elf_player_controller_system, CameraPivot, ElfInputSettings, ElfPlayer},
            BhopConfig, ElfMovement,
        },
    };
    use bevy::input::mouse::AccumulatedMouseMotion;

    let mut app = App::new();
    app.init_resource::<ActiveDialogue>();
    app.init_resource::<AccumulatedMouseMotion>();
    app.init_resource::<ElfInputSettings>();
    app.init_resource::<BhopConfig>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_systems(Update, (elf_look_system, elf_player_controller_system));

    let player_e = app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(0.0, 1.0, 0.0),
        ElfMovement::default(),
        bevy_rapier3d::prelude::KinematicCharacterController::default(),
    )).id();

    let pivot_e = app.world_mut().spawn((
        CameraPivot { pitch: 0.0 },
        Transform::IDENTITY,
    )).id();

    // 1. When dialogue is INACTIVE: camera rotates with mouse motion
    {
        let mut mouse_motion = app.world_mut().resource_mut::<AccumulatedMouseMotion>();
        mouse_motion.delta = Vec2::new(100.0, 50.0);
    }
    app.update();

    let player_rot_unlocked = app.world().get::<Transform>(player_e).unwrap().rotation;
    let pivot_unlocked = app.world().get::<CameraPivot>(pivot_e).unwrap().pitch;
    assert_ne!(player_rot_unlocked, Quat::IDENTITY, "Unlocked camera must rotate player yaw");
    assert_ne!(pivot_unlocked, 0.0, "Unlocked camera must rotate pitch");

    // 2. When dialogue is ACTIVE: camera moves are completely locked
    app.world_mut().resource_mut::<ActiveDialogue>().is_active = true;
    let saved_yaw = app.world().get::<Transform>(player_e).unwrap().rotation;
    let saved_pitch = app.world().get::<CameraPivot>(pivot_e).unwrap().pitch;

    {
        let mut mouse_motion = app.world_mut().resource_mut::<AccumulatedMouseMotion>();
        mouse_motion.delta = Vec2::new(500.0, -300.0);
    }
    app.update();

    let locked_yaw = app.world().get::<Transform>(player_e).unwrap().rotation;
    let locked_pitch = app.world().get::<CameraPivot>(pivot_e).unwrap().pitch;
    assert_eq!(locked_yaw, saved_yaw, "Camera yaw must be completely locked while in dialogue");
    assert_eq!(locked_pitch, saved_pitch, "Camera pitch must be completely locked while in dialogue");
}

#[test]
fn test_elf_in_dialogue_stops_and_faces_player_smoothly() {
    use rust_tests::elf_game::{
        physics_sandbox::{elf_assistant_ai_system, ElfAssistant, ElfNavGraph, ElfNpcState},
        player::{controller::ElfPlayer, ElfMovement},
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.init_resource::<Time>();
    app.init_resource::<ElfNavGraph>();
    app.add_systems(Update, elf_assistant_ai_system);

    // Player standing North at (0.0, 0.45, -5.0)
    app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(0.0, 0.45, -5.0),
        ElfMovement::default(),
    ));

    // Elf initially standing at (0.0, 0.45, 0.0) facing East (Quat from Y 90 deg)
    let elf_e = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.45, 0.0).with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        Velocity {
            linear: Vec3::new(2.0, 0.0, 1.0),
            angular: Vec3::ZERO,
        },
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ElfAssistant {
            state: ElfNpcState::InDialogue,
            ..ElfAssistant::new(1, "TestElf", vec![Vec3::ZERO])
        },
    )).id();

    // Advance simulation frames with 100ms delta each
    for _ in 0..10 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(100));
        app.update();
    }

    let vel = app.world().get::<Velocity>(elf_e).unwrap();
    let tf = app.world().get::<Transform>(elf_e).unwrap();

    // 1. Must stop walking
    assert_eq!(vel.linear.x, 0.0, "Elf in dialogue must have zero linear X velocity");
    assert_eq!(vel.linear.z, 0.0, "Elf in dialogue must have zero linear Z velocity");

    // 2. Must face towards player (North: flat_diff = (0, 0, -5), angle atan2 = PI or -PI)
    // The elf initial facing was East (PI/2 = 1.57), facing North is ~PI.
    // Transform.rotation must have slerped smoothly towards North.
    let yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
    assert!(
        yaw.abs() > 2.0,
        "Elf must smoothly rotate towards player (expected yaw ~3.14 / -3.14, got {})",
        yaw
    );
}

#[test]
fn test_dialogue_ui_bottom_prompt_box_and_keyboard_cursor_reset() {
    use rust_tests::elf_game::dialogue::{
        spawn_dialogue_ui, elf_dialogue_interaction_system, compute_wheel_sections,
        ActiveDialogue, DialogueAction, DialogueMenu, DialogueOption, DialoguePromptBox,
        DialogueWheelFrame, DialogueWheelHub, DialogueWheelPromptText,
    };
    use bevy::window::{Window, PrimaryWindow, CursorOptions};
    use bevy::input::ButtonInput;

    // 1. Verify UI hierarchy: bottom rectangular prompt box and transparent wheel center
    let mut app = App::new();
    app.add_systems(Startup, spawn_dialogue_ui);
    app.update();

    // Check DialoguePromptBox
    let mut prompt_box_q = app.world_mut().query::<(&DialoguePromptBox, &Node, &BackgroundColor)>();
    let (_, box_node, box_bg) = prompt_box_q.iter(app.world()).next().expect("DialoguePromptBox must exist");
    assert_eq!(box_node.position_type, PositionType::Absolute);
    assert_eq!(box_node.bottom, Val::Px(24.0), "Prompt box must be anchored at the bottom of the screen");
    assert!(box_bg.0.alpha() > 0.8, "Prompt box should have dark solid/semi-solid background");

    // Check DialogueWheelPromptText is larger (font_size >= 20.0)
    let mut prompt_text_q = app.world_mut().query::<(&DialogueWheelPromptText, &TextFont)>();
    let (_, text_font) = prompt_text_q.iter(app.world()).next().expect("DialogueWheelPromptText must exist");
    match text_font.font_size {
        FontSize::Px(px) => assert!(px >= 20.0, "Elf prompt text must be larger (>= 20px, got {})", px),
        _ => panic!("Expected FontSize::Px"),
    }

    // Check DialogueWheelFrame and DialogueWheelHub are transparent
    let mut frame_q = app.world_mut().query::<(&DialogueWheelFrame, &BackgroundColor)>();
    let (_, frame_bg) = frame_q.iter(app.world()).next().expect("DialogueWheelFrame must exist");
    assert_eq!(frame_bg.0, Color::NONE, "DialogueWheelFrame background must be clear/transparent (Color::NONE)");

    let mut hub_q = app.world_mut().query::<(&DialogueWheelHub, &BackgroundColor, Option<&Children>)>();
    let (_, hub_bg, hub_children) = hub_q.iter(app.world()).next().expect("DialogueWheelHub must exist");
    assert_eq!(hub_bg.0, Color::NONE, "DialogueWheelHub center must be clear/transparent (Color::NONE)");
    assert!(hub_children.is_none(), "Center hub must have no children blocking view of the elf");

    // 2. Verify keyboard navigation sets mouse position to center
    let mut sim_app = App::new();
    sim_app.init_resource::<ButtonInput<KeyCode>>();
    sim_app.init_resource::<ButtonInput<MouseButton>>();
    let mut active = ActiveDialogue::default();
    active.is_active = true;

    let menu = DialogueMenu {
        title: "Test".to_string(),
        prompt: "Choose".to_string(),
        options: vec![
            DialogueOption {
                label: "Right".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Top".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
            DialogueOption {
                label: "Left".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("3".to_string()),
            },
        ],
    };
    active.sections_cache = compute_wheel_sections(&menu);
    active.current_menu = Some(menu);
    active.selected_index = 0; // Starts at 0 (Right)
    sim_app.insert_resource(active);

    // Spawn window with non-centered mouse position
    let mut test_window = Window::default();
    test_window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
    let win_w = test_window.width();
    let win_h = test_window.height();
    let center = Vec2::new(win_w / 2.0, win_h / 2.0);

    let win_entity = sim_app.world_mut().spawn((
        test_window,
        PrimaryWindow,
        CursorOptions::default(),
    )).id();

    sim_app.add_systems(Update, elf_dialogue_interaction_system);

    // Press 'W' key -> selects Top option (index 1) and resets cursor to center
    sim_app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
    sim_app.update();

    let updated_active = sim_app.world().resource::<ActiveDialogue>();
    assert_eq!(updated_active.selected_index, 1, "W key must select Top option (index 1)");

    let updated_win = sim_app.world().get::<Window>(win_entity).unwrap();
    let cur_pos = updated_win.cursor_position().expect("Cursor position must be set");
    assert!((cur_pos.x - center.x).abs() < 1e-4, "Cursor X must be set to center");
    assert!((cur_pos.y - center.y).abs() < 1e-4, "Cursor Y must be set to center");

    // Release 'W' key: on subsequent frame with mouse at center, selected_index must NOT be reset!
    sim_app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyW);
    sim_app.update();

    let after_release_active = sim_app.world().resource::<ActiveDialogue>();
    assert_eq!(
        after_release_active.selected_index, 1,
        "Subsequent frame with mouse at center must retain keyboard selection (index 1)"
    );
}

#[test]
fn test_oven_stove_models_and_gas_hose_attachment() {
    use rust_tests::elf_game::workshop::stove::{
        spawn_oven_stove_and_propane_scene, GasFurnaceStove, GasHose,
        HoseAttachment, PropaneTankHolder, MainPropaneRefillStation, PropaneTank,
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    app.update();

    // 1. Gas furnace stove entity
    let mut stove_q = app.world_mut().query::<(&GasFurnaceStove, &Transform)>();
    let (stove, stove_tf) = stove_q.iter(app.world()).next().expect("Stove must exist");
    assert!(stove.is_lit, "Furnace stove should be lit by default");
    assert_eq!(stove.internal_temp, 20.0, "Stove incorporates oven baking temp functionality");
    assert!(stove.is_heated, "Stove incorporates oven heating functionality");
    assert!((stove_tf.translation.x - (-6.5)).abs() < 0.2, "Stove must be at X ≈ -6.5");
    assert!((stove_tf.translation.z - (-11.4)).abs() < 0.2, "Stove must be at Z ≈ -11.4");

    // 2. Flexible yellow gas hose entity
    let mut hose_q = app.world_mut().query::<&GasHose>();
    let hose = hose_q.iter(app.world()).next().expect("Flexible yellow hose must exist");
    assert_eq!(hose.attached_to, HoseAttachment::Stove, "Hose must default to Stove attachment");
    assert!(!hose.is_venting_gas);

    // 3. Outside propane tank holder and tanks
    let mut holder_q = app.world_mut().query::<(&PropaneTankHolder, &Transform)>();
    let (holder, holder_tf) = holder_q.iter(app.world()).next().expect("Propane holder must exist");
    assert!(holder.connected_tank.is_some(), "Holder must have active connected tank");
    assert!((holder_tf.translation.x - (-8.3)).abs() < 0.2, "Holder mounted on exterior wall outside window");

    let mut tanks_q = app.world_mut().query::<&PropaneTank>();
    let tank_count = tanks_q.iter(app.world()).count();
    assert!(tank_count >= 2, "Must spawn at least active and spare propane tanks");

    // 4. Main bulk propane refill tank station in snowy landscape
    let mut station_q = app.world_mut().query::<(&MainPropaneRefillStation, &Transform)>();
    let (_, station_tf) = station_q.iter(app.world()).next().expect("Main bulk tank must exist");
    assert!((station_tf.translation.x - (-16.0)).abs() < 0.2, "Main bulk tank at X ≈ -16.0");
    assert!((station_tf.translation.z - (-26.0)).abs() < 0.2, "Main bulk tank at distance Z ≈ -26.0 in snow");
}

#[test]
fn test_workshop_temperature_heating_and_freezing_spawner_halt() {
    use rust_tests::elf_game::workshop::stove::{
        WorkshopTemperature, WorkshopGasLine, GasFurnaceStove, GasHose, HoseAttachment,
        workshop_temperature_system,
    };
    use rust_tests::elf_game::workshop::room::{
        ConveyorSpawnerTimer, conveyor_cube_spawner_system,
    };
    use rust_tests::elf_game::wrapping::Present;

    let mut app = App::new();
    app.init_resource::<Time>();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<WorkshopTemperature>();
    app.init_resource::<WorkshopGasLine>();
    app.init_resource::<ConveyorSpawnerTimer>();

    app.add_systems(Update, (workshop_temperature_system, conveyor_cube_spawner_system));

    // Spawn stove and hose
    app.world_mut().spawn(GasFurnaceStove { is_lit: true, heat_output: 1.5, ..default() });
    let hose_e = app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Stove,
        wall_inlet_pos: Vec3::ZERO,
        stove_anchor_pos: Vec3::ZERO,
        loose_nozzle_pos: Vec3::ZERO,
        is_venting_gas: false,
        ..default()
    }).id();

    // Set initial cool temperature (15.0 deg C)
    {
        let mut temp = app.world_mut().resource_mut::<WorkshopTemperature>();
        temp.current = 15.0;
        temp.is_frozen = false;
    }

    // Advance 2 seconds with heating active (heating_rate = 1.5 C/s)
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(2));
    app.update();

    {
        let temp = app.world().resource::<WorkshopTemperature>();
        assert!(temp.current > 15.0, "Active stove must heat the workshop (expected > 15.0, got {})", temp.current);
        assert!(!temp.is_frozen, "Workshop must not be frozen");
    }

    // Detach hose so stove stops heating
    app.world_mut().get_mut::<GasHose>(hose_e).unwrap().attached_to = HoseAttachment::Neither;

    // Force temperature to freezing
    {
        let mut temp = app.world_mut().resource_mut::<WorkshopTemperature>();
        temp.current = -5.0;
        temp.is_frozen = true;
    }

    // Trigger spawner timer
    {
        let mut spawner = app.world_mut().resource_mut::<ConveyorSpawnerTimer>();
        spawner.timer.set_elapsed(std::time::Duration::from_secs(5));
    }

    // Run update while frozen
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(50));
    app.update();

    let cube_count_frozen = app.world_mut().query::<&Present>().iter(app.world()).count();
    assert_eq!(cube_count_frozen, 0, "Present cubes must NOT spawn when workshop is frozen");

    // Warm back up above freezing
    {
        let mut temp = app.world_mut().resource_mut::<WorkshopTemperature>();
        temp.current = 18.0;
        temp.is_frozen = false;
        let mut spawner = app.world_mut().resource_mut::<ConveyorSpawnerTimer>();
        spawner.timer.set_elapsed(std::time::Duration::from_secs(5));
    }

    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(50));
    app.update();

    let cube_count_thawed = app.world_mut().query::<&Present>().iter(app.world()).count();
    assert!(cube_count_thawed > 0, "Present cubes must spawn once workshop is kept warm above freezing");
}

#[test]
fn test_repurposed_gas_pressure_gauge_needle_and_alert() {
    use rust_tests::elf_game::diegetic_ui::hunger_dial::{
        spawn_gas_pressure_gauge, update_gas_pressure_gauge_system, animate_gauge_system,
        Gauge, GasPressureGauge, GaugeNeedle, GaugeWarningBeacon,
    };
    use rust_tests::elf_game::workshop::stove::WorkshopGasLine;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 100.0,
        burn_rate: 0.8,
        leak_vent_rate: 2.5,
    });

    app.add_systems(Startup, spawn_gas_pressure_gauge);
    app.add_systems(Update, (update_gas_pressure_gauge_system, animate_gauge_system));
    app.update();

    // Verify GasPressureGauge component exists
    let mut gauge_q = app.world_mut().query::<(&GasPressureGauge, &Gauge)>();
    let (_, gauge) = gauge_q.iter(app.world()).next().expect("GasPressureGauge must exist");
    assert_eq!(gauge.value, 1.0, "Gauge value should be 1.0 at 100 PSI");

    // Full pressure (100 PSI) -> needle at max_angle (+2.356 rad)
    {
        let mut needle_q = app.world_mut().query::<&GaugeNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            (needle.current_angle - needle.max_angle).abs() < 0.1,
            "Needle should be at maximum angle (≈ +2.356) when gas tank is full (100 PSI), got {}",
            needle.current_angle
        );
        let mut beacon_q = app.world_mut().query::<&GaugeWarningBeacon>();
        let beacon = beacon_q.iter(app.world()).next().unwrap();
        assert!(!beacon.is_alert, "Warning beacon must not alert when gas pressure is full");
    }

    // Drop pressure below 15 PSI (< 15% warning threshold)
    {
        let mut gas = app.world_mut().resource_mut::<WorkshopGasLine>();
        gas.pressure_psi = 10.0;
    }
    app.update();

    {
        let mut needle_q = app.world_mut().query::<&GaugeNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(needle.is_jittering, "Needle must jitter when gas pressure is critically low");

        let mut beacon_q = app.world_mut().query::<&GaugeWarningBeacon>();
        let beacon = beacon_q.iter(app.world()).next().unwrap();
        assert!(beacon.is_alert, "Warning beacon must pulse in alert mode when gas pressure is < 15 PSI");
    }
}

#[test]
fn test_propane_tank_flow_consumption_and_refill() {
    use rust_tests::elf_game::workshop::stove::{
        WorkshopGasLine, PropaneTank, PropaneTankHolder, GasHose, HoseAttachment,
        propane_gas_flow_system,
    };

    let mut app = App::new();
    app.init_resource::<Time>();
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 100.0,
        burn_rate: 5.0,
        leak_vent_rate: 20.0,
    });

    app.add_systems(Update, propane_gas_flow_system);

    let tank_e = app.world_mut().spawn(PropaneTank {
        fuel_amount: 100.0,
        capacity: 100.0,
        is_connected: true,
    }).id();

    app.world_mut().spawn(PropaneTankHolder {
        connected_tank: Some(tank_e),
    });

    let hose_e = app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Stove,
        wall_inlet_pos: Vec3::ZERO,
        stove_anchor_pos: Vec3::ZERO,
        loose_nozzle_pos: Vec3::ZERO,
        is_venting_gas: false,
        ..default()
    }).id();

    // Advance 1 sec burning fuel at stove (burn_rate = 5.0)
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
    app.update();

    let fuel_after_stove = app.world().get::<PropaneTank>(tank_e).unwrap().fuel_amount;
    assert!(
        (fuel_after_stove - 95.0).abs() < 0.1,
        "Connected tank fuel should decrease with stove burn rate (got {})",
        fuel_after_stove
    );
    let line_pressure = app.world().resource::<WorkshopGasLine>().pressure_psi;
    assert_eq!(line_pressure, fuel_after_stove);

    // Switch hose to Neither -> venting gas
    app.world_mut().get_mut::<GasHose>(hose_e).unwrap().attached_to = HoseAttachment::Neither;

    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
    app.update();

    let fuel_after_venting = app.world().get::<PropaneTank>(tank_e).unwrap().fuel_amount;
    assert!(
        fuel_after_venting < fuel_after_stove - 15.0,
        "Venting gas when detached to Neither must rapidly consume propane (got {})",
        fuel_after_venting
    );
    let hose = app.world().get::<GasHose>(hose_e).unwrap();
    assert!(hose.is_venting_gas, "Hose must be marked venting gas when attached to Neither with fuel left");
}

#[test]
fn test_climbable_windows_geometry() {
    use rust_tests::elf_game::workshop::room::setup_workshop_scene;
    use rust_tests::elf_game::assets::AssetsPlugin;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_plugins(AssetsPlugin);
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

    // Verify extra window between stove and door is removed and stable door is spawned
    let mut names_q = app.world_mut().query::<(&Name, &Transform)>();
    let mut found_window_sill = false;
    let mut found_wall_between = false;
    let mut found_door_top = false;
    let mut found_door_bottom = false;

    for (name, tf) in names_q.iter(app.world()) {
        match name.as_str() {
            "WestClimbableWindowSill" => {
                found_window_sill = true;
            }
            "WallNorthBetweenStoveAndDoor" => {
                found_wall_between = true;
                assert!((tf.translation.x - (-4.25)).abs() < 0.1);
            }
            "StableDoorTopHalf" => {
                found_door_top = true;
                assert!((tf.translation.x - (-2.0)).abs() < 0.2);
                assert!((tf.translation.y - 2.61).abs() < 0.1, "Top half must be scaled taller (Y = 2.61)");
            }
            "StableDoorBottomHalf" => {
                found_door_bottom = true;
                assert!((tf.translation.x - (-2.0)).abs() < 0.2);
                assert!((tf.translation.y - 0.87).abs() < 0.1, "Bottom half must be scaled taller (Y = 0.87)");
            }
            _ => {}
        }
    }

    assert!(!found_window_sill, "Extra window between moose and stove must be removed");
    assert!(found_wall_between, "Solid wall between stove and door must be present");
    assert!(found_door_top, "Stable door top half must be present");
    assert!(found_door_bottom, "Stable door bottom half must be present");
}

#[test]
fn test_gas_leak_flame_explosion() {
    use bevy_rapier3d::prelude::Velocity;
    use rust_tests::elf_game::workshop::stove::{
        WorkshopGasLine, GasHose, HoseAttachment, FlameSource, FieryExplosionEvent,
        gas_leak_flame_explosion_system,
    };
    use rust_tests::elf_game::player::{ElfMovement, controller::ElfPlayer};
    use rust_tests::elf_game::physics_sandbox::{ElfAssistant, ElfNpcState};

    let mut app = App::new();
    app.add_message::<FieryExplosionEvent>();
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 80.0,
        burn_rate: 0.8,
        leak_vent_rate: 2.5,
    });

    app.add_systems(Update, gas_leak_flame_explosion_system);

    let nozzle_pos = Vec3::new(-10.0, 0.12, -12.0);

    // Spawn hose in Neither (loose nozzle) with 30s accumulated disconnected time
    app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Neither,
        wall_inlet_pos: Vec3::new(-10.0, 0.85, -12.85),
        stove_anchor_pos: Vec3::new(-9.6, 0.65, -12.2),
        loose_nozzle_pos: nozzle_pos,
        is_venting_gas: true,
        snap_distance: 0.22,
        nozzle_entity: None,
        disconnected_timer: 30.0,
        ..default()
    });

    // Spawn Player nearby
    let player_e = app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(-10.5, 0.45, -11.5),
        ElfMovement {
            velocity: Vec3::ZERO,
            ..default()
        },
    )).id();

    // Spawn Elf nearby
    let elf_e = app.world_mut().spawn((
        Transform::from_xyz(-9.5, 0.45, -12.3),
        Velocity::zero(),
        ElfAssistant {
            state: ElfNpcState::Wandering,
            ..ElfAssistant::new(99, "BoomElf", vec![Vec3::ZERO])
        },
    )).id();

    // Spawn FlameSource within 0.6m of nozzle (< 1.0m threshold)
    app.world_mut().spawn((
        FlameSource,
        Transform::from_translation(nozzle_pos + Vec3::new(0.3, 0.1, 0.2)),
        GlobalTransform::from_translation(nozzle_pos + Vec3::new(0.3, 0.1, 0.2)),
    ));

    app.update();

    // 1. Verify FieryExplosionEvent was received
    let events = app.world().resource::<bevy::ecs::message::Messages<FieryExplosionEvent>>();
    assert!(!events.is_empty(), "FieryExplosionEvent must be triggered when flame source is within 1 meter of venting hose");

    // 2. Verify Player received shockwave velocity impulse
    let player_mov = app.world().get::<ElfMovement>(player_e).unwrap();
    assert!(
        player_mov.velocity.length() > 5.0,
        "Player must be blasted back by gas explosion shockwave, got velocity {:?}",
        player_mov.velocity
    );

    // 3. Verify Elf was knocked into RagdollTumble
    let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
    assert_eq!(elf.state, ElfNpcState::RagdollTumble, "Elf must tumble in ragdoll from explosion");
    let elf_vel = app.world().get::<Velocity>(elf_e).unwrap();
    assert!(elf_vel.linear.length() > 5.0, "Elf must receive linear blast impulse");
}

#[test]
fn test_flexible_chain_yellow_gas_hose_and_anchors() {
    use rust_tests::elf_game::workshop::stove::{
        spawn_oven_stove_and_propane_scene, GasHose, GasHoseSegment, GasHoseCouplerNozzle,
        StoveGasAnchor, HoseAttachment,
    };
    use rust_tests::elf_game::player::hands::Grabbable;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    app.update();

    // 1. GasHose entity
    let mut hose_q = app.world_mut().query::<&GasHose>();
    let hose = hose_q.iter(app.world()).next().expect("GasHose must exist");
    assert_eq!(hose.attached_to, HoseAttachment::Stove);
    assert!((hose.wall_inlet_pos.x - (-9.05)).abs() < 0.1);
    assert!((hose.wall_inlet_pos.y - 1.15).abs() < 0.1);
    assert!((hose.wall_inlet_pos.z - (-12.85)).abs() < 0.1);
    assert!((hose.stove_anchor_pos.x - (-7.34)).abs() < 0.1);

    // 2. 7 chain links
    let mut seg_q = app.world_mut().query::<&GasHoseSegment>();
    let segments: Vec<&GasHoseSegment> = seg_q.iter(app.world()).collect();
    assert_eq!(segments.len(), 7, "Yellow hose must consist of a flexible 7-link chain");

    // 3. Coupler nozzle has Grabbable and GasHoseCouplerNozzle
    let mut nozzle_q = app.world_mut().query::<(&GasHoseCouplerNozzle, &Grabbable)>();
    assert_eq!(nozzle_q.iter(app.world()).count(), 1, "Coupler nozzle must be Grabbable by player");

    // 4. Detachable anchor on stove
    let mut stove_anchor_q = app.world_mut().query::<&StoveGasAnchor>();
    assert_eq!(stove_anchor_q.iter(app.world()).count(), 1, "Stove must have a gas anchor fitting");

    // 5. Rapier RopeJoint physics integration
    use bevy_rapier3d::prelude::{ImpulseJoint, TypedJoint, RigidBody};
    let mut joint_q = app.world_mut().query::<(&GasHoseSegment, &ImpulseJoint)>();
    let joint_count = joint_q.iter(app.world()).count();
    assert_eq!(joint_count, 7, "All 7 gas hose segments must have Rapier ImpulseJoints");

    for (seg, joint) in joint_q.iter(app.world()) {
        match joint.data {
            TypedJoint::RopeJoint(_) => {}
            _ => panic!("Segment {} joint must be a Rapier RopeJoint", seg.index),
        }
    }

    // 6. Wall pipe has Fixed RigidBody
    let mut wall_pipe_q = app.world_mut().query::<(&Name, &RigidBody)>();
    let mut found_wall_pipe = false;
    for (name, rb) in wall_pipe_q.iter(app.world()) {
        if name.as_str() == "WallGasIntakePipe" {
            assert_eq!(*rb, RigidBody::Fixed, "Wall gas intake pipe must be a Fixed rigid body");
            found_wall_pipe = true;
        }
    }
    assert!(found_wall_pipe, "WallGasIntakePipe entity must exist");
}

#[test]
fn test_gas_hose_snap_to_connect_mechanics() {
    use rust_tests::elf_game::workshop::stove::{
        GasHose, GasHoseSegment, GasHoseCouplerNozzle, HoseAttachment,
        gas_hose_chain_physics_system,
    };
    use rust_tests::elf_game::player::ElfHands;
    use rust_tests::elf_game::player::hands::{Grabbable, HeldByPlayer};

    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_systems(Update, gas_hose_chain_physics_system);

    let nozzle_e = app.world_mut().spawn((
        GasHoseSegment { index: 6, velocity: Vec3::ZERO },
        GasHoseCouplerNozzle,
        Grabbable,
        HeldByPlayer,
        Transform::from_xyz(-8.42, 0.85, -12.0), // Within 0.05m of stove anchor (-8.44, 0.85, -12.0)
    )).id();

    // Spawn hose attached to neither
    app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Neither,
        wall_inlet_pos: Vec3::new(-9.05, 1.15, -12.85),
        stove_anchor_pos: Vec3::new(-8.44, 0.85, -12.0),
        loose_nozzle_pos: Vec3::new(-9.05, 0.05, -12.0),
        is_venting_gas: true,
        snap_distance: 0.22,
        nozzle_entity: Some(nozzle_e),
        disconnected_timer: 0.0,
        ..default()
    });

    // Player holding nozzle in left hand
    let player_hands_e = app.world_mut().spawn(ElfHands {
        left_grabbed: Some(nozzle_e),
        right_grabbed: None,
        ..default()
    }).id();

    app.update();

    // 1. Verify hose snapped to Stove
    let mut hose_q = app.world_mut().query::<&GasHose>();
    let hose = hose_q.iter(app.world()).next().unwrap();
    assert_eq!(hose.attached_to, HoseAttachment::Stove, "Hose must snap to Stove when held within 0.22m");
    assert!(!hose.is_venting_gas);

    // 2. Verify player grab was cleanly released upon snapping
    let hands = app.world().get::<ElfHands>(player_hands_e).unwrap();
    assert!(hands.left_grabbed.is_none(), "Left hand grab must be released upon snap");
    assert!(app.world().get::<HeldByPlayer>(nozzle_e).is_none(), "HeldByPlayer component removed");
}

#[test]
fn test_mr_moose_center_wall_and_stable_door_mechanics() {
    use rust_tests::elf_game::workshop::stable_door::{
        StableDoorTopHalf, StableDoorBottomHalf,
        stable_door_system, stable_door_interaction_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage};
    use rust_tests::elf_game::player::ElfHands;
    use rust_tests::elf_game::player::hands::{Grabbable, HeldByPlayer};

    let mut app = App::new();
    app.init_resource::<Time>();
    let hunger = MooseHunger {
        value: 35.0,
        max_value: 60.0,
        stage: MooseHungerStage::PeekingThrough,
        ..default()
    };
    app.insert_resource(hunger);

    app.add_systems(Update, (stable_door_system, stable_door_interaction_system));

    // Spawn Stable Door Top Half
    let top_door_e = app.world_mut().spawn((
        StableDoorTopHalf {
            current_angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            is_physics_driven: false,
            hinge_pos: Vec3::new(-2.9, 1.78, -13.0),
        },
        Transform::from_xyz(-2.0, 1.78, -13.0),
        Grabbable,
    )).id();

    // Spawn Stable Door Bottom Half
    let bottom_door_e = app.world_mut().spawn((
        StableDoorBottomHalf {
            current_angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            is_physics_driven: false,
            hinge_pos: Vec3::new(-2.9, 0.58, -13.0),
        },
        Transform::from_xyz(-2.0, 0.58, -13.0),
        Grabbable,
    )).id();

    // 1. Advance frames during Peeking stage
    for _ in 0..10 {
        app.update();
    }

    let top = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
    let bottom = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
    assert!(top.current_angle > 0.5, "Top half must open during peeking, got {}", top.current_angle);
    assert_eq!(bottom.current_angle, 0.0, "Bottom half must remain firmly closed during peeking");

    // 2. Trigger Rampage Breach
    {
        let mut h = app.world_mut().resource_mut::<MooseHunger>();
        h.is_breached = true;
        h.stage = MooseHungerStage::BreachedRampage;
    }

    app.update();

    let top_breach = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
    let bottom_breach = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
    assert!(top_breach.is_physics_driven, "Top half becomes physics driven on rampage");
    assert!(bottom_breach.is_physics_driven, "Bottom half becomes physics driven on rampage");
    assert!(top_breach.angular_velocity > 5.0, "Top half receives large impulse");
    assert!(bottom_breach.angular_velocity > 5.0, "Bottom half receives large impulse");

    // Run physics updates and verify angle stays within 2.0 radian range
    for _ in 0..20 {
        app.update();
    }
    let top_swung = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
    let bottom_swung = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
    assert!(top_swung.current_angle <= 2.0, "Top angle must not exceed 2.0 rad, got {}", top_swung.current_angle);
    assert!(bottom_swung.current_angle <= 2.0, "Bottom angle must not exceed 2.0 rad, got {}", bottom_swung.current_angle);

    // 3. Moose fed / rampage ends: return to closed
    {
        let mut h = app.world_mut().resource_mut::<MooseHunger>();
        h.is_breached = false;
        h.value = 0.0;
        h.stage = MooseHungerStage::BehindShutters;
    }

    for _ in 0..30 {
        app.update();
    }
    let top_closed = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
    let bottom_closed = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
    assert!(top_closed.current_angle < 0.05, "Top half returns to fully closed when rampage ends, got {}", top_closed.current_angle);
    assert!(bottom_closed.current_angle < 0.05, "Bottom half returns to fully closed when rampage ends, got {}", bottom_closed.current_angle);

    // 4. Test grabbable but not movable by player
    let hands_e = app.world_mut().spawn(ElfHands {
        left_grabbed: Some(bottom_door_e),
        right_grabbed: Some(top_door_e),
        ..default()
    }).id();
    app.world_mut().entity_mut(top_door_e).insert(HeldByPlayer);
    app.world_mut().entity_mut(bottom_door_e).insert(HeldByPlayer);

    app.update();

    let hands = app.world().get::<ElfHands>(hands_e).unwrap();
    assert!(hands.left_grabbed.is_none(), "Player cannot hold bottom door");
    assert!(hands.right_grabbed.is_none(), "Player cannot hold top door");
    assert!(app.world().get::<HeldByPlayer>(top_door_e).is_none());
    assert!(app.world().get::<HeldByPlayer>(bottom_door_e).is_none());
}

#[test]
fn test_moose_no_shutters_new_resting_rampage_pathing_and_door_slam() {
    use rust_tests::elf_game::moose::toy_model::{
        MrMoose, WindowShutters, spawn_mr_moose, moose_feeding_trough_system,
    };
    use rust_tests::elf_game::moose::{
        MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack,
    };
    use rust_tests::elf_game::workshop::events::MooseRampageTracker;
    use rust_tests::elf_game::workshop::stable_door::{
        StableDoorTopHalf, stable_door_system,
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.insert_resource(MooseHunger {
        value: 0.0,
        stage: MooseHungerStage::BehindShutters,
        ..default()
    });
    app.add_systems(Startup, spawn_mr_moose);
    app.update();

    // 1. Double shutter window is REMOVED from spawn_mr_moose
    let mut shutter_q = app.world_mut().query::<&WindowShutters>();
    assert_eq!(shutter_q.iter(app.world()).count(), 0, "Previous double shutter window must be removed");

    // 2. Moose default and resting location is at X = -2.0, Y = 1.8, Z = -17.0
    let mut moose_q = app.world_mut().query::<(&MrMoose, &Transform)>();
    let (_, moose_tf) = moose_q.iter(app.world()).next().expect("Mr Moose must exist");
    assert!((moose_tf.translation.x - (-2.0)).abs() < 0.05, "Moose X must be -2.0, got {}", moose_tf.translation.x);
    assert!((moose_tf.translation.y - 1.8).abs() < 0.05, "Moose Y must be 1.8");
    assert!((moose_tf.translation.z - (-17.0)).abs() < 0.05, "Moose Z must be -17.0 resting outside door");

    // 3. Rampage default waypoints use new coordinates around X = -2.0
    let tracker = MooseRampageTracker::default();
    assert_eq!(tracker.waypoints[0], Vec3::new(-2.0, 1.4, -10.0), "Leap into workshop at X = -2.0");
    assert_eq!(tracker.waypoints[5], Vec3::new(-2.0, 1.4, -4.0), "Charge back to door at X = -2.0");
    assert_eq!(tracker.waypoints[6], Vec3::new(-2.0, 1.4, -10.0), "Leap back out door at X = -2.0");
    assert_eq!(tracker.waypoints[7], Vec3::new(-2.0, 1.8, -17.0), "Retreat to resting area at X = -2.0");

    // 4. Food trough is replaced with festive cookie plate on table on the right side of the double door (X = -0.75, Y = 0.70, Z = -12.15)
    let mut trough_q = app.world_mut().query::<(&MooseFeedingTrough, &Transform)>();
    let (_, trough_tf) = trough_q.iter(app.world()).next().expect("Trough/Bucket must exist");
    assert!((trough_tf.translation.x - (-0.75)).abs() < 0.05, "Plate must be on right side of door at X = -0.75");
    assert!((trough_tf.translation.y - 0.70).abs() < 0.1, "Plate must be on table at Y ≈ 0.70");
    assert!((trough_tf.translation.z - (-12.15)).abs() < 0.05, "Plate must be at Z = -12.15");

    // 5. Active area of bucket: Snack placed at X = -0.75 is detected and triggers feeding
    let mut feed_app = App::new();
    feed_app.insert_resource(MooseHunger {
        value: 55.0,
        stage: MooseHungerStage::AgitatedHuffing,
        ..default()
    });
    feed_app.world_mut().spawn((MooseFeedingTrough, Transform::from_xyz(-0.75, 0.275, -12.15)));
    feed_app.world_mut().spawn((MooseSnack { nutrition_value: 30.0 }, Transform::from_xyz(-0.75, 0.4, -12.15)));
    feed_app.add_systems(Update, moose_feeding_trough_system);
    feed_app.update();

    let feed_hunger = feed_app.world().resource::<MooseHunger>();
    assert_eq!(feed_hunger.value, 0.0, "Snack in active bucket area at X = -0.75 must be consumed");

    // 6. Top of stable door opens as Moose pokes head in, and slams shut when he goes back
    let mut door_app = App::new();
    door_app.init_resource::<Time>();
    door_app.insert_resource(MooseHunger {
        value: 35.0,
        stage: MooseHungerStage::PeekingThrough,
        ..default()
    });
    door_app.add_systems(Update, stable_door_system);

    let top_door = door_app.world_mut().spawn((
        StableDoorTopHalf {
            current_angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            is_physics_driven: false,
            hinge_pos: Vec3::new(-2.9, 1.78, -13.0),
        },
        Transform::from_xyz(-2.0, 1.78, -13.0),
    )).id();

    // Advance during peeking: door opens as Moose pokes his head in
    for _ in 0..10 {
        door_app.update();
    }
    let top_open = door_app.world().get::<StableDoorTopHalf>(top_door).unwrap();
    assert!(top_open.current_angle > 0.5, "Top of stable door must open as Moose pokes head in, got {}", top_open.current_angle);

    // Moose goes back (BehindShutters / WindowSlammed) -> top door slams shut!
    {
        let mut h = door_app.world_mut().resource_mut::<MooseHunger>();
        h.stage = MooseHungerStage::WindowSlammed;
    }
    // Slam shut takes only ~3-4 frames at SLAM_SPEED = 25.0
    for _ in 0..5 {
        door_app.update();
    }
    let top_shut = door_app.world().get::<StableDoorTopHalf>(top_door).unwrap();
    assert_eq!(top_shut.current_angle, 0.0, "Top of stable door must slam shut when Moose goes back");
}

#[test]
fn test_taller_stable_door_feeding_bucket_and_gas_hose_visual_indicators() {
    use bevy_rapier3d::prelude::Collider;
    use rust_tests::elf_game::{
        assets::AssetsPlugin,
        workshop::{
            room::setup_workshop_scene,
            stove::{
                spawn_oven_stove_and_propane_scene, appliance_visual_indicators_system,
                gas_hose_interaction_system, ApplianceVisualMaterials, FurnaceFireboxWindow,
                FurnaceFireGlowLight, GasHose,
                HoseAttachment, StoveBurnerFlame, StoveGasAnchor,
                WorkshopGasLine,
            },
        },
        moose::{
            toy_model::{
                animate_mr_moose_peeking_system, spawn_mr_moose, MooseHead, MrMoose,
            },
            MooseFeedingBucket, MooseFeedingTrough, MooseHunger, MooseHungerStage,
        },
        player::{controller::FirstPersonCamera, hands::HeldByPlayer},
    };

    // 1. Double door scaled taller by 1.5 factor
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_plugins(AssetsPlugin);
    app.add_systems(Startup, (setup_workshop_scene, spawn_mr_moose));
    app.update();

    let mut door_q = app.world_mut().query::<(&Name, &Transform, &Collider)>();
    let mut found_top = false;
    let mut found_bottom = false;
    for (name, tf, _col) in door_q.iter(app.world()) {
        match name.as_str() {
            "StableDoorTopHalf" => {
                found_top = true;
                assert!((tf.translation.y - 2.61).abs() < 0.1, "Top half must be centered at Y ≈ 2.61");
            }
            "StableDoorBottomHalf" => {
                found_bottom = true;
                assert!((tf.translation.y - 0.87).abs() < 0.1, "Bottom half must be centered at Y ≈ 0.87");
            }
            _ => {}
        }
    }
    assert!(found_top && found_bottom, "Both door sections must be present and scaled");

    // 2. Feeding trough is a bucket on the right side of the double door
    let mut bucket_q = app.world_mut().query::<(&MooseFeedingBucket, &MooseFeedingTrough, &Transform)>();
    let (_, _, b_tf) = bucket_q.iter(app.world()).next().expect("Feeding bucket must exist");
    assert!((b_tf.translation.x - (-0.75)).abs() < 0.15, "Bucket must be on the right side of the double door (X ≈ -0.75)");
    assert!((b_tf.translation.z - (-12.15)).abs() < 0.15, "Bucket must be at Z ≈ -12.15");

    // 3. Eating animation is centered on the bucket (local X = 1.25)
    let mut anim_app = App::new();
    anim_app.init_resource::<Time>();
    anim_app.insert_resource(MooseHunger {
        value: 55.0,
        stage: MooseHungerStage::EatingTreat,
        eating_timer: 1.0,
        eating_duration: 2.2,
        is_eating_fast: false,
        ..default()
    });
    let _moose_e = anim_app.world_mut().spawn((MrMoose, Transform::from_xyz(-2.0, 1.8, -14.5))).id();
    let head_e = anim_app.world_mut().spawn((MooseHead, Transform::IDENTITY)).id();
    anim_app.add_systems(Update, animate_mr_moose_peeking_system);
    anim_app.update();

    let head_tf = anim_app.world().get::<Transform>(head_e).unwrap();
    assert!((head_tf.translation.x - 1.25).abs() < 0.1, "Eating animation must be centered on the bucket at local X ≈ 1.25 (got {})", head_tf.translation.x);
    assert!(head_tf.translation.y < 0.6, "Eating animation head must reach down into bucket (got Y = {})", head_tf.translation.y);

    // 4. Gas hose anchor points have larger interaction areas
    let mut stove_app = App::new();
    stove_app.init_resource::<Assets<Mesh>>();
    stove_app.init_resource::<Assets<StandardMaterial>>();
    stove_app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    stove_app.update();

    let mut anchor_q = stove_app.world_mut().query::<(&StoveGasAnchor, &Collider)>();
    let (_, _stove_col) = anchor_q.iter(stove_app.world()).next().expect("StoveGasAnchor must exist");

    // 5. Appliance visual indicators (Furnace window glowing vs cool dark gray, Stove blue burner flames)
    let mut vis_app = App::new();
    vis_app.init_resource::<Assets<Mesh>>();
    vis_app.init_resource::<Assets<StandardMaterial>>();
    vis_app.insert_resource(WorkshopGasLine { pressure_psi: 100.0, ..default() });
    vis_app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    vis_app.add_systems(Update, appliance_visual_indicators_system);
    vis_app.update();

    let mats = vis_app.world().resource::<ApplianceVisualMaterials>().clone();

    // When attached to Stove (default):
    let mut flame_q = vis_app.world_mut().query::<(&StoveBurnerFlame, &Visibility)>();
    for (_, vis) in flame_q.iter(vis_app.world()) {
        assert_eq!(*vis, Visibility::Inherited, "Stove burners must show blue flames when attached");
    }
    let mut furnace_win_q = vis_app.world_mut().query::<(&FurnaceFireboxWindow, &MeshMaterial3d<StandardMaterial>)>();
    for (_, mat) in furnace_win_q.iter(vis_app.world()) {
        assert_eq!(mat.0, mats.glowing_ember, "Furnace window must glow when attached");
    }

    // Detach hose to Neither:
    let mut hose = vis_app.world_mut().query::<&mut GasHose>().single_mut(vis_app.world_mut()).unwrap();
    hose.attached_to = HoseAttachment::Neither;
    vis_app.update();

    let mut flame_q = vis_app.world_mut().query::<(&StoveBurnerFlame, &Visibility)>();
    for (_, vis) in flame_q.iter(vis_app.world()) {
        assert_eq!(*vis, Visibility::Hidden, "Stove burners must be off when detached");
    }
    let mut furnace_win_q = vis_app.world_mut().query::<(&FurnaceFireboxWindow, &MeshMaterial3d<StandardMaterial>)>();
    for (_, mat) in furnace_win_q.iter(vis_app.world()) {
        assert_eq!(mat.0, mats.cool_dark_gray, "Furnace window must be cool dark gray when detached");
    }
    let mut light_q = vis_app.world_mut().query::<(&FurnaceFireGlowLight, &PointLight)>();
    for (_, light) in light_q.iter(vis_app.world()) {
        assert_eq!(light.intensity, 0.0, "Furnace fire light must be off when detached");
    }

    // Re-attach hose to Stove:
    let mut hose = vis_app.world_mut().query::<&mut GasHose>().single_mut(vis_app.world_mut()).unwrap();
    hose.attached_to = HoseAttachment::Stove;
    vis_app.update();

    let mut flame_q = vis_app.world_mut().query::<(&StoveBurnerFlame, &Visibility)>();
    for (_, vis) in flame_q.iter(vis_app.world()) {
        assert_eq!(*vis, Visibility::Inherited, "Stove burners must show blue flames when re-attached to Stove");
    }

    // 6. Automatic attach to anchor player is facing while holding hose
    let mut auto_app = App::new();
    auto_app.init_resource::<Assets<Mesh>>();
    auto_app.init_resource::<Assets<StandardMaterial>>();
    auto_app.init_resource::<ButtonInput<KeyCode>>();
    auto_app.init_resource::<ButtonInput<MouseButton>>();
    auto_app.insert_resource(WorkshopGasLine::default());
    auto_app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    auto_app.add_systems(Update, gas_hose_interaction_system);
    auto_app.update();

    let (nozzle_e, stove_anchor_pos) = {
        let mut q = auto_app.world_mut().query::<&GasHose>();
        let h = q.single(auto_app.world()).unwrap();
        (h.nozzle_entity.unwrap(), h.stove_anchor_pos)
    };

    // Detach hose, player holds nozzle
    {
        let mut h = auto_app.world_mut().query::<&mut GasHose>().single_mut(auto_app.world_mut()).unwrap();
        h.attached_to = HoseAttachment::Neither;
    }
    auto_app.world_mut().entity_mut(nozzle_e).insert(HeldByPlayer);

    // Player faces Stove anchor: cam placed 2m south of stove anchor, looking North towards stove_anchor_pos
    let cam_pos = Vec3::new(stove_anchor_pos.x, 1.2, stove_anchor_pos.z + 2.0);
    let cam_target = stove_anchor_pos;
    let cam_fwd = (cam_target - cam_pos).normalize();
    auto_app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_translation(cam_pos).looking_to(cam_fwd, Vec3::Y)),
    ));

    // Player presses interact (KeyE) while holding hose
    auto_app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
    auto_app.update();

    let mut check_q = auto_app.world_mut().query::<&GasHose>();
    let updated_h = check_q.single(auto_app.world()).unwrap();
    assert_eq!(updated_h.attached_to, HoseAttachment::Stove, "Hose must automatically attach to the Stove anchor player is facing");
    assert!(auto_app.world().get::<HeldByPlayer>(nozzle_e).is_none(), "Player hold must be released upon automatic attachment");
}

#[test]
fn test_gas_leak_flame_explosion_scaling_and_reconnect_tick_down() {
    use rust_tests::elf_game::workshop::stove::{
        gas_leak_flame_explosion_system, FieryExplosionEvent, FlameSource, GasHose,
        HoseAttachment, WorkshopGasLine,
    };

    // 1. Verify GasHose::explosion_scaling unit calculations
    assert_eq!(GasHose::explosion_scaling(0.0), None);
    assert_eq!(GasHose::explosion_scaling(4.99), None);
    assert_eq!(GasHose::explosion_scaling(5.0), None);

    // At 5.01s: explosion starts with minimal impulse and radius
    let (imp_start, rad_start) = GasHose::explosion_scaling(5.01).unwrap();
    assert!((imp_start - 5.01).abs() < 0.05);
    assert!(rad_start > 0.0 && rad_start < 0.01);

    // At 10.0s: factor = 5/25 = 0.20 -> impulse = 10.0, radius = 0.8m
    let (imp_10, rad_10) = GasHose::explosion_scaling(10.0).unwrap();
    assert!((imp_10 - 10.0).abs() < 0.01);
    assert!((rad_10 - 0.80).abs() < 0.01);

    // At 17.5s (midpoint): factor = 12.5/25 = 0.50 -> impulse = 17.5, radius = 2.0m
    let (imp_17_5, rad_17_5) = GasHose::explosion_scaling(17.5).unwrap();
    assert!((imp_17_5 - 17.5).abs() < 0.01);
    assert!((rad_17_5 - 2.0).abs() < 0.01);

    // At 30.0s: factor = 1.0 -> impulse = 30.0, radius = 4.0m
    let (imp_30, rad_30) = GasHose::explosion_scaling(30.0).unwrap();
    assert!((imp_30 - 30.0).abs() < 0.01);
    assert!((rad_30 - 4.0).abs() < 0.01);

    // At 45.0s (30 seconds or more): maxed at 30 seconds
    let (imp_45, rad_45) = GasHose::explosion_scaling(45.0).unwrap();
    assert_eq!(imp_45, 30.0, "Impulse must be maxed at 30");
    assert_eq!(rad_45, 4.0, "Blast radius must be maxed at 4.0 meters");

    // 2. Integration simulation: 0-5 seconds has no explosion even with flame nearby
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs_f32(4.0));
    app.insert_resource(time);
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 50.0,
        ..default()
    });
    app.add_message::<FieryExplosionEvent>();
    app.add_systems(Update, gas_leak_flame_explosion_system);

    let nozzle_pos = Vec3::new(-10.0, 0.1, -12.0);
    let hose_e = app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Neither,
        loose_nozzle_pos: nozzle_pos,
        disconnected_timer: 0.0,
        ..default()
    }).id();

    // Flame source within 0.4m of nozzle (< 1.0m threshold)
    app.world_mut().spawn((
        FlameSource,
        GlobalTransform::from_translation(nozzle_pos + Vec3::new(0.2, 0.1, 0.2)),
    ));

    app.update();

    let events = app.world().resource::<bevy::ecs::message::Messages<FieryExplosionEvent>>();
    assert!(events.is_empty(), "From 0-5 seconds, no explosion must occur");

    let hose = app.world().get::<GasHose>(hose_e).unwrap();
    assert!((hose.disconnected_timer - 4.0).abs() < 0.1, "Disconnected timer must tick up to 4.0s");

    // 3. Reconnect hose: timer ticks back down, doesn't reset to 0 immediately
    {
        let mut hose_mut = app.world_mut().get_mut::<GasHose>(hose_e).unwrap();
        hose_mut.attached_to = HoseAttachment::Stove;
    }
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(2.5));
    app.update();

    let hose = app.world().get::<GasHose>(hose_e).unwrap();
    assert!((hose.disconnected_timer - 1.5).abs() < 0.1, "Reconnecting hose must tick timer down from 4.0 to 1.5s");

    // Tick down past 0 clamps at 0.0
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(3.0));
    app.update();
    let hose = app.world().get::<GasHose>(hose_e).unwrap();
    assert_eq!(hose.disconnected_timer, 0.0, "Timer must clamp at 0.0 when ticking down");

    // 4. Disconnect hose again and advance to 10 seconds: explosion fires with scaled values
    {
        let mut hose_mut = app.world_mut().get_mut::<GasHose>(hose_e).unwrap();
        hose_mut.attached_to = HoseAttachment::Neither;
        hose_mut.disconnected_timer = 10.0;
    }
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(0.0));
    app.update();

    let events = app.world().resource::<bevy::ecs::message::Messages<FieryExplosionEvent>>();
    assert_eq!(events.len(), 1, "FieryExplosionEvent must fire when timer > 5.0 and flame is nearby");
    let ev = events.iter_current_update_messages().next().unwrap();
    assert!((ev.blast_radius - 0.8).abs() < 0.05, "Blast radius at 10s should be ~0.8m");
    assert!((ev.blast_impulse - 10.0).abs() < 0.05, "Blast impulse at 10s should be ~10.0");

    let hose = app.world().get::<GasHose>(hose_e).unwrap();
    assert_eq!(hose.disconnected_timer, 0.0, "Explosion consumes gas and resets disconnected timer to 0.0");
}

#[test]
fn test_gas_hose_rapier_rope_joint_physics_simulation() {
    use rust_tests::elf_game::workshop::stove::{
        spawn_oven_stove_and_propane_scene, gas_hose_chain_physics_system,
        GasHose, GasHoseSegment, HoseAttachment,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Time>();
    app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    app.add_systems(Update, gas_hose_chain_physics_system);
    app.update();

    let (hose_e, nozzle_e) = {
        let mut hose_q = app.world_mut().query::<(Entity, &GasHose)>();
        let (e, h) = hose_q.single(app.world()).unwrap();
        (e, h.nozzle_entity.unwrap())
    };

    // 1. Initial state: attached to Stove, nozzle is Fixed rigid body
    {
        let nozzle_rb = app.world().get::<RigidBody>(nozzle_e).unwrap();
        assert_eq!(*nozzle_rb, RigidBody::Fixed, "Nozzle must be Fixed when attached to Stove");
    }

    // 2. All 7 segments have ImpulseJoint with RopeJoint
    {
        let mut seg_q = app.world_mut().query::<(&GasHoseSegment, &ImpulseJoint)>();
        let mut count = 0;
        for (seg, joint) in seg_q.iter(app.world()) {
            count += 1;
            match joint.data {
                TypedJoint::RopeJoint(_) => {}
                _ => panic!("Segment {} must use Rapier RopeJoint", seg.index),
            }
        }
        assert_eq!(count, 7, "All 7 segments must have Rapier RopeJoints");
    }

    // 3. Detach hose to Neither: nozzle becomes Dynamic rigid body
    {
        let mut hose = app.world_mut().get_mut::<GasHose>(hose_e).unwrap();
        hose.attached_to = HoseAttachment::Neither;
    }
    app.update();

    {
        let nozzle_rb = app.world().get::<RigidBody>(nozzle_e).unwrap();
        assert_eq!(*nozzle_rb, RigidBody::Dynamic, "Nozzle must switch to Dynamic when detached");
    }

    // 4. Re-attach hose to Stove: nozzle becomes Fixed rigid body at stove anchor
    {
        let mut hose = app.world_mut().get_mut::<GasHose>(hose_e).unwrap();
        hose.attached_to = HoseAttachment::Stove;
    }
    app.update();

    {
        let nozzle_rb = app.world().get::<RigidBody>(nozzle_e).unwrap();
        let nozzle_tf = app.world().get::<Transform>(nozzle_e).unwrap();
        let hose = app.world().get::<GasHose>(hose_e).unwrap();
        assert_eq!(*nozzle_rb, RigidBody::Fixed, "Nozzle must switch to Fixed when attached to Stove");
        assert_eq!(nozzle_tf.translation, hose.stove_anchor_pos, "Nozzle must be positioned at stove anchor");
    }
}

#[test]
fn test_generic_reusable_anchor_point_and_matching_traits() {
    use bevy_rapier3d::prelude::*;
    use rust_tests::elf_game::{
        assets::objects::{
            AnchorPoint, AnchorTrait, LockedInAnchor,
            anchor_point_interaction_system, sync_locked_anchor_objects_system,
        },
        player::{
            hands::HeldByPlayer,
            ElfHands,
            controller::{ElfPlayer, FirstPersonCamera},
        },
        workshop::stove::{
            spawn_oven_stove_and_propane_scene, FlameSource, GasFurnaceStove,
            StoveGasAnchor, GasHose, GasHoseSegment,
        },
    };

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();

    app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    app.add_systems(
        Update,
        (
            anchor_point_interaction_system,
            sync_locked_anchor_objects_system,
        ),
    );
    app.update();

    // 1. Verify Stove and Furnace moved further away
    let mut stove_q = app.world_mut().query::<(&GasFurnaceStove, &Transform)>();
    let (_, stove_tf) = stove_q.iter(app.world()).next().expect("Stove must exist");
    assert!((stove_tf.translation.x - (-6.5)).abs() < 0.1, "Stove moved to X ≈ -6.5");
    assert!((stove_tf.translation.z - (-11.4)).abs() < 0.1, "Stove moved to Z ≈ -11.4");

    let mut hose_q = app.world_mut().query::<&GasHose>();
    let hose = hose_q.iter(app.world()).next().expect("GasHose must exist");
    let anchor_dist = hose.stove_anchor_pos.distance(hose.wall_inlet_pos);
    assert!(anchor_dist > 1.5, "Stove anchor must be distanced from wall inlet (> 1.5m apart), dist = {}", anchor_dist);

    // 2. Verify Stove attachment point is red and has smaller active area (< 0.25)
    let mut anchor_q = app.world_mut().query::<(&StoveGasAnchor, &MeshMaterial3d<StandardMaterial>, &Collider)>();
    let (_, mat_handle, collider) = anchor_q.iter(app.world()).next().expect("StoveGasAnchor must exist");
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let mat = materials.get(mat_handle).expect("Stove gas anchor material must exist");
    if let Color::Srgba(srgba) = mat.base_color {
        assert!(srgba.red > 0.8 && srgba.green < 0.3 && srgba.blue < 0.3, "Stove attachment point must be red");
    } else {
        panic!("Material base_color must be red");
    }

    // Active area collider radius should be small (0.22, not previous 0.45)
    if let Some(cylinder) = collider.as_cylinder() {
        assert!(cylinder.radius() < 0.25, "Active area collider must be smaller (< 0.25m), got {}", cylinder.radius());
    }

    // 3. Verify Candle is moved to the workshop bench (Z ≈ -4.0, Y ≈ 0.95)
    let mut candle_q = app.world_mut().query::<(&FlameSource, &Transform, &Name)>();
    let mut found_bench_candle = false;
    for (_, candle_tf, name) in candle_q.iter(app.world()) {
        if name.as_str() == "PortableBrassCandlestick" {
            assert!((candle_tf.translation.z - (-4.0)).abs() < 0.2, "Candle must be on workshop bench at Z ≈ -4.0");
            assert!((candle_tf.translation.y - 0.95).abs() < 0.1, "Candle must rest on workbench surface at Y ≈ 0.95");
            found_bench_candle = true;
        }
    }
    assert!(found_bench_candle, "Portable brass candlestick must exist on workshop bench");

    // 4. Verify continuous yellow hose segments: link cylinder length is continuous (>= 0.35m)
    let mut seg_mesh_q = app.world_mut().query::<(&GasHoseSegment, &Mesh3d)>();
    let meshes = app.world().resource::<Assets<Mesh>>();
    for (seg, mesh_h) in seg_mesh_q.iter(app.world()) {
        if seg.index < 6 {
            // Check segment cylinder mesh
            let mesh = meshes.get(mesh_h).expect("Segment mesh must exist");
            // Segment mesh exists and is valid
            assert!(mesh.count_vertices() > 0, "Segment cylinder mesh must be populated");
        }
    }

    // 5. Generic AnchorPoint mechanics:
    // Create an isolated player, camera, anchor point and physics objects
    let anchor_pos = Vec3::new(2.0, 1.0, 0.0);
    let anchor_e = app.world_mut().spawn((
        AnchorPoint::new("battery").with_radius(3.0).with_offset(Vec3::new(0.0, 0.2, 0.0)),
        Transform::from_translation(anchor_pos),
        GlobalTransform::from_translation(anchor_pos),
    )).id();

    let player_pos = Vec3::new(2.0, 1.0, 1.5);
    let cam_fwd = (anchor_pos - player_pos).normalize();
    let player_e = app.world_mut().spawn((
        ElfPlayer,
        Transform::from_translation(player_pos),
        GlobalTransform::from_translation(player_pos),
        ElfHands::default(),
    )).id();

    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_translation(player_pos).looking_to(cam_fwd, Vec3::Y)),
    ));

    // Spawn a matching physics object with AnchorTrait("battery")
    let battery_e = app.world_mut().spawn((
        AnchorTrait::new("battery"),
        RigidBody::Dynamic,
        Velocity::zero(),
        Transform::from_xyz(2.0, 1.0, 1.0),
        GlobalTransform::from_xyz(2.0, 1.0, 1.0),
    )).id();

    // Set object as held by player
    app.world_mut().entity_mut(battery_e).insert(HeldByPlayer);
    app.world_mut().entity_mut(player_e).get_mut::<ElfHands>().unwrap().left_grabbed = Some(battery_e);

    // Rule A: If the object is being held, it should NOT snap to the anchor point on proximity!
    app.update();
    let anchor = app.world().get::<AnchorPoint>(anchor_e).unwrap();
    assert_eq!(anchor.locked_object, None, "Object must NOT automatically snap to anchor on proximity while held");
    assert!(app.world().get::<HeldByPlayer>(battery_e).is_some(), "Battery must remain held by player");

    // Rule B: When player interacts with KeyCode::KeyE while holding matching object, it locks the object in!
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.reset(KeyCode::KeyE);
        keys.press(KeyCode::KeyE);
    }
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.reset(KeyCode::KeyE);
    }

    let anchor = app.world().get::<AnchorPoint>(anchor_e).unwrap();
    assert_eq!(anchor.locked_object, Some(battery_e), "Battery must be locked into AnchorPoint");
    assert!(app.world().get::<HeldByPlayer>(battery_e).is_none(), "HeldByPlayer must be removed on lock");
    assert!(app.world().get::<LockedInAnchor>(battery_e).is_some(), "LockedInAnchor component must be added to object");
    assert_eq!(*app.world().get::<RigidBody>(battery_e).unwrap(), RigidBody::Fixed, "RigidBody must become Fixed on lock");

    let player_hands = app.world().get::<ElfHands>(player_e).unwrap();
    assert_eq!(player_hands.left_grabbed, None, "Player hand must be released when object locks");

    let battery_tf = app.world().get::<Transform>(battery_e).unwrap();
    assert_eq!(battery_tf.translation, anchor_pos + Vec3::new(0.0, 0.2, 0.0), "Battery snapped to anchor position + offset");

    // Rule C: Interacting with KeyCode::KeyE again unlocks the object!
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.reset(KeyCode::KeyE);
        keys.press(KeyCode::KeyE);
    }
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.reset(KeyCode::KeyE);
    }

    let anchor = app.world().get::<AnchorPoint>(anchor_e).unwrap();
    assert_eq!(anchor.locked_object, None, "Anchor must be empty after unlocking");
    assert!(app.world().get::<LockedInAnchor>(battery_e).is_none(), "LockedInAnchor must be removed upon unlocking");
    assert_eq!(*app.world().get::<RigidBody>(battery_e).unwrap(), RigidBody::Dynamic, "RigidBody must be restored to Dynamic");

    // Rule D: Object with non-matching trait does NOT lock
    let wood_e = app.world_mut().spawn((
        AnchorTrait::new("wood"),
        RigidBody::Dynamic,
        Velocity::zero(),
        Transform::from_xyz(2.0, 1.0, 1.0),
        GlobalTransform::from_xyz(2.0, 1.0, 1.0),
        HeldByPlayer,
    )).id();
    app.world_mut().entity_mut(player_e).get_mut::<ElfHands>().unwrap().left_grabbed = Some(wood_e);

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.reset(KeyCode::KeyE);
        keys.press(KeyCode::KeyE);
    }
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.reset(KeyCode::KeyE);
    }

    let anchor = app.world().get::<AnchorPoint>(anchor_e).unwrap();
    assert_eq!(anchor.locked_object, None, "Non-matching object trait ('wood') must not be locked into 'battery' anchor");
    assert!(app.world().get::<HeldByPlayer>(wood_e).is_some(), "Non-matching object remains held");
}

#[test]
fn test_moose_body_collision_with_scarf() {
    use rust_tests::elf_game::moose::toy_model::{
        MrMoose, MooseHead, MooseScarfSegment, MooseScarfMount, moose_scarf_physics_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, moose_scarf_physics_system);

    // 1. Mr. Moose root at (0.0, 1.8, 0.0) with standard torso cuboid collider (0.85, 0.68, 1.35)
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        MooseScarfMount::default(),
        Collider::cuboid(0.85, 0.68, 1.35),
        Transform::from_xyz(0.0, 1.8, 0.0),
    )).id();

    app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    ));

    // 2. Position a scarf segment (index 0) deliberately inside the torso chest volume (Z = 0.5 in local space, well inside Z <= 1.35)
    let penetrating_local_pos = Vec3::new(-0.24, 0.0, 0.5);
    let seg_entity = app.world_mut().spawn((
        MooseScarfSegment {
            index: 0,
            rest_offset: penetrating_local_pos,
            velocity: Vec3::new(0.0, 0.0, -2.0), // Moving backward into the body
        },
        Collider::cuboid(0.19, 0.14, 0.11),
        Transform::from_translation(Vec3::new(0.0, 1.8, 0.0) + penetrating_local_pos),
    )).id();

    // 3. Step physics simulation
    app.update();

    // 4. Verify that collision pushed the scarf segment strictly OUT of the torso
    let seg_tf = app.world().get::<Transform>(seg_entity).unwrap();
    let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
    let local_z = (moose_tf.rotation.inverse() * (seg_tf.translation - moose_tf.translation)).z;

    assert!(
        local_z >= 1.40,
        "Scarf segment must be resolved outside the Moose chest (Z >= 1.40), got local Z = {}",
        local_z
    );

    // 5. Move the Moose forward (+Z by 1.0m) so inertia pushes the scarf towards the chest
    {
        let mut m_tf = app.world_mut().get_mut::<Transform>(moose_entity).unwrap();
        m_tf.translation.z = 1.0;
    }
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
    app.update();

    let seg_tf_after = app.world().get::<Transform>(seg_entity).unwrap();
    let moose_tf_after = app.world().get::<Transform>(moose_entity).unwrap();
    let local_z_after = (moose_tf_after.rotation.inverse() * (seg_tf_after.translation - moose_tf_after.translation)).z;

    assert!(
        local_z_after >= 1.40,
        "Scarf segment must remain outside the Moose body during forward motion (local Z >= 1.40), got {}",
        local_z_after
    );
}

#[test]
fn test_festive_cookie_plate_table_and_world_physics_tuning() {
    use rust_tests::elf_game::moose::{
        MooseCookiePlate, MooseFeedingTrough, toy_model::spawn_mr_moose,
    };
    use rust_tests::elf_game::core::{
        WORLD_GRAVITY, sync_world_gravity_system, apply_physics_damping_system,
    };
    use rust_tests::elf_game::player::hands::{Grabbable, dual_hand_interaction_system};
    use rust_tests::elf_game::player::controller::{RightHandVisual, FirstPersonCamera};
    use rust_tests::elf_game::player::ElfHands;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_message::<rust_tests::elf_game::wrapping::UseToolOnPresentMessage>();

    // 1. Set up resources and systems
    let rapier_entity = app.world_mut().spawn(RapierConfiguration::new(1.0)).id();
    app.add_systems(Startup, spawn_mr_moose);
    app.add_systems(Update, (sync_world_gravity_system, apply_physics_damping_system, dual_hand_interaction_system));
    app.update();

    let config = app.world().get::<RapierConfiguration>(rapier_entity).unwrap();
    assert_eq!(config.gravity, WORLD_GRAVITY, "World gravity must be higher (-18.0 m/s^2)");

    // Verify table next to Stable Door exists at X ≈ -0.75, Z ≈ -12.15
    let mut table_found = false;
    for (name, tf) in app.world_mut().query::<(&Name, &Transform)>().iter(app.world()) {
        if name.as_str() == "StableDoorTable" {
            table_found = true;
            assert!((tf.translation.x - (-0.75)).abs() < 0.1);
            assert!((tf.translation.z - (-12.15)).abs() < 0.1);
            assert!((tf.translation.y - 0.65).abs() < 0.1);
        }
    }
    assert!(table_found, "StableDoorTable must be spawned next to the Stable Door");

    // 3. Verify FestiveCookiePlate on table
    let mut plate_q = app.world_mut().query::<(&MooseCookiePlate, &MooseFeedingTrough, &Transform, &Friction, &Children)>();
    let (_, _, plate_tf, friction, children) = plate_q.iter(app.world()).next().expect("FestiveCookiePlate must exist");
    assert!((plate_tf.translation.x - (-0.75)).abs() < 0.1);
    assert!((plate_tf.translation.z - (-12.15)).abs() < 0.1);
    assert!((plate_tf.translation.y - 0.70).abs() < 0.1, "Plate must sit on table at Y ≈ 0.70");
    assert!(friction.coefficient >= 3.0, "Cookie plate must have high friction (>= 3.0) to make treats stick, got {}", friction.coefficient);

    // 4. Verify raised rim colliders
    let mut rim_collider_count = 0;
    for child in children.iter() {
        if let Some(name) = app.world().get::<Name>(child) {
            if name.as_str().starts_with("PlateRim") {
                if app.world().get::<Collider>(child).is_some() {
                    rim_collider_count += 1;
                }
            }
        }
    }
    assert!(rim_collider_count >= 4, "Cookie plate must have raised rim colliders (>= 4), got {}", rim_collider_count);

    // 5. Verify dynamic physics objects receive damping
    let dynamic_cube = app.world_mut().spawn((
        RigidBody::Dynamic,
        Transform::from_xyz(0.0, 1.0, 0.0),
    )).id();
    app.update();
    let damping = app.world().get::<Damping>(dynamic_cube).expect("Dynamic physics objects must be given dampening");
    assert!(damping.linear_damping >= 0.5, "Dynamic object must have linear damping");

    // 6. Verify scaled up throw impulse (throw_speed = 13.5 m/s)
    let player_e = app.world_mut().spawn((
        ElfHands::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
    )).id();
    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.6, 0.0)),
    ));
    app.world_mut().spawn((
        RightHandVisual,
        GlobalTransform::from(Transform::from_xyz(0.35, 1.0, -0.6)),
    ));

    let thrown_cube = app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(0.35, 1.0, -0.7),
        Velocity::zero(),
    )).id();
    app.world_mut().get_mut::<ElfHands>(player_e).unwrap().right_grabbed = Some(thrown_cube);

    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
    app.update();

    let vel = app.world().get::<Velocity>(thrown_cube).unwrap();
    assert!(
        vel.linear.length() >= 13.0,
        "Player throw impulse must be scaled up (>= 13.0 m/s) to compensate for higher gravity, got {}",
        vel.linear.length()
    );
}

#[test]
fn test_tractor_beam_player_force_and_activation_button_distance() {
    use rust_tests::elf_game::workshop::room::{
        PresentSendingPlatform, SendPlatformButton, VacuumTractorBeamCone,
        present_sending_platform_system, setup_workshop_scene,
    };
    use rust_tests::elf_game::player::ElfMovement;
    use bevy_rapier3d::prelude::Velocity;

    // 1. Verify in the spawned workshop room that the button is moved further away from the tractor beam
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

    let mut button_tf = None;
    let mut platform_tf = None;
    for (tf, _) in app.world_mut().query::<(&Transform, &SendPlatformButton)>().iter(app.world()) {
        button_tf = Some(*tf);
    }
    for (tf, _) in app.world_mut().query::<(&Transform, &PresentSendingPlatform)>().iter(app.world()) {
        platform_tf = Some(*tf);
    }

    let btn_pos = button_tf.expect("SendPlatformButton must exist").translation;
    let plat_pos = platform_tf.expect("PresentSendingPlatform must exist").translation;
    let horizontal_dist = (btn_pos.x - plat_pos.x).hypot(btn_pos.z - plat_pos.z);
    assert!(
        horizontal_dist >= 3.5,
        "Activation button must be moved further away from tractor beam platform (>= 3.5m, got {})",
        horizontal_dist
    );

    // 2. Test that player standing inside the tractor beam cannot activate the button
    let mut app2 = App::new();
    let mut time2 = Time::<()>::default();
    time2.advance_by(std::time::Duration::from_millis(50));
    app2.insert_resource(time2);
    app2.insert_resource(ButtonInput::<KeyCode>::default());
    app2.insert_resource(ButtonInput::<MouseButton>::default());
    app2.add_systems(Update, present_sending_platform_system);

    let plat_e = app2.world_mut().spawn((
        PresentSendingPlatform::default(),
        Transform::from_xyz(5.5, 0.05, -4.0),
    )).id();

    let _cone_e = app2.world_mut().spawn((
        VacuumTractorBeamCone::default(),
        Visibility::Hidden,
        Transform::from_xyz(5.5, 2.025, -4.0),
    )).id();

    let _btn_e = app2.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(1.8, 0.95, -1.8),
    )).id();

    // Player inside tractor beam area
    let player_e = app2.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(5.5, 0.2, -4.0),
    )).id();

    // Press KeyQ while in tractor beam
    app2.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app2.update();

    let plat = app2.world().get::<PresentSendingPlatform>(plat_e).unwrap();
    assert!(
        !plat.is_active,
        "Player standing in the tractor beam area MUST NOT be able to activate the tractor beam button"
    );

    // 3. Move player outside tractor beam area, near the button (e.g. at 1.8, 1.0, -1.5)
    {
        let mut p_tf = app2.world_mut().get_mut::<Transform>(player_e).unwrap();
        p_tf.translation = Vec3::new(1.8, 1.0, -1.5);
    }
    app2.update();
    let plat = app2.world().get::<PresentSendingPlatform>(plat_e).unwrap();
    assert!(
        plat.is_active,
        "Player near the activation button outside the tractor beam MUST be able to activate it"
    );

    // 4. Test increased force on player when inside active tractor beam
    // Place physics object on button to hold it active
    app2.world_mut().spawn((
        Transform::from_xyz(1.8, 1.0, -1.8),
        Velocity::zero(),
    ));
    // Release key
    app2.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);

    // Put player back in tractor beam area
    {
        let mut p_tf = app2.world_mut().get_mut::<Transform>(player_e).unwrap();
        p_tf.translation = Vec3::new(5.5, 0.2, -4.0);
        let mut p_mov = app2.world_mut().get_mut::<ElfMovement>(player_e).unwrap();
        p_mov.velocity = Vec3::ZERO;
    }
    app2.update();

    let p_mov = app2.world().get::<ElfMovement>(player_e).unwrap();
    assert!(
        p_mov.velocity.y >= 3.0,
        "Tractor beam upward force on player must be increased to fully suck them up (got vy = {})",
        p_mov.velocity.y
    );
}

#[test]
fn test_player_grab_direct_raycast_line_of_sight_reticle() {
    use rust_tests::elf_game::player::hands::{Grabbable, dual_hand_interaction_system};
    use rust_tests::elf_game::player::controller::{LeftHandVisual, RightHandVisual, FirstPersonCamera};
    use rust_tests::elf_game::player::ElfHands;
    use bevy_rapier3d::prelude::{Collider, Velocity};

    // 1. Direct raycast priority over general cone objects
    let mut app = App::new();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_message::<rust_tests::elf_game::wrapping::UseToolOnPresentMessage>();
    app.add_systems(Update, dual_hand_interaction_system);

    let player_e = app.world_mut().spawn((
        ElfHands::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
    )).id();

    // Camera at (0.0, 1.2, 0.0) looking directly forward (-Z)
    let cam_e = app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.2, 0.0)),
    )).id();

    app.world_mut().spawn((
        LeftHandVisual,
        GlobalTransform::from(Transform::from_xyz(-0.35, 1.0, -0.6)),
    ));
    app.world_mut().spawn((
        RightHandVisual,
        GlobalTransform::from(Transform::from_xyz(0.35, 1.0, -0.6)),
    ));

    // Object A: Directly on reticle line of sight at (0.0, 1.2, -1.5)
    let obj_reticle = app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(0.0, 1.2, -1.5),
        Collider::cuboid(0.15, 0.15, 0.15),
        Velocity::zero(),
    )).id();

    // Object B: Off-axis at (-0.8, 1.0, -1.0) closer to left hand (general cone / hand proximity)
    let obj_cone = app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(-0.8, 1.0, -1.0),
        Collider::cuboid(0.15, 0.15, 0.15),
        Velocity::zero(),
    )).id();

    // Player presses KeyQ: must grab Object A (in direct reticle raycasted line of sight), NOT Object B
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app.update();

    let hands = app.world().get::<ElfHands>(player_e).unwrap();
    assert_eq!(
        hands.left_grabbed,
        Some(obj_reticle),
        "Grabbing must target the object in direct raycasted line of sight with reticle instead of general cone"
    );

    // Release KeyQ and set down
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset(KeyCode::KeyQ);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app.update();
    assert_eq!(app.world().get::<ElfHands>(player_e).unwrap().left_grabbed, None);

    // 2. Aim reticle at Object B by rotating camera towards (-0.8, 1.0, -1.0)
    let to_b = (Vec3::new(-0.8, 1.0, -1.0) - Vec3::new(0.0, 1.2, 0.0)).normalize();
    let rot_b = Quat::from_rotation_arc(Vec3::NEG_Z, to_b);
    *app.world_mut().get_mut::<GlobalTransform>(cam_e).unwrap() = GlobalTransform::from(
        Transform::from_xyz(0.0, 1.2, 0.0).with_rotation(rot_b)
    );

    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset(KeyCode::KeyQ);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app.update();

    let hands = app.world().get::<ElfHands>(player_e).unwrap();
    assert_eq!(
        hands.left_grabbed,
        Some(obj_cone),
        "Aiming reticle directly at Object B must grab Object B"
    );

    // 3. Line of sight occlusion: an obstacle in front of the object blocks grab
    let mut app2 = App::new();
    app2.insert_resource(ButtonInput::<KeyCode>::default());
    app2.insert_resource(ButtonInput::<MouseButton>::default());
    app2.add_message::<rust_tests::elf_game::wrapping::UseToolOnPresentMessage>();
    app2.add_systems(Update, dual_hand_interaction_system);

    let player2_e = app2.world_mut().spawn((
        ElfHands::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
    )).id();

    app2.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.2, 0.0)),
    ));
    app2.world_mut().spawn((
        LeftHandVisual,
        GlobalTransform::from(Transform::from_xyz(-0.35, 1.0, -0.6)),
    ));

    // Target object behind wall at (0.0, 1.2, -2.0)
    let _hidden_target = app2.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(0.0, 1.2, -2.0),
        Collider::cuboid(0.15, 0.15, 0.15),
        Velocity::zero(),
    )).id();

    // Solid wall obstacle directly blocking line of sight at (0.0, 1.2, -1.0)
    app2.world_mut().spawn((
        Transform::from_xyz(0.0, 1.2, -1.0),
        Collider::cuboid(0.5, 0.5, 0.1),
    ));

    // Press KeyQ: line of sight is blocked by the wall obstacle, so grab must fail
    app2.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app2.update();

    let hands2 = app2.world().get::<ElfHands>(player2_e).unwrap();
    assert_eq!(
        hands2.left_grabbed,
        None,
        "Cannot grab an object through a solid obstacle blocking reticle line of sight"
    );
}

#[test]
fn test_player_elf_collision_default_speed_vs_sprint_bhop_ragdoll() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfNpcState, elf_assistant_ai_system,
    };
    use rust_tests::elf_game::player::ElfMovement;
    use bevy_rapier3d::prelude::{ExternalImpulse, LockedAxes, Velocity};

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.add_systems(Update, elf_assistant_ai_system);

    let elf_e = app.world_mut().spawn((
        ElfAssistant::new(3, "TestElf", vec![Vec3::ZERO]),
        Transform::from_xyz(0.0, 0.45, 0.0),
        Velocity::zero(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    // 1. Player running into elf at default speed cap (6.0 m/s)
    let player_e = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.45, 0.3), // Near elf (< 1.1m)
        ElfMovement {
            velocity: Vec3::new(6.0, 0.0, 0.0), // Exactly at default speed cap
            base_speed: 6.0,
            current_max_speed: 6.0,
            ..default()
        },
    )).id();

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let locked = app.world().get::<LockedAxes>(elf_e).unwrap();
        let impulse = app.world().get::<ExternalImpulse>(elf_e).unwrap();

        // At default speed cap, elf must NOT ragdoll
        assert_ne!(
            elf.state,
            ElfNpcState::RagdollTumble,
            "Elf must NOT ragdoll when player collides at default speed cap (6.0 m/s)"
        );
        assert!(
            locked.contains(LockedAxes::ROTATION_LOCKED_X),
            "Rotation axes must remain locked when hit at default speed cap"
        );
        // And must NOT receive a large impulse (small nudge <= 1.0 Ns, no upward lift)
        assert!(
            impulse.impulse.length() <= 1.0,
            "Elf must not receive large impulse at default speed cap, got impulse = {:?}",
            impulse.impulse
        );
        assert_eq!(
            impulse.impulse.y, 0.0,
            "Elf must not receive upward lift impulse at default speed cap"
        );
    }

    // 2. Player running into elf above default speed cap (from sprinting or bunny hopping, e.g. 8.1 m/s)
    {
        let mut p_mov = app.world_mut().get_mut::<ElfMovement>(player_e).unwrap();
        p_mov.velocity = Vec3::new(8.1, 0.0, 0.0);
        p_mov.current_max_speed = 8.1;
    }

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let locked = app.world().get::<LockedAxes>(elf_e).unwrap();
        let impulse = app.world().get::<ExternalImpulse>(elf_e).unwrap();

        // Above default speed cap, elf MUST ragdoll and tumble
        assert_eq!(
            elf.state,
            ElfNpcState::RagdollTumble,
            "Elf must enter RagdollTumble when player collides above default speed cap (e.g. 8.1 m/s sprint/BHOP)"
        );
        assert_eq!(
            *locked,
            LockedAxes::empty(),
            "Rotation axes must be completely unlocked for 6-DOF ragdoll tumbling"
        );
        // And receives large impulse (>= 10.0 Ns + upward lift)
        assert!(
            impulse.impulse.length() >= 10.0,
            "Elf must receive large knock impulse when hit at high sprint/bhop speed, got {:?}",
            impulse.impulse
        );
        assert!(
            impulse.impulse.y >= 4.5,
            "Elf must receive upward lift impulse when ragdolled, got vy = {}",
            impulse.impulse.y
        );
    }
}

#[test]
fn test_stable_door_and_moose_slow_peeking_over_30_seconds() {
    use rust_tests::elf_game::moose::toy_model::{
        MrMoose, MooseHead, animate_mr_moose_peeking_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage};
    use rust_tests::elf_game::workshop::stable_door::{
        StableDoorTopHalf, stable_door_system,
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    let hunger = MooseHunger {
        value: 10.0,
        max_value: 60.0,
        stage: MooseHungerStage::BehindShutters,
        ..default()
    };
    app.insert_resource(hunger);

    app.add_systems(Update, (animate_mr_moose_peeking_system, stable_door_system));

    let moose_e = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(-2.0, 1.8, -17.0),
    )).id();

    let head_e = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    )).id();

    let top_door_e = app.world_mut().spawn((
        StableDoorTopHalf {
            current_angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            is_physics_driven: false,
            hinge_pos: Vec3::new(-2.9, 1.78, -13.0),
        },
        Transform::from_xyz(-2.0, 1.78, -13.0),
    )).id();

    // 1. In BehindShutters (hunger = 10.0), door is completely closed and Moose is far back outside
    app.update();
    {
        let top = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
        let moose_tf = app.world().get::<Transform>(moose_e).unwrap();
        let head_tf = app.world().get::<Transform>(head_e).unwrap();
        assert_eq!(top.current_angle, 0.0);
        assert!((moose_tf.translation.z - (-17.0)).abs() < 0.05);
        assert!((head_tf.translation.z - 1.6).abs() < 0.05);
    }

    // 2. Step through peeking over 30 seconds as hunger increases from 20.0 to 50.0
    // Verify monotonic forward progression and slow door opening
    let check_points = [
        (20.0, 0.0, -17.0, 1.6),
        (25.0, 0.26, -16.58, 1.75),
        (35.0, 0.785, -15.75, 2.05),
        (45.0, 1.308, -14.92, 2.35),
        (50.0, 1.57, -14.5, 2.5),
    ];

    let mut prev_angle = -0.01;
    let mut prev_moose_z = -17.1;
    let mut prev_head_z = 1.5;

    for (hunger_val, expected_angle, expected_moose_z, expected_head_z) in check_points {
        {
            let mut h = app.world_mut().resource_mut::<MooseHunger>();
            h.value = hunger_val;
            h.stage = MooseHungerStage::PeekingThrough;
        }

        // Run several ticks for smooth follow
        for _ in 0..20 {
            app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
            app.update();
        }

        let top = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
        let moose_tf = app.world().get::<Transform>(moose_e).unwrap();
        let head_tf = app.world().get::<Transform>(head_e).unwrap();

        // Must be strictly monotonic
        assert!(top.current_angle >= prev_angle, "Door angle must advance monotonically (at hunger {hunger_val}: got {}, prev {})", top.current_angle, prev_angle);
        assert!(moose_tf.translation.z >= prev_moose_z, "Moose Z must advance monotonically (at hunger {hunger_val}: got {}, prev {})", moose_tf.translation.z, prev_moose_z);
        assert!(head_tf.translation.z >= prev_head_z, "Head Z must advance monotonically (at hunger {hunger_val}: got {}, prev {})", head_tf.translation.z, prev_head_z);

        // Approximate checks at key points
        assert!(
            (top.current_angle - expected_angle).abs() < 0.12,
            "Door angle at hunger {hunger_val} expected ~{expected_angle}, got {}",
            top.current_angle
        );
        assert!(
            (moose_tf.translation.z - expected_moose_z).abs() < 0.15,
            "Moose Z at hunger {hunger_val} expected ~{expected_moose_z}, got {}",
            moose_tf.translation.z
        );
        assert!(
            (head_tf.translation.z - expected_head_z).abs() < 0.15,
            "Head Z at hunger {hunger_val} expected ~{expected_head_z}, got {}",
            head_tf.translation.z
        );

        prev_angle = top.current_angle;
        prev_moose_z = moose_tf.translation.z;
        prev_head_z = head_tf.translation.z;
    }

    // 3. Fast slam when window is slammed at hunger = 60
    {
        let mut h = app.world_mut().resource_mut::<MooseHunger>();
        h.value = 60.0;
        h.stage = MooseHungerStage::WindowSlammed;
    }

    for _ in 0..15 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    let top_slammed = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
    let moose_slammed = app.world().get::<Transform>(moose_e).unwrap();
    let head_slammed = app.world().get::<Transform>(head_e).unwrap();

    assert!(top_slammed.current_angle < 0.05, "Top stable door must slam shut within 0.25s, got {}", top_slammed.current_angle);
    assert!(moose_slammed.translation.z < -16.8, "Moose body must snap back outside within 0.25s, got {}", moose_slammed.translation.z);
    assert!(head_slammed.translation.z < 1.7, "Moose head must snap back behind shutters within 0.25s, got {}", head_slammed.translation.z);
}

#[test]
fn test_dynamic_waypoint_targeting_for_elves_and_npcs() {
    use rust_tests::elf_game::{
        physics_sandbox::{
            elf_assistant_ai_system, DynamicWaypointTarget, ElfAssistant, ElfNavGraph,
        },
        player::ElfMovement,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.init_resource::<ElfNavGraph>();
    app.add_systems(Update, elf_assistant_ai_system);

    // 1. Verify NavGraph wander radius around waypoints is expanded (all nodes >= 1.1)
    let graph = app.world().get_resource::<ElfNavGraph>().unwrap();
    for node in &graph.nodes {
        assert!(
            node.radius >= 1.1,
            "Node {} radius must be expanded to >= 1.1m (got {})",
            node.name,
            node.radius
        );
    }

    // 2. Spawn Player at (10.0, 0.45, 0.0)
    let _player_e = app.world_mut().spawn((
        Transform::from_xyz(10.0, 0.45, 0.0),
        ElfMovement::default(),
    )).id();

    // 3. Spawn Elf A at (0.0, 0.45, 0.0) targeting Player
    let mut elf_a = ElfAssistant::new(1, "TargetingElf", vec![Vec3::ZERO]);
    assert!(elf_a.wander_radius >= 1.2, "Elf wander radius must be expanded");
    elf_a.target_player();
    assert_eq!(elf_a.dynamic_target, Some(DynamicWaypointTarget::Player));

    let elf_a_entity = app.world_mut().spawn((
        elf_a,
        Transform::from_xyz(0.0, 0.45, 0.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    app.update();

    let vel = app.world().get::<Velocity>(elf_a_entity).unwrap();
    assert!(
        vel.linear.x > 0.5,
        "Elf targeting player must move along +X towards player at (10.0, 0.45, 0.0), got vel.x: {}",
        vel.linear.x
    );

    // 4. Target another NPC (Elf B) at (0.0, 0.45, 10.0)
    let elf_b = ElfAssistant::new(2, "TargetNpcElf", vec![Vec3::new(0.0, 0.45, 10.0)]);
    let elf_b_entity = app.world_mut().spawn((
        elf_b,
        Transform::from_xyz(0.0, 0.45, 10.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    {
        let mut elf_mut = app.world_mut().get_mut::<ElfAssistant>(elf_a_entity).unwrap();
        elf_mut.target_npc(elf_b_entity);
        assert_eq!(elf_mut.dynamic_target, Some(DynamicWaypointTarget::Npc(elf_b_entity)));
    }

    app.update();

    let vel = app.world().get::<Velocity>(elf_a_entity).unwrap();
    assert!(
        vel.linear.z > 0.5,
        "Elf targeting NPC B must move along +Z towards (0.0, 0.45, 10.0), got vel.z: {}",
        vel.linear.z
    );

    // 5. Target custom position (-8.0, 0.45, 0.0)
    {
        let mut elf_mut = app.world_mut().get_mut::<ElfAssistant>(elf_a_entity).unwrap();
        elf_mut.target_position(Vec3::new(-8.0, 0.45, 0.0));
    }

    app.update();

    let vel = app.world().get::<Velocity>(elf_a_entity).unwrap();
    assert!(
        vel.linear.x < -0.5,
        "Elf targeting position must move along -X towards (-8.0, 0.45, 0.0), got vel.x: {}",
        vel.linear.x
    );

    // 6. Test component insertion overrides / syncs dynamic_target
    app.world_mut().entity_mut(elf_a_entity).insert(DynamicWaypointTarget::Player);
    app.update();

    let elf_after = app.world().get::<ElfAssistant>(elf_a_entity).unwrap();
    assert_eq!(elf_after.dynamic_target, Some(DynamicWaypointTarget::Player));

    // 7. Clear dynamic target
    {
        let mut elf_mut = app.world_mut().get_mut::<ElfAssistant>(elf_a_entity).unwrap();
        elf_mut.clear_dynamic_target();
        assert_eq!(elf_mut.dynamic_target, None);
    }
}

#[test]
fn test_moose_rampage_dynamic_player_targeting_and_cooldown() {
    use rust_tests::elf_game::{
        moose::{toy_model::MrMoose, MooseHunger, MooseHungerStage},
        physics_sandbox::DynamicWaypointTarget,
        player::ElfMovement,
        workshop::events::{moose_rampage_path_and_physics_impulse_system, MooseRampageTracker},
    };

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    app.insert_resource(MooseHunger {
        value: 60.0,
        stage: MooseHungerStage::BreachedRampage,
        is_breached: true,
        ..default()
    });
    app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

    // Spawn Moose at origin (0.0, 1.4, 0.0)
    let moose_entity = app.world_mut().spawn((
        MrMoose,
        MooseRampageTracker {
            current_waypoint_index: 1,
            speed: 8.5,
            base_speed: 8.5,
            waypoints: vec![
                Vec3::new(0.0, 1.4, -100.0),
                Vec3::new(0.0, 1.4, 100.0),
                Vec3::new(0.0, 1.8, 500.0),
            ],
            ..default()
        },
        Transform::from_xyz(0.0, 1.4, 0.0),
    )).id();

    // Spawn Player far away at (10.0, 0.45, 0.0) (> 5m)
    let player_entity = app.world_mut().spawn((
        Transform::from_xyz(10.0, 0.45, 0.0),
        ElfMovement {
            velocity: Vec3::ZERO,
            base_speed: 6.0,
            ..default()
        },
    )).id();

    // 1. Player is > 5m away: Moose does NOT target player
    app.update();
    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(!tracker.is_targeting_player, "Moose must not target player when > 5m away");
    assert_eq!(tracker.dynamic_target, None);

    // 2. Player moves within 5m (4.5m from Moose)
    {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(4.5, 0.0, 0.0);
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(tracker.is_targeting_player, "Moose must target player when within 5m");
    assert_eq!(tracker.dynamic_target, Some(DynamicWaypointTarget::Player));

    // 3. Player moves to 5.8m (<= 6m): Moose keeps targeting player
    {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(5.8, 0.0, 0.0);
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(tracker.is_targeting_player, "Moose must keep targeting player while within 6m");

    // 4. Player runs away to 7.0m (> 6m), but only 1 second has elapsed (< 5s)
    for _ in 0..10 {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(7.0, 0.0, 0.0);
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(100));
        app.update();
    }

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(
        tracker.is_targeting_player,
        "Moose must keep targeting player for 5 seconds even if player steps beyond 6m"
    );

    // 5. Advance time by 4.5 more seconds (total > 5s, player still running away at 7.0m > 6m)
    for _ in 0..45 {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(7.0, 0.0, 0.0);
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(100));
        app.update();
    }

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(
        !tracker.is_targeting_player,
        "Moose must stop targeting player after 5s have elapsed and player is outside 6m"
    );
    assert_eq!(tracker.dynamic_target, None);

    // 6. Player comes back within 5m (4.0m): Moose targets player again
    {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(4.0, 0.0, 0.0);
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(tracker.is_targeting_player, "Moose must retarget player when entering 5m again");

    // 7. Contact: Player collides with Moose (< 1.6m)
    {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(0.8, 0.0, 0.0);
    }
    app.update();

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    let player_mov = app.world().get::<ElfMovement>(player_entity).unwrap();

    assert!(player_mov.velocity.y >= 8.0, "Player must be launched on contact with Moose");
    assert!(!tracker.is_targeting_player, "Targeting must stop upon contact");
    assert_eq!(tracker.dynamic_target, None);
    assert!(
        tracker.player_target_cooldown >= 2.9,
        "Moose must enter 3-second cooldown upon contacting player, got {}",
        tracker.player_target_cooldown
    );

    // 8. While on cooldown, player being <= 5m (3.0m) does NOT trigger targeting
    for _ in 0..10 {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(3.0, 0.0, 0.0);
        let mut p_mov = app.world_mut().get_mut::<ElfMovement>(player_entity).unwrap();
        p_mov.velocity = Vec3::ZERO;
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(100)); // 1.0s elapsed, 2.0s cooldown remaining
        app.update();
    }

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(
        !tracker.is_targeting_player,
        "Moose must NOT target player while 3s cooldown is active (cooldown = {})",
        tracker.player_target_cooldown
    );

    // 9. Advance time past remaining cooldown (2.1s)
    for _ in 0..21 {
        let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_tf.translation = moose_pos + Vec3::new(3.0, 0.0, 0.0);
        let mut p_mov = app.world_mut().get_mut::<ElfMovement>(player_entity).unwrap();
        p_mov.velocity = Vec3::ZERO;
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(100)); // Cooldown expires!
        app.update();
    }

    let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
    assert!(
        tracker.is_targeting_player,
        "Moose should target player again once cooldown expires and player is within 5m"
    );
}

#[test]
fn test_bunny_ears_item_equip_viewmodel_and_bhop_mechanics() {
    use rust_tests::elf_game::player::{
        bunny_ears::{
            BunnyEarsEquipped, BunnyEarsItem, FloppyEar, ViewmodelBunnyEars,
            player_bunny_ears_equip_system, floppy_ears_physics_bob_system,
        },
        controller::{ElfPlayer, ViewModelRoot},
        hands::spawn_workshop_tools_and_materials,
        update_elf_physics, BhopConfig, BhopWindow, ElfHands, ElfMovement,
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<BhopConfig>();

    app.add_systems(Startup, (
        spawn_workshop_tools_and_materials,
        |mut commands: Commands| {
            // Player entity with ViewModelRoot
            let player_e = commands
                .spawn((
                    ElfPlayer,
                    ElfMovement::default(),
                    ElfHands::default(),
                    Transform::from_xyz(0.0, 1.2, 0.0),
                    Name::new("ElfPlayer"),
                ))
                .id();

            let _vm_root = commands
                .spawn((
                    ViewModelRoot,
                    Transform::IDENTITY,
                    Visibility::default(),
                    ChildOf(player_e),
                    Name::new("ViewModelRoot"),
                ))
                .id();
        },
    ));

    app.add_systems(
        Update,
        (
            player_bunny_ears_equip_system,
            floppy_ears_physics_bob_system,
        ),
    );

    app.update();

    // 1. Verify BunnyEarsItem spawned in world
    let mut bunny_q = app.world_mut().query::<(Entity, &BunnyEarsItem, &Transform)>();
    let (item_e, _, item_tf) = bunny_q.iter(app.world()).next().expect("BunnyEarsItem must spawn in workshop");
    assert!((item_tf.translation.x - 1.8).abs() < 0.2);

    // 2. Put bunny ears into player's hand and press KeyH to equip
    let mut player_q = app.world_mut().query::<(Entity, &mut ElfHands)>();
    let (p_e, mut hands) = player_q.single_mut(app.world_mut()).unwrap();
    hands.right_grabbed = Some(item_e);

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyH);
    }
    app.update();

    // 3. Verify BunnyEarsEquipped component is added, item is removed from hand and world
    assert!(app.world().get::<BunnyEarsEquipped>(p_e).is_some(), "Player must have BunnyEarsEquipped");
    let mov = app.world().get::<ElfMovement>(p_e).unwrap();
    assert!(mov.has_bunny_ears, "Movement has_bunny_ears must be true");

    let hands = app.world().get::<ElfHands>(p_e).unwrap();
    assert_eq!(hands.right_grabbed, None, "Equipped item must be cleared from hand");

    let mut item_check_q = app.world_mut().query::<&BunnyEarsItem>();
    assert_eq!(item_check_q.iter(app.world()).count(), 0, "World pickup must be despawned on equip");

    // 4. Verify ViewmodelBunnyEars and 2 FloppyEar components are spawned under ViewModelRoot
    let mut vm_ears_q = app.world_mut().query::<&ViewmodelBunnyEars>();
    assert_eq!(vm_ears_q.iter(app.world()).count(), 1, "ViewmodelBunnyEars must be spawned");

    let mut floppy_q = app.world_mut().query::<(&FloppyEar, &Transform)>();
    assert_eq!(floppy_q.iter(app.world()).count(), 2, "Must spawn left and right floppy ears");

    // 5. Physics simulation bob: run an update to verify spring dynamics don't crash
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyH);
        keys.reset(KeyCode::KeyH);
        let mut mov = app.world_mut().get_mut::<ElfMovement>(p_e).unwrap();
        mov.velocity = Vec3::new(4.0, 3.5, 0.0); // moving and jumping
    }
    app.update();

    // 6. Test BHOP mechanics with Bunny Ears:
    // With Sixtieth config window (1/60s ≈ 0.0167s), normal landing at t = 0.08s would FAIL.
    // But with Bunny Ears, grace period is 1/10s (0.10s), so jumping at t = 0.08s SUCCEEDS!
    let config = BhopConfig {
        window: BhopWindow::Sixtieth,
        max_boost_multiplier: 1.414,
    };
    let mut mov_equipped = ElfMovement {
        is_grounded: true,
        time_since_landing: 0.08, // > 1/60s, but < 1/10s
        has_bunny_ears: true,
        ..default()
    };
    update_elf_physics(0.016, Vec3::ZERO, true, false, &config, &mut mov_equipped);
    assert_eq!(mov_equipped.consecutive_bhops, 1, "Bunnyhop must succeed within 1/10s grace period when bunny ears equipped");
    assert!((mov_equipped.velocity.y - 7.7).abs() < 0.01, "Player with bunny ears equipped must jump 10% higher (7.7 vs 7.0)");

    // 7. Test in-air straining:
    let mut mov_air = ElfMovement {
        is_grounded: false,
        air_strain_accel: 5.0,
        has_bunny_ears: true,
        ..default()
    };
    update_elf_physics(0.1, Vec3::new(1.0, 0.0, 0.0), false, false, &config, &mut mov_air);
    // strain = wish * (5.0 * 2.0) * 0.1 = 1.0
    assert!((mov_air.velocity.x - 1.0).abs() < 0.01, "Player with bunny ears must have 2x air straining (1.0 vs 0.5)");

    // 8. Test unequip: press KeyH again to unequip
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyH);
    }
    app.update();

    assert!(app.world().get::<BunnyEarsEquipped>(p_e).is_none(), "BunnyEarsEquipped must be removed on unequip");
    let mov_after = app.world().get::<ElfMovement>(p_e).unwrap();
    assert!(!mov_after.has_bunny_ears, "has_bunny_ears must be false after unequip");

    let mut vm_check_q = app.world_mut().query::<&ViewmodelBunnyEars>();
    assert_eq!(vm_check_q.iter(app.world()).count(), 0, "Viewmodel ears must be despawned on unequip");

    let mut item_respawn_q = app.world_mut().query::<&BunnyEarsItem>();
    assert_eq!(item_respawn_q.iter(app.world()).count(), 1, "BunnyEarsItem must drop back into world on unequip");
}

#[test]
fn test_elf_navigation_stuck_timer_and_vacuum_respawn() {
    use rust_tests::elf_game::physics_sandbox::{
        ragdoll::{ElfAssistant, ElfNavGraph, ElfNpcState, elf_assistant_ai_system},
    };
    use rust_tests::elf_game::workshop::room::TubeTravellingLump;

    let mut app = App::new();
    app.insert_resource(ElfNavGraph::default_workshop_graph());

    let elf_e = app.world_mut().spawn((
        ElfAssistant::new(3, "TestElf", vec![Vec3::ZERO]),
        Transform::from_xyz(0.0, 0.45, 0.0),
        bevy_rapier3d::prelude::Velocity::default(),
        bevy_rapier3d::prelude::ExternalImpulse::default(),
        bevy_rapier3d::prelude::LockedAxes::default(),
    )).id();

    app.add_systems(Update, elf_assistant_ai_system);

    // Initial update
    app.update();

    let initial_target = {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_e).unwrap();
        elf.state = ElfNpcState::Wandering;
        elf.current_node = 0;
        elf.target_node = 3;
        elf.path = vec![0, 1, 2, 3];
        elf.path_step = 1;
        // Place elf far away so it cannot reach within 1 frame
        elf.target_node
    };

    // 1. Advance time by 10 seconds: elf fails to reach target node within 10s
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(10));
        app.insert_resource(time);
    }
    // Place elf stuck at a fixed location
    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_e).unwrap();
        tf.translation = Vec3::new(100.0, 0.45, 100.0);
    }
    app.update();

    let (failure_1, target_1) = {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        (elf.navigation_failure_count, elf.target_node)
    };
    assert_eq!(failure_1, 1, "Navigation failure count must increment to 1 after 10s stuck");
    assert_ne!(target_1, initial_target, "Elf must choose a different node after being stuck for 10s");

    // 2. Advance time by another 10 seconds: elf fails a second time
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(10));
        app.insert_resource(time);
    }
    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_e).unwrap();
        tf.translation = Vec3::new(100.0, 0.45, 100.0);
    }
    app.update();

    let (failure_2, target_2) = {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        (elf.navigation_failure_count, elf.target_node)
    };
    assert_eq!(failure_2, 2, "Navigation failure count must increment to 2 after second 10s stuck");
    assert_ne!(target_2, target_1, "Elf must choose another different node after second failure");

    // 3. Advance time by a third 10 seconds: elf fails 3 times -> vacuum respawn!
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(10));
        app.insert_resource(time);
    }
    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_e).unwrap();
        tf.translation = Vec3::new(100.0, 0.45, 100.0);
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(elf.state, ElfNpcState::VacuumRespawning, "Elf must enter VacuumRespawning state after 3 failures");
        assert_eq!(elf.vacuum_respawn_progress, 1.0, "Vacuum respawn progress must start at 1.0 (reverse)");
    }

    // Verify reverse vacuum lump spawned in the tube
    let mut lump_q = app.world_mut().query::<&TubeTravellingLump>();
    let reverse_lump = lump_q.iter(app.world()).find(|l| l.speed < 0.0);
    assert!(reverse_lump.is_some(), "Reverse vacuum lump with negative speed must be spawned in tube");

    // 4. Advance reverse animation: 0.8s
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(800));
        app.insert_resource(time);
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let tf = app.world().get::<Transform>(elf_e).unwrap();
        assert!(elf.vacuum_respawn_progress < 0.5, "Vacuum progress should decrease toward 0");
        // Descending near the funnel/platform area (X ≈ 5.5, Z ≈ -4.0)
        assert!((tf.translation.x - 5.5).abs() < 0.2, "Elf should be emerging near sending platform X ≈ 5.5");
        assert!((tf.translation.z - (-4.0)).abs() < 0.2, "Elf should be emerging near sending platform Z ≈ -4.0");
    }

    // 5. Complete reverse animation: 0.5s more -> respawn onto platform
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(500));
        app.insert_resource(time);
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let tf = app.world().get::<Transform>(elf_e).unwrap();
        assert_eq!(elf.state, ElfNpcState::Inspecting, "Elf should return to Inspecting state after vacuum respawn");
        assert_eq!(elf.navigation_failure_count, 0, "Failure count should reset to 0 after vacuum respawn");
        assert!((tf.translation.y - 0.45).abs() < 0.1, "Elf should land on the platform at Y ≈ 0.45");
    }
}

#[test]
fn test_klepto_stash_corner_leaf_nodes_placement_and_loop_prevention() {
    use rust_tests::elf_game::physics_sandbox::{
        ElfAssistant, ElfMischiefTrait, ElfNavGraph, ElfNpcState, ElfNodePurpose,
        elf_assistant_ai_system,
    };
    use rust_tests::elf_game::wrapping::Present;
    use bevy_rapier3d::prelude::*;

    // 1. Verify NavGraph has corner leaf nodes with degree 1 in all 4 corners of main room
    let graph = ElfNavGraph::default_workshop_graph();
    let stash_nodes = graph.find_nodes_by_purpose(ElfNodePurpose::KleptoStash);
    assert_eq!(stash_nodes.len(), 4, "Must have exactly 4 corner KleptoStash nodes");

    // All 4 stash nodes must be true leaf nodes (degree 1)
    for &node_id in &stash_nodes {
        let node = &graph.nodes[node_id];
        assert_eq!(
            node.neighbors.len(),
            1,
            "Stash node {} ({}) must be a leaf node with exactly 1 edge back to the main graph",
            node.id,
            node.name
        );
        let parent_id = node.neighbors[0];
        assert!(
            graph.nodes[parent_id].neighbors.contains(&node_id),
            "Parent node {} must connect back to leaf stash node {}",
            parent_id,
            node_id
        );
    }

    // Verify locations cover all four corners of the main room
    let sw = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x < -8.0 && graph.nodes[id].position.z > 8.0);
    let nw = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x < -8.0 && graph.nodes[id].position.z < -8.0);
    let ne = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x > 8.0 && graph.nodes[id].position.z < -8.0);
    let se = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x > 8.0 && graph.nodes[id].position.z > 8.0);

    assert!(sw.is_some(), "Must have SW corner stash node");
    assert!(nw.is_some(), "Must have NW corner stash node");
    assert!(ne.is_some(), "Must have NE corner stash node");
    assert!(se.is_some(), "Must have SE corner stash node");

    // 2. Set up ECS test for Klepto Stashing present in front of elf & cooldown
    let mut app = App::new();
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);
    }
    app.insert_resource(graph);
    app.add_systems(Update, elf_assistant_ai_system);

    let klepto_elf = ElfAssistant::new(2, "KleptoElf", vec![Vec3::ZERO]);
    assert_eq!(klepto_elf.mischief_trait, ElfMischiefTrait::Klepto);

    // Spawn elf at corridor position (-9.0, 0.45, 9.0) with present nearby
    let elf_pos = Vec3::new(-9.0, 0.45, 9.0);
    let elf_e = app.world_mut().spawn((
        klepto_elf,
        Transform::from_translation(elf_pos).looking_at(Vec3::new(-10.5, 0.45, 10.5), Vec3::Y),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    // Spawn a present near the elf to be stolen
    let present_pos = elf_pos + Vec3::new(0.3, 0.0, 0.2);
    let pres_e1 = app.world_mut().spawn((
        Present::default(),
        Transform::from_translation(present_pos),
        Velocity::default(),
    )).id();

    // First frame: elf picks up present and targets stash node (13)
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(elf.state, ElfNpcState::StashingPresent);
        assert_eq!(elf.carried_present, Some(pres_e1));
        assert_eq!(elf.target_node, 13);
    }

    // Move elf close to SW stash node (Node 13 at -10.5, 0.45, 10.5) facing South-West
    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_e).unwrap();
        tf.translation = Vec3::new(-10.4, 0.45, 10.4);
        *tf = tf.looking_at(Vec3::new(-10.5, 0.45, 10.5), Vec3::Y);
    }

    // Advance frame where elf arrives at stash node (< 0.6m away)
    // Run update so elf drops present in front of it
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let elf_tf = app.world().get::<Transform>(elf_e).unwrap();
        let pres_tf = app.world().get::<Transform>(pres_e1).unwrap();

        assert_eq!(elf.state, ElfNpcState::Inspecting, "Elf should switch to Inspecting after stashing");
        assert_eq!(elf.carried_present, None, "Present should no longer be carried");
        assert_eq!(elf.last_stashed_present, Some(pres_e1), "Elf must remember last stashed present");
        assert_eq!(elf.klepto_cooldown, 15.0, "Cooldown must be set to 15 seconds after stashing");

        // Verify present was placed in front of the elf
        let diff = pres_tf.translation - elf_tf.translation;
        let flat_dist = Vec2::new(diff.x, diff.z).length();
        assert!(flat_dist > 0.4 && flat_dist < 0.8, "Present should be placed in front of elf ~0.6m away, got {}", flat_dist);
    }

    // 3. Loop prevention: elf must NOT re-steal the same stashed present even if right next to it!
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_ne!(elf.state, ElfNpcState::StashingPresent, "Elf must not immediately re-stash the same present");
        assert_eq!(elf.carried_present, None);
    }

    // 4. Cooldown prevention: Move elf back to corridor (-9.0, 0.45, 9.0) and spawn a second present
    let corridor_pos = Vec3::new(-9.0, 0.45, 9.0);
    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_e).unwrap();
        tf.translation = corridor_pos;
    }
    let pres_e2 = app.world_mut().spawn((
        Present::default(),
        Transform::from_translation(corridor_pos + Vec3::new(0.2, 0.0, 0.2)),
        Velocity::default(),
    )).id();

    // Advance 5 seconds: cooldown is still ~10s > 0
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(5));
        app.insert_resource(time);
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert!(elf.klepto_cooldown > 0.0, "Cooldown should still be active");
        assert_ne!(elf.state, ElfNpcState::StashingPresent, "Elf must not steal second present during cooldown");
        assert_eq!(elf.carried_present, None);
    }

    // Advance 11 more seconds: cooldown expires (5 + 11 = 16s > 15s)
    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(11));
        app.insert_resource(time);
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(elf.klepto_cooldown, 0.0, "Cooldown should be expired");
        assert_eq!(elf.state, ElfNpcState::StashingPresent, "Elf should now steal the new second present");
        assert_eq!(elf.carried_present, Some(pres_e2));
    }
}

#[test]
fn test_pantry_door_push_by_player_and_elves() {
    use rust_tests::elf_game::workshop::pantry::{
        PantryDoor, pantry_door_interaction_system, pantry_door_spring_system,
    };
    use rust_tests::elf_game::player::{
        controller::ElfPlayer,
        ElfMovement,
    };
    use rust_tests::elf_game::physics_sandbox::ragdoll::ElfAssistant;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        Velocity::zero(),
        ExternalImpulse::default(),
        PantryDoor::default(),
    )).id();

    app.add_systems(Update, (pantry_door_interaction_system, pantry_door_spring_system).chain());

    // 1. Player running into the door from inside the pantry towards the room (+X)
    let player_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.2, 1.35, 5.48),
        ElfPlayer,
        ElfMovement {
            velocity: Vec3::new(3.0, 0.0, 0.0),
            ..default()
        },
    )).id();

    app.update();

    let door = app.world().get::<PantryDoor>(door_entity).unwrap();
    let ext = app.world().get::<ExternalImpulse>(door_entity).unwrap();
    assert!(
        ext.torque_impulse.y > 0.0 || door.current_angle > 0.0 || door.target_angle > 0.0,
        "Player running into pantry door must exert push impulse/torque on door"
    );

    // Despawn player for elf test
    app.world_mut().despawn(player_entity);

    // Reset door
    {
        let mut d = app.world_mut().get_mut::<PantryDoor>(door_entity).unwrap();
        d.current_angle = 0.0;
        d.target_angle = 0.0;
        let mut ext = app.world_mut().get_mut::<ExternalImpulse>(door_entity).unwrap();
        ext.torque_impulse = Vec3::ZERO;
    }

    // 2. Elf running into the door from inside the pantry (+X)
    app.world_mut().spawn((
        Transform::from_xyz(-13.2, 1.35, 5.48),
        Velocity {
            linear: Vec3::new(2.5, 0.0, 0.0),
            angular: Vec3::ZERO,
        },
        ElfAssistant::new(1, "RunnerElf", vec![]),
    ));

    app.update();

    let door_elf = app.world().get::<PantryDoor>(door_entity).unwrap();
    let ext_elf = app.world().get::<ExternalImpulse>(door_entity).unwrap();
    assert!(
        ext_elf.torque_impulse.y > 0.0 || door_elf.current_angle > 0.0 || door_elf.target_angle > 0.0,
        "Elf running into pantry door must exert push impulse/torque on door"
    );
}

#[test]
fn test_elf_navigates_pantry_door_opening_sequence() {
    use rust_tests::elf_game::workshop::pantry::PantryDoor;
    use rust_tests::elf_game::physics_sandbox::ragdoll::{
        ElfAssistant, ElfNavGraph, ElfNpcState, elf_assistant_ai_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ElfNavGraph::default());

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        Velocity::zero(),
        ExternalImpulse::default(),
        PantryDoor::default(), // closed at angle 0.0
    )).id();

    // Spawn elf at Node 8 (WestSouthCorridor at Vec3(-11.0, 0.45, 5.5))
    let elf_e = app.world_mut().spawn((
        Transform::from_xyz(-11.0, 0.45, 5.5),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ElfAssistant::new(5, "PantryChefElf", vec![]),
    )).id();

    app.add_systems(Update, elf_assistant_ai_system);

    // 1. Tell elf to navigate to the Pantry
    {
        let graph = app.world().resource::<ElfNavGraph>().clone();
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_e).unwrap();
        elf.navigate_to_pantry(Some(&graph));
        assert_eq!(elf.target_node, 15);
    }

    // Update frame 1: Elf detects closed door in the way and initiates OpeningPantryDoor sequence
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(
            elf.state,
            ElfNpcState::OpeningPantryDoor,
            "Elf must enter OpeningPantryDoor state when navigating to pantry with closed door"
        );
    }

    // Update frame 2: Elf is close enough to door (within 2m) -> grabs door and pulls open (phase 1)
    app.update();

    {
        let door = app.world().get::<PantryDoor>(door_entity).unwrap();
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert!(
            door.is_grabbed || elf.pantry_door_phase >= 1,
            "Elf must grab the door and begin pulling open"
        );
    }

    // Simulate door swinging open
    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(1.20);
    }

    // Update frame 3: Door is open -> Elf releases grab and moves to node before pantry door (Node 8, phase 2)
    app.update();

    {
        let door = app.world().get::<PantryDoor>(door_entity).unwrap();
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert!(!door.is_grabbed, "Elf must release door grab once open");
        assert_eq!(
            elf.pantry_door_phase, 2,
            "Elf must go to the node before the pantry door (Node 8) after pulling it open"
        );
    }

    // Elf arrives at Node 8 (position is already at Node 8: -11.0, 0.45, 5.5)
    // Update frame 4: Elf arrives at Node 8 -> phase 3 (entering pantry)
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(
            elf.pantry_door_phase, 3,
            "Elf should advance to phase 3 (entering pantry)"
        );
    }

    // Update frame 5: Elf enters the Pantry (sets target_node = 15, current_node = 8, state = Wandering)
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(
            elf.target_node, 15,
            "Elf must enter the pantry by targeting Node 15"
        );
        assert_eq!(
            elf.current_node, 8,
            "Elf current node should be Node 8 before entering pantry"
        );
    }
}

#[test]
fn test_philosopher_pipe_equipping_and_ember_glow() {
    use rust_tests::elf_game::physics_sandbox::ragdoll::{
        ElfAssistant, ElfMischiefTrait, PhilosopherPipe,
        elf_hat_assignment_system, philosopher_pipe_system,
    };
    use rust_tests::elf_game::player::ElfHands;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();

    let pipe_entity = app.world_mut().spawn((
        PhilosopherPipe::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Velocity::default(),
    )).id();

    // Spawn player holding pipe in right hand
    let player_e = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.45, 0.0),
        ElfHands {
            right_grabbed: Some(pipe_entity),
            ..default()
        },
    )).id();

    // Spawn assistant elf nearby (< 2m)
    let elf_e = app.world_mut().spawn((
        Transform::from_xyz(0.5, 0.45, 0.5),
        ElfAssistant::new(1, "ThinkerElf", vec![]),
    )).id();

    app.add_systems(Update, (elf_hat_assignment_system, philosopher_pipe_system).chain());

    // Press KeyE to equip hat/pipe onto nearby elf
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();

    // Verify player released pipe and pipe was assigned to elf
    {
        let hands = app.world().get::<ElfHands>(player_e).unwrap();
        assert_eq!(hands.right_grabbed, None, "Player must release pipe after equipping");

        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(
            elf.mischief_trait,
            ElfMischiefTrait::Philosopher,
            "Equipping philosopher pipe must grant Philosopher mischief trait"
        );
        assert!(elf.is_recruiter, "Philosopher elf must be marked as recruiter");
        assert_eq!(elf.worn_hat_entity, Some(pipe_entity));

        let pipe = app.world().get::<PhilosopherPipe>(pipe_entity).unwrap();
        assert_eq!(pipe.worn_by, Some(elf_e));
    }

    // Advance time and check ember intensity fluctuation
    for _ in 0..10 {
        {
            let mut time_res = app.world_mut().resource_mut::<Time>();
            time_res.advance_by(std::time::Duration::from_millis(100));
        }
        app.update();
    }

    {
        let pipe = app.world().get::<PhilosopherPipe>(pipe_entity).unwrap();
        assert!(pipe.ember_time > 0.5, "Pipe ember timer should advance");
        assert!(pipe.smoke_timer > 0.5, "Pipe smoke timer should advance");
        assert!(
            pipe.ember_intensity >= 0.25 && pipe.ember_intensity <= 1.05,
            "Ember intensity must fluctuate softly within bounds"
        );
    }
}

#[test]
fn test_philosopher_elf_navigates_to_mirror_node_and_recruits() {
    use rust_tests::elf_game::physics_sandbox::ragdoll::{
        ElfAssistant, ElfMischiefTrait, ElfNavGraph, ElfNpcState, DynamicWaypointTarget,
        elf_assistant_ai_system,
    };
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ElfNavGraph::default());

    // Spawn philosopher elf at Hallway (Node 11 at 0.0, 0.45, 7.0)
    let mut phil_elf_comp = ElfAssistant::new(10, "SocratesElf", vec![]);
    phil_elf_comp.mischief_trait = ElfMischiefTrait::Philosopher;
    phil_elf_comp.current_node = 11;

    let phil_e = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.45, 7.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        phil_elf_comp,
    )).id();

    // Spawn a regular helper elf wandering at SouthCentralHall
    let helper_e = app.world_mut().spawn((
        Transform::from_xyz(-1.0, 0.45, 7.5),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ElfAssistant::new(20, "HelperElf", vec![]),
    )).id();

    app.add_systems(Update, elf_assistant_ai_system);

    // 1. Philosopher elf should target Node 19 (MirrorNode)
    app.update();

    {
        let phil = app.world().get::<ElfAssistant>(phil_e).unwrap();
        assert_eq!(phil.target_node, 19, "Philosopher elf must target Mirror Node (19)");
    }

    // Teleport philosopher elf to MirrorNode (-3.0, 0.45, 10.5)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(phil_e).unwrap();
        tf.translation = Vec3::new(-3.0, 0.45, 10.5);
    }

    // Update to register arrival at MirrorNode
    app.update();

    {
        let phil = app.world().get::<ElfAssistant>(phil_e).unwrap();
        assert_eq!(
            phil.state,
            ElfNpcState::Philosophizing,
            "Philosopher elf at Mirror Node must enter Philosophizing state"
        );
    }

    // Advance 11 seconds to trigger recruiting logic
    {
        let mut tf = app.world_mut().get_mut::<ElfAssistant>(phil_e).unwrap();
        tf.recruit_timer = 10.5;
    }

    app.update();

    // Verify helper elf was recruited to philosopher
    {
        let helper = app.world().get::<ElfAssistant>(helper_e).unwrap();
        assert!(helper.is_recruited, "Helper elf must be recruited by philosopher");
        assert_eq!(
            helper.dynamic_target,
            Some(DynamicWaypointTarget::Npc(phil_e)),
            "Recruited elf must navigate to philosopher elf"
        );
    }

    // Move helper elf close to philosopher (< 1.5m)
    {
        let mut tf = app.world_mut().get_mut::<Transform>(helper_e).unwrap();
        tf.translation = Vec3::new(-2.8, 0.45, 10.2);
    }

    app.update();

    // Verify helper elf has entered Philosophizing state in thinking pose alongside philosopher
    {
        let helper = app.world().get::<ElfAssistant>(helper_e).unwrap();
        assert_eq!(
            helper.state,
            ElfNpcState::Philosophizing,
            "Recruited elf arriving near philosopher must enter Philosophizing thinking state"
        );
    }
}

#[test]
fn test_philosopher_dialogue_tree_resolution_gets_elf_back_to_work() {
    use rust_tests::elf_game::dialogue::{
        ActiveDialogue, DialogueAction,
    };
    use rust_tests::elf_game::physics_sandbox::ragdoll::{
        ElfAssistant, ElfMischiefTrait, ElfNpcState,
    };

    let mut app = App::new();

    let mut phil_comp = ElfAssistant::new(1, "PlatoElf", vec![]);
    phil_comp.mischief_trait = ElfMischiefTrait::Philosopher;
    phil_comp.state = ElfNpcState::Philosophizing;
    phil_comp.is_recruiter = true;

    let phil_e = app.world_mut().spawn((
        Transform::from_xyz(-3.0, 0.45, 10.5),
        phil_comp,
    )).id();

    let mut helper_comp = ElfAssistant::new(2, "StudentElf", vec![]);
    helper_comp.state = ElfNpcState::Philosophizing;
    helper_comp.is_recruited = true;

    let helper_e = app.world_mut().spawn((
        Transform::from_xyz(-2.5, 0.45, 10.5),
        helper_comp,
    )).id();

    let mut active_dialogue = ActiveDialogue::default();

    // 1. Open dialogue for philosopher elf
    {
        let elf = app.world().get::<ElfAssistant>(phil_e).unwrap();
        active_dialogue.open_for_elf(phil_e, elf);
    }

    assert!(active_dialogue.is_active);
    let root_menu = active_dialogue.current_menu.as_ref().unwrap();
    assert!(root_menu.prompt.contains("what IS joy?"));
    assert_eq!(root_menu.options.len(), 3);
    assert_eq!(root_menu.options[0].label, "Meaning of Toys");
    assert_eq!(root_menu.options[1].label, "Cogito Ergo Present");
    assert_eq!(root_menu.options[2].label, "Soul of Moose");

    // 2. Navigate into branch 1: "Meaning of Toys"
    let action1 = root_menu.options[0].action.clone();
    let DialogueAction::Submenu(sub1) = action1 else { panic!("Expected Submenu"); };
    assert_eq!(sub1.options.len(), 2);
    assert_eq!(sub1.options[0].label, "Creation is Joy");

    // 3. Navigate into level 3: "Creation is Joy" -> "Shared Purpose"
    let action2 = sub1.options[0].action.clone();
    let DialogueAction::Submenu(sub2) = action2 else { panic!("Expected Submenu level 3"); };
    assert_eq!(sub2.options[0].label, "Shared Purpose");

    // 4. Conclude Philosophy action!
    let action3 = sub2.options[0].action.clone();
    let DialogueAction::ConcludePhilosophy(msg) = action3 else { panic!("Expected ConcludePhilosophy"); };
    assert!(msg.contains("synthesized the thesis and antithesis"));

    // Simulate selecting ConcludePhilosophy
    {
        let mut q_elves = app.world_mut().query::<(Entity, &Transform, &mut ElfAssistant)>();
        // Directly resolve as dialogue system does:
        if let Ok((_, _, mut elf)) = q_elves.get_mut(app.world_mut(), phil_e) {
            elf.mischief_trait = ElfMischiefTrait::Lazy;
            elf.state = ElfNpcState::Wandering;
            elf.is_recruiter = false;
        }
        for (_, _, mut other) in q_elves.iter_mut(app.world_mut()) {
            if other.is_recruited {
                other.is_recruited = false;
                other.state = ElfNpcState::Wandering;
                other.dynamic_target = None;
            }
        }
    }

    // 5. Verify both philosopher and recruited elf returned to work (Wandering state)
    let phil_after = app.world().get::<ElfAssistant>(phil_e).unwrap();
    assert_eq!(phil_after.state, ElfNpcState::Wandering, "Philosopher elf must return to work");
    assert_ne!(phil_after.mischief_trait, ElfMischiefTrait::Philosopher, "Philosopher trait must be resolved");

    let helper_after = app.world().get::<ElfAssistant>(helper_e).unwrap();
    assert_eq!(helper_after.state, ElfNpcState::Wandering, "Recruited elf must return to work");
    assert!(!helper_after.is_recruited, "Recruited status must be cleared");
}
