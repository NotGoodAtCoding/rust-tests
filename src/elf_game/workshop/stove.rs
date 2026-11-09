use bevy::prelude::*;
use bevy::ecs::message::Message;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::WorkshopMaterials, physics_sandbox::{ElfAssistant, ElfNpcState}, player::{
        ElfMovement, controller::{ElfPlayer, FirstPersonCamera}, hands::Grabbable,
    },
};

/// Thermal state of Santa's Workshop. The player must keep the furnace stove fueled and burning
/// or the temperature drops below freezing (0.0 C), freezing the workshop and halting present spawns.
#[derive(Resource, Debug, Clone)]
pub struct WorkshopTemperature {
    /// Current workshop ambient temperature in degrees Celsius
    pub current: f32,
    /// Target temperature when heating is active
    pub target: f32,
    /// Rate at which cold Arctic air cools the workshop (deg C / sec)
    pub cooling_rate: f32,
    /// Rate at which the burning stove warms the workshop (deg C / sec)
    pub heating_rate: f32,
    /// Whether the workshop is currently below freezing (<= 0.0 C)
    pub is_frozen: bool,
}

impl Default for WorkshopTemperature {
    fn default() -> Self {
        Self {
            current: 20.0,
            target: 22.0,
            cooling_rate: 0.35,
            heating_rate: 1.5,
            is_frozen: false,
        }
    }
}

/// Global gas pressure line supplied by the connected propane tank.
#[derive(Resource, Debug, Clone)]
pub struct WorkshopGasLine {
    /// Active gas line pressure in PSI (0.0 to 100.0)
    pub pressure_psi: f32,
    /// Rate of fuel consumption when stove is active
    pub burn_rate: f32,
    /// Rapid venting rate when hose is detached to Neither
    pub leak_vent_rate: f32,
}

impl Default for WorkshopGasLine {
    fn default() -> Self {
        Self {
            pressure_psi: 100.0,
            burn_rate: 0.8,
            leak_vent_rate: 2.5,
        }
    }
}

/// Attachment destinations for the flexible yellow gas hose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoseAttachment {
    /// Connected to the cast-iron furnace stove
    Stove,
    /// Detached from the anchor point; loose nozzle on the floor venting gas!
    Neither,
}

/// The flexible thin yellow gas supply hose inside the workshop,
/// modeled as a dynamic chain of links (similar to the Moose scarf).
#[derive(Component, Debug, Clone)]
pub struct GasHose {
    pub attached_to: HoseAttachment,
    pub wall_inlet_pos: Vec3,
    pub stove_anchor_pos: Vec3,
    pub loose_nozzle_pos: Vec3,
    pub is_venting_gas: bool,
    pub snap_distance: f32,
    pub nozzle_entity: Option<Entity>,
    /// Accumulated time in seconds that the hose has been disconnected and venting gas (maxed at 30.0s).
    pub disconnected_timer: f32,
}

impl GasHose {
    /// Computes (blast_impulse, blast_radius) based on accumulated disconnected_timer in seconds.
    /// - 0 to 5 seconds: returns None (no explosion).
    /// - 5 to 30 seconds (or more, maxed at 30 seconds):
    ///   - blast_radius: scales linearly from 0.0 to 4.0 meters.
    ///   - blast_impulse: scales linearly from 5.0 to 30.0 (maxed at 30, the same as the timer).
    pub fn explosion_scaling(timer: f32) -> Option<(f32, f32)> {
        if timer <= 5.0 {
            return None;
        }
        let t_effective = timer.min(30.0);
        let factor = (t_effective - 5.0) / 25.0;
        let blast_radius = factor * 4.0;
        let blast_impulse = 5.0 + factor * 25.0; // identically equal to t_effective
        Some((blast_impulse, blast_radius))
    }
}

impl Default for GasHose {
    fn default() -> Self {
        Self {
            attached_to: HoseAttachment::Stove,
            wall_inlet_pos: Vec3::new(-9.05, 1.15, -12.85),
            stove_anchor_pos: Vec3::new(-7.34, 0.85, -11.4),
            loose_nozzle_pos: Vec3::new(-9.05, 0.05, -11.4),
            is_venting_gas: false,
            snap_distance: 0.85,
            nozzle_entity: None,
            disconnected_timer: 0.0,
        }
    }
}

/// Individual link segment along the flexible yellow gas hose chain.
#[derive(Component, Debug, Clone)]
pub struct GasHoseSegment {
    pub index: usize,
    pub velocity: Vec3,
}

/// The brass quick-connect coupler nozzle on the free end of the flexible yellow gas hose.
/// Can be grabbed and carried by the player, snapping into place when within reach of an anchor.
#[derive(Component, Debug, Clone)]
pub struct GasHoseCouplerNozzle;

/// Anchor port on the gas-burning furnace stove.
#[derive(Component, Debug, Clone)]
pub struct StoveGasAnchor;

/// Firebox inspection window on the furnace stove. Glows if gas hose is attached, cool dark gray if not.
#[derive(Component, Debug, Clone)]
pub struct FurnaceFireboxWindow;

/// Internal firebox glow light for the furnace stove.
#[derive(Component, Debug, Clone)]
pub struct FurnaceFireGlowLight;

/// Blue flame jet effect on the stove cooking burners.
#[derive(Component, Debug, Clone)]
pub struct StoveBurnerFlame;

/// Shared material handles for appliance visual indicators (glowing ember vs cool dark gray vs blue flame).
#[derive(Resource, Clone)]
pub struct ApplianceVisualMaterials {
    pub glowing_ember: Handle<StandardMaterial>,
    pub cool_dark_gray: Handle<StandardMaterial>,
    pub blue_flame: Handle<StandardMaterial>,
}

/// Gas-burning furnace stove heating the workshop, keeping temperatures above freezing,
/// and providing baking/cooking capabilities.
#[derive(Component, Debug, Clone)]
pub struct GasFurnaceStove {
    pub is_lit: bool,
    pub heat_output: f32,
    pub is_heated: bool,
    pub internal_temp: f32,
}

impl Default for GasFurnaceStove {
    fn default() -> Self {
        Self {
            is_lit: true,
            heat_output: 1.5,
            is_heated: true,
            internal_temp: 20.0,
        }
    }
}

/// Portable propane gas cylinder used to supply fuel to the workshop.
#[derive(Component, Debug, Clone)]
pub struct PropaneTank {
    pub fuel_amount: f32,
    pub capacity: f32,
    pub is_connected: bool,
}

impl Default for PropaneTank {
    fn default() -> Self {
        Self {
            fuel_amount: 100.0,
            capacity: 100.0,
            is_connected: false,
        }
    }
}

/// Outside propane tank manifold/cradle mounted on the exterior wall.
#[derive(Component, Debug, Clone)]
pub struct PropaneTankHolder {
    pub connected_tank: Option<Entity>,
}

/// Main bulk propane tank located a distance away in the snowy outdoor landscape.
#[derive(Component, Debug, Clone)]
pub struct MainPropaneRefillStation;

/// Identifies an open flame source (e.g. candle flame tip, torch) capable of igniting gas leaks.
#[derive(Component, Debug, Clone)]
pub struct FlameSource;

/// Event emitted when venting gas meets an open flame source within 1 meter.
#[derive(Message, Debug, Clone)]
pub struct FieryExplosionEvent {
    pub origin: Vec3,
    pub blast_radius: f32,
    pub blast_impulse: f32,
}

/// Short-lived visual flash light spawned during a gas explosion.
#[derive(Component, Debug, Clone)]
pub struct ExplosionFlash {
    pub timer: f32,
}

/// Spawns the Gas Furnace Stove, Gas Hose, Propane Tank Holder, Propane Tanks,
/// and the outdoor Bulk Propane Refill Tank.
pub fn spawn_oven_stove_and_propane_scene(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let wm = match workshop_mats {
        Some(mats) => mats.clone(),
        None => WorkshopMaterials::new(&mut materials),
    };

    let iron_mat = wm.cast_iron;
    let brass_mat = wm.polished_brass;
    let _leather_mat = wm.walnut_leather;
    let snow_mat = wm.snow_white;
    let dark_mat = wm.industrial_dark;
    let warning_mat = wm.warning_lantern;

    // Safety yellow material for flexible gas hose
    let yellow_hose_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.82, 0.10),
        perceptual_roughness: 0.5,
        metallic: 0.1,
        ..default()
    });

    // Stove gas anchor fitting red material
    let red_anchor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.12, 0.14),
        metallic: 0.35,
        perceptual_roughness: 0.4,
        ..default()
    });

    // Propane tank blue/steel material
    let propane_blue_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.42, 0.75),
        metallic: 0.65,
        perceptual_roughness: 0.35,
        ..default()
    });

    // Bulk propane tank white industrial coating
    let bulk_tank_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.94, 0.95),
        metallic: 0.4,
        perceptual_roughness: 0.3,
        ..default()
    });

    // Appliance visual indicator materials: glowing ember vs cool dark gray vs blue flame
    let glowing_ember_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.20),
        emissive: LinearRgba::new(4.5, 2.2, 0.4, 1.0),
        perceptual_roughness: 0.2,
        ..default()
    });
    let cool_dark_gray_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.20, 0.22),
        emissive: LinearRgba::BLACK,
        perceptual_roughness: 0.8,
        ..default()
    });
    let blue_flame_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.65, 1.0, 0.85),
        emissive: LinearRgba::new(0.5, 2.5, 7.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });

    commands.insert_resource(ApplianceVisualMaterials {
        glowing_ember: glowing_ember_mat.clone(),
        cool_dark_gray: cool_dark_gray_mat.clone(),
        blue_flame: blue_flame_mat.clone(),
    });

    // -------------------------------------------------------------------------
    // 1. Gas-Burning Furnace Stove (Moved further away: X = -6.5, Z = -11.4)
    // -------------------------------------------------------------------------
    let stove_pos = Vec3::new(-6.5, 0.0, -11.4);
    let stove_anchor_pos = Vec3::new(-7.34, 0.85, -11.4);

    commands
        .spawn((
            GasFurnaceStove {
                is_lit: true,
                heat_output: 1.5,
                is_heated: true,
                internal_temp: 20.0,
            },
            Transform::from_translation(stove_pos),
            Visibility::default(),
            Collider::cuboid(0.80, 0.675, 0.60),
            RigidBody::Fixed,
            Name::new("GasFurnaceStove"),
        ))
        .with_children(|stove| {
            // Main heavy cast-iron stove body (Width: 1.6m, Height: 1.35m, Depth: 1.2m)
            stove.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.60, 1.35, 1.20))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(0.0, 0.675, 0.0),
                Name::new("StoveBodyIron"),
            ));

            // Four top cooking burner grates with blue flame effects
            let burner_offsets = [
                (-0.40, -0.32),
                (0.40, -0.32),
                (-0.40, 0.32),
                (0.40, 0.32),
            ];
            for (bx, bz) in burner_offsets {
                stove.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.18, 0.05))),
                    MeshMaterial3d(iron_mat.clone()),
                    Transform::from_xyz(bx, 1.38, bz),
                    Name::new("BurnerGrate"),
                ));

                // Blue flame effect jet atop each burner grate
                stove.spawn((
                    StoveBurnerFlame,
                    Mesh3d(meshes.add(Cylinder::new(0.13, 0.035))),
                    MeshMaterial3d(blue_flame_mat.clone()),
                    Transform::from_xyz(bx, 1.41, bz),
                    Visibility::Inherited,
                    Name::new("StoveBurnerBlueFlame"),
                ));
            }

            // Furnace firebox inspection window with glowing ember glass
            stove.spawn((
                FurnaceFireboxWindow,
                Mesh3d(meshes.add(Cuboid::new(0.65, 0.45, 0.08))),
                MeshMaterial3d(glowing_ember_mat.clone()),
                Transform::from_xyz(0.0, 0.50, 0.61),
                Name::new("FurnaceFireboxWindow"),
            ));

            // Internal firebox light (warm furnace glow)
            stove.spawn((
                FurnaceFireGlowLight,
                PointLight {
                    color: Color::srgb(1.0, 0.55, 0.15),
                    intensity: 1600.0,
                    radius: 6.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.50, 0.50),
                Name::new("FurnaceFireGlowLight"),
            ));

            // Vertical exhaust flue pipe leading upward
            stove.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.12, 4.0))),
                MeshMaterial3d(dark_mat.clone()),
                Transform::from_xyz(0.0, 3.35, -0.35),
                Name::new("StoveExhaustFluePipe"),
            ));

            // Stove red gas inlet anchor fitting on West flank with compact active area
            stove.spawn((
                StoveGasAnchor,
                crate::elf_game::assets::objects::AnchorPoint::new("gas_hose").with_radius(2.0),
                Mesh3d(meshes.add(Cylinder::new(0.08, 0.22))),
                MeshMaterial3d(red_anchor_mat.clone()),
                Transform::from_xyz(-0.84, 0.85, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                Collider::cylinder(0.18, 0.22),
                Sensor,
                Grabbable,
                Name::new("StoveGasAnchorFitting"),
            ));
        });

    // -------------------------------------------------------------------------
    // 2. Wall Gas Supply Pipe & Flexible Chain Yellow Hose (Rapier RopeJoint Physics)
    // -------------------------------------------------------------------------
    let wall_inlet_pos = Vec3::new(-9.05, 1.15, -12.85);

    // Permanent wall intake pipe fitting as Fixed Rapier RigidBody
    let wall_anchor_e = commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.045, 0.45))),
            MeshMaterial3d(iron_mat.clone()),
            Transform::from_xyz(wall_inlet_pos.x, wall_inlet_pos.y, wall_inlet_pos.z)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            RigidBody::Fixed,
            Collider::cylinder(0.225, 0.045),
            Name::new("WallGasIntakePipe"),
        ))
        .id();

    // Flexible yellow chain hose: 7 links (0..6)
    // Links 0..5 are intermediate dynamic segments connected by Rapier RopeJoints
    // Link 6 is the coupler nozzle (grabbable by player, snaps to anchor within snap_distance)
    let mut prev_joint_body = wall_anchor_e;

    for i in 0..6 {
        let t = (i + 1) as f32 / 7.0;
        let rest_pos = wall_inlet_pos.lerp(stove_anchor_pos, t)
            + Vec3::new(0.0, -0.15 * (1.0 - (t - 0.5).abs() * 2.0), 0.1 * t);

        let rope_joint = RopeJointBuilder::new(0.38).contacts_enabled(false);

        let link_e = commands
            .spawn((
                GasHoseSegment {
                    index: i,
                    velocity: Vec3::ZERO,
                },
                Transform::from_translation(rest_pos),
                RigidBody::Dynamic,
                Collider::ball(0.04),
                Damping {
                    linear_damping: 1.5,
                    angular_damping: 1.5,
                },
                Velocity::zero(),
                ImpulseJoint::new(prev_joint_body, rope_joint),
                Mesh3d(meshes.add(Cylinder::new(0.026, 0.38))),
                MeshMaterial3d(yellow_hose_mat.clone()),
                Name::new(format!("YellowHoseChainLink_{}", i)),
            ))
            .id();
        prev_joint_body = link_e;
    }

    // Coupler Nozzle (Link 6) connected to Link 5 via Rapier RopeJoint
    let nozzle_rope_joint = RopeJointBuilder::new(0.38).contacts_enabled(false);

    let nozzle_e = commands
        .spawn((
            GasHoseSegment {
                index: 6,
                velocity: Vec3::ZERO,
            },
            GasHoseCouplerNozzle,
            crate::elf_game::assets::objects::AnchorTrait::new("gas_hose"),
            Transform::from_translation(stove_anchor_pos),
            RigidBody::Fixed, // Starts securely docked to Stove
            Collider::cylinder(0.08, 0.05),
            Damping {
                linear_damping: 1.5,
                angular_damping: 1.5,
            },
            Velocity::zero(),
            ImpulseJoint::new(prev_joint_body, nozzle_rope_joint),
            Grabbable,
            Mesh3d(meshes.add(Cylinder::new(0.045, 0.12))),
            MeshMaterial3d(brass_mat.clone()),
            Name::new("YellowHoseCouplerNozzle"),
        ))
        .id();

    // Flexible Yellow Hose Manager entity
    commands.spawn((
        GasHose {
            attached_to: HoseAttachment::Stove,
            wall_inlet_pos,
            stove_anchor_pos,
            loose_nozzle_pos: stove_anchor_pos,
            is_venting_gas: false,
            snap_distance: 0.85,
            nozzle_entity: Some(nozzle_e),
            disconnected_timer: 0.0,
        },
        Transform::from_translation(wall_inlet_pos),
        Visibility::default(),
        Name::new("FlexibleYellowGasHose"),
    ));

    // -------------------------------------------------------------------------
    // 4. Outside Propane Tank Holder (Exterior wall outside window: X = -8.3, Z = -13.55)
    // -------------------------------------------------------------------------
    let holder_pos = Vec3::new(-8.3, 0.45, -13.55);

    // Active Connected Propane Tank seated inside the holder
    let connected_tank_e = commands
        .spawn((
            PropaneTank {
                fuel_amount: 100.0,
                capacity: 100.0,
                is_connected: true,
            },
            Transform::from_xyz(holder_pos.x, holder_pos.y + 0.15, holder_pos.z + 0.05),
            Mesh3d(meshes.add(Cylinder::new(0.20, 0.65))),
            MeshMaterial3d(propane_blue_mat.clone()),
            Collider::cylinder(0.325, 0.20),
            RigidBody::Dynamic,
            Grabbable,
            Name::new("ActivePropaneTank"),
        ))
        .id();

    // Holder cradle frame mounted to exterior wall with active tank connected
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.65, 0.85, 0.55))),
        MeshMaterial3d(dark_mat.clone()),
        Transform::from_translation(holder_pos),
        Collider::cuboid(0.32, 0.42, 0.27),
        RigidBody::Fixed,
        PropaneTankHolder {
            connected_tank: Some(connected_tank_e),
        },
        Name::new("PropaneTankHolderCradle"),
    ));

    // Spare portable propane tank resting in the snow outside
    commands.spawn((
        PropaneTank {
            fuel_amount: 100.0,
            capacity: 100.0,
            is_connected: false,
        },
        Transform::from_xyz(-10.2, 0.05, -15.2),
        Mesh3d(meshes.add(Cylinder::new(0.20, 0.65))),
        MeshMaterial3d(propane_blue_mat.clone()),
        Collider::cylinder(0.325, 0.20),
        RigidBody::Dynamic,
        Grabbable,
        Name::new("SparePropaneTank"),
    ));

    // -------------------------------------------------------------------------
    // 5. Main Bulk Propane Tank (Outside in snow yard a distance away: X = -16.0, Z = -26.0)
    // -------------------------------------------------------------------------
    let bulk_pos = Vec3::new(-16.0, 0.2, -26.0);

    commands
        .spawn((
            MainPropaneRefillStation,
            Transform::from_translation(bulk_pos),
            Visibility::default(),
            Collider::cylinder(2.1, 1.15),
            RigidBody::Fixed,
            Name::new("MainBulkPropaneTankStation"),
        ))
        .with_children(|bulk| {
            // Large horizontal white cylindrical tank
            bulk.spawn((
                Mesh3d(meshes.add(Cylinder::new(1.15, 4.2))),
                MeshMaterial3d(bulk_tank_mat.clone()),
                Transform::from_xyz(0.0, 1.25, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                Name::new("BulkTankCylinder"),
            ));

            // Hemispherical endcaps
            bulk.spawn((
                Mesh3d(meshes.add(Sphere::new(1.15))),
                MeshMaterial3d(bulk_tank_mat.clone()),
                Transform::from_xyz(-2.1, 1.25, 0.0),
                Name::new("BulkTankEndcapWest"),
            ));
            bulk.spawn((
                Mesh3d(meshes.add(Sphere::new(1.15))),
                MeshMaterial3d(bulk_tank_mat.clone()),
                Transform::from_xyz(2.1, 1.25, 0.0),
                Name::new("BulkTankEndcapEast"),
            ));

            // Concrete mounting piers
            bulk.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.50, 0.80, 2.5))),
                MeshMaterial3d(snow_mat.clone()),
                Transform::from_xyz(-1.2, 0.35, 0.0),
                Name::new("ConcretePierWest"),
            ));
            bulk.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.50, 0.80, 2.5))),
                MeshMaterial3d(snow_mat.clone()),
                Transform::from_xyz(1.2, 0.35, 0.0),
                Name::new("ConcretePierEast"),
            ));

            // Refill pump dispenser pedestal & brass handwheel valve
            bulk.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.40, 1.10, 0.40))),
                MeshMaterial3d(dark_mat.clone()),
                Transform::from_xyz(0.0, 0.55, 1.45),
                Name::new("RefillPumpDispenserPedestal"),
            ));
            bulk.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.12, 0.04))),
                MeshMaterial3d(warning_mat.clone()),
                Transform::from_xyz(0.0, 1.12, 1.45),
                Name::new("RefillShutoffValveWheel"),
            ));
        });

    // -------------------------------------------------------------------------
    // 6. Portable Lit Candle Stick on Workshop Bench (A grabbable FlameSource)
    // -------------------------------------------------------------------------
    commands
        .spawn((
            FlameSource,
            Grabbable,
            Transform::from_xyz(0.60, 0.95, -4.0),
            Collider::cylinder(0.15, 0.08),
            RigidBody::Dynamic,
            Name::new("PortableBrassCandlestick"),
        ))
        .with_children(|candle| {
            // Brass candlestick base & holder
            candle.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.08, 0.05))),
                MeshMaterial3d(brass_mat.clone()),
                Transform::from_xyz(0.0, 0.025, 0.0),
                Name::new("CandlestickBase"),
            ));
            // Wax candle stick
            candle.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.03, 0.20))),
                MeshMaterial3d(wm.candle_wax.clone()),
                Transform::from_xyz(0.0, 0.14, 0.0),
                Name::new("CandleWax"),
            ));
            // Flame cone
            candle.spawn((
                Mesh3d(meshes.add(Cone { radius: 0.022, height: 0.05 })),
                MeshMaterial3d(wm.candle_flame.clone()),
                Transform::from_xyz(0.0, 0.26, 0.0),
                Name::new("CandleFlame"),
            ));
            // Candle light point
            candle.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.72, 0.35),
                    intensity: 150.0,
                    radius: 2.5,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.28, 0.0),
                Name::new("CandleFlameLight"),
            ));
        });
}

pub use spawn_oven_stove_and_propane_scene as spawn_stove_and_propane_scene;

/// Updates the workshop temperature based on active heating (stove burning gas) vs ambient Arctic cooling.
/// If temperature drops to or below 0.0 C, marks `is_frozen = true` which halts present spawning.
pub fn workshop_temperature_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    temperature: Option<ResMut<WorkshopTemperature>>,
    gas_line: Option<Res<WorkshopGasLine>>,
    q_stove: Query<&GasFurnaceStove>,
    q_hose: Query<&GasHose>,
) {
    let (Some(time), Some(mut temp)) = (time, temperature) else {
        return;
    };
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    let gas_pressure = gas_line.as_ref().map(|g| g.pressure_psi).unwrap_or(0.0);
    let hose_attached_to_stove = q_hose.iter().any(|h| h.attached_to == HoseAttachment::Stove);
    let stove_lit = q_stove.iter().any(|s| s.is_lit);

    let is_heating = hose_attached_to_stove && stove_lit && gas_pressure > 0.0;

    if is_heating {
        // Stove heats workshop up towards target temperature
        if temp.current < temp.target {
            temp.current = (temp.current + temp.heating_rate * dt).min(temp.target);
        }
    } else {
        // Outside Arctic winter cools the workshop down towards freezing
        temp.current -= temp.cooling_rate * dt;
    }

    temp.is_frozen = temp.current <= 0.0;
}

/// Manages propane fuel consumption, gas line pressure, and updates the connected tank.
pub fn propane_gas_flow_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    gas_line: Option<ResMut<WorkshopGasLine>>,
    mut q_tanks: Query<&mut PropaneTank>,
    q_holders: Query<&PropaneTankHolder>,
    mut q_hose: Query<&mut GasHose>,
) {
    let (Some(time), Some(mut line)) = (time, gas_line) else {
        return;
    };
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.delta_secs();

    // Find the currently active tank entity in the holder
    let mut active_tank_e: Option<Entity> = None;
    for holder in &q_holders {
        if let Some(tank_e) = holder.connected_tank {
            active_tank_e = Some(tank_e);
            break;
        }
    }

    let Some(tank_e) = active_tank_e else {
        line.pressure_psi = 0.0;
        for mut hose in &mut q_hose {
            hose.is_venting_gas = false;
        }
        return;
    };

    let Ok(mut tank) = q_tanks.get_mut(tank_e) else {
        line.pressure_psi = 0.0;
        for mut hose in &mut q_hose {
            hose.is_venting_gas = false;
        }
        return;
    };

    for mut hose in &mut q_hose {
        match hose.attached_to {
            HoseAttachment::Stove => {
                hose.is_venting_gas = false;
                if tank.fuel_amount > 0.0 && dt > 0.0 {
                    tank.fuel_amount = (tank.fuel_amount - line.burn_rate * dt).max(0.0);
                }
            }
            HoseAttachment::Neither => {
                // Open nozzle! Gas is venting into the workshop room!
                if tank.fuel_amount > 0.0 {
                    hose.is_venting_gas = true;
                    if dt > 0.0 {
                        tank.fuel_amount = (tank.fuel_amount - line.leak_vent_rate * dt).max(0.0);
                    }
                } else {
                    hose.is_venting_gas = false;
                }
            }
        }
    }

    line.pressure_psi = tank.fuel_amount;
}

/// Manages the flexible yellow gas supply hose Rapier physics and anchor docking.
/// The hose physics is simulated by the Rapier 3D physics plugin using RopeJoints connecting
/// rigid bodies between the wall intake and the coupler nozzle.
/// This system updates nozzle attachment docking, player grab transitions, and synchronizes
/// segment state with Rapier.
pub fn gas_hose_chain_physics_system(
    _time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut commands: Commands,
    mut q_hands: Query<&mut crate::elf_game::player::ElfHands>,
    q_held: Query<Entity, With<crate::elf_game::player::hands::HeldByPlayer>>,
    mut q_hose: Query<&mut GasHose>,
    mut q_segments: Query<
        (
            Entity,
            &mut GasHoseSegment,
            &mut Transform,
            Option<&Velocity>,
            Option<&mut RigidBody>,
        ),
        (Without<FirstPersonCamera>, Without<crate::elf_game::player::ElfHands>),
    >,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Ok(mut hose) = q_hose.single_mut() else {
        return;
    };

    let wall_pos = hose.wall_inlet_pos;
    let stove_anchor = hose.stove_anchor_pos;
    let snap_dist = hose.snap_distance;

    let is_nozzle_held = if let Some(nozzle_e) = hose.nozzle_entity {
        q_held.get(nozzle_e).is_ok()
    } else {
        false
    };

    // Locate nozzle entity and transform
    let mut nozzle_entity_opt = hose.nozzle_entity;
    let mut nozzle_pos = match hose.attached_to {
        HoseAttachment::Stove => stove_anchor,
        HoseAttachment::Neither => hose.loose_nozzle_pos,
    };

    for (e, seg, tf, _, _) in &q_segments {
        if seg.index == 6 || Some(e) == nozzle_entity_opt {
            nozzle_pos = tf.translation;
            if nozzle_entity_opt.is_none() {
                nozzle_entity_opt = Some(e);
            }
        }
    }

    // 1. Coupler Nozzle Snapping and Attachment Transitions
    if is_nozzle_held {
        let dist_to_stove = nozzle_pos.distance(stove_anchor);

        if dist_to_stove <= snap_dist {
            // Snap to Stove!
            hose.attached_to = HoseAttachment::Stove;
            hose.is_venting_gas = false;
            nozzle_pos = stove_anchor;
            if let Some(nozzle_e) = nozzle_entity_opt {
                commands.entity(nozzle_e).remove::<(crate::elf_game::player::hands::HeldByPlayer, Sensor)>();
                for mut hands in &mut q_hands {
                    if hands.left_grabbed == Some(nozzle_e) {
                        hands.left_grabbed = None;
                    }
                    if hands.right_grabbed == Some(nozzle_e) {
                        hands.right_grabbed = None;
                    }
                }
            }
        } else {
            // Held and carried in open space -> detached to Neither
            hose.attached_to = HoseAttachment::Neither;
            hose.is_venting_gas = true;
            hose.loose_nozzle_pos = nozzle_pos;
        }
    } else {
        match hose.attached_to {
            HoseAttachment::Stove => {
                nozzle_pos = stove_anchor;
            }
            HoseAttachment::Neither => {
                hose.loose_nozzle_pos = nozzle_pos;
            }
        }
    }

    // 2. Synchronize Rapier RigidBody states & segment velocities
    let mut seg_translations: [Option<Vec3>; 7] = [None; 7];
    for (_, seg, tf, _, _) in &q_segments {
        if seg.index < 7 {
            seg_translations[seg.index] = Some(tf.translation);
        }
    }

    for (e, mut seg, mut tf, rapier_vel, mut rb_opt) in &mut q_segments {
        let i = seg.index;

        // Synchronize linear velocity from Rapier physics
        if let Some(vel) = rapier_vel {
            seg.velocity = vel.linear;
        }

        if i == 6 || Some(e) == nozzle_entity_opt {
            // Coupler nozzle rigid body & position docking
            match hose.attached_to {
                HoseAttachment::Stove => {
                    tf.translation = stove_anchor;
                    if let Some(ref mut rb) = rb_opt {
                        **rb = RigidBody::Fixed;
                    }
                }
                HoseAttachment::Neither => {
                    if !is_nozzle_held {
                        if let Some(ref mut rb) = rb_opt {
                            **rb = RigidBody::Dynamic;
                        }
                    }
                    hose.loose_nozzle_pos = tf.translation;
                }
            }
        } else if i < 6 {
            // Intermediate segment links: ensure dynamic rigid body when physics runs
            if let Some(ref mut rb) = rb_opt {
                if **rb != RigidBody::Dynamic {
                    **rb = RigidBody::Dynamic;
                }
            }

            // Align segment visual mesh rotation along chain tangent
            let p_prev = if i > 0 {
                seg_translations[i - 1].unwrap_or(wall_pos)
            } else {
                wall_pos
            };
            let p_next = if i < 5 {
                seg_translations[i + 1].unwrap_or(nozzle_pos)
            } else {
                nozzle_pos
            };
            let tangent = p_next - p_prev;
            if tangent.length_squared() > 0.0001 {
                tf.rotation = Quat::from_rotation_arc(Vec3::Y, tangent.normalize());
            }
        }
    }
}

/// Allows the player to interact with the flexible yellow hose coupler nozzle or anchor fittings
/// to pull it off an anchor point, cycle its attachment, or automatically attach to whichever
/// anchor point the player is facing while holding the hose.
pub fn gas_hose_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    dialogue_state: Option<Res<crate::elf_game::dialogue::ActiveDialogue>>,
    mut commands: Commands,
    mut q_hands: Query<&mut crate::elf_game::player::ElfHands>,
    q_held: Query<Entity, With<crate::elf_game::player::hands::HeldByPlayer>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    q_stove_anchor: Query<Entity, With<StoveGasAnchor>>,
    mut q_hose: Query<&mut GasHose>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    if dialogue_state.as_ref().map(|d| d.is_active).unwrap_or(false) {
        return;
    }

    let Ok(mut hose) = q_hose.single_mut() else {
        return;
    };

    let is_holding_nozzle = if let Some(nozzle_e) = hose.nozzle_entity {
        q_held.get(nozzle_e).is_ok()
            || q_hands.iter().any(|h| h.left_grabbed == Some(nozzle_e) || h.right_grabbed == Some(nozzle_e))
    } else {
        false
    };

    let stove_anchor_e = q_stove_anchor.iter().next();

    // Check if player grabbed the stove anchor point directly while holding the nozzle
    if is_holding_nozzle {
        let mut grabbed_stove = false;
        for hands in &q_hands {
            if let Some(left) = hands.left_grabbed {
                if Some(left) == stove_anchor_e { grabbed_stove = true; }
            }
            if let Some(right) = hands.right_grabbed {
                if Some(right) == stove_anchor_e { grabbed_stove = true; }
            }
        }

        if grabbed_stove {
            hose.attached_to = HoseAttachment::Stove;
            hose.is_venting_gas = false;
            hose.loose_nozzle_pos = hose.stove_anchor_pos;
            if let Some(nozzle_e) = hose.nozzle_entity {
                commands.entity(nozzle_e).remove::<(crate::elf_game::player::hands::HeldByPlayer, Sensor)>();
            }
            for mut hands in &mut q_hands {
                if hands.left_grabbed == hose.nozzle_entity || hands.left_grabbed == stove_anchor_e {
                    hands.left_grabbed = None;
                }
                if hands.right_grabbed == hose.nozzle_entity || hands.right_grabbed == stove_anchor_e {
                    hands.right_grabbed = None;
                }
            }
            return;
        }
    }

    let Some(keys) = keys else { return; };
    let Some(mouse) = mouse else { return; };

    let interact_pressed = keys.just_pressed(KeyCode::KeyE)
        || keys.just_pressed(KeyCode::KeyQ)
        || keys.just_pressed(KeyCode::KeyF)
        || mouse.just_pressed(MouseButton::Left);

    if !interact_pressed {
        return;
    }

    let Ok(cam_gt) = q_camera.single() else {
        return;
    };
    let cam_pos = cam_gt.translation();
    let cam_fwd = cam_gt.forward();

    let stove_target = hose.stove_anchor_pos;
    let loose_target = hose.loose_nozzle_pos;

    let dist_stove = cam_pos.distance(stove_target);
    let dist_loose = cam_pos.distance(loose_target);

    let dot_stove = cam_fwd.dot((stove_target - cam_pos).normalize());
    let dot_loose = cam_fwd.dot((loose_target - cam_pos).normalize());

    // When the player interacts or grabs while holding the hose:
    // Automatically attach to the stove anchor point if player is facing it!
    if is_holding_nozzle {
        if dot_stove > 0.40 && dist_stove < 3.8 {
            hose.attached_to = HoseAttachment::Stove;
            hose.is_venting_gas = false;
            hose.loose_nozzle_pos = stove_target;
            if let Some(nozzle_e) = hose.nozzle_entity {
                commands.entity(nozzle_e).remove::<(crate::elf_game::player::hands::HeldByPlayer, Sensor)>();
            }
            for mut hands in &mut q_hands {
                if hands.left_grabbed == hose.nozzle_entity {
                    hands.left_grabbed = None;
                }
                if hands.right_grabbed == hose.nozzle_entity {
                    hands.right_grabbed = None;
                }
            }
            return;
        }
    }

    // Standard interaction when not holding the nozzle:
    if dist_stove < 3.2 && dot_stove > 0.55 {
        // Toggle / attach to Stove
        if hose.attached_to == HoseAttachment::Stove {
            hose.attached_to = HoseAttachment::Neither;
        } else {
            hose.attached_to = HoseAttachment::Stove;
        }
    } else if (dist_loose < 3.0 && dot_loose > 0.55) || (cam_pos.distance(hose.wall_inlet_pos) < 3.0) {
        // Cycle attachment: Neither <-> Stove
        hose.attached_to = match hose.attached_to {
            HoseAttachment::Neither => HoseAttachment::Stove,
            HoseAttachment::Stove => HoseAttachment::Neither,
        };
    }
}

/// Updates visual indicators for the furnace and stove depending on gas hose connection and gas flow:
/// - Furnace door window glows if attached, cool dark gray if not.
/// - Stove burners show blue flame effects if connected, off if not.
pub fn appliance_visual_indicators_system(
    time: Option<Res<Time>>,
    gas_line: Option<Res<WorkshopGasLine>>,
    q_hose: Query<&GasHose>,
    appliance_mats: Option<Res<ApplianceVisualMaterials>>,
    mut q_burner_flames: Query<&mut Visibility, With<StoveBurnerFlame>>,
    mut q_furnace_window: Query<&mut MeshMaterial3d<StandardMaterial>, With<FurnaceFireboxWindow>>,
    mut q_furnace_light: Query<&mut PointLight, With<FurnaceFireGlowLight>>,
) {
    let Ok(hose) = q_hose.single() else {
        return;
    };

    let has_gas = gas_line.as_ref().map(|g| g.pressure_psi > 0.0).unwrap_or(true);
    let stove_active = hose.attached_to == HoseAttachment::Stove && has_gas;

    let Some(mats) = appliance_mats else {
        return;
    };

    // 1. Stove Burners: Blue flame effects if connected, off if not
    for mut vis in &mut q_burner_flames {
        *vis = if stove_active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // 2. Furnace Firebox Window: Glows if attached, cool dark gray if not
    for mut mat in &mut q_furnace_window {
        *mat = if stove_active {
            MeshMaterial3d(mats.glowing_ember.clone())
        } else {
            MeshMaterial3d(mats.cool_dark_gray.clone())
        };
    }

    // Furnace interior glow light
    let flicker = time.as_ref().map(|t| (t.elapsed_secs() * 12.0).sin() * 80.0).unwrap_or(0.0);
    for mut light in &mut q_furnace_light {
        light.intensity = if stove_active {
            1600.0 + flicker
        } else {
            0.0
        };
    }
}

/// Allows the player to replace propane tanks in the outside holder and refill them at the main bulk tank.
pub fn propane_tank_refill_and_holder_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    dialogue_state: Option<Res<crate::elf_game::dialogue::ActiveDialogue>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    mut q_holders: Query<(Entity, &Transform, &mut PropaneTankHolder)>,
    mut q_tanks: Query<(Entity, &mut Transform, &mut PropaneTank), (Without<FirstPersonCamera>, Without<PropaneTankHolder>, Without<MainPropaneRefillStation>)>,
    q_refill_stations: Query<&Transform, (With<MainPropaneRefillStation>, Without<PropaneTank>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    if dialogue_state.as_ref().map(|d| d.is_active).unwrap_or(false) {
        return;
    }

    let Some(keys) = keys else { return; };
    let Some(mouse) = mouse else { return; };

    let interact_pressed = keys.just_pressed(KeyCode::KeyE)
        || keys.just_pressed(KeyCode::KeyQ)
        || keys.just_pressed(KeyCode::KeyF)
        || mouse.just_pressed(MouseButton::Left);

    let Ok(cam_gt) = q_camera.single() else {
        return;
    };
    let cam_pos = cam_gt.translation();
    let cam_fwd = cam_gt.forward();

    // 1. Refill at Main Bulk Propane Tank (Outside a distance away: X = -16.0, Z = -26.0)
    for station_tf in &q_refill_stations {
        let station_pos = station_tf.translation;
        if cam_pos.distance(station_pos) < 4.0 && interact_pressed {
            // Find any propane tank near the player/station and refill it!
            for (_, _, mut tank) in &mut q_tanks {
                if tank.fuel_amount < tank.capacity {
                    tank.fuel_amount = tank.capacity;
                }
            }
        }
    }

    // 2. Propane Tank Holder Replace / Insert / Remove
    if interact_pressed {
        for (_, holder_tf, mut holder) in &mut q_holders {
            let holder_pos = holder_tf.translation;
            if cam_pos.distance(holder_pos) < 2.5 && cam_fwd.dot((holder_pos - cam_pos).normalize()) > 0.60 {
                // Check if player is targeting a nearby tank to insert, or removing the current one
                if let Some(curr_tank_e) = holder.connected_tank {
                    // Remove tank from holder
                    holder.connected_tank = None;
                    if let Ok((_, _, mut tank)) = q_tanks.get_mut(curr_tank_e) {
                        tank.is_connected = false;
                    }
                } else {
                    // Find closest available tank near holder and mount it
                    let mut closest_tank: Option<(Entity, f32)> = None;
                    for (tank_e, tank_tf, _) in &q_tanks {
                        let d = tank_tf.translation.distance(holder_pos);
                        if d < 2.2 && (closest_tank.is_none() || d < closest_tank.unwrap().1) {
                            closest_tank = Some((tank_e, d));
                        }
                    }
                    if let Some((tank_e, _)) = closest_tank {
                        holder.connected_tank = Some(tank_e);
                        if let Ok((_, mut tank_tf, mut tank)) = q_tanks.get_mut(tank_e) {
                            tank.is_connected = true;
                            tank_tf.translation = Vec3::new(holder_pos.x, holder_pos.y + 0.15, holder_pos.z + 0.05);
                        }
                    }
                }
            }
        }
    }
}

/// Checks if the gas hose is attached to Neither and gas is flowing.
/// Scales the impulse and blast radius with the amount of time that the hose has been disconnected:
/// - From 0-5 seconds: no explosion.
/// - 5 seconds to 30 seconds or more (maxed at 30 seconds):
///   - Blast radius scales linearly from 0 to 4 meters.
///   - Impulse scales linearly from 5 to 30 (maxed at 30, the same as the timer).
/// - If the hose is reconnected, ticks the counter back down (does not reset to 0 immediately).
pub fn gas_leak_flame_explosion_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut commands: Commands,
    gas_line: Option<Res<WorkshopGasLine>>,
    mut explosion_events: MessageWriter<FieryExplosionEvent>,
    mut q_hose: Query<&mut GasHose>,
    q_flames: Query<(&GlobalTransform, Option<&CandleParticle>), With<FlameSource>>,
    mut q_player: Query<(&Transform, &mut ElfMovement), With<ElfPlayer>>,
    mut q_elves: Query<(&Transform, &mut Velocity, &mut ElfAssistant), Without<ElfPlayer>>,
    mut q_dynamics: Query<(&Transform, &mut Velocity), (With<Grabbable>, Without<ElfPlayer>, Without<ElfAssistant>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let gas_pressure = gas_line.as_ref().map(|g| g.pressure_psi).unwrap_or(100.0);

    for mut hose in &mut q_hose {
        // Disconnected timer tracking:
        if hose.attached_to == HoseAttachment::Neither {
            if gas_pressure > 0.0 && dt > 0.0 {
                // Disconnected and venting: tick counter up, maxed at 30 seconds
                hose.disconnected_timer = (hose.disconnected_timer + dt).min(30.0);
            } else if dt > 0.0 {
                // Disconnected but no gas pressure: leaked gas dissipates over time
                hose.disconnected_timer = (hose.disconnected_timer - dt).max(0.0);
            }
        } else {
            // If the hose is reconnected, tick the counter back down, don't reset to 0 immediately
            if dt > 0.0 {
                hose.disconnected_timer = (hose.disconnected_timer - dt).max(0.0);
            }
        }

        // Only an actively venting disconnected hose can ignite
        if hose.attached_to != HoseAttachment::Neither || gas_pressure <= 0.0 {
            continue;
        }

        // 0-5 seconds: no explosion.
        // 5-30+ seconds: impulse 5 to 30, blast radius 0 to 4 meters.
        let Some((blast_impulse, blast_radius)) = GasHose::explosion_scaling(hose.disconnected_timer) else {
            continue;
        };

        let nozzle_pos = hose.loose_nozzle_pos;

        // Check distance to all FlameSource entities
        let mut flame_nearby = false;
        for (flame_gt, _) in &q_flames {
            let flame_pos = flame_gt.translation();
            if flame_pos.distance(nozzle_pos) < 1.0 {
                flame_nearby = true;
                break;
            }
        }

        if flame_nearby {
            // BOOM! Fiery gas explosion!
            explosion_events.write(FieryExplosionEvent {
                origin: nozzle_pos,
                blast_radius,
                blast_impulse,
            });

            // 1. Blast light flash & explosion shockwave
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.45, 0.08),
                    intensity: (35000.0 * (blast_impulse / 30.0)).max(5000.0),
                    radius: (blast_radius * 1.5).max(2.0),
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(nozzle_pos + Vec3::Y * 0.5),
                ExplosionFlash { timer: 0.5 },
                Name::new("GasExplosionBlastFlash"),
            ));

            // 2. Push player back with blast velocity
            if blast_radius > 0.0 {
                for (p_tf, mut movement) in &mut q_player {
                    let diff = p_tf.translation - nozzle_pos;
                    let dist = diff.length().max(0.1);
                    if dist < blast_radius {
                        let dir = diff / dist;
                        let force = (1.0 - dist / blast_radius) * blast_impulse;
                        movement.velocity += dir * force + Vec3::Y * (blast_impulse * 0.2).min(6.0);
                    }
                }

                // 3. Knock elves into RagdollTumble
                for (e_tf, mut vel, mut elf) in &mut q_elves {
                    let diff = e_tf.translation - nozzle_pos;
                    let dist = diff.length().max(0.1);
                    if dist < blast_radius {
                        let dir = diff / dist;
                        let force = (1.0 - dist / blast_radius) * blast_impulse;
                        vel.linear += dir * force + Vec3::Y * (blast_impulse * 0.16).min(5.0);
                        vel.angular = Vec3::new(8.0, 4.0, -6.0);
                        elf.state = ElfNpcState::RagdollTumble;
                        elf.tumble_timer = 0.0;
                    }
                }

                // 4. Scatter dynamic physics props
                for (d_tf, mut vel) in &mut q_dynamics {
                    let diff = d_tf.translation - nozzle_pos;
                    let dist = diff.length().max(0.1);
                    if dist < blast_radius {
                        let dir = diff / dist;
                        let force = (1.0 - dist / blast_radius) * (blast_impulse * 1.2);
                        vel.linear += dir * force + Vec3::Y * 4.0;
                    }
                }
            }

            // Gas burned off in explosion: reset disconnected_timer so gas must accumulate again
            hose.disconnected_timer = 0.0;
            break;
        }
    }
}

/// Fades out the explosion light flash over time.
pub fn explosion_flash_fade_system(
    time: Option<Res<Time>>,
    mut commands: Commands,
    mut q_flashes: Query<(Entity, &mut ExplosionFlash, &mut PointLight)>,
) {
    let Some(time) = time else { return; };
    let dt = time.delta_secs();

    for (entity, mut flash, mut light) in &mut q_flashes {
        flash.timer -= dt;
        if flash.timer <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            light.intensity = (flash.timer / 0.5) * 35000.0;
        }
    }
}

// Particle helper for candle emitter import
pub use crate::elf_game::workshop::room::CandleParticle;
