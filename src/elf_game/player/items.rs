use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::WorkshopMaterials,
    physics_sandbox::{ElfAssistant, ElfJob, ElfJobHat, PhilosopherPipe},
    player::{
        bunny_ears::{
            spawn_bunny_ears_item, spawn_viewmodel_bunny_ears,
            BunnyEarsEquipped, BunnyEarsItem, ViewmodelBunnyEars,
        },
        controller::{ElfPlayer, ViewModelRoot},
        hands::{Grabbable, HeldByPlayer},
        ElfHands, ElfMovement,
    },
    ui::MenuState,
};

/// High-level taxonomy of item classes in Santa's Workshop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum ItemClass {
    /// Grabbable physics objects (present cubes, snack jars, crates, props, etc.)
    GrabbablePhysics,
    /// Handheld active tools that produce an effect (scissors, tape, pen, auto-wrapper, tinsel cannon)
    HandheldTool,
    /// Wearable equipment/hats that can be equipped onto the player or elves
    WearableHat,
}

/// Identifies the kind of wearable hat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HatType {
    BunnyEars,
    Job(ElfJob),
    PhilosopherPipe,
    Custom(String),
}

/// Marker component for any world item that can be worn as a hat on the player or elves.
#[derive(Component, Debug, Clone)]
pub struct WearableHatItem {
    pub hat_type: HatType,
}

impl WearableHatItem {
    pub fn new(hat_type: HatType) -> Self {
        Self { hat_type }
    }
}

/// Component attached to the player indicating which hat entity and type is currently equipped.
#[derive(Component, Debug, Clone)]
pub struct PlayerWornHat {
    pub hat_entity: Entity,
    pub hat_type: HatType,
}

/// Helper function to resolve the `HatType` of an entity from its components.
pub fn resolve_hat_type(
    opt_item: Option<&WearableHatItem>,
    opt_job: Option<&ElfJobHat>,
    opt_pipe: Option<&PhilosopherPipe>,
    opt_bunny: Option<&BunnyEarsItem>,
) -> Option<HatType> {
    if let Some(item) = opt_item {
        Some(item.hat_type.clone())
    } else if let Some(job) = opt_job {
        Some(HatType::Job(job.job))
    } else if opt_pipe.is_some() {
        Some(HatType::PhilosopherPipe)
    } else if opt_bunny.is_some() {
        Some(HatType::BunnyEars)
    } else {
        None
    }
}

/// System managing equipping, unequipping, and swapping wearable hats with KeyF.
///
/// Rules:
/// - Hats are equipped by holding them in a hand and pressing F.
/// - If wearing a hat, pressing F will put it in a free hand if there is a free hand.
/// - If there is no free hand, pressing F does NOT unequip the hat, unless a hand is
///   holding a DIFFERENT hat, in which case they are swapped.
pub fn player_hat_equip_system(
    mut commands: Commands,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
    mut q_player: Query<
        (
            Entity,
            &Transform,
            &mut ElfMovement,
            &mut ElfHands,
            Option<&PlayerWornHat>,
            Has<BunnyEarsEquipped>,
        ),
        With<ElfPlayer>,
    >,
    q_vm_root: Query<Entity, With<ViewModelRoot>>,
    q_vm_ears: Query<Entity, With<ViewmodelBunnyEars>>,
    q_hat_items: Query<(
        Entity,
        Option<&WearableHatItem>,
        Option<&ElfJobHat>,
        Option<&PhilosopherPipe>,
        Option<&BunnyEarsItem>,
    )>,
    mut q_transforms: Query<&mut Transform, (Without<ElfPlayer>, Without<ElfAssistant>)>,
    mut q_visibilities: Query<&mut Visibility, Without<ElfPlayer>>,
    q_elves: Query<(Entity, &Transform), With<ElfAssistant>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Some(keys) = keys else { return; };
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    let Ok((player_entity, player_tf, mut movement, mut hands, opt_worn, has_bunny_equipped)) =
        q_player.single_mut()
    else {
        return;
    };

    // If standing very close to an elf and holding a hat, give priority to elf hat assignment if that elf has no hat
    let near_elf = q_elves.iter().any(|(_, elf_tf)| {
        elf_tf.translation.distance(player_tf.translation) < 2.0
    });

    // Check which hand holds a hat, if any
    let right_hat_info = hands.right_grabbed.and_then(|e| {
        if let Ok((entity, opt_w, opt_j, opt_p, opt_b)) = q_hat_items.get(e) {
            resolve_hat_type(opt_w, opt_j, opt_p, opt_b).map(|ht| (entity, ht))
        } else {
            None
        }
    });

    let left_hat_info = hands.left_grabbed.and_then(|e| {
        if let Ok((entity, opt_w, opt_j, opt_p, opt_b)) = q_hat_items.get(e) {
            resolve_hat_type(opt_w, opt_j, opt_p, opt_b).map(|ht| (entity, ht))
        } else {
            None
        }
    });

    // Determine current worn hat, if any
    let is_wearing_hat = opt_worn.is_some() || has_bunny_equipped;

    if is_wearing_hat {
        // Player IS currently wearing a hat!
        let (worn_entity, worn_type) = if let Some(worn) = opt_worn {
            (worn.hat_entity, worn.hat_type.clone())
        } else {
            (Entity::PLACEHOLDER, HatType::BunnyEars)
        };

        let right_free = hands.right_grabbed.is_none();
        let left_free = hands.left_grabbed.is_none();

        if right_free || left_free {
            // CASE 1: Free hand available -> Unequip into free hand!
            let target_hand_is_right = right_free;
            let drop_pos = player_tf.translation + player_tf.forward() * 0.6 + Vec3::Y * 0.2;

            let restored_entity = if commands.get_entity(worn_entity).is_ok() && worn_entity != Entity::PLACEHOLDER {
                // Restore existing entity
                commands.entity(worn_entity)
                    .insert((
                        Grabbable,
                        HeldByPlayer,
                        RigidBody::Dynamic,
                        Collider::ball(0.22),
                    ));
                if let Ok(mut vis) = q_visibilities.get_mut(worn_entity) {
                    *vis = Visibility::Inherited;
                }
                if let Ok(mut tf) = q_transforms.get_mut(worn_entity) {
                    tf.translation = drop_pos;
                }
                worn_entity
            } else if worn_type == HatType::BunnyEars {
                // Spawn a fresh world pickup item for bunny ears
                if let (Some(m), Some(mat)) = (meshes.as_deref_mut(), materials.as_deref_mut()) {
                    spawn_bunny_ears_item(
                        &mut commands,
                        m,
                        mat,
                        workshop_mats.as_deref(),
                        drop_pos,
                    )
                } else {
                    worn_entity
                }
            } else {
                worn_entity
            };

            if target_hand_is_right {
                hands.right_grabbed = Some(restored_entity);
            } else {
                hands.left_grabbed = Some(restored_entity);
            }

            // Remove worn effects
            commands.entity(player_entity).remove::<PlayerWornHat>();
            if worn_type == HatType::BunnyEars || has_bunny_equipped {
                commands.entity(player_entity).remove::<BunnyEarsEquipped>();
                movement.has_bunny_ears = false;
                for vm_e in &q_vm_ears {
                    commands.entity(vm_e).despawn();
                }
            }
        } else {
            // CASE 2: Both hands full! Check if one hand holds a DIFFERENT hat to swap with
            let swap_candidate = if let Some((rh_e, ref rh_type)) = right_hat_info {
                Some((rh_e, rh_type.clone(), true))
            } else if let Some((lh_e, ref lh_type)) = left_hat_info {
                Some((lh_e, lh_type.clone(), false))
            } else {
                None
            };

            if let Some((new_hat_e, new_hat_type, is_right_hand)) = swap_candidate {
                // SWAP worn hat with the held hat!
                let drop_pos = player_tf.translation + player_tf.forward() * 0.6 + Vec3::Y * 0.2;

                // 1. Restore previously worn hat in the swapped hand
                let restored_old_e = if commands.get_entity(worn_entity).is_ok() && worn_entity != Entity::PLACEHOLDER {
                    commands.entity(worn_entity)
                        .insert((
                            Grabbable,
                            HeldByPlayer,
                            RigidBody::Dynamic,
                            Collider::ball(0.22),
                        ));
                    if let Ok(mut vis) = q_visibilities.get_mut(worn_entity) {
                        *vis = Visibility::Inherited;
                    }
                    if let Ok(mut tf) = q_transforms.get_mut(worn_entity) {
                        tf.translation = drop_pos;
                    }
                    worn_entity
                } else if worn_type == HatType::BunnyEars {
                    if let (Some(m), Some(mat)) = (meshes.as_deref_mut(), materials.as_deref_mut()) {
                        spawn_bunny_ears_item(
                            &mut commands,
                            m,
                            mat,
                            workshop_mats.as_deref(),
                            drop_pos,
                        )
                    } else {
                        worn_entity
                    }
                } else {
                    worn_entity
                };

                if is_right_hand {
                    hands.right_grabbed = Some(restored_old_e);
                } else {
                    hands.left_grabbed = Some(restored_old_e);
                }

                // 2. Hide newly equipped hat from hand and world
                commands.entity(new_hat_e)
                    .remove::<(HeldByPlayer, RigidBody, Collider, Grabbable)>()
                    .insert(Visibility::Hidden);

                // 3. Update player worn components
                commands.entity(player_entity).insert(PlayerWornHat {
                    hat_entity: new_hat_e,
                    hat_type: new_hat_type.clone(),
                });

                // 4. Update BunnyEars specific effects
                if worn_type == HatType::BunnyEars && new_hat_type != HatType::BunnyEars {
                    commands.entity(player_entity).remove::<BunnyEarsEquipped>();
                    movement.has_bunny_ears = false;
                    for vm_e in &q_vm_ears {
                        commands.entity(vm_e).despawn();
                    }
                } else if new_hat_type == HatType::BunnyEars && worn_type != HatType::BunnyEars {
                    commands.entity(player_entity).insert(BunnyEarsEquipped);
                    movement.has_bunny_ears = true;
                    if let Ok(vm_root) = q_vm_root.single() {
                        if let (Some(m), Some(mat)) = (meshes.as_deref_mut(), materials.as_deref_mut()) {
                            spawn_viewmodel_bunny_ears(
                                &mut commands,
                                vm_root,
                                m,
                                mat,
                                workshop_mats.as_deref(),
                            );
                        }
                    }
                }
            } else {
                // Both hands full with non-hat items: DO NOT unequip hat!
                // Guard requirement: "If there is no free hand, then pressing F does not unequip the hat"
            }
        }
    } else {
        // Player is NOT wearing a hat: check if holding a hat to equip!
        if near_elf {
            // If standing near an elf, prefer elf interaction if player is not wearing a hat
            return;
        }

        let held_hat_candidate = right_hat_info
            .map(|(e, t)| (e, t, true))
            .or_else(|| left_hat_info.map(|(e, t)| (e, t, false)));

        if let Some((hat_e, hat_type, is_right)) = held_hat_candidate {
            // Clear hat from hand
            if is_right {
                hands.right_grabbed = None;
            } else {
                hands.left_grabbed = None;
            }

            // Hide from world / disable physics
            commands.entity(hat_e)
                .remove::<(HeldByPlayer, RigidBody, Collider, Grabbable)>()
                .insert(Visibility::Hidden);

            // Insert PlayerWornHat
            commands.entity(player_entity).insert(PlayerWornHat {
                hat_entity: hat_e,
                hat_type: hat_type.clone(),
            });

            // Handle specific hat type effects
            if hat_type == HatType::BunnyEars {
                commands.entity(player_entity).insert(BunnyEarsEquipped);
                movement.has_bunny_ears = true;
                if let Ok(vm_root) = q_vm_root.single() {
                    if let (Some(m), Some(mat)) = (meshes.as_deref_mut(), materials.as_deref_mut()) {
                        spawn_viewmodel_bunny_ears(
                            &mut commands,
                            vm_root,
                            m,
                            mat,
                            workshop_mats.as_deref(),
                        );
                    }
                }
            }
        }
    }
}
