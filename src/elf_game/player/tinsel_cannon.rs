use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    physics_sandbox::ragdoll::{ElfAssistant, ElfNpcState},
    player::{
        controller::{ElfPlayer, FirstPersonCamera},
        hands::{Grabbable, HeldByPlayer},
        items::ItemClass,
        ElfHands,
    },
    ui::MenuState,
};

/// Handheld Tinsel Hand Cannon weapon.
#[derive(Component, Debug, Clone)]
pub struct TinselHandCannon {
    pub ammo: u32,
    pub cooldown: f32,
    pub fire_timer: f32,
    pub equipped_by: Option<Entity>,
}

impl Default for TinselHandCannon {
    fn default() -> Self {
        Self {
            ammo: 50,
            cooldown: 0.4,
            fire_timer: 0.0,
            equipped_by: None,
        }
    }
}

/// Dynamic strand of festive tinsel with very high friction that can anchor to rafters and items.
#[derive(Component, Debug, Clone)]
pub struct TinselStrand {
    pub is_anchored: bool,
    pub anchor_point: Option<Vec3>,
    pub attached_entity: Option<Entity>,
    pub lifetime: f32,
    pub friction_coeff: f32,
    pub is_climbable: bool,
    pub fired_by: Option<Entity>,
}

impl Default for TinselStrand {
    fn default() -> Self {
        Self {
            is_anchored: false,
            anchor_point: None,
            attached_entity: None,
            lifetime: 25.0,
            friction_coeff: 4.0,
            is_climbable: true,
            fired_by: None,
        }
    }
}

/// Spawns a physical Tinsel Hand Cannon tool item in the world.
pub fn spawn_tinsel_hand_cannon(
    commands: &mut Commands,
    meshes: Option<&mut Assets<Mesh>>,
    mut materials: Option<&mut Assets<StandardMaterial>>,
    wm: Option<&WorkshopMaterials>,
    pos: Vec3,
) -> Entity {
    let body_mat = wm.map(|w| w.festive_red.clone()).unwrap_or_else(|| {
        materials
            .as_mut()
            .map(|m| {
                m.add(StandardMaterial {
                    base_color: FESTIVE_RED,
                    metallic: 0.7,
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
                    metallic: 0.85,
                    perceptual_roughness: 0.2,
                    ..default()
                })
            })
            .unwrap_or_default()
    });

    let cannon_e = commands
        .spawn((
            TinselHandCannon::default(),
            Grabbable,
            ItemClass::HandheldTool,
            Transform::from_translation(pos),
            RigidBody::Dynamic,
            Collider::cuboid(0.12, 0.15, 0.28),
            Velocity::default(),
            Visibility::default(),
            Name::new("TinselHandCannon"),
        ))
        .id();

    if let (Some(m), Some(_)) = (meshes, materials) {
        commands.entity(cannon_e).insert((
            Mesh3d(m.add(Cylinder::new(0.09, 0.38))),
            MeshMaterial3d(body_mat),
        ));

        // Spool chamber on top of cannon
        commands.spawn((
            Mesh3d(m.add(Cylinder::new(0.11, 0.12))),
            MeshMaterial3d(brass_mat),
            Transform::from_xyz(0.0, 0.12, -0.06),
            ChildOf(cannon_e),
            Name::new("TinselSpoolChamber"),
        ));
    }

    cannon_e
}

/// Fires a physical tinsel strand with high friction along a direction vector.
pub fn fire_tinsel_strand(
    commands: &mut Commands,
    meshes: Option<&mut Assets<Mesh>>,
    mut materials: Option<&mut Assets<StandardMaterial>>,
    wm: Option<&WorkshopMaterials>,
    start_pos: Vec3,
    direction: Vec3,
    speed: f32,
    fired_by: Option<Entity>,
) -> Entity {
    let tinsel_mat = wm.map(|w| w.gold_ribbon.clone()).unwrap_or_else(|| {
        materials
            .as_mut()
            .map(|m| {
                m.add(StandardMaterial {
                    base_color: GOLD_RIBBON,
                    metallic: 0.95,
                    perceptual_roughness: 0.15,
                    ..default()
                })
            })
            .unwrap_or_default()
    });

    let strand_e = commands
        .spawn((
            TinselStrand {
                fired_by,
                ..default()
            },
            Transform::from_translation(start_pos)
                .looking_to(direction, Vec3::Y),
            RigidBody::Dynamic,
            Collider::capsule_z(0.35, 0.04),
            Friction::coefficient(4.0),
            Restitution::coefficient(0.05),
            Velocity {
                linear: direction * speed,
                angular: Vec3::new(1.0, 2.5, 0.5),
            },
            Visibility::default(),
            Name::new("TinselStrand"),
        ))
        .id();

    if let (Some(m), Some(_)) = (meshes, materials) {
        commands.entity(strand_e).insert((
            Mesh3d(m.add(Cylinder::new(0.035, 0.70))),
            MeshMaterial3d(tinsel_mat),
        ));
    }

    strand_e
}

/// System for player firing the Tinsel Hand Cannon when held in hand.
pub fn player_tinsel_cannon_system(
    mut commands: Commands,
    time: Option<Res<Time>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
    q_player: Query<(&Transform, &ElfHands), With<ElfPlayer>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    mut q_cannons: Query<(Entity, &mut TinselHandCannon)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Ok((player_tf, hands)) = q_player.single() else {
        return;
    };

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);

    let held_cannon_e = [hands.right_grabbed, hands.left_grabbed]
        .into_iter()
        .flatten()
        .find(|&e| q_cannons.contains(e));

    for (cannon_e, mut cannon) in &mut q_cannons {
        if cannon.fire_timer > 0.0 {
            cannon.fire_timer -= dt;
        }

        if Some(cannon_e) != held_cannon_e {
            continue;
        }

        let mouse_clicked = mouse_buttons
            .as_ref()
            .map(|m| m.just_pressed(MouseButton::Left))
            .unwrap_or(false);

        if mouse_clicked && cannon.fire_timer <= 0.0 {
            cannon.fire_timer = cannon.cooldown;
            if cannon.ammo > 0 {
                cannon.ammo -= 1;
            }

            let (cam_pos, cam_fwd) = if let Ok(cam_gtf) = q_camera.single() {
                (cam_gtf.translation(), cam_gtf.forward().into())
            } else {
                (
                    player_tf.translation + Vec3::Y * 0.6,
                    player_tf.forward().into(),
                )
            };

            let spawn_pos = cam_pos + cam_fwd * 0.45;
            fire_tinsel_strand(
                &mut commands,
                meshes.as_deref_mut(),
                materials.as_deref_mut(),
                workshop_mats.as_deref(),
                spawn_pos,
                cam_fwd,
                22.0,
                None,
            );
        }
    }
}

/// System for mischievous elves retaliating with their equipped Tinsel Hand Cannon
/// at whatever entity knocked them down, suffering massive slapstick recoil that ragdolls them backwards!
pub fn elf_tinsel_cannon_retaliation_system(
    mut commands: Commands,
    time: Option<Res<Time>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
    mut q_elves: Query<(
        Entity,
        &Transform,
        &mut ElfAssistant,
        &mut Velocity,
        &mut ExternalImpulse,
        &mut LockedAxes,
    )>,
    mut q_cannons: Query<&mut TinselHandCannon>,
    q_all_entities: Query<&Transform>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);

    for (elf_e, elf_tf, mut elf, mut vel, mut impulse, mut locked_axes) in &mut q_elves {
        if elf.tinsel_fire_cooldown > 0.0 {
            elf.tinsel_fire_cooldown -= dt;
        }

        let Some(cannon_e) = elf.equipped_tinsel_cannon else {
            continue;
        };

        // If elf is currently tumbling, wait until standing/recovering before retaliating
        if elf.state == ElfNpcState::RagdollTumble {
            continue;
        }

        // Check if there is a target that knocked this elf down
        let Some(target_e) = elf.last_knockdown_source else {
            continue;
        };

        if elf.tinsel_fire_cooldown > 0.0 {
            continue;
        }

        let Ok(target_tf) = q_all_entities.get(target_e) else {
            // Target no longer exists
            elf.last_knockdown_source = None;
            continue;
        };

        // Aim at target
        let aim_vec = target_tf.translation - elf_tf.translation;
        let aim_dir = aim_vec.normalize_or_zero();
        if aim_dir == Vec3::ZERO {
            continue;
        }

        // Fire cannon!
        if let Ok(mut cannon) = q_cannons.get_mut(cannon_e) {
            if cannon.ammo > 0 {
                cannon.ammo -= 1;
            }
            cannon.fire_timer = cannon.cooldown;
        }

        let spawn_pos = elf_tf.translation + Vec3::Y * 0.35 + aim_dir * 0.4;
        fire_tinsel_strand(
            &mut commands,
            meshes.as_deref_mut(),
            materials.as_deref_mut(),
            workshop_mats.as_deref(),
            spawn_pos,
            aim_dir,
            20.0,
            Some(elf_e),
        );

        // SLAPSTICK EXPLOSIVE RECOIL:
        // Knocks the elf backward at high speed and throws them into ragdoll tumble!
        let mut recoil = -aim_dir * 14.0;
        recoil.y = recoil.y.max(0.0) + 6.0;
        vel.linear = recoil;
        impulse.torque_impulse += Vec3::new(8.0, 5.0, 7.0);
        *locked_axes = LockedAxes::empty();

        elf.state = ElfNpcState::RagdollTumble;
        elf.tumble_timer = 0.0;
        elf.tumble_duration = 2.5;
        elf.tinsel_fire_cooldown = 4.0;
        elf.last_knockdown_source = None; // Revenge achieved!
    }
}

/// Allows elves with the Mischief trait to pick up / equip a Tinsel Hand Cannon
/// when player interacts or places it near them.
pub fn elf_equip_tinsel_cannon_system(
    mut commands: Commands,
    mut q_player: Query<(&Transform, &mut ElfHands), With<ElfPlayer>>,
    mut q_elves: Query<(Entity, &Transform, &mut ElfAssistant)>,
    mut q_cannons: Query<(Entity, &Transform, &mut TinselHandCannon, Option<&mut Velocity>), Without<ElfAssistant>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let is_interacting = keys
        .as_ref()
        .map(|k| k.any_just_pressed([KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyQ]))
        .unwrap_or(false)
        || mouse_buttons
            .as_ref()
            .map(|m| m.just_pressed(MouseButton::Left))
            .unwrap_or(false);

    if !is_interacting {
        return;
    }

    let Ok((player_tf, mut hands)) = q_player.single_mut() else {
        return;
    };

    // Find held cannon
    let held_cannon_info = if let Some(e) = hands.right_grabbed {
        if q_cannons.contains(e) { Some((e, true)) } else { None }
    } else if let Some(e) = hands.left_grabbed {
        if q_cannons.contains(e) { Some((e, false)) } else { None }
    } else {
        None
    };

    let Some((cannon_e, is_right)) = held_cannon_info else {
        return;
    };

    // Find nearest elf within 2.2m that does not already have a cannon
    let mut nearest_elf = None;
    let mut min_d = 2.2;
    for (elf_e, elf_tf, elf) in &q_elves {
        if elf.equipped_tinsel_cannon.is_none() {
            let d = elf_tf.translation.distance(player_tf.translation);
            if d < min_d {
                min_d = d;
                nearest_elf = Some(elf_e);
            }
        }
    }

    let Some(target_elf_e) = nearest_elf else {
        return;
    };

    // Release from player hand
    if is_right {
        hands.right_grabbed = None;
    } else {
        hands.left_grabbed = None;
    }

    // Equip on elf
    if let Ok((_, _, mut cannon, opt_vel)) = q_cannons.get_mut(cannon_e) {
        cannon.equipped_by = Some(target_elf_e);
        if let Some(mut vel) = opt_vel {
            vel.linear = Vec3::ZERO;
            vel.angular = Vec3::ZERO;
        }
    }

    commands
        .entity(cannon_e)
        .remove::<(HeldByPlayer, RigidBody, Collider, Grabbable)>()
        .insert(ChildOf(target_elf_e));

    if let Ok((_, _, mut elf)) = q_elves.get_mut(target_elf_e) {
        elf.equipped_tinsel_cannon = Some(cannon_e);
    }
}
