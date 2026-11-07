use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::tailwind,
    input::mouse::AccumulatedMouseMotion,
    light::NotShadowCaster,
    prelude::*,
};
use bevy_rapier3d::prelude::*;

use super::{
    platforms::{MovingPlatform, MovingPlatformType},
    scene::SphereObject,
    ui::{InteractionPromptText, MenuState},
    VIEW_MODEL_RENDER_LAYER,
};

#[derive(Debug, Component)]
pub struct Player;

#[derive(Debug, Component)]
pub struct CameraPivot;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct CameraSensitivity(pub Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(Vec2::new(0.0020, 0.0014))
    }
}

#[derive(Debug, Component)]
pub struct WorldModelCamera;

#[derive(Debug, Component)]
pub struct ViewModelArm {
    pub base_translation: Vec3,
}

#[derive(Debug, Component)]
pub struct CharacterMovement {
    pub vertical_speed: f32,
    pub grounded_timer: f32,
    pub horizontal_speed: f32,
    pub horizontal_dir: Vec3,
    pub platform_inertia: Vec3,
}

impl Default for CharacterMovement {
    fn default() -> Self {
        Self {
            vertical_speed: 0.0,
            grounded_timer: 0.0,
            horizontal_speed: 0.0,
            horizontal_dir: Vec3::ZERO,
            platform_inertia: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Component)]
pub struct HeldByPlayer;

pub fn spawn_player_viewmodel(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arm_mesh = meshes.add(Cuboid::new(0.12, 0.12, 0.5));
    let sleeve_mesh = meshes.add(Cuboid::new(0.15, 0.15, 0.2));
    let arm_material = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::TEAL_300),
        perceptual_roughness: 0.8,
        ..default()
    });
    let sleeve_material = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::SLATE_800),
        perceptual_roughness: 0.5,
        ..default()
    });

    let initial_arm_pos = Vec3::new(0.25, -0.2, -0.4);

    commands.spawn((
        Player,
        CharacterMovement::default(),
        CameraSensitivity::default(),
        Transform::from_xyz(0.0, 2.0, 6.0),
        Visibility::default(),
        Collider::capsule_y(0.5, 0.35),
        ColliderDebug::NeverRender,
        KinematicCharacterController {
            custom_mass: Some(70.0),
            up: Vec3::Y,
            offset: CharacterLength::Absolute(0.01),
            slide: true,
            autostep: Some(CharacterAutostep {
                max_height: CharacterLength::Relative(0.35),
                min_width: CharacterLength::Relative(0.5),
                include_dynamic_bodies: false,
            }),
            max_slope_climb_angle: 45.0_f32.to_radians(),
            min_slope_slide_angle: 30.0_f32.to_radians(),
            apply_impulse_to_dynamic_bodies: true,
            snap_to_ground: None,
            ..default()
        },
        children![
            (
                CameraPivot,
                Transform::from_xyz(0.0, 0.5, 0.0),
                Visibility::default(),
                children![
                    (
                        WorldModelCamera,
                        Camera3d::default(),
                        Projection::from(PerspectiveProjection {
                            fov: 80.0_f32.to_radians(),
                            ..default()
                        }),
                    ),
                    (
                        Camera3d::default(),
                        Camera {
                            order: 1,
                            ..default()
                        },
                        Projection::from(PerspectiveProjection {
                            fov: 70.0_f32.to_radians(),
                            ..default()
                        }),
                        RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                    ),
                    (
                        ViewModelArm {
                            base_translation: initial_arm_pos,
                        },
                        Transform::from_translation(initial_arm_pos),
                        Visibility::default(),
                        children![
                            (
                                Mesh3d(arm_mesh),
                                MeshMaterial3d(arm_material),
                                Transform::from_xyz(0.0, 0.0, 0.0),
                                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                                NotShadowCaster,
                            ),
                            (
                                Mesh3d(sleeve_mesh),
                                MeshMaterial3d(sleeve_material),
                                Transform::from_xyz(0.0, 0.0, 0.15),
                                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                                NotShadowCaster,
                            ),
                        ],
                    ),
                ],
            ),
        ],
    ));
}

pub fn player_look(
    menu_state: Res<MenuState>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    player: Single<(&mut Transform, &CameraSensitivity), (With<Player>, Without<CameraPivot>)>,
    pivot: Single<&mut Transform, (With<CameraPivot>, Without<Player>)>,
) {
    if menu_state.is_open {
        return;
    }

    let (mut player_transform, sensitivity) = player.into_inner();
    let mut pivot_transform = pivot.into_inner();
    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        let effective_sensitivity = sensitivity.0 * menu_state.mouse_sensitivity;
        let delta_yaw = -delta.x * effective_sensitivity.x;
        let delta_pitch = -delta.y * effective_sensitivity.y;

        player_transform.rotate_y(delta_yaw);

        let (current_pitch, _, _) = pivot_transform.rotation.to_euler(EulerRot::XYZ);
        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.02;
        let new_pitch = (current_pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        pivot_transform.rotation = Quat::from_rotation_x(new_pitch);
    }
}

pub fn player_move(
    menu_state: Res<MenuState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    player: Single<(
        &mut Transform,
        &mut KinematicCharacterController,
        Option<&KinematicCharacterControllerOutput>,
        &mut CharacterMovement,
    ), With<Player>>,
    held_sphere_query: Query<&SphereObject, With<HeldByPlayer>>,
    platform_query: Query<(Entity, &MovingPlatform)>,
) {
    let (mut player_transform, mut controller, output, mut movement) = player.into_inner();

    if menu_state.is_open {
        controller.translation = Some(Vec3::ZERO);
        movement.horizontal_speed = 0.0;
        movement.platform_inertia = Vec3::ZERO;
        return;
    }

    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    const GRAVITY: f32 = 22.0;
    const JUMP_SPEED: f32 = 8.5;
    const BASE_MOVE_SPEED: f32 = 6.0;
    const COYOTE_TIME: f32 = 0.15;

    let is_rapier_grounded = output.map(|o| o.grounded).unwrap_or(false);

    // 1. Horizontal movement input
    let forward = player_transform.forward();
    let forward_planar = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right = player_transform.right();
    let right_planar = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let mut horizontal_dir = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        horizontal_dir += forward_planar;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        horizontal_dir -= forward_planar;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        horizontal_dir -= right_planar;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        horizontal_dir += right_planar;
    }

    let mut speed = BASE_MOVE_SPEED;
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        speed *= 1.5;
    }

    if let Some(held_sphere) = held_sphere_query.iter().next() {
        let weight_factor = (1.0 - (held_sphere.mass / 150.0)).clamp(0.4, 1.0);
        speed *= weight_factor;
    }

    let dir_normalized = horizontal_dir.normalize_or_zero();
    let actual_horizontal_speed = if dir_normalized != Vec3::ZERO { speed } else { 0.0 };

    movement.horizontal_speed = actual_horizontal_speed;
    movement.horizontal_dir = dir_normalized;

    let player_horizontal_vel = dir_normalized * actual_horizontal_speed;

    // 2. Moving Platform Detection & Motion Transfer
    let player_pos = player_transform.translation;
    let feet_y = player_pos.y - 0.85;

    let mut active_platform: Option<(&MovingPlatform, Vec3, f32, f32)> = None;

    // Only detect being grounded on a platform if the player is not actively ascending from a jump
    if movement.vertical_speed <= 0.0 {
        for (p_entity, platform) in platform_query.iter() {
            let p_pos = platform.current_translation;
            let p_top = p_pos.y + 0.2;
            let diff_y = feet_y - p_top;

            let in_horizontal = match platform.platform_type {
                MovingPlatformType::SideToSide { .. } | MovingPlatformType::Vertical { .. } => {
                    let dx = (player_pos.x - p_pos.x).abs();
                    let dz = (player_pos.z - p_pos.z).abs();
                    dx <= 1.70 && dz <= 1.70
                }
                MovingPlatformType::Spinning { origin, .. } => {
                    let r_vec = Vec3::new(player_pos.x - origin.x, 0.0, player_pos.z - origin.z);
                    r_vec.length() <= 2.70
                }
            };

            let is_touching_rapier = output.map_or(false, |out| {
                out.collisions.iter().any(|c| c.entity == p_entity)
            });

            if in_horizontal && (is_touching_rapier || (diff_y >= -0.20 && diff_y <= 0.15)) {
                let (plat_disp_h, plat_disp_y, plat_yaw) = match platform.platform_type {
                    MovingPlatformType::SideToSide { .. } => {
                        (
                            Vec3::new(platform.delta_translation.x, 0.0, platform.delta_translation.z),
                            0.0,
                            0.0,
                        )
                    }
                    MovingPlatformType::Vertical { .. } => {
                        (
                            Vec3::ZERO,
                            platform.delta_translation.y,
                            0.0,
                        )
                    }
                    MovingPlatformType::Spinning { origin, speed } => {
                        let r_vec = Vec3::new(player_pos.x - origin.x, 0.0, player_pos.z - origin.z);
                        let delta_yaw = speed * dt;
                        let rotated_r = Quat::from_rotation_y(delta_yaw) * r_vec;
                        let spin_disp = rotated_r - r_vec;
                        (
                            Vec3::new(spin_disp.x, 0.0, spin_disp.z),
                            0.0,
                            delta_yaw,
                        )
                    }
                };

                active_platform = Some((platform, plat_disp_h, plat_disp_y, plat_yaw));
                break;
            }
        }
    }

    // 3. Grounded State, Jumping, and Vertical Resolution
    let mut total_displacement = player_horizontal_vel * dt;

    if let Some((platform, plat_disp_h, plat_disp_y, plat_yaw)) = active_platform {
        movement.grounded_timer = COYOTE_TIME;

        if plat_yaw.abs() > 1e-6 {
            player_transform.rotate_y(plat_yaw);
        }

        total_displacement += plat_disp_h;

        if keyboard_input.just_pressed(KeyCode::Space) {
            let upward_boost = match platform.platform_type {
                MovingPlatformType::Vertical { .. } => {
                    (platform.delta_translation.y / dt).max(0.0)
                }
                _ => 0.0,
            };
            movement.vertical_speed = JUMP_SPEED + upward_boost;
            movement.grounded_timer = 0.0;
            movement.platform_inertia = plat_disp_h / dt;
            total_displacement.y += movement.vertical_speed * dt;
        } else {
            movement.vertical_speed = -0.5;
            total_displacement.y += plat_disp_y - (0.5 * dt);
            movement.platform_inertia = plat_disp_h / dt;
        }
    } else {
        let is_grounded = is_rapier_grounded;
        if is_grounded {
            movement.grounded_timer = COYOTE_TIME;
            movement.platform_inertia = Vec3::ZERO;
        } else if movement.grounded_timer > 0.0 {
            movement.grounded_timer -= dt;
        }

        let can_jump = movement.grounded_timer > 0.0;
        if can_jump {
            if movement.vertical_speed < 0.0 {
                movement.vertical_speed = -0.5;
            }
            if keyboard_input.just_pressed(KeyCode::Space) {
                movement.vertical_speed = JUMP_SPEED;
                movement.grounded_timer = 0.0;
            }
        } else {
            movement.vertical_speed -= GRAVITY * dt;
            movement.vertical_speed = movement.vertical_speed.max(-30.0);
        }

        if !is_grounded && movement.platform_inertia != Vec3::ZERO {
            total_displacement += movement.platform_inertia * dt;
            movement.platform_inertia = movement.platform_inertia.lerp(Vec3::ZERO, dt * 2.0);
        }

        total_displacement.y += movement.vertical_speed * dt;
    }

    controller.translation = Some(total_displacement);
}

pub fn player_push_spheres(
    time: Res<Time>,
    player_query: Single<(&Transform, &CharacterMovement), With<Player>>,
    mut sphere_query: Query<
        (&Transform, &mut Velocity, &mut Sleeping, &mut SphereObject),
        Without<HeldByPlayer>,
    >,
) {
    let (player_transform, movement) = player_query.into_inner();
    let player_pos = player_transform.translation;
    let player_radius = 0.45;
    let dt = time.delta_secs();

    if movement.horizontal_speed < 0.1 {
        return;
    }

    for (sphere_transform, mut sphere_velocity, mut sleeping, mut sphere) in sphere_query.iter_mut() {
        let sphere_pos = sphere_transform.translation;
        let delta = sphere_pos - player_pos;
        let horizontal_delta = Vec3::new(delta.x, 0.0, delta.z);
        let dist = horizontal_delta.length();
        let min_contact_dist = player_radius + sphere.radius;

        if dist < min_contact_dist + 0.20 && (sphere_pos.y - player_pos.y).abs() < 1.3 {
            let push_dir = horizontal_delta.normalize_or_zero();
            let movement_alignment = movement.horizontal_dir.dot(push_dir);

            if movement_alignment > 0.08 {
                sleeping.sleeping = false;
                sphere.settle_cooldown = 1.0;

                let mass_factor = (70.0 / (70.0 + sphere.mass * 1.2)).clamp(0.25, 0.9);
                let target_speed = movement.horizontal_speed * mass_factor * movement_alignment;

                let current_push_speed = sphere_velocity.linear.dot(push_dir);
                if current_push_speed < target_speed {
                    let accel = 9.0 * mass_factor;
                    let speed_increment = (accel * dt).min(target_speed - current_push_speed);
                    sphere_velocity.linear += push_dir * speed_increment;
                }

                let lateral_dir = Vec3::new(-push_dir.z, 0.0, push_dir.x);
                let lateral_speed = sphere_velocity.linear.dot(lateral_dir);
                sphere_velocity.linear -= lateral_dir * lateral_speed * (dt * 6.0).min(0.35);

                let roll_axis = Vec3::Y.cross(push_dir);
                sphere_velocity.angular = roll_axis * (sphere_velocity.linear.length() / sphere.radius);
            }
        }
    }
}

pub fn handle_pickup_interaction(
    mut commands: Commands,
    menu_state: Res<MenuState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Single<&GlobalTransform, With<WorldModelCamera>>,
    player_query: Single<(&Transform, &CharacterMovement), With<Player>>,
    mut held_query: Query<(Entity, &mut Velocity, &mut Sleeping, &mut SphereObject), With<HeldByPlayer>>,
    mut non_held_spheres: Query<(Entity, &GlobalTransform, &mut Velocity, &mut Sleeping, &mut SphereObject), Without<HeldByPlayer>>,
    mut prompt_query: Single<&mut Text, With<InteractionPromptText>>,
) {
    if menu_state.is_open {
        return;
    }

    let camera_gt = camera_query.into_inner();
    let camera_pos = camera_gt.translation();
    let camera_forward = camera_gt.forward();

    if let Some((held_entity, mut held_velocity, mut sleeping, mut held_sphere)) = held_query.iter_mut().next() {
        prompt_query.0 = format!(
            "Holding: {} ({} kg) - [E] Drop / Throw",
            held_sphere.name, held_sphere.mass as u32
        );

        if keyboard_input.just_pressed(KeyCode::KeyE) {
            commands
                .entity(held_entity)
                .remove::<(HeldByPlayer, RigidBodyDisabled, ColliderDisabled)>();

            let (_, movement) = player_query.into_inner();
            let forward_dir = *camera_forward;

            let throw_speed = (60.0 / (15.0 + held_sphere.mass)) * 2.8;
            let player_momentum = movement.horizontal_dir * movement.horizontal_speed * 0.4;

            held_velocity.linear = (forward_dir * throw_speed) + (Vec3::Y * 0.9) + player_momentum;
            held_velocity.angular = camera_gt.right().as_vec3() * (throw_speed * 0.5);

            sleeping.sleeping = false;
            held_sphere.settle_cooldown = 1.5;
        }
    } else {
        const MAX_PICKUP_DIST: f32 = 4.0;
        let mut best_target: Option<(Entity, &'static str, f32, f32)> = None;

        for (entity, sphere_gt, _, _, sphere) in non_held_spheres.iter() {
            let sphere_pos = sphere_gt.translation();
            let to_sphere = sphere_pos - camera_pos;
            let dist = to_sphere.length();

            if dist <= MAX_PICKUP_DIST {
                let dir = to_sphere.normalize_or_zero();
                let dot = dir.dot(*camera_forward);

                if dot > 0.65 {
                    let score = dot / (dist + 0.3);
                    if let Some((_, _, _, best_score)) = best_target {
                        if score > best_score {
                            best_target = Some((entity, sphere.name, sphere.mass, score));
                        }
                    } else {
                        best_target = Some((entity, sphere.name, sphere.mass, score));
                    }
                }
            }
        }

        if let Some((target_entity, name, mass, _)) = best_target {
            prompt_query.0 = format!("[E] Pick up {} ({} kg)", name, mass as u32);

            if keyboard_input.just_pressed(KeyCode::KeyE) {
                if let Ok((_, _, mut vel, mut sleeping, mut sphere)) = non_held_spheres.get_mut(target_entity) {
                    vel.linear = Vec3::ZERO;
                    vel.angular = Vec3::ZERO;
                    sleeping.sleeping = false;
                    sphere.settle_cooldown = 1.5;
                }

                commands.entity(target_entity).insert((
                    HeldByPlayer,
                    RigidBodyDisabled,
                    ColliderDisabled,
                ));
            }
        } else {
            prompt_query.0 = "Aim at a sphere and press [E] to pick up".into();
        }
    }
}

pub fn update_held_sphere(
    camera_query: Single<&GlobalTransform, With<WorldModelCamera>>,
    time: Res<Time>,
    mut held_spheres: Query<(&mut Transform, &SphereObject), With<HeldByPlayer>>,
) {
    let camera_gt = camera_query.into_inner();
    let camera_pos = camera_gt.translation();
    let forward = *camera_gt.forward();
    let right = *camera_gt.right();
    let up = *camera_gt.up();

    for (mut sphere_transform, sphere) in held_spheres.iter_mut() {
        let hold_dist = 1.35 + sphere.radius * 0.4;
        let target_pos = camera_pos + (forward * hold_dist) + (right * 0.22) - (up * 0.3);

        sphere_transform.translation = sphere_transform
            .translation
            .lerp(target_pos, time.delta_secs() * 25.0);
    }
}

pub fn viewmodel_bob(
    menu_state: Res<MenuState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    player_movement: Single<&CharacterMovement, With<Player>>,
    view_model: Single<(&mut Transform, &ViewModelArm)>,
) {
    let (mut transform, arm) = view_model.into_inner();

    if menu_state.is_open {
        transform.translation = transform
            .translation
            .lerp(arm.base_translation, time.delta_secs() * 10.0);
        return;
    }

    let is_moving = keyboard_input.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyS,
        KeyCode::KeyA,
        KeyCode::KeyD,
    ]);

    let is_grounded = player_movement.grounded_timer > 0.0;

    if is_moving && is_grounded {
        let speed_mult = if keyboard_input.pressed(KeyCode::ShiftLeft) {
            12.0
        } else {
            8.0
        };
        let elapsed = time.elapsed_secs() * speed_mult;
        let bob_x = (elapsed * 0.5).sin() * 0.015;
        let bob_y = elapsed.sin().abs() * 0.015;
        transform.translation = arm.base_translation + Vec3::new(bob_x, -bob_y, 0.0);
    } else {
        transform.translation = transform
            .translation
            .lerp(arm.base_translation, time.delta_secs() * 10.0);
    }
}

pub fn change_fov(
    input: Res<ButtonInput<KeyCode>>,
    mut world_model_projection: Single<&mut Projection, With<WorldModelCamera>>,
) {
    let Projection::Perspective(perspective) = world_model_projection.as_mut() else {
        return;
    };

    if input.pressed(KeyCode::ArrowUp) {
        perspective.fov -= 1.0_f32.to_radians();
        perspective.fov = perspective.fov.max(20.0_f32.to_radians());
    }
    if input.pressed(KeyCode::ArrowDown) {
        perspective.fov += 1.0_f32.to_radians();
        perspective.fov = perspective.fov.min(150.0_f32.to_radians());
    }
}
