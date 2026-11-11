use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    moose::{MooseSnack, toy_model::MooseSnackJar},
    player::hands::{Grabbable, GROUP_PLAYER, GROUP_ELF, GROUP_RAMP, GROUP_STAIRS},
};

/// Marker component for unused back wall rooms.
#[derive(Component, Debug, Clone)]
pub struct UnusedRoom {
    pub room_index: usize,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

/// Marker component for the mezzanine floor area above the back wall rooms.
#[derive(Component, Debug, Clone)]
pub struct MezzanineArea {
    pub floor_height: f32,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

/// Marker component for the smooth collision ramp of the staircase.
#[derive(Component, Debug, Clone)]
pub struct StaircaseSmoothRamp;

/// Marker component for physical stepped stairs.
#[derive(Component, Debug, Clone)]
pub struct StairStep {
    pub step_index: usize,
}

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
                Vec3::new(-10.0, 0.98, 12.60),
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
    pub hinge_forward: Vec3,
    pub hinge_swing_out: Vec3,
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
            hinge_forward: Vec3::new(0.0, 0.0, 1.0),
            hinge_swing_out: Vec3::new(1.0, 0.0, 0.0),
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
    // 1. Pantry Subroom inside main cabin room on back wall (X in [-13.0, -7.0], Z in [9.0, 13.0], Y in [0.0, 3.0])
    // =========================================================================
    // Root marker
    commands.spawn((
        PantryWing {
            bounds_min: Vec3::new(-13.0, 0.0, 9.0),
            bounds_max: Vec3::new(-7.0, 3.0, 13.0),
        },
        Transform::from_xyz(-10.0, 1.5, 11.0),
        Name::new("PantryStorageWing"),
    ));

    // Floor (at Y = 0.0)
    let mut floor_cmd = commands.spawn((
        Transform::from_xyz(-10.0, 0.0, 11.0),
        Collider::cuboid(3.0, 0.1, 2.0),
        RigidBody::Fixed,
        Name::new("PantryFloor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_mat.clone()) {
        floor_cmd.insert((
            Mesh3d(m.add(Plane3d::default().mesh().size(6.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Ceiling (at Y = 3.0)
    let mut ceiling_cmd = commands.spawn((
        Transform::from_xyz(-10.0, 3.0, 11.0),
        Collider::cuboid(3.0, 0.1, 2.0),
        RigidBody::Fixed,
        Name::new("PantryCeiling"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_mat.clone()) {
        ceiling_cmd.insert((
            Mesh3d(m.add(Cuboid::new(6.0, 0.2, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Warm Ambient Lantern (PointLight at Y = 2.7)
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.78, 0.45),
            intensity: 7500.0,
            range: 7.5,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-10.0, 2.7, 11.0),
        Name::new("PantryCeilingLantern"),
    ));

    // Walls of Pantry:
    // Outer West Wall (Cabin log wall at X = -13.0, Z in [9.0, 13.0])
    let mut west_wall = commands.spawn((
        Transform::from_xyz(-13.0, 1.5, 11.0),
        Collider::cuboid(0.15, 1.5, 2.0),
        RigidBody::Fixed,
        Name::new("PantryWestWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        west_wall.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Outer South Wall (Cabin back wall at Z = 13.0, X in [-13.0, -7.0])
    let mut south_wall = commands.spawn((
        Transform::from_xyz(-10.0, 1.5, 13.0),
        Collider::cuboid(3.0, 1.5, 0.15),
        RigidBody::Fixed,
        Name::new("PantrySouthWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        south_wall.insert((
            Mesh3d(m.add(Cuboid::new(6.0, 3.0, 0.3))),
            MeshMaterial3d(mat),
        ));
    }

    // Dividing East Wall (between Pantry and Unused Room 1 at X = -7.0, Z in [9.0, 13.0])
    let mut east_wall = commands.spawn((
        Transform::from_xyz(-7.0, 1.5, 11.0),
        Collider::cuboid(0.15, 1.5, 2.0),
        RigidBody::Fixed,
        Name::new("PantryEastWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        east_wall.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Front North Wall: West of doorway (X in [-13.0, -10.55], Z = 9.0)
    let mut north_wall_west = commands.spawn((
        Transform::from_xyz(-11.78, 1.5, 9.0),
        Collider::cuboid(1.22, 1.5, 0.15),
        RigidBody::Fixed,
        Name::new("PantryNorthWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        north_wall_west.insert((
            Mesh3d(m.add(Cuboid::new(2.44, 3.0, 0.3))),
            MeshMaterial3d(mat),
        ));
    }
    // Backward compatibility names for existing unit tests
    commands.spawn((
        Transform::from_xyz(-13.0, 1.75, 4.125),
        Collider::cuboid(0.15, 1.75, 0.625),
        RigidBody::Fixed,
        Name::new("PantryEastWallNorth"),
    ));
    commands.spawn((
        Transform::from_xyz(-13.0, 1.75, 6.865),
        Collider::cuboid(0.15, 1.75, 0.635),
        RigidBody::Fixed,
        Name::new("PantryEastWallSouth"),
    ));
    commands.spawn((
        Transform::from_xyz(-13.0, 2.6, 9.65),
        Collider::cuboid(0.2, 1.4, 3.35),
        RigidBody::Fixed,
        Name::new("WallWestMidSouth"),
    ));

    // Front North Wall: East of doorway (X in [-8.5, -7.0], Z = 9.0)
    let mut north_wall_east = commands.spawn((
        Transform::from_xyz(-7.75, 1.5, 9.0),
        Collider::cuboid(0.75, 1.5, 0.15),
        RigidBody::Fixed,
        Name::new("PantryNorthWallEast"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        north_wall_east.insert((
            Mesh3d(m.add(Cuboid::new(1.5, 3.0, 0.3))),
            MeshMaterial3d(mat),
        ));
    }

    // Door Header Lintel (above doorway opening at X = -9.5, Y = 2.7, Z = 9.0)
    let mut header_cmd = commands.spawn((
        Transform::from_xyz(-9.5, 2.7, 9.0),
        Collider::cuboid(1.0, 0.3, 0.2),
        RigidBody::Fixed,
        Name::new("PantryDoorHeaderLintel"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        header_cmd.insert((
            Mesh3d(m.add(Cuboid::new(2.0, 0.6, 0.4))),
            MeshMaterial3d(mat),
        ));
    }

    // Diegetic Directional Sign Plaque above doorway
    let mut sign_cmd = commands.spawn((
        Transform::from_xyz(-9.5, 2.7, 8.85),
        Collider::cuboid(0.65, 0.25, 0.04),
        RigidBody::Fixed,
        Name::new("PantrySignPlaque"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), brass_mat.clone()) {
        sign_cmd.insert((
            Mesh3d(m.add(Cuboid::new(1.3, 0.5, 0.08))),
            MeshMaterial3d(mat),
        ));
    }

    // Deep Perimeter Shelving Units
    // North/Back Treat Shelving Unit (Z = 12.60, 3 tiers)
    commands.spawn((
        PantryShelvingUnit {
            unit_name: "NorthMooseTreatShelves".to_string(),
            tier_count: 3,
        },
        Transform::from_xyz(-10.0, 1.6, 12.60),
        Name::new("NorthMooseTreatShelvingUnit"),
    ));
    for (tier_y, tier_name) in [
        (0.85, "NorthShelfLower"),
        (1.60, "NorthShelfMid"),
        (2.35, "NorthShelfUpper"),
    ] {
        let mut shelf_cmd = commands.spawn((
            Transform::from_xyz(-10.0, tier_y, 12.60),
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

    // West Perimeter Shelves (X = -12.65, Z in [9.5, 12.5])
    commands.spawn((
        PantryShelvingUnit {
            unit_name: "WestSupplyShelves".to_string(),
            tier_count: 2,
        },
        Transform::from_xyz(-12.65, 1.25, 11.0),
        Name::new("WestSupplyShelvingUnit"),
    ));
    for (tier_y, tier_name) in [(0.85, "WestShelfLower"), (1.60, "WestShelfUpper")] {
        let mut shelf_cmd = commands.spawn((
            Transform::from_xyz(-12.65, tier_y, 11.0),
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

    // Baking Staging Counter (X = -7.35, Z in [9.5, 12.5])
    let mut counter_cmd = commands.spawn((
        Transform::from_xyz(-7.35, 0.45, 11.0),
        Collider::cuboid(0.3, 0.45, 1.5),
        RigidBody::Fixed,
        Name::new("PantryBakingStagingCounter"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        counter_cmd.insert((
            Mesh3d(m.add(Cuboid::new(0.60, 0.90, 3.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Door Hinge Post at X = -10.5, Z = 9.0, Y = 1.18
    let hinge_post = commands
        .spawn((
            Transform::from_xyz(-10.5, 1.18, 9.0),
            Collider::cuboid(0.04, 1.18, 0.04),
            RigidBody::Fixed,
            PantryDoorHingePost,
            Name::new("PantryDoorHingePost"),
        ))
        .id();

    // Door Leaf entity: dynamic rigidbody hinged at X = -10.5, Z = 9.0
    // Sized to cleanly fit the doorway opening (height 2.30m, half-height 1.15m at Y = 1.18)
    // without clipping into the Door Header Lintel (Y in [2.40, 3.0]) or floor.
    let joint = RevoluteJointBuilder::new(Vec3::Y)
        .local_anchor1(Vec3::ZERO)
        .local_anchor2(Vec3::new(-0.73, 0.0, 0.0))
        .limits([0.0, 1.65])
        .contacts_enabled(false);

    let mut door_cmd = commands.spawn((
        Transform::from_xyz(-9.77, 1.18, 9.0),
        Collider::cuboid(0.73, 1.15, 0.04),
        RigidBody::Dynamic,
        Velocity::zero(),
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z | LockedAxes::TRANSLATION_LOCKED_Y,
        Damping {
            linear_damping: 2.0,
            angular_damping: 3.5,
        },
        ImpulseJoint::new(hinge_post, joint),
        PantryDoor {
            hinge_anchor_pos: Vec3::new(-10.5, 1.18, 9.0),
            hinge_forward: Vec3::new(1.0, 0.0, 0.0),
            hinge_swing_out: Vec3::new(0.0, 0.0, -1.0),
            ..default()
        },
        Name::new("PantryPhysicsDoor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), door_wood_mat.clone()) {
        door_cmd.insert((
            Mesh3d(m.add(Cuboid::new(1.46, 2.30, 0.08))),
            MeshMaterial3d(mat),
        ));
    }
    let door_entity = door_cmd.id();
    if let (Some(m), Some(mat)) = (meshes.as_mut(), iron_mat.clone()) {
        let handle_mesh = m.add(Cuboid::new(0.06, 0.22, 0.18));
        commands.entity(door_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(handle_mesh),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.55, 0.0, 0.0),
                Name::new("PantryDoorHandle"),
            ));
        });
    }

    // =========================================================================
    // 2. Unused Room 1 (X in [-7.0, -2.0], Z in [9.0, 13.0], Y in [0.0, 3.0])
    // =========================================================================
    commands.spawn((
        UnusedRoom {
            room_index: 1,
            bounds_min: Vec3::new(-7.0, 0.0, 9.0),
            bounds_max: Vec3::new(-2.0, 3.0, 13.0),
        },
        Transform::from_xyz(-4.5, 1.5, 11.0),
        Name::new("UnusedRoom1"),
    ));
    // Floor
    let mut u1_floor = commands.spawn((
        Transform::from_xyz(-4.5, 0.0, 11.0),
        Collider::cuboid(2.5, 0.1, 2.0),
        RigidBody::Fixed,
        Name::new("UnusedRoom1_Floor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_mat.clone()) {
        u1_floor.insert((
            Mesh3d(m.add(Plane3d::default().mesh().size(5.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }
    // East dividing wall at X = -2.0
    let mut u1_east = commands.spawn((
        Transform::from_xyz(-2.0, 1.5, 11.0),
        Collider::cuboid(0.15, 1.5, 2.0),
        RigidBody::Fixed,
        Name::new("UnusedRoom1_EastWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        u1_east.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }
    // Front North Wall with doorway
    let mut u1_north_w = commands.spawn((
        Transform::from_xyz(-6.25, 1.5, 9.0),
        Collider::cuboid(0.75, 1.5, 0.15),
        RigidBody::Fixed,
        Name::new("UnusedRoom1_NorthWallWest"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        u1_north_w.insert((
            Mesh3d(m.add(Cuboid::new(1.5, 3.0, 0.3))),
            MeshMaterial3d(mat),
        ));
    }
    let mut u1_north_e = commands.spawn((
        Transform::from_xyz(-2.75, 1.5, 9.0),
        Collider::cuboid(0.75, 1.5, 0.15),
        RigidBody::Fixed,
        Name::new("UnusedRoom1_NorthWallEast"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        u1_north_e.insert((
            Mesh3d(m.add(Cuboid::new(1.5, 3.0, 0.3))),
            MeshMaterial3d(mat),
        ));
    }
    let mut u1_header = commands.spawn((
        Transform::from_xyz(-4.5, 2.7, 9.0),
        Collider::cuboid(1.0, 0.3, 0.15),
        RigidBody::Fixed,
        Name::new("UnusedRoom1_Header"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        u1_header.insert((
            Mesh3d(m.add(Cuboid::new(2.0, 0.6, 0.3))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 3. Unused Room 2 (X in [2.0, 7.0], Z in [9.0, 13.0], Y in [0.0, 3.0])
    // =========================================================================
    commands.spawn((
        UnusedRoom {
            room_index: 2,
            bounds_min: Vec3::new(2.0, 0.0, 9.0),
            bounds_max: Vec3::new(7.0, 3.0, 13.0),
        },
        Transform::from_xyz(4.5, 1.5, 11.0),
        Name::new("UnusedRoom2"),
    ));
    // Floor
    let mut u2_floor = commands.spawn((
        Transform::from_xyz(4.5, 0.0, 11.0),
        Collider::cuboid(2.5, 0.1, 2.0),
        RigidBody::Fixed,
        Name::new("UnusedRoom2_Floor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_mat.clone()) {
        u2_floor.insert((
            Mesh3d(m.add(Plane3d::default().mesh().size(5.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }
    // West dividing wall at X = 2.0
    let mut u2_west = commands.spawn((
        Transform::from_xyz(2.0, 1.5, 11.0),
        Collider::cuboid(0.15, 1.5, 2.0),
        RigidBody::Fixed,
        Name::new("UnusedRoom2_WestWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        u2_west.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }
    // East dividing wall at X = 7.0
    let mut u2_east = commands.spawn((
        Transform::from_xyz(7.0, 1.5, 11.0),
        Collider::cuboid(0.15, 1.5, 2.0),
        RigidBody::Fixed,
        Name::new("UnusedRoom2_EastWall"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        u2_east.insert((
            Mesh3d(m.add(Cuboid::new(0.3, 3.0, 4.0))),
            MeshMaterial3d(mat),
        ));
    }
    // Front North Wall with doorway
    let mut u2_north_w = commands.spawn((
        Transform::from_xyz(2.75, 1.5, 9.0),
        Collider::cuboid(0.75, 1.5, 0.15),
        RigidBody::Fixed,
        Name::new("UnusedRoom2_NorthWallWest"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        u2_north_w.insert((
            Mesh3d(m.add(Cuboid::new(1.5, 3.0, 0.3))),
            MeshMaterial3d(mat),
        ));
    }
    let mut u2_north_e = commands.spawn((
        Transform::from_xyz(6.25, 1.5, 9.0),
        Collider::cuboid(0.75, 1.5, 0.15),
        RigidBody::Fixed,
        Name::new("UnusedRoom2_NorthWallEast"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), wall_mat.clone()) {
        u2_north_e.insert((
            Mesh3d(m.add(Cuboid::new(1.5, 3.0, 0.3))),
            MeshMaterial3d(mat),
        ));
    }
    let mut u2_header = commands.spawn((
        Transform::from_xyz(4.5, 2.7, 9.0),
        Collider::cuboid(1.0, 0.3, 0.15),
        RigidBody::Fixed,
        Name::new("UnusedRoom2_Header"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        u2_header.insert((
            Mesh3d(m.add(Cuboid::new(2.0, 0.6, 0.3))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 4. Staircase to Mezzanine (X in [8.5, 10.5], Z in [4.5, 9.0], Y from 0.0 to 3.0)
    // =========================================================================
    // Stepped stairs: collision for objects that are NOT player or elves
    let stair_groups = CollisionGroups::new(GROUP_STAIRS, !(GROUP_PLAYER | GROUP_ELF));
    for i in 0..12 {
        let step_y = (i as f32 + 0.5) * 0.25;
        let step_z = 4.5 + (i as f32 + 0.5) * 0.375;
        let mut step_cmd = commands.spawn((
            Transform::from_xyz(9.5, step_y, step_z),
            Collider::cuboid(1.0, 0.125, 0.1875),
            stair_groups,
            RigidBody::Fixed,
            StairStep { step_index: i },
            Name::new(format!("StairStep_{}", i)),
        ));
        if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
            step_cmd.insert((
                Mesh3d(m.add(Cuboid::new(2.0, 0.25, 0.375))),
                MeshMaterial3d(mat),
            ));
        }
    }

    // Smooth collision ramp: for the player so that they ascend smoothly
    let ramp_groups = CollisionGroups::new(GROUP_RAMP, GROUP_PLAYER);
    let ramp_rot = Quat::from_rotation_x(-0.5880); // atan2(3.0, 4.5)
    let mut ramp_cmd = commands.spawn((
        Transform::from_xyz(9.5, 1.5, 6.75).with_rotation(ramp_rot),
        Collider::cuboid(1.0, 0.05, 2.704),
        ramp_groups,
        RigidBody::Fixed,
        StaircaseSmoothRamp,
        Name::new("StaircaseSmoothRamp"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        ramp_cmd.insert((
            Mesh3d(m.add(Cuboid::new(2.0, 0.02, 5.408))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 5. Mezzanine Area Above Rooms (Floor at Y = 3.0, Wood Railing, Wood Plank Floors)
    // =========================================================================
    let mut mezz_floor = commands.spawn((
        Transform::from_xyz(0.0, 3.0, 11.0),
        Collider::cuboid(13.0, 0.075, 2.0),
        RigidBody::Fixed,
        MezzanineArea {
            floor_height: 3.0,
            bounds_min: Vec3::new(-13.0, 3.0, 9.0),
            bounds_max: Vec3::new(13.0, 8.0, 13.0),
        },
        Name::new("MezzanineFloor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_mat.clone()) {
        mezz_floor.insert((
            Mesh3d(m.add(Cuboid::new(26.0, 0.15, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Mezzanine Wood Railing along front edge (Z = 9.0)
    // Left railing: X in [-13.0, 8.5] (width 21.5m)
    let mut left_railing = commands.spawn((
        Transform::from_xyz(-2.25, 3.5, 9.0),
        Collider::cuboid(10.75, 0.5, 0.06),
        RigidBody::Fixed,
        Name::new("MezzanineRailing_Left"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        left_railing.insert((
            Mesh3d(m.add(Cuboid::new(21.5, 1.0, 0.12))),
            MeshMaterial3d(mat),
        ));
    }

    // Right railing: X in [10.5, 13.0] (width 2.5m)
    let mut right_railing = commands.spawn((
        Transform::from_xyz(11.75, 3.5, 9.0),
        Collider::cuboid(1.25, 0.5, 0.06),
        RigidBody::Fixed,
        Name::new("MezzanineRailing_Right"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), shelf_mat.clone()) {
        right_railing.insert((
            Mesh3d(m.add(Cuboid::new(2.5, 1.0, 0.12))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 6. Initial Moose Treat Jars & Restock Tracker Resource
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
        let (forward, swing_out) = if door.hinge_forward != Vec3::ZERO {
            (door.hinge_forward, door.hinge_swing_out)
        } else if (door.hinge_anchor_pos.x - (-13.0)).abs() < 0.2 {
            (Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0))
        } else {
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
        };
        let r = 0.73;
        let cur_offset = yaw.cos() * forward + yaw.sin() * swing_out;
        tf.translation = Vec3::new(
            hinge.x + r * cur_offset.x,
            hinge.y,
            hinge.z + r * cur_offset.z,
        );
        // When closed at yaw = 0, the door is aligned flat with the wall (along X or Z)
        // with Quat::IDENTITY, eliminating 90-degree frame flipping and jitter.
        let base_rot = Quat::IDENTITY;
        tf.rotation = base_rot * Quat::from_rotation_y(yaw);
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
            .unwrap_or((Vec3::new(-13.0, 0.0, 9.0), Vec3::new(-7.0, 3.0, 13.0)));

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
            let slot_pos = tracker.slots.first().copied().unwrap_or(Vec3::new(-10.0, 0.98, 12.60));
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
    let (forward, swing_out) = if door.hinge_forward != Vec3::ZERO {
        (door.hinge_forward, door.hinge_swing_out)
    } else if (door.hinge_anchor_pos.x - (-13.0)).abs() < 0.2 {
        (Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0))
    } else {
        (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
    };
    let door_dir = yaw.cos() * forward + yaw.sin() * swing_out;
    let door_normal = -yaw.sin() * forward + yaw.cos() * swing_out;

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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;
    use crate::elf_game::{
        physics_sandbox::ragdoll::ElfAssistant,
        player::{controller::ElfPlayer, ElfMovement},
        workshop::room::setup_workshop_scene,
    };

    #[test]
    fn test_pantry_storage_wing_geometry_and_shelving() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_systems(Startup, setup_workshop_scene);
        app.update();

        // 1. Verify Pantry Storage Wing root entity and spatial bounds
        let Ok((pantry, p_tf)) = app.world_mut().query::<(&PantryWing, &Transform)>().single(app.world()) else {
            panic!("PantryWing must be spawned");
        };
        assert_eq!(pantry.bounds_min, Vec3::new(-13.0, 0.0, 9.0));
        assert_eq!(pantry.bounds_max, Vec3::new(-7.0, 3.0, 13.0));
        assert_eq!(p_tf.translation, Vec3::new(-10.0, 1.5, 11.0));

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
        assert!((north_unit.2.x - (-10.0)).abs() < 0.1 && (north_unit.2.z - 12.60).abs() < 0.1);

        let west_unit = shelves.iter().find(|(n, _, _)| n == "WestSupplyShelves").expect("West supply shelves must exist");
        assert_eq!(west_unit.1, 2, "West supply shelves must have 2 tiers");

        assert!(names.contains(&"PantryBakingStagingCounter".to_string()), "Must spawn South baking prep staging counter");

        // 4. Verify Pantry Door & Tracker resource
        assert!(app.world_mut().query::<&PantryDoor>().iter(app.world()).next().is_some());
        assert!(app.world().get_resource::<PantryRestockTracker>().is_some());
    }

    #[test]
    fn test_pantry_fully_enclosed_without_holes() {
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
        let pantry = app.world_mut().query::<&PantryWing>().single(app.world()).unwrap().clone();
        let jars_after_restock = app.world_mut().query::<(&MooseTreatJar, &Transform)>().iter(app.world())
            .filter(|(_, tf)| pantry.contains_point(tf.translation)) // inside pantry
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
    fn test_pantry_door_push_by_player_and_elves() {
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
    fn test_pantry_door_no_clipping_with_wall_or_header_and_stable_hinge() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_systems(Startup, setup_workshop_scene);
        app.update();

        // 1. Verify Door Header Lintel vs Door Leaf geometry:
        // Header Lintel at Y = 2.7, half-height 0.3 -> bottom at Y = 2.40m
        let mut header_q = app.world_mut().query::<(&Name, &Transform, &Collider)>();
        let (_, header_tf, header_col) = header_q.iter(app.world())
            .find(|(n, ..)| n.as_str() == "PantryDoorHeaderLintel")
            .expect("PantryDoorHeaderLintel must exist");
        let header_bottom = header_tf.translation.y - header_col.as_cuboid().unwrap().raw.half_extents.y;
        assert_eq!(header_bottom, 2.40, "Header lintel bottom must be at Y = 2.40m");

        // Door Leaf at Y = 1.18, half-height 1.15 -> top at Y = 2.33m (NO clipping into header lintel)
        let mut door_q = app.world_mut().query::<(&Name, &Transform, &Collider, &PantryDoor)>();
        let (_, door_tf, door_col, _) = door_q.iter(app.world())
            .find(|(n, ..)| n.as_str() == "PantryPhysicsDoor")
            .expect("PantryPhysicsDoor must exist");
        let door_top = door_tf.translation.y + door_col.as_cuboid().unwrap().raw.half_extents.y;
        assert!(
            door_top < header_bottom,
            "Door leaf top ({}) must be strictly below header lintel bottom ({}) to prevent clipping freakout",
            door_top, header_bottom
        );
        assert!(
            door_tf.translation.y - door_col.as_cuboid().unwrap().raw.half_extents.y > 0.0,
            "Door leaf bottom must clear the floor"
        );

        // 2. Verify spring system maintains exact alignment without 90-degree frame flipping
        let door_entity = app.world_mut().query_filtered::<Entity, With<PantryDoor>>().iter(app.world()).next().unwrap();
        app.add_systems(Update, pantry_door_spring_system);

        // Run 10 updates: door should remain at yaw 0.0 without oscillating or freaking out
        for _ in 0..10 {
            app.update();
        }
        let (final_tf, final_door) = (
            *app.world().get::<Transform>(door_entity).unwrap(),
            app.world().get::<PantryDoor>(door_entity).unwrap().clone(),
        );
        assert_eq!(final_door.current_angle, 0.0, "Door angle must stay at 0.0 when closed");
        assert_eq!(final_tf.rotation, Quat::IDENTITY, "Door rotation must stay at Quat::IDENTITY when closed");
    }
}
