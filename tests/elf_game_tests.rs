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

    assert!(signs.len() >= 9, "Expected at least 9 directional signs/labels, found: {}", signs.len());

    let stations: Vec<&str> = signs.iter().map(|(st, _)| st.as_str()).collect();
    assert!(stations.contains(&"WrappingWorkbench"), "Must have Workbench Wrapping plaque");
    assert!(stations.contains(&"PresentSendingPlatform"), "Must have Present Sending Platform sign");
    assert!(stations.contains(&"SendPlatformButton"), "Must have Send Button sign");
    assert!(stations.contains(&"MooseSnackJar"), "Must have Moose Snack Jar label");
    assert!(stations.contains(&"MooseFeedingTrough"), "Must have Moose Feeding Trough sign");
    assert!(stations.contains(&"MooseWindow"), "Must have Moose Warning Window sign");
    assert!(stations.contains(&"MainConveyorBelt"), "Must have Conveyor Belt intake sign");
    assert!(stations.contains(&"ConveyorDespawnChute"), "Must have Recessed Floor Chute sign");
    assert!(stations.contains(&"OverheadSuctionFunnel"), "Must have Overhead Suction Funnel sign");

    let workbench_sign = signs.iter().find(|(st, _)| st == "WrappingWorkbench").unwrap();
    assert!(workbench_sign.1.contains("WRAPPING STATION"));
    assert!(workbench_sign.1.contains("ASSEMBLE, TAPE & TIE"));

    let send_sign = signs.iter().find(|(st, _)| st == "PresentSendingPlatform").unwrap();
    assert!(send_sign.1.contains("PLACE WRAPPED GIFT ON CIRCLE"));
    assert!(send_sign.1.contains("HIT RED BUTTON TO SEND"));

    let moose_snack_sign = signs.iter().find(|(st, _)| st == "MooseSnackJar").unwrap();
    assert!(moose_snack_sign.1.contains("MOOSE SNACKS"));
    assert!(moose_snack_sign.1.contains("GRAB [Q/E] & CARRY TO TROUGH"));
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

    // 1. Enter PeekingThrough stage
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 20.5; // Barely entered PeekingThrough!
        hunger.stage = MooseHungerStage::PeekingThrough;
    }

    // Advance 15 frames (~0.24s)
    for _ in 0..15 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
        app.update();
    }

    // Verify Moose body snapped close to window (Z ≈ -14.5), head poked moderately into room (Z ≈ 2.4, reduced by 1/2), shutters open (> 1.0 rad)
    let m_tf = app.world().get::<Transform>(moose).unwrap();
    let h_tf = app.world().get::<Transform>(head).unwrap();
    let ls_tf = app.world().get::<Transform>(left_shutter).unwrap();
    let (left_yaw, _, _) = ls_tf.rotation.to_euler(EulerRot::YXZ);

    assert!(
        m_tf.translation.z > -14.7,
        "Moose body must have snapped forward quickly to the window (Z > -14.7) within 0.25s, got {}",
        m_tf.translation.z
    );
    assert!(
        h_tf.translation.z > 2.3 && h_tf.translation.z < 2.6,
        "Moose head must have poked forward into room with reduced 1/2 distance (2.3 < Z < 2.6) within 0.25s, got {}",
        h_tf.translation.z
    );
    assert!(
        left_yaw < -1.0,
        "Left shutter must have swung open inward within 0.25s, got yaw {}",
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
        MooseHungerDial, MooseHungerNeedle, MooseHungerWarningBeacon, MooseTroughScalePlate,
        spawn_moose_hunger_dial_and_scale, animate_moose_hunger_dial_system,
    };
    use rust_tests::elf_game::moose::{MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack};

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.insert_resource(MooseHunger {
        value: 0.0,
        max_value: 60.0,
        stage: MooseHungerStage::BehindShutters,
        ..default()
    });

    app.add_systems(Startup, spawn_moose_hunger_dial_and_scale);
    app.add_systems(Update, animate_moose_hunger_dial_system);

    // Spawn feeding trough at default location
    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    // Run startup
    app.update();

    // 1. Verify all components exist
    let mut dial_q = app.world_mut().query::<(&MooseHungerDial, &Transform)>();
    let dial = dial_q.iter(app.world()).next();
    assert!(dial.is_some(), "MooseHungerDial must be spawned outside window");
    let (_, dial_tf) = dial.unwrap();
    assert!(
        (dial_tf.translation.x - (-5.35)).abs() < 0.2 && (dial_tf.translation.z - (-13.25)).abs() < 0.2,
        "Dial must be mounted beside window exterior jamb, got {:?}",
        dial_tf.translation
    );

    let mut needle_q = app.world_mut().query::<&MooseHungerNeedle>();
    assert_eq!(needle_q.iter(app.world()).count(), 1, "Needle entity must exist");

    let mut beacon_q = app.world_mut().query::<&MooseHungerWarningBeacon>();
    assert_eq!(beacon_q.iter(app.world()).count(), 1, "Warning beacon must exist");

    let mut scale_q = app.world_mut().query::<&MooseTroughScalePlate>();
    assert_eq!(scale_q.iter(app.world()).count(), 1, "Trough scale plate must exist under feeding trough");

    // 2. Check initial rest needle angle (at hunger 0, angle ≈ -2.356 rad = -135 deg)
    {
        let mut needle_q = app.world_mut().query::<&MooseHungerNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            (needle.current_angle - (-2.356)).abs() < 0.1,
            "Needle should be at far left green zone (≈ -2.356) at hunger 0, got {}",
            needle.current_angle
        );
        assert!(!needle.is_jittering, "Needle should not jitter when calm");
    }

    // 3. Advance to Hunger 30.0 (PeekingThrough -> midpoint angle ≈ 0.0 rad)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 30.0;
        hunger.stage = MooseHungerStage::PeekingThrough;
    }
    app.update();
    {
        let mut needle_q = app.world_mut().query::<&MooseHungerNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            needle.current_angle.abs() < 0.2,
            "Needle should be at top center (≈ 0.0) at hunger 30, got {}",
            needle.current_angle
        );
    }

    // 4. Advance to Hunger 55.0 (AgitatedHuffing -> warning zone angle > 1.2 rad, jitter enabled, beacon alert enabled)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 55.0;
        hunger.stage = MooseHungerStage::AgitatedHuffing;
    }
    app.update();
    {
        let mut needle_q = app.world_mut().query::<&MooseHungerNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            needle.current_angle > 1.2,
            "Needle should be in orange/warning zone (> 1.2 rad) at hunger 55, got {}",
            needle.current_angle
        );
        assert!(needle.is_jittering, "Needle must jitter mechanically during AgitatedHuffing");

        let mut beacon_q = app.world_mut().query::<&MooseHungerWarningBeacon>();
        let beacon = beacon_q.iter(app.world()).next().unwrap();
        assert!(beacon.is_alert, "Warning beacon must be in alert mode during AgitatedHuffing");
    }

    // 5. Advance to Hunger 60.0 (WindowSlammed -> max angle ≈ +2.356 rad = +135 deg)
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
    }
    app.update();
    {
        let mut needle_q = app.world_mut().query::<&MooseHungerNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            (needle.current_angle - 2.356).abs() < 0.1,
            "Needle should be at far right red breach zone (≈ 2.356 rad) at hunger 60, got {}",
            needle.current_angle
        );
    }

    // 6. Test scale deflection when snack is placed in trough
    let snack = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.4, -12.0),
    )).id();
    app.update();

    {
        let mut scale_q = app.world_mut().query::<&MooseTroughScalePlate>();
        let scale = scale_q.iter(app.world()).next().unwrap();
        assert!(
            scale.current_deflection > 0.01,
            "Scale plate should deflect under the weight of food, got deflection {}",
            scale.current_deflection
        );
    }

    // Despawn snack (simulating consumption) and reset hunger to 0
    app.world_mut().despawn(snack);
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 0.0;
        hunger.stage = MooseHungerStage::BehindShutters;
    }
    app.update();

    {
        let mut needle_q = app.world_mut().query::<&MooseHungerNeedle>();
        let needle = needle_q.iter(app.world()).next().unwrap();
        assert!(
            (needle.current_angle - (-2.356)).abs() < 0.1,
            "Needle should reset to 0 (far left) after feeding, got {}",
            needle.current_angle
        );
        let mut beacon_q = app.world_mut().query::<&MooseHungerWarningBeacon>();
        let beacon = beacon_q.iter(app.world()).next().unwrap();
        assert!(!beacon.is_alert, "Warning beacon must return to calm state after feeding");
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
        hunger_dial::spawn_moose_hunger_dial_and_scale,
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
        spawn_moose_hunger_dial_and_scale,
        spawn_diegetic_screens_and_signs,
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

    // Verify entities were successfully spawned using WorkshopMaterials
    let mut entity_count = 0;
    for _ in app.world().iter_entities() {
        entity_count += 1;
    }
    assert!(entity_count > 20, "Spawning systems with WorkshopMaterials must instantiate world entities (got {})", entity_count);
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
    assert!(sign_3d_count >= 10, "Expected at least 10 in-world directional sign 3D text quads, found: {}", sign_3d_count);

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
    assert_eq!(graph.nodes.len(), 15, "Graph must define 15 workshop nodes");

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
        Transform::from_xyz(-6.5, 1.4, -10.0),
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
