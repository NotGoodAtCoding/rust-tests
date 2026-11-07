use std::f32::consts::FRAC_PI_2;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
};
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
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
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let hand_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.7, 0.3), // Elf green glove
        perceptual_roughness: 0.7,
        ..default()
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
            Collider::capsule_y(0.5, 0.3),
            KinematicCharacterController {
                offset: CharacterLength::Absolute(0.01),
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
                            Mesh3d(meshes.add(Sphere::new(0.12))),
                            MeshMaterial3d(hand_material.clone()),
                            Transform::from_xyz(-0.35, -0.3, -0.6),
                            LeftHandVisual,
                            Name::new("LeftHandGlove"),
                        ));

                        // Right hand model (controlled with E)
                        vm.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.12))),
                            MeshMaterial3d(hand_material.clone()),
                            Transform::from_xyz(0.35, -0.3, -0.6),
                            RightHandVisual,
                            Name::new("RightHandGlove"),
                        ));
                    });
                });
        });
}

pub fn elf_look_system(
    menu_state: Option<Res<MenuState>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    settings: Res<ElfInputSettings>,
    mut q_pivot: Query<(&mut Transform, &mut CameraPivot)>,
    mut q_player: Query<&mut Transform, (With<ElfPlayer>, Without<CameraPivot>)>,
) {
    if let Some(ref menu) = menu_state {
        if menu.is_open {
            return;
        }
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
    time: Option<Res<Time>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    bhop_config: Res<BhopConfig>,
    q_conveyor: Query<(&Transform, &ConveyorBelt)>,
    mut q_player: Query<
        (
            &Transform,
            &mut ElfMovement,
            &mut KinematicCharacterController,
            Option<&KinematicCharacterControllerOutput>,
        ),
        With<ElfPlayer>,
    >,
) {
    if let Some(ref menu) = menu_state {
        if menu.is_open {
            for (_, mut movement, mut controller, _) in &mut q_player {
                controller.translation = Some(Vec3::ZERO);
                movement.velocity = Vec3::ZERO;
            }
            return;
        }
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

    for (transform, mut movement, mut controller, output_opt) in &mut q_player {
        if let Some(output) = output_opt {
            movement.is_grounded = output.grounded;
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

pub fn viewmodel_bob(
    menu_state: Option<Res<MenuState>>,
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

    if let Some(ref menu) = menu_state {
        if menu.is_open {
            vm_tf.translation = vm_tf.translation.lerp(Vec3::ZERO, time.delta_secs() * 10.0);
            return;
        }
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
