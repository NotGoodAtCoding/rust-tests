use std::f32::consts::FRAC_PI_2;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
};
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    player::{BhopConfig, ElfHands, ElfMovement, update_elf_physics},
    ui::MenuState,
    workshop::room::ConveyorBelt,
};

#[derive(Component)]
pub struct ElfPlayer;

#[derive(Component)]
pub struct CameraPivot {
    pub pitch: f32,
}

#[derive(Component)]
pub struct FirstPersonCamera;

#[derive(Component)]
pub struct LeftHandVisual;

#[derive(Component)]
pub struct RightHandVisual;

#[derive(Component, Debug, Clone)]
pub struct FloatyHand {
    pub rest_position: Vec3,
    pub current_offset: Vec3,
    pub velocity: Vec3,
    pub spring_stiffness: f32,
    pub damping: f32,
    pub phase: f32,
}

impl FloatyHand {
    pub fn new(rest_position: Vec3, phase: f32) -> Self {
        Self {
            rest_position,
            current_offset: Vec3::ZERO,
            velocity: Vec3::ZERO,
            spring_stiffness: 28.0,
            damping: 8.5,
            phase,
        }
    }
}

#[derive(Component)]
pub struct ViewModelRoot;

#[derive(Resource)]
pub struct ElfInputSettings {
    pub mouse_sensitivity: Vec2,
}

impl Default for ElfInputSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: Vec2::new(0.0020, 0.0016),
        }
    }
}

pub fn spawn_elf_player(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let hand_material = workshop_mats.as_ref().map(|wm| wm.pine_green.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PINE_GREEN,
            perceptual_roughness: 0.7,
            ..default()
        })
    });

    // Player entity with Rapier character controller
    commands
        .spawn((
            ElfPlayer,
            ElfMovement::default(),
            ElfHands::default(),
            Transform::from_xyz(0.0, 1.2, 0.0),
            Visibility::default(),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.42, 0.48),
            CollisionGroups::new(crate::elf_game::player::hands::GROUP_PLAYER, !crate::elf_game::player::hands::GROUP_HELD_ITEM),
            KinematicCharacterController {
                filter_groups: Some(CollisionGroups::new(
                    crate::elf_game::player::hands::GROUP_PLAYER,
                    !crate::elf_game::player::hands::GROUP_HELD_ITEM,
                )),
                offset: CharacterLength::Absolute(0.04),
                slide: true,
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.25),
                    min_width: CharacterLength::Absolute(0.2),
                    include_dynamic_bodies: true,
                }),
                snap_to_ground: None,
                ..default()
            },
            Name::new("ElfPlayer"),
        ))
        .with_children(|parent| {
            // Camera pivot (head level)
            parent
                .spawn((
                    CameraPivot { pitch: 0.0 },
                    Transform::from_xyz(0.0, 0.6, 0.0),
                    Visibility::default(),
                    Name::new("CameraPivot"),
                ))
                .with_children(|head| {
                    head.spawn((
                        Camera3d::default(),
                        FirstPersonCamera,
                        Transform::IDENTITY,
                        Name::new("FirstPersonCamera"),
                    ));

                    // Viewmodel root for hands - rotates with head pitch
                    head.spawn((
                        ViewModelRoot,
                        Transform::IDENTITY,
                        Visibility::default(),
                        Name::new("ViewModelRoot"),
                    ))
                    .with_children(|vm| {
                        // Left hand model (controlled with Q)
                        vm.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.065))),
                            MeshMaterial3d(hand_material.clone()),
                            Transform::from_xyz(-0.35, -0.3, -0.6),
                            LeftHandVisual,
                            FloatyHand::new(Vec3::new(-0.35, -0.3, -0.6), 0.0),
                            Name::new("LeftHandGlove"),
                        ));

                        // Right hand model (controlled with E)
                        vm.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.065))),
                            MeshMaterial3d(hand_material.clone()),
                            Transform::from_xyz(0.35, -0.3, -0.6),
                            RightHandVisual,
                            FloatyHand::new(Vec3::new(0.35, -0.3, -0.6), std::f32::consts::PI),
                            Name::new("RightHandGlove"),
                        ));
                    });
                });
        });
}

pub fn elf_look_system(
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    settings: Res<ElfInputSettings>,
    mut q_pivot: Query<(&mut Transform, &mut CameraPivot)>,
    mut q_player: Query<&mut Transform, (With<ElfPlayer>, Without<CameraPivot>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let delta = accumulated_mouse_motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    let sensitivity_multiplier = menu_state.map(|m| m.mouse_sensitivity).unwrap_or(1.0);
    let effective_sensitivity = settings.mouse_sensitivity * sensitivity_multiplier;

    // Horizontal rotation (Yaw) rotates the entire player body around Y
    if let Ok(mut player_tf) = q_player.single_mut() {
        let yaw = -delta.x * effective_sensitivity.x;
        player_tf.rotate_y(yaw);
    }

    // Vertical rotation (Pitch) directly accumulates pitch in radians
    if let Ok((mut pivot_tf, mut pivot)) = q_pivot.single_mut() {
        let pitch_delta = -delta.y * effective_sensitivity.y;
        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.05;
        pivot.pitch = (pivot.pitch + pitch_delta).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        pivot_tf.rotation = Quat::from_rotation_x(pivot.pitch);
    }
}

pub fn elf_player_controller_system(
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    time: Option<Res<Time>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bhop_config: Res<BhopConfig>,
    q_conveyor: Query<(&Transform, &ConveyorBelt), Without<ElfPlayer>>,
    q_pivot: Query<&CameraPivot>,
    mut q_player: Query<
        (
            &mut Transform,
            &mut ElfMovement,
            &mut KinematicCharacterController,
            Option<&KinematicCharacterControllerOutput>,
        ),
        With<ElfPlayer>,
    >,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        for (_, mut movement, mut controller, _) in &mut q_player {
            controller.translation = Some(Vec3::ZERO);
            movement.velocity = Vec3::ZERO;
        }
        return;
    }

    let Some(time) = time else {
        return;
    };
    let Some(keys) = keys else {
        return;
    };

    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    // Noclip mode: free 3D flying movement without gravity or collision
    let noclip = menu_state.as_ref().map(|m| m.noclip).unwrap_or(false);
    if noclip {
        let pitch = q_pivot.iter().next().map(|p| p.pitch).unwrap_or(0.0);
        for (mut transform, mut movement, mut controller, _) in &mut q_player {
            movement.velocity = Vec3::ZERO;
            movement.is_grounded = false;
            controller.translation = None;

            let yaw_quat = transform.rotation;
            let pitch_quat = Quat::from_rotation_x(pitch);
            let look_quat = yaw_quat * pitch_quat;
            let forward_3d = look_quat * Vec3::NEG_Z;
            let right_3d = yaw_quat * Vec3::X;

            let mut fly_dir = Vec3::ZERO;
            if keys.pressed(KeyCode::KeyW) {
                fly_dir += forward_3d;
            }
            if keys.pressed(KeyCode::KeyS) {
                fly_dir -= forward_3d;
            }
            if keys.pressed(KeyCode::KeyA) {
                fly_dir -= right_3d;
            }
            if keys.pressed(KeyCode::KeyD) {
                fly_dir += right_3d;
            }
            if keys.pressed(KeyCode::Space) {
                fly_dir += Vec3::Y;
            }
            if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::KeyC) {
                fly_dir -= Vec3::Y;
            }

            let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                24.0
            } else {
                12.0
            };

            if fly_dir.length_squared() > 0.001 {
                let move_vec = fly_dir.normalize() * speed * dt;
                transform.translation += move_vec;
            }
        }
        return;
    }

    for (transform, mut movement, mut controller, output_opt) in &mut q_player {
        if let Some(output) = output_opt {
            movement.is_grounded = output.grounded;
            if movement.is_grounded && movement.velocity.y < 0.0 {
                movement.velocity.y = 0.0;
            }
        }

        // Calculate wish direction in local player space
        let mut wish = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            wish.z -= 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            wish.z += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            wish.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            wish.x += 1.0;
        }

        let wish_world = if wish.length_squared() > 0.001 {
            (transform.rotation * wish.normalize()).with_y(0.0).normalize()
        } else {
            Vec3::ZERO
        };

        let jump_just_pressed = keys.just_pressed(KeyCode::Space);
        let jump_held = keys.pressed(KeyCode::Space);

        update_elf_physics(
            dt,
            wish_world,
            jump_just_pressed,
            jump_held,
            &bhop_config,
            &mut movement,
        );

        // Conveyor belt riding: if grounded on the conveyor belt, add conveyor velocity
        let mut conveyor_drift = Vec3::ZERO;
        if movement.is_grounded {
            for (c_tf, belt) in &q_conveyor {
                if (transform.translation.z - c_tf.translation.z).abs() < 0.75
                    && (transform.translation.x - c_tf.translation.x).abs() < 9.0
                    && transform.translation.y > 0.4
                    && transform.translation.y < 1.6
                {
                    conveyor_drift = belt.direction * belt.speed * dt;
                    break;
                }
            }
        }

        let displacement = movement.velocity * dt + conveyor_drift;
        controller.translation = Some(displacement);
    }
}

pub fn toggle_noclip_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    menu_state: Option<ResMut<MenuState>>,
) {
    let (Some(keys), Some(mut menu_state)) = (keys, menu_state) else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyV) {
        menu_state.noclip = !menu_state.noclip;
    }
}

pub fn sync_noclip_collider_system(
    menu_state: Option<Res<MenuState>>,
    mut commands: Commands,
    q_player: Query<(Entity, Has<ColliderDisabled>), With<ElfPlayer>>,
) {
    let Some(menu_state) = menu_state else {
        return;
    };
    if !menu_state.is_changed() {
        return;
    }
    for (entity, has_disabled) in &q_player {
        if menu_state.noclip && !has_disabled {
            commands.entity(entity).insert(ColliderDisabled);
        } else if !menu_state.noclip && has_disabled {
            commands.entity(entity).remove::<ColliderDisabled>();
        }
    }
}

pub fn viewmodel_bob(
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    time: Option<Res<Time>>,
    movement_query: Query<&ElfMovement, With<ElfPlayer>>,
    mut vm_query: Query<&mut Transform, With<ViewModelRoot>>,
) {
    let (Some(time), Ok(movement), Ok(mut vm_tf)) = (
        time,
        movement_query.single(),
        vm_query.single_mut(),
    ) else {
        return;
    };

    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    if menu_state.as_ref().map(|m| m.noclip).unwrap_or(false) {
        vm_tf.translation = Vec3::ZERO;
        return;
    }

    let h_vel = Vec2::new(movement.velocity.x, movement.velocity.z).length();
    if movement.is_grounded && h_vel > 0.5 {
        let bob_speed = (h_vel * 2.0).clamp(6.0, 14.0);
        let elapsed = time.elapsed_secs() * bob_speed;
        let bob_x = (elapsed * 0.5).sin() * 0.015;
        let bob_y = elapsed.sin().abs() * 0.012;
        vm_tf.translation = Vec3::new(bob_x, -bob_y, 0.0);
    } else {
        vm_tf.translation = vm_tf.translation.lerp(Vec3::ZERO, time.delta_secs() * 10.0);
    }
}

/// Dynamic floaty hands with physics momentum, viewmodel sway, and spring-damped inertia
pub fn floaty_hands_system(
    time: Option<Res<Time>>,
    mouse_motion: Option<Res<AccumulatedMouseMotion>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_player: Query<&ElfMovement, With<ElfPlayer>>,
    mut q_hands: Query<(&mut Transform, &mut FloatyHand)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Some(time) = time else {
        return;
    };
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    // Camera turning inertial sway: opposite direction to view rotation
    let mouse_delta = mouse_motion.map(|m| m.delta).unwrap_or(Vec2::ZERO);
    let sway_impulse = Vec3::new(
        -mouse_delta.x * 0.0012,
        mouse_delta.y * 0.0012,
        0.0,
    );

    // Player vertical momentum: dipping downward on jump, compression on landing
    let player_vel = q_player.single().map(|m| m.velocity).unwrap_or(Vec3::ZERO);
    let vert_dip = (-player_vel.y * 0.015).clamp(-0.08, 0.08);

    let t = time.elapsed_secs();

    for (mut tf, mut hand) in &mut q_hands {
        // Apply impulses into velocity
        hand.velocity += sway_impulse * 30.0;
        hand.velocity.y += vert_dip * 5.0;

        // Damped harmonic oscillator: F_spring = -k * x, F_damping = -c * v
        let spring_force = -hand.spring_stiffness * hand.current_offset;
        let damping_force = -hand.damping * hand.velocity;
        let accel = spring_force + damping_force;

        hand.velocity += accel * dt;
        let v = hand.velocity;
        hand.current_offset += v * dt;

        // Safety clamp on offset
        hand.current_offset.x = hand.current_offset.x.clamp(-0.16, 0.16);
        hand.current_offset.y = hand.current_offset.y.clamp(-0.14, 0.14);
        hand.current_offset.z = hand.current_offset.z.clamp(-0.12, 0.12);

        // Floaty idle breathing motion
        let idle_x = (t * 2.0 + hand.phase).sin() * 0.004;
        let idle_y = (t * 3.0 + hand.phase).cos() * 0.005;

        tf.translation = hand.rest_position + hand.current_offset + Vec3::new(idle_x, idle_y, 0.0);
    }
}
