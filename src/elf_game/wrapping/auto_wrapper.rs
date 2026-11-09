use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    physics_sandbox::ragdoll::{ElfAssistant, ElfNpcState},
    player::{
        controller::{ElfPlayer, FirstPersonCamera},
        hands::{Grabbable, HeavyItem},
        items::ItemClass,
        ElfHands,
    },
    ui::MenuState,
    wrapping::{PaperPattern, Present, WrappingFeature, WrappingTool},
};

/// Handheld Auto Wrapper tool item that can wrap any physics object or elf in wrapping paper.
#[derive(Component, Debug, Clone)]
pub struct AutoWrapperTool {
    pub current_target: Option<Entity>,
    pub wrap_progress: f32, // 0.0 to 1.0
    pub required_duration: f32,
    pub is_wrapping: bool,
    pub base_duration: f32,
}

impl Default for AutoWrapperTool {
    fn default() -> Self {
        Self {
            current_target: None,
            wrap_progress: 0.0,
            required_duration: 1.5,
            is_wrapping: false,
            base_duration: 1.5,
        }
    }
}

/// Visual marker and state on an object or elf that has been wrapped in festive wrapping paper.
#[derive(Component, Debug, Clone)]
pub struct WrappedInPaper {
    pub pattern: PaperPattern,
    pub original_material: Option<Handle<StandardMaterial>>,
}

/// Component added to an elf that was wrapped by the Auto Wrapper tool.
/// Immobilizes them for 10 seconds in ragdoll mode with periodic slight bounces.
#[derive(Component, Debug, Clone)]
pub struct WrappedElf {
    pub remaining_time: f32, // 10.0 seconds
    pub bounce_timer: f32,   // periodic small bounces
    pub original_materials: Vec<(Entity, Handle<StandardMaterial>)>,
}

impl Default for WrappedElf {
    fn default() -> Self {
        Self {
            remaining_time: 10.0,
            bounce_timer: 1.2,
            original_materials: Vec::new(),
        }
    }
}

/// Calculates wrap duration based on physical dimensions / volume: larger items take longer.
pub fn calculate_wrap_duration(
    base_duration: f32,
    opt_collider: Option<&Collider>,
    opt_heavy: Option<&HeavyItem>,
    is_elf: bool,
) -> f32 {
    if is_elf {
        return (base_duration * 1.6).clamp(1.0, 6.0);
    }

    let mut volume = 0.064; // default 0.4m cube
    if let Some(col) = opt_collider {
        if let Some(cuboid) = col.as_cuboid() {
            let h = cuboid.half_extents();
            volume = (h.x * 2.0) * (h.y * 2.0) * (h.z * 2.0);
        } else if let Some(ball) = col.as_ball() {
            let r = ball.radius();
            volume = (4.0 / 3.0) * std::f32::consts::PI * r * r * r;
        } else if let Some(cyl) = col.as_cylinder() {
            let hh = cyl.half_height();
            let r = cyl.radius();
            volume = (hh * 2.0) * std::f32::consts::PI * r * r;
        }
    }

    if let Some(heavy) = opt_heavy {
        volume = volume.max(heavy.mass * 0.005);
    }

    // Scale duration proportionally to cube root of volume (linear dimension)
    let scale = (volume / 0.064).cbrt().max(0.4);
    (base_duration * scale).clamp(0.6, 6.0)
}

/// Spawns a physical Auto Wrapper tool in the workshop.
pub fn spawn_auto_wrapper_tool(
    commands: &mut Commands,
    meshes: Option<&mut Assets<Mesh>>,
    mut materials: Option<&mut Assets<StandardMaterial>>,
    wm: Option<&WorkshopMaterials>,
    pos: Vec3,
) -> Entity {
    let casing_mat = wm.map(|w| w.pine_green.clone()).unwrap_or_else(|| {
        materials
            .as_mut()
            .map(|m| {
                m.add(StandardMaterial {
                    base_color: PINE_GREEN,
                    metallic: 0.6,
                    perceptual_roughness: 0.3,
                    ..default()
                })
            })
            .unwrap_or_default()
    });

    let brass_mat = wm.map(|w| w.polished_brass.clone()).unwrap_or_else(|| {
        materials
            .as_mut()
            .map(|m| {
                m.add(StandardMaterial {
                    base_color: POLISHED_BRASS,
                    metallic: 0.8,
                    perceptual_roughness: 0.25,
                    ..default()
                })
            })
            .unwrap_or_default()
    });

    let tool_entity = commands
        .spawn((
            AutoWrapperTool::default(),
            Grabbable,
            ItemClass::HandheldTool,
            crate::elf_game::player::hands::ToolItem {
                tool: WrappingTool::AutoWrapper,
            },
            Transform::from_translation(pos),
            RigidBody::Dynamic,
            Collider::cuboid(0.12, 0.15, 0.25),
            Velocity::default(),
            Visibility::default(),
            Name::new("AutoWrapperGun"),
        ))
        .id();

    if let (Some(m), Some(_)) = (meshes, materials) {
        // Main gun body
        commands.entity(tool_entity).insert((
            Mesh3d(m.add(Cuboid::new(0.16, 0.22, 0.36))),
            MeshMaterial3d(casing_mat),
        ));

        // Front dispenser nozzle
        commands.spawn((
            Mesh3d(m.add(Cylinder::new(0.06, 0.14))),
            MeshMaterial3d(brass_mat),
            Transform::from_xyz(0.0, 0.0, 0.24)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ChildOf(tool_entity),
            Name::new("AutoWrapperNozzle"),
        ));
    }

    tool_entity
}

/// System driving the Auto Wrapper tool: raycasting to target physics items/elves,
/// advancing progress when Mouse1 is held, and wrapping the target upon 100% completion.
pub fn auto_wrapper_usage_system(
    mut commands: Commands,
    time: Option<Res<Time>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
    q_player: Query<(&Transform, &ElfHands), With<ElfPlayer>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    mut q_auto_wrapper: Query<(Entity, &mut AutoWrapperTool)>,
    mut q_elves: Query<
        (
            Entity,
            &Transform,
            &mut ElfAssistant,
            Option<&Children>,
            Has<WrappedElf>,
        ),
        Without<ElfPlayer>,
    >,
    mut q_presents: Query<
        (
            Entity,
            &Transform,
            &mut Present,
            Option<&MeshMaterial3d<StandardMaterial>>,
        ),
        (Without<ElfPlayer>, Without<ElfAssistant>),
    >,
    q_physics_targets: Query<
        (
            Entity,
            &Transform,
            Option<&Collider>,
            Option<&HeavyItem>,
            Option<&MeshMaterial3d<StandardMaterial>>,
        ),
        (
            Without<ElfPlayer>,
            Without<ElfAssistant>,
            Without<Present>,
            With<RigidBody>,
        ),
    >,
    q_child_materials: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Ok((player_tf, hands)) = q_player.single() else {
        return;
    };

    let mouse_held = mouse_buttons
        .as_ref()
        .map(|m| m.pressed(MouseButton::Left))
        .unwrap_or(false);

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);

    // Check which hand (if any) holds an AutoWrapperTool
    let held_wrapper_e = [hands.right_grabbed, hands.left_grabbed]
        .into_iter()
        .flatten()
        .find(|&e| q_auto_wrapper.contains(e));

    // For any unheld auto wrapper, reset wrapping state
    for (tool_e, mut tool) in &mut q_auto_wrapper {
        if Some(tool_e) != held_wrapper_e {
            tool.is_wrapping = false;
            tool.wrap_progress = 0.0;
            tool.current_target = None;
        }
    }

    let Some(tool_e) = held_wrapper_e else {
        return;
    };

    let Ok((_, mut tool)) = q_auto_wrapper.get_mut(tool_e) else {
        return;
    };

    if !mouse_held {
        tool.is_wrapping = false;
        tool.wrap_progress = 0.0;
        tool.current_target = None;
        return;
    }

    // Camera look ray
    let (cam_pos, cam_fwd) = if let Ok(cam_gtf) = q_camera.single() {
        (cam_gtf.translation(), cam_gtf.forward().into())
    } else {
        (
            player_tf.translation + Vec3::Y * 0.7,
            player_tf.forward().into(),
        )
    };

    // Find closest target in front of player within reach (4.0m)
    let max_reach = 4.0;
    let mut best_target: Option<(Entity, bool, f32, f32)> = None; // (Entity, is_elf, duration, distance)

    // Check Elves
    for (elf_e, elf_tf, _, _, is_already_wrapped) in &q_elves {
        if is_already_wrapped {
            continue;
        }
        let to_elf = elf_tf.translation - cam_pos;
        let dist = to_elf.length();
        if dist <= max_reach {
            let dot = to_elf.normalize().dot(cam_fwd);
            if dot > 0.85 {
                let dur = calculate_wrap_duration(tool.base_duration, None, None, true);
                if best_target.is_none() || dist < best_target.as_ref().unwrap().3 {
                    best_target = Some((elf_e, true, dur, dist));
                }
            }
        }
    }

    // Check Presents
    for (pres_e, pres_tf, _, _) in &q_presents {
        if pres_e == tool_e {
            continue;
        }
        let to_item = pres_tf.translation - cam_pos;
        let dist = to_item.length();
        if dist <= max_reach {
            let dot = to_item.normalize().dot(cam_fwd);
            if dot > 0.85 {
                let dur = calculate_wrap_duration(tool.base_duration, None, None, false);
                if best_target.is_none() || dist < best_target.as_ref().unwrap().3 {
                    best_target = Some((pres_e, false, dur, dist));
                }
            }
        }
    }

    // Check other physics bodies
    for (item_e, item_tf, opt_col, opt_heavy, _) in &q_physics_targets {
        if item_e == tool_e {
            continue;
        }
        let to_item = item_tf.translation - cam_pos;
        let dist = to_item.length();
        if dist <= max_reach {
            let dot = to_item.normalize().dot(cam_fwd);
            if dot > 0.85 {
                let dur = calculate_wrap_duration(tool.base_duration, opt_col, opt_heavy, false);
                if best_target.is_none() || dist < best_target.as_ref().unwrap().3 {
                    best_target = Some((item_e, false, dur, dist));
                }
            }
        }
    }

    let Some((target_entity, is_elf, required_dur, _)) = best_target else {
        tool.is_wrapping = false;
        tool.wrap_progress = 0.0;
        tool.current_target = None;
        return;
    };

    // If target changed, reset progress
    if tool.current_target != Some(target_entity) {
        tool.current_target = Some(target_entity);
        tool.wrap_progress = 0.0;
        tool.required_duration = required_dur;
    }

    tool.is_wrapping = true;
    tool.wrap_progress += dt / tool.required_duration.max(0.1);

    // Wrapping completed!
    if tool.wrap_progress >= 1.0 {
        tool.wrap_progress = 0.0;
        tool.is_wrapping = false;
        tool.current_target = None;

        let wrap_mat = workshop_mats
            .as_ref()
            .map(|wm| wm.festive_red.clone())
            .unwrap_or_else(|| {
                materials
                    .as_mut()
                    .map(|m| {
                        m.add(StandardMaterial {
                            base_color: FESTIVE_RED,
                            perceptual_roughness: 0.5,
                            ..default()
                        })
                    })
                    .unwrap_or_default()
            });

        if is_elf {
            // Immobilize elf for 10 seconds in ragdoll mode!
            if let Ok((_, _, mut elf, opt_children, _)) = q_elves.get_mut(target_entity) {
                elf.state = ElfNpcState::RagdollTumble;
                elf.tumble_duration = 10.0;
                elf.tumble_timer = 0.0;

                // Collect original materials from child meshes to restore on break-free
                let mut orig_mats = Vec::new();
                if let Some(children) = opt_children {
                    for child in children.iter() {
                        if let Ok((c_e, c_mat)) = q_child_materials.get(child) {
                            orig_mats.push((c_e, c_mat.0.clone()));
                            commands.entity(c_e).insert(MeshMaterial3d(wrap_mat.clone()));
                        }
                    }
                }

                commands.entity(target_entity).insert((
                    WrappedElf {
                        remaining_time: 10.0,
                        bounce_timer: 1.2,
                        original_materials: orig_mats,
                    },
                    WrappedInPaper {
                        pattern: PaperPattern::Stripes,
                        original_material: None,
                    },
                ));
            }
        } else {
            // Wrap physics object / present
            if let Ok((_, _, mut pres, opt_mat)) = q_presents.get_mut(target_entity) {
                pres.features.insert(WrappingFeature::PaperWrapped);
                pres.features.insert(WrappingFeature::StripedPattern);
                commands.entity(target_entity).insert((
                    WrappedInPaper {
                        pattern: PaperPattern::Stripes,
                        original_material: opt_mat.map(|m| m.0.clone()),
                    },
                    MeshMaterial3d(wrap_mat),
                ));
            } else if let Ok((_, _, _, _, opt_mat)) = q_physics_targets.get(target_entity) {
                commands.entity(target_entity).insert((
                    WrappedInPaper {
                        pattern: PaperPattern::Stripes,
                        original_material: opt_mat.map(|m| m.0.clone()),
                    },
                    MeshMaterial3d(wrap_mat),
                ));
            }
        }
    }
}

/// System managing immobilized wrapped elves: counts down 10 seconds, adds slight periodic
/// physics bounces, and removes the wrapping paper to let the elf break free.
pub fn wrapped_elf_system(
    mut commands: Commands,
    time: Option<Res<Time>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_wrapped_elves: Query<(
        Entity,
        &mut ElfAssistant,
        &mut WrappedElf,
        Option<&mut Velocity>,
    )>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);

    for (elf_e, mut elf, mut wrapped, mut opt_vel) in &mut q_wrapped_elves {
        wrapped.remaining_time -= dt;
        wrapped.bounce_timer -= dt;

        // Keep elf in RagdollTumble while immobilized
        elf.state = ElfNpcState::RagdollTumble;
        elf.tumble_duration = 10.0;

        // Occasionally bounce around slightly while wrapped
        if wrapped.bounce_timer <= 0.0 {
            wrapped.bounce_timer = 1.4;
            if let Some(ref mut vel) = opt_vel {
                vel.linear.y = (vel.linear.y + 2.2).min(3.0);
                vel.linear.x += ((elf.id as f32 * 1.5).sin()) * 0.9;
                vel.linear.z += ((elf.id as f32 * 1.5).cos()) * 0.9;
            }
        }

        // Break free when 10 seconds expire!
        if wrapped.remaining_time <= 0.0 {
            // Restore original materials on child meshes
            for &(child_e, ref orig_mat) in &wrapped.original_materials {
                commands
                    .entity(child_e)
                    .insert(MeshMaterial3d(orig_mat.clone()));
            }

            // Remove wrapping components
            commands
                .entity(elf_e)
                .remove::<WrappedElf>()
                .remove::<WrappedInPaper>();

            // Recover from ragdoll
            elf.state = ElfNpcState::Recovering;
            elf.recovery_timer = 0.0;
            elf.recovery_duration = 1.0;
            elf.tumble_timer = 0.0;
        }
    }
}
