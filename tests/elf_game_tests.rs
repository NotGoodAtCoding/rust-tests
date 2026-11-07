use std::collections::HashSet;
use bevy::prelude::*;
use rust_tests::elf_game::wrapping::{
    WrappingPlugin, Present, WrappingFeature, WrappingSpecification, WrappingTool, UseToolOnPresentMessage,
};
use rust_tests::elf_game::moose::{
    MoosePlugin, MooseHunger, MooseSnack, FeedMooseMessage,
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
fn test_moose_hunger_drain_and_feeding() {
    let mut app = App::new();
    app.add_plugins(bevy::time::TimePlugin);
    app.add_plugins(MoosePlugin);

    // Initial hunger is 90.0
    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.current, 90.0);
        assert!(!hunger.is_breached);
    }

    // Spawn a snack
    let snack_entity = app.world_mut().spawn(MooseSnack {
        nutrition_value: 45.0,
    }).id();

    // Simulate hunger drop by manually editing resource to 30.0
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.current = 30.0;
    }

    // Feed the moose
    app.world_mut().write_message(FeedMooseMessage { snack_entity });
    app.update();

    // Check hunger restored (30 + 45 = 75)
    let hunger = app.world().resource::<MooseHunger>();
    assert_eq!(hunger.current, 75.0);

    // Verify snack entity was consumed / despawned
    assert!(app.world().get_entity(snack_entity).is_err());
}

#[test]
fn test_moose_breach_when_starved() {
    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    // Initial hunger is 90.0, drain rate is 1.0.
    // Insert a mock Time<()> resource with 100 seconds delta so it drains 100 points
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs(100));
    app.insert_resource(time);

    app.update();

    let hunger = app.world().resource::<MooseHunger>();
    assert!(hunger.is_breached, "Moose must breach when hunger drops to 0");
    assert_eq!(hunger.current, 0.0);
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
