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
            hinge_pos: Vec3::new(-2.9, 1.78, -13.0),
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
            hinge_pos: Vec3::new(-2.9, 0.58, -13.0),
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
                base_color: CABIN_LOG,
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

    // 1. Frame: Left jamb, right jamb, and upper header
    commands.spawn((
        Mesh3d(m.add(Cuboid::new(0.18, 2.45, 0.35))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(hinge_x - 0.05, 1.225, door_z),
        Collider::cuboid(0.09, 1.225, 0.175),
        RigidBody::Fixed,
        StableDoorFrame,
        Name::new("StableDoorLeftJamb"),
    ));

    commands.spawn((
        Mesh3d(m.add(Cuboid::new(0.18, 2.45, 0.35))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(-1.05, 1.225, door_z),
        Collider::cuboid(0.09, 1.225, 0.175),
        RigidBody::Fixed,
        StableDoorFrame,
        Name::new("StableDoorRightJamb"),
    ));

    commands.spawn((
        Mesh3d(m.add(Cuboid::new(2.10, 0.18, 0.40))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_xyz(door_center_x, 2.49, door_z),
        Collider::cuboid(1.05, 0.09, 0.20),
        RigidBody::Fixed,
        StableDoorFrame,
        Name::new("StableDoorHeaderLintel"),
    ));

    // 2. Bottom Half Section (Height: 1.16m, Y in [0.0, 1.18])
    let bottom_hinge_pos = Vec3::new(hinge_x, 0.58, door_z);
    let bottom_rest_pos = Vec3::new(door_center_x, 0.58, door_z);

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
            Collider::cuboid(half_w, 0.58, thickness * 0.5),
            RigidBody::Fixed,
            Name::new("StableDoorBottomHalf"),
        ))
        .with_children(|door| {
            // Main solid timber plank body
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width, 1.16, thickness))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::default(),
                Name::new("DoorBottomPlanks"),
            ));

            // Diagonal Z-brace crossbeam
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width * 0.95, 0.12, 0.04))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, 0.0, thickness * 0.55).with_rotation(Quat::from_rotation_z(0.45)),
                Name::new("DoorBottomZBrace"),
            ));

            // Heavy black iron strap hinges on the left
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, 0.35, thickness * 0.55),
                Name::new("DoorBottomHingeStrapTop"),
            ));
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, -0.35, thickness * 0.55),
                Name::new("DoorBottomHingeStrapLow"),
            ));

            // Iron latch handle on right
            door.spawn((
                Mesh3d(m.add(Cylinder::new(0.025, 0.25))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(half_w - 0.18, 0.35, thickness * 0.65),
                Name::new("DoorBottomLatchHandle"),
            ));
        })
        .id();

    // 3. Top Half Section (Height: 1.16m, Y in [1.22, 2.40])
    let top_hinge_pos = Vec3::new(hinge_x, 1.78, door_z);
    let top_rest_pos = Vec3::new(door_center_x, 1.78, door_z);

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
            Collider::cuboid(half_w, 0.58, thickness * 0.5),
            RigidBody::Fixed,
            Name::new("StableDoorTopHalf"),
        ))
        .with_children(|door| {
            // Main solid timber plank body
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width, 1.16, thickness))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::default(),
                Name::new("DoorTopPlanks"),
            ));

            // Diagonal Z-brace crossbeam
            door.spawn((
                Mesh3d(m.add(Cuboid::new(width * 0.95, 0.12, 0.04))),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, 0.0, thickness * 0.55).with_rotation(Quat::from_rotation_z(-0.45)),
                Name::new("DoorTopZBrace"),
            ));

            // Heavy black iron strap hinges on the left
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, 0.35, thickness * 0.55),
                Name::new("DoorTopHingeStrapTop"),
            ));
            door.spawn((
                Mesh3d(m.add(Cuboid::new(0.55, 0.08, 0.03))),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_xyz(-half_w + 0.25, -0.35, thickness * 0.55),
                Name::new("DoorTopHingeStrapLow"),
            ));

            // Iron latch handle on right
            door.spawn((
                Mesh3d(m.add(Cylinder::new(0.025, 0.25))),
                MeshMaterial3d(iron_mat),
                Transform::from_xyz(half_w - 0.18, -0.35, thickness * 0.65),
                Name::new("DoorTopLatchHandle"),
            ));
        });

    let _ = bottom_half_e;
}

/// Controls the opening and physics movement of the Stable Door top and bottom halves.
/// - During peeking / hunger agitation: Only the top half opens!
/// - During rampage breach: Both top and bottom fly open with large impulse (2 radian range).
/// - When fed / rampage ends: Both top and bottom set to fully closed.
pub fn stable_door_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    hunger: Option<Res<MooseHunger>>,
    mut q_top: Query<(&mut StableDoorTopHalf, &mut Transform), Without<StableDoorBottomHalf>>,
    mut q_bottom: Query<(&mut StableDoorBottomHalf, &mut Transform), Without<StableDoorTopHalf>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let raw_dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let dt = if raw_dt > 0.0 { raw_dt } else { 0.016 };

    let (is_peeking, is_rampage, is_closed) = if let Some(h) = hunger {
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
            // Smoothly swing top half open for peeking (~1.57 rad = 90 deg)
            top.target_angle = 1.57;
            let speed = (1.0 - (-8.0 * dt).exp()).clamp(0.0, 1.0);
            top.current_angle += (top.target_angle - top.current_angle) * speed;
        } else if is_closed {
            top.is_physics_driven = false;
            top.angular_velocity = 0.0;
            top.target_angle = 0.0;
            let speed = (1.0 - (-10.0 * dt).exp()).clamp(0.0, 1.0);
            top.current_angle += (top.target_angle - top.current_angle) * speed;
            if top.current_angle.abs() < 0.01 {
                top.current_angle = 0.0;
            }
        }

        // Apply rotation around hinge pivot H
        let hinge = top.hinge_pos;
        let rot = Quat::from_rotation_y(top.current_angle);
        let offset = rot * Vec3::new(DOOR_HALF_WIDTH, 0.0, 0.0);
        tf.translation = hinge + offset;
        tf.rotation = rot;
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
        let hinge = bottom.hinge_pos;
        let rot = Quat::from_rotation_y(bottom.current_angle);
        let offset = rot * Vec3::new(DOOR_HALF_WIDTH, 0.0, 0.0);
        tf.translation = hinge + offset;
        tf.rotation = rot;
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
