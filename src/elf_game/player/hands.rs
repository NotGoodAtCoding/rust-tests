use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    player::{ElfHands, controller::{LeftHandVisual, RightHandVisual}},
    wrapping::{Present, WrappingSpecification, WrappingTool, UseToolOnPresentMessage},
};

#[derive(Component)]
pub struct Grabbable;

#[derive(Component)]
pub struct ToolItem {
    pub tool: WrappingTool,
}

pub fn spawn_workshop_tools_and_materials(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let metal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.8, 0.85),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    let paper_roll_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 0.2), // Red wrapping paper
        perceptual_roughness: 0.5,
        ..default()
    });

    let bow_box_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.8, 0.1), // Gold ribbon box
        metallic: 0.5,
        perceptual_roughness: 0.3,
        ..default()
    });

    // 1. Scissors on Workbench Table (X = -1.2, Y = 0.9, Z = -4.0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.3, 0.05, 0.6))),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_xyz(-1.2, 0.9, -4.0),
        Collider::cuboid(0.15, 0.05, 0.3),
        RigidBody::Dynamic,
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
        Grabbable,
        ToolItem { tool: WrappingTool::DecorativeBowBox },
        Name::new("DecorativeBowBox"),
    ));

    // 5. Initial Unwrapped Toy on Center Table (Wooden Train)
    let toy_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.5, 0.8), // Blue painted toy
        perceptual_roughness: 0.4,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.4, 0.8))),
        MeshMaterial3d(toy_mat),
        Transform::from_xyz(0.0, 1.0, -3.8),
        Collider::cuboid(0.3, 0.2, 0.4),
        RigidBody::Dynamic,
        Grabbable,
        Present::default(),
        WrappingSpecification::default(),
        Name::new("WorkbenchToyPresent"),
    ));
}

/// Handles Left Hand (Q) and Right Hand (E) grabbing, holding, and tool triggering
pub fn dual_hand_interaction_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<(&Transform, &mut ElfHands)>,
    q_left_visual: Query<&GlobalTransform, With<LeftHandVisual>>,
    q_right_visual: Query<&GlobalTransform, With<RightHandVisual>>,
    mut q_grabbables: Query<(Entity, &mut Transform, Option<&ToolItem>), (With<Grabbable>, Without<ElfHands>)>,
    q_presents: Query<Entity, With<Present>>,
    mut tool_messages: MessageWriter<UseToolOnPresentMessage>,
) {
    let Ok((_player_tf, mut hands)) = q_player.single_mut() else {
        return;
    };

    let left_hand_pos = q_left_visual.single().map(|gt| gt.translation()).unwrap_or(Vec3::ZERO);
    let right_hand_pos = q_right_visual.single().map(|gt| gt.translation()).unwrap_or(Vec3::ZERO);

    let left_grab_pressed = keys.pressed(KeyCode::KeyQ);
    let right_grab_pressed = keys.pressed(KeyCode::KeyE);

    // Left hand grab management
    if left_grab_pressed {
        if hands.left_grabbed.is_none() {
            // Find closest grabbable within reach (1.8m)
            let mut closest: Option<(Entity, f32)> = None;
            for (entity, tf, _) in &q_grabbables {
                let dist = tf.translation.distance(left_hand_pos);
                if dist < 1.8 && (closest.is_none() || dist < closest.unwrap().1) {
                    closest = Some((entity, dist));
                }
            }
            if let Some((e, _)) = closest {
                hands.left_grabbed = Some(e);
            }
        }
    } else {
        hands.left_grabbed = None;
    }

    // Right hand grab management
    if right_grab_pressed {
        if hands.right_grabbed.is_none() {
            let mut closest: Option<(Entity, f32)> = None;
            for (entity, tf, _) in &q_grabbables {
                let dist = tf.translation.distance(right_hand_pos);
                if dist < 1.8 && (closest.is_none() || dist < closest.unwrap().1) {
                    closest = Some((entity, dist));
                }
            }
            if let Some((e, _)) = closest {
                hands.right_grabbed = Some(e);
            }
        }
    } else {
        hands.right_grabbed = None;
    }

    // Hold grabbed items in position with kinematic follow
    if let Some(left_e) = hands.left_grabbed {
        let left_tool = if let Ok((_, mut tf, tool_opt)) = q_grabbables.get_mut(left_e) {
            tf.translation = left_hand_pos;
            tool_opt.map(|t| t.tool)
        } else {
            None
        };

        if let Some(tool) = left_tool {
            for present_e in &q_presents {
                if let Ok((_, p_tf, _)) = q_grabbables.get(present_e) {
                    if p_tf.translation.distance(left_hand_pos) < 0.8 {
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
            tf.translation = right_hand_pos;
            tool_opt.map(|t| t.tool)
        } else {
            None
        };

        if let Some(tool) = right_tool {
            for present_e in &q_presents {
                if let Ok((_, p_tf, _)) = q_grabbables.get(present_e) {
                    if p_tf.translation.distance(right_hand_pos) < 0.8 {
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
