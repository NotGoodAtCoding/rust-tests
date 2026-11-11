use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
};
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    player::{
        controller::{ElfPlayer, ViewModelRoot},
        hands::Grabbable,
        ElfHands, ElfMovement,
    },
    ui::MenuState,
};

/// Dynamic pickup item in the world representing floppy bunny ears.
#[derive(Component, Debug, Clone)]
pub struct BunnyEarsItem;

/// Marker component on ElfPlayer when bunny ears are equipped.
#[derive(Component, Debug, Clone, Default)]
pub struct BunnyEarsEquipped;

/// Root entity for viewmodel bunny ears parented to ViewModelRoot.
#[derive(Component, Debug, Clone)]
pub struct ViewmodelBunnyEars;

/// Physics bob component for a viewmodel floppy bunny ear.
#[derive(Component, Debug, Clone)]
pub struct FloppyEar {
    pub rest_translation: Vec3,
    pub rest_rotation: Quat,
    pub current_offset: Vec3,
    pub current_rot_offset: Vec3, // pitch, yaw, roll offsets in radians
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub spring_stiffness: f32,
    pub damping: f32,
    pub is_left: bool,
    pub phase: f32,
}

impl FloppyEar {
    pub fn new(rest_translation: Vec3, rest_rotation: Quat, is_left: bool, phase: f32) -> Self {
        Self {
            rest_translation,
            rest_rotation,
            current_offset: Vec3::ZERO,
            current_rot_offset: Vec3::ZERO,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            spring_stiffness: 35.0,
            damping: 8.0,
            is_left,
            phase,
        }
    }
}

/// Spawns a physical Bunny Ears pickup item in the workshop.
pub fn spawn_bunny_ears_item(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    wm: Option<&WorkshopMaterials>,
    position: Vec3,
) -> Entity {
    let white_mat = wm.map(|w| w.fluffy_white.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SNOW_WHITE,
            perceptual_roughness: 0.85,
            ..default()
        })
    });

    let pink_mat = wm.map(|w| w.bunny_ear_pink.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: BUNNY_EAR_PINK,
            perceptual_roughness: 0.60,
            ..default()
        })
    });

    let headband_mat = wm.map(|w| w.pine_green.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PINE_GREEN,
            perceptual_roughness: 0.70,
            ..default()
        })
    });

    // Parent item entity with Rapier dynamic physics
    commands
        .spawn((
            BunnyEarsItem,
            crate::elf_game::player::items::WearableHatItem::new(crate::elf_game::player::items::HatType::BunnyEars),
            crate::elf_game::player::items::ItemClass::WearableHat,
            Grabbable,
            RigidBody::Dynamic,
            Collider::ball(0.22),
            Transform::from_translation(position),
            Visibility::default(),
            Name::new("BunnyEarsItem"),
        ))
        .with_children(|root| {
            // Headband arc
            root.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.14, 0.04))),
                MeshMaterial3d(headband_mat),
                Transform::from_xyz(0.0, 0.05, 0.0)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("BunnyHeadband"),
            ));

            // Left Ear
            root.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.035, 0.28))),
                MeshMaterial3d(white_mat.clone()),
                Transform::from_xyz(-0.08, 0.18, 0.0)
                    .with_rotation(Quat::from_rotation_z(0.15) * Quat::from_rotation_x(-0.10)),
                Name::new("BunnyEarLeft"),
            ))
            .with_children(|ear| {
                // Pink inner ear
                ear.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.03, 0.22, 0.015))),
                    MeshMaterial3d(pink_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.025),
                    Name::new("BunnyEarInnerPinkLeft"),
                ));
            });

            // Right Ear
            root.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.035, 0.28))),
                MeshMaterial3d(white_mat),
                Transform::from_xyz(0.08, 0.18, 0.0)
                    .with_rotation(Quat::from_rotation_z(-0.15) * Quat::from_rotation_x(-0.10)),
                Name::new("BunnyEarRight"),
            ))
            .with_children(|ear| {
                // Pink inner ear
                ear.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.03, 0.22, 0.015))),
                    MeshMaterial3d(pink_mat),
                    Transform::from_xyz(0.0, 0.0, 0.025),
                    Name::new("BunnyEarInnerPinkRight"),
                ));
            });
        })
        .id()
}

/// Spawns the viewmodel bunny ears at the top of the player's view screen parented to ViewModelRoot.
pub fn spawn_viewmodel_bunny_ears(
    commands: &mut Commands,
    vm_root: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    wm: Option<&WorkshopMaterials>,
) -> Entity {
    let white_mat = wm.map(|w| w.fluffy_white.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SNOW_WHITE,
            perceptual_roughness: 0.85,
            ..default()
        })
    });

    let pink_mat = wm.map(|w| w.bunny_ear_pink.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: BUNNY_EAR_PINK,
            perceptual_roughness: 0.60,
            ..default()
        })
    });

    let headband_mat = wm.map(|w| w.pine_green.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PINE_GREEN,
            perceptual_roughness: 0.70,
            ..default()
        })
    });

    let vm_ears = commands
        .spawn((
            ViewmodelBunnyEars,
            Transform::IDENTITY,
            Visibility::default(),
            ChildOf(vm_root),
            Name::new("ViewmodelBunnyEarsRoot"),
        ))
        .with_children(|parent| {
            // Viewmodel headband arch at top of screen (Y ≈ 0.38, Z ≈ -0.50)
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.24, 0.025))),
                MeshMaterial3d(headband_mat),
                Transform::from_xyz(0.0, 0.40, -0.48)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Name::new("VmHeadband"),
            ));

            // Left Floppy Ear: top-left of view screen
            let left_rest_pos = Vec3::new(-0.16, 0.42, -0.48);
            let left_rest_rot = Quat::from_rotation_z(0.18) * Quat::from_rotation_x(-0.25);
            parent
                .spawn((
                    FloppyEar::new(left_rest_pos, left_rest_rot, true, 0.0),
                    Transform::from_translation(left_rest_pos).with_rotation(left_rest_rot),
                    Visibility::default(),
                    Name::new("VmFloppyEarLeft"),
                ))
                .with_children(|ear_root| {
                    ear_root.spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.042, 0.30))),
                        MeshMaterial3d(white_mat.clone()),
                        Transform::from_xyz(0.0, 0.12, 0.0),
                        Name::new("VmLeftEarMesh"),
                    ))
                    .with_children(|ear_body| {
                        ear_body.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.038, 0.24, 0.018))),
                            MeshMaterial3d(pink_mat.clone()),
                            Transform::from_xyz(0.0, 0.0, 0.032),
                            Name::new("VmLeftEarPink"),
                        ));
                    });
                });

            // Right Floppy Ear: top-right of view screen
            let right_rest_pos = Vec3::new(0.16, 0.42, -0.48);
            let right_rest_rot = Quat::from_rotation_z(-0.18) * Quat::from_rotation_x(-0.25);
            parent
                .spawn((
                    FloppyEar::new(right_rest_pos, right_rest_rot, false, std::f32::consts::PI),
                    Transform::from_translation(right_rest_pos).with_rotation(right_rest_rot),
                    Visibility::default(),
                    Name::new("VmFloppyEarRight"),
                ))
                .with_children(|ear_root| {
                    ear_root.spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.042, 0.30))),
                        MeshMaterial3d(white_mat),
                        Transform::from_xyz(0.0, 0.12, 0.0),
                        Name::new("VmRightEarMesh"),
                    ))
                    .with_children(|ear_body| {
                        ear_body.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.038, 0.24, 0.018))),
                            MeshMaterial3d(pink_mat),
                            Transform::from_xyz(0.0, 0.0, 0.032),
                            Name::new("VmRightEarPink"),
                        ));
                    });
                });
        })
        .id();

    vm_ears
}

/// System managing equipping and unequipping of Bunny Ears via KeyH / KeyB or interaction.
pub fn player_bunny_ears_equip_system(
    mut commands: Commands,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
    mut q_player: Query<
        (Entity, &Transform, &mut ElfMovement, &mut ElfHands, Has<BunnyEarsEquipped>),
        With<ElfPlayer>,
    >,
    q_vm_root: Query<Entity, With<ViewModelRoot>>,
    q_vm_ears: Query<Entity, With<ViewmodelBunnyEars>>,
    q_world_items: Query<(Entity, &Transform), With<BunnyEarsItem>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Some(keys) = keys else { return; };
    let equip_pressed = keys.just_pressed(KeyCode::KeyH) || keys.just_pressed(KeyCode::KeyB);
    if !equip_pressed {
        return;
    }

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let Ok((player_entity, player_tf, mut movement, mut hands, is_equipped)) = q_player.single_mut() else {
        return;
    };

    if is_equipped {
        // Unequip bunny ears!
        commands.entity(player_entity).remove::<BunnyEarsEquipped>();
        movement.has_bunny_ears = false;

        // Despawn viewmodel ears
        for vm_e in &q_vm_ears {
            commands.entity(vm_e).despawn();
        }

        // Spawn a fresh world pickup item slightly in front of the player
        let drop_pos = player_tf.translation + player_tf.forward() * 0.8 + Vec3::Y * 0.4;
        spawn_bunny_ears_item(
            &mut commands,
            &mut meshes,
            &mut materials,
            workshop_mats.as_deref(),
            drop_pos,
        );
    } else {
        // Check if player is holding a BunnyEarsItem in hand or standing near one
        let held_item_e = if let Some(left) = hands.left_grabbed {
            if q_world_items.contains(left) { Some(left) } else { None }
        } else if let Some(right) = hands.right_grabbed {
            if q_world_items.contains(right) { Some(right) } else { None }
        } else {
            None
        };

        let target_item_e = held_item_e.or_else(|| {
            // Find closest bunny ears within 2.2m
            let mut nearest = None;
            let mut min_d = 2.2;
            for (item_e, item_tf) in &q_world_items {
                let d = item_tf.translation.distance(player_tf.translation);
                if d < min_d {
                    min_d = d;
                    nearest = Some(item_e);
                }
            }
            nearest
        });

        if let Some(item_e) = target_item_e {
            // Release from player hand if held
            if hands.left_grabbed == Some(item_e) {
                hands.left_grabbed = None;
            }
            if hands.right_grabbed == Some(item_e) {
                hands.right_grabbed = None;
            }

            // Despawn the world pickup
            commands.entity(item_e).despawn();

            // Equip on player
            commands.entity(player_entity).insert(BunnyEarsEquipped);
            movement.has_bunny_ears = true;

            // Spawn viewmodel floppy bunny ears under ViewModelRoot
            if let Ok(vm_root) = q_vm_root.single() {
                spawn_viewmodel_bunny_ears(
                    &mut commands,
                    vm_root,
                    &mut meshes,
                    &mut materials,
                    workshop_mats.as_deref(),
                );
            }
        }
    }
}

/// Dynamic spring-damped physics simulation for floppy bunny ears bobbing on top of the view screen.
pub fn floppy_ears_physics_bob_system(
    time: Option<Res<Time>>,
    mouse_motion: Option<Res<AccumulatedMouseMotion>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_player: Query<&ElfMovement, With<ElfPlayer>>,
    mut q_ears: Query<(&mut Transform, &mut FloppyEar)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Some(time) = time else { return; };
    let dt = time.delta_secs();
    if dt <= 0.0 || dt > 0.1 {
        return;
    }

    let Ok(movement) = q_player.single() else { return; };
    let mouse_delta = mouse_motion.map(|m| m.delta).unwrap_or(Vec2::ZERO);

    // Player locomotion signals
    let h_speed = Vec2::new(movement.velocity.x, movement.velocity.z).length();
    let v_vel = movement.velocity.y;
    let t = time.elapsed_secs();

    for (mut tf, mut ear) in &mut q_ears {
        // Target positional inertial displacement
        let mut target_offset = Vec3::ZERO;
        let mut target_rot = Vec3::ZERO;

        // 1. Vertical motion (jump, fall, landing)
        // Droop down when rising, bounce back when falling
        target_offset.y += -v_vel * 0.008;
        target_rot.x += -v_vel * 0.035; // pitch flop

        // 2. Horizontal locomotion bob
        if movement.is_grounded && h_speed > 0.5 {
            let bob_freq = (h_speed * 2.2).clamp(6.0, 14.0);
            let ear_phase = ear.phase;
            let sway_x = ((t * bob_freq * 0.5) + ear_phase).sin() * 0.015;
            let bounce_y = (t * bob_freq).sin().abs() * 0.018;
            target_offset.x += sway_x;
            target_offset.y -= bounce_y;

            let roll_flop = ((t * bob_freq * 0.5) + ear_phase).cos() * 0.08;
            target_rot.z += if ear.is_left { roll_flop } else { -roll_flop };
            target_rot.x += (t * bob_freq).sin() * 0.06;
        } else {
            // Idle gentle breathing sway
            let idle_sway = ((t * 2.0) + ear.phase).sin() * 0.004;
            target_offset.y += idle_sway;
            target_rot.x += ((t * 2.0) + ear.phase).cos() * 0.02;
        }

        // 3. Camera turning sway (inertial reaction to mouse look)
        let yaw_lag = -mouse_delta.x * 0.0025;
        let pitch_lag = -mouse_delta.y * 0.0020;
        target_offset.x += yaw_lag * 0.1;
        target_rot.y += yaw_lag;
        target_rot.x += pitch_lag;

        // Damped harmonic spring update for translation: F = -k*x - c*v
        let disp_x = target_offset - ear.current_offset;
        let spring_force_pos = disp_x * ear.spring_stiffness;
        let damping_force_pos = ear.velocity * ear.damping;
        let accel_pos = spring_force_pos - damping_force_pos;
        ear.velocity += accel_pos * dt;
        let pos_vel = ear.velocity;
        ear.current_offset += pos_vel * dt;

        // Damped harmonic spring update for rotation:
        let disp_r = target_rot - ear.current_rot_offset;
        let spring_force_rot = disp_r * (ear.spring_stiffness * 0.9);
        let damping_force_rot = ear.angular_velocity * ear.damping;
        let accel_rot = spring_force_rot - damping_force_rot;
        ear.angular_velocity += accel_rot * dt;
        let ang_vel = ear.angular_velocity;
        ear.current_rot_offset += ang_vel * dt;

        // Apply to local transform
        tf.translation = ear.rest_translation + ear.current_offset;
        let rot_delta = Quat::from_euler(
            EulerRot::YXZ,
            ear.current_rot_offset.y,
            ear.current_rot_offset.x,
            ear.current_rot_offset.z,
        );
        tf.rotation = ear.rest_rotation * rot_delta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf_game::player::{
        hands::spawn_workshop_tools_and_materials,
        update_elf_physics, BhopConfig, BhopWindow,
    };

    #[test]
    fn test_bunny_ears_item_equip_viewmodel_and_bhop_mechanics() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<BhopConfig>();

        app.add_systems(Startup, (
            spawn_workshop_tools_and_materials,
            |mut commands: Commands| {
                let player_e = commands
                    .spawn((
                        ElfPlayer,
                        ElfMovement::default(),
                        ElfHands::default(),
                        Transform::from_xyz(0.0, 1.2, 0.0),
                        Name::new("ElfPlayer"),
                    ))
                    .id();

                let _vm_root = commands
                    .spawn((
                        ViewModelRoot,
                        Transform::IDENTITY,
                        Visibility::default(),
                        ChildOf(player_e),
                        Name::new("ViewModelRoot"),
                    ))
                    .id();
            },
        ));

        app.add_systems(
            Update,
            (
                player_bunny_ears_equip_system,
                floppy_ears_physics_bob_system,
            ),
        );

        app.update();

        let mut bunny_q = app.world_mut().query::<(Entity, &BunnyEarsItem, &Transform)>();
        let (item_e, _, item_tf) = bunny_q.iter(app.world()).next().expect("BunnyEarsItem must spawn in workshop");
        assert!((item_tf.translation.x - 1.8).abs() < 0.2);

        let mut player_q = app.world_mut().query::<(Entity, &mut ElfHands)>();
        let (p_e, mut hands) = player_q.single_mut(app.world_mut()).unwrap();
        hands.right_grabbed = Some(item_e);

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyH);
        }
        app.update();

        assert!(app.world().get::<BunnyEarsEquipped>(p_e).is_some(), "Player must have BunnyEarsEquipped");
        let mov = app.world().get::<ElfMovement>(p_e).unwrap();
        assert!(mov.has_bunny_ears, "Movement has_bunny_ears must be true");

        let hands = app.world().get::<ElfHands>(p_e).unwrap();
        assert_eq!(hands.right_grabbed, None, "Equipped item must be cleared from hand");

        let mut item_check_q = app.world_mut().query::<&BunnyEarsItem>();
        assert_eq!(item_check_q.iter(app.world()).count(), 0, "World pickup must be despawned on equip");

        let mut vm_ears_q = app.world_mut().query::<&ViewmodelBunnyEars>();
        assert_eq!(vm_ears_q.iter(app.world()).count(), 1, "ViewmodelBunnyEars must be spawned");

        let mut floppy_q = app.world_mut().query::<(&FloppyEar, &Transform)>();
        assert_eq!(floppy_q.iter(app.world()).count(), 2, "Must spawn left and right floppy ears");

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyH);
            keys.reset(KeyCode::KeyH);
            let mut mov = app.world_mut().get_mut::<ElfMovement>(p_e).unwrap();
            mov.velocity = Vec3::new(4.0, 3.5, 0.0);
        }
        app.update();

        let config = BhopConfig {
            window: BhopWindow::Sixtieth,
            max_boost_multiplier: 1.414,
        };
        let mut mov_equipped = ElfMovement {
            is_grounded: true,
            time_since_landing: 0.08,
            has_bunny_ears: true,
            ..default()
        };
        update_elf_physics(0.016, Vec3::ZERO, true, false, &config, &mut mov_equipped);
        assert_eq!(mov_equipped.consecutive_bhops, 1, "Bunnyhop must succeed within 1/10s grace period when bunny ears equipped");
        assert!((mov_equipped.velocity.y - 7.7).abs() < 0.01, "Player with bunny ears equipped must jump 10% higher (7.7 vs 7.0)");

        let mut mov_air = ElfMovement {
            is_grounded: false,
            air_strain_accel: 5.0,
            has_bunny_ears: true,
            ..default()
        };
        update_elf_physics(0.1, Vec3::new(1.0, 0.0, 0.0), false, false, &config, &mut mov_air);
        assert!((mov_air.velocity.x - 1.0).abs() < 0.01, "Player with bunny ears must have 2x air straining (1.0 vs 0.5)");

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyH);
        }
        app.update();

        assert!(app.world().get::<BunnyEarsEquipped>(p_e).is_none(), "BunnyEarsEquipped must be removed on unequip");
        let mov_after = app.world().get::<ElfMovement>(p_e).unwrap();
        assert!(!mov_after.has_bunny_ears, "has_bunny_ears must be false after unequip");

        let mut vm_check_q = app.world_mut().query::<&ViewmodelBunnyEars>();
        assert_eq!(vm_check_q.iter(app.world()).count(), 0, "Viewmodel ears must be despawned on unequip");

        let mut item_respawn_q = app.world_mut().query::<&BunnyEarsItem>();
        assert_eq!(item_respawn_q.iter(app.world()).count(), 1, "BunnyEarsItem must drop back into world on unequip");
    }
}
