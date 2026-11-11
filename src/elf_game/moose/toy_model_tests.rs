use super::*;
use crate::elf_game::{
    moose::{MooseFeedingBucket, MoosePlugin, MooseCookiePlate},
    workshop::{
        events::MooseRampageTracker,
        stable_door::{StableDoorTopHalf, stable_door_system},
    },
};

#[test]
fn test_moose_window_shutters_rotate_inward_on_peek() {
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

    let moose_entity = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(-6.5, 1.4, -17.0).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
    )).id();

    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 0.5, 1.6).with_rotation(Quat::from_rotation_z(0.3)),
    )).id();

    // 1. BehindShutters (hunger 0.0)
    app.update();
    {
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
        let head_tf = app.world().get::<Transform>(head_entity).unwrap();
        assert_eq!(moose_tf.rotation, Quat::IDENTITY, "Moose must face +Z in BehindShutters");
        assert_eq!(moose_tf.translation.y, 1.8, "Moose Y must be 1.8 in BehindShutters");
        assert_eq!(head_tf.rotation, Quat::IDENTITY, "Moose head must be Quat::IDENTITY in BehindShutters");
        assert_eq!(head_tf.translation.y, 1.1, "Moose head Y must be 1.1 in BehindShutters");
    }

    // 2. PeekingThrough (hunger 35.0)
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

    // 3. AgitatedHuffing (hunger 55.0)
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

    // 4. WindowSlammed (hunger 60.0)
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
fn test_moose_feeding_trough_stage_wait_and_reset() {
    let mut app = App::new();
    app.insert_resource(MooseHunger {
        value: 30.0,
        stage: MooseHungerStage::PeekingThrough,
        ..default()
    });
    app.add_message::<FeedMooseMessage>();
    app.add_systems(Update, moose_feeding_trough_system);

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    app.update();

    assert!(
        app.world().get_entity(snack_entity).is_ok(),
        "Snack must remain in the food box while hunger is below 50"
    );
    let hunger = app.world().resource::<MooseHunger>();
    assert_eq!(hunger.value, 30.0, "Hunger should not reset before hunger reaches 50");

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 52.0;
        hunger.stage = MooseHungerStage::AgitatedHuffing;
    }
    app.update();

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
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, moose_scarf_physics_system);

    let moose_entity = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(-6.5, 1.8, -17.0),
    )).id();

    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    )).id();

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

    app.update();

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
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_mr_moose);
    app.update();

    let mut tree_q = app.world_mut().query::<(&WindowsillMiniTree, &Transform, &Grabbable)>();
    let tree = tree_q.iter(app.world()).next();
    assert!(tree.is_some(), "WindowsillMiniTree should be spawned");
    let (_, tree_tf, _) = tree.unwrap();
    assert!(
        (tree_tf.translation.z - (-12.85)).abs() < 0.2,
        "Mini tree must be located on windowsill (Z ≈ -12.85), got {}",
        tree_tf.translation.z
    );

    let mut cocoa_q = app.world_mut().query::<(&WindowsillHotCocoa, &Transform, &Grabbable)>();
    let cocoa = cocoa_q.iter(app.world()).next();
    assert!(cocoa.is_some(), "WindowsillHotCocoa mug should be spawned");
    let (_, cocoa_tf, _) = cocoa.unwrap();
    assert!(
        (cocoa_tf.translation.z - (-12.85)).abs() < 0.2,
        "Cocoa mug must be located on windowsill (Z ≈ -12.85), got {}",
        cocoa_tf.translation.z
    );

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
fn test_moose_bulky_fluffy_scarf_structure() {
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

    let mut scarf_q = app.world_mut().query::<(&MooseScarf, &Name)>();
    let scarf_entities: Vec<String> = scarf_q.iter(app.world()).map(|(_, n)| n.to_string()).collect();
    assert!(scarf_entities.len() >= 9, "Scarf must have collar and segments");
}

#[test]
fn test_moose_snappy_state_animations() {
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

    app.update();

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 20.5;
        hunger.stage = MooseHungerStage::PeekingThrough;
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

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
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
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, moose_scarf_physics_system);

    let moose_entity = app.world_mut().spawn((
        MrMoose,
        MooseScarfMount::default(),
        Transform::from_xyz(-6.5, 1.8, -17.0),
    )).id();

    let head_entity = app.world_mut().spawn((
        MooseHead,
        Transform::from_xyz(0.0, 1.1, 1.6),
    )).id();

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

    app.update();

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

    {
        let mut m_tf = app.world_mut().get_mut::<Transform>(moose_entity).unwrap();
        m_tf.translation.z = -14.5;
        m_tf.rotation = Quat::from_rotation_x(0.35);

        let mut h_tf = app.world_mut().get_mut::<Transform>(head_entity).unwrap();
        h_tf.translation.z = 3.6;
    }

    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
    app.update();

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
fn test_moose_gentle_eating_animation_on_time() {
    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 55.0;
        hunger.stage = MooseHungerStage::AgitatedHuffing;
    }

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

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

    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(500));
    }
    app.update();

    {
        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.value, 55.0, "Hunger timer must remain paused during eating animation");

        let mut q_head = app.world_mut().query_filtered::<&Transform, With<MooseHead>>();
        if let Ok(head_tf) = q_head.single(app.world()) {
            assert!(head_tf.translation.y < 0.8, "Moose head must reach down towards trough (got Y = {})", head_tf.translation.y);
            assert!(head_tf.translation.z > 2.4, "Moose head must extend forward over trough (got Z = {})", head_tf.translation.z);
        }
    }

    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(700));
    }
    app.update();

    assert!(
        app.world().get_entity(snack_entity).is_err(),
        "Snack must be consumed (despawned) halfway through gentle eating animation"
    );

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
    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
        hunger.breach_delay_remaining = Some(6.0);
    }

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let _snack = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

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
    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
        hunger.breach_delay_remaining = Some(6.0);
        hunger.late_treat_decision_override = Some(true);
    }

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    assert_eq!(app.world().resource::<MooseHunger>().stage, MooseHungerStage::DeliberatingLateTreat);

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

    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(400));
    }
    app.update();

    assert!(
        app.world().get_entity(snack_entity).is_err(),
        "Snack must be consumed during fast annoyed eating animation"
    );

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
    let mut app = App::new();
    app.add_plugins(MoosePlugin);

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 60.0;
        hunger.stage = MooseHungerStage::WindowSlammed;
        hunger.breach_delay_remaining = Some(4.0);
        hunger.late_treat_decision_override = Some(false);
    }

    app.world_mut().spawn((
        MooseFeedingTrough,
        Transform::from_xyz(-6.5, 0.3, -12.0),
    ));

    let snack_entity = app.world_mut().spawn((
        MooseSnack { nutrition_value: 60.0 },
        Transform::from_xyz(-6.5, 0.45, -12.0),
    )).id();

    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    assert_eq!(app.world().resource::<MooseHunger>().stage, MooseHungerStage::DeliberatingLateTreat);

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
        assert!(
            app.world().get_entity(snack_entity).is_ok(),
            "Rejected snack must NOT be consumed"
        );
    }

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

    {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
    }
    app.update();

    assert_eq!(app.world().resource::<MooseHunger>().stage, MooseHungerStage::DeliberatingLateTreat);

    app.world_mut().despawn(snack_entity);

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
fn test_moose_no_shutters_new_resting_rampage_pathing_and_door_slam() {
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

    let mut shutter_q = app.world_mut().query::<&WindowShutters>();
    assert_eq!(shutter_q.iter(app.world()).count(), 0, "Previous double shutter window must be removed");

    let mut moose_q = app.world_mut().query::<(&MrMoose, &Transform)>();
    let (_, moose_tf) = moose_q.iter(app.world()).next().expect("Mr Moose must exist");
    assert!((moose_tf.translation.x - (-2.0)).abs() < 0.05, "Moose X must be -2.0, got {}", moose_tf.translation.x);
    assert!((moose_tf.translation.y - 1.8).abs() < 0.05, "Moose Y must be 1.8");
    assert!((moose_tf.translation.z - (-17.0)).abs() < 0.05, "Moose Z must be -17.0 resting outside door");

    let tracker = MooseRampageTracker::default();
    assert_eq!(tracker.waypoints[0], Vec3::new(-2.0, 1.4, -10.0), "Leap into workshop at X = -2.0");
    assert_eq!(tracker.waypoints[5], Vec3::new(-2.0, 1.4, -4.0), "Charge back to door at X = -2.0");
    assert_eq!(tracker.waypoints[6], Vec3::new(-2.0, 1.4, -10.0), "Leap back out door at X = -2.0");
    assert_eq!(tracker.waypoints[7], Vec3::new(-2.0, 1.8, -17.0), "Retreat to resting area at X = -2.0");

    let mut trough_q = app.world_mut().query::<(&MooseFeedingTrough, &Transform)>();
    let (_, trough_tf) = trough_q.iter(app.world()).next().expect("Trough/Bucket must exist");
    assert!((trough_tf.translation.x - (-0.75)).abs() < 0.05, "Plate must be on right side of door at X = -0.75");
    assert!((trough_tf.translation.y - 0.70).abs() < 0.1, "Plate must be on table at Y ≈ 0.70");
    assert!((trough_tf.translation.z - (-12.15)).abs() < 0.05, "Plate must be at Z = -12.15");

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

    for _ in 0..10 {
        door_app.update();
    }
    let top_open = door_app.world().get::<StableDoorTopHalf>(top_door).unwrap();
    assert!(top_open.current_angle > 0.5, "Top of stable door must open as Moose pokes head in, got {}", top_open.current_angle);

    {
        let mut h = door_app.world_mut().resource_mut::<MooseHunger>();
        h.stage = MooseHungerStage::WindowSlammed;
    }
    for _ in 0..5 {
        door_app.update();
    }
    let top_shut = door_app.world().get::<StableDoorTopHalf>(top_door).unwrap();
    assert_eq!(top_shut.current_angle, 0.0, "Top of stable door must slam shut when Moose goes back");
}

#[test]
fn test_feeding_bucket_and_eating_animation() {
    use crate::elf_game::{assets::AssetsPlugin, workshop::room::setup_workshop_scene};

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_plugins(AssetsPlugin);
    app.add_systems(Startup, (setup_workshop_scene, spawn_mr_moose));
    app.update();

    let mut bucket_q = app.world_mut().query::<(&MooseFeedingBucket, &MooseFeedingTrough, &Transform)>();
    let (_, _, b_tf) = bucket_q.iter(app.world()).next().expect("Feeding bucket must exist");
    assert!((b_tf.translation.x - (-0.75)).abs() < 0.15, "Bucket must be on the right side of the double door (X ≈ -0.75)");
    assert!((b_tf.translation.z - (-12.15)).abs() < 0.15, "Bucket must be at Z ≈ -12.15");

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
}

#[test]
fn test_moose_body_collision_with_scarf() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, moose_scarf_physics_system);

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

    let penetrating_local_pos = Vec3::new(-0.24, 0.0, 0.5);
    let seg_entity = app.world_mut().spawn((
        MooseScarfSegment {
            index: 0,
            rest_offset: penetrating_local_pos,
            velocity: Vec3::new(0.0, 0.0, -2.0),
        },
        Collider::cuboid(0.19, 0.14, 0.11),
        Transform::from_translation(Vec3::new(0.0, 1.8, 0.0) + penetrating_local_pos),
    )).id();

    app.update();

    let seg_tf = app.world().get::<Transform>(seg_entity).unwrap();
    let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
    let local_z = (moose_tf.rotation.inverse() * (seg_tf.translation - moose_tf.translation)).z;

    assert!(
        local_z >= 1.40,
        "Scarf segment must be resolved outside the Moose chest (Z >= 1.40), got local Z = {}",
        local_z
    );

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
fn test_festive_cookie_plate_and_table() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_mr_moose);
    app.update();

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

    let mut plate_q = app.world_mut().query::<(&MooseCookiePlate, &MooseFeedingTrough, &Transform, &Friction, &Children)>();
    let (_, _, plate_tf, friction, children) = plate_q.iter(app.world()).next().expect("FestiveCookiePlate must exist");
    assert!((plate_tf.translation.x - (-0.75)).abs() < 0.1);
    assert!((plate_tf.translation.z - (-12.15)).abs() < 0.1);
    assert!((plate_tf.translation.y - 0.70).abs() < 0.1, "Plate must sit on table at Y ≈ 0.70");
    assert!(friction.coefficient >= 3.0, "Cookie plate must have high friction (>= 3.0) to make treats stick, got {}", friction.coefficient);

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
}
