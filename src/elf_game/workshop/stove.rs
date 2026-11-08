use bevy::prelude::*;
use bevy::ecs::message::Message;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::WorkshopMaterials,
    player::{
        hands::Grabbable,
        controller::{ElfPlayer, FirstPersonCamera},
        ElfMovement,
    },
    physics_sandbox::{ElfAssistant, ElfNpcState},
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
    /// Rate of fuel consumption when stove/oven is active
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
    /// Connected to the baker's oven
    Oven,
    /// Detached from both anchor points; loose nozzle on the floor venting gas!
    Neither,
}

/// The flexible thin yellow gas supply hose inside the workshop.
#[derive(Component, Debug, Clone)]
pub struct GasHose {
    pub attached_to: HoseAttachment,
    pub wall_inlet_pos: Vec3,
    pub stove_anchor_pos: Vec3,
    pub oven_anchor_pos: Vec3,
    pub loose_nozzle_pos: Vec3,
    pub is_venting_gas: bool,
}

/// Anchor port on the gas-burning furnace stove.
#[derive(Component, Debug, Clone)]
pub struct StoveGasAnchor;

/// Anchor port on the baker's oven.
#[derive(Component, Debug, Clone)]
pub struct OvenGasAnchor;

/// Gas-burning furnace stove heating the workshop and keeping temperatures above freezing.
#[derive(Component, Debug, Clone)]
pub struct GasFurnaceStove {
    pub is_lit: bool,
    pub heat_output: f32,
}

/// Baker's oven for future cookie and pastry baking.
#[derive(Component, Debug, Clone)]
pub struct WorkshopOven {
    pub is_heated: bool,
    pub internal_temp: f32,
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

/// Spawns the Oven, Gas Furnace Stove, Gas Hose, Propane Tank Holder, Propane Tanks,
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

    // Oven brick red/terracotta tone
    let oven_brick_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.28, 0.20),
        perceptual_roughness: 0.85,
        ..default()
    });

    // -------------------------------------------------------------------------
    // 1. Gas-Burning Furnace Stove (Located by the window: X = -9.2, Z = -12.2)
    // -------------------------------------------------------------------------
    let stove_pos = Vec3::new(-9.2, 0.0, -12.2);
    let stove_anchor_pos = Vec3::new(-9.6, 0.65, -12.2);

    commands
        .spawn((
            GasFurnaceStove {
                is_lit: true,
                heat_output: 1.5,
            },
            Transform::from_translation(stove_pos),
            Visibility::default(),
            Collider::cuboid(0.48, 0.48, 0.42),
            RigidBody::Fixed,
            Name::new("GasFurnaceStove"),
        ))
        .with_children(|stove| {
            // Main heavy cast-iron stove body
            stove.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.92, 0.90, 0.80))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(0.0, 0.45, 0.0),
                Name::new("StoveBodyIron"),
            ));

            // Four top cooking burner grates
            let burner_offsets = [
                (-0.24, -0.20),
                (0.24, -0.20),
                (-0.24, 0.20),
                (0.24, 0.20),
            ];
            for (bx, bz) in burner_offsets {
                stove.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.12, 0.04))),
                    MeshMaterial3d(iron_mat.clone()),
                    Transform::from_xyz(bx, 0.92, bz),
                    Name::new("BurnerGrate"),
                ));
            }

            // Furnace firebox inspection window with glowing ember glass
            stove.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.40, 0.28, 0.05))),
                MeshMaterial3d(warning_mat.clone()),
                Transform::from_xyz(0.0, 0.35, 0.41),
                Name::new("FurnaceFireboxWindow"),
            ));

            // Internal firebox light (warm furnace glow)
            stove.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.55, 0.15),
                    intensity: 1200.0,
                    radius: 4.5,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.35, 0.35),
                Name::new("FurnaceFireGlowLight"),
            ));

            // Vertical exhaust flue pipe leading upward
            stove.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.08, 4.2))),
                MeshMaterial3d(dark_mat.clone()),
                Transform::from_xyz(0.0, 3.0, -0.25),
                Name::new("StoveExhaustFluePipe"),
            ));

            // Stove brass gas inlet anchor fitting on West flank
            stove.spawn((
                StoveGasAnchor,
                Mesh3d(meshes.add(Cylinder::new(0.045, 0.12))),
                MeshMaterial3d(brass_mat.clone()),
                Transform::from_xyz(-0.48, 0.65, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                Name::new("StoveGasAnchorFitting"),
            ));
        });

    // -------------------------------------------------------------------------
    // 2. Baker's Oven (Located adjacent to stove: X = -10.9, Z = -12.2)
    // -------------------------------------------------------------------------
    let oven_pos = Vec3::new(-10.9, 0.0, -12.2);
    let oven_anchor_pos = Vec3::new(-10.4, 0.65, -12.2);

    commands
        .spawn((
            WorkshopOven {
                is_heated: false,
                internal_temp: 20.0,
            },
            Transform::from_translation(oven_pos),
            Visibility::default(),
            Collider::cuboid(0.55, 0.55, 0.45),
            RigidBody::Fixed,
            Name::new("WorkshopOven"),
        ))
        .with_children(|oven| {
            // Main rustic masonry oven body
            oven.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.10, 1.05, 0.88))),
                MeshMaterial3d(oven_brick_mat.clone()),
                Transform::from_xyz(0.0, 0.525, 0.0),
                Name::new("OvenBrickBody"),
            ));

            // Heavy cast-iron arched oven door
            oven.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.65, 0.55, 0.06))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(0.0, 0.50, 0.45),
                Name::new("OvenDoor"),
            ));

            // Brass handle on door
            oven.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.025, 0.35))),
                MeshMaterial3d(brass_mat.clone()),
                Transform::from_xyz(0.24, 0.50, 0.50).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                Name::new("OvenDoorHandle"),
            ));

            // Oven brass gas inlet anchor fitting on East flank
            oven.spawn((
                OvenGasAnchor,
                Mesh3d(meshes.add(Cylinder::new(0.045, 0.12))),
                MeshMaterial3d(brass_mat.clone()),
                Transform::from_xyz(0.56, 0.65, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                Name::new("OvenGasAnchorFitting"),
            ));
        });

    // -------------------------------------------------------------------------
    // 3. Wall Gas Supply Pipe & Flexible Thin Yellow Hose
    // -------------------------------------------------------------------------
    let wall_inlet_pos = Vec3::new(-10.0, 0.85, -12.85);
    let loose_nozzle_pos = Vec3::new(-10.0, 0.12, -12.0);

    // Wall intake pipe coming from outside manifold
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.04, 0.45))),
        MeshMaterial3d(iron_mat.clone()),
        Transform::from_xyz(wall_inlet_pos.x, wall_inlet_pos.y, wall_inlet_pos.z)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        Name::new("WallGasIntakePipe"),
    ));

    // Flexible Thin Yellow Hose entity
    commands
        .spawn((
            GasHose {
                attached_to: HoseAttachment::Stove,
                wall_inlet_pos,
                stove_anchor_pos,
                oven_anchor_pos,
                loose_nozzle_pos,
                is_venting_gas: false,
            },
            Transform::from_translation(wall_inlet_pos),
            Visibility::default(),
            Name::new("FlexibleYellowGasHose"),
        ))
        .with_children(|hose| {
            // Visual segments of the yellow hose arched to stove by default
            hose.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.022, 0.95))),
                MeshMaterial3d(yellow_hose_mat.clone()),
                Transform::from_xyz(0.2, -0.1, 0.3).with_rotation(Quat::from_rotation_y(0.6) * Quat::from_rotation_x(0.3)),
                Name::new("YellowHoseSegment1"),
            ));
            hose.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.022, 0.85))),
                MeshMaterial3d(yellow_hose_mat.clone()),
                Transform::from_xyz(0.35, -0.2, 0.55).with_rotation(Quat::from_rotation_y(0.8) * Quat::from_rotation_x(0.4)),
                Name::new("YellowHoseSegment2"),
            ));
            // Brass quick-connect nozzle coupler
            hose.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.038, 0.10))),
                MeshMaterial3d(brass_mat.clone()),
                Transform::from_xyz(0.40, -0.20, 0.65),
                Name::new("YellowHoseCouplerNozzle"),
            ));
        });

    // -------------------------------------------------------------------------
    // 4. Outside Propane Tank Holder (Exterior wall outside window: X = -8.3, Z = -13.55)
    // -------------------------------------------------------------------------
    let holder_pos = Vec3::new(-8.3, 0.45, -13.55);

    // Holder cradle frame mounted to exterior wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.65, 0.85, 0.55))),
        MeshMaterial3d(dark_mat.clone()),
        Transform::from_translation(holder_pos),
        Collider::cuboid(0.32, 0.42, 0.27),
        RigidBody::Fixed,
        PropaneTankHolder {
            connected_tank: None, // Initialized below with the connected tank
        },
        Name::new("PropaneTankHolderCradle"),
    ));

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

    // Link connected tank into holder
    commands.spawn((
        Transform::from_translation(holder_pos),
        PropaneTankHolder {
            connected_tank: Some(connected_tank_e),
        },
        Name::new("PropaneTankHolderManager"),
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
    // 6. Portable Lit Candle Stick on Windowsill (A grabbable FlameSource)
    // -------------------------------------------------------------------------
    commands
        .spawn((
            FlameSource,
            Grabbable,
            Transform::from_xyz(-8.35, 0.95, -12.95),
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
            HoseAttachment::Stove | HoseAttachment::Oven => {
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

/// Allows the player to interact with the flexible thin yellow hose to pull it off an anchor point
/// and attach it to either the Stove, Oven, or leave it detached (Neither).
pub fn gas_hose_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    dialogue_state: Option<Res<crate::elf_game::dialogue::ActiveDialogue>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    mut q_hose: Query<(&mut GasHose, &Children)>,
    mut q_child_tf: Query<&mut Transform, Without<FirstPersonCamera>>,
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

    if !interact_pressed {
        return;
    }

    let Ok(cam_gt) = q_camera.single() else {
        return;
    };
    let cam_pos = cam_gt.translation();
    let cam_fwd = cam_gt.forward();

    for (mut hose, children) in &mut q_hose {
        let stove_target = hose.stove_anchor_pos;
        let oven_target = hose.oven_anchor_pos;
        let loose_target = hose.loose_nozzle_pos;

        let dist_stove = cam_pos.distance(stove_target);
        let dist_oven = cam_pos.distance(oven_target);
        let dist_loose = cam_pos.distance(loose_target);

        let dot_stove = cam_fwd.dot((stove_target - cam_pos).normalize());
        let dot_oven = cam_fwd.dot((oven_target - cam_pos).normalize());
        let dot_loose = cam_fwd.dot((loose_target - cam_pos).normalize());

        if dist_stove < 2.5 && dot_stove > 0.70 {
            // Toggle / attach to Stove
            if hose.attached_to == HoseAttachment::Stove {
                hose.attached_to = HoseAttachment::Neither;
            } else {
                hose.attached_to = HoseAttachment::Stove;
            }
            update_hose_visuals(&hose, children, &mut q_child_tf);
            break;
        } else if dist_oven < 2.5 && dot_oven > 0.70 {
            // Toggle / attach to Oven
            if hose.attached_to == HoseAttachment::Oven {
                hose.attached_to = HoseAttachment::Neither;
            } else {
                hose.attached_to = HoseAttachment::Oven;
            }
            update_hose_visuals(&hose, children, &mut q_child_tf);
            break;
        } else if (dist_loose < 2.5 && dot_loose > 0.65) || (cam_pos.distance(hose.wall_inlet_pos) < 2.5) {
            // Cycle attachment: Neither -> Stove -> Oven -> Neither
            hose.attached_to = match hose.attached_to {
                HoseAttachment::Neither => HoseAttachment::Stove,
                HoseAttachment::Stove => HoseAttachment::Oven,
                HoseAttachment::Oven => HoseAttachment::Neither,
            };
            update_hose_visuals(&hose, children, &mut q_child_tf);
            break;
        }
    }
}

fn update_hose_visuals(
    hose: &GasHose,
    children: &Children,
    q_child_tf: &mut Query<&mut Transform, Without<FirstPersonCamera>>,
) {
    let offset_target = match hose.attached_to {
        HoseAttachment::Stove => Vec3::new(0.40, -0.20, 0.65),
        HoseAttachment::Oven => Vec3::new(-0.40, -0.20, 0.65),
        HoseAttachment::Neither => Vec3::new(0.0, -0.72, 0.85),
    };

    if let Some(&coupler_e) = children.last() {
        if let Ok(mut tf) = q_child_tf.get_mut(coupler_e) {
            tf.translation = offset_target;
        }
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

/// Checks if the gas hose is attached to Neither and gas is flowing. If any FLAME source gets
/// within 1 meter of the loose venting hose nozzle, triggers a catastrophic fiery explosion!
pub fn gas_leak_flame_explosion_system(
    mut commands: Commands,
    gas_line: Option<Res<WorkshopGasLine>>,
    mut explosion_events: MessageWriter<FieryExplosionEvent>,
    q_hose: Query<&GasHose>,
    q_flames: Query<(&GlobalTransform, Option<&CandleParticle>), With<FlameSource>>,
    mut q_player: Query<(&Transform, &mut ElfMovement), With<ElfPlayer>>,
    mut q_elves: Query<(&Transform, &mut Velocity, &mut ElfAssistant), Without<ElfPlayer>>,
    mut q_dynamics: Query<(&Transform, &mut Velocity), (With<Grabbable>, Without<ElfPlayer>, Without<ElfAssistant>)>,
) {
    let gas_pressure = gas_line.as_ref().map(|g| g.pressure_psi).unwrap_or(0.0);
    if gas_pressure <= 0.0 {
        return;
    }

    for hose in &q_hose {
        if hose.attached_to != HoseAttachment::Neither {
            continue;
        }

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
            let blast_radius = 5.5;
            let blast_impulse = 32.0;

            explosion_events.write(FieryExplosionEvent {
                origin: nozzle_pos,
                blast_radius,
                blast_impulse,
            });

            // 1. Blast light flash & explosion shockwave
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.45, 0.08),
                    intensity: 35000.0,
                    radius: 8.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(nozzle_pos + Vec3::Y * 0.5),
                ExplosionFlash { timer: 0.5 },
                Name::new("GasExplosionBlastFlash"),
            ));

            // 2. Push player back with blast velocity
            for (p_tf, mut movement) in &mut q_player {
                let diff = p_tf.translation - nozzle_pos;
                let dist = diff.length().max(0.1);
                if dist < blast_radius {
                    let dir = diff / dist;
                    let force = (1.0 - dist / blast_radius) * blast_impulse;
                    movement.velocity += dir * force + Vec3::Y * 6.0;
                }
            }

            // 3. Knock elves into RagdollTumble
            for (e_tf, mut vel, mut elf) in &mut q_elves {
                let diff = e_tf.translation - nozzle_pos;
                let dist = diff.length().max(0.1);
                if dist < blast_radius {
                    let dir = diff / dist;
                    let force = (1.0 - dist / blast_radius) * blast_impulse;
                    vel.linear += dir * force + Vec3::Y * 5.0;
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
