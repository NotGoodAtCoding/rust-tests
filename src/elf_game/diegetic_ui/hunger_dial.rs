use bevy::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    moose::{MooseHunger, MooseHungerStage, MooseFeedingTrough, MooseSnack},
};

/// Tag component for the physical mechanical hunger gauge housing mounted outside the window
#[derive(Component, Debug, Clone)]
pub struct MooseHungerDial;

/// Type alias repurposing MooseHungerDial as the workshop gas pressure gauge
pub type GasPressureGauge = MooseHungerDial;
/// Type alias repurposing MooseHungerNeedle as the gas pressure needle pointer
pub type GasPressureNeedle = MooseHungerNeedle;

/// Mechanical needle pointer that sweeps across the dial face
#[derive(Component, Debug, Clone)]
pub struct MooseHungerNeedle {
    pub current_angle: f32,
    pub target_angle: f32,
    pub min_angle: f32, // -2.356 rad (-135 deg)
    pub max_angle: f32, // +2.356 rad (+135 deg)
    pub is_jittering: bool,
}

impl Default for MooseHungerNeedle {
    fn default() -> Self {
        Self {
            current_angle: -2.356,
            target_angle: -2.356,
            min_angle: -2.356,
            max_angle: 2.356,
            is_jittering: false,
        }
    }
}

/// Warning beacon mounted atop the hunger dial housing
#[derive(Component, Debug, Clone)]
pub struct MooseHungerWarningBeacon {
    pub is_alert: bool,
    pub pulse_timer: f32,
    pub base_intensity: f32,
    pub alert_intensity: f32,
}

impl Default for MooseHungerWarningBeacon {
    fn default() -> Self {
        Self {
            is_alert: false,
            pulse_timer: 0.0,
            base_intensity: 60.0,
            alert_intensity: 450.0,
        }
    }
}

/// Physical scale base platform installed under the moose feeding trough
#[derive(Component, Debug, Clone)]
pub struct MooseTroughScalePlate {
    pub rest_y: f32,
    pub current_deflection: f32,
}

impl Default for MooseTroughScalePlate {
    fn default() -> Self {
        Self {
            rest_y: 0.08,
            current_deflection: 0.0,
        }
    }
}

/// Spawns the physical mechanical hunger dial outside the window and the scale under the trough
pub fn spawn_moose_hunger_dial_and_scale(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let has_assets = meshes.is_some() && (materials.is_some() || workshop_mats.is_some());
    let mut meshes = meshes;
    let mut materials = materials;

    let (
        casing_iron_mat,
        dial_brass_mat,
        face_enamel_mat,
        needle_red_mat,
        beacon_glass_mat,
        arc_green_mat,
        arc_yellow_mat,
        arc_orange_mat,
        arc_red_mat,
    ) = if let Some(ref wm) = workshop_mats {
        (
            Some(wm.cast_iron.clone()),
            Some(wm.polished_brass.clone()),
            Some(wm.ivory_enamel.clone()),
            Some(wm.gauge_red.clone()),
            Some(wm.warning_lantern.clone()),
            Some(wm.gauge_green.clone()),
            Some(wm.gauge_yellow.clone()),
            Some(wm.gauge_orange.clone()),
            Some(wm.gauge_red.clone()),
        )
    } else if let Some(ref mut mats) = materials {
        (
            Some(mats.add(StandardMaterial {
                base_color: CAST_IRON,
                metallic: 0.85,
                perceptual_roughness: 0.45,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: POLISHED_BRASS,
                metallic: 0.80,
                perceptual_roughness: 0.30,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: IVORY_ENAMEL,
                perceptual_roughness: 0.35,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: GAUGE_RED,
                metallic: 0.30,
                perceptual_roughness: 0.40,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: WARNING_LANTERN_RUBY,
                emissive: LinearRgba::rgb(0.8, 0.15, 0.05),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: GAUGE_GREEN,
                perceptual_roughness: 0.50,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: GAUGE_YELLOW,
                perceptual_roughness: 0.50,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: GAUGE_ORANGE,
                perceptual_roughness: 0.50,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: GAUGE_RED,
                perceptual_roughness: 0.50,
                ..default()
            })),
        )
    } else {
        (None, None, None, None, None, None, None, None, None)
    };

    // 1. Physical Mechanical Gas Pressure Gauge / Hunger Dial mounted by the stove & window
    // Mounted prominently on the North wall by the furnace stove and climbable window bay at X = -6.6, Y = 1.95, Z = -12.75
    // Angled slightly (+0.15 rad) so it is clearly readable from the stove, oven, and workbench
    let mut dial_cmd = commands.spawn((
        Transform::from_xyz(-6.6, 1.95, -12.75).with_rotation(Quat::from_rotation_y(0.15)),
        Visibility::default(),
        MooseHungerDial,
        Name::new("MooseHungerMechanicalDial"),
    ));

    if has_assets {
        if let (Some(m), Some(iron_mat)) = (&mut meshes, casing_iron_mat.clone()) {
            dial_cmd.insert((
                Mesh3d(m.add(Cuboid::new(0.08, 0.55, 0.18))), // Wall mounting bracket
                MeshMaterial3d(iron_mat),
            ));
        }
    }

    dial_cmd.with_children(|dial| {
        // Main cylindrical iron drum casing
        if let (Some(m), Some(iron_mat)) = (&mut meshes, casing_iron_mat.clone()) {
            dial.spawn((
                Mesh3d(m.add(Cylinder::new(0.48, 0.16))),
                MeshMaterial3d(iron_mat),
                Transform::from_xyz(0.0, 0.0, 0.08).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("HungerDialDrumCasing"),
            ));
        }

        // Brass bezel flange ring
        if let (Some(m), Some(brass_mat)) = (&mut meshes, dial_brass_mat.clone()) {
            dial.spawn((
                Mesh3d(m.add(Cylinder::new(0.52, 0.04))),
                MeshMaterial3d(brass_mat),
                Transform::from_xyz(0.0, 0.0, 0.15).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("HungerDialBrassBezel"),
            ));
        }

        // Ivory enamel dial face plate
        if let (Some(m), Some(face_mat)) = (&mut meshes, face_enamel_mat) {
            dial.spawn((
                Mesh3d(m.add(Cylinder::new(0.44, 0.02))),
                MeshMaterial3d(face_mat),
                Transform::from_xyz(0.0, 0.0, 0.165).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("HungerDialFacePlate"),
            ));
        }

        // Color-coded gauge arc sectors around dial face:
        // Green Safe Arc (0..20)
        if let (Some(m), Some(mat)) = (&mut meshes, arc_green_mat) {
            dial.spawn((
                Mesh3d(m.add(Cuboid::new(0.18, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(-0.24, 0.06, 0.18).with_rotation(Quat::from_rotation_z(0.85)),
                Name::new("GaugeArcGreenSafe"),
            ));
        }

        // Yellow Caution Arc (20..50)
        if let (Some(m), Some(mat)) = (&mut meshes, arc_yellow_mat) {
            dial.spawn((
                Mesh3d(m.add(Cuboid::new(0.24, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(-0.04, 0.25, 0.18).with_rotation(Quat::from_rotation_z(0.15)),
                Name::new("GaugeArcYellowCaution"),
            ));
        }

        // Orange Agitation Arc (50..58)
        if let (Some(m), Some(mat)) = (&mut meshes, arc_orange_mat) {
            dial.spawn((
                Mesh3d(m.add(Cuboid::new(0.14, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.20, 0.16, 0.18).with_rotation(Quat::from_rotation_z(-0.75)),
                Name::new("GaugeArcOrangeWarning"),
            ));
        }

        // Red Breach Danger Arc (58..60)
        if let (Some(m), Some(mat)) = (&mut meshes, arc_red_mat) {
            dial.spawn((
                Mesh3d(m.add(Cuboid::new(0.10, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.26, -0.02, 0.18).with_rotation(Quat::from_rotation_z(-1.25)),
                Name::new("GaugeArcRedBreach"),
            ));
        }

        // Mechanical Dial Needle (Pivoting Root)
        let mut needle_cmd = dial.spawn((
            Transform::from_xyz(0.0, 0.0, 0.185).with_rotation(Quat::from_rotation_z(2.356)), // Initialized to far-left min angle
            Visibility::default(),
            MooseHungerNeedle::default(),
            Name::new("MooseHungerNeedle"),
        ));

        needle_cmd.with_children(|needle| {
            // Center brass spindle hub
            if let (Some(m), Some(brass_mat)) = (&mut meshes, dial_brass_mat.clone()) {
                needle.spawn((
                    Mesh3d(m.add(Cylinder::new(0.05, 0.03))),
                    MeshMaterial3d(brass_mat),
                    Transform::from_xyz(0.0, 0.0, 0.01).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    Name::new("NeedleSpindleHub"),
                ));
            }

            // Tapered pointer blade
            if let (Some(m), Some(red_mat)) = (&mut meshes, needle_red_mat.clone()) {
                needle.spawn((
                    Mesh3d(m.add(Cuboid::new(0.022, 0.26, 0.012))),
                    MeshMaterial3d(red_mat),
                    Transform::from_xyz(0.0, 0.13, 0.01),
                    Name::new("NeedleBlade"),
                ));
            }

            // High-visibility red arrow pointer tip
            if let (Some(m), Some(red_mat)) = (&mut meshes, needle_red_mat) {
                needle.spawn((
                    Mesh3d(m.add(Cone { radius: 0.038, height: 0.065 })),
                    MeshMaterial3d(red_mat),
                    Transform::from_xyz(0.0, 0.28, 0.01),
                    Name::new("NeedleArrowTip"),
                ));
            }

            // Brass counterweight on tail
            if let (Some(m), Some(brass_mat)) = (&mut meshes, dial_brass_mat.clone()) {
                needle.spawn((
                    Mesh3d(m.add(Cylinder::new(0.032, 0.02))),
                    MeshMaterial3d(brass_mat),
                    Transform::from_xyz(0.0, -0.07, 0.01).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    Name::new("NeedleCounterweight"),
                ));
            }
        });

        // Pulsating Warning Beacon atop the dial casing
        let mut beacon_cmd = dial.spawn((
            Transform::from_xyz(0.0, 0.54, 0.08),
            Visibility::default(),
            MooseHungerWarningBeacon::default(),
            PointLight {
                color: Color::srgb(0.95, 0.70, 0.20),
                intensity: 60.0,
                range: 4.5,
                ..default()
            },
            Name::new("MooseHungerWarningBeacon"),
        ));

        beacon_cmd.with_children(|beacon| {
            // Brass beacon base ring
            if let (Some(m), Some(brass_mat)) = (&mut meshes, dial_brass_mat) {
                beacon.spawn((
                    Mesh3d(m.add(Cylinder::new(0.12, 0.06))),
                    MeshMaterial3d(brass_mat),
                    Transform::from_xyz(0.0, -0.03, 0.0),
                    Name::new("BeaconBrassBase"),
                ));
            }

            // Translucent glass dome
            if let (Some(m), Some(glass_mat)) = (&mut meshes, beacon_glass_mat) {
                beacon.spawn((
                    Mesh3d(m.add(Sphere::new(0.08))),
                    MeshMaterial3d(glass_mat),
                    Transform::from_xyz(0.0, 0.05, 0.0),
                    Name::new("BeaconGlassDome"),
                ));
            }
        });
    });

    // 2. Physical Scale Base Platform under the Moose Feeding Trough
    // Positioned at X = -2.0, Y = 0.08, Z = -12.0 (directly beneath trough at Y = 0.3)
    let mut scale_cmd = commands.spawn((
        Transform::from_xyz(-2.0, 0.08, -12.0),
        Visibility::default(),
        MooseTroughScalePlate::default(),
        Name::new("MooseTroughScalePlatform"),
    ));

    if has_assets {
        if let (Some(m), Some(iron_mat)) = (&mut meshes, casing_iron_mat) {
            scale_cmd.insert((
                Mesh3d(m.add(Cuboid::new(2.2, 0.08, 1.4))), // Heavy cast-iron scale base
                MeshMaterial3d(iron_mat),
            ));
        }
    }
}

/// Animates the physical mechanical pressure / hunger needle, warning beacon, and trough scale plate
pub fn animate_moose_hunger_dial_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    gas_line: Option<Res<crate::elf_game::workshop::stove::WorkshopGasLine>>,
    hunger: Option<Res<MooseHunger>>,
    q_snacks: Query<&Transform, (With<MooseSnack>, Without<MooseHungerNeedle>, Without<MooseTroughScalePlate>)>,
    q_trough: Query<&Transform, (With<MooseFeedingTrough>, Without<MooseHungerNeedle>, Without<MooseTroughScalePlate>)>,
    mut q_needle: Query<(&mut MooseHungerNeedle, &mut Transform), (Without<MooseTroughScalePlate>, Without<MooseSnack>, Without<MooseFeedingTrough>)>,
    mut q_beacon: Query<(&mut MooseHungerWarningBeacon, &mut PointLight)>,
    mut q_scale: Query<(&mut MooseTroughScalePlate, &mut Transform), (Without<MooseHungerNeedle>, Without<MooseSnack>, Without<MooseFeedingTrough>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let total_time = time.as_ref().map(|t| t.elapsed_secs()).unwrap_or(0.0);

    let (progress, is_critical, stage_opt) = if let Some(ref gas) = gas_line {
        // Repurposed as Gas Pressure Gauge: tracks connected tank fuel pressure (0.0 to 100.0 PSI)
        let prog = (gas.pressure_psi / 100.0).clamp(0.0, 1.0);
        let crit = gas.pressure_psi < 15.0;
        (prog, crit, None)
    } else {
        let (hunger_val, hunger_max, stage) = if let Some(h) = hunger {
            (h.value, h.max_value.max(1.0), h.stage)
        } else {
            (0.0, 60.0, MooseHungerStage::BehindShutters)
        };
        let prog = (hunger_val / hunger_max).clamp(0.0, 1.0);
        let crit = hunger_val >= 50.0
            || stage == MooseHungerStage::AgitatedHuffing
            || stage == MooseHungerStage::WindowSlammed
            || stage == MooseHungerStage::BreachedRampage;
        (prog, crit, Some(stage))
    };

    // 1. Update Needle Rotation
    for (mut needle, mut tf) in &mut q_needle {
        // Map 0.0..1.0 progress to angle between min_angle and max_angle
        let base_target = needle.min_angle + progress * (needle.max_angle - needle.min_angle);

        // Agitation jitter when critical / low pressure or AgitatedHuffing
        let is_agitating = is_critical || stage_opt == Some(MooseHungerStage::AgitatedHuffing);
        needle.is_jittering = is_agitating;

        let jitter = if is_agitating {
            (total_time * 26.0).sin() * 0.045
        } else {
            0.0
        };

        needle.target_angle = base_target + jitter;

        // Smooth mechanical spring-damped tracking
        let lerp_factor = if time.is_some() {
            (1.0 - (-12.0 * dt).exp()).clamp(0.0, 1.0)
        } else {
            1.0 // In headless unit tests without time, snap immediately to target
        };

        needle.current_angle += (needle.target_angle - needle.current_angle) * lerp_factor;

        // Rotating around local -Z swings pointer clockwise from left to right
        tf.rotation = Quat::from_rotation_z(-needle.current_angle);
    }

    // 2. Update Warning Beacon
    for (mut beacon, mut light) in &mut q_beacon {
        beacon.is_alert = is_critical;

        if is_critical {
            beacon.pulse_timer += dt * 7.0;
            let pulse = (beacon.pulse_timer.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            light.color = Color::srgb(1.0, 0.12 + 0.20 * pulse, 0.05);
            light.intensity = beacon.base_intensity + pulse * (beacon.alert_intensity - beacon.base_intensity);
        } else {
            beacon.pulse_timer = 0.0;
            light.color = Color::srgb(0.95, 0.70, 0.20);
            light.intensity = beacon.base_intensity;
        }
    }

    // 3. Update Feeding Trough Scale Plate
    let Ok(trough_tf) = q_trough.single() else {
        return;
    };

    // Count snacks currently resting in the trough
    let mut snack_count = 0;
    for s_tf in &q_snacks {
        if (s_tf.translation.x - trough_tf.translation.x).abs() < 1.1
            && (s_tf.translation.z - trough_tf.translation.z).abs() < 0.8
            && s_tf.translation.y < 1.4
        {
            snack_count += 1;
        }
    }

    for (mut scale, mut tf) in &mut q_scale {
        let target_deflection = (snack_count as f32 * 0.015).min(0.04);
        let lerp_rate = if time.is_some() {
            (1.0 - (-15.0 * dt).exp()).clamp(0.0, 1.0)
        } else {
            1.0
        };
        scale.current_deflection += (target_deflection - scale.current_deflection) * lerp_rate;
        tf.translation.y = scale.rest_y - scale.current_deflection;
    }
}
