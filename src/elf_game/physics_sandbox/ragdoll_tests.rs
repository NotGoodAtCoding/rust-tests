use super::*;
use crate::elf_game::{
    player::controller::{ElfPlayer, FirstPersonCamera},
    workshop::room::{CandleChandelier, TubeTravellingLump},
    wrapping::ToyType,
};

#[test]
fn test_ragdoll_npc_elf_assistants_spawning_and_limbs() {
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

    for (_, _, locked) in &elves {
        assert_eq!(
            **locked,
            LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
            "Walking elves must have X/Z rotation locked"
        );
    }

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

    let elf_entity = app.world_mut().spawn((
        ElfAssistant::new(1, "TestElf", vec![Vec3::new(3.0, 0.55, 0.0)]),
        Transform::from_xyz(0.0, 0.55, 0.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let vel = app.world().get::<Velocity>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Wandering);
        assert!(vel.linear.x > 0.5, "Elf must move towards waypoint along +X");
    }

    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 55.0;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let vel = app.world().get::<Velocity>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Panicking, "Agitated Moose must cause elf panic");
        assert!(vel.linear.z > 1.0, "Panicking elf must flee south (+Z) away from window");
    }

    let moose_entity = app.world_mut().spawn((
        MrMoose,
        Transform::from_xyz(0.0, 1.4, 0.5),
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

    app.world_mut().despawn(moose_entity);
    {
        let mut hunger = app.world_mut().resource_mut::<MooseHunger>();
        hunger.value = 0.0;
        hunger.stage = MooseHungerStage::BehindShutters;

        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.tumble_timer = 2.5;
        let mut vel = app.world_mut().get_mut::<Velocity>(elf_entity).unwrap();
        vel.linear = Vec3::ZERO;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Recovering, "Resting ragdoll must transition into Recovering");
    }

    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.recovery_timer = 1.0;
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let locked = app.world().get::<LockedAxes>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Wandering, "Fully recovered elf must resume wandering");
        assert_eq!(*locked, LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z, "Upright elf must relock tilt axes");
    }

    let player_entity = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.55, 0.3),
        ElfMovement {
            velocity: Vec3::new(7.0, 0.0, 0.0),
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

    {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_entity).unwrap();
        elf.state = ElfNpcState::Panicking;
    }
    app.update();
    {
        let arm_tf = app.world().get::<Transform>(l_arm_entity).unwrap();
        let (x, _, _) = arm_tf.rotation.to_euler(EulerRot::XYZ);
        assert!(
            x.abs() > 0.5,
            "Panicking arm must be flailed upwards overhead, got rotation {x}"
        );
    }
}

#[test]
fn test_elf_nav_graph_pathfinding_and_clearances() {
    let graph = ElfNavGraph::default_workshop_graph();
    assert!(graph.nodes.len() >= 15, "Graph must define at least 15 workshop nodes (has {})", graph.nodes.len());
    assert_eq!(graph.nodes[15].name, "PantryShelves");
    assert_eq!(graph.nodes[15].neighbors, vec![8]);

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
    assert!(
        (west_bypass.position.x - (-9.05)).abs() >= 1.5,
        "West bypass edge must clear spawner duct by at least 1.5m"
    );

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
    assert!(
        east_bypass.position.x - 10.47 >= 0.9,
        "East bypass edge must clear despawner chute by at least 0.9m"
    );

    let path_to_closet = graph.find_path(4, 12).expect("Path from Trough to Closet must exist");
    assert!(path_to_closet.len() >= 4);
    assert_eq!(*path_to_closet.first().unwrap(), 4);
    assert_eq!(*path_to_closet.last().unwrap(), 12);

    let path_to_stash = graph.find_path(0, 13).expect("Path from Workbench to Stash must exist");
    assert_eq!(*path_to_stash.first().unwrap(), 0);
    assert_eq!(*path_to_stash.last().unwrap(), 13);
    assert!(path_to_stash.contains(&7) || path_to_stash.contains(&8));

    assert_eq!(graph.closest_node(Vec3::new(0.0, 0.45, -1.9)), 0);
    assert_eq!(graph.closest_node(Vec3::new(-10.0, 0.45, 10.0)), 13);
}

#[test]
fn test_elf_grounded_floor_translation() {
    let mut app = App::new();
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_systems(Startup, setup_elf_assistants);
    app.update();

    let mut elf_q = app.world_mut().query::<(&ElfAssistant, &Transform, &Collider)>();
    for (elf, tf, _collider) in elf_q.iter(app.world()) {
        if elf.name == "Elf Sparky" {
            assert!(
                (tf.translation.y - 4.65).abs() < 0.001,
                "Elf Sparky perched on chandelier Y must be 4.65m, was {}",
                tf.translation.y
            );
        } else {
            assert!(
                (tf.translation.y - 0.45).abs() < 0.001,
                "Elf {} root Y must be 0.45m, was {}",
                elf.name,
                tf.translation.y
            );
        }
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
fn test_chandelier_sparky_easter_egg_unlock() {
    let mut app = App::new();
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);

    let chandelier_e = app.world_mut().spawn((
        CandleChandelier {
            base_intensity: 480_000.0,
            flicker_seed: 0.0,
        },
        Transform::from_xyz(0.0, 4.4, -4.5),
        Velocity::zero(),
        ExternalImpulse::default(),
    )).id();

    app.add_systems(Startup, setup_elf_assistants);
    app.add_systems(Update, (chandelier_sparky_easter_egg_system, elf_assistant_ai_system).chain());
    app.update();

    let mut sparky_q = app.world_mut().query::<(Entity, &ElfAssistant, &Transform, &LockedAxes, &ChandelierPerchedElf)>();
    let (sparky_e, sparky_elf, sparky_tf, sparky_locked, sparky_perched) = sparky_q
        .iter(app.world())
        .find(|(_, elf, _, _, _)| elf.name == "Elf Sparky")
        .expect("Elf Sparky must be spawned");

    assert_eq!(sparky_elf.state, ElfNpcState::PerchedAsleep);
    assert!(!sparky_perched.is_awakened);
    assert!((sparky_tf.translation.y - 4.65).abs() < 0.01);
    assert_eq!(*sparky_locked, LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z);

    app.update();
    let sparky_elf = app.world().get::<ElfAssistant>(sparky_e).unwrap();
    assert_eq!(sparky_elf.state, ElfNpcState::PerchedAsleep);

    {
        let mut c_impulse = app.world_mut().get_mut::<ExternalImpulse>(chandelier_e).unwrap();
        c_impulse.impulse = Vec3::new(8.0, 4.0, 0.0);
    }
    app.update();

    let sparky_elf = app.world().get::<ElfAssistant>(sparky_e).unwrap();
    let sparky_perched = app.world().get::<ChandelierPerchedElf>(sparky_e).unwrap();
    let sparky_locked = app.world().get::<LockedAxes>(sparky_e).unwrap();
    assert!(sparky_perched.is_awakened, "Sparky must be marked as awakened");
    assert_eq!(sparky_elf.state, ElfNpcState::RagdollTumble, "Sparky must tumble in dynamic ragdoll");
    assert_eq!(*sparky_locked, LockedAxes::empty(), "All rotation axes unlocked for 6-DOF tumble");

    {
        let mut sparky_mut = app.world_mut().get_mut::<ElfAssistant>(sparky_e).unwrap();
        sparky_mut.tumble_timer = sparky_mut.tumble_duration + 0.1;
        let mut vel = app.world_mut().get_mut::<Velocity>(sparky_e).unwrap();
        vel.linear = Vec3::new(0.1, 0.0, 0.1);
    }
    app.update();

    let sparky_elf = app.world().get::<ElfAssistant>(sparky_e).unwrap();
    assert_eq!(sparky_elf.state, ElfNpcState::Recovering, "Sparky transitions to Recovering");

    {
        let mut sparky_mut = app.world_mut().get_mut::<ElfAssistant>(sparky_e).unwrap();
        sparky_mut.recovery_timer = sparky_mut.recovery_duration + 0.1;
    }
    app.update();

    let sparky_elf = app.world().get::<ElfAssistant>(sparky_e).unwrap();
    let sparky_locked = app.world().get::<LockedAxes>(sparky_e).unwrap();
    assert_eq!(sparky_elf.state, ElfNpcState::Wandering, "Sparky recovers into active roaming elf pool");
    assert_eq!(*sparky_locked, LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z);
}

#[test]
fn test_elf_mischief_traits_assignment_and_lazy_slacking() {
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

    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        let vel = app.world().get::<Velocity>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Slacking, "Lazy elf must enter Slacking state");
        assert_eq!(vel.linear, Vec3::ZERO, "Slacking elf must stop moving");
    }

    {
        let mut t = app.world_mut().resource_mut::<Time>();
        t.advance_by(std::time::Duration::from_millis(50));
    }
    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Slacking);
    }

    let prop_entity = app.world_mut().spawn((
        Grabbable,
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
        Transform::from_xyz(-4.0, 3.0, 11.0),
        ElfMovement::default(),
        ElfHands::default(),
    )).id();

    app.update();

    let mut hat_q = app.world_mut().query::<(Entity, &ElfJobHat)>();
    let hats: Vec<(Entity, ElfJob)> = hat_q.iter(app.world()).map(|(e, h)| (e, h.job)).collect();
    assert!(!hats.is_empty(), "Hat rack on mezzanine must spawn initial random hat");

    let (wrapper_hat_e, expected_job) = hats.first().unwrap();
    let expected_job = *expected_job;
    {
        let mut hands = app.world_mut().get_mut::<ElfHands>(player_entity).unwrap();
        hands.right_grabbed = Some(*wrapper_hat_e);
    }

    let elf_entity = app.world_mut().spawn((
        ElfAssistant::new(1, "RecipientElf", vec![Vec3::ZERO]),
        Transform::from_xyz(-4.0, 3.0, 11.0),
        Velocity::default(),
    )).id();

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.assigned_job, Some(expected_job), "Elf must receive assigned job from hat");
        assert_eq!(elf.worn_hat_entity, Some(*wrapper_hat_e));

        let hands = app.world().get::<ElfHands>(player_entity).unwrap();
        assert_eq!(hands.right_grabbed, None, "Player hand must release the hat upon placing it");

        let hat = app.world().get::<ElfJobHat>(*wrapper_hat_e).unwrap();
        assert_eq!(hat.worn_by, Some(elf_entity));
    }
}

#[test]
fn test_elf_nav_graph_pantry_node_and_purposes() {
    let graph = ElfNavGraph::default_workshop_graph();

    assert!(graph.nodes.len() >= 16);
    let pantry_node = &graph.nodes[15];
    assert_eq!(pantry_node.name, "PantryShelves");
    assert_eq!(pantry_node.position, Vec3::new(-10.0, 0.45, 11.0));
    assert_eq!(pantry_node.purpose, Some(ElfNodePurpose::PantryShelves));

    assert!(pantry_node.neighbors.contains(&8));
    assert!(graph.nodes[8].neighbors.contains(&15));

    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::PantryShelves), Some(15));
    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::HatCloset), Some(12));
    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::KleptoStash), Some(13));
    assert_eq!(graph.find_node_by_purpose(ElfNodePurpose::MooseFeedingStation), Some(4));
    assert!(graph.find_node_by_purpose(ElfNodePurpose::Workbench).is_some());
    assert!(graph.find_node_by_purpose(ElfNodePurpose::MainTableTreatJar).is_some());

    let path = graph.find_path(0, 15).expect("Path from Workbench to Pantry must exist");
    assert_eq!(*path.first().unwrap(), 0);
    assert_eq!(*path.last().unwrap(), 15);
    assert_eq!(path[path.len() - 2], 8);

    let path_to_trough = graph.find_path(15, 4).expect("Path from Pantry to Trough must exist");
    assert_eq!(*path_to_trough.first().unwrap(), 15);
    assert_eq!(*path_to_trough.last().unwrap(), 4);
}

#[test]
fn test_moose_feeder_job_routine_loop() {
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

    let jar_entity = app.world_mut().spawn((
        MooseTreatJar { treat_count: 2, max_treats: 4 },
        Transform::from_xyz(-3.5, 0.98, -4.0),
    )).id();

    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::FetchingMooseTreat, "Elf must enter FetchingMooseTreat");
        assert_eq!(elf.target_node, 1, "Target node must be MainTableTreatJar (Node 1)");
    }

    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_entity).unwrap();
        tf.translation = Vec3::new(-3.7, 0.45, -4.0);
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::DeliveringMooseTreat, "Elf must transition to DeliveringMooseTreat");
        assert!(elf.carried_treat.is_some(), "Elf must be carrying a treat entity");
        assert_eq!(elf.target_node, 4, "Destination must be MooseFeedingStation (Node 4)");

        let jar = app.world().get::<MooseTreatJar>(jar_entity).unwrap();
        assert_eq!(jar.treat_count, 1, "Jar treat_count must be decremented from 2 to 1");
    }

    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_entity).unwrap();
        tf.translation = Vec3::new(-6.5, 0.45, -10.5);
    }
    app.update();

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

    let empty_jar = app.world_mut().spawn((
        MooseTreatJar { treat_count: 0, max_treats: 4 },
        Transform::from_xyz(-3.8, 0.98, -4.0),
    )).id();

    app.update();
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Panicking, "Elf must transition into Panicking");
        assert!(elf.missing_treats_panic, "missing_treats_panic flag must be set to true");
    }

    for _ in 0..10 {
        app.update();
    }
    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::Panicking, "Elf must remain in Panicking while treats are missing");
        assert!(elf.missing_treats_panic);
    }

    app.world_mut().despawn(empty_jar);
    app.world_mut().spawn((
        MooseTreatJar { treat_count: 4, max_treats: 4 },
        Transform::from_xyz(-3.6, 0.98, -4.0),
    ));

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::FetchingMooseTreat, "Elf must recover from panic and resume FetchingMooseTreat");
        assert!(!elf.missing_treats_panic, "missing_treats_panic must be cleared");
        assert_eq!(elf.target_node, 1, "Must target Node 1 to retrieve treat");
    }
}

#[test]
fn test_moose_feeder_knockdown_drops_treat_and_recovers() {
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

    app.world_mut().spawn((
        Grabbable,
        Transform::from_xyz(0.3, 0.45, 0.0),
        Velocity {
            linear: Vec3::new(-4.0, 0.0, 0.0),
            angular: Vec3::ZERO,
        },
    ));

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_entity).unwrap();
        assert_eq!(elf.state, ElfNpcState::RagdollTumble, "Impulse must send elf into RagdollTumble");
        assert!(elf.carried_treat.is_none(), "Carried treat must be dropped upon knockdown");
    }

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
    let mut app = App::new();
    let mut key_inputs = ButtonInput::<KeyCode>::default();
    key_inputs.press(KeyCode::KeyE);
    app.insert_resource(key_inputs);
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_systems(Update, elf_hat_assignment_system);

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
fn test_player_elf_collision_default_speed_vs_sprint_bhop_ragdoll() {
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

    let player_e = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.45, 0.3),
        ElfMovement {
            velocity: Vec3::new(6.0, 0.0, 0.0),
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

        assert_ne!(
            elf.state,
            ElfNpcState::RagdollTumble,
            "Elf must NOT ragdoll when player collides at default speed cap (6.0 m/s)"
        );
        assert!(
            locked.contains(LockedAxes::ROTATION_LOCKED_X),
            "Rotation axes must remain locked when hit at default speed cap"
        );
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
fn test_dynamic_waypoint_targeting_for_elves_and_npcs() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.init_resource::<ElfNavGraph>();
    app.add_systems(Update, elf_assistant_ai_system);

    let graph = app.world().get_resource::<ElfNavGraph>().unwrap();
    for node in &graph.nodes {
        assert!(
            node.radius >= 1.1,
            "Node {} radius must be expanded to >= 1.1m (got {})",
            node.name,
            node.radius
        );
    }

    let _player_e = app.world_mut().spawn((
        Transform::from_xyz(10.0, 0.45, 0.0),
        ElfMovement::default(),
    )).id();

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

    app.world_mut().entity_mut(elf_a_entity).insert(DynamicWaypointTarget::Player);
    app.update();

    let elf_after = app.world().get::<ElfAssistant>(elf_a_entity).unwrap();
    assert_eq!(elf_after.dynamic_target, Some(DynamicWaypointTarget::Player));

    {
        let mut elf_mut = app.world_mut().get_mut::<ElfAssistant>(elf_a_entity).unwrap();
        elf_mut.clear_dynamic_target();
        assert_eq!(elf_mut.dynamic_target, None);
    }
}

#[test]
fn test_elf_navigation_stuck_timer_and_vacuum_respawn() {
    let mut app = App::new();
    app.insert_resource(ElfNavGraph::default_workshop_graph());

    let elf_e = app.world_mut().spawn((
        ElfAssistant::new(3, "TestElf", vec![Vec3::ZERO]),
        Transform::from_xyz(0.0, 0.45, 0.0),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::default(),
    )).id();

    app.add_systems(Update, elf_assistant_ai_system);
    app.update();

    let initial_target = {
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_e).unwrap();
        elf.state = ElfNpcState::Wandering;
        elf.current_node = 0;
        elf.target_node = 3;
        elf.path = vec![0, 1, 2, 3];
        elf.path_step = 1;
        elf.target_node
    };

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

    let (failure_1, target_1) = {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        (elf.navigation_failure_count, elf.target_node)
    };
    assert_eq!(failure_1, 1, "Navigation failure count must increment to 1 after 10s stuck");
    assert_ne!(target_1, initial_target, "Elf must choose a different node after being stuck for 10s");

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

    let mut lump_q = app.world_mut().query::<&TubeTravellingLump>();
    let reverse_lump = lump_q.iter(app.world()).find(|l| l.speed < 0.0);
    assert!(reverse_lump.is_some(), "Reverse vacuum lump with negative speed must be spawned in tube");

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
        assert!((tf.translation.x - 5.5).abs() < 0.2, "Elf should be emerging near sending platform X ≈ 5.5");
        assert!((tf.translation.z - (-4.0)).abs() < 0.2, "Elf should be emerging near sending platform Z ≈ -4.0");
    }

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
    let graph = ElfNavGraph::default_workshop_graph();
    let stash_nodes = graph.find_nodes_by_purpose(ElfNodePurpose::KleptoStash);
    assert_eq!(stash_nodes.len(), 4, "Must have exactly 4 corner KleptoStash nodes");

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

    let sw = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x < -8.0 && graph.nodes[id].position.z > 8.0);
    let nw = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x < -8.0 && graph.nodes[id].position.z < -8.0);
    let ne = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x > 8.0 && graph.nodes[id].position.z < -8.0);
    let se = stash_nodes.iter().find(|&&id| graph.nodes[id].position.x > 8.0 && graph.nodes[id].position.z > 8.0);

    assert!(sw.is_some(), "Must have SW corner stash node");
    assert!(nw.is_some(), "Must have NW corner stash node");
    assert!(ne.is_some(), "Must have NE corner stash node");
    assert!(se.is_some(), "Must have SE corner stash node");

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

    let elf_pos = Vec3::new(-9.0, 0.45, 9.0);
    let elf_e = app.world_mut().spawn((
        klepto_elf,
        Transform::from_translation(elf_pos).looking_at(Vec3::new(-10.5, 0.45, 10.5), Vec3::Y),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    let present_pos = elf_pos + Vec3::new(0.3, 0.0, 0.2);
    let pres_e1 = app.world_mut().spawn((
        Present::default(),
        Transform::from_translation(present_pos),
        Velocity::default(),
    )).id();

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(elf.state, ElfNpcState::StashingPresent);
        assert_eq!(elf.carried_present, Some(pres_e1));
        assert_eq!(elf.target_node, 13);
    }

    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_e).unwrap();
        tf.translation = Vec3::new(-10.4, 0.45, 10.4);
        *tf = tf.looking_at(Vec3::new(-10.5, 0.45, 10.5), Vec3::Y);
    }

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let elf_tf = app.world().get::<Transform>(elf_e).unwrap();
        let pres_tf = app.world().get::<Transform>(pres_e1).unwrap();

        assert_eq!(elf.state, ElfNpcState::Inspecting, "Elf should switch to Inspecting after stashing");
        assert_eq!(elf.carried_present, None, "Present should no longer be carried");
        assert_eq!(elf.last_stashed_present, Some(pres_e1), "Elf must remember last stashed present");
        assert_eq!(elf.klepto_cooldown, 15.0, "Cooldown must be set to 15 seconds after stashing");

        let diff = pres_tf.translation - elf_tf.translation;
        let flat_dist = Vec2::new(diff.x, diff.z).length();
        assert!(flat_dist > 0.4 && flat_dist < 0.8, "Present should be placed in front of elf ~0.6m away, got {}", flat_dist);
    }

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
fn test_elf_navigates_pantry_door_opening_sequence() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ElfNavGraph::default());

    let door_entity = app.world_mut().spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        Velocity::zero(),
        ExternalImpulse::default(),
        PantryDoor::default(),
    )).id();

    let elf_e = app.world_mut().spawn((
        Transform::from_xyz(-11.0, 0.45, 5.5),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ElfAssistant::new(5, "PantryChefElf", vec![]),
    )).id();

    app.add_systems(Update, elf_assistant_ai_system);

    {
        let graph = app.world().resource::<ElfNavGraph>().clone();
        let mut elf = app.world_mut().get_mut::<ElfAssistant>(elf_e).unwrap();
        elf.navigate_to_pantry(Some(&graph));
        assert_eq!(elf.target_node, 15);
    }

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(
            elf.state,
            ElfNpcState::OpeningPantryDoor,
            "Elf must enter OpeningPantryDoor state when navigating to pantry with closed door"
        );
    }

    app.update();

    {
        let door = app.world().get::<PantryDoor>(door_entity).unwrap();
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert!(
            door.is_grabbed || elf.pantry_door_phase >= 1,
            "Elf must grab the door and begin pulling open"
        );
    }

    {
        let mut tf = app.world_mut().get_mut::<Transform>(door_entity).unwrap();
        tf.rotation = Quat::from_rotation_y(1.20);
    }

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

    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        assert_eq!(
            elf.pantry_door_phase, 3,
            "Elf should advance to phase 3 (entering pantry)"
        );
    }

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

    let player_e = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.45, 0.0),
        ElfHands {
            right_grabbed: Some(pipe_entity),
            ..default()
        },
    )).id();

    let elf_e = app.world_mut().spawn((
        Transform::from_xyz(0.5, 0.45, 0.5),
        ElfAssistant::new(1, "ThinkerElf", vec![]),
    )).id();

    app.add_systems(Update, (elf_hat_assignment_system, philosopher_pipe_system).chain());

    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();

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
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ElfNavGraph::default());

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

    let helper_e = app.world_mut().spawn((
        Transform::from_xyz(-1.0, 0.45, 7.5),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ElfAssistant::new(20, "HelperElf", vec![]),
    )).id();

    app.add_systems(Update, elf_assistant_ai_system);

    app.update();

    {
        let phil = app.world().get::<ElfAssistant>(phil_e).unwrap();
        assert_eq!(phil.target_node, 19, "Philosopher elf must target Mirror Node (19)");
    }

    {
        let mut tf = app.world_mut().get_mut::<Transform>(phil_e).unwrap();
        tf.translation = Vec3::new(-3.0, 0.45, 10.5);
    }

    app.update();

    {
        let phil = app.world().get::<ElfAssistant>(phil_e).unwrap();
        assert_eq!(
            phil.state,
            ElfNpcState::Philosophizing,
            "Philosopher elf at Mirror Node must enter Philosophizing state"
        );
    }

    {
        let mut tf = app.world_mut().get_mut::<ElfAssistant>(phil_e).unwrap();
        tf.recruit_timer = 10.5;
    }

    app.update();

    {
        let helper = app.world().get::<ElfAssistant>(helper_e).unwrap();
        assert!(helper.is_recruited, "Helper elf must be recruited by philosopher");
        assert_eq!(
            helper.dynamic_target,
            Some(DynamicWaypointTarget::Npc(phil_e)),
            "Recruited elf must navigate to philosopher elf"
        );
    }

    {
        let mut tf = app.world_mut().get_mut::<Transform>(helper_e).unwrap();
        tf.translation = Vec3::new(-2.8, 0.45, 10.2);
    }

    app.update();

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
fn test_hat_rack_mirror_and_respawn() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Startup, spawn_hat_closet);
    app.add_systems(Update, hat_rack_respawn_system);
    app.update();

    let Ok((rack_entity, rack)) = app.world_mut().query::<(Entity, &HatRack)>().single(app.world()) else {
        panic!("HatRack must be spawned on the mezzanine");
    };
    assert!(rack.current_hat.is_some(), "Hat Rack must spawn 1 random hat initially");
    let initial_hat = rack.current_hat.unwrap();

    let mirror_count = app.world_mut().query::<&HatStandMirror>().iter(app.world()).count();
    assert_eq!(mirror_count, 1, "Mirror must be spawned beside the Hat Stand");

    app.world_mut().entity_mut(initial_hat).insert(HeldByPlayer);
    let player_e = app.world_mut().spawn((
        ElfPlayer,
        ElfHands {
            right_grabbed: Some(initial_hat),
            ..default()
        },
        Transform::from_xyz(0.0, 1.2, 0.0),
    )).id();

    let cam_e = app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_xyz(0.0, 1.2, 0.0).looking_to(-Vec3::Z, Vec3::Y)),
    )).id();

    app.update();

    {
        let rack_check = app.world().get::<HatRack>(rack_entity).unwrap();
        assert!(rack_check.current_hat.is_none(), "Rack hat was taken off");
    }

    {
        let mut hands = app.world_mut().get_mut::<ElfHands>(player_e).unwrap();
        hands.right_grabbed = None;
        app.world_mut().entity_mut(initial_hat).remove::<HeldByPlayer>();
        let mut hat_tf = app.world_mut().get_mut::<Transform>(initial_hat).unwrap();
        hat_tf.translation = Vec3::new(10.0, 0.5, 0.0);

        let mut cam_gtf = app.world_mut().get_mut::<GlobalTransform>(cam_e).unwrap();
        let look_dir = (Vec3::new(-4.0, 4.6, 11.0) - Vec3::new(0.0, 1.2, 0.0)).normalize();
        *cam_gtf = GlobalTransform::from(Transform::from_xyz(0.0, 1.2, 0.0).looking_to(look_dir, Vec3::Y));
    }

    app.update();

    {
        let rack_check = app.world().get::<HatRack>(rack_entity).unwrap();
        assert!(rack_check.current_hat.is_none(), "Must NOT spawn while player is looking at the rack");
    }

    {
        let mut cam_gtf = app.world_mut().get_mut::<GlobalTransform>(cam_e).unwrap();
        *cam_gtf = GlobalTransform::from(Transform::from_xyz(0.0, 1.2, 0.0).looking_to(-Vec3::Z, Vec3::Y));
    }

    app.update();

    {
        let rack_check = app.world().get::<HatRack>(rack_entity).unwrap();
        assert!(rack_check.current_hat.is_some(), "Must spawn a new random hat on the rack when player is not looking and not holding a hat");
        let new_hat = rack_check.current_hat.unwrap();
        assert_ne!(new_hat, initial_hat, "New hat must be a newly spawned entity");
    }
}

#[test]
fn test_present_wrapper_elf_wrapping_and_vacuum_delivery() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(200));
    app.insert_resource(time);
    let graph = ElfNavGraph::default_workshop_graph();
    let workbench_pos = graph.nodes[graph.find_node_by_purpose(ElfNodePurpose::Workbench).unwrap()].position;
    let sending_pos = graph.nodes[graph.find_node_by_purpose(ElfNodePurpose::SendingPlatform).unwrap()].position;
    app.insert_resource(graph);
    app.add_systems(Update, (elf_assistant_ai_system, elf_ragdoll_limb_animation_system));

    let mut wrapper_elf = ElfAssistant::new(10, "WrapperElf", vec![workbench_pos]);
    wrapper_elf.assigned_job = Some(ElfJob::PresentWrapper);
    wrapper_elf.state = ElfNpcState::Wandering;

    let elf_e = app.world_mut().spawn((
        wrapper_elf,
        Transform::from_translation(workbench_pos),
        Velocity::default(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
    )).id();

    let arm_limb_e = app.world_mut().spawn((
        ElfRagdollLimb {
            elf_entity: elf_e,
            elf_id: 10,
            limb_type: ElfLimbType::RightArm,
            rest_local_translation: Vec3::new(-0.22, 0.0, 0.0),
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        Transform::from_xyz(-0.22, 0.0, 0.0),
    )).id();

    let bear_spec = WrappingSpecification::for_toy(ToyType::TeddyBear);
    let present_e = app.world_mut().spawn((
        Present::new(ToyType::TeddyBear),
        bear_spec.clone(),
        Transform::from_translation(workbench_pos + Vec3::new(0.4, 0.4, -0.6)),
        Velocity::default(),
    )).id();

    app.update();
    assert_eq!(app.world().get::<ElfAssistant>(elf_e).unwrap().state, ElfNpcState::WrappingPresent);

    for _ in 0..7 {
        app.world_mut().resource_mut::<Time<()>>().advance_by(std::time::Duration::from_millis(200));
        app.update();
    }

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let present = app.world().get::<Present>(present_e).unwrap();
        assert!(present.is_completed, "Wrapper elf must complete present wrapping at workbench");
        assert!(present.meets_specification(&bear_spec), "Present must satisfy toy-specific WrappingSpecification");
        assert_eq!(elf.carried_present, Some(present_e), "Elf must pick up completed present");
        assert_eq!(elf.state, ElfNpcState::DeliveringPresentToVacuum);
        assert_ne!(
            app.world().get::<Transform>(arm_limb_e).unwrap().rotation,
            Quat::IDENTITY,
            "Limb animation must run cleanly while wrapping/carrying"
        );
    }

    {
        let mut tf = app.world_mut().get_mut::<Transform>(elf_e).unwrap();
        tf.translation = sending_pos;
    }
    app.update();

    {
        let elf = app.world().get::<ElfAssistant>(elf_e).unwrap();
        let pres_tf = app.world().get::<Transform>(present_e).unwrap();
        assert_eq!(elf.carried_present, None, "Elf must release present at SendingPlatform");
        assert_eq!(elf.state, ElfNpcState::WrappingPresent, "Elf must loop back to WrappingPresent");
        let dist_flat = Vec2::new(pres_tf.translation.x - sending_pos.x, pres_tf.translation.z - sending_pos.z).length();
        assert!(dist_flat < 0.25, "Present must be deposited on SendingPlatform center");
    }
}
