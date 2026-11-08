use bevy::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    core::WorkshopQuota,
    player::ElfMovement,
    player::hands::Grabbable,
    physics_sandbox::{SoftBodyLattice, SoftBodyDeformableMesh},
    wrapping::{Present, WrappingSpecification, PresentDeliveredMessage},
};

#[derive(Component)]
pub struct WorkshopWorkbench;

#[derive(Component)]
pub struct ConveyorBelt {
    pub speed: f32,
    pub direction: Vec3,
}

#[derive(Component)]
pub struct PresentDeliveryChute;

#[derive(Component)]
pub struct ConveyorDespawnChute;

#[derive(Component, Debug, Clone)]
pub struct ConveyorDespawnDuct;

#[derive(Component, Debug, Clone)]
pub struct ConveyorSpawnerDuct;

#[derive(Component, Debug, Clone)]
pub struct PresentVacuumDespawner;

#[derive(Component)]
pub struct BreakableWindowPane {
    pub is_broken: bool,
}

#[derive(Component, Debug, Clone)]
pub struct CandleChandelier {
    pub base_intensity: f32,
    pub flicker_seed: f32,
}

#[derive(Component, Debug, Clone)]
pub struct ChandelierCeilingAnchor;

#[derive(Component, Debug, Clone)]
pub struct CandleFlameEmitter {
    pub smoke_timer: Timer,
    pub spark_timer: Timer,
    pub candle_index: usize,
}

impl Default for CandleFlameEmitter {
    fn default() -> Self {
        Self {
            smoke_timer: Timer::from_seconds(0.16, TimerMode::Repeating),
            spark_timer: Timer::from_seconds(1.1, TimerMode::Repeating),
            candle_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleParticleKind {
    Smoke,
    Spark,
}

#[derive(Component, Debug, Clone)]
pub struct CandleParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub kind: CandleParticleKind,
    pub base_color: Color,
}

#[derive(Component)]
pub struct SendPlatformButton;

#[derive(Component, Debug, Clone)]
pub struct PresentSendingPlatform {
    pub is_active: bool,
    pub suction_timer: f32,
    pub active_duration: f32,
    pub cooldown_timer: f32,
    pub player_despawner_contact_time: f32,
}

impl Default for PresentSendingPlatform {
    fn default() -> Self {
        Self {
            is_active: false,
            suction_timer: 0.0,
            active_duration: 0.0,
            cooldown_timer: 0.0,
            player_despawner_contact_time: 0.0,
        }
    }
}

/// A cylinder area over the present platform that produces the tractor beam gravity draw
/// towards the overhead vacuum opening. Suction is proportional to distance to the despawner.
#[derive(Component, Debug, Clone)]
pub struct VacuumTractorBeamCone {
    pub platform_center: Vec3,
    pub opening_center: Vec3,
    pub platform_radius: f32,
    pub opening_radius: f32,
    pub gravity_draw_strength: f32,
    pub is_active: bool,
}

impl Default for VacuumTractorBeamCone {
    fn default() -> Self {
        Self {
            platform_center: Vec3::new(5.5, 0.05, -4.0),
            opening_center: Vec3::new(5.5, 4.0, -4.0),
            platform_radius: 1.45,
            opening_radius: 1.45,
            gravity_draw_strength: 4.0,
            is_active: false,
        }
    }
}

impl VacuumTractorBeamCone {
    pub fn new(platform_center: Vec3, opening_center: Vec3, platform_radius: f32, opening_radius: f32) -> Self {
        Self {
            platform_center,
            opening_center,
            platform_radius,
            opening_radius,
            gravity_draw_strength: 4.0,
            is_active: false,
        }
    }

    /// Determines if a world position falls inside the tractor beam cylinder area over the platform.
    pub fn contains_point(&self, pos: Vec3) -> bool {
        // Vertical bounds: from slightly below platform floor to slightly above vacuum opening
        if pos.y < self.platform_center.y - 0.25 || pos.y > self.opening_center.y + 0.45 {
            return false;
        }

        let dx = pos.x - self.platform_center.x;
        let dz = pos.z - self.platform_center.z;
        let h_dist_sq = dx * dx + dz * dz;

        // Cylinder area: constant radius column across the entire vertical height
        let r = self.platform_radius.max(self.opening_radius);
        h_dist_sq <= r * r
    }

    /// Computes the gravitational suction acceleration vector towards the vacuum opening.
    /// Suction acceleration is proportional to the distance between the physics object and the despawner opening.
    pub fn compute_gravity_draw(&self, pos: Vec3) -> Vec3 {
        let to_opening = self.opening_center - pos;
        let dist = to_opening.length();
        if dist < 0.01 {
            return Vec3::ZERO;
        }
        let dir = to_opening / dist;
        // Suction acceleration proportional to the distance between physics object and despawner
        let suction_strength = self.gravity_draw_strength * dist;
        // Directional suction towards vacuum opening + counter-gravity lift
        dir * suction_strength + Vec3::Y * 9.81
    }
}

#[derive(Component)]
pub struct VacuumTubeSegment {
    pub index: usize,
    pub base_scale: Vec3,
    pub rest_position: Vec3,
}

#[derive(Component)]
pub struct TubeTravellingLump {
    pub progress: f32,
    pub speed: f32,
    pub mass: f32,
    pub size: f32,
}

#[derive(Resource)]
pub struct ConveyorSpawnerTimer {
    pub timer: Timer,
}

impl Default for ConveyorSpawnerTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.5, TimerMode::Repeating),
        }
    }
}

/// Spawns the Santa's Workshop room: walls, floor, tables, conveyors, delivery chute, windows, and outdoor snowy scene
pub fn setup_workshop_scene(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    if meshes.is_none() || materials.is_none() {
        super::progression::spawn_storage_wing_and_forbidden_workshop(
            &mut commands,
            &mut meshes,
            &mut materials,
            workshop_mats.as_deref(),
        );
        super::pantry::spawn_pantry_storage_wing(
            &mut commands,
            &mut meshes,
            &mut materials,
            workshop_mats.as_deref(),
        );
        return;
    }

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let floor_mat = workshop_mats.as_ref().map(|wm| wm.rustic_planks.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: RUSTIC_PLANKS,
            perceptual_roughness: 0.85,
            ..default()
        })
    });

    // 1. Room Floor: Warm wooden workshop floor (26m x 26m)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(26.0, 26.0))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider::cuboid(13.0, 0.1, 13.0),
        RigidBody::Fixed,
        Name::new("WorkshopFloor"),
    ));

    // Cabin Materials: Warm rustic logs, heavy structural timber beams, and wrought iron
    let wall_material = workshop_mats.as_ref().map(|wm| wm.cabin_log.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: CABIN_LOG,
            perceptual_roughness: 0.85,
            ..default()
        })
    });

    let wood_beam_material = workshop_mats.as_ref().map(|wm| wm.timber_beam.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: TIMBER_BEAM,
            perceptual_roughness: 0.9,
            ..default()
        })
    });

    let corner_post_material = workshop_mats.as_ref().map(|wm| wm.weathered_post.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: WEATHERED_POST,
            perceptual_roughness: 0.95,
            ..default()
        })
    });

    let window_glass_material = workshop_mats.as_ref().map(|wm| wm.window_glass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: WINDOW_PANE_GLASS,
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.1,
            metallic: 0.1,
            ..default()
        })
    });

    let window_frame_material = workshop_mats.as_ref().map(|wm| wm.pine_sign.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PINE_SIGN_BOARD,
            perceptual_roughness: 0.7,
            ..default()
        })
    });

    // 2. High Vaulted Pitch Ceiling (Steep Pitched A-frame rising from Y = 5.2m to a 14.3m ridge peak)
    // Pitch angle: 35 degrees (0.6109 rad), rise = 9.1m over 13m half-span, peak apex at Y = 14.3m
    // West Roof Slope: rises from Y = 5.2m at X = -13.0m up to Y = 14.3m at X = 0.0m (positive rotation around Z)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(15.87, 0.35, 26.0))),
        MeshMaterial3d(wood_beam_material.clone()),
        Transform::from_xyz(-6.5, 9.75, 0.0).with_rotation(Quat::from_rotation_z(0.6109)),
        Collider::cuboid(7.935, 0.175, 13.0),
        RigidBody::Fixed,
        Name::new("VaultedRoofWest"),
    ));

    // East Roof Slope: rises from Y = 5.2m at X = +13.0m up to Y = 14.3m at X = 0.0m (negative rotation around Z)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(15.87, 0.35, 26.0))),
        MeshMaterial3d(wood_beam_material.clone()),
        Transform::from_xyz(6.5, 9.75, 0.0).with_rotation(Quat::from_rotation_z(-0.6109)),
        Collider::cuboid(7.935, 0.175, 13.0),
        RigidBody::Fixed,
        Name::new("VaultedRoofEast"),
    ));

    // Central Ridge Apex Beam (runs full 26m length of room along Z at X = 0.0, Y = 14.25m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.7, 26.0))),
        MeshMaterial3d(wood_beam_material.clone()),
        Transform::from_xyz(0.0, 14.25, 0.0),
        Collider::cuboid(0.3, 0.35, 13.0),
        RigidBody::Fixed,
        Name::new("VaultedRidgeBeam"),
    ));

    // Heavy Timber Cross Beams (King Post Trusses) spanning across the workshop width at Y = 5.2m
    for tz in [-9.0, -4.5, 0.0, 4.5, 9.0] {
        // Horizontal Tie Beam spanning from West wall to East wall
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(26.0, 0.45, 0.45))),
            MeshMaterial3d(wood_beam_material.clone()),
            Transform::from_xyz(0.0, 5.2, tz),
            Collider::cuboid(13.0, 0.225, 0.225),
            RigidBody::Fixed,
            Name::new("TimberCrossTieBeam"),
        ));

        // Vertical King Post connecting tie beam at Y = 5.2m to roof apex at Y = 14.25m
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.45, 9.05, 0.45))),
            MeshMaterial3d(wood_beam_material.clone()),
            Transform::from_xyz(0.0, 9.725, tz),
            Collider::cuboid(0.225, 4.525, 0.225),
            RigidBody::Fixed,
            Name::new("TimberKingPost"),
        ));

        // Diagonal truss support struts from tie beam up to rafters
        for (sx, s_rot) in [(-4.5, -0.6109), (4.5, 0.6109)] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.35, 5.2, 0.35))),
                MeshMaterial3d(wood_beam_material.clone()),
                Transform::from_xyz(sx, 7.3, tz).with_rotation(Quat::from_rotation_z(s_rot)),
                Collider::cuboid(0.175, 2.6, 0.175),
                RigidBody::Fixed,
                Name::new("TimberTrussStrut"),
            ));
        }
    }

    // Heavy Corner Log Posts (4 corners of room)
    for (cx, cz) in [(-13.0, -13.0), (13.0, -13.0), (-13.0, 13.0), (13.0, 13.0)] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 5.2, 0.8))),
            MeshMaterial3d(corner_post_material.clone()),
            Transform::from_xyz(cx, 2.6, cz),
            Collider::cuboid(0.4, 2.6, 0.4),
            RigidBody::Fixed,
            Name::new("CabinCornerLogPost"),
        ));
    }

    // 3. Workshop Cabin Walls
    // Back wall (South: Z = 13.0) - with central doorway for Forbidden Back Workshop Door
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(11.0, 5.2, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-7.5, 2.6, 13.0),
        Collider::cuboid(5.5, 2.6, 0.2),
        RigidBody::Fixed,
        Name::new("WallSouthWest"),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(11.0, 5.2, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(7.5, 2.6, 13.0),
        Collider::cuboid(5.5, 2.6, 0.2),
        RigidBody::Fixed,
        Name::new("WallSouthEast"),
    ));

    // South Gable upper wall tiered triangular fill (Y = 5.2 to 14.3m)
    for (tier_w, tier_h, tier_y, name) in [
        (21.5, 3.0, 6.7, "GableSouthLower"),
        (13.0, 3.0, 9.7, "GableSouthMid"),
        (4.5, 3.0, 12.7, "GableSouthApex"),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(tier_w, tier_h, 0.4))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_xyz(0.0, tier_y, 13.0),
            Collider::cuboid(tier_w * 0.5, tier_h * 0.5, 0.2),
            RigidBody::Fixed,
            Name::new(name),
        ));
    }

    // North Gable upper wall tiered triangular fill (Y = 5.2 to 14.3m)
    for (tier_w, tier_h, tier_y, name) in [
        (21.5, 3.0, 6.7, "GableNorthLower"),
        (13.0, 3.0, 9.7, "GableNorthMid"),
        (4.5, 3.0, 12.7, "GableNorthApex"),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(tier_w, tier_h, 0.4))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_xyz(0.0, tier_y, -13.0),
            Collider::cuboid(tier_w * 0.5, tier_h * 0.5, 0.2),
            RigidBody::Fixed,
            Name::new(name),
        ));
    }

    // Left wall (West: X = -13.0) - with Side Snowy Observation Windows and doorway to Pantry Storage Wing (Z = 5.5)
    // North section of lower wall (Z in [-13.0, 4.7])
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 1.2, 17.7))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-13.0, 0.6, -4.15),
        Collider::cuboid(0.2, 0.6, 8.85),
        RigidBody::Fixed,
        Name::new("WallWestLower"),
    ));
    // South section of lower wall (Z in [6.3, 13.0])
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 1.2, 6.7))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-13.0, 0.6, 9.65),
        Collider::cuboid(0.2, 0.6, 3.35),
        RigidBody::Fixed,
        Name::new("WallWestLowerSouth"),
    ));
    // West Windows along North section
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 2.8, 14.0))),
        MeshMaterial3d(window_glass_material.clone()),
        Transform::from_xyz(-13.0, 2.6, -4.0),
        Collider::cuboid(0.1, 1.4, 7.0),
        RigidBody::Fixed,
        Name::new("WestWindowGlass"),
    ));
    // North section of upper wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 1.5, 17.7))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-13.0, 4.75, -4.15),
        Collider::cuboid(0.2, 0.75, 8.85),
        RigidBody::Fixed,
        Name::new("WallWestUpper"),
    ));
    // South section of upper wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 1.5, 6.7))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-13.0, 4.75, 9.65),
        Collider::cuboid(0.2, 0.75, 3.35),
        RigidBody::Fixed,
        Name::new("WallWestUpperSouth"),
    ));
    // Wall Header above Pantry Doorway (Y in [2.7, 5.5], Z in [4.7, 6.3])
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 2.8, 1.6))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-13.0, 4.1, 5.5),
        Collider::cuboid(0.2, 1.4, 0.8),
        RigidBody::Fixed,
        Name::new("WallWestPantryHeader"),
    ));

    // Right wall (East: X = 13.0) - with central doorway for Storage Wing Door
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 5.5, 11.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(13.0, 2.75, -7.5),
        Collider::cuboid(0.2, 2.75, 5.5),
        RigidBody::Fixed,
        Name::new("WallEastNorth"),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 5.5, 11.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(13.0, 2.75, 7.5),
        Collider::cuboid(0.2, 2.75, 5.5),
        RigidBody::Fixed,
        Name::new("WallEastSouth"),
    ));

    // North wall (Front: Z = -13.0):
    // 1. Left North Wall section (from X = -13.0 to X = -11.6, width 1.4m, center X = -12.3)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.4, 5.5, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-12.3, 2.75, -13.0),
        Collider::cuboid(0.7, 2.75, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthLeft"),
    ));

    // Solid wall backing behind the large Oven and Gas Furnace Stove (from X = -11.6 to X = -5.6, width 6.0m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 5.5, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-8.6, 2.75, -13.0),
        Collider::cuboid(3.0, 2.75, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthStoveBacking"),
    ));

    // 2. Climbable Window Bay (from X = -5.6 to X = -4.0, width 1.6m, center X = -4.8)
    // Low sill height at Y = 0.85m allows player to jump onto the sill and climb directly through into the snowy yard.
    // - Low wall below window sill (Y = 0.0 to 0.85, height 0.85m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.6, 0.85, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-4.8, 0.425, -13.0),
        Collider::cuboid(0.8, 0.425, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthBelowWindow"),
    ));

    // - Upper wall lintel above window (Y = 2.70 to 5.50, height 2.80m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.6, 2.80, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-4.8, 4.10, -13.0),
        Collider::cuboid(0.8, 1.40, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthAboveWindow"),
    ));

    // Climbable Window Sill shelf (Wooden ledge protruding slightly for stepping/climbing)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.6, 0.08, 0.6))),
        MeshMaterial3d(window_frame_material.clone()),
        Transform::from_xyz(-4.8, 0.85, -12.95),
        Collider::cuboid(0.8, 0.04, 0.3),
        RigidBody::Fixed,
        Name::new("WestClimbableWindowSill"),
    ));

    // Wall section / timber post between climbable window and Stable Door (from X = -4.0 to X = -2.9, width 1.1m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.1, 5.5, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-3.45, 2.75, -13.0),
        Collider::cuboid(0.55, 2.75, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthWindowDoorMullion"),
    ));

    // Upper lintel above Stable Door (from X = -2.9 to X = -1.1, width 1.8m, Y = 2.45 to 5.50, height 3.05m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.8, 3.05, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-2.0, 3.975, -13.0),
        Collider::cuboid(0.9, 1.525, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthAboveStableDoor"),
    ));

    // Breakable window / door breach state entity for event integration (at Stable Door X = -2.0)
    commands.spawn((
        BreakableWindowPane { is_broken: false },
        Transform::from_xyz(-2.0, 1.78, -13.0),
        Visibility::default(),
        Name::new("MooseWindowBreachState"),
    ));

    // 3. Right & Center North Wall (Behind the workbench & monitor: X = -1.1 to X = 13.0, width 14.1m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(14.1, 5.5, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(5.95, 2.75, -13.0),
        Collider::cuboid(7.05, 2.75, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthMainSolid"),
    ));

    // 3. Outdoor Snowy Scene & Pine Trees (Outside the window around X = -6.5 and North)
    let snow_ground = workshop_mats.as_ref().map(|wm| wm.snow_white.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SNOW_WHITE,
            perceptual_roughness: 0.95,
            ..default()
        })
    });

    let pine_foliage = workshop_mats.as_ref().map(|wm| wm.pine_green.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PINE_GREEN,
            perceptual_roughness: 0.8,
            ..default()
        })
    });

    let pine_trunk = workshop_mats.as_ref().map(|wm| wm.tree_bark.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: TREE_BARK,
            perceptual_roughness: 0.9,
            ..default()
        })
    });

    // Outdoor snow landscape
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(snow_ground.clone()),
        Transform::from_xyz(0.0, -0.6, -35.0),
        Collider::cuboid(30.0, 0.2, 30.0),
        RigidBody::Fixed,
        Name::new("NorthSnowLandscape"),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 60.0))),
        MeshMaterial3d(snow_ground),
        Transform::from_xyz(-30.0, -0.6, 0.0),
        Collider::cuboid(20.0, 0.2, 30.0),
        RigidBody::Fixed,
        Name::new("WestSnowLandscape"),
    ));

    // Scenic Snowy Pine Trees outside
    let tree_coords = [
        (-10.0, -22.0), (-3.0, -25.0), (-14.0, -28.0), (6.0, -22.0),
        (-20.0, -14.0), (-24.0, -4.0), (-22.0, 8.0), (-6.0, -32.0),
    ];

    let trunk_mesh = meshes.add(Cylinder::new(0.3, 2.5));
    let foliage_lower = meshes.add(Cone { radius: 2.0, height: 3.0 });
    let foliage_upper = meshes.add(Cone { radius: 1.4, height: 2.2 });

    for (tx, tz) in tree_coords {
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(pine_trunk.clone()),
            Transform::from_xyz(tx, 0.7, tz),
            Collider::cylinder(1.25, 0.3),
            RigidBody::Fixed,
            Name::new("SnowyPineTrunk"),
        )).with_children(|trunk| {
            trunk.spawn((
                Mesh3d(foliage_lower.clone()),
                MeshMaterial3d(pine_foliage.clone()),
                Transform::from_xyz(0.0, 2.0, 0.0),
                Name::new("FoliageLower"),
            ));
            trunk.spawn((
                Mesh3d(foliage_upper.clone()),
                MeshMaterial3d(pine_foliage.clone()),
                Transform::from_xyz(0.0, 3.5, 0.0),
                Name::new("FoliageUpper"),
            ));
        });
    }

    // 4. Elevated Conveyor Belt: Spans 17.5m from X = -8.5 to X = 9.0 along Z = 3.0
    let conveyor_material = workshop_mats.as_ref().map(|wm| wm.industrial_dark.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: INDUSTRIAL_DARK,
            perceptual_roughness: 0.5,
            ..default()
        })
    });

    let iron_trestle_mat = workshop_mats.as_ref().map(|wm| wm.cast_iron.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: CAST_IRON,
            metallic: 0.7,
            perceptual_roughness: 0.4,
            ..default()
        })
    });

    // Elevated belt frame (surface at Y = 0.75m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(17.5, 0.3, 1.2))),
        MeshMaterial3d(conveyor_material),
        Transform::from_xyz(0.25, 0.6, 3.0),
        Collider::cuboid(8.75, 0.15, 0.6),
        RigidBody::Fixed,
        ConveyorBelt {
            speed: 2.0,
            direction: Vec3::new(1.0, 0.0, 0.0),
        },
        Name::new("MainConveyorBelt"),
    ));

    // A-frame trestle support legs holding up the conveyor belt
    for lx in [-8.0, -4.0, 0.0, 4.0, 8.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.25, 0.6, 1.2))),
            MeshMaterial3d(iron_trestle_mat.clone()),
            Transform::from_xyz(lx, 0.3, 3.0),
            Collider::cuboid(0.125, 0.3, 0.6),
            RigidBody::Fixed,
            Name::new("ConveyorSupportLeg"),
        ));
    }

    let brass_rim_mat = workshop_mats.as_ref().map(|wm| wm.polished_brass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: POLISHED_BRASS,
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        })
    });

    let duct_metal_mat = workshop_mats.as_ref().map(|wm| wm.sheet_metal.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SHEET_METAL,
            metallic: 0.85,
            perceptual_roughness: 0.28,
            ..default()
        })
    });

    let duct_dark_mat = workshop_mats.as_ref().map(|wm| wm.chute_void.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: CHUTE_PIT_VOID,
            metallic: 0.9,
            perceptual_roughness: 0.9,
            ..default()
        })
    });

    // Spawner side: Metal duct going down from ceiling rafters and opening up where presents spawn (X = -8.5, Z = 3.0)
    // Vertical duct shaft from rafters (Y = 6.2m) down to Y = 1.6m
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.1, 4.6, 1.1))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(-8.5, 3.9, 3.0),
        Collider::cuboid(0.55, 2.3, 0.55),
        RigidBody::Fixed,
        ConveyorSpawnerDuct,
        Name::new("ConveyorSpawnerVerticalDuct"),
    ));

    // Brass mounting bracket collars connecting spawner duct to ceiling cross beams
    for by in [3.2, 5.6] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.18, 0.12, 1.18))),
            MeshMaterial3d(brass_rim_mat.clone()),
            Transform::from_xyz(-8.5, by, 3.0),
            ConveyorSpawnerDuct,
            Name::new("SpawnerDuctMountingCollar"),
        ));
    }

    // Curved dispenser mouth at bottom (Y = 1.0 to 1.6m), opening up facing +X over conveyor belt
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.08, 0.9, 1.1))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(-9.05, 1.25, 3.0),
        ConveyorSpawnerDuct,
        Name::new("SpawnerDuctBackPlate"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.65, 0.08, 1.1))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(-8.35, 1.65, 3.0).with_rotation(Quat::from_rotation_z(-0.45)),
        ConveyorSpawnerDuct,
        Name::new("SpawnerDuctTopHood"),
    ));
    for sz in [-0.55, 0.55] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.9, 0.75, 0.06))),
            MeshMaterial3d(duct_metal_mat.clone()),
            Transform::from_xyz(-8.5, 1.30, 3.0 + sz),
            ConveyorSpawnerDuct,
            Name::new("SpawnerDuctSidePlate"),
        ));
    }

    // Despawner side: Metal duct with a curve down into the above-floor despawner (X = 9.8, Z = 3.0)
    // 1. Above-Floor Despawner Box (repositioned above floor, sits on floor Y=0.0 up to Y=0.70m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.3, 0.7, 1.4))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(9.8, 0.35, 3.0),
        Collider::cuboid(0.65, 0.35, 0.7),
        RigidBody::Fixed,
        ConveyorDespawnChute,
        Name::new("ConveyorDespawnChute"),
    )).with_children(|box_parent| {
        box_parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.1, 0.05, 1.2))),
            MeshMaterial3d(duct_dark_mat),
            Transform::from_xyz(0.0, 0.351, 0.0),
            Name::new("DespawnerVoidOpening"),
        ));
        box_parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.34, 0.08, 1.44))),
            MeshMaterial3d(brass_rim_mat.clone()),
            Transform::from_xyz(0.0, 0.32, 0.0),
            Name::new("DespawnerTopRim"),
        ));
    });

    // 2. Curved metal duct connecting end of conveyor belt (X = 8.85, Y = 0.75) down into despawner
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.55, 0.06, 1.25))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(9.20, 0.65, 3.0).with_rotation(Quat::from_rotation_z(-0.38)),
        ConveyorDespawnDuct,
        Name::new("DespawnDuctSlideRamp1"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.45, 0.06, 1.25))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(9.55, 0.48, 3.0).with_rotation(Quat::from_rotation_z(-0.68)),
        ConveyorDespawnDuct,
        Name::new("DespawnDuctSlideRamp2"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.50, 0.06, 1.25))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(9.10, 1.25, 3.0).with_rotation(Quat::from_rotation_z(-0.30)),
        ConveyorDespawnDuct,
        Name::new("DespawnDuctTopHood1"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.45, 0.06, 1.25))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(9.45, 1.05, 3.0).with_rotation(Quat::from_rotation_z(-0.65)),
        ConveyorDespawnDuct,
        Name::new("DespawnDuctTopHood2"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.40, 0.06, 1.25))),
        MeshMaterial3d(duct_metal_mat.clone()),
        Transform::from_xyz(9.75, 0.75, 3.0).with_rotation(Quat::from_rotation_z(-1.25)),
        ConveyorDespawnDuct,
        Name::new("DespawnDuctTopHood3"),
    ));
    for sz in [-0.63, 0.63] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.1, 0.95, 0.06))),
            MeshMaterial3d(duct_metal_mat.clone()),
            Transform::from_xyz(9.40, 0.82, 3.0 + sz),
            ConveyorDespawnDuct,
            Name::new("DespawnDuctSideCheek"),
        ));
    }

    // 5. Central Wrapping Workbench Table (Width = 4.0m, Depth = 1.8m, Z = -4.0)
    let table_wood = workshop_mats.as_ref().map(|wm| wm.workbench_wood.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: WORKBENCH_WOOD,
            perceptual_roughness: 0.6,
            ..default()
        })
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 0.8, 1.8))),
        MeshMaterial3d(table_wood),
        Transform::from_xyz(0.0, 0.4, -4.0),
        Collider::cuboid(2.0, 0.4, 0.9),
        RigidBody::Fixed,
        WorkshopWorkbench,
        Name::new("WrappingWorkbench"),
    ));

    // 6. Present Sending Platform with Pneumatic Tube Vacuum System
    // Circular sending platform on floor at (5.5, 0.05, -4.0)
    let platform_mat = workshop_mats.as_ref().map(|wm| wm.platform_metal.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PLATFORM_METAL,
            metallic: 0.6,
            perceptual_roughness: 0.4,
            ..default()
        })
    });

    let hazard_rim_mat = workshop_mats.as_ref().map(|wm| wm.hazard_yellow.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: HAZARD_YELLOW,
            perceptual_roughness: 0.3,
            ..default()
        })
    });

    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.3, 0.1))),
        MeshMaterial3d(platform_mat),
        Transform::from_xyz(5.5, 0.05, -4.0),
        Collider::cylinder(0.05, 1.3),
        RigidBody::Fixed,
        PresentSendingPlatform::default(),
        Name::new("PresentSendingPlatform"),
    )).with_children(|plat| {
        plat.spawn((
            Mesh3d(meshes.add(Cylinder::new(1.35, 0.12))),
            MeshMaterial3d(hazard_rim_mat),
            Transform::from_xyz(0.0, 0.01, 0.0),
            Name::new("PlatformBorder"),
        ));
    });

    // Control Pedestal & Big Red Send Button at (3.8, 0.5, -2.8)
    let btn_pedestal_mat = workshop_mats.as_ref().map(|wm| wm.cast_iron.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: CAST_IRON,
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        })
    });
    let red_plunger_mat = workshop_mats.as_ref().map(|wm| wm.send_button_red.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: FESTIVE_RED,
            emissive: LinearRgba::new(0.6, 0.05, 0.05, 1.0),
            perceptual_roughness: 0.2,
            ..default()
        })
    });

    // Control Pedestal
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.12, 0.9))),
        MeshMaterial3d(btn_pedestal_mat),
        Transform::from_xyz(3.8, 0.45, -2.8),
        Collider::cylinder(0.45, 0.12),
        RigidBody::Fixed,
        Name::new("SendButtonPedestal"),
    ));

    // Big Red Plunger Send Button on top of pedestal (Y = 0.95m)
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.18, 0.10))),
        MeshMaterial3d(red_plunger_mat),
        Transform::from_xyz(3.8, 0.95, -2.8),
        Collider::cylinder(0.05, 0.18),
        RigidBody::Fixed,
        SendPlatformButton,
        Name::new("BigRedSendButton"),
    ));

    // Overhead Circular Intake Funnel at Y = 4.0m directly above sending platform (5.5, 4.0, -4.0)
    let tube_glass_mat = workshop_mats.as_ref().map(|wm| wm.vacuum_tube_glass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: VACUUM_TUBE_GLASS,
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.15,
            metallic: 0.2,
            ..default()
        })
    });

    let brass_collar_mat = workshop_mats.as_ref().map(|wm| wm.polished_brass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: POLISHED_BRASS,
            metallic: 0.85,
            perceptual_roughness: 0.3,
            ..default()
        })
    });

    commands.spawn((
        Mesh3d(meshes.add(Cone { radius: 1.15, height: 0.8 })),
        MeshMaterial3d(brass_collar_mat.clone()),
        Transform::from_xyz(5.5, 4.0, -4.0).with_rotation(Quat::IDENTITY),
        Collider::cylinder(0.4, 1.15),
        RigidBody::Fixed,
        PresentVacuumDespawner,
        Name::new("OverheadSuctionFunnel"),
    ));

    // Invisible / Translucent Tractor Beam Cone Volume spanning from platform (5.5, 0.05, -4.0) to funnel (5.5, 4.0, -4.0)
    let tractor_beam_mat = workshop_mats.as_ref().map(|wm| wm.tractor_beam.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: TRACTOR_BEAM_GLOW,
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::new(0.08, 0.35, 0.55, 0.4),
            perceptual_roughness: 0.1,
            ..default()
        })
    });

    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.35, 3.95))),
        MeshMaterial3d(tractor_beam_mat),
        Transform::from_xyz(5.5, 2.025, -4.0),
        Collider::cylinder(1.975, 1.40),
        Sensor,
        Visibility::Hidden, // Invisible by default; displays translucent glowing beam when vacuum is active
        VacuumTractorBeamCone::default(),
        Name::new("VacuumTractorBeamCone"),
    ));

    // Curved Pneumatic Vacuum Tube with 360-degree loop routing through vaulted rafters
    // Trajectory spline points from intake funnel (5.5, 4.2, -4.0) through a loop to dispatch wall (12.8, 5.5, -4.0)
    let mut spline_points = Vec::new();
    // 1. Rising straight up into rafters
    spline_points.push(Vec3::new(5.5, 4.2, -4.0));
    spline_points.push(Vec3::new(5.5, 4.7, -4.0));
    spline_points.push(Vec3::new(5.5, 5.3, -4.0));
    spline_points.push(Vec3::new(5.6, 5.8, -4.0));
    // 2. 360-degree loop around center (6.2, 6.7, -3.8) radius 0.75m
    let loop_center = Vec3::new(6.2, 6.7, -3.8);
    for i in 0..10 {
        let theta = (i as f32 / 9.0) * std::f32::consts::TAU;
        let lx = loop_center.x + theta.cos() * 0.75;
        let ly = loop_center.y + theta.sin() * 0.75;
        let lz = loop_center.z + (i as f32 - 4.5) * 0.08;
        spline_points.push(Vec3::new(lx, ly, lz));
    }
    // 3. Sweeping across rafters to East dispatch wall
    spline_points.push(Vec3::new(7.5, 6.1, -4.0));
    spline_points.push(Vec3::new(8.5, 5.9, -4.0));
    spline_points.push(Vec3::new(9.6, 5.8, -4.0));
    spline_points.push(Vec3::new(10.7, 5.7, -4.0));
    spline_points.push(Vec3::new(11.8, 5.6, -4.0));
    spline_points.push(Vec3::new(12.8, 5.5, -4.0));

    // Spawn 20 segmented tube sections along spline
    for idx in 0..(spline_points.len() - 1) {
        let p1 = spline_points[idx];
        let p2 = spline_points[idx + 1];
        let mid = (p1 + p2) * 0.5;
        let dir = p2 - p1;
        let len = dir.length();
        let rot = Quat::from_rotation_arc(Vec3::Y, dir.normalize());

        let segment_mesh = meshes.add(Cylinder::new(0.26, len));
        commands.spawn((
            Mesh3d(segment_mesh),
            MeshMaterial3d(tube_glass_mat.clone()),
            Transform::from_translation(mid).with_rotation(rot),
            VacuumTubeSegment {
                index: idx,
                base_scale: Vec3::ONE,
                rest_position: mid,
            },
            Name::new(format!("VacuumTubeSegment_{}", idx)),
        ));

        // Reinforcing brass joint collar at each segment connection
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.30, 0.08))),
            MeshMaterial3d(brass_collar_mat.clone()),
            Transform::from_translation(p1).with_rotation(rot),
            Name::new(format!("TubeJointCollar_{}", idx)),
        ));
    }

    // Terminus chute box on East wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 1.2, 0.8))),
        MeshMaterial3d(brass_collar_mat),
        Transform::from_xyz(12.8, 5.5, -4.0),
        Collider::cuboid(0.4, 0.6, 0.4),
        RigidBody::Fixed,
        PresentDeliveryChute,
        Name::new("PneumaticDispatchTerminus"),
    ));

    // 7. Dynamic Lighting from Candles Hung in Two Rustic Chandeliers
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.95, 0.88),
        brightness: 350.0,
        ..default()
    });

    let iron_chain_mat = workshop_mats.as_ref().map(|wm| wm.wrought_iron.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: WROUGHT_IRON,
            metallic: 0.9,
            perceptual_roughness: 0.35,
            ..default()
        })
    });

    let candle_wax_mat = workshop_mats.as_ref().map(|wm| wm.candle_wax.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: CANDLE_TALLOW,
            perceptual_roughness: 0.5,
            ..default()
        })
    });

    let candle_flame_mat = workshop_mats.as_ref().map(|wm| wm.candle_flame.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: CANDLE_FLAME,
            emissive: LinearRgba::new(1.0, 0.82, 0.25, 1.0),
            unlit: true,
            ..default()
        })
    });

    let candle_cylinder = meshes.add(Cylinder::new(0.035, 0.22));
    let candle_flame_cone = meshes.add(Cone { radius: 0.025, height: 0.06 });

    // Spawn Two Chandeliers: North at (0.0, 4.4, -4.5) and South at (0.0, 4.4, 4.5)
    // Pinned to the ceiling tie beam via a spherical joint, swinging on its chain as a physical object
    for (cz, seed) in [(-4.5, 0.0), (4.5, 3.1415)] {
        // Ceiling Mount Fixed Anchor (tie beam apex at Y = 5.2m)
        let ceiling_anchor = commands.spawn((
            Transform::from_xyz(0.0, 5.2, cz),
            RigidBody::Fixed,
            ChandelierCeilingAnchor,
            Name::new("ChandelierCeilingAnchor"),
        )).id();

        // Spherical Joint: local_anchor1 is anchor origin, local_anchor2 is 0.8m above chandelier hub
        let joint = SphericalJointBuilder::new()
            .local_anchor1(Vec3::ZERO)
            .local_anchor2(Vec3::new(0.0, 0.8, 0.0));

        // Chandelier Hub & Wagon Wheel Ring as Dynamic Rigid Body with Collider & Damping
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(1.35, 0.06))),
            MeshMaterial3d(iron_chain_mat.clone()),
            Transform::from_xyz(0.0, 4.4, cz),
            RigidBody::Dynamic,
            Collider::cylinder(0.06, 1.35),
            AdditionalMassProperties::Mass(25.0),
            Damping {
                linear_damping: 0.35,
                angular_damping: 0.50,
            },
            ExternalImpulse::default(),
            Velocity::zero(),
            PointLight {
                intensity: 480_000.0,
                range: 24.0,
                shadow_maps_enabled: true,
                color: CANDLE_LIGHT_EMISSION, // Warm 2200K candle flame glow
                ..default()
            },
            CandleChandelier {
                base_intensity: 480_000.0,
                flicker_seed: seed,
            },
            CandleFlameEmitter::default(),
            Name::new("RusticCandleChandelier"),
        )).with_children(|ch| {
            // Joint connection linking chandelier body to ceiling anchor
            ch.spawn(ImpulseJoint::new(ceiling_anchor, joint));

            // Iron suspension chain connecting hub (0.0, 0.0, 0.0) up to ceiling pivot (0.0, 0.8, 0.0)
            ch.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.025, 0.8))),
                MeshMaterial3d(iron_chain_mat.clone()),
                Transform::from_xyz(0.0, 0.4, 0.0),
                Name::new("ChandelierHangingChain"),
            ));

            // 8 Candles spaced evenly around the perimeter ring
            for k in 0..8 {
                let theta = (k as f32 / 8.0) * std::f32::consts::TAU;
                let cx = theta.cos() * 1.25;
                let cy_pos = 0.14;
                let cz_pos = theta.sin() * 1.25;

                // Candle wax cylinder
                ch.spawn((
                    Mesh3d(candle_cylinder.clone()),
                    MeshMaterial3d(candle_wax_mat.clone()),
                    Transform::from_xyz(cx, cy_pos, cz_pos),
                    Name::new("ChandelierCandle"),
                ));

                // Glowing flame tip
                ch.spawn((
                    Mesh3d(candle_flame_cone.clone()),
                    MeshMaterial3d(candle_flame_mat.clone()),
                    Transform::from_xyz(cx, cy_pos + 0.14, cz_pos),
                    crate::elf_game::workshop::stove::FlameSource,
                    Name::new("CandleFlameTip"),
                ));
            }
        });
    }

    // Outdoor snowy sunlight shining in through the windows
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadow_maps_enabled: true,
            color: WINTER_SUNLIGHT,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(0.5, -0.8, 0.6), Vec3::Y),
        Name::new("WinterSunlight"),
    ));

    let mut meshes_opt = Some(meshes);
    let mut materials_opt = Some(materials);

    // Spawn Storage Wing & Forbidden Back Workshop doorways, door barriers, and machinery
    super::progression::spawn_storage_wing_and_forbidden_workshop(
        &mut commands,
        &mut meshes_opt,
        &mut materials_opt,
        workshop_mats.as_deref(),
    );

    // Spawn Pantry Storage Wing & interactive restocking physics door
    super::pantry::spawn_pantry_storage_wing(
        &mut commands,
        &mut meshes_opt,
        &mut materials_opt,
        workshop_mats.as_deref(),
    );

    // Spawn Stable Door (Dutch Door) where Mr. Moose peeks and breaches
    super::stable_door::spawn_stable_door(
        &mut commands,
        &mut meshes_opt,
        &mut materials_opt,
        workshop_mats.as_deref(),
    );
}

/// Spawns raw present cubes at the start of the conveyor belt (X = -8.5, Z = 3.0).
/// Halts spawning if workshop temperature drops below freezing (is_frozen).
pub fn conveyor_cube_spawner_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    temperature: Option<Res<crate::elf_game::workshop::stove::WorkshopTemperature>>,
    spawner: Option<ResMut<ConveyorSpawnerTimer>>,
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(time), Some(mut spawner)) = (time, spawner) else {
        return;
    };
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    if let Some(ref temp) = temperature {
        if temp.is_frozen {
            return;
        }
    }
    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let cube_mat = workshop_mats.as_ref().map(|wm| wm.toy_blue.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: UNWRAPPED_TOY_BLUE,
            perceptual_roughness: 0.4,
            ..default()
        })
    });

    let raw_mesh = Mesh::from(Cuboid::new(0.6, 0.6, 0.6));
    let deformable = SoftBodyDeformableMesh::from_mesh(&raw_mesh, Vec3::splat(0.3));
    let mesh_handle = meshes.add(raw_mesh);

    let mut entity_cmds = commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(cube_mat),
        Transform::from_xyz(-8.5, 1.15, 3.0),
        Collider::cuboid(0.3, 0.3, 0.3),
        RigidBody::Dynamic,
        Velocity {
            linear: Vec3::new(2.0, 0.0, 0.0),
            angular: Vec3::ZERO,
        },
        ExternalImpulse::default(),
        SoftBodyLattice::new_cube(Vec3::splat(0.3), 60.0, 6.0),
        Grabbable,
        Present::default(),
        WrappingSpecification::default(),
        Name::new("ConveyorToyCube"),
    ));
    if let Some(def) = deformable {
        entity_cmds.insert(def);
    }
}

/// Moves cubes resting on the conveyor belt forward (+X)
pub fn conveyor_belt_physics_system(
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_conveyor: Query<(&Transform, &ConveyorBelt)>,
    mut q_cubes: Query<(&Transform, &mut Velocity), (With<Present>, With<Grabbable>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Ok((c_tf, belt)) = q_conveyor.single() else {
        return;
    };

    for (p_tf, mut vel) in &mut q_cubes {
        if (p_tf.translation.z - c_tf.translation.z).abs() < 0.65
            && p_tf.translation.y > 0.45
            && p_tf.translation.y < 1.5
            && p_tf.translation.x > -9.0
            && p_tf.translation.x < 9.0
        {
            vel.linear.x = belt.speed;
        }
    }
}

/// Despawns present cubes that fall into the above-floor despawner or sub-floor chute
pub fn conveyor_despawn_chute_system(
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut commands: Commands,
    q_chute: Query<&Transform, With<ConveyorDespawnChute>>,
    q_presents: Query<(Entity, &Transform), With<Present>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Ok(chute_tf) = q_chute.single() else {
        return;
    };

    let chute_pos = chute_tf.translation;

    for (entity, p_tf) in &q_presents {
        let p_pos = p_tf.translation;
        let in_chute_column = (p_pos.x - chute_pos.x).abs() < 1.0
            && (p_pos.z - chute_pos.z).abs() < 1.0;
        let inside_despawner_volume = in_chute_column && (p_pos.y <= chute_pos.y + 0.35);
        let near_chute = p_pos.distance(chute_pos) < 0.85;
        let below_floor = p_pos.y < 0.0 && in_chute_column;

        // Ensure presents still resting on the conveyor belt (X <= 9.05, Y >= 0.65) never despawn early
        let on_belt = p_pos.x <= 9.05 && p_pos.y >= 0.65;

        if !on_belt && (inside_despawner_volume || near_chute || below_floor) {
            commands.entity(entity).despawn();
        }
    }
}

/// Smooth, organic procedural candle flame flickering for the two rustic chandeliers.
/// Allowed to continue running when the game is paused so lighting flickers organically
/// and shadow map render entities remain populated in bevy_pbr.
pub fn chandelier_candle_flicker_system(
    time_real: Option<Res<Time<bevy::time::Real>>>,
    time: Option<Res<Time>>,
    mut q_chandeliers: Query<(&CandleChandelier, &mut PointLight)>,
) {
    let elapsed = time_real
        .as_ref()
        .map(|t| t.elapsed_secs())
        .or_else(|| time.as_ref().map(|t| t.elapsed_secs()))
        .unwrap_or(0.0);

    for (chandelier, mut light) in &mut q_chandeliers {
        let t = elapsed * 7.5 + chandelier.flicker_seed;
        let noise = (t * 1.0).sin() * 0.12 + (t * 2.7).sin() * 0.08 + (t * 5.3).sin() * 0.04;
        let factor = (1.0 + noise).clamp(0.75, 1.25);
        light.intensity = chandelier.base_intensity * factor;
    }
}

/// Allows the player to push or bat the chandeliers with physical impulses, making them swing on their chains
pub fn chandelier_physics_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    q_player: Query<&Transform, With<crate::elf_game::player::controller::ElfPlayer>>,
    q_camera: Query<&GlobalTransform, With<crate::elf_game::player::controller::FirstPersonCamera>>,
    mut q_chandeliers: Query<(&Transform, &mut ExternalImpulse), With<CandleChandelier>>,
) {
    let interact_pressed = keys.as_ref().map(|k| k.just_pressed(KeyCode::KeyQ) || k.just_pressed(KeyCode::KeyE) || k.just_pressed(KeyCode::KeyF)).unwrap_or(false)
        || mouse_buttons.as_ref().map(|m| m.just_pressed(MouseButton::Left)).unwrap_or(false);

    let Ok(player_tf) = q_player.single() else { return };
    let camera_fwd = q_camera.single().map(|gt| gt.forward().into()).unwrap_or(Vec3::NEG_Z);

    for (chandelier_tf, mut impulse) in &mut q_chandeliers {
        let delta = chandelier_tf.translation - player_tf.translation;
        let dist = delta.length();

        // If player is close enough to bat or interact with the chandelier
        if dist < 4.5 && interact_pressed {
            let mut push_dir = camera_fwd;
            if push_dir.length_squared() < 0.01 {
                push_dir = (chandelier_tf.translation - player_tf.translation).normalize_or_zero();
            }
            // Apply physical impulse & tilt torque to the chandelier body
            impulse.impulse += push_dir * 18.0 + Vec3::new(push_dir.z, 0.0, -push_dir.x) * 6.0;
            impulse.torque_impulse += Vec3::new(push_dir.z * 10.0, 0.0, -push_dir.x * 10.0);
        }
    }
}

/// Emits light fading smoke wisps and occasional ascending spark embers from candle flame tips.
/// Allowed to continue running when the game is paused so ambient candle smoke and sparks continue to drift.
pub fn candle_particle_emitter_system(
    mut commands: Commands,
    time_real: Option<Res<Time<bevy::time::Real>>>,
    time: Option<Res<Time>>,
    mut q_emitters: Query<(&GlobalTransform, &mut CandleFlameEmitter)>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
) {
    let dt = time_real
        .as_ref()
        .map(|t| t.delta())
        .or_else(|| time.as_ref().map(|t| t.delta()))
        .unwrap_or_default();
    if dt.is_zero() {
        return;
    }
    let has_assets = meshes.is_some() && (materials.is_some() || workshop_mats.is_some());

    let (smoke_mat, spark_mat) = if let Some(ref wm) = workshop_mats {
        (Some(wm.candle_smoke.clone()), Some(wm.candle_spark.clone()))
    } else {
        (None, None)
    };

    let sphere_mesh = meshes.as_mut().map(|m| m.add(Sphere::new(0.035)));
    let spark_mesh = meshes.as_mut().map(|m| m.add(Sphere::new(0.018)));

    for (gt, mut emitter) in &mut q_emitters {
        emitter.smoke_timer.tick(dt);
        emitter.spark_timer.tick(dt);

        // 1. Light smoke wisp emission
        if emitter.smoke_timer.just_finished() {
            emitter.candle_index = (emitter.candle_index + 1) % 8;
            let theta = (emitter.candle_index as f32 / 8.0) * std::f32::consts::TAU;
            let local_flame_pos = Vec3::new(theta.cos() * 1.25, 0.28, theta.sin() * 1.25);
            let world_flame_pos = gt.transform_point(local_flame_pos);

            let jitter_x = ((emitter.candle_index as f32 * 1.7).sin() * 0.05).clamp(-0.04, 0.04);
            let jitter_z = ((emitter.candle_index as f32 * 2.3).cos() * 0.05).clamp(-0.04, 0.04);
            let vy = 0.65 + ((emitter.candle_index as f32 * 3.1).sin() * 0.15);

            let mut particle_cmd = commands.spawn((
                Transform::from_translation(world_flame_pos).with_scale(Vec3::splat(0.025)),
                CandleParticle {
                    velocity: Vec3::new(jitter_x, vy, jitter_z),
                    lifetime: 0.0,
                    max_lifetime: 1.4,
                    start_scale: 0.025,
                    end_scale: 0.075,
                    kind: CandleParticleKind::Smoke,
                    base_color: CANDLE_SMOKE,
                },
                Name::new("CandleSmokeParticle"),
            ));

            if has_assets {
                if let (Some(ref sm), Some(ref s_mesh)) = (smoke_mat.clone(), sphere_mesh.clone()) {
                    particle_cmd.insert((
                        Mesh3d(s_mesh.clone()),
                        MeshMaterial3d(sm.clone()),
                    ));
                }
            }
        }

        // 2. Rising glowing spark emission
        if emitter.spark_timer.just_finished() {
            let spark_candle = (emitter.candle_index + 3) % 8;
            let theta = (spark_candle as f32 / 8.0) * std::f32::consts::TAU;
            let local_flame_pos = Vec3::new(theta.cos() * 1.25, 0.29, theta.sin() * 1.25);
            let world_flame_pos = gt.transform_point(local_flame_pos);

            let spark_count = 1 + (spark_candle % 2);
            for s_i in 0..spark_count {
                let offset_angle = theta + s_i as f32 * 0.8;
                let spark_vx = offset_angle.cos() * 0.12;
                let spark_vz = offset_angle.sin() * 0.12;
                let spark_vy = 1.4 + (s_i as f32 * 0.4);

                let mut spark_cmd = commands.spawn((
                    Transform::from_translation(world_flame_pos).with_scale(Vec3::splat(0.02)),
                    CandleParticle {
                        velocity: Vec3::new(spark_vx, spark_vy, spark_vz),
                        lifetime: 0.0,
                        max_lifetime: 0.60 + (s_i as f32 * 0.15),
                        start_scale: 0.02,
                        end_scale: 0.005,
                        kind: CandleParticleKind::Spark,
                        base_color: CANDLE_SPARK,
                    },
                    Name::new("CandleSparkParticle"),
                ));

                if has_assets {
                    if let (Some(ref sp), Some(ref p_mesh)) = (spark_mat.clone(), spark_mesh.clone()) {
                        spark_cmd.insert((
                            Mesh3d(p_mesh.clone()),
                            MeshMaterial3d(sp.clone()),
                        ));
                    }
                }
            }
        }
    }
}

/// Updates candle smoke wisps and ascending spark particles over time.
/// Allowed to continue running when the game is paused so smoke and spark particles animate seamlessly.
pub fn candle_particle_system(
    mut commands: Commands,
    time_real: Option<Res<Time<bevy::time::Real>>>,
    time: Option<Res<Time>>,
    mut q_particles: Query<(Entity, &mut Transform, &mut CandleParticle)>,
) {
    let dt = time_real
        .as_ref()
        .map(|t| t.delta_secs())
        .or_else(|| time.as_ref().map(|t| t.delta_secs()))
        .unwrap_or(0.0);
    if dt <= 0.0 {
        return;
    }
    let elapsed = time_real
        .as_ref()
        .map(|t| t.elapsed_secs())
        .or_else(|| time.as_ref().map(|t| t.elapsed_secs()))
        .unwrap_or(0.0);

    for (entity, mut tf, mut particle) in &mut q_particles {
        particle.lifetime += dt;
        if particle.lifetime >= particle.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = (particle.lifetime / particle.max_lifetime).clamp(0.0, 1.0);

        // Scale interpolation
        let cur_scale = particle.start_scale * (1.0 - progress) + particle.end_scale * progress;
        tf.scale = Vec3::splat(cur_scale);

        // Motion & buoyancy
        tf.translation += particle.velocity * dt;

        match particle.kind {
            CandleParticleKind::Smoke => {
                // Smoke decelerates slightly as it cools and drifts with gentle turbulent draft
                particle.velocity.y *= (1.0 - 0.35 * dt).max(0.0);
                let wisp_x = ((elapsed * 4.5 + tf.translation.y * 3.0).sin()) * 0.035;
                let wisp_z = ((elapsed * 3.8 + tf.translation.y * 2.5).cos()) * 0.035;
                tf.translation.x += wisp_x * dt;
                tf.translation.z += wisp_z * dt;
            }
            CandleParticleKind::Spark => {
                // Sparks decelerate under air resistance and gravity
                particle.velocity.y -= 1.8 * dt;
                particle.velocity.x *= (1.0 - 0.7 * dt).max(0.0);
                particle.velocity.z *= (1.0 - 0.7 * dt).max(0.0);
            }
        }
    }
}

/// Handles the present sending platform and pneumatic vacuum system:
/// 1. Momentary button activation: vacuum is only on while button is pressed by player or physics object is on it
/// 2. Proportional gravity suction: draw acceleration is proportional to distance to despawner opening, affecting only within the cylinder area
/// 3. Affects player: draws player upwards; if touching despawner for 3 seconds, teleports player on top of the roof
/// 4. Auto-shutoff: automatically turns off after 10 seconds continuous run, with 1-second cooldown penalty
pub fn present_sending_platform_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut q_player: Query<(&mut Transform, &mut ElfMovement), (Without<PresentSendingPlatform>, Without<SendPlatformButton>, Without<VacuumTractorBeamCone>)>,
    q_button: Query<(Option<&GlobalTransform>, &Transform), With<SendPlatformButton>>,
    mut q_platform: Query<(&mut PresentSendingPlatform, &Transform)>,
    mut q_cone: Query<(&mut VacuumTractorBeamCone, Option<&mut Visibility>)>,
    mut q_physics_objects: Query<
        (
            Entity,
            &Transform,
            &mut Velocity,
            Option<&mut crate::elf_game::physics_sandbox::SoftBodyLattice>,
        ),
        (
            Without<ElfMovement>,
            Without<PresentSendingPlatform>,
            Without<SendPlatformButton>,
            Without<VacuumTractorBeamCone>,
        ),
    >,
    mut commands: Commands,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Ok((mut platform, plat_tf)) = q_platform.single_mut() else {
        return;
    };

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let plat_pos = plat_tf.translation; // (5.5, 0.05, -4.0)

    // Build or sync tractor beam cylinder model (Requirement 1)
    let mut beam_cone = VacuumTractorBeamCone::new(
        plat_pos,
        Vec3::new(plat_pos.x, 4.0, plat_pos.z),
        1.45,
        1.45,
    );

    let mut opt_cone_entity = q_cone.iter_mut().next();
    if let Some((ref mut cone, _)) = opt_cone_entity {
        beam_cone = (*cone).clone();
    }

    // Cooldown management (Requirement 4)
    if platform.cooldown_timer > 0.0 {
        platform.cooldown_timer = (platform.cooldown_timer - dt).max(0.0);
    }

    // Check Button Press / Physics Object on Button (Requirement 2)
    let mut button_present = false;
    let mut is_button_held = false;

    if let Some((opt_gt, btn_tf)) = q_button.iter().next() {
        button_present = true;
        let btn_pos = if let Some(gt) = opt_gt {
            let gt_pos = gt.translation();
            if gt_pos != Vec3::ZERO || btn_tf.translation == Vec3::ZERO {
                gt_pos
            } else {
                btn_tf.translation
            }
        } else {
            btn_tf.translation
        };

        // Player interaction: Q, E, F or Left/Right Click within reach (< 3.2m)
        let player_near = q_player.iter().any(|(p_tf, _)| p_tf.translation.distance(btn_pos) < 3.2);
        let button_pressed_by_player = player_near && (
            keys.as_ref().map(|k| {
                k.pressed(KeyCode::KeyQ)
                    || k.pressed(KeyCode::KeyE)
                    || k.pressed(KeyCode::KeyF)
            }).unwrap_or(false)
            || mouse.as_ref().map(|m| {
                m.pressed(MouseButton::Left)
                    || m.pressed(MouseButton::Right)
            }).unwrap_or(false)
        );

        // Physics object on the button: any dynamic body resting on or hitting the button
        let button_held_by_physics = q_physics_objects.iter().any(|(_, p_tf, vel, _)| {
            let p_pos = p_tf.translation;
            let h_dist = (p_pos.x - btn_pos.x).hypot(p_pos.z - btn_pos.z);
            let y_diff = p_pos.y - btn_pos.y;
            let dist_3d = p_pos.distance(btn_pos);

            // Object resting on top of the button cylinder (radius 0.18, height 0.10)
            // or dynamic impact hitting the button
            (h_dist < 0.65 && y_diff >= -0.25 && y_diff <= 1.35)
                || (dist_3d < 0.85 && (vel.linear.length() > 0.05 || y_diff.abs() < 0.5))
        });

        is_button_held = button_pressed_by_player || button_held_by_physics;
    }

    if button_present {
        if platform.cooldown_timer > 0.0 {
            // Still in 1-second cooldown after 10-second automatic shutoff
            platform.is_active = false;
            platform.active_duration = 0.0;
        } else if is_button_held {
            platform.is_active = true;
            platform.active_duration += dt;
            if platform.active_duration >= 10.0 {
                // Requirement 4: automatically turn off after being on for 10 seconds, then have a cooldown of 1 second
                platform.is_active = false;
                platform.active_duration = 0.0;
                platform.cooldown_timer = 1.0;
            }
        } else {
            // Requirement 2: only on while button is pressed or object on button
            platform.is_active = false;
            platform.active_duration = 0.0;
        }
    } else {
        // Direct test activation support when no SendPlatformButton is spawned in scene
        if platform.is_active {
            platform.active_duration += dt;
            if platform.active_duration >= 10.0 {
                platform.is_active = false;
                platform.active_duration = 0.0;
                platform.cooldown_timer = 1.0;
            }
        }
    }

    // Update suction_timer for backwards compatibility with tests checking suction_timer > 0.0
    if platform.is_active {
        platform.suction_timer = (10.0 - platform.active_duration).max(0.01);
    } else {
        platform.suction_timer = 0.0;
    }

    // Sync cone component & visibility with platform activation
    if let Some((ref mut cone, ref mut opt_vis)) = opt_cone_entity {
        cone.is_active = platform.is_active;
        if let Some(vis) = opt_vis {
            **vis = if platform.is_active {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }

    if !platform.is_active {
        // If vacuum is off, reset despawner contact timer
        platform.player_despawner_contact_time = 0.0;
        return;
    }

    // 2. Cylinder Suction / Tractor Beam Gravity Draw on Physics Objects (Requirement 1)
    for (obj_entity, obj_tf, mut vel, mut opt_lattice) in &mut q_physics_objects {
        let pos = obj_tf.translation;

        if beam_cone.contains_point(pos) {
            // Compute gravity draw acceleration towards vacuum opening (proportional to distance)
            let draw_accel = beam_cone.compute_gravity_draw(pos);
            vel.linear += draw_accel * dt;

            // Horizontal centering towards the central suction axis
            let h_diff = Vec3::new(beam_cone.opening_center.x - pos.x, 0.0, beam_cone.opening_center.z - pos.z);
            let h_dist = h_diff.length();
            if h_dist > 0.05 {
                vel.linear.x += (h_diff.x / h_dist) * 6.0 * dt;
                vel.linear.z += (h_diff.z / h_dist) * 6.0 * dt;
            }
            // Gentle lateral damping to stabilize trajectory into the nozzle
            vel.linear.x *= (1.0 - 0.35 * dt).max(0.0);
            vel.linear.z *= (1.0 - 0.35 * dt).max(0.0);

            // Optional soft-body deformation: stretch along the suction axis
            if let Some(ref mut lattice) = opt_lattice {
                let to_opening = (beam_cone.opening_center - pos).normalize_or_zero();
                if to_opening != Vec3::ZERO {
                    lattice.apply_directional_squash(to_opening, 0.25 * dt);
                }
            }

            // Ingestion into PresentVacuumDespawner at the funnel opening
            let dist_to_opening = pos.distance(beam_cone.opening_center);
            if dist_to_opening < 0.95 || (pos.y >= 3.4 && h_dist < 1.15) {
                // NO PRESENT VERIFICATION: Any object is ingested, despawned, and sent through the tube!
                commands.entity(obj_entity).despawn();

                // Spawn travelling lump through the vacuum tube to create the pneumatic transit effect
                commands.spawn((
                    TubeTravellingLump {
                        progress: 0.0,
                        speed: 1.25,
                        mass: 2.0,
                        size: 0.6,
                    },
                    Name::new("VacuumTubeLump"),
                ));
            }
        }
    }

    // 3. Suction on the Player & Teleport to Roof (Requirement 3)
    let despawner_pos = beam_cone.opening_center; // (5.5, 4.0, -4.0)
    let mut player_touching = false;

    for (player_tf, mut movement) in &mut q_player {
        let player_pos = player_tf.translation;

        if beam_cone.contains_point(player_pos) {
            // Apply vacuum suction to player (proportional to distance)
            let draw_accel = beam_cone.compute_gravity_draw(player_pos);
            movement.velocity += draw_accel * dt;
            movement.is_grounded = false;

            // Horizontal centering towards the central suction axis
            let h_diff = Vec3::new(beam_cone.opening_center.x - player_pos.x, 0.0, beam_cone.opening_center.z - player_pos.z);
            let h_dist = h_diff.length();
            if h_dist > 0.05 {
                movement.velocity.x += (h_diff.x / h_dist) * 6.0 * dt;
                movement.velocity.z += (h_diff.z / h_dist) * 6.0 * dt;
            }
            movement.velocity.x *= (1.0 - 0.35 * dt).max(0.0);
            movement.velocity.z *= (1.0 - 0.35 * dt).max(0.0);

            // Check if player is touching the present despawner
            let dist_to_despawner = player_pos.distance(despawner_pos);
            let h_dist_despawner = (player_pos.x - despawner_pos.x).hypot(player_pos.z - despawner_pos.z);
            if dist_to_despawner < 1.35 || (player_pos.y >= 3.2 && h_dist_despawner < 1.3) {
                player_touching = true;
            }
        }
    }

    if player_touching {
        platform.player_despawner_contact_time += dt;
        if platform.player_despawner_contact_time >= 3.0 {
            // Teleport player on top of the roof of the workshop (apex ridge peak: X = 0.0, Y = 15.0, Z = 0.0)
            for (mut player_tf, mut movement) in &mut q_player {
                player_tf.translation = Vec3::new(0.0, 15.0, 0.0);
                movement.velocity = Vec3::ZERO;
                movement.is_grounded = false;
            }
            platform.player_despawner_contact_time = 0.0;
        }
    } else {
        platform.player_despawner_contact_time = 0.0;
    }
}

/// Advances travelling lumps through the curved pneumatic vacuum tube, dilates tube segments,
/// applies centrifugal/directional reaction impulses, and completes present deliveries at the East wall.
pub fn tube_travelling_lump_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_lumps: Query<(Entity, &mut TubeTravellingLump)>,
    mut q_segments: Query<(&VacuumTubeSegment, &mut Transform)>,
    quota: Option<ResMut<WorkshopQuota>>,
    mut delivery_messages: Option<MessageWriter<PresentDeliveredMessage>>,
    moose_hunger: Option<ResMut<crate::elf_game::moose::MooseHunger>>,
    mut moose_awakened_writer: Option<MessageWriter<crate::elf_game::moose::MooseAwakenedMessage>>,
    mut commands: Commands,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);

    let mut completed_deliveries = 0;

    // Collect active lumps data for tube deformation
    let mut lump_positions = Vec::new();

    for (lump_entity, mut lump) in &mut q_lumps {
        lump.progress += lump.speed * dt;

        if lump.progress >= 1.0 {
            completed_deliveries += 1;
            if let Some(ref mut writer) = delivery_messages {
                writer.write(PresentDeliveredMessage {
                    present_entity: lump_entity,
                    quality: 100.0,
                });
            }
            commands.entity(lump_entity).despawn();
        } else {
            let active_seg_idx = lump.progress * 19.0;
            lump_positions.push((active_seg_idx, lump.size, lump.mass));
        }
    }

    if let Some(mut quota) = quota {
        quota.presents_delivered += completed_deliveries;
    }

    if completed_deliveries > 0 {
        if let Some(mut hunger) = moose_hunger {
            if hunger.is_dormant {
                hunger.is_dormant = false;
                if let Some(ref mut writer) = moose_awakened_writer {
                    writer.write(crate::elf_game::moose::MooseAwakenedMessage);
                }
            }
        }
    }

    // Apply deformation & reaction impulses to tube segments
    for (seg, mut tf) in &mut q_segments {
        let seg_idx = seg.index as f32;
        let mut total_bulge = 0.0;
        let mut total_impulse_offset = 0.0;

        for &(lump_idx, size, mass) in &lump_positions {
            let dist = (seg_idx - lump_idx).abs();
            if dist < 2.0 {
                let bulge_factor = (1.0 - dist / 2.0).max(0.0);
                total_bulge += bulge_factor * size * 0.75;

                // Centrifugal impulse in loop region (segments 4 to 13)
                if seg.index >= 4 && seg.index <= 13 {
                    // Outward centrifugal reaction force: F_c = m * v^2 / R
                    let loop_phase = (seg_idx - 4.0) / 9.0 * std::f32::consts::TAU;
                    total_impulse_offset += loop_phase.sin() * bulge_factor * 0.12 * (mass / 2.0);
                }
            }
        }

        let target_scale = Vec3::new(
            seg.base_scale.x * (1.0 + total_bulge),
            seg.base_scale.y,
            seg.base_scale.z * (1.0 + total_bulge),
        );
        let target_pos = seg.rest_position + Vec3::new(0.0, total_impulse_offset, 0.0);

        // Spring-damper interpolation towards target deformation and rest position
        tf.scale = tf.scale.lerp(target_scale, 10.0 * dt);
        tf.translation = tf.translation.lerp(target_pos, 8.0 * dt);
    }
}
