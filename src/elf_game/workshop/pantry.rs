use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    moose::{MooseSnack, toy_model::MooseSnackJar},
    player::hands::Grabbable,
};

/// Marker component for the Pantry Storage Wing root entity.
#[derive(Component, Debug, Clone)]
pub struct PantryWing {
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

impl PantryWing {
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.bounds_min.x && point.x <= self.bounds_max.x
            && point.y >= self.bounds_min.y && point.y <= self.bounds_max.y
            && point.z >= self.bounds_min.z && point.z <= self.bounds_max.z
    }
}

/// Marker component for supplies spawned inside or belonging to the Pantry.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PantrySupply;

/// Marker component for perimeter shelving units in the pantry.
#[derive(Component, Debug, Clone)]
pub struct PantryShelvingUnit {
    pub unit_name: String,
    pub tier_count: usize,
}

/// Tracks the shelf slot positions for Moose Treat jars in the pantry
/// and oversees the infinite restock cycle.
#[derive(Resource, Debug, Clone)]
pub struct PantryRestockTracker {
    /// 3D coordinates on the shelves where treat jars belong.
    pub slots: Vec<Vec3>,
    /// Number of completed restock cycles triggered by closing and reopening the door.
    pub restock_cycle_count: u32,
    /// Total number of fresh treat jars restocked across all cycles.
    pub total_restocked_jars: u32,
    /// Proximity radius to consider a slot occupied by a jar.
    pub slot_radius: f32,
}

impl Default for PantryRestockTracker {
    fn default() -> Self {
        Self {
            slots: vec![
                Vec3::new(-15.0, 0.98, 3.80),
            ],
            restock_cycle_count: 0,
            total_restocked_jars: 0,
            slot_radius: 0.45,
        }
    }
}

/// Interactive hinged physics door guarding the Pantry Storage Wing.
/// Mounted via a simple fixed-point hinge rotating around the hinge post axis,
/// opening into the workshop room and slowly closing itself if less than 1/4 rad from closed
/// to rest flush with the wall.
#[derive(Component, Debug, Clone)]
pub struct PantryDoor {
    pub hinge_anchor_pos: Vec3,
    pub current_angle: f32,
    pub target_angle: f32,
    pub was_closed: bool,
    pub closed_threshold: f32, // 0.05 rad
    pub open_threshold: f32,   // 0.35 rad
    pub close_snap_threshold: f32, // 0.25 rad (1/4 rad)
    pub spring_stiffness: f32,
    pub spring_damping: f32,
    pub close_speed: f32,
    pub is_grabbed: bool,
    pub grab_reach: f32,
    pub grab_release_distance: f32,
    pub grab_offset_angle: f32,
    pub grabbed_via_mouse: bool,
}

impl Default for PantryDoor {
    fn default() -> Self {
        Self {
            hinge_anchor_pos: Vec3::new(-13.0, 1.35, 4.75),
            current_angle: 0.0,
            target_angle: 0.0,
            was_closed: true,
            closed_threshold: 0.05,
            open_threshold: 0.35,
            close_snap_threshold: 0.25, // 1/4 rad
            spring_stiffness: 15.0,
            spring_damping: 4.0,
            close_speed: 0.75,
            is_grabbed: false,
            grab_reach: 2.5,
            grab_release_distance: 2.8,
            grab_offset_angle: 0.0,
            grabbed_via_mouse: false,
        }
    }
}

/// Fixed door frame anchor post component for the pantry door revolute hinge.
#[derive(Component, Debug, Clone)]
pub struct PantryDoorHingePost;

/// A portable Moose Treat jar with treats that can be carried or used to feed Mr. Moose.
#[derive(Component, Debug, Clone)]
pub struct MooseTreatJar {
    pub treat_count: u32,
    pub max_treats: u32,
}

impl Default for MooseTreatJar {
    fn default() -> Self {
        Self {
            treat_count: 4,
            max_treats: 4,
        }
    }
}

/// Message fired whenever the pantry restocks depleted jar slots.
#[derive(Message, Debug, Clone)]
pub struct PantryRestockEvent {
    pub slots_restocked: usize,
    pub total_restocked_all_time: u32,
    pub cycle_index: u32,
}

/// Request message to interact with the pantry door.
#[derive(Message, Debug, Clone)]
pub struct PantryDoorInteractMessage {
    pub open: bool,
}

/// Spawns a fresh Moose Treat jar at the given slot position.
pub fn spawn_moose_treat_jar(
    commands: &mut Commands,
    meshes: &mut Option<ResMut<Assets<Mesh>>>,
    materials: &mut Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<&WorkshopMaterials>,
    pos: Vec3,
) -> Entity {
    let jar_glass_mat = workshop_mats
        .map(|wm| wm.snack_jar_glass.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: SNACK_JAR_GLASS,
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.15,
                    ..default()
                })
            })
        });

    let jar_lid_mat = workshop_mats
        .map(|wm| wm.polished_brass.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: POLISHED_BRASS,
                    metallic: 0.8,
                    perceptual_roughness: 0.3,
                    ..default()
                })
            })
        });

    let snack_cookie_mat = workshop_mats
        .map(|wm| wm.moose_snack_oat.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: MOOSE_SNACK_GOLDEN_OAT,
                    perceptual_roughness: 0.6,
                    ..default()
                })
            })
        });

    let mut jar_cmd = commands.spawn((
        Transform::from_translation(pos),
        Collider::cylinder(0.18, 0.20),
        RigidBody::Dynamic,
        Velocity::zero(),
        ExternalImpulse::default(),
        Damping {
            linear_damping: 1.0,
            angular_damping: 2.0,
        },
        Grabbable,
        MooseTreatJar::default(),
        MooseSnackJar,
        PantrySupply,
        Name::new("PantryMooseTreatJar"),
    ));

    if let (Some(m), Some(mat)) = (meshes.as_mut(), jar_glass_mat) {
        jar_cmd.insert((
            Mesh3d(m.add(Cylinder::new(0.20, 0.36))),
            MeshMaterial3d(mat),
        ));
    }

    let jar_entity = jar_cmd.id();

    // Spawn brass screw lid child
    if let (Some(m), Some(mat)) = (meshes.as_mut(), jar_lid_mat) {
        let lid_mesh = m.add(Cylinder::new(0.21, 0.06));
        commands.entity(jar_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(lid_mesh),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.0, 0.20, 0.0),
                Name::new("JarBrassLid"),
            ));
        });
    }

    // Spawn 4 small physics snack cookies resting around/with the jar
    for (ox, oz) in [(-0.10, -0.06), (0.08, -0.04), (-0.04, 0.08), (0.07, 0.07)] {
        let mut cookie_cmd = commands.spawn((
            Transform::from_xyz(pos.x + ox, pos.y + 0.12, pos.z + oz),
            Collider::cylinder(0.025, 0.07),
            RigidBody::Dynamic,
            Velocity::zero(),
            Damping {
                linear_damping: 0.8,
                angular_damping: 1.5,
            },
            Grabbable,
            MooseSnack {
                nutrition_value: 60.0,
            },
            PantrySupply,
            Name::new("PantrySmallMooseSnack"),
        ));
        if let (Some(m), Some(mat)) = (meshes.as_mut(), snack_cookie_mat.clone()) {
            cookie_cmd.insert((
                Mesh3d(m.add(Cylinder::new(0.07, 0.05))),
                MeshMaterial3d(mat),
            ));
        }
    }

    jar_entity
}

/// Spawns the Pantry Storage Wing architecture (4.0m x 4.0m x 3.5m) situated off the
/// main workshop corridor (West wall at X = -13.0, Z = 5.5), perimeter timber shelving units,
/// the interactive physics door on Rapier RevoluteJoint, and initial Moose Treat jars.
pub fn spawn_pantry_storage_wing(
    commands: &mut Commands,
    meshes: &mut Option<ResMut<Assets<Mesh>>>,
    materials: &mut Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<&WorkshopMaterials>,
) {
    let floor_mat = workshop_mats
        .map(|wm| wm.rustic_planks.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: RUSTIC_PLANKS,
                    perceptual_roughness: 0.85,
                    ..default()
                })
            })
        });

    let wall_mat = workshop_mats
        .map(|wm| wm.cabin_log.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: CABIN_LOG,
                    perceptual_roughness: 0.85,
                    ..default()
                })
            })
        });

    let shelf_mat = workshop_mats
        .map(|wm| wm.timber_beam.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: TIMBER_BEAM,
                    perceptual_roughness: 0.80,
                    ..default()
                })
            })
        });

    let door_wood_mat = workshop_mats
        .map(|wm| wm.rustic_planks.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: RUSTIC_PLANKS,
                    perceptual_roughness: 0.75,
                    ..default()
                })
            })
        });

    let iron_mat = workshop_mats
        .map(|wm| wm.wrought_iron.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: WROUGHT_IRON,
                    metallic: 0.75,
                    perceptual_roughness: 0.5,
                    ..default()
                })
            })
        });

    let brass_mat = workshop_mats
        .map(|wm| wm.polished_brass.clone())
        .or_else(|| {
            materials.as_mut().map(|m| {
                m.add(StandardMaterial {
                    base_color: POLISHED_BRASS,
                    metallic: 0.85,
                    perceptual_roughness: 0.3,
                    ..default()
                })
            })
        });

    // =========================================================================
    // 1. Pantry Room Shell: Floor & Ceiling (4.0m x 4.0m x 3.5m)
    // =========================================================================
    // Root marker
    commands.spawn((
        PantryWing {
            bounds_min: Vec3::new(-17.0, 0.0, 3.5),
            bounds_max: Vec3::new(-13.0, 3.5, 7.5),
        },
        Transform::from_xyz(-15.0, 1.75, 5.5),
        Name::new("PantryStorageWing"),
    ));

    // Floor (at Y = 0.0)
    let mut floor_cmd = commands.spawn((
        Transform::from_xyz(-15.0, 0.0, 5.5),
        Collider::cuboid(2.0, 0.1, 2.0),
        RigidBody::Fixed,
        Name::new("PantryFloor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_mat.clone()) {
        floor_cmd.insert((
            Mesh3d(m.add(Plane3d::default().mesh().size(4.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Ceiling (at Y = 3.5)
    let mut ceiling_cmd = commands.spawn((
        Transform::from_xyz(-15.0, 3.5, 5.5),
        Collider::cuboid(2.0, 0.1, 2.0),
        RigidBody::Fixed,
        Name::new("PantryCeiling"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_mat.clone()) {
        ceiling_cmd.insert((
            Mesh3d(m.add(Cuboid::new(4.0, 0.2, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Warm Ambient Lantern (PointLight at Y = 3.1)
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.78, 0.45),
            intensity: 7500.0,
            range: 7.5,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-15.0, 3.1, 5.5),
        Name::new("PantryCeilingLantern"),
    ));

    // =========================================================================
    // 2. Pantry Perimeter Walls
    // =========================================================================
    // Outer North Wall (Z = 3.5, X in [-17.0, -13.0])
    let mut north_wall = commands.spawn((
        Transform::from_xyz(-15.0, 1.75, 3.5),
        Collider::cuboid(2.0, 1.75, 0.15),
        RigidBody::Fixed,
        Name::new("PantryNorthWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        north_wall.insert((
            Mesh3d(m.add(Cuboid::new(4.0, 3.5, 0.3))),
            MeshMaterial3d(mat),
        ));
    }

    // Outer South Wall (Z = 7.5, X in [-17.0, -13.0])
    let mut south_wall = commands.spawn((
        Transform::from_xyz(-15.0, 1.75, 7.5),
        Collider::cuboid(2.0, 1.75, 0.15),
        RigidBody::Fixed,
        Name::new("PantrySouthWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        south_wall.insert((
            Mesh3d(m.add(Cuboid::new(4.0, 3.5, 0.3))),
            MeshMaterial3d(mat),
        ));
    }

    // Outer West Wall (X = -17.0, Z in [3.5, 7.5])
    let mut west_wall = commands.spawn((
        Transform::from_xyz(-17.0, 1.75, 5.5),
        Collider::cuboid(0.15, 1.75, 2.0),
        RigidBody::Fixed,
        Name::new("PantryWestWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        west_wall.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.5, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // East Dividing Wall - North of doorway (X = -13.0, Z in [3.5, 4.75])
    let mut east_wall_north = commands.spawn((
        Transform::from_xyz(-13.0, 1.75, 4.125),
        Collider::cuboid(0.15, 1.75, 0.625),
        RigidBody::Fixed,
        Name::new("PantryEastWallNorth"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        east_wall_north.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.5, 1.25))),
            MeshMaterial3d(mat),
        ));
    }

    // East Dividing Wall - South of doorway (X = -13.0, Z in [6.23, 7.5])
    let mut east_wall_south = commands.spawn((
        Transform::from_xyz(-13.0, 1.75, 6.865),
        Collider::cuboid(0.15, 1.75, 0.635),
        RigidBody::Fixed,
        Name::new("PantryEastWallSouth"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        east_wall_south.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.5, 1.27))),
            MeshMaterial3d(mat),
        ));
    }

    // Doorway Frame on West Wall of main workshop (X = -13.0, doorway opening Z in [4.7, 6.3])
    // Door Header Lintel (Y in [2.7, 3.5] above opening)
    let mut header_cmd = commands.spawn((
        Transform::from_xyz(-13.0, 3.1, 5.5),
        Collider::cuboid(0.2, 0.4, 0.8),
        RigidBody::Fixed,
        Name::new("PantryDoorHeaderLintel"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        header_cmd.insert((
            Mesh3d(m.add(Cuboid::new(0.4, 0.8, 1.6))),
            MeshMaterial3d(mat),
        ));
    }

    // Diegetic Directional Sign Plaque above doorway facing workshop aisle
    let mut sign_cmd = commands.spawn((
        Transform::from_xyz(-12.75, 3.0, 5.5),
        Collider::cuboid(0.04, 0.25, 0.65),
        RigidBody::Fixed,
        Name::new("PantrySignPlaque"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), brass_mat.clone()) {
        sign_cmd.insert((
            Mesh3d(m.add(Cuboid::new(0.08, 0.5, 1.3))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 3. Deep Perimeter Shelving Units
    // =========================================================================
    // North Shelving Unit (Z = 3.80, 3 tiers for treat jars)
    commands.spawn((
        PantryShelvingUnit {
            unit_name: "NorthMooseTreatShelves".to_string(),
            tier_count: 3,
        },
        Transform::from_xyz(-15.0, 1.6, 3.80),
        Name::new("NorthMooseTreatShelvingUnit"),
    ));

    for (tier_y, tier_name) in [
        (0.85, "NorthShelfLower"),
        (1.60, "NorthShelfMid"),
        (2.35, "NorthShelfUpper"),
    ] {
        let mut shelf_cmd = commands.spawn((
            Transform::from_xyz(-15.0, tier_y, 3.80),
            Collider::cuboid(1.6, 0.03, 0.275),
            RigidBody::Fixed,
            Name::new(tier_name),
        ));
        if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
            shelf_cmd.insert((
                Mesh3d(m.add(Cuboid::new(3.2, 0.06, 0.55))),
                MeshMaterial3d(mat),
            ));
        }
    }

    // West Perimeter Shelves (X = -16.65, Z in [4.2, 6.8]) for crates and general supplies
    commands.spawn((
        PantryShelvingUnit {
            unit_name: "WestSupplyShelves".to_string(),
            tier_count: 2,
        },
        Transform::from_xyz(-16.65, 1.25, 5.5),
        Name::new("WestSupplyShelvingUnit"),
    ));

    for (tier_y, tier_name) in [(0.85, "WestShelfLower"), (1.60, "WestShelfUpper")] {
        let mut shelf_cmd = commands.spawn((
            Transform::from_xyz(-16.65, tier_y, 5.5),
            Collider::cuboid(0.275, 0.03, 1.3),
            RigidBody::Fixed,
            Name::new(tier_name),
        ));
        if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
            shelf_cmd.insert((
                Mesh3d(m.add(Cuboid::new(0.55, 0.06, 2.6))),
                MeshMaterial3d(mat),
            ));
        }
    }

    // South Cookie Baking Staging Counter (Z = 7.15, X in [-16.5, -13.5])
    let mut counter_cmd = commands.spawn((
        Transform::from_xyz(-15.0, 0.45, 7.15),
        Collider::cuboid(1.5, 0.45, 0.3),
        RigidBody::Fixed,
        Name::new("PantryBakingStagingCounter"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        counter_cmd.insert((
            Mesh3d(m.add(Cuboid::new(3.0, 0.90, 0.60))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 4. Interactive Physics Door (RevoluteJoint & Spring Momentum)
    // =========================================================================
    // Fixed Hinge Post at North edge of doorway (X = -13.0, Z = 4.75, Y = 1.35)
    let hinge_post = commands
        .spawn((
            Transform::from_xyz(-13.0, 1.35, 4.75),
            Collider::cuboid(0.04, 1.35, 0.04),
            RigidBody::Fixed,
            PantryDoorHingePost,
            Name::new("PantryDoorHingePost"),
        ))
        .id();

    // Door Leaf entity: dynamic rigidbody centered at Z = 5.48 (between 4.75 and 6.21)
    // Width = 1.46m, height = 2.55m, thickness = 0.08m
    let joint = RevoluteJointBuilder::new(Vec3::Y)
        .local_anchor1(Vec3::ZERO)
        .local_anchor2(Vec3::new(0.0, 0.0, -0.73))
        .limits([0.0, 1.65])
        .contacts_enabled(false);

    let mut door_cmd = commands.spawn((
        Transform::from_xyz(-13.0, 1.35, 5.48),
        Collider::cuboid(0.04, 1.275, 0.73),
        RigidBody::Dynamic,
        Velocity::zero(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z | LockedAxes::TRANSLATION_LOCKED_Y,
        Damping {
            linear_damping: 2.0,
            angular_damping: 3.5,
        },
        ImpulseJoint::new(hinge_post, joint),
        PantryDoor::default(),
        Name::new("PantryPhysicsDoor"),
    ));

    if let (Some(m), Some(mat)) = (meshes.as_mut(), door_wood_mat.clone()) {
        door_cmd.insert((
            Mesh3d(m.add(Cuboid::new(0.08, 2.55, 1.46))),
            MeshMaterial3d(mat),
        ));
    }

    // Add door latch handle
    let door_entity = door_cmd.id();
    if let (Some(m), Some(mat)) = (meshes.as_mut(), iron_mat.clone()) {
        let handle_mesh = m.add(Cuboid::new(0.18, 0.22, 0.06));
        commands.entity(door_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(handle_mesh),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.0, 0.0, 0.55),
                Name::new("PantryDoorHandle"),
            ));
        });
    }

    // =========================================================================
    // 5. Initial Moose Treat Jars & Restock Tracker Resource
    // =========================================================================
    let tracker = PantryRestockTracker::default();
    for slot_pos in &tracker.slots {
        spawn_moose_treat_jar(commands, meshes, materials, workshop_mats, *slot_pos);
    }
    commands.insert_resource(tracker);
}

/// Maintains the fixed-point hinge alignment, applies swing dynamics toward target angle,
/// and slowly closes the door if less than 1/4 rad from closed to rest flush with the wall.
pub fn pantry_door_spring_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_door: Query<(
        &mut Transform,
        &mut Velocity,
        &mut ExternalImpulse,
        &mut PantryDoor,
    )>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    for (mut tf, mut vel, mut ext_impulse, mut door) in &mut q_door {
        let (_, mut yaw, _) = tf.rotation.to_euler(EulerRot::XYZ);

        // If door is pushed/displaced into negative angles (penetrating wall), push back with positive torque
        if yaw < 0.0 {
            let delta = 0.0 - yaw;
            ext_impulse.torque_impulse.y += door.spring_stiffness * delta * dt;
            yaw = 0.0;
            vel.angular.y = vel.angular.y.max(0.0);
        } else if yaw > 1.65 {
            yaw = 1.65;
            vel.angular.y = vel.angular.y.min(0.0);
        }

        if door.is_grabbed {
            // Player or elf is holding and swinging the door!
            let diff = door.target_angle - yaw;
            let follow_speed = 12.0;
            let max_step = follow_speed * dt;
            let step = diff.clamp(-max_step, max_step);
            yaw += step;
            vel.angular.y = step / dt;
            vel.linear = Vec3::ZERO;
        } else {
            // If pushed by dynamic torque impulse from player or elf:
            if ext_impulse.torque_impulse.y.abs() > 0.01 {
                vel.angular.y += ext_impulse.torque_impulse.y * dt;
                let push_step = vel.angular.y * dt;
                yaw = (yaw + push_step).clamp(0.0, 1.65);
            }

            // Auto-close behavior:
            // "slowly close itself if less than 1/4 rad closed and flush with the wall"
            if yaw > 0.0 && yaw < door.close_snap_threshold {
                // Only auto-close if not actively being pushed open with strong positive torque
                if ext_impulse.torque_impulse.y <= 0.5 {
                    let close_step = door.close_speed * dt;
                    if yaw <= close_step || yaw <= 0.015 {
                        yaw = 0.0;
                        vel.angular.y = 0.0;
                        vel.linear = Vec3::ZERO;
                    } else {
                        yaw -= close_step;
                        vel.angular.y = -door.close_speed;
                    }
                }
            } else if yaw >= door.close_snap_threshold && door.target_angle > 0.0 {
                let delta = door.target_angle - yaw;
                let torque = door.spring_stiffness * delta - door.spring_damping * vel.angular.y;
                ext_impulse.torque_impulse.y += torque * dt;
            }
        }

        door.current_angle = yaw;

        // Strictly enforce simple hinge rotating around fixed pivot:
        let hinge = door.hinge_anchor_pos;
        let r = 0.73;
        tf.translation = Vec3::new(
            hinge.x + r * yaw.sin(),
            hinge.y,
            hinge.z + r * yaw.cos(),
        );
        tf.rotation = Quat::from_rotation_y(yaw);
    }
}

/// Tracks the door closure cycle (`Closed -> Opened` state transition).
/// Whenever the door transitions Closed -> Opened, if the Moose Treat jar was removed from the pantry,
/// all pantry supplies are reset to the default initialized state (exactly 1 jar at the designated shelf slot).
pub fn pantry_door_restock_cycle_system(
    mut commands: Commands,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    tracker: Option<ResMut<PantryRestockTracker>>,
    pantry_wing: Option<Query<&PantryWing>>,
    mut q_door: Query<(&Transform, &mut PantryDoor)>,
    q_jars: Query<&Transform, (Or<(With<MooseTreatJar>, With<MooseSnackJar>)>, Without<PantryDoor>)>,
    q_pantry_supplies: Query<(Entity, &Transform), (With<PantrySupply>, Without<PantryDoor>)>,
    mut restock_events: Option<MessageWriter<PantryRestockEvent>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Some(mut tracker) = tracker else {
        return;
    };
    let Ok((door_tf, mut door)) = q_door.single_mut() else {
        return;
    };

    let (_, yaw, _) = door_tf.rotation.to_euler(EulerRot::XYZ);
    door.current_angle = yaw;

    let abs_angle = yaw.abs();

    if abs_angle <= door.closed_threshold {
        door.was_closed = true;
    } else if door.was_closed && abs_angle >= door.open_threshold {
        // Transition: Closed -> Opened!
        door.was_closed = false;

        let pantry_bounds = pantry_wing
            .and_then(|q| q.iter().next().map(|w| (w.bounds_min, w.bounds_max)))
            .unwrap_or((Vec3::new(-17.0, 0.0, 3.5), Vec3::new(-13.0, 3.5, 7.5)));

        let is_in_pantry = |pos: Vec3| {
            pos.x >= pantry_bounds.0.x && pos.x <= pantry_bounds.1.x
                && pos.y >= pantry_bounds.0.y && pos.y <= pantry_bounds.1.y
                && pos.z >= pantry_bounds.0.z && pos.z <= pantry_bounds.1.z
        };

        // Check if there is already a Moose Treat jar inside the pantry
        let jar_in_pantry = q_jars.iter().any(|tf| is_in_pantry(tf.translation));

        // The Pantry should only have one jar of Moose Treats at a time.
        // Once removed, it should not refresh until the door is closed and then reopened,
        // which should reset all of the Pantry supplies to the default initialized state.
        if !jar_in_pantry {
            tracker.restock_cycle_count += 1;

            // Clear any lingering pantry supplies remaining inside the pantry
            for (supply_entity, supply_tf) in &q_pantry_supplies {
                if is_in_pantry(supply_tf.translation) {
                    commands.entity(supply_entity).despawn();
                }
            }

            // Reset pantry supplies to default initialized state: spawn 1 jar at primary slot
            let slot_pos = tracker.slots.first().copied().unwrap_or(Vec3::new(-15.0, 0.98, 3.80));
            spawn_moose_treat_jar(
                &mut commands,
                &mut meshes,
                &mut materials,
                workshop_mats.as_deref(),
                slot_pos,
            );
            tracker.total_restocked_jars += 1;

            if let Some(ref mut writer) = restock_events {
                writer.write(PantryRestockEvent {
                    slots_restocked: 1,
                    total_restocked_all_time: tracker.total_restocked_jars,
                    cycle_index: tracker.restock_cycle_count,
                });
            }
        }
    }
}

/// Calculates push torque exerted on the pantry door by a moving body at position `pos` with velocity `vel`.
pub fn calculate_pantry_door_push_torque(
    door_tf: &Transform,
    door: &PantryDoor,
    pos: Vec3,
    vel: Vec3,
) -> f32 {
    let (_, yaw, _) = door_tf.rotation.to_euler(EulerRot::XYZ);
    let hinge = door.hinge_anchor_pos;
    let door_dir = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let door_normal = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

    let delta = Vec3::new(pos.x - hinge.x, 0.0, pos.z - hinge.z);
    let span = delta.dot(door_dir);
    let perp = delta.dot(door_normal);

    // If body is along the length of the door (0.0 to 1.5m) and within contact distance (|perp| <= 0.65m)
    if span >= -0.2 && span <= 1.6 && perp.abs() <= 0.65 {
        let v_h = Vec3::new(vel.x, 0.0, vel.z);
        let speed = v_h.length();
        if speed > 0.05 {
            let v_normal = v_h.dot(door_normal);
            let leverage = span.clamp(0.2, 1.46);
            // If pushing from outside (+normal) with negative v_normal (towards door)
            // or from inside (-normal) with positive v_normal (towards door)
            if (perp >= 0.0 && v_normal < -0.05) || (perp <= 0.0 && v_normal > 0.05) {
                let push_dir = if perp >= 0.0 { -1.0 } else { 1.0 };
                return push_dir * leverage * speed * 4.0;
            }
        }
    }
    0.0
}

/// Allows the player to grab and swing the pantry door open and shut,
/// rather than applying a static force, and handles physical push impulses
/// when the player or elves run into the door.
/// If the player moves away while grabbing, the grab is automatically released.
pub fn pantry_door_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_player: Query<
        (&Transform, Option<&crate::elf_game::player::ElfMovement>),
        With<crate::elf_game::player::controller::ElfPlayer>,
    >,
    q_elves: Query<
        (&Transform, &Velocity),
        With<crate::elf_game::physics_sandbox::ragdoll::ElfAssistant>,
    >,
    q_camera: Query<&GlobalTransform, With<crate::elf_game::player::controller::FirstPersonCamera>>,
    mut q_door: Query<(&Transform, &mut ExternalImpulse, &mut PantryDoor)>,
    mut interact_messages: Option<MessageReader<PantryDoorInteractMessage>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Ok((door_tf, mut ext_impulse, mut door)) = q_door.single_mut() else {
        return;
    };

    // 1. Dynamic physical pushing: check if the player runs into the door
    for (p_tf, opt_mov) in &q_player {
        if let Some(mov) = opt_mov {
            let push_torque = calculate_pantry_door_push_torque(door_tf, &door, p_tf.translation, mov.velocity);
            if push_torque.abs() > 0.01 {
                ext_impulse.torque_impulse.y += push_torque;
                door.target_angle = (door.target_angle + push_torque * 0.1).clamp(0.0, 1.65);
            }
        }
    }

    // Dynamic physical pushing: check if elves run into the door
    for (elf_tf, elf_vel) in &q_elves {
        let push_torque = calculate_pantry_door_push_torque(door_tf, &door, elf_tf.translation, elf_vel.linear);
        if push_torque.abs() > 0.01 {
            ext_impulse.torque_impulse.y += push_torque;
            door.target_angle = (door.target_angle + push_torque * 0.1).clamp(0.0, 1.65);
        }
    }

    let mut trigger = false;
    let mut open_command = None;

    if let Some(ref mut reader) = interact_messages {
        for msg in reader.read() {
            trigger = true;
            open_command = Some(msg.open);
        }
    }

    if let Some(open) = open_command {
        door.is_grabbed = false;
        door.grabbed_via_mouse = false;
        if open {
            ext_impulse.torque_impulse.y = 8.0;
            door.target_angle = 1.40;
        } else {
            ext_impulse.torque_impulse.y = -8.0;
            door.target_angle = 0.0;
        }
        return;
    }

    if trigger {
        return;
    }

    let key_interact = keys
        .as_ref()
        .map(|k| {
            k.just_pressed(KeyCode::KeyE)
                || k.just_pressed(KeyCode::KeyQ)
                || k.just_pressed(KeyCode::KeyF)
        })
        .unwrap_or(false);

    let mouse_just_pressed = mouse
        .as_ref()
        .map(|m| m.just_pressed(MouseButton::Left))
        .unwrap_or(false);

    let mouse_just_released = mouse
        .as_ref()
        .map(|m| m.just_released(MouseButton::Left))
        .unwrap_or(false);

    let interact_pressed = key_interact || mouse_just_pressed;

    if let Some((player_tf, _)) = q_player.iter().next() {
        let player_pos = player_tf.translation;
        let dist = player_pos.distance(door_tf.translation);

        if door.is_grabbed {
            // Requirement: "When grabbing the door, if the player moves away then the grab should be released."
            if dist > door.grab_release_distance {
                door.is_grabbed = false;
                door.grabbed_via_mouse = false;
            } else if interact_pressed {
                // Toggling interact while holding releases the grab
                door.is_grabbed = false;
                door.grabbed_via_mouse = false;
            } else if door.grabbed_via_mouse && mouse_just_released {
                // Releasing mouse click releases the grab
                door.is_grabbed = false;
                door.grabbed_via_mouse = false;
            } else {
                // Player is actively holding and swinging the door!
                let mut pull_pos = player_pos;
                if let Some(cam_gt) = q_camera.iter().next() {
                    let fwd: Vec3 = cam_gt.forward().into();
                    let fwd_h = Vec3::new(fwd.x, 0.0, fwd.z);
                    if fwd_h.length_squared() > 1e-4 {
                        pull_pos += fwd_h.normalize() * 0.6;
                    }
                }

                let hinge = door.hinge_anchor_pos;
                let delta_x = pull_pos.x - hinge.x;
                let delta_z = pull_pos.z - hinge.z;
                let player_angle = delta_x.atan2(delta_z);

                let desired_angle = (player_angle + door.grab_offset_angle).clamp(0.0, 1.65);
                door.target_angle = desired_angle;
            }
        } else {
            // Not grabbed: player can grab if within reach
            if interact_pressed && dist <= door.grab_reach {
                door.is_grabbed = true;
                door.grabbed_via_mouse = mouse_just_pressed;

                let (_, yaw, _) = door_tf.rotation.to_euler(EulerRot::XYZ);
                let current_yaw = yaw.clamp(0.0, 1.65);

                let mut pull_pos = player_pos;
                if let Some(cam_gt) = q_camera.iter().next() {
                    let fwd: Vec3 = cam_gt.forward().into();
                    let fwd_h = Vec3::new(fwd.x, 0.0, fwd.z);
                    if fwd_h.length_squared() > 1e-4 {
                        pull_pos += fwd_h.normalize() * 0.6;
                    }
                }

                let hinge = door.hinge_anchor_pos;
                let delta_x = pull_pos.x - hinge.x;
                let delta_z = pull_pos.z - hinge.z;
                let player_angle = delta_x.atan2(delta_z);

                // Initialize offset so door angle does not jump upon grabbing
                door.grab_offset_angle = current_yaw - player_angle;
                door.target_angle = current_yaw;
            }
        }
    }
}
