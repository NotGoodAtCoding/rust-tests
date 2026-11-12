use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    player::hands::Grabbable,
    moose::{MooseHunger, MooseHungerStage},
};

/// Top half section of the Stable Door (Dutch Door) where Mr. Moose peeks his head through.
/// Opens independently during peeking while bottom half stays closed.
#[derive(Component, Debug, Clone)]
pub struct StableDoorTopHalf {
    pub current_angle: f32, // 0.0 = closed, up to ~2.0 rad open
    pub target_angle: f32,
    pub angular_velocity: f32,
    pub is_physics_driven: bool,
    pub hinge_pos: Vec3,
}

impl Default for StableDoorTopHalf {
    fn default() -> Self {
        Self {
            current_angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            is_physics_driven: false,
            hinge_pos: Vec3::new(-2.9, 2.61, -13.0),
        }
    }
}

/// Bottom half section of the Stable Door. Remains closed during peeking,
/// but flies open during Mr. Moose rampages to allow the player outside.
#[derive(Component, Debug, Clone)]
pub struct StableDoorBottomHalf {
    pub current_angle: f32, // 0.0 = closed, up to ~2.0 rad open
    pub target_angle: f32,
    pub angular_velocity: f32,
    pub is_physics_driven: bool,
    pub hinge_pos: Vec3,
}

impl Default for StableDoorBottomHalf {
    fn default() -> Self {
        Self {
            current_angle: 0.0,
            target_angle: 0.0,
            angular_velocity: 0.0,
            is_physics_driven: false,
            hinge_pos: Vec3::new(-2.9, 0.87, -13.0),
        }
    }
}

/// Tag component for the stable door timber jambs and casing frame.
#[derive(Component, Debug, Clone)]
pub struct StableDoorFrame;

/// Spawns the rustic Stable Door (Dutch Door) at the North wall (X = -2.0, Z = -13.0),
/// with independently operating top and bottom half sections.
pub fn spawn_stable_door(
    commands: &mut Commands,
    meshes: &mut Option<ResMut<Assets<Mesh>>>,
    materials: &mut Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<&WorkshopMaterials>,
) {
    let (wood_mat, iron_mat) = if let Some(wm) = workshop_mats {
        (wm.workbench_wood.clone(), wm.cast_iron.clone())
    } else if let Some(mats) = materials.as_deref_mut() {
        (
            mats.add(StandardMaterial {
                base_color: PINE_GREEN,
                perceptual_roughness: 0.75,
                ..default()
            }),
            mats.add(StandardMaterial {
                base_color: CAST_IRON,
                metallic: 0.8,
                perceptual_roughness: 0.4,
                ..default()
            }),
        )
    } else {
        return;
    };

    let Some(m) = meshes else {
        return;
    };

    let door_center_x = -2.0;
    let door_z = -13.0;
    let hinge_x = -2.9;
    let width = 1.76;
    let half_w = width * 0.5;
    let thickness = 0.12;

    // 1. Frame: Left jamb, right jamb, and upper header (Scaled up by 1.5 to 3.66m)
    commands.spawn((
        Mesh3d(m.add(Cuboid::new(0.18, 3.66, 0.35))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(hinge_x - 0.05, 1.83, door_z),
        Collider::cuboid(0.09, 1.83, 0.175),
        RigidBody::Fixed,
        StableDoorFrame,
        Name::new("StableDoorLeftJamb"),
    ));

    commands.spawn((
        Mesh3d(m.add(Cuboid::new(0.18, 3.66, 0.35))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(-1.05, 1.83, door_z),
        Collider::cuboid(0.09, 1.83, 0.175),
        RigidBody::Fixed,
        StableDoorFrame,
        Name::new("StableDoorRightJamb"),
    ));

    commands.spawn((
        Mesh3d(m.add(Cuboid::new(2.10, 0.18, 0.40))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(door_center_x, 3.57, door_z),
        Collider::cuboid(1.05, 0.09, 0.20),
        RigidBody::Fixed,
        StableDoorFrame,
        Name::new("StableDoorHeaderLintel"),
    ));

    // 2. Bottom Half Section (Height: 1.74m = 1.16m * 1.5, Y in [0.0, 1.74])
    let bottom_hinge_pos = Vec3::new(hinge_x, 0.87, door_z);
    let bottom_rest_pos = Vec3::new(door_center_x, 0.87, door_z);

    let bottom_half_e = commands
        .spawn((
            StableDoorBottomHalf {
                current_angle: 0.0,
                target_angle: 0.0,
                angular_velocity: 0.0,
                is_physics_driven: false,
                hinge_pos: bottom_hinge_pos,
            },
            Transform::from_translation(bottom_rest_pos),
            Visibility::default(),
            Grabbable, // Grabbable by player, but not movable
            Collider::cuboid(half_w, 0.87, thickness * 0.5),
            RigidBody::Fixed,
            Name::new("StableDoorBottomHalf"),
        ))
        .with_children(|door| {
            // Main solid timber plank body (Height: 1.74m)
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width, 1.74, thickness))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::default(),
                Name::new("DoorBottomPlanks"),
            ));

            // Diagonal Z-brace crossbeam
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width * 0.95, 0.14, 0.04))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, 0.0, thickness * 0.55).with_rotation(Quat::from_rotation_z(0.45)),
                Name::new("DoorBottomZBrace"),
            ));

            // Heavy black iron strap hinges on the left
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, 0.525, thickness * 0.55),
                Name::new("DoorBottomHingeStrapTop"),
            ));
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, -0.525, thickness * 0.55),
                Name::new("DoorBottomHingeStrapLow"),
            ));

            // Iron latch handle on right
            door.spawn((
                Mesh3d(m.add(Cylinder::new(0.025, 0.25))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(half_w - 0.18, 0.525, thickness * 0.65),
                Name::new("DoorBottomLatchHandle"),
            ));
        })
        .id();

    // 3. Top Half Section (Height: 1.74m = 1.16m * 1.5, Y in [1.74, 3.48])
    let top_hinge_pos = Vec3::new(hinge_x, 2.61, door_z);
    let top_rest_pos = Vec3::new(door_center_x, 2.61, door_z);

    commands
        .spawn((
            StableDoorTopHalf {
                current_angle: 0.0,
                target_angle: 0.0,
                angular_velocity: 0.0,
                is_physics_driven: false,
                hinge_pos: top_hinge_pos,
            },
            Transform::from_translation(top_rest_pos),
            Visibility::default(),
            Grabbable, // Grabbable by player, but not movable
            Collider::cuboid(half_w, 0.87, thickness * 0.5),
            RigidBody::Fixed,
            Name::new("StableDoorTopHalf"),
        ))
        .with_children(|door| {
            // Main solid timber plank body (Height: 1.74m)
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width, 1.74, thickness))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::default(),
                Name::new("DoorTopPlanks"),
            ));

            // Diagonal Z-brace crossbeam
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width * 0.95, 0.14, 0.04))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, 0.0, thickness * 0.55).with_rotation(Quat::from_rotation_z(-0.45)),
                Name::new("DoorTopZBrace"),
            ));

            // Heavy black iron strap hinges on the left
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, 0.525, thickness * 0.55),
                Name::new("DoorTopHingeStrapTop"),
            ));
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, -0.525, thickness * 0.55),
                Name::new("DoorTopHingeStrapLow"),
            ));

            // Iron latch handle on right
            door.spawn((
                Mesh3d(m.add(Cylinder::new(0.025, 0.25))),
                MeshMaterial3d(iron_mat),
                Transform::from_xyz(half_w - 0.18, -0.525, thickness * 0.65),
                Name::new("DoorTopLatchHandle"),
            ));
        });

    let _ = bottom_half_e;
}

/// Applies hinge pivot transform around `hinge_pos` so positive `open_angle`
/// swings the `+X` door leaf inward (`+Z`, into the workshop).
fn apply_hinge_transform(tf: &mut Transform, hinge: Vec3, open_angle: f32, half_width: f32) {
    let yaw = if open_angle == 0.0 { 0.0 } else { -open_angle };
    let rot = Quat::from_rotation_y(yaw);
    let offset = rot * Vec3::new(half_width, 0.0, 0.0);
    tf.translation = hinge + offset;
    tf.rotation = rot;
}

/// Controls the opening and physics movement of the Stable Door top and bottom halves.
/// - During peeking / hunger agitation: Only the top half opens as Mr. Moose pokes his head in!
/// - When Mr. Moose snaps back or retreats: The top half slams shut with high velocity and impact.
/// - During rampage breach: Both top and bottom fly open with large impulse (2 radian range).
/// - When fed / rampage ends: Both top and bottom set to fully closed.
pub fn stable_door_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    hunger: Option<Res<MooseHunger>>,
    q_moose: Query<&Transform, With<crate::elf_game::moose::toy_model::MrMoose>>,
    q_head: Query<&Transform, With<crate::elf_game::moose::toy_model::MooseHead>>,
    mut slam_events: Option<MessageWriter<crate::elf_game::moose::MooseWindowSlamEvent>>,
    mut q_top: Query<
        (&mut StableDoorTopHalf, &mut Transform),
        (
            Without<StableDoorBottomHalf>,
            Without<crate::elf_game::moose::toy_model::MrMoose>,
            Without<crate::elf_game::moose::toy_model::MooseHead>,
        ),
    >,
    mut q_bottom: Query<
        (&mut StableDoorBottomHalf, &mut Transform),
        (
            Without<StableDoorTopHalf>,
            Without<crate::elf_game::moose::toy_model::MrMoose>,
            Without<crate::elf_game::moose::toy_model::MooseHead>,
        ),
    >,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let raw_dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let dt = if raw_dt > 0.0 { raw_dt } else { 0.016 };

    let (is_peeking, is_rampage, is_closed) = if let Some(h) = hunger.as_ref() {
        let peeking = h.stage == MooseHungerStage::PeekingThrough
            || h.stage == MooseHungerStage::AgitatedHuffing
            || h.stage == MooseHungerStage::DeliberatingLateTreat
            || h.stage == MooseHungerStage::EatingTreat;
        let rampage = h.is_breached || h.stage == MooseHungerStage::BreachedRampage;
        let closed = h.stage == MooseHungerStage::BehindShutters || h.stage == MooseHungerStage::WindowSlammed;
        (peeking, rampage, closed)
    } else {
        (false, false, true)
    };

    const DOOR_HALF_WIDTH: f32 = 0.88;
    const MAX_OPEN_ANGLE: f32 = 2.0;

    // Calculate how far Mr. Moose's head is poking forward through the doorway
    let moose_snout_z = match (q_moose.iter().next(), q_head.iter().next()) {
        (Some(moose_tf), Some(head_tf)) => Some(moose_tf.translation.z + head_tf.translation.z),
        _ => None,
    };

    // 1. Process Top Half
    for (mut top, mut tf) in &mut q_top {
        if is_rampage {
            if !top.is_physics_driven {
                // Large impulse from Moose breaching!
                top.is_physics_driven = true;
                top.angular_velocity = 9.5;
            }
            // Free swinging physics within 2 radian range
            top.current_angle += top.angular_velocity * dt;
            top.angular_velocity *= (1.0 - 1.2 * dt).clamp(0.0, 1.0); // Damping

            // Bounce off 0.0 and 2.0 limits
            if top.current_angle >= MAX_OPEN_ANGLE {
                top.current_angle = MAX_OPEN_ANGLE;
                top.angular_velocity = -top.angular_velocity.abs() * 0.35;
            } else if top.current_angle <= 0.0 {
                top.current_angle = 0.0;
                top.angular_velocity = top.angular_velocity.abs() * 0.35;
            }
        } else if is_peeking {
            top.is_physics_driven = false;
            top.angular_velocity = 0.0;

            // Top half opens smoothly as Mr. Moose slowly pokes his head into the workshop doorway
            let open_factor = if let Some(ref h) = hunger {
                if h.stage == MooseHungerStage::PeekingThrough {
                    if let Some(snout_z) = moose_snout_z {
                        ((snout_z - (-15.4)) / 3.4).clamp(0.0, 1.0)
                    } else {
                        ((h.value - 20.0) / 30.0).clamp(0.0, 1.0)
                    }
                } else {
                    1.0
                }
            } else if let Some(snout_z) = moose_snout_z {
                ((snout_z - (-15.4)) / 3.4).clamp(0.0, 1.0)
            } else {
                1.0
            };

            top.target_angle = 1.57 * open_factor;
            let speed = (1.0 - (-12.0 * dt).exp()).clamp(0.0, 1.0);
            top.current_angle += (top.target_angle - top.current_angle) * speed;
        } else if is_closed {
            top.is_physics_driven = false;
            // Slam shut with high velocity when Mr. Moose snaps back or goes back outside!
            const SLAM_SPEED: f32 = 25.0;
            let prev_angle = top.current_angle;
            if top.current_angle > 0.0 {
                top.current_angle -= SLAM_SPEED * dt;
                if top.current_angle <= 0.0 {
                    top.current_angle = 0.0;
                    top.angular_velocity = 0.0;
                    // Trigger slam event / impact sound when striking frame
                    if prev_angle > 0.08 {
                        if let Some(ref mut writer) = slam_events {
                            writer.write(crate::elf_game::moose::MooseWindowSlamEvent);
                        }
                    }
                }
            } else {
                top.current_angle = 0.0;
                top.angular_velocity = 0.0;
            }
        }

        // Apply rotation around hinge pivot H
        apply_hinge_transform(&mut tf, top.hinge_pos, top.current_angle, DOOR_HALF_WIDTH);
    }

    // 2. Process Bottom Half
    for (mut bottom, mut tf) in &mut q_bottom {
        if is_rampage {
            if !bottom.is_physics_driven {
                // Large impulse from Moose breaching!
                bottom.is_physics_driven = true;
                bottom.angular_velocity = 9.0;
            }
            // Free swinging physics within 2 radian range
            bottom.current_angle += bottom.angular_velocity * dt;
            bottom.angular_velocity *= (1.0 - 1.2 * dt).clamp(0.0, 1.0);

            // Bounce off 0.0 and 2.0 limits
            if bottom.current_angle >= MAX_OPEN_ANGLE {
                bottom.current_angle = MAX_OPEN_ANGLE;
                bottom.angular_velocity = -bottom.angular_velocity.abs() * 0.35;
            } else if bottom.current_angle <= 0.0 {
                bottom.current_angle = 0.0;
                bottom.angular_velocity = bottom.angular_velocity.abs() * 0.35;
            }
        } else {
            // In peeking, behind shutters, or when fed: bottom half stays firmly closed!
            bottom.is_physics_driven = false;
            bottom.angular_velocity = 0.0;
            bottom.target_angle = 0.0;
            let speed = (1.0 - (-12.0 * dt).exp()).clamp(0.0, 1.0);
            bottom.current_angle += (bottom.target_angle - bottom.current_angle) * speed;
            if bottom.current_angle.abs() < 0.01 {
                bottom.current_angle = 0.0;
            }
        }

        // Apply rotation around hinge pivot H
        apply_hinge_transform(&mut tf, bottom.hinge_pos, bottom.current_angle, DOOR_HALF_WIDTH);
    }
}

/// Prevents the player from forcing the Stable Door open while it is grabbed.
/// The door is grabbable by the player, but only Mr. Moose can move it.
pub fn stable_door_interaction_system(
    mut commands: Commands,
    mut q_hands: Query<&mut crate::elf_game::player::ElfHands>,
    q_doors: Query<Entity, (With<Grabbable>, Or<(With<StableDoorTopHalf>, With<StableDoorBottomHalf>)>)>,
    q_held: Query<Entity, (With<crate::elf_game::player::hands::HeldByPlayer>, Or<(With<StableDoorTopHalf>, With<StableDoorBottomHalf>)>)>,
) {
    // If the player attempts to hold/move the door, release the hold so player movement forces cannot drag it
    for door_e in &q_held {
        commands.entity(door_e).remove::<crate::elf_game::player::hands::HeldByPlayer>();
    }

    for mut hands in &mut q_hands {
        if let Some(left) = hands.left_grabbed {
            if q_doors.get(left).is_ok() {
                hands.left_grabbed = None;
            }
        }
        if let Some(right) = hands.right_grabbed {
            if q_doors.get(right).is_ok() {
                hands.right_grabbed = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf_game::{
        moose::{
            MooseHunger, MooseHungerStage,
            toy_model::{MrMoose, MooseHead, animate_mr_moose_peeking_system},
        },
        player::{
            ElfHands,
            hands::{Grabbable, HeldByPlayer},
        },
    };

    #[test]
    fn test_mr_moose_center_wall_and_stable_door_mechanics() {
        let mut app = App::new();
        app.init_resource::<Time>();
        let hunger = MooseHunger {
            value: 35.0,
            max_value: 60.0,
            stage: MooseHungerStage::PeekingThrough,
            ..default()
        };
        app.insert_resource(hunger);

        app.add_systems(Update, (stable_door_system, stable_door_interaction_system));

        // Spawn Stable Door Top Half
        let top_door_e = app.world_mut().spawn((
            StableDoorTopHalf {
                current_angle: 0.0,
                target_angle: 0.0,
                angular_velocity: 0.0,
                is_physics_driven: false,
                hinge_pos: Vec3::new(-2.9, 1.78, -13.0),
            },
            Transform::from_xyz(-2.0, 1.78, -13.0),
            Grabbable,
        )).id();

        // Spawn Stable Door Bottom Half
        let bottom_door_e = app.world_mut().spawn((
            StableDoorBottomHalf {
                current_angle: 0.0,
                target_angle: 0.0,
                angular_velocity: 0.0,
                is_physics_driven: false,
                hinge_pos: Vec3::new(-2.9, 0.58, -13.0),
            },
            Transform::from_xyz(-2.0, 0.58, -13.0),
            Grabbable,
        )).id();

        // 1. Advance frames during Peeking stage
        for _ in 0..10 {
            app.update();
        }

        let top = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
        let bottom = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
        assert!(top.current_angle > 0.5, "Top half must open during peeking, got {}", top.current_angle);
        assert_eq!(bottom.current_angle, 0.0, "Bottom half must remain firmly closed during peeking");

        // 2. Trigger Rampage Breach
        {
            let mut h = app.world_mut().resource_mut::<MooseHunger>();
            h.is_breached = true;
            h.stage = MooseHungerStage::BreachedRampage;
        }

        app.update();

        let top_breach = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
        let bottom_breach = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
        assert!(top_breach.is_physics_driven, "Top half becomes physics driven on rampage");
        assert!(bottom_breach.is_physics_driven, "Bottom half becomes physics driven on rampage");
        assert!(top_breach.angular_velocity > 5.0, "Top half receives large impulse");
        assert!(bottom_breach.angular_velocity > 5.0, "Bottom half receives large impulse");

        // Run physics updates and verify angle stays within 2.0 radian range
        for _ in 0..20 {
            app.update();
        }
        let top_swung = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
        let bottom_swung = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
        assert!(top_swung.current_angle <= 2.0, "Top angle must not exceed 2.0 rad, got {}", top_swung.current_angle);
        assert!(bottom_swung.current_angle <= 2.0, "Bottom angle must not exceed 2.0 rad, got {}", bottom_swung.current_angle);

        // 3. Moose fed / rampage ends: return to closed
        {
            let mut h = app.world_mut().resource_mut::<MooseHunger>();
            h.is_breached = false;
            h.value = 0.0;
            h.stage = MooseHungerStage::BehindShutters;
        }

        for _ in 0..30 {
            app.update();
        }
        let top_closed = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
        let bottom_closed = app.world().get::<StableDoorBottomHalf>(bottom_door_e).unwrap();
        assert!(top_closed.current_angle < 0.05, "Top half returns to fully closed when rampage ends, got {}", top_closed.current_angle);
        assert!(bottom_closed.current_angle < 0.05, "Bottom half returns to fully closed when rampage ends, got {}", bottom_closed.current_angle);

        // 4. Test grabbable but not movable by player
        let hands_e = app.world_mut().spawn(ElfHands {
            left_grabbed: Some(bottom_door_e),
            right_grabbed: Some(top_door_e),
            ..default()
        }).id();
        app.world_mut().entity_mut(top_door_e).insert(HeldByPlayer);
        app.world_mut().entity_mut(bottom_door_e).insert(HeldByPlayer);

        app.update();

        let hands = app.world().get::<ElfHands>(hands_e).unwrap();
        assert!(hands.left_grabbed.is_none(), "Player cannot hold bottom door");
        assert!(hands.right_grabbed.is_none(), "Player cannot hold top door");
        assert!(app.world().get::<HeldByPlayer>(top_door_e).is_none());
        assert!(app.world().get::<HeldByPlayer>(bottom_door_e).is_none());
    }

    #[test]
    fn test_stable_door_and_moose_slow_peeking_over_30_seconds() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);

        let hunger = MooseHunger {
            value: 10.0,
            max_value: 60.0,
            stage: MooseHungerStage::BehindShutters,
            ..default()
        };
        app.insert_resource(hunger);

        app.add_systems(Update, (animate_mr_moose_peeking_system, stable_door_system));

        let moose_e = app.world_mut().spawn((
            MrMoose,
            Transform::from_xyz(-2.0, 1.8, -17.0),
        )).id();

        let head_e = app.world_mut().spawn((
            MooseHead,
            Transform::from_xyz(0.0, 1.1, 1.6),
        )).id();

        let top_door_e = app.world_mut().spawn((
            StableDoorTopHalf {
                current_angle: 0.0,
                target_angle: 0.0,
                angular_velocity: 0.0,
                is_physics_driven: false,
                hinge_pos: Vec3::new(-2.9, 1.78, -13.0),
            },
            Transform::from_xyz(-2.0, 1.78, -13.0),
        )).id();

        // 1. In BehindShutters (hunger = 10.0), door is completely closed and Moose is far back outside
        app.update();
        {
            let top = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
            let moose_tf = app.world().get::<Transform>(moose_e).unwrap();
            let head_tf = app.world().get::<Transform>(head_e).unwrap();
            assert_eq!(top.current_angle, 0.0);
            assert!((moose_tf.translation.z - (-17.0)).abs() < 0.05);
            assert!((head_tf.translation.z - 1.6).abs() < 0.05);
        }

        // 2. Step through peeking over 30 seconds as hunger increases from 20.0 to 50.0
        // Verify monotonic forward progression and slow door opening
        let check_points = [
            (20.0, 0.0, -17.0, 1.6),
            (25.0, 0.26, -16.58, 1.75),
            (35.0, 0.785, -15.75, 2.05),
            (45.0, 1.308, -14.92, 2.35),
            (50.0, 1.57, -14.5, 2.5),
        ];

        let mut prev_angle = -0.01;
        let mut prev_moose_z = -17.1;
        let mut prev_head_z = 1.5;

        for (hunger_val, expected_angle, expected_moose_z, expected_head_z) in check_points {
            {
                let mut h = app.world_mut().resource_mut::<MooseHunger>();
                h.value = hunger_val;
                h.stage = MooseHungerStage::PeekingThrough;
            }

            // Run several ticks for smooth follow
            for _ in 0..20 {
                app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
                app.update();
            }

            let top = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
            let moose_tf = app.world().get::<Transform>(moose_e).unwrap();
            let head_tf = app.world().get::<Transform>(head_e).unwrap();

            // Must be strictly monotonic
            assert!(top.current_angle >= prev_angle, "Door angle must advance monotonically (at hunger {hunger_val}: got {}, prev {})", top.current_angle, prev_angle);
            assert!(moose_tf.translation.z >= prev_moose_z, "Moose Z must advance monotonically (at hunger {hunger_val}: got {}, prev {})", moose_tf.translation.z, prev_moose_z);
            assert!(head_tf.translation.z >= prev_head_z, "Head Z must advance monotonically (at hunger {hunger_val}: got {}, prev {})", head_tf.translation.z, prev_head_z);

            // Approximate checks at key points
            assert!(
                (top.current_angle - expected_angle).abs() < 0.12,
                "Door angle at hunger {hunger_val} expected ~{expected_angle}, got {}",
                top.current_angle
            );
            assert!(
                (moose_tf.translation.z - expected_moose_z).abs() < 0.15,
                "Moose Z at hunger {hunger_val} expected ~{expected_moose_z}, got {}",
                moose_tf.translation.z
            );
            assert!(
                (head_tf.translation.z - expected_head_z).abs() < 0.15,
                "Head Z at hunger {hunger_val} expected ~{expected_head_z}, got {}",
                head_tf.translation.z
            );

            prev_angle = top.current_angle;
            prev_moose_z = moose_tf.translation.z;
            prev_head_z = head_tf.translation.z;
        }

        // 3. Fast slam when window is slammed at hunger = 60
        {
            let mut h = app.world_mut().resource_mut::<MooseHunger>();
            h.value = 60.0;
            h.stage = MooseHungerStage::WindowSlammed;
        }

        for _ in 0..15 {
            app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
            app.update();
        }

        let top_slammed = app.world().get::<StableDoorTopHalf>(top_door_e).unwrap();
        let moose_slammed = app.world().get::<Transform>(moose_e).unwrap();
        let head_slammed = app.world().get::<Transform>(head_e).unwrap();

        assert!(top_slammed.current_angle < 0.05, "Top stable door must slam shut within 0.25s, got {}", top_slammed.current_angle);
        assert!(moose_slammed.translation.z < -16.8, "Moose body must snap back outside within 0.25s, got {}", moose_slammed.translation.z);
        assert!(head_slammed.translation.z < 1.7, "Moose head must snap back behind shutters within 0.25s, got {}", head_slammed.translation.z);
    }

    #[test]
    fn test_taller_stable_door_scaling() {
        use crate::elf_game::{
            assets::AssetsPlugin,
            moose::toy_model::spawn_mr_moose,
            workshop::room::setup_workshop_scene,
        };

        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_plugins(AssetsPlugin);
        app.add_systems(Startup, (setup_workshop_scene, spawn_mr_moose));
        app.update();

        let mut door_q = app.world_mut().query::<(&Name, &Transform, &Collider)>();
        let mut found_top = false;
        let mut found_bottom = false;
        for (name, tf, _col) in door_q.iter(app.world()) {
            match name.as_str() {
                "StableDoorTopHalf" => {
                    found_top = true;
                    assert!((tf.translation.y - 2.61).abs() < 0.1, "Top half must be centered at Y ≈ 2.61");
                }
                "StableDoorBottomHalf" => {
                    found_bottom = true;
                    assert!((tf.translation.y - 0.87).abs() < 0.1, "Bottom half must be centered at Y ≈ 0.87");
                }
                _ => {}
            }
        }
        assert!(found_top && found_bottom, "Both door sections must be present and scaled");
    }

    #[test]
    fn test_stable_door_swings_inward_into_workshop() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.insert_resource(MooseHunger {
            value: 45.0,
            max_value: 60.0,
            stage: MooseHungerStage::PeekingThrough,
            ..default()
        });
        app.add_systems(Update, stable_door_system);

        let top_hinge = Vec3::new(-2.9, 2.61, -13.0);
        let bottom_hinge = Vec3::new(-2.9, 0.87, -13.0);

        let top_door_e = app.world_mut().spawn((
            StableDoorTopHalf {
                hinge_pos: top_hinge,
                ..default()
            },
            Transform::from_xyz(-2.02, 2.61, -13.0),
        )).id();

        let bottom_door_e = app.world_mut().spawn((
            StableDoorBottomHalf {
                hinge_pos: bottom_hinge,
                ..default()
            },
            Transform::from_xyz(-2.02, 0.87, -13.0),
        )).id();

        // 1. Peeking stage: top half swings inward (+Z, into workshop), not outward (-Z, toward Moose)
        for _ in 0..15 {
            app.update();
        }

        let top_tf = *app.world().get::<Transform>(top_door_e).unwrap();
        let top_hinge_edge = top_tf.transform_point(Vec3::new(-0.88, 0.0, 0.0));
        let top_latch_edge = top_tf.transform_point(Vec3::new(0.88, 0.0, 0.0));
        assert!(
            top_hinge_edge.distance(top_hinge) < 0.01,
            "Top door hinge edge must stay pinned at hinge_pos {:?}, got {:?}",
            top_hinge,
            top_hinge_edge
        );
        assert!(
            top_tf.translation.z > -13.0 && top_latch_edge.z > -12.0,
            "Top door must swing inward into +Z workshop interior when moose peeks, got center z={}, tip z={}",
            top_tf.translation.z,
            top_latch_edge.z
        );

        let bottom_tf = *app.world().get::<Transform>(bottom_door_e).unwrap();
        assert!(
            (bottom_tf.translation.z - (-13.0)).abs() < 0.01,
            "Bottom door must stay closed at z = -13.0 during peeking, got {}",
            bottom_tf.translation.z
        );

        // 2. Rampage breach: both halves swing inward (+Z, into workshop)
        {
            let mut h = app.world_mut().resource_mut::<MooseHunger>();
            h.is_breached = true;
            h.stage = MooseHungerStage::BreachedRampage;
        }
        for _ in 0..5 {
            app.update();
        }

        let top_rampage_tf = *app.world().get::<Transform>(top_door_e).unwrap();
        let bottom_rampage_tf = *app.world().get::<Transform>(bottom_door_e).unwrap();
        let top_tip = top_rampage_tf.transform_point(Vec3::new(0.88, 0.0, 0.0));
        let bottom_tip = bottom_rampage_tf.transform_point(Vec3::new(0.88, 0.0, 0.0));
        let bottom_hinge_edge = bottom_rampage_tf.transform_point(Vec3::new(-0.88, 0.0, 0.0));

        assert!(
            bottom_hinge_edge.distance(bottom_hinge) < 0.01,
            "Bottom door hinge edge must stay pinned at hinge_pos {:?}, got {:?}",
            bottom_hinge,
            bottom_hinge_edge
        );
        assert!(
            top_tip.z > -12.0 && bottom_tip.z > -12.0,
            "Both stable door halves must swing inward (+Z) during rampage breach, got top tip z={}, bottom tip z={}",
            top_tip.z,
            bottom_tip.z
        );
    }
}
