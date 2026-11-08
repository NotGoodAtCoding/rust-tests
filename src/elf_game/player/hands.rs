use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    player::{ElfHands, ElfMovement, controller::{LeftHandVisual, RightHandVisual}},
    physics_sandbox::{SoftBodyLattice, SoftBodyDeformableMesh},
    wrapping::{Present, WrappingSpecification, WrappingTool, UseToolOnPresentMessage},
};

pub const GROUP_PLAYER: Group = Group::GROUP_1;
pub const GROUP_HELD_ITEM: Group = Group::GROUP_2;

#[derive(Component)]
pub struct Grabbable;

#[derive(Component)]
pub struct HeldByPlayer;

#[derive(Component)]
pub struct ToolItem {
    pub tool: WrappingTool,
}

pub fn spawn_workshop_tools_and_materials(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let metal_mat = workshop_mats.as_ref().map(|wm| wm.sheet_metal.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SHEET_METAL,
            metallic: 0.9,
            perceptual_roughness: 0.2,
            ..default()
        })
    });

    let paper_roll_mat = workshop_mats.as_ref().map(|wm| wm.festive_red.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: FESTIVE_RED,
            perceptual_roughness: 0.5,
            ..default()
        })
    });

    let bow_box_mat = workshop_mats.as_ref().map(|wm| wm.gold_ribbon.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: GOLD_RIBBON,
            metallic: 0.5,
            perceptual_roughness: 0.3,
            ..default()
        })
    });

    // 1. Scissors on Workbench Table (X = -1.2, Y = 0.9, Z = -4.0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.3, 0.05, 0.6))),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_xyz(-1.2, 0.9, -4.0),
        Collider::cuboid(0.15, 0.05, 0.3),
        RigidBody::Dynamic,
        Velocity::default(),
        Grabbable,
        ToolItem { tool: WrappingTool::Scissors },
        Name::new("WorkbenchScissors"),
    ));

    // 2. Tape Dispenser on Workbench Table (X = -0.5, Y = 0.95, Z = -4.0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.25, 0.3))),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_xyz(-0.5, 0.95, -4.0),
        Collider::cuboid(0.2, 0.12, 0.15),
        RigidBody::Dynamic,
        Velocity::default(),
        Grabbable,
        ToolItem { tool: WrappingTool::TapeDispenser },
        Name::new("WorkbenchTapeDispenser"),
    ));

    // 3. Wrapping Paper Roll on Workbench Table (X = 0.5, Y = 0.95, Z = -4.0)
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.12, 1.2))),
        MeshMaterial3d(paper_roll_mat),
        Transform::from_xyz(0.5, 0.95, -4.0).with_rotation(Quat::from_rotation_z(1.5708)),
        Collider::cylinder(0.6, 0.12),
        RigidBody::Dynamic,
        Velocity::default(),
        Grabbable,
        ToolItem { tool: WrappingTool::WrappingPaperRoll },
        Name::new("WrappingPaperRoll"),
    ));

    // 4. Decorative Bow Box on Workbench (X = 1.2, Y = 0.95, Z = -4.0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.35, 0.25, 0.35))),
        MeshMaterial3d(bow_box_mat),
        Transform::from_xyz(1.2, 0.95, -4.0),
        Collider::cuboid(0.17, 0.12, 0.17),
        RigidBody::Dynamic,
        Velocity::default(),
        Grabbable,
        ToolItem { tool: WrappingTool::DecorativeBowBox },
        Name::new("DecorativeBowBox"),
    ));

    // 5. Initial Unwrapped Toy on Center Table (Wooden Train)
    let toy_mat = workshop_mats.as_ref().map(|wm| wm.toy_blue.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: UNWRAPPED_TOY_BLUE,
            perceptual_roughness: 0.4,
            ..default()
        })
    });

    let raw_mesh = Mesh::from(Cuboid::new(0.6, 0.4, 0.8));
    let deformable = SoftBodyDeformableMesh::from_mesh(&raw_mesh, Vec3::new(0.3, 0.2, 0.4));
    let mesh_handle = meshes.add(raw_mesh);

    let mut entity_cmds = commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(toy_mat),
        Transform::from_xyz(0.0, 1.0, -3.8),
        Collider::cuboid(0.3, 0.2, 0.4),
        RigidBody::Dynamic,
        Velocity::default(),
        ExternalImpulse::default(),
        SoftBodyLattice::new_cube(Vec3::new(0.3, 0.2, 0.4), 60.0, 6.0),
        Grabbable,
        Present::default(),
        WrappingSpecification::default(),
        Name::new("WorkbenchToyPresent"),
    ));
    if let Some(def) = deformable {
        entity_cmds.insert(def);
    }
}

/// Handles Left Hand (Q) and Right Hand (E) toggle grabbing, setting down, and Mouse 1 throwing
pub fn dual_hand_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    dialogue_state: Option<Res<crate::elf_game::dialogue::ActiveDialogue>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_player: Query<(&Transform, &mut ElfHands, Option<&ElfMovement>), Without<Grabbable>>,
    q_camera: Query<&GlobalTransform, With<crate::elf_game::player::controller::FirstPersonCamera>>,
    q_left_visual: Query<&GlobalTransform, With<LeftHandVisual>>,
    q_right_visual: Query<&GlobalTransform, With<RightHandVisual>>,
    mut q_grabbables: Query<(Entity, &mut Transform, Option<&ToolItem>), (With<Grabbable>, Without<ElfHands>)>,
    mut q_velocities: Query<&mut Velocity, With<Grabbable>>,
    q_presents: Query<Entity, With<Present>>,
    mut tool_messages: MessageWriter<UseToolOnPresentMessage>,
    mut commands: Commands,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    // Suppress hands interaction during elf dialogue so Q, E, and Mouse1 select options
    if dialogue_state.as_ref().map(|d| d.is_active).unwrap_or(false) {
        return;
    }

    let Ok((_player_tf, mut hands, movement_opt)) = q_player.single_mut() else {
        return;
    };

    let player_vel = movement_opt
        .map(|m| {
            if m.is_grounded && m.velocity.y < 0.0 {
                Vec3::new(m.velocity.x, 0.0, m.velocity.z)
            } else {
                m.velocity
            }
        })
        .unwrap_or(Vec3::ZERO);

    let left_hand_pos = q_left_visual.single().map(|gt| gt.translation()).unwrap_or(Vec3::ZERO);
    let right_hand_pos = q_right_visual.single().map(|gt| gt.translation()).unwrap_or(Vec3::ZERO);

    let left_toggle = keys.as_ref().map(|k| k.just_pressed(KeyCode::KeyQ)).unwrap_or(false);
    let right_toggle = keys.as_ref().map(|k| k.just_pressed(KeyCode::KeyE)).unwrap_or(false);
    let throw_clicked = mouse_buttons.as_ref().map(|m| m.just_pressed(MouseButton::Left)).unwrap_or(false);

    // 1. Mouse 1 Throwing: Throws held object forward along camera look vector,
    // inheriting the player's current velocity (enabling jump-throws to fling items further)
    if throw_clicked {
        let thrown_entity_opt = if hands.right_grabbed.is_some() {
            hands.right_grabbed.take()
        } else if hands.left_grabbed.is_some() {
            hands.left_grabbed.take()
        } else {
            None
        };

        if let Some(thrown_e) = thrown_entity_opt {
            commands.entity(thrown_e).remove::<(HeldByPlayer, Sensor)>();
            commands.entity(thrown_e).insert(CollisionGroups::new(Group::ALL, Group::ALL));

            let camera_fwd = q_camera.single().map(|gt| gt.forward().into()).unwrap_or(Vec3::NEG_Z);
            let throw_dir = (camera_fwd + Vec3::Y * 0.15).normalize();
            let throw_speed = 13.5;
            let base_throw_vel = throw_dir * throw_speed;
            let throw_vel = base_throw_vel + player_vel;
            let tumble = Vec3::new(2.5, 1.2, -1.8);

            if let Ok(mut vel) = q_velocities.get_mut(thrown_e) {
                vel.linear = throw_vel;
                vel.angular = tumble;
            } else {
                commands.entity(thrown_e).insert(Velocity {
                    linear: throw_vel,
                    angular: tumble,
                });
            }
        }
    }

    // 2. Left Hand Toggle Grab & Set Down (KeyQ)
    if left_toggle {
        if let Some(held_e) = hands.left_grabbed {
            // Setting down: release gently with zero velocity at hand location
            hands.left_grabbed = None;
            commands.entity(held_e).remove::<(HeldByPlayer, Sensor)>();
            commands.entity(held_e).insert(CollisionGroups::new(Group::ALL, Group::ALL));
            if let Ok(mut vel) = q_velocities.get_mut(held_e) {
                vel.linear = Vec3::ZERO;
                vel.angular = Vec3::ZERO;
            }
        } else {
            // Find closest grabbable within reach (1.8m) not held by right hand
            let mut closest: Option<(Entity, f32)> = None;
            for (entity, tf, _) in &q_grabbables {
                if Some(entity) == hands.right_grabbed {
                    continue;
                }
                let dist = tf.translation.distance(left_hand_pos);
                if dist < 1.8 && (closest.is_none() || dist < closest.unwrap().1) {
                    closest = Some((entity, dist));
                }
            }
            if let Some((e, _)) = closest {
                hands.left_grabbed = Some(e);
                commands.entity(e).insert((
                    HeldByPlayer,
                    Sensor,
                    CollisionGroups::new(GROUP_HELD_ITEM, !GROUP_PLAYER),
                ));
                if let Ok(mut vel) = q_velocities.get_mut(e) {
                    vel.linear = Vec3::ZERO;
                    vel.angular = Vec3::ZERO;
                }
            }
        }
    }

    // 3. Right Hand Toggle Grab & Set Down (KeyE)
    if right_toggle {
        if let Some(held_e) = hands.right_grabbed {
            // Setting down: release gently with zero velocity at hand location
            hands.right_grabbed = None;
            commands.entity(held_e).remove::<(HeldByPlayer, Sensor)>();
            commands.entity(held_e).insert(CollisionGroups::new(Group::ALL, Group::ALL));
            if let Ok(mut vel) = q_velocities.get_mut(held_e) {
                vel.linear = Vec3::ZERO;
                vel.angular = Vec3::ZERO;
            }
        } else {
            // Find closest grabbable within reach (1.8m) not held by left hand
            let mut closest: Option<(Entity, f32)> = None;
            for (entity, tf, _) in &q_grabbables {
                if Some(entity) == hands.left_grabbed {
                    continue;
                }
                let dist = tf.translation.distance(right_hand_pos);
                if dist < 1.8 && (closest.is_none() || dist < closest.unwrap().1) {
                    closest = Some((entity, dist));
                }
            }
            if let Some((e, _)) = closest {
                hands.right_grabbed = Some(e);
                commands.entity(e).insert((
                    HeldByPlayer,
                    Sensor,
                    CollisionGroups::new(GROUP_HELD_ITEM, !GROUP_PLAYER),
                ));
                if let Ok(mut vel) = q_velocities.get_mut(e) {
                    vel.linear = Vec3::ZERO;
                    vel.angular = Vec3::ZERO;
                }
            }
        }
    }

    // 4. Hold grabbed items in position with kinematic follow, pushed out forward in front of hands
    let camera_fwd = q_camera.single().map(|gt| gt.forward().into()).unwrap_or(Vec3::NEG_Z);
    let camera_rot = q_camera.single().map(|gt| gt.to_scale_rotation_translation().1).unwrap_or(Quat::IDENTITY);

    let hold_forward_offset = 0.45;
    let left_hold_pos = left_hand_pos + camera_fwd * hold_forward_offset;
    let right_hold_pos = right_hand_pos + camera_fwd * hold_forward_offset;

    if let Some(left_e) = hands.left_grabbed {
        let left_tool = if let Ok((_, mut tf, tool_opt)) = q_grabbables.get_mut(left_e) {
            tf.translation = left_hold_pos;
            tf.rotation = camera_rot;
            tool_opt.map(|t| t.tool)
        } else {
            hands.left_grabbed = None;
            None
        };
        if let Ok(mut vel) = q_velocities.get_mut(left_e) {
            vel.linear = Vec3::ZERO;
            vel.angular = Vec3::ZERO;
        }

        if let Some(tool) = left_tool {
            for present_e in &q_presents {
                if let Ok((_, p_tf, _)) = q_grabbables.get(present_e) {
                    if p_tf.translation.distance(left_hold_pos) < 1.0 {
                        tool_messages.write(UseToolOnPresentMessage {
                            tool,
                            present_entity: present_e,
                        });
                    }
                }
            }
        }
    }

    if let Some(right_e) = hands.right_grabbed {
        let right_tool = if let Ok((_, mut tf, tool_opt)) = q_grabbables.get_mut(right_e) {
            tf.translation = right_hold_pos;
            tf.rotation = camera_rot;
            tool_opt.map(|t| t.tool)
        } else {
            hands.right_grabbed = None;
            None
        };
        if let Ok(mut vel) = q_velocities.get_mut(right_e) {
            vel.linear = Vec3::ZERO;
            vel.angular = Vec3::ZERO;
        }

        if let Some(tool) = right_tool {
            for present_e in &q_presents {
                if let Ok((_, p_tf, _)) = q_grabbables.get(present_e) {
                    if p_tf.translation.distance(right_hold_pos) < 1.0 {
                        tool_messages.write(UseToolOnPresentMessage {
                            tool,
                            present_entity: present_e,
                        });
                    }
                }
            }
        }
    }
}
