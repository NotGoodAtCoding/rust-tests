use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader};
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    physics_sandbox::ragdoll::{ElfAssistant, ElfNpcState},
    player::{ElfMovement, controller::ElfPlayer, hands::Grabbable},
    wrapping::Present,
};

/// Trigger catapult message to manually or diegetically launch the catapult
#[derive(Message, Debug, Clone)]
pub struct TriggerCatapultMessage;

/// Toggle message for the conveyor assembly line diverter paddle
#[derive(Message, Debug, Clone)]
pub struct ToggleDiverterMessage;

/// State of the mechanical conveyor catapult flipper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatapultState {
    #[default]
    Ready,
    Launching,
    Recoil,
    Cooldown,
}

/// Mechanical conveyor catapult that launches presents and ragdolls across the workshop
#[derive(Component, Debug, Clone)]
pub struct ConveyorCatapult {
    pub state: CatapultState,
    pub launch_velocity: Vec3,
    pub arm_angle: f32,
    pub target_angle: f32,
    pub state_timer: f32,
    pub cooldown_duration: f32,
    pub auto_trigger: bool,
    pub trigger_radius: f32,
}

impl Default for ConveyorCatapult {
    fn default() -> Self {
        Self {
            state: CatapultState::Ready,
            launch_velocity: Vec3::new(-10.0, 9.5, -6.5),
            arm_angle: 0.0,
            target_angle: 0.0,
            state_timer: 0.0,
            cooldown_duration: 1.5,
            auto_trigger: true,
            trigger_radius: 0.75,
        }
    }
}

/// Tag component for the manual interactive trigger lever next to the catapult
#[derive(Component, Debug, Clone)]
pub struct CatapultLever {
    pub catapult_entity: Entity,
}

/// Mechanical diverter paddle that redirects presents from the main conveyor onto alternate assembly lines
#[derive(Component, Debug, Clone)]
pub struct AssemblyLineDiverter {
    pub is_diverting: bool,
    pub paddle_angle: f32,
    pub target_angle: f32,
    pub divert_direction: Vec3,
    pub divert_speed: f32,
}

impl Default for AssemblyLineDiverter {
    fn default() -> Self {
        Self {
            is_diverting: false,
            paddle_angle: 0.0,
            target_angle: 0.0,
            divert_direction: Vec3::new(-0.35, 0.0, -0.93).normalize(),
            divert_speed: 2.2,
        }
    }
}

/// Tag component for the manual interactive lever that toggles the diverter paddle
#[derive(Component, Debug, Clone)]
pub struct DiverterLever {
    pub diverter_entity: Entity,
}

/// Sloped gravity slide ramp guiding diverted presents down toward the wrapping workbench
#[derive(Component, Debug, Clone)]
pub struct AssemblyLineRamp {
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub slide_speed: f32,
}

impl Default for AssemblyLineRamp {
    fn default() -> Self {
        Self {
            start_pos: Vec3::new(3.0, 0.85, 2.8),
            end_pos: Vec3::new(1.5, 0.55, -2.0),
            slide_speed: 2.5,
        }
    }
}

/// Spawns the conveyor catapult, assembly line diverter, slide ramp, and interactive levers
pub fn spawn_catapult_and_assembly_line_scene(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let brass_mat = workshop_mats.as_ref().map(|wm| wm.polished_brass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: POLISHED_BRASS,
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        })
    });

    let steel_mat = workshop_mats.as_ref().map(|wm| wm.sheet_metal.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SHEET_METAL,
            metallic: 0.9,
            perceptual_roughness: 0.2,
            ..default()
        })
    });

    let wood_mat = workshop_mats.as_ref().map(|wm| wm.timber_beam.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: TIMBER_BEAM,
            perceptual_roughness: 0.85,
            ..default()
        })
    });

    let red_mat = workshop_mats.as_ref().map(|wm| wm.festive_red.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: FESTIVE_RED,
            perceptual_roughness: 0.4,
            ..default()
        })
    });

    // 1. Assembly Line Diverter Paddle on Main Conveyor (at X = 3.0, Y = 0.85, Z = 3.0)
    let diverter_entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.08, 0.25, 0.85))),
            MeshMaterial3d(steel_mat.clone()),
            Transform::from_xyz(3.0, 0.88, 3.0),
            AssemblyLineDiverter::default(),
            Name::new("ConveyorDiverterPaddle"),
        ))
        .id();

    // Diverter Lever Pedestal (at X = 3.0, Y = 0.6, Z = 2.0)
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.08, 0.6))),
        MeshMaterial3d(brass_mat.clone()),
        Transform::from_xyz(3.0, 0.3, 2.0),
        Collider::cylinder(0.3, 0.08),
        DiverterLever { diverter_entity },
        Name::new("DiverterLeverPedestal"),
    ));

    // 2. Assembly Line Gravity Slide Ramp (connecting conveyor diverter to wrapping workbench)
    let ramp_start = Vec3::new(3.0, 0.85, 2.8);
    let ramp_end = Vec3::new(1.5, 0.55, -2.0);
    let ramp_vec = ramp_end - ramp_start;
    let ramp_mid = (ramp_start + ramp_end) * 0.5;
    let ramp_len = ramp_vec.length();
    let ramp_rot = Quat::from_rotation_arc(Vec3::Z, ramp_vec.normalize());

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.65, 0.06, ramp_len))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_translation(ramp_mid).with_rotation(ramp_rot),
        Collider::cuboid(0.32, 0.03, ramp_len * 0.5),
        AssemblyLineRamp {
            start_pos: ramp_start,
            end_pos: ramp_end,
            slide_speed: 2.5,
        },
        Name::new("AssemblyLineGravityRamp"),
    ));

    // 3. Mechanical Conveyor Catapult (at X = 6.5, Y = 0.85, Z = 3.0)
    let catapult_entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.65, 0.12, 0.65))),
            MeshMaterial3d(red_mat.clone()),
            Transform::from_xyz(6.5, 0.88, 3.0),
            ConveyorCatapult::default(),
            Name::new("ConveyorCatapultFlipper"),
        ))
        .id();

    // Catapult Lever Pedestal (at X = 6.5, Y = 0.6, Z = 2.0)
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.08, 0.6))),
        MeshMaterial3d(brass_mat.clone()),
        Transform::from_xyz(6.5, 0.3, 2.0),
        Collider::cylinder(0.3, 0.08),
        CatapultLever { catapult_entity },
        Name::new("CatapultLeverPedestal"),
    ));
}

/// Diverter paddle physics and animation: rotates paddle and deflects cubes onto the slide ramp
pub fn conveyor_diverter_paddle_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_diverters: Query<(&mut Transform, &mut AssemblyLineDiverter), Without<Present>>,
    mut q_cubes: Query<(&Transform, &mut Velocity), (With<Present>, With<Grabbable>, Without<AssemblyLineDiverter>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    for (mut tf, mut diverter) in &mut q_diverters {
        diverter.target_angle = if diverter.is_diverting { 0.785 } else { 0.0 };
        diverter.paddle_angle = diverter.paddle_angle.lerp(diverter.target_angle, dt * 10.0);
        tf.rotation = Quat::from_rotation_y(diverter.paddle_angle);

        // If active and angled, redirect present cubes colliding with the diverter area
        if diverter.is_diverting && diverter.paddle_angle > 0.3 {
            let paddle_pos = tf.translation;
            for (cube_tf, mut vel) in &mut q_cubes {
                let d = cube_tf.translation - paddle_pos;
                if d.x.abs() < 0.65 && d.z.abs() < 0.65 && d.y.abs() < 0.6 {
                    let dir = diverter.divert_direction;
                    vel.linear.x = dir.x * diverter.divert_speed;
                    vel.linear.z = dir.z * diverter.divert_speed;
                }
            }
        }
    }
}

/// Guided gravity acceleration for cubes sliding down the alternate assembly line ramp
pub fn assembly_line_ramp_physics_system(
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_ramps: Query<&AssemblyLineRamp>,
    mut q_cubes: Query<(&Transform, &mut Velocity), (With<Present>, With<Grabbable>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    for ramp in &q_ramps {
        let ramp_vec = ramp.end_pos - ramp.start_pos;
        let ramp_dir = ramp_vec.normalize();
        let ramp_len_sq = ramp_vec.length_squared();

        for (cube_tf, mut vel) in &mut q_cubes {
            let to_cube = cube_tf.translation - ramp.start_pos;
            let proj = to_cube.dot(ramp_dir);
            if proj >= -0.3 && proj * proj <= ramp_len_sq + 0.5 {
                // Lateral distance to ramp centerline
                let perp = to_cube - ramp_dir * proj;
                if perp.length_squared() < 0.6 * 0.6 {
                    vel.linear.x = ramp_dir.x * ramp.slide_speed;
                    vel.linear.z = ramp_dir.z * ramp.slide_speed;
                    if vel.linear.y > -0.5 {
                        vel.linear.y = ramp_dir.y * ramp.slide_speed;
                    }
                }
            }
        }
    }
}

/// Conveyor Catapult state machine: launches presents, elves, and players with slapstick impulses
pub fn conveyor_catapult_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_catapults: Query<
        (Entity, &mut Transform, &mut ConveyorCatapult),
        (Without<Present>, Without<ElfAssistant>, Without<ElfPlayer>),
    >,
    mut q_presents: Query<
        (&Transform, &mut Velocity),
        (With<Present>, With<Grabbable>, Without<ElfAssistant>, Without<ConveyorCatapult>),
    >,
    mut q_elves: Query<
        (&Transform, &mut Velocity, &mut ElfAssistant),
        (Without<Present>, Without<ConveyorCatapult>),
    >,
    mut q_player: Query<(&Transform, &mut ElfMovement), (With<ElfPlayer>, Without<ConveyorCatapult>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    for (_cat_e, mut tf, mut catapult) in &mut q_catapults {
        let cat_pos = tf.translation;

        match catapult.state {
            CatapultState::Ready => {
                catapult.arm_angle = catapult.arm_angle.lerp(0.0, dt * 10.0);
                tf.rotation = Quat::from_rotation_z(catapult.arm_angle);

                if catapult.auto_trigger {
                    // Check if any present cube is resting in the catapult cradle
                    let mut triggered = false;
                    for (cube_tf, _) in &q_presents {
                        let d = cube_tf.translation - cat_pos;
                        if d.x.abs() < catapult.trigger_radius
                            && d.z.abs() < catapult.trigger_radius
                            && d.y > -0.2
                            && d.y < 0.8
                        {
                            triggered = true;
                            break;
                        }
                    }

                    // Also check for ragdoll elf standing on the flipper
                    if !triggered {
                        for (elf_tf, _, _) in &q_elves {
                            let d = elf_tf.translation - cat_pos;
                            if d.x.abs() < catapult.trigger_radius && d.z.abs() < catapult.trigger_radius {
                                triggered = true;
                                break;
                            }
                        }
                    }

                    if triggered {
                        catapult.state = CatapultState::Launching;
                        catapult.state_timer = 0.0;
                    }
                }
            }
            CatapultState::Launching => {
                catapult.state_timer += dt;
                catapult.arm_angle = catapult.arm_angle.lerp(1.25, dt * 25.0);
                tf.rotation = Quat::from_rotation_z(catapult.arm_angle);

                // Impart extreme launch impulse on all objects in cradle
                let launch_v = catapult.launch_velocity;

                // 1. Launch presents
                for (cube_tf, mut vel) in &mut q_presents {
                    let d = cube_tf.translation - cat_pos;
                    if d.x.abs() < catapult.trigger_radius + 0.3
                        && d.z.abs() < catapult.trigger_radius + 0.3
                        && d.y > -0.3
                        && d.y < 1.0
                    {
                        vel.linear = launch_v;
                        vel.angular = Vec3::new(3.5, 1.2, 2.8);
                    }
                }

                // 2. Launch ragdoll elves
                for (elf_tf, mut vel, mut elf) in &mut q_elves {
                    let d = elf_tf.translation - cat_pos;
                    if d.x.abs() < catapult.trigger_radius + 0.3
                        && d.z.abs() < catapult.trigger_radius + 0.3
                    {
                        elf.state = ElfNpcState::RagdollTumble;
                        elf.state_timer = 0.0;
                        vel.linear = launch_v * 0.9;
                    }
                }

                // 3. Launch player if standing on catapult
                for (player_tf, mut movement) in &mut q_player {
                    let d = player_tf.translation - cat_pos;
                    if d.x.abs() < catapult.trigger_radius && d.z.abs() < catapult.trigger_radius && d.y > 0.0 && d.y < 1.2 {
                        movement.velocity = launch_v * 0.85;
                        movement.is_grounded = false;
                    }
                }

                if catapult.state_timer >= 0.12 {
                    catapult.state = CatapultState::Recoil;
                    catapult.state_timer = 0.0;
                }
            }
            CatapultState::Recoil => {
                catapult.state_timer += dt;
                // Damped harmonic recoil returning to rest
                catapult.arm_angle = catapult.arm_angle.lerp(0.0, dt * 8.0);
                tf.rotation = Quat::from_rotation_z(catapult.arm_angle);

                if catapult.state_timer >= 0.35 {
                    catapult.state = CatapultState::Cooldown;
                    catapult.state_timer = 0.0;
                }
            }
            CatapultState::Cooldown => {
                catapult.state_timer += dt;
                catapult.arm_angle = catapult.arm_angle.lerp(0.0, dt * 10.0);
                tf.rotation = Quat::from_rotation_z(catapult.arm_angle);

                if catapult.state_timer >= catapult.cooldown_duration {
                    catapult.state = CatapultState::Ready;
                    catapult.state_timer = 0.0;
                }
            }
        }
    }
}

/// Player manual interaction with the Catapult and Diverter levers
pub fn catapult_and_diverter_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    q_player: Query<&Transform, With<ElfPlayer>>,
    mut q_catapults: Query<&mut ConveyorCatapult>,
    mut q_diverters: Query<&mut AssemblyLineDiverter>,
    q_cat_levers: Query<(&Transform, &CatapultLever)>,
    q_div_levers: Query<(&Transform, &DiverterLever)>,
    mut trigger_messages: MessageReader<TriggerCatapultMessage>,
    mut toggle_messages: MessageReader<ToggleDiverterMessage>,
) {
    let mut do_trigger_catapult = false;
    let mut do_toggle_diverter = false;

    // 1. Process Messages
    for _ in trigger_messages.read() {
        do_trigger_catapult = true;
    }
    for _ in toggle_messages.read() {
        do_toggle_diverter = true;
    }

    // 2. Process Player Proximity & Input
    let interact_pressed = keys.as_ref().map(|k| {
        k.just_pressed(KeyCode::KeyE)
            || k.just_pressed(KeyCode::KeyQ)
            || k.just_pressed(KeyCode::KeyF)
    }).unwrap_or(false)
        || mouse.as_ref().map(|m| m.just_pressed(MouseButton::Left)).unwrap_or(false);

    if let Ok(player_tf) = q_player.single() {
        let p_pos = player_tf.translation;

        // Diverter Lever
        for (lever_tf, lever) in &q_div_levers {
            if (lever_tf.translation - p_pos).length_squared() < 2.2 * 2.2 && interact_pressed {
                if let Ok(mut diverter) = q_diverters.get_mut(lever.diverter_entity) {
                    diverter.is_diverting = !diverter.is_diverting;
                }
            }
        }

        // Catapult Lever
        for (lever_tf, lever) in &q_cat_levers {
            if (lever_tf.translation - p_pos).length_squared() < 2.2 * 2.2 && interact_pressed {
                if let Ok(mut catapult) = q_catapults.get_mut(lever.catapult_entity) {
                    if catapult.state == CatapultState::Ready {
                        catapult.state = CatapultState::Launching;
                        catapult.state_timer = 0.0;
                    }
                }
            }
        }
    }

    // Apply direct messages if received
    if do_trigger_catapult {
        for mut catapult in &mut q_catapults {
            if catapult.state == CatapultState::Ready {
                catapult.state = CatapultState::Launching;
                catapult.state_timer = 0.0;
            }
        }
    }
    if do_toggle_diverter {
        for mut diverter in &mut q_diverters {
            diverter.is_diverting = !diverter.is_diverting;
        }
    }
}
