use super::*;
use crate::elf_game::{
    assets::AssetsPlugin,
    core::GameState,
    player::{
        controller::ElfPlayer,
        hands::{GROUP_ELF, GROUP_PLAYER, GROUP_RAMP, GROUP_STAIRS},
    },
    ui::MenuState,
    workshop::pantry::{MezzanineArea, PantryWing, StairStep, StaircaseSmoothRamp, UnusedRoom},
    wrapping::WrappingFeature,
};

#[test]
fn test_conveyor_cube_spawner_cycles_toy_types() {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_resource(MenuState::default());

    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    let spawner = ConveyorSpawnerTimer {
        timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        spawn_count: 0,
    };
    app.insert_resource(spawner);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Update, conveyor_cube_spawner_system);

    // Initial update - no ticks finished
    app.update();
    let count_init = app.world_mut().query::<&ToyType>().iter(app.world()).count();
    assert_eq!(count_init, 0);

    // Advance 1s -> Spawn 1 (WoodenTrain)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(1050));
    }
    app.update();
    let toys1: Vec<ToyType> = app.world_mut().query::<&ToyType>().iter(app.world()).copied().collect();
    assert_eq!(toys1.len(), 1);
    assert_eq!(toys1[0], ToyType::WoodenTrain);

    // Advance 1s -> Spawn 2 (TeddyBear)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(1050));
    }
    app.update();
    let toys2: Vec<ToyType> = app.world_mut().query::<&ToyType>().iter(app.world()).copied().collect();
    assert_eq!(toys2.len(), 2);
    assert_eq!(toys2[1], ToyType::TeddyBear);

    // Advance 1s -> Spawn 3 (TinRobot)
    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(1050));
    }
    app.update();
    let toys3: Vec<ToyType> = app.world_mut().query::<&ToyType>().iter(app.world()).copied().collect();
    assert_eq!(toys3.len(), 3);
    assert_eq!(toys3[2], ToyType::TinRobot);

    // Verify presents have corresponding toy_names
    let presents: Vec<Present> = app.world_mut().query::<&Present>().iter(app.world()).cloned().collect();
    assert_eq!(presents[0].toy_name, "Wooden Train");
    assert_eq!(presents[1].toy_name, "Teddy Bear");
    assert_eq!(presents[2].toy_name, "Tin Robot");
}

#[test]
fn test_conveyor_despawn_chute() {
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
fn test_chandelier_candle_flicker() {
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
fn test_vaulted_ceiling_steep_a_frame_geometry() {
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

                let (_, _, roll) = tf.rotation.to_euler(EulerRot::XYZ);
                assert!(
                    roll > 0.5,
                    "West roof slope must have steep positive Z rotation (> 0.5 rad / ~28 deg), got {}",
                    roll
                );

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
fn test_above_floor_despawner_and_curved_metal_duct() {
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
    let belt_cube = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(8.8, 0.75, 3.0),
    )).id();

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

    app.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(3.8, 0.95, -2.8),
    ));

    app.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(3.8, 1.0, -2.5),
    ));

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

    {
        let mut tf = app.world_mut().get_mut::<Transform>(loose_physics_prop).unwrap();
        tf.translation.y = 3.8;
    }
    app.update();

    assert!(app.world().get_entity(loose_physics_prop).is_err(), "Physics object must despawn upon hitting vacuum despawner");
    let lump_count = app.world_mut().query_filtered::<Entity, With<TubeTravellingLump>>().iter(app.world()).count();
    assert!(lump_count >= 1, "Vacuum tube must have travelling lump effect showing object going through it");
}

#[test]
fn test_vacuum_tractor_beam_cone_gravity_draw_and_no_verification() {
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

    let raw_cube = app.world_mut().spawn((
        Transform::from_xyz(5.7, 0.3, -4.0),
        Velocity::zero(),
        Name::new("RawWoodenCube"),
    )).id();

    let random_tool = app.world_mut().spawn((
        Transform::from_xyz(5.5, 0.3, -3.8),
        Velocity::zero(),
        Name::new("HeavyWrench"),
    )).id();

    {
        let mut p = app.world_mut().get_mut::<PresentSendingPlatform>(platform_entity).unwrap();
        p.is_active = true;
        p.suction_timer = 6.0;
    }
    app.update();

    let cone_vis = app.world().get::<Visibility>(cone_entity).unwrap();
    assert_eq!(*cone_vis, Visibility::Inherited, "Active tractor beam cone must be visible");

    for &entity in &[uncompleted_present, raw_cube, random_tool] {
        let vel = app.world().get::<Velocity>(entity).unwrap();
        assert!(
            vel.linear.y > 0.0,
            "Physics object without completion must be pulled upward by tractor beam: got vy = {}",
            vel.linear.y
        );
    }

    for &entity in &[uncompleted_present, raw_cube, random_tool] {
        let mut tf = app.world_mut().get_mut::<Transform>(entity).unwrap();
        tf.translation.y = 3.6;
    }
    app.update();

    assert!(app.world().get_entity(uncompleted_present).is_err(), "Uncompleted present must be ingested without verification");
    assert!(app.world().get_entity(raw_cube).is_err(), "Raw cube without Present must be ingested without verification");
    assert!(app.world().get_entity(random_tool).is_err(), "Random tool must be ingested without verification");

    let mut lump_q = app.world_mut().query::<&TubeTravellingLump>();
    let lumps: Vec<&TubeTravellingLump> = lump_q.iter(app.world()).collect();
    assert_eq!(lumps.len(), 3, "3 TubeTravellingLumps must be spawned for the 3 ingested objects");
}

#[test]
fn test_vacuum_proportional_distance_suction_and_cylinder_area() {
    let vacuum_cylinder = VacuumTractorBeamCone::new(
        Vec3::new(5.5, 0.05, -4.0),
        Vec3::new(5.5, 4.0, -4.0),
        1.45,
        1.45,
    );

    for y in [0.2, 1.5, 2.5, 3.8] {
        assert!(
            vacuum_cylinder.contains_point(Vec3::new(5.5 + 1.2, y, -4.0)),
            "Point at y={} inside radius must be within cylinder area",
            y
        );
        assert!(
            !vacuum_cylinder.contains_point(Vec3::new(5.5 + 1.6, y, -4.0)),
            "Point at y={} outside radius must NOT be within cylinder area",
            y
        );
    }

    assert!(!vacuum_cylinder.contains_point(Vec3::new(5.5, -0.6, -4.0)), "Point below floor must be excluded");
    assert!(!vacuum_cylinder.contains_point(Vec3::new(5.5, 5.0, -4.0)), "Point above despawner must be excluded");

    let p_far = Vec3::new(5.5, 0.0, -4.0);
    let p_mid = Vec3::new(5.5, 2.0, -4.0);
    let p_near = Vec3::new(5.5, 3.0, -4.0);

    let a_far_total = vacuum_cylinder.compute_gravity_draw(p_far);
    let a_mid_total = vacuum_cylinder.compute_gravity_draw(p_mid);
    let a_near_total = vacuum_cylinder.compute_gravity_draw(p_near);

    let a_far_suction = a_far_total - Vec3::Y * 9.81;
    let a_mid_suction = a_mid_total - Vec3::Y * 9.81;
    let a_near_suction = a_near_total - Vec3::Y * 9.81;

    let mag_far = a_far_suction.length();
    let mag_mid = a_mid_suction.length();
    let mag_near = a_near_suction.length();

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

    app.world_mut().spawn((
        SendPlatformButton,
        Transform::from_xyz(3.8, 0.95, -2.8),
    ));

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
        Transform::from_xyz(3.81, 1.02, -2.79),
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
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyE);
    }
    app.update();
    assert!(app.world().get::<PresentSendingPlatform>(platform_e).unwrap().is_active);

    {
        let mut p = app.world_mut().get_mut::<PresentSendingPlatform>(platform_e).unwrap();
        p.active_duration = 9.9;
    }
    {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_millis(200));
    }
    app.update();

    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must automatically turn OFF after being on for 10 seconds");
    assert!(p.cooldown_timer > 0.0, "Cooldown timer must be active after 10s auto shutoff (got {})", p.cooldown_timer);

    {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_millis(400));
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must remain OFF during cooldown even if button is held");

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

    app.world_mut().spawn((
        Transform::from_xyz(3.8, 1.0, -2.8),
        Velocity::zero(),
    ));

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
    let p_tf = app.world().get::<Transform>(player_e).unwrap();
    assert!(p_tf.translation.y < 10.0, "Player must not teleport before 3 seconds");

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
fn test_candle_particles_smoke_and_ascending_sparks() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    app.add_systems(Update, (candle_particle_emitter_system, candle_particle_system));

    app.world_mut().spawn((
        Transform::from_xyz(0.0, 4.4, -4.5),
        GlobalTransform::from(Transform::from_xyz(0.0, 4.4, -4.5)),
        CandleFlameEmitter::default(),
    ));

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

    let (smoke_tf, smoke_p) = q_particles
        .iter(app.world())
        .find(|(_, p)| p.kind == CandleParticleKind::Smoke)
        .unwrap();
    assert!(smoke_p.velocity.y > 0.0, "Smoke must drift upward (got vy = {})", smoke_p.velocity.y);
    assert_eq!(smoke_p.start_scale, 0.025);
    assert_eq!(smoke_p.end_scale, 0.075);
    let initial_y = smoke_tf.translation.y;

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

    let (smoke_tf_later, _) = q_particles
        .iter(app.world())
        .find(|(_, p)| p.kind == CandleParticleKind::Smoke)
        .unwrap();
    assert!(smoke_tf_later.translation.y > initial_y, "Smoke must ascend over time");

    {
        let mut t = app.world_mut().resource_mut::<Time<()>>();
        t.advance_by(std::time::Duration::from_millis(2000));
    }
    app.update();
}

#[test]
fn test_chandelier_physics_swinging_and_joints() {
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

    let chandeliers: Vec<(Entity, Transform, RigidBody, ExternalImpulse)> = {
        let mut q = app.world_mut().query_filtered::<(Entity, &Transform, &RigidBody, &ExternalImpulse), With<CandleChandelier>>();
        q.iter(app.world()).map(|(e, tf, rb, imp)| (e, *tf, *rb, *imp)).collect()
    };
    assert_eq!(chandeliers.len(), 2, "Must spawn 2 chandeliers (North & South)");
    for (_, tf, rb, _) in &chandeliers {
        assert_eq!(*rb, RigidBody::Dynamic, "Chandeliers must be Dynamic physics bodies to swing");
        assert_eq!(tf.translation.y, 4.4, "Chandelier hub resting height must be Y = 4.4m");
    }

    let anchors: Vec<(Entity, Transform, RigidBody)> = {
        let mut q = app.world_mut().query_filtered::<(Entity, &Transform, &RigidBody), With<ChandelierCeilingAnchor>>();
        q.iter(app.world()).map(|(e, tf, rb)| (e, *tf, *rb)).collect()
    };
    assert_eq!(anchors.len(), 2, "Must spawn 2 fixed ceiling anchors for chandelier chains");
    for (_, tf, rb) in &anchors {
        assert_eq!(*rb, RigidBody::Fixed, "Ceiling mount anchors must be Fixed rigid bodies");
        assert_eq!(tf.translation.y, 5.2, "Ceiling mount anchor must be positioned at tie beam apex Y = 5.2m");
    }

    let joints: Vec<&ImpulseJoint> = {
        let mut q = app.world_mut().query::<&ImpulseJoint>();
        q.iter(app.world()).collect()
    };
    assert!(joints.len() >= 2, "Chandeliers must contain ImpulseJoint linking them to ceiling anchors");

    let (c_entity, _, _, _) = chandeliers[0];
    let chandelier_z = chandeliers[0].1.translation.z;

    app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(0.0, 1.8, chandelier_z + 2.0),
    ));

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
fn test_pause_freezes_workshop_timers_vacuum_and_spawner() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    app.insert_resource(MenuState::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_resource(ConveyorSpawnerTimer {
        timer: Timer::from_seconds(2.0, TimerMode::Repeating),
        ..default()
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

    app.world_mut().resource_mut::<MenuState>().is_open = true;

    for _ in 0..50 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(100));
        app.update();
    }

    let spawner = app.world().resource::<ConveyorSpawnerTimer>();
    assert_eq!(
        spawner.timer.elapsed_secs(), 0.0,
        "Conveyor cube spawner timer must not tick while paused"
    );

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
fn test_vacuum_button_activation_pedestal_player_inputs_and_physics_props() {
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

    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release(MouseButton::Left);
    }
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must turn off when click is released");

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

    {
        let mut p_tf = app.world_mut().get_mut::<Transform>(player_e).unwrap();
        p_tf.translation = Vec3::new(0.0, 1.0, 0.0);
    }

    let prop_on_button = app.world_mut().spawn((
        Transform::from_xyz(3.8, 1.05, -2.8),
        Velocity::zero(),
    )).id();
    app.update();

    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(p.is_active, "Vacuum must activate when physics object rests on button, even without Velocity component");

    app.world_mut().despawn(prop_on_button);
    app.update();
    let p = app.world().get::<PresentSendingPlatform>(platform_e).unwrap();
    assert!(!p.is_active, "Vacuum must turn off when physics object is removed from button");
}

#[test]
fn test_pause_allows_lighting_flicker_and_particles_to_continue() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    let mut time_real = Time::<bevy::time::Real>::default();
    time_real.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time_real);

    app.insert_resource(MenuState { is_open: true, ..default() });
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

    let min_i = intensities.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_i = intensities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max_i - min_i > 1_000.0,
        "Chandelier candle lighting must continue flickering dynamically while game is paused"
    );

    let mut q_particles = app.world_mut().query::<(&Transform, &CandleParticle)>();
    let count = q_particles.iter(app.world()).count();
    assert!(
        count >= 1,
        "Candle particle emitter must continue spawning particles while game is paused"
    );
}

#[test]
fn test_candle_lighting_flicker_stabilization_and_config() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);

    let mut time_real = Time::<bevy::time::Real>::default();
    time_real.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time_real);

    app.init_resource::<CandleLightingConfig>();
    app.add_systems(Update, chandelier_candle_flicker_system);

    let base_intensity = 480_000.0;
    let chandelier_e = app.world_mut().spawn((
        CandleChandelier {
            base_intensity,
            flicker_seed: 0.5,
        },
        PointLight {
            intensity: base_intensity,
            ..default()
        },
    )).id();

    for _ in 0..20 {
        {
            let mut tr = app.world_mut().resource_mut::<Time<bevy::time::Real>>();
            tr.advance_by(std::time::Duration::from_millis(50));
        }
        app.update();

        let light = app.world().get::<PointLight>(chandelier_e).unwrap();
        assert!(
            light.intensity >= base_intensity * 0.985 && light.intensity <= base_intensity * 1.015,
            "Light intensity {} exceeded gentle breathing range [{}, {}]",
            light.intensity,
            base_intensity * 0.985,
            base_intensity * 1.015
        );
    }

    {
        let mut cfg = app.world_mut().resource_mut::<CandleLightingConfig>();
        cfg.flicker_enabled = false;
    }

    for _ in 0..10 {
        {
            let mut tr = app.world_mut().resource_mut::<Time<bevy::time::Real>>();
            tr.advance_by(std::time::Duration::from_millis(50));
        }
        app.update();

        let light = app.world().get::<PointLight>(chandelier_e).unwrap();
        assert_eq!(
            light.intensity, base_intensity,
            "Light intensity must be strictly base_intensity when flicker is disabled"
        );
    }
}

#[test]
fn test_climbable_windows_geometry() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_plugins(AssetsPlugin);
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

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
fn test_tractor_beam_player_force_and_activation_button_distance() {
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

    let player_e = app2.world_mut().spawn((
        ElfMovement::default(),
        Transform::from_xyz(5.5, 0.2, -4.0),
    )).id();

    app2.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app2.update();

    let plat = app2.world().get::<PresentSendingPlatform>(plat_e).unwrap();
    assert!(
        !plat.is_active,
        "Player standing in the tractor beam area MUST NOT be able to activate the tractor beam button"
    );

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

    app2.world_mut().spawn((
        Transform::from_xyz(1.8, 1.0, -1.8),
        Velocity::zero(),
    ));
    app2.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);

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
fn test_back_wall_subrooms_staircase_and_mezzanine() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, setup_workshop_scene);
    app.update();

    // 1. Verify Pantry subroom and at least 2 unused rooms along the back wall
    let pantry_count = app.world_mut().query::<&PantryWing>().iter(app.world()).count();
    assert_eq!(pantry_count, 1, "Pantry subroom must be spawned on the back wall");

    let unused_rooms: Vec<usize> = app
        .world_mut()
        .query::<&UnusedRoom>()
        .iter(app.world())
        .map(|r| r.room_index)
        .collect();
    assert!(unused_rooms.len() >= 2, "Back wall must contain at least 2 unused rooms");
    assert!(unused_rooms.contains(&1));
    assert!(unused_rooms.contains(&2));

    // 2. Verify Staircase: 12 steps and smooth collision ramp
    let steps_count = app.world_mut().query::<(&StairStep, &CollisionGroups)>().iter(app.world()).count();
    assert_eq!(steps_count, 12, "Staircase must contain 12 stepped stairs");

    for (_, groups) in app.world_mut().query::<(&StairStep, &CollisionGroups)>().iter(app.world()) {
        assert_eq!(groups.memberships, GROUP_STAIRS);
        assert_eq!(groups.filters & GROUP_PLAYER, Group::NONE, "Stair steps must not collide with player");
        assert_eq!(groups.filters & GROUP_ELF, Group::NONE, "Stair steps must not collide with elves");
    }

    let Ok((_ramp, ramp_groups)) = app.world_mut().query::<(&StaircaseSmoothRamp, &CollisionGroups)>().single(app.world()) else {
        panic!("StaircaseSmoothRamp must exist");
    };
    assert_eq!(ramp_groups.memberships, GROUP_RAMP);
    assert_eq!(ramp_groups.filters, GROUP_PLAYER, "Smooth ramp must collide exclusively with the player");

    // 3. Verify Mezzanine area: wood plank floor and railings
    let Ok((mezzanine, _)) = app.world_mut().query::<(&MezzanineArea, &Transform)>().single(app.world()) else {
        panic!("MezzanineArea must exist");
    };
    assert_eq!(mezzanine.floor_height, 3.0, "Mezzanine floor must be elevated at Y = 3.0");

    let names: Vec<String> = app.world_mut().query::<&Name>().iter(app.world()).map(|n| n.to_string()).collect();
    assert!(names.contains(&"MezzanineFloor".to_string()));
    assert!(names.contains(&"MezzanineRailing_Left".to_string()));
    assert!(names.contains(&"MezzanineRailing_Right".to_string()));
}
