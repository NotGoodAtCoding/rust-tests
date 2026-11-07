use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::moose::{MooseFeedingTrough, MooseHunger, MooseHungerStage, MooseSnack};

#[derive(Component)]
pub struct MrMoose;

#[derive(Component)]
pub struct MooseHead;

#[derive(Component)]
pub struct WindowShutters;

#[derive(Component)]
pub struct LeftWindowShutter;

#[derive(Component)]
pub struct RightWindowShutter;

/// Spawns Mr. Moose outside the window on a snowy hill, completely unshaded solid brown plastic
pub fn spawn_mr_moose(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    // Solid unshaded brown plastic: unlit: true
    let moose_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.20, 0.10), // Solid toy brown
        unlit: true,
        ..default()
    });

    let snow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.95, 1.0),
        perceptual_roughness: 0.9,
        ..default()
    });

    let shutter_wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.45, 0.25), // Festive dark pine green shutters
        perceptual_roughness: 0.7,
        ..default()
    });

    let shutter_trim_wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.22, 0.12), // Warm wood accent battens
        perceptual_roughness: 0.8,
        ..default()
    });

    let iron_hardware = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.14), // Wrought iron hardware & hinges
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 30.0))),
        MeshMaterial3d(snow_material),
        Transform::from_xyz(-6.5, -0.5, -20.0),
        Collider::cuboid(15.0, 0.1, 15.0),
        RigidBody::Fixed,
        Name::new("OutsideSnowyHill"),
    ));

    // Dual Window Shutters: Sized just large enough for Mr. Moose's head (1.8m x 1.8m opening)
    // Left and Right hinged panels that rotate inward into the workshop room as Mr. Moose pokes in
    let shutter_panel_mesh = meshes.add(Cuboid::new(0.88, 1.76, 0.06));
    let shutter_batten_mesh = meshes.add(Cuboid::new(0.84, 0.10, 0.08));
    let shutter_handle_mesh = meshes.add(Cuboid::new(0.04, 0.24, 0.08));

    // Left Window Shutter Hinge (mounted at X = -7.4, Y = 2.9, Z = -13.0)
    commands
        .spawn((
            Transform::from_xyz(-7.4, 2.9, -13.0),
            Visibility::default(),
            LeftWindowShutter,
            WindowShutters,
            RigidBody::KinematicPositionBased,
            Name::new("LeftWindowShutterHinge"),
        ))
        .with_children(|shutter| {
            // Main shutter leaf panel (hinged at x = 0, extends to +x = 0.88)
            shutter.spawn((
                Mesh3d(shutter_panel_mesh.clone()),
                MeshMaterial3d(shutter_wood.clone()),
                Transform::from_xyz(0.44, 0.0, 0.0),
                Collider::cuboid(0.44, 0.88, 0.03),
                Name::new("LeftShutterPanel"),
            ));
            // Decorative battens
            shutter.spawn((
                Mesh3d(shutter_batten_mesh.clone()),
                MeshMaterial3d(shutter_trim_wood.clone()),
                Transform::from_xyz(0.44, 0.65, 0.02),
                Name::new("LeftShutterBattenTop"),
            ));
            shutter.spawn((
                Mesh3d(shutter_batten_mesh.clone()),
                MeshMaterial3d(shutter_trim_wood.clone()),
                Transform::from_xyz(0.44, -0.65, 0.02),
                Name::new("LeftShutterBattenBottom"),
            ));
            // Iron handle fixture
            shutter.spawn((
                Mesh3d(shutter_handle_mesh.clone()),
                MeshMaterial3d(iron_hardware.clone()),
                Transform::from_xyz(0.80, 0.0, 0.04),
                Name::new("LeftShutterHandle"),
            ));
        });

    // Right Window Shutter Hinge (mounted at X = -5.6, Y = 2.9, Z = -13.0)
    commands
        .spawn((
            Transform::from_xyz(-5.6, 2.9, -13.0),
            Visibility::default(),
            RightWindowShutter,
            WindowShutters,
            RigidBody::KinematicPositionBased,
            Name::new("RightWindowShutterHinge"),
        ))
        .with_children(|shutter| {
            // Main shutter leaf panel (hinged at x = 0, extends to -x = -0.88)
            shutter.spawn((
                Mesh3d(shutter_panel_mesh),
                MeshMaterial3d(shutter_wood),
                Transform::from_xyz(-0.44, 0.0, 0.0),
                Collider::cuboid(0.44, 0.88, 0.03),
                Name::new("RightShutterPanel"),
            ));
            // Decorative battens
            shutter.spawn((
                Mesh3d(shutter_batten_mesh.clone()),
                MeshMaterial3d(shutter_trim_wood.clone()),
                Transform::from_xyz(-0.44, 0.65, 0.02),
                Name::new("RightShutterBattenTop"),
            ));
            shutter.spawn((
                Mesh3d(shutter_batten_mesh),
                MeshMaterial3d(shutter_trim_wood),
                Transform::from_xyz(-0.44, -0.65, 0.02),
                Name::new("RightShutterBattenBottom"),
            ));
            // Iron handle fixture
            shutter.spawn((
                Mesh3d(shutter_handle_mesh),
                MeshMaterial3d(iron_hardware),
                Transform::from_xyz(-0.80, 0.0, 0.04),
                Name::new("RightShutterHandle"),
            ));
        });

    // Moose root entity: positioned outside window off to the side at X = -6.5, Z = -17.0
    let moose_body = meshes.add(Cuboid::new(1.8, 1.4, 2.8));
    let moose_head = meshes.add(Cuboid::new(1.0, 1.0, 1.4));
    let moose_antler = meshes.add(Cuboid::new(2.4, 0.2, 0.8));
    let moose_leg = meshes.add(Cylinder::new(0.2, 1.8));

    commands
        .spawn((
            Mesh3d(moose_body),
            MeshMaterial3d(moose_material.clone()),
            Transform::from_xyz(-6.5, 1.8, -17.0),
            Collider::cuboid(0.9, 0.7, 1.4),
            RigidBody::KinematicPositionBased,
            MrMoose,
            Name::new("MrMooseToy"),
        ))
        .with_children(|parent| {
            // Head (Tagged with MooseHead so it can animate / poke forward)
            parent.spawn((
                Mesh3d(moose_head),
                MeshMaterial3d(moose_material.clone()),
                Transform::from_xyz(0.0, 1.1, 1.6),
                MooseHead,
                Name::new("MooseHead"),
            ));

            // Wide stylized plastic antlers
            parent.spawn((
                Mesh3d(moose_antler),
                MeshMaterial3d(moose_material.clone()),
                Transform::from_xyz(0.0, 1.8, 1.5),
                Name::new("MooseAntlers"),
            ));

            // 4 Stilt Legs
            for (lx, lz) in [(-0.7, -1.0), (0.7, -1.0), (-0.7, 1.0), (0.7, 1.0)] {
                parent.spawn((
                    Mesh3d(moose_leg.clone()),
                    MeshMaterial3d(moose_material.clone()),
                    Transform::from_xyz(lx, -1.2, lz),
                    Name::new("MooseLeg"),
                ));
            }
        });

    // Feeding Trough / Chute in front of the window (X = -6.5, Z = -12.0)
    let trough_metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.35, 0.4),
        metallic: 0.7,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 0.5, 1.2))),
        MeshMaterial3d(trough_metal),
        Transform::from_xyz(-6.5, 0.3, -12.0),
        Collider::cuboid(1.0, 0.25, 0.6),
        RigidBody::Fixed,
        MooseFeedingTrough,
        Name::new("MooseFeedingTrough"),
    ));

    // Moose Snack canister placed in the pantry corner (X = -9.0, Z = 8.0)
    let snack_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.6, 0.1),
        perceptual_roughness: 0.3,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.35, 0.8))),
        MeshMaterial3d(snack_material),
        Transform::from_xyz(-9.0, 0.4, 8.0),
        Collider::cylinder(0.4, 0.35),
        RigidBody::Dynamic,
        MooseSnack::default(),
        Name::new("MooseSnackCanister"),
    ));
}

const MAX_PEEK_SHUTTER_ANGLE: f32 = 1.35; // ~77 degrees inward
const BREACHED_SHUTTER_ANGLE: f32 = 1.95; // ~112 degrees wide open

/// Animates Mr. Moose's position and hinged shutters based on hunger value (0..60)
pub fn animate_mr_moose_peeking_system(
    hunger: Res<MooseHunger>,
    mut q_moose: Query<&mut Transform, (With<MrMoose>, Without<MooseHead>, Without<LeftWindowShutter>, Without<RightWindowShutter>)>,
    mut q_head: Query<&mut Transform, (With<MooseHead>, Without<MrMoose>, Without<LeftWindowShutter>, Without<RightWindowShutter>)>,
    mut q_left_shutter: Query<&mut Transform, (With<LeftWindowShutter>, Without<RightWindowShutter>, Without<MrMoose>, Without<MooseHead>)>,
    mut q_right_shutter: Query<&mut Transform, (With<RightWindowShutter>, Without<LeftWindowShutter>, Without<MrMoose>, Without<MooseHead>)>,
) {
    if hunger.is_breached || hunger.stage == MooseHungerStage::BreachedRampage {
        // During breach rampage, shutters are blown wide open
        let left_rot = Quat::from_rotation_y(-BREACHED_SHUTTER_ANGLE);
        let right_rot = Quat::from_rotation_y(BREACHED_SHUTTER_ANGLE);
        for mut tf in &mut q_left_shutter {
            tf.rotation = left_rot;
        }
        for mut tf in &mut q_right_shutter {
            tf.rotation = right_rot;
        }
        return;
    }

    match hunger.stage {
        MooseHungerStage::BehindShutters => {
            // Shutters shut, Moose far back (Z = -17.0, X = -6.5)
            for mut tf in &mut q_left_shutter {
                tf.rotation = Quat::IDENTITY;
            }
            for mut tf in &mut q_right_shutter {
                tf.rotation = Quat::IDENTITY;
            }
            for mut m_tf in &mut q_moose {
                m_tf.translation.x = -6.5;
                m_tf.translation.z = -17.0;
            }
            for mut h_tf in &mut q_head {
                h_tf.translation.z = 1.6;
            }
        }
        MooseHungerStage::PeekingThrough => {
            // Shutters rotate inward into the room as Mr. Moose pokes his head in
            let progress = ((hunger.value - 20.0) / 30.0).clamp(0.0, 1.0);
            let angle = progress * MAX_PEEK_SHUTTER_ANGLE;
            let left_rot = Quat::from_rotation_y(-angle);
            let right_rot = Quat::from_rotation_y(angle);
            for mut tf in &mut q_left_shutter {
                tf.rotation = left_rot;
            }
            for mut tf in &mut q_right_shutter {
                tf.rotation = right_rot;
            }

            // Moose moves closer to window (from -17.0 to -14.5)
            for mut m_tf in &mut q_moose {
                m_tf.translation.x = -6.5;
                m_tf.translation.z = -17.0 + progress * 2.5;
            }
            // Head pokes through the window (Z goes from 1.6 up to 3.6, reaching into room at Z = -10.9)
            for mut h_tf in &mut q_head {
                h_tf.translation.z = 1.6 + progress * 2.0;
            }
        }
        MooseHungerStage::AgitatedHuffing => {
            // Shutters remain held open with slight huffing jitter
            let jitter = (hunger.value * 12.0).sin() * 0.03;
            let left_rot = Quat::from_rotation_y(-(MAX_PEEK_SHUTTER_ANGLE + jitter));
            let right_rot = Quat::from_rotation_y(MAX_PEEK_SHUTTER_ANGLE + jitter);
            for mut tf in &mut q_left_shutter {
                tf.rotation = left_rot;
            }
            for mut tf in &mut q_right_shutter {
                tf.rotation = right_rot;
            }

            for mut m_tf in &mut q_moose {
                m_tf.translation.x = -6.5;
                m_tf.translation.z = -14.5;
            }
            // Head rests fully poked through window (Z = 3.6)
            for mut h_tf in &mut q_head {
                h_tf.translation.z = 3.6;
            }
        }
        MooseHungerStage::WindowSlammed => {
            // Slammed shut: shutters snap shut, head pulled back immediately
            for mut tf in &mut q_left_shutter {
                tf.rotation = Quat::IDENTITY;
            }
            for mut tf in &mut q_right_shutter {
                tf.rotation = Quat::IDENTITY;
            }
            for mut m_tf in &mut q_moose {
                m_tf.translation.x = -6.5;
                m_tf.translation.z = -17.0;
            }
            for mut h_tf in &mut q_head {
                h_tf.translation.z = 1.6;
            }
        }
        MooseHungerStage::BreachedRampage => {}
    }
}
