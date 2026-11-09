use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    player::{ElfHands, ElfMovement, controller::{LeftHandVisual, RightHandVisual}},
    physics_sandbox::{SoftBodyLattice, SoftBodyDeformableMesh},
    wrapping::{Present, WrappingSpecification, WrappingTool, UseToolOnPresentMessage, PaperPatternRoll, GiftTagPenTool, ToyType},
};

pub const GROUP_PLAYER: Group = Group::GROUP_1;
pub const GROUP_HELD_ITEM: Group = Group::GROUP_2;
pub const GROUP_ELF: Group = Group::GROUP_3;
pub const GROUP_RAMP: Group = Group::GROUP_4;
pub const GROUP_STAIRS: Group = Group::GROUP_5;

#[derive(Component)]
pub struct Grabbable;

#[derive(Component)]
pub struct HeldByPlayer;

#[derive(Component)]
pub struct ToolItem {
    pub tool: WrappingTool,
}

/// Bulky/heavy physics items that require both hands to carry stably without slipping, sagging, or tilting.
#[derive(Component, Debug, Clone)]
pub struct HeavyItem {
    pub mass: f32,
    pub slip_rate: f32,
    pub max_single_hand_duration: f32,
    pub single_hand_timer: f32,
    pub drag_tilt_angle: f32,
    pub is_two_handed: bool,
}

impl Default for HeavyItem {
    fn default() -> Self {
        Self {
            mass: 18.0,
            slip_rate: 0.35,
            max_single_hand_duration: 2.5,
            single_hand_timer: 0.0,
            drag_tilt_angle: 0.38,
            is_two_handed: false,
        }
    }
}

impl HeavyItem {
    pub fn new(mass: f32, slip_rate: f32, max_duration: f32) -> Self {
        Self {
            mass,
            slip_rate,
            max_single_hand_duration: max_duration,
            single_hand_timer: 0.0,
            drag_tilt_angle: 0.38,
            is_two_handed: false,
        }
    }
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
        crate::elf_game::player::items::ItemClass::HandheldTool,
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
        crate::elf_game::player::items::ItemClass::HandheldTool,
        ToolItem { tool: WrappingTool::TapeDispenser },
        Name::new("WorkbenchTapeDispenser"),
    ));

    // 2b. Gift Tag Pen on Workbench Table (X = -0.85, Y = 0.95, Z = -4.0)
    let pen_mat = workshop_mats.as_ref().map(|wm| wm.polished_brass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.65, 0.15),
            perceptual_roughness: 0.3,
            ..default()
        })
    });
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.025, 0.28))),
        MeshMaterial3d(pen_mat),
        Transform::from_xyz(-0.85, 0.95, -4.0),
        Collider::cylinder(0.14, 0.025),
        RigidBody::Dynamic,
        Velocity::default(),
        Grabbable,
        crate::elf_game::player::items::ItemClass::HandheldTool,
        ToolItem { tool: WrappingTool::GiftTagPen },
        GiftTagPenTool::default(),
        Name::new("WorkbenchGiftTagPen"),
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
        crate::elf_game::player::items::ItemClass::HandheldTool,
        ToolItem { tool: WrappingTool::WrappingPaperRoll },
        PaperPatternRoll::default(),
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
        crate::elf_game::player::items::ItemClass::HandheldTool,
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
        ToyType::WoodenTrain,
        Present::new(ToyType::WoodenTrain),
        WrappingSpecification::for_toy(ToyType::WoodenTrain),
        Name::new("WorkbenchToyPresent"),
    ));
    if let Some(def) = deformable {
        entity_cmds.insert(def);
    }

    // 6. Bunny Ears Item placed on workbench (X = 1.8, Y = 0.95, Z = -4.2)
    crate::elf_game::player::bunny_ears::spawn_bunny_ears_item(
        &mut commands,
        &mut meshes,
        &mut materials,
        workshop_mats.as_deref(),
        Vec3::new(1.8, 0.95, -4.2),
    );

    // 7. Auto Wrapper Tool placed on workbench (X = -1.8, Y = 0.95, Z = -4.2)
    crate::elf_game::wrapping::spawn_auto_wrapper_tool(
        &mut commands,
        Some(&mut meshes),
        Some(&mut materials),
        workshop_mats.as_deref(),
        Vec3::new(-1.8, 0.95, -4.2),
    );

    // 8. Tinsel Hand Cannon placed on workbench (X = -1.5, Y = 0.95, Z = -4.2)
    crate::elf_game::player::tinsel_cannon::spawn_tinsel_hand_cannon(
        &mut commands,
        Some(&mut meshes),
        Some(&mut materials),
        workshop_mats.as_deref(),
        Vec3::new(-1.5, 0.95, -4.2),
    );
}

/// Finds the best grabbable object in direct raycasted line of sight with the player's reticle.
/// Prioritizes objects directly pierced by the reticle raycast (ordered by distance along ray to respect line-of-sight),
/// rejects objects occluded by solid obstacles, and falls back to off-center crosshair alignment only if no direct hit exists.
fn find_grabbable_in_reticle_los(
    ray_origin: Vec3,
    ray_dir: Vec3,
    max_reach: f32,
    excluded_entity: Option<Entity>,
    q_grabbables: &Query<(Entity, &mut Transform, Option<&Collider>, Option<&ToolItem>), (With<Grabbable>, Without<ElfHands>)>,
    q_obstacles: &Query<(&Transform, &Collider), (Without<Grabbable>, Without<ElfHands>, Without<ElfMovement>, Without<Sensor>)>,
) -> Option<Entity> {
    let mut best_candidate: Option<(Entity, u8, f32)> = None;

    for (entity, tf, opt_collider, _) in q_grabbables {
        if Some(entity) == excluded_entity {
            continue;
        }

        let obj_pos = tf.translation;
        let to_obj = obj_pos - ray_origin;
        let t_proj = to_obj.dot(ray_dir);
        if t_proj <= 0.0 || t_proj > max_reach {
            continue;
        }

        // 1. Direct raycast check against collider if available
        let mut direct_hit: Option<f32> = None;
        if let Some(collider) = opt_collider {
            if let Some(toi) = collider.cast_ray(obj_pos, tf.rotation, ray_origin, ray_dir, max_reach, true) {
                direct_hit = Some(toi);
            }
        }

        // 2. Direct raycast check against bounding sphere (radius 0.35m)
        let d_perp_sq = to_obj.length_squared() - t_proj * t_proj;
        if direct_hit.is_none() {
            let radius = 0.35;
            if d_perp_sq <= radius * radius {
                let half_chord = (radius * radius - d_perp_sq).max(0.0).sqrt();
                direct_hit = Some((t_proj - half_chord).max(0.0));
            }
        }

        if let Some(hit_toi) = direct_hit {
            // Line of sight check: verify no solid obstacle intersects the ray before the grabbable object
            let is_occluded = q_obstacles.iter().any(|(obs_tf, obs_col)| {
                obs_col.cast_ray(obs_tf.translation, obs_tf.rotation, ray_origin, ray_dir, hit_toi, true)
                    .map(|obs_toi| obs_toi < hit_toi - 0.02)
                    .unwrap_or(false)
            });

            if !is_occluded {
                // Tier 0: Direct raycasted line-of-sight hit, prioritized by distance along the ray
                let is_better = match best_candidate {
                    None => true,
                    Some((_, tier, dist)) => tier > 0 || hit_toi < dist,
                };
                if is_better {
                    best_candidate = Some((entity, 0, hit_toi));
                }
                continue;
            }
        }

        // 3. Fallback: within crosshair view window (d_perp <= 0.75m), only if no direct raycast hit exists
        if d_perp_sq <= 0.75 * 0.75 {
            let is_occluded = q_obstacles.iter().any(|(obs_tf, obs_col)| {
                obs_col.cast_ray(obs_tf.translation, obs_tf.rotation, ray_origin, ray_dir, t_proj, true)
                    .map(|obs_toi| obs_toi < t_proj - 0.05)
                    .unwrap_or(false)
            });

            if !is_occluded {
                let d_perp = d_perp_sq.sqrt();
                let is_better = match best_candidate {
                    None => true,
                    Some((_, tier, dist)) => tier > 1 || (tier == 1 && d_perp < dist),
                };
                if is_better {
                    best_candidate = Some((entity, 1, d_perp));
                }
            }
        }
    }

    best_candidate.map(|(entity, _, _)| entity)
}

/// Handles Left Hand (Q) and Right Hand (E) toggle grabbing, setting down, and Mouse 1 throwing
pub fn dual_hand_interaction_system(
    inputs: (Option<Res<ButtonInput<KeyCode>>>, Option<Res<ButtonInput<MouseButton>>>),
    states: (
        Option<Res<crate::elf_game::ui::MenuState>>,
        Option<Res<crate::elf_game::dialogue::ActiveDialogue>>,
        Option<Res<State<crate::elf_game::core::GameState>>>,
        Option<Res<Time>>,
    ),
    mut q_player: Query<(&Transform, &mut ElfHands, Option<&ElfMovement>), Without<Grabbable>>,
    q_camera: Query<&GlobalTransform, With<crate::elf_game::player::controller::FirstPersonCamera>>,
    q_left_visual: Query<&GlobalTransform, With<LeftHandVisual>>,
    q_right_visual: Query<&GlobalTransform, With<RightHandVisual>>,
    mut q_grabbables: Query<(Entity, &mut Transform, Option<&Collider>, Option<&ToolItem>), (With<Grabbable>, Without<ElfHands>)>,
    q_obstacles: Query<(&Transform, &Collider), (Without<Grabbable>, Without<ElfHands>, Without<ElfMovement>, Without<Sensor>)>,
    mut q_velocities: Query<&mut Velocity, With<Grabbable>>,
    q_presents: Query<Entity, With<Present>>,
    mut q_rolls: Query<&mut PaperPatternRoll>,
    q_pens: Query<&GiftTagPenTool>,
    mut q_heavy: Query<&mut HeavyItem>,
    mut tool_messages: MessageWriter<UseToolOnPresentMessage>,
    mut commands: Commands,
) {
    let (keys, mouse_buttons) = inputs;
    let (menu_state, dialogue_state, game_state, time_opt) = states;
    let dt = time_opt.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);

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
            let r = hands.right_grabbed.take();
            if hands.left_grabbed == r {
                hands.left_grabbed = None;
            }
            r
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

    let (ray_origin, ray_dir) = if let Some(cam_gt) = q_camera.iter().next() {
        (cam_gt.translation(), cam_gt.forward().into())
    } else {
        (_player_tf.translation + Vec3::new(0.0, 0.6, 0.0), _player_tf.rotation * Vec3::NEG_Z)
    };
    let max_reach = 2.8;

    // 2. Left Hand Toggle Grab & Set Down (KeyQ)
    if left_toggle {
        if let Some(held_e) = hands.left_grabbed {
            hands.left_grabbed = None;
            if hands.right_grabbed != Some(held_e) {
                commands.entity(held_e).remove::<(HeldByPlayer, Sensor)>();
                commands.entity(held_e).insert(CollisionGroups::new(Group::ALL, Group::ALL));
                if let Ok(mut vel) = q_velocities.get_mut(held_e) {
                    vel.linear = Vec3::ZERO;
                    vel.angular = Vec3::ZERO;
                }
            }
        } else {
            // Find object in reticle LOS. If right hand holds a HeavyItem, don't exclude it so we can two-hand grab!
            let excluded = if let Some(r_e) = hands.right_grabbed {
                if q_heavy.contains(r_e) {
                    None
                } else {
                    Some(r_e)
                }
            } else {
                None
            };
            if let Some(target_e) = find_grabbable_in_reticle_los(
                ray_origin,
                ray_dir,
                max_reach,
                excluded,
                &q_grabbables,
                &q_obstacles,
            ) {
                hands.left_grabbed = Some(target_e);
                commands.entity(target_e).insert((
                    HeldByPlayer,
                    Sensor,
                    CollisionGroups::new(GROUP_HELD_ITEM, !GROUP_PLAYER),
                ));
                if let Ok(mut vel) = q_velocities.get_mut(target_e) {
                    vel.linear = Vec3::ZERO;
                    vel.angular = Vec3::ZERO;
                }
            }
        }
    }

    // 3. Right Hand Toggle Grab & Set Down (KeyE)
    if right_toggle {
        if let Some(held_e) = hands.right_grabbed {
            hands.right_grabbed = None;
            if hands.left_grabbed != Some(held_e) {
                commands.entity(held_e).remove::<(HeldByPlayer, Sensor)>();
                commands.entity(held_e).insert(CollisionGroups::new(Group::ALL, Group::ALL));
                if let Ok(mut vel) = q_velocities.get_mut(held_e) {
                    vel.linear = Vec3::ZERO;
                    vel.angular = Vec3::ZERO;
                }
            }
        } else {
            let excluded = if let Some(l_e) = hands.left_grabbed {
                if q_heavy.contains(l_e) {
                    None
                } else {
                    Some(l_e)
                }
            } else {
                None
            };
            if let Some(target_e) = find_grabbable_in_reticle_los(
                ray_origin,
                ray_dir,
                max_reach,
                excluded,
                &q_grabbables,
                &q_obstacles,
            ) {
                hands.right_grabbed = Some(target_e);
                commands.entity(target_e).insert((
                    HeldByPlayer,
                    Sensor,
                    CollisionGroups::new(GROUP_HELD_ITEM, !GROUP_PLAYER),
                ));
                if let Ok(mut vel) = q_velocities.get_mut(target_e) {
                    vel.linear = Vec3::ZERO;
                    vel.angular = Vec3::ZERO;
                }
            }
        }
    }

    // Cycle paper roll pattern if holding roll and pressing KeyR or KeyF
    if let Some(ref k) = keys {
        if k.just_pressed(KeyCode::KeyR) || k.just_pressed(KeyCode::KeyF) {
            for held_opt in [hands.left_grabbed, hands.right_grabbed] {
                if let Some(held_e) = held_opt {
                    if let Ok(mut roll) = q_rolls.get_mut(held_e) {
                        roll.cycle();
                    }
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

    let both_hold_same = match (hands.left_grabbed, hands.right_grabbed) {
        (Some(l), Some(r)) => l == r,
        _ => false,
    };

    if both_hold_same {
        let held_e = hands.left_grabbed.unwrap();
        let center_hold_pos = (left_hold_pos + right_hold_pos) * 0.5;
        if let Ok((_, mut tf, _, _)) = q_grabbables.get_mut(held_e) {
            tf.translation = center_hold_pos;
            tf.rotation = camera_rot;
        }
        if let Ok(mut vel) = q_velocities.get_mut(held_e) {
            vel.linear = Vec3::ZERO;
            vel.angular = Vec3::ZERO;
        }
        if let Ok(mut heavy) = q_heavy.get_mut(held_e) {
            heavy.is_two_handed = true;
            heavy.single_hand_timer = 0.0;
        }
    } else {
        if let Some(left_e) = hands.left_grabbed {
            let mut left_slipped_out = false;
            if let Ok(mut heavy) = q_heavy.get_mut(left_e) {
                heavy.is_two_handed = false;
                heavy.single_hand_timer += dt;
                if heavy.single_hand_timer >= heavy.max_single_hand_duration {
                    left_slipped_out = true;
                }
            }

            if left_slipped_out {
                hands.left_grabbed = None;
                commands.entity(left_e).remove::<(HeldByPlayer, Sensor)>();
                commands.entity(left_e).insert(CollisionGroups::new(Group::ALL, Group::ALL));
                if let Ok(mut vel) = q_velocities.get_mut(left_e) {
                    vel.linear = Vec3::new(0.0, -2.5, 0.0);
                    vel.angular = Vec3::new(1.0, 0.5, -0.8);
                }
                if let Ok(mut heavy) = q_heavy.get_mut(left_e) {
                    heavy.single_hand_timer = 0.0;
                }
            } else {
                let left_tool = if let Ok((_, mut tf, _, tool_opt)) = q_grabbables.get_mut(left_e) {
                    let mut hold_pos = left_hold_pos;
                    let mut rot = camera_rot;
                    if let Ok(heavy) = q_heavy.get(left_e) {
                        let sag = (heavy.single_hand_timer * heavy.slip_rate).min(0.40);
                        hold_pos.y -= sag;
                        let tilt = Quat::from_rotation_z(-heavy.drag_tilt_angle) * Quat::from_rotation_x(0.20);
                        rot = camera_rot * tilt;
                    }
                    tf.translation = hold_pos;
                    tf.rotation = rot;
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
                    let pattern = q_rolls.get(left_e).ok().map(|r| r.pattern);
                    let inscription = q_pens.get(left_e).ok().map(|p| p.inscription.clone());
                    for present_e in &q_presents {
                        if let Ok((_, p_tf, _, _)) = q_grabbables.get(present_e) {
                            if p_tf.translation.distance(left_hold_pos) < 1.0 {
                                tool_messages.write(UseToolOnPresentMessage {
                                    tool,
                                    present_entity: present_e,
                                    pattern,
                                    inscription: inscription.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if let Some(right_e) = hands.right_grabbed {
            let mut right_slipped_out = false;
            if let Ok(mut heavy) = q_heavy.get_mut(right_e) {
                heavy.is_two_handed = false;
                heavy.single_hand_timer += dt;
                if heavy.single_hand_timer >= heavy.max_single_hand_duration {
                    right_slipped_out = true;
                }
            }

            if right_slipped_out {
                hands.right_grabbed = None;
                commands.entity(right_e).remove::<(HeldByPlayer, Sensor)>();
                commands.entity(right_e).insert(CollisionGroups::new(Group::ALL, Group::ALL));
                if let Ok(mut vel) = q_velocities.get_mut(right_e) {
                    vel.linear = Vec3::new(0.0, -2.5, 0.0);
                    vel.angular = Vec3::new(1.0, 0.5, -0.8);
                }
                if let Ok(mut heavy) = q_heavy.get_mut(right_e) {
                    heavy.single_hand_timer = 0.0;
                }
            } else {
                let right_tool = if let Ok((_, mut tf, _, tool_opt)) = q_grabbables.get_mut(right_e) {
                    let mut hold_pos = right_hold_pos;
                    let mut rot = camera_rot;
                    if let Ok(heavy) = q_heavy.get(right_e) {
                        let sag = (heavy.single_hand_timer * heavy.slip_rate).min(0.40);
                        hold_pos.y -= sag;
                        let tilt = Quat::from_rotation_z(heavy.drag_tilt_angle) * Quat::from_rotation_x(0.20);
                        rot = camera_rot * tilt;
                    }
                    tf.translation = hold_pos;
                    tf.rotation = rot;
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
                    let pattern = q_rolls.get(right_e).ok().map(|r| r.pattern);
                    let inscription = q_pens.get(right_e).ok().map(|p| p.inscription.clone());
                    for present_e in &q_presents {
                        if let Ok((_, p_tf, _, _)) = q_grabbables.get(present_e) {
                            if p_tf.translation.distance(right_hold_pos) < 1.0 {
                                tool_messages.write(UseToolOnPresentMessage {
                                    tool,
                                    present_entity: present_e,
                                    pattern,
                                    inscription: inscription.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}
