use bevy::prelude::*;
use crate::elf_game::assets::{palette::*, WorkshopMaterials};

/// Generic Gauge component representing an analog dial instrument.
/// Updates on a given normalized value between 0.0 and 1.0.
#[derive(Component, Debug, Clone)]
pub struct Gauge {
    /// Normalized gauge reading between 0.0 and 1.0.
    pub value: f32,
    /// Threshold at or below which the gauge enters a critical alert state (default: 0.15).
    pub critical_threshold: f32,
    /// Whether the gauge is currently in a critical alert state.
    pub is_critical: bool,
    /// Whether the warning beacon should activate when critical.
    pub has_warning_beacon: bool,
}

impl Default for Gauge {
    fn default() -> Self {
        Self {
            value: 1.0,
            critical_threshold: 0.15,
            is_critical: false,
            has_warning_beacon: true,
        }
    }
}

impl Gauge {
    /// Creates a new Gauge with an initial normalized value (clamped between 0.0 and 1.0).
    pub fn new(value: f32) -> Self {
        let v = value.clamp(0.0, 1.0);
        Self {
            value: v,
            critical_threshold: 0.15,
            is_critical: v <= 0.15,
            has_warning_beacon: true,
        }
    }

    /// Sets the critical threshold for low-value alerts.
    pub fn with_critical_threshold(mut self, threshold: f32) -> Self {
        self.critical_threshold = threshold;
        self.is_critical = self.value <= threshold;
        self
    }

    /// Updates the gauge value (clamped between 0.0 and 1.0) and updates `is_critical`.
    pub fn set_value(&mut self, val: f32) {
        self.value = val.clamp(0.0, 1.0);
        self.is_critical = self.value <= self.critical_threshold;
    }
}

/// Marker component for the workshop Gas Pressure gauge instrument mounted beside the stove.
#[derive(Component, Debug, Clone, Default)]
pub struct GasPressureGauge;

/// Mechanical needle pointer that sweeps across a dial face.
#[derive(Component, Debug, Clone)]
pub struct GaugeNeedle {
    pub current_angle: f32,
    pub target_angle: f32,
    pub min_angle: f32, // -2.356 rad (-135 deg)
    pub max_angle: f32, // +2.356 rad (+135 deg)
    pub is_jittering: bool,
}

impl Default for GaugeNeedle {
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

impl GaugeNeedle {
    pub fn new(min_angle: f32, max_angle: f32) -> Self {
        Self {
            current_angle: min_angle,
            target_angle: min_angle,
            min_angle,
            max_angle,
            is_jittering: false,
        }
    }
}

/// Warning beacon mounted atop a gauge housing.
#[derive(Component, Debug, Clone)]
pub struct GaugeWarningBeacon {
    pub is_alert: bool,
    pub pulse_timer: f32,
    pub base_intensity: f32,
    pub alert_intensity: f32,
}

impl Default for GaugeWarningBeacon {
    fn default() -> Self {
        Self {
            is_alert: false,
            pulse_timer: 0.0,
            base_intensity: 60.0,
            alert_intensity: 450.0,
        }
    }
}

/// Spawns a generic mechanical gauge at a given transform.
pub fn spawn_gauge(
    commands: &mut Commands,
    transform: Transform,
    initial_value: f32,
    critical_threshold: f32,
    meshes: Option<&mut Assets<Mesh>>,
    materials: Option<&mut Assets<StandardMaterial>>,
    workshop_mats: Option<&WorkshopMaterials>,
) -> Entity {
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
    ) = if let Some(wm) = workshop_mats {
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

    let mut gauge_cmd = commands.spawn((
        transform,
        Visibility::default(),
        Gauge::new(initial_value).with_critical_threshold(critical_threshold),
        Name::new("MechanicalGauge"),
    ));

    if has_assets {
        if let (Some(m), Some(iron_mat)) = (&mut meshes, casing_iron_mat.clone()) {
            gauge_cmd.insert((
                Mesh3d(m.add(Cuboid::new(0.08, 0.55, 0.18))), // Wall mounting bracket
                MeshMaterial3d(iron_mat),
            ));
        }
    }

    gauge_cmd.with_children(|gauge| {
        // Main cylindrical iron drum casing
        if let (Some(m), Some(iron_mat)) = (&mut meshes, casing_iron_mat.clone()) {
            gauge.spawn((
                Mesh3d(m.add(Cylinder::new(0.48, 0.16))),
                MeshMaterial3d(iron_mat),
                Transform::from_xyz(0.0, 0.0, 0.08).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("GaugeDrumCasing"),
            ));
        }

        // Brass bezel flange ring
        if let (Some(m), Some(brass_mat)) = (&mut meshes, dial_brass_mat.clone()) {
            gauge.spawn((
                Mesh3d(m.add(Cylinder::new(0.52, 0.04))),
                MeshMaterial3d(brass_mat),
                Transform::from_xyz(0.0, 0.0, 0.15).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("GaugeBrassBezel"),
            ));
        }

        // Ivory enamel dial face plate
        if let (Some(m), Some(face_mat)) = (&mut meshes, face_enamel_mat) {
            gauge.spawn((
                Mesh3d(m.add(Cylinder::new(0.44, 0.02))),
                MeshMaterial3d(face_mat),
                Transform::from_xyz(0.0, 0.0, 0.165).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("GaugeFacePlate"),
            ));
        }

        // Color-coded gauge arc sectors around dial face:
        // Green Safe Arc
        if let (Some(m), Some(mat)) = (&mut meshes, arc_green_mat) {
            gauge.spawn((
                Mesh3d(m.add(Cuboid::new(0.18, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(-0.24, 0.06, 0.18).with_rotation(Quat::from_rotation_z(0.85)),
                Name::new("GaugeArcGreenSafe"),
            ));
        }

        // Yellow Caution Arc
        if let (Some(m), Some(mat)) = (&mut meshes, arc_yellow_mat) {
            gauge.spawn((
                Mesh3d(m.add(Cuboid::new(0.24, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(-0.04, 0.25, 0.18).with_rotation(Quat::from_rotation_z(0.15)),
                Name::new("GaugeArcYellowCaution"),
            ));
        }

        // Orange Warning Arc
        if let (Some(m), Some(mat)) = (&mut meshes, arc_orange_mat) {
            gauge.spawn((
                Mesh3d(m.add(Cuboid::new(0.14, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.20, 0.16, 0.18).with_rotation(Quat::from_rotation_z(-0.75)),
                Name::new("GaugeArcOrangeWarning"),
            ));
        }

        // Red Critical Arc
        if let (Some(m), Some(mat)) = (&mut meshes, arc_red_mat) {
            gauge.spawn((
                Mesh3d(m.add(Cuboid::new(0.10, 0.05, 0.015))),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.26, -0.02, 0.18).with_rotation(Quat::from_rotation_z(-1.25)),
                Name::new("GaugeArcRedCritical"),
            ));
        }

        // Mechanical Dial Needle (Pivoting Root)
        let mut needle_cmd = gauge.spawn((
            Transform::from_xyz(0.0, 0.0, 0.185).with_rotation(Quat::from_rotation_z(2.356)),
            Visibility::default(),
            GaugeNeedle::default(),
            Name::new("GaugeNeedle"),
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
        let mut beacon_cmd = gauge.spawn((
            Transform::from_xyz(0.0, 0.54, 0.08),
            Visibility::default(),
            GaugeWarningBeacon::default(),
            PointLight {
                color: Color::srgb(0.95, 0.70, 0.20),
                intensity: 60.0,
                range: 4.5,
                ..default()
            },
            Name::new("GaugeWarningBeacon"),
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

    gauge_cmd.id()
}

/// Spawns the physical mechanical Gas Pressure Gauge on the North wall beside the stove.
pub fn spawn_gas_pressure_gauge(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let mut meshes = meshes;
    let mut materials = materials;
    let gauge_tf = Transform::from_xyz(-6.6, 1.95, -12.75).with_rotation(Quat::from_rotation_y(0.15));
    let gauge_e = spawn_gauge(
        &mut commands,
        gauge_tf,
        1.0,
        0.15,
        meshes.as_deref_mut(),
        materials.as_deref_mut(),
        workshop_mats.as_deref(),
    );

    commands.entity(gauge_e).insert((
        GasPressureGauge,
        Name::new("GasPressureGauge"),
    ));
}

/// Updates GasPressureGauge instances from the workshop gas line pressure (0..100 PSI -> 0.0..1.0).
pub fn update_gas_pressure_gauge_system(
    gas_line: Option<Res<crate::elf_game::workshop::stove::WorkshopGasLine>>,
    mut q_gauge: Query<&mut Gauge, With<GasPressureGauge>>,
) {
    let Some(gas) = gas_line else { return; };
    for mut gauge in &mut q_gauge {
        let normalized = (gas.pressure_psi / 100.0).clamp(0.0, 1.0);
        gauge.set_value(normalized);
    }
}

/// Animates any generic Gauge object's needle and warning beacon based on its normalized value (0.0..1.0).
pub fn animate_gauge_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    gas_line: Option<Res<crate::elf_game::workshop::stove::WorkshopGasLine>>,
    mut q_gauge: Query<(Entity, &mut Gauge, Option<&GasPressureGauge>)>,
    mut q_needle: Query<(&mut GaugeNeedle, &mut Transform, Option<&ChildOf>)>,
    mut q_beacon: Query<(&mut GaugeWarningBeacon, &mut PointLight, Option<&ChildOf>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let total_time = time.as_ref().map(|t| t.elapsed_secs()).unwrap_or(0.0);

    // If Gas line is present, synchronize any GasPressureGauge instances
    if let Some(ref gas) = gas_line {
        for (_, mut gauge, is_gas_gauge) in &mut q_gauge {
            if is_gas_gauge.is_some() {
                let normalized = (gas.pressure_psi / 100.0).clamp(0.0, 1.0);
                gauge.set_value(normalized);
            }
        }
    }

    // Build read-only lookup of entity -> (value, is_critical)
    let gauge_data: std::collections::HashMap<Entity, (f32, bool)> = q_gauge
        .iter()
        .map(|(e, g, _)| (e, (g.value, g.is_critical)))
        .collect();

    let default_reading = q_gauge
        .iter()
        .next()
        .map(|(_, g, _)| (g.value, g.is_critical))
        .unwrap_or((1.0, false));

    // 1. Update Needle Rotation
    for (mut needle, mut tf, child_of) in &mut q_needle {
        let (val, is_critical) = child_of
            .and_then(|c| gauge_data.get(&c.parent()).copied())
            .unwrap_or(default_reading);

        let progress = val.clamp(0.0, 1.0);
        let base_target = needle.min_angle + progress * (needle.max_angle - needle.min_angle);

        needle.is_jittering = is_critical;

        let jitter = if is_critical {
            (total_time * 26.0).sin() * 0.045
        } else {
            0.0
        };

        needle.target_angle = base_target + jitter;

        let lerp_factor = if time.is_some() {
            (1.0 - (-12.0 * dt).exp()).clamp(0.0, 1.0)
        } else {
            1.0 // In headless unit tests without time, snap immediately to target
        };

        needle.current_angle += (needle.target_angle - needle.current_angle) * lerp_factor;
        tf.rotation = Quat::from_rotation_z(-needle.current_angle);
    }

    // 2. Update Warning Beacon
    for (mut beacon, mut light, child_of) in &mut q_beacon {
        let (_, is_critical) = child_of
            .and_then(|c| gauge_data.get(&c.parent()).copied())
            .unwrap_or(default_reading);

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
}
