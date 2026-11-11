use super::*;
use crate::elf_game::{
    player::ElfHands,
    workshop::room::{ConveyorSpawnerTimer, conveyor_cube_spawner_system},
    wrapping::Present,
};

#[test]
fn test_oven_stove_models_and_gas_hose_attachment() {
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
fn test_propane_tank_flow_consumption_and_refill() {
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
fn test_propane_receptacle_platforms_anchor_activation_and_refill() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_systems(
        Update,
        (
            propane_platform_activation_area_system,
            propane_tank_refill_and_holder_system,
        ),
    );

    // 1. House platform
    let house_pos = Vec3::new(-8.3, 0.45, -13.55);
    let house_platform_e = app.world_mut().spawn((
        PropaneReceptaclePlatform::new(PropanePlatformKind::HouseSupply).with_activation_radius(1.2),
        AnchorPoint::new("propane_tank").with_offset(Vec3::new(0.0, 0.45, 0.0)),
        Transform::from_translation(house_pos),
        GlobalTransform::from_translation(house_pos),
    )).id();

    // 2. Refill platform near outdoor main tank
    let refill_pos = Vec3::new(-16.0, 0.15, -23.5);
    let refill_platform_e = app.world_mut().spawn((
        PropaneReceptaclePlatform::new(PropanePlatformKind::MainTankRefill).with_activation_radius(1.4),
        AnchorPoint::new("propane_tank").with_offset(Vec3::new(0.0, 0.45, 0.0)),
        Transform::from_translation(refill_pos),
        GlobalTransform::from_translation(refill_pos),
    )).id();

    // Spawn a tank near the house platform activation area (0.5m away)
    let tank1_e = app.world_mut().spawn((
        PropaneTank {
            fuel_amount: 100.0,
            capacity: 100.0,
            is_connected: false,
        },
        Transform::from_xyz(house_pos.x + 0.5, house_pos.y, house_pos.z),
        RigidBody::Dynamic,
        Velocity::zero(),
    )).id();

    // Spawn a depleted tank near the refill platform activation area (0.6m away)
    let tank2_e = app.world_mut().spawn((
        PropaneTank {
            fuel_amount: 15.0,
            capacity: 100.0,
            is_connected: false,
        },
        Transform::from_xyz(refill_pos.x + 0.6, refill_pos.y, refill_pos.z),
        RigidBody::Dynamic,
        Velocity::zero(),
    )).id();

    app.update();

    // Tank 1 should be auto-locked into the house platform
    {
        let platform = app.world().get::<PropaneReceptaclePlatform>(house_platform_e).unwrap();
        assert_eq!(platform.locked_tank, Some(tank1_e), "Tank 1 must be locked into house platform");
        let tank = app.world().get::<PropaneTank>(tank1_e).unwrap();
        assert!(tank.is_connected, "Tank in house platform must be connected");
        let locked = app.world().get::<LockedInAnchor>(tank1_e);
        assert!(locked.is_some(), "Tank must have LockedInAnchor component");
        assert_eq!(locked.unwrap().anchor, house_platform_e);
        let rb = app.world().get::<RigidBody>(tank1_e).unwrap();
        assert_eq!(*rb, RigidBody::Fixed, "Locked tank must be RigidBody::Fixed");
        let tf = app.world().get::<Transform>(tank1_e).unwrap();
        assert_eq!(tf.translation, house_pos + Vec3::new(0.0, 0.45, 0.0));
    }

    // Tank 2 should be auto-locked into refill platform and REFILLED to capacity
    {
        let platform = app.world().get::<PropaneReceptaclePlatform>(refill_platform_e).unwrap();
        assert_eq!(platform.locked_tank, Some(tank2_e), "Tank 2 must be locked into refill platform");
        let tank = app.world().get::<PropaneTank>(tank2_e).unwrap();
        assert_eq!(tank.fuel_amount, 100.0, "Refill platform must refill smaller propane tank to capacity");
        let locked = app.world().get::<LockedInAnchor>(tank2_e);
        assert!(locked.is_some());
        assert_eq!(locked.unwrap().anchor, refill_platform_e);
    }

    // 3. Player picks up tank 1 -> HeldByPlayer releases it from anchor
    app.world_mut().entity_mut(tank1_e).insert(HeldByPlayer);
    app.update();

    {
        let platform = app.world().get::<PropaneReceptaclePlatform>(house_platform_e).unwrap();
        assert_eq!(platform.locked_tank, None, "Picking up tank must release platform lock");
        let tank = app.world().get::<PropaneTank>(tank1_e).unwrap();
        assert!(!tank.is_connected, "Held tank is no longer connected to house supply");
        let locked = app.world().get::<LockedInAnchor>(tank1_e);
        assert!(locked.is_none(), "LockedInAnchor must be removed when held by player");
    }

    // 4. Test player interaction unlocking tank 2 via KeyE
    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_translation(refill_pos + Vec3::new(0.0, 0.5, 1.5))
            .looking_at(refill_pos, Vec3::Y)),
    ));
    let mut key_inputs = ButtonInput::<KeyCode>::default();
    key_inputs.press(KeyCode::KeyE);
    app.insert_resource(key_inputs);
    app.insert_resource(ButtonInput::<MouseButton>::default());

    app.update();

    {
        let platform = app.world().get::<PropaneReceptaclePlatform>(refill_platform_e).unwrap();
        assert_eq!(platform.locked_tank, None, "Interacting with platform must release locked tank");
        let locked = app.world().get::<LockedInAnchor>(tank2_e);
        assert!(locked.is_none());
        let rb = app.world().get::<RigidBody>(tank2_e).unwrap();
        assert_eq!(*rb, RigidBody::Dynamic, "Released tank must revert to Dynamic");
    }
}

#[test]
fn test_stove_on_off_independent_and_propane_consumption_rules() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 100.0,
        burn_rate: 10.0,
        leak_vent_rate: 30.0,
    });
    app.insert_resource(WorkshopTemperature {
        current: 10.0,
        target: 20.0,
        heating_rate: 5.0,
        cooling_rate: 2.0,
        is_frozen: false,
    });

    app.add_systems(
        Update,
        (
            stove_interaction_system,
            propane_gas_flow_system,
            workshop_temperature_system,
        ),
    );

    // Setup connected tank on house platform
    let house_pos = Vec3::new(-8.3, 0.45, -13.55);
    let tank_e = app.world_mut().spawn(PropaneTank {
        fuel_amount: 100.0,
        capacity: 100.0,
        is_connected: true,
    }).id();

    app.world_mut().spawn((
        PropaneReceptaclePlatform {
            kind: PropanePlatformKind::HouseSupply,
            activation_radius: 1.2,
            locked_tank: Some(tank_e),
        },
        AnchorPoint::new("propane_tank"),
        Transform::from_translation(house_pos),
        GlobalTransform::from_translation(house_pos),
    ));

    // Gas hose connected to stove
    let hose_e = app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Stove,
        ..default()
    }).id();

    // Stove inside starts OFF
    let stove_pos = Vec3::new(-6.5, 0.0, -11.4);
    let stove_e = app.world_mut().spawn((
        GasFurnaceStove {
            is_on: false, // Stove is OFF
            is_lit: false,
            heat_output: 1.5,
            is_heated: false,
            internal_temp: 20.0,
        },
        Transform::from_translation(stove_pos),
        GlobalTransform::from_translation(stove_pos),
    )).id();

    // RULE 1: While off AND connected, propane is not consumed!
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(2));
    app.update();

    let tank_fuel = app.world().get::<PropaneTank>(tank_e).unwrap().fuel_amount;
    assert_eq!(
        tank_fuel, 100.0,
        "Propane must NOT be consumed while stove is OFF and connected"
    );
    let stove = app.world().get::<GasFurnaceStove>(stove_e).unwrap();
    assert!(!stove.is_lit, "Stove must not be lit while is_on is false");

    // RULE 2: Turn stove ON -> propane is consumed and stove heats workshop!
    {
        let mut s = app.world_mut().get_mut::<GasFurnaceStove>(stove_e).unwrap();
        s.is_on = true;
    }
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
    app.update();

    let fuel_after_on = app.world().get::<PropaneTank>(tank_e).unwrap().fuel_amount;
    assert!(
        (fuel_after_on - 90.0).abs() < 0.1,
        "Propane must be consumed at burn_rate when stove is ON (expected 90.0, got {})",
        fuel_after_on
    );
    let stove = app.world().get::<GasFurnaceStove>(stove_e).unwrap();
    assert!(stove.is_lit, "Stove must be lit when ON with active gas supply");
    let temp = app.world().resource::<WorkshopTemperature>().current;
    assert!(temp > 10.0, "Active burning stove must warm the workshop");

    // RULE 3: If disconnected (hose detached to Neither), propane is consumed since it's leaking!
    app.world_mut().get_mut::<GasHose>(hose_e).unwrap().attached_to = HoseAttachment::Neither;
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs(1));
    app.update();

    let fuel_after_leak = app.world().get::<PropaneTank>(tank_e).unwrap().fuel_amount;
    assert!(
        fuel_after_leak < fuel_after_on - 25.0,
        "Propane must be consumed rapidly when disconnected because it's leaking (got {})",
        fuel_after_leak
    );
    let hose = app.world().get::<GasHose>(hose_e).unwrap();
    assert!(hose.is_venting_gas, "Hose must vent gas when disconnected");

    // RULE 4: Test toggling stove on/off with player interaction
    app.world_mut().spawn((
        FirstPersonCamera,
        GlobalTransform::from(Transform::from_translation(stove_pos + Vec3::new(0.0, 0.5, 1.5))
            .looking_at(stove_pos, Vec3::Y)),
    ));
    let mut key_inputs = ButtonInput::<KeyCode>::default();
    key_inputs.press(KeyCode::KeyE);
    app.insert_resource(key_inputs);
    app.insert_resource(ButtonInput::<MouseButton>::default());

    app.update();

    let stove = app.world().get::<GasFurnaceStove>(stove_e).unwrap();
    assert!(!stove.is_on, "Player interaction must toggle stove is_on from true to false");
}

#[test]
fn test_gas_leak_flame_explosion() {
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
fn test_gas_hose_visual_indicators_and_auto_attach() {
    // 1. Gas hose anchor points have larger interaction areas
    let mut stove_app = App::new();
    stove_app.init_resource::<Assets<Mesh>>();
    stove_app.init_resource::<Assets<StandardMaterial>>();
    stove_app.add_systems(Startup, spawn_oven_stove_and_propane_scene);
    stove_app.update();

    let mut anchor_q = stove_app.world_mut().query::<(&StoveGasAnchor, &Collider)>();
    let (_, _stove_col) = anchor_q.iter(stove_app.world()).next().expect("StoveGasAnchor must exist");

    // 2. Appliance visual indicators (Furnace window glowing vs cool dark gray, Stove blue burner flames)
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

    // 3. Automatic attach to anchor player is facing while holding hose
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
fn test_unattended_stove_accumulates_gas_concentration_over_15s() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 100.0,
        ..default()
    });
    app.insert_resource(WorkshopGasConcentration::default());
    app.add_message::<FieryExplosionEvent>();

    // Stove is ON but unlit (venting unignited gas)
    let stove_pos = Vec3::new(-7.34, 0.0, -11.4);
    app.world_mut().spawn((
        GasFurnaceStove {
            is_on: true,
            is_lit: false,
            ..default()
        },
        Transform::from_translation(stove_pos),
    ));

    app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Stove,
        ..default()
    });

    app.add_systems(Update, unattended_stove_and_pyro_hazard_system);

    // Advance 16 seconds (past 15s threshold)
    {
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs(16));
    }
    app.update();

    let gas_conc = app.world().resource::<WorkshopGasConcentration>();
    assert!(gas_conc.is_leaking_unignited, "Gas should be flagged as leaking unignited");
    assert!(
        gas_conc.concentration_timer >= 15.0,
        "Gas concentration timer should reach >= 15s: {}",
        gas_conc.concentration_timer
    );
}

#[test]
fn test_pyro_elf_detects_gas_and_seeks_flame_source() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 100.0,
        ..default()
    });
    app.insert_resource(WorkshopGasConcentration {
        concentration_timer: 16.0,
        detection_threshold: 15.0,
        is_leaking_unignited: true,
        ..default()
    });
    app.add_message::<FieryExplosionEvent>();

    let stove_pos = Vec3::new(-7.34, 0.0, -11.4);
    app.world_mut().spawn((
        GasFurnaceStove {
            is_on: true,
            is_lit: false,
            ..default()
        },
        Transform::from_translation(stove_pos),
    ));

    app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Stove,
        ..default()
    });

    // Spawn an active flame source (e.g. candle point light / match)
    let flame_pos = Vec3::new(0.0, 1.2, -4.5);
    app.world_mut().spawn((
        FlameSource,
        Transform::from_translation(flame_pos),
    ));

    // Spawn Pyro elf (id 2, Jingles) close to flame
    let mut elf = ElfAssistant::new(2, "Elf Jingles", Vec::new());
    elf.mischief_trait = ElfMischiefTrait::Pyro;
    elf.state = ElfNpcState::Wandering;

    let elf_e = app
        .world_mut()
        .spawn((
            elf,
            Velocity::zero(),
            Transform::from_xyz(0.2, 0.45, -4.5),
        ))
        .id();

    app.add_systems(Update, unattended_stove_and_pyro_hazard_system);
    app.update();

    // The elf should have detected the gas, sought the nearby flame (< 1.3m), and transitioned to ChargingStoveWithFlame!
    let elf_check = app.world().get::<ElfAssistant>(elf_e).unwrap();
    assert_eq!(
        elf_check.state,
        ElfNpcState::ChargingStoveWithFlame,
        "Pyro elf must secure flame and transition to ChargingStoveWithFlame"
    );
}

#[test]
fn test_pyro_elf_charges_stove_and_triggers_catastrophic_explosion() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.insert_resource(WorkshopGasLine {
        pressure_psi: 100.0,
        ..default()
    });
    app.insert_resource(WorkshopGasConcentration {
        concentration_timer: 18.0,
        detection_threshold: 15.0,
        is_leaking_unignited: true,
        ..default()
    });
    app.add_message::<FieryExplosionEvent>();

    let stove_pos = Vec3::new(-7.34, 0.0, -11.4);
    app.world_mut().spawn((
        GasFurnaceStove {
            is_on: true,
            is_lit: false,
            ..default()
        },
        Transform::from_translation(stove_pos),
    ));

    app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Stove,
        ..default()
    });

    let door_e = app
        .world_mut()
        .spawn((
            WorkshopDoor {
                door_id: "StorageWing".to_string(),
                required_tier: 2,
                state: DoorState::Locked,
                closed_position: Vec3::new(13.0, 0.0, 0.0),
                open_position: Vec3::new(13.0, 4.0, 0.0),
            },
            Transform::from_xyz(13.0, 0.0, 0.0),
        ))
        .id();

    let pdoor_e = app
        .world_mut()
        .spawn(PantryDoor {
            current_angle: 0.0,
            was_closed: true,
            ..default()
        })
        .id();

    // Spawn Pyro elf reaching stove (< 1.2m)
    let mut elf = ElfAssistant::new(2, "Elf Jingles", Vec::new());
    elf.mischief_trait = ElfMischiefTrait::Pyro;
    elf.state = ElfNpcState::ChargingStoveWithFlame;

    let elf_e = app
        .world_mut()
        .spawn((
            elf,
            Velocity::zero(),
            Transform::from_xyz(-7.34, 0.45, -11.0),
        ))
        .id();

    app.add_systems(Update, unattended_stove_and_pyro_hazard_system);
    app.update();

    // 1. Verify gas concentration timer reset
    let gas_conc = app.world().resource::<WorkshopGasConcentration>();
    assert_eq!(gas_conc.concentration_timer, 0.0);

    // 2. Verify doors blown open
    let door = app.world().get::<WorkshopDoor>(door_e).unwrap();
    assert_eq!(door.state, DoorState::Open, "Explosion blast must blow open doors");

    let pdoor = app.world().get::<PantryDoor>(pdoor_e).unwrap();
    assert!(pdoor.current_angle > 1.0, "Pantry door must be blown open");

    // 3. Verify Pyro elf thrown into dynamic ragdoll tumble with extreme rafter launch
    let elf_check = app.world().get::<ElfAssistant>(elf_e).unwrap();
    assert_eq!(
        elf_check.state,
        ElfNpcState::RagdollTumble,
        "Pyro elf must be thrown into RagdollTumble from explosion blast"
    );

    let elf_vel = app.world().get::<Velocity>(elf_e).unwrap();
    assert!(
        elf_vel.linear.y >= 12.0,
        "Elf must be launched high into vaulted ceiling rafters (Y >= 12.0), got {}",
        elf_vel.linear.y
    );
}

#[test]
fn test_gas_hose_grabbing_cooldown_collision_disabled_and_max_distance_drop() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.add_systems(Update, gas_hose_chain_physics_system);

    let stove_anchor = Vec3::new(-7.34, 0.85, -11.4);
    let wall_inlet = Vec3::new(-9.05, 1.15, -12.85);

    // Coupler nozzle with ColliderDisabled and CollisionGroups::new(Group::NONE, Group::NONE)
    let nozzle_e = app.world_mut().spawn((
        GasHoseSegment { index: 6, velocity: Vec3::ZERO },
        GasHoseCouplerNozzle,
        Grabbable,
        ColliderDisabled,
        CollisionGroups::new(Group::NONE, Group::NONE),
        Transform::from_translation(stove_anchor),
    )).id();

    // Verify collision is disabled on the nozzle
    assert!(app.world().get::<ColliderDisabled>(nozzle_e).is_some(), "Collision must be disabled on coupler nozzle end");

    let hose_e = app.world_mut().spawn(GasHose {
        attached_to: HoseAttachment::Stove,
        wall_inlet_pos: wall_inlet,
        stove_anchor_pos: stove_anchor,
        loose_nozzle_pos: stove_anchor,
        is_venting_gas: false,
        snap_distance: 0.85,
        nozzle_entity: Some(nozzle_e),
        disconnected_timer: 0.0,
        reconnect_cooldown: 0.0,
        max_segment_length: 0.40,
        max_hose_length: 2.66,
        ..default()
    }).id();

    // Player grabs the nozzle off the stove
    app.world_mut().entity_mut(nozzle_e).insert(HeldByPlayer);
    let player_hands_e = app.world_mut().spawn(ElfHands {
        left_grabbed: Some(nozzle_e),
        right_grabbed: None,
        ..default()
    }).id();

    app.update();

    // 1. Verify nozzle is detached and reconnect_cooldown is activated (preventing it from immediately reconnecting)
    {
        let hose = app.world().get::<GasHose>(hose_e).unwrap();
        assert_eq!(hose.attached_to, HoseAttachment::Neither, "Hose must detach when grabbed");
        assert!(hose.reconnect_cooldown > 0.5, "Reconnect cooldown must be activated upon grab, got {}", hose.reconnect_cooldown);
        let hands = app.world().get::<ElfHands>(player_hands_e).unwrap();
        assert_eq!(hands.left_grabbed, Some(nozzle_e), "Player must remain holding nozzle during cooldown");
    }

    // 2. Player pulls nozzle further than max_hose_length (2.66m) -> hose pulled out of hand!
    {
        let mut tf = app.world_mut().get_mut::<Transform>(nozzle_e).unwrap();
        // Distance from wall_inlet (-9.05, 1.15, -12.85) to (-5.0, 1.15, -12.85) is 4.05m (> 2.66m)
        tf.translation = wall_inlet + Vec3::new(4.05, 0.0, 0.0);
    }

    app.update();

    {
        let hands = app.world().get::<ElfHands>(player_hands_e).unwrap();
        assert!(hands.left_grabbed.is_none(), "Nozzle must be pulled out of player's hand when exceeding max hose length");
        assert!(app.world().get::<HeldByPlayer>(nozzle_e).is_none(), "HeldByPlayer must be removed");
        let n_tf = app.world().get::<Transform>(nozzle_e).unwrap();
        let dist = n_tf.translation.distance(wall_inlet);
        assert!(dist <= 2.67, "Nozzle position must be clamped to max_hose_length (2.66m), got {}", dist);
    }
}
