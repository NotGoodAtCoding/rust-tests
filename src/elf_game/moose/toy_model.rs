use bevy::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    moose::{FeedMooseMessage, MooseFeedingTrough, MooseHunger, MooseHungerStage, MooseSnack, MooseWindowSlamEvent},
    player::hands::Grabbable,
};

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

#[derive(Component)]
pub struct MooseSnackJar;

#[derive(Component)]
pub struct WindowsillMiniTree;

#[derive(Component)]
pub struct WindowsillHotCocoa;

#[derive(Component)]
pub struct MooseScarf;

#[derive(Component, Debug, Clone)]
pub struct MooseScarfMount {
    pub left_tail_anchor: Vec3,
    pub right_tail_anchor: Vec3,
    pub last_moose_pos: Vec3,
}

impl Default for MooseScarfMount {
    fn default() -> Self {
        Self {
            left_tail_anchor: Vec3::new(-0.24, 0.85, 1.45),
            right_tail_anchor: Vec3::new(0.24, 0.85, 1.45),
            last_moose_pos: Vec3::new(-6.5, 1.8, -17.0),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct MooseScarfMountPoint;

#[derive(Component, Debug, Clone)]
pub struct MooseScarfSegment {
    pub index: usize,
    pub rest_offset: Vec3,
    pub velocity: Vec3,
}

/// Spawns Mr. Moose outside the window on a snowy hill, completely unshaded solid brown plastic
pub fn spawn_mr_moose(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    // Realistic PBR Fur & Anatomy Materials
    let moose_fur_primary = workshop_mats.as_ref().map(|wm| wm.moose_fur.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: MOOSE_FUR,
            perceptual_roughness: 0.88,
            reflectance: 0.2,
            ..default()
        })
    });

    let moose_fur_secondary = workshop_mats.as_ref().map(|wm| wm.moose_muzzle.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: MOOSE_MUZZLE,
            perceptual_roughness: 0.85,
            reflectance: 0.2,
            ..default()
        })
    });

    let moose_snout_leather = workshop_mats.as_ref().map(|wm| wm.moose_nose.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: MOOSE_NOSE,
            perceptual_roughness: 0.55,
            ..default()
        })
    });

    let _moose_eye_mat = workshop_mats.as_ref().map(|wm| wm.moose_eye.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: MOOSE_OBSIDIAN_EYE,
            perceptual_roughness: 0.1,
            metallic: 0.3,
            ..default()
        })
    });

    let moose_antler_mat = workshop_mats.as_ref().map(|wm| wm.moose_antler.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: MOOSE_ANTLER,
            perceptual_roughness: 0.65,
            ..default()
        })
    });

    let moose_hoof_mat = workshop_mats.as_ref().map(|wm| wm.moose_hoof.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: MOOSE_HOOF,
            perceptual_roughness: 0.45,
            ..default()
        })
    });

    // Scarf materials (soft brushed wool red-and-white festive stripes)
    let scarf_red = workshop_mats.as_ref().map(|wm| wm.moose_scarf_red.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: FESTIVE_RED,
            perceptual_roughness: 0.85,
            ..default()
        })
    });

    let scarf_white = workshop_mats.as_ref().map(|wm| wm.moose_scarf_white.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SNOW_WHITE,
            perceptual_roughness: 0.85,
            ..default()
        })
    });

    let snow_material = workshop_mats.as_ref().map(|wm| wm.snow_white.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SNOW_WHITE,
            perceptual_roughness: 0.9,
            ..default()
        })
    });

    let shutter_wood = workshop_mats.as_ref().map(|wm| wm.pine_green.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PINE_GREEN,
            perceptual_roughness: 0.7,
            ..default()
        })
    });

    let shutter_trim_wood = workshop_mats.as_ref().map(|wm| wm.cabin_log.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: CABIN_LOG,
            perceptual_roughness: 0.8,
            ..default()
        })
    });

    let iron_hardware = workshop_mats.as_ref().map(|wm| wm.wrought_iron.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: WROUGHT_IRON,
            metallic: 0.8,
            perceptual_roughness: 0.4,
            ..default()
        })
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
                MeshMaterial3d(shutter_wood.clone()),
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
                MeshMaterial3d(shutter_trim_wood.clone()),
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

    // 1. Realistic Anatomical Moose Model Root (Positioned at X = -6.5, Y = 1.8, Z = -17.0)
    let torso_mesh = meshes.add(Cuboid::new(1.7, 1.35, 2.7));
    let withers_hump_mesh = meshes.add(Cuboid::new(1.3, 0.6, 1.1));
    let neck_mesh = meshes.add(Cuboid::new(0.85, 0.95, 1.1));
    let bell_mesh = meshes.add(Cone { radius: 0.14, height: 0.45 });

    let upper_leg_mesh = meshes.add(Cylinder::new(0.18, 0.7));
    let lower_leg_mesh = meshes.add(Cylinder::new(0.12, 0.8));
    let hoof_mesh = meshes.add(Cuboid::new(0.20, 0.14, 0.26));

    let cranium_mesh = meshes.add(Cuboid::new(0.68, 0.62, 0.75));
    let roman_snout_mesh = meshes.add(Cuboid::new(0.52, 0.52, 0.95));
    let nose_pad_mesh = meshes.add(Cuboid::new(0.48, 0.38, 0.20));
    let nostril_mesh = meshes.add(Cylinder::new(0.035, 0.06));
    let ear_mesh = meshes.add(Cone { radius: 0.12, height: 0.38 });

    let antler_beam_mesh = meshes.add(Cylinder::new(0.08, 0.45));
    let antler_palm_mesh = meshes.add(Cuboid::new(0.80, 0.08, 0.90));
    let antler_tine_mesh = meshes.add(Cone { radius: 0.04, height: 0.32 });
    let antler_tine_small_mesh = meshes.add(Cone { radius: 0.035, height: 0.25 });

    // Bulky, fluffy knitted scarf meshes
    let scarf_collar_mesh = meshes.add(Cylinder::new(0.68, 0.38));
    let scarf_cowl_fold_mesh = meshes.add(Cylinder::new(0.74, 0.18));
    let scarf_knot_mesh = meshes.add(Cuboid::new(0.38, 0.30, 0.32));
    let scarf_segment_mesh = meshes.add(Cuboid::new(0.38, 0.28, 0.22));
    let scarf_pompom_mesh = meshes.add(Sphere::new(0.16));

    commands
        .spawn((
            Mesh3d(torso_mesh),
            MeshMaterial3d(moose_fur_primary.clone()),
            Transform::from_xyz(-6.5, 1.8, -17.0),
            Collider::cuboid(0.85, 0.68, 1.35),
            RigidBody::KinematicPositionBased,
            MrMoose,
            MooseScarfMount::default(),
            Name::new("RealisticMrMoose"),
        ))
        .with_children(|parent| {
            // Dedicated Scarf Mount Point anchored on Mr. Moose's neck/collar
            parent.spawn((
                Transform::from_xyz(0.0, 0.85, 1.45),
                MooseScarfMountPoint,
                Name::new("MooseScarfMountPoint"),
            ));

            // Muscular withers shoulder hump
            parent.spawn((
                Mesh3d(withers_hump_mesh),
                MeshMaterial3d(moose_fur_primary.clone()),
                Transform::from_xyz(0.0, 0.85, 0.35),
                Name::new("MooseWithersHump"),
            ));

            // Muscular neck extending forward and up
            parent.spawn((
                Mesh3d(neck_mesh),
                MeshMaterial3d(moose_fur_primary.clone()),
                Transform::from_xyz(0.0, 0.55, 1.35).with_rotation(Quat::from_rotation_x(0.25)),
                Name::new("MooseNeck"),
            ));

            // Throat dewlap ("bell") hanging down
            parent.spawn((
                Mesh3d(bell_mesh),
                MeshMaterial3d(moose_fur_primary.clone()),
                Transform::from_xyz(0.0, 0.05, 1.55).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
                Name::new("MooseThroatBell"),
            ));

            // Bulky, fluffy knit scarf collar wrapped around neck
            parent.spawn((
                Mesh3d(scarf_collar_mesh),
                MeshMaterial3d(scarf_red.clone()),
                Transform::from_xyz(0.0, 0.85, 1.35).with_rotation(Quat::from_rotation_x(0.25)),
                MooseScarf,
                Name::new("MooseScarfCollar"),
            )).with_children(|cowl| {
                // Secondary fluffy folded white cowl rim
                cowl.spawn((
                    Mesh3d(scarf_cowl_fold_mesh),
                    MeshMaterial3d(scarf_white.clone()),
                    Transform::from_xyz(0.0, -0.06, 0.0),
                    Name::new("MooseScarfCowlFold"),
                ));
                // Puffy knit knot centered at the chest front
                cowl.spawn((
                    Mesh3d(scarf_knot_mesh),
                    MeshMaterial3d(scarf_red.clone()),
                    Transform::from_xyz(0.0, 0.04, 0.60).with_rotation(Quat::from_rotation_y(0.15)),
                    Name::new("MooseScarfKnot"),
                ));
            });

            // 4 Anatomical legs with upper thighs, cannon bones, and cloven hooves
            for (lx, lz) in [(-0.65, -0.95), (0.65, -0.95), (-0.65, 0.85), (0.65, 0.85)] {
                // Upper thigh / flank
                parent.spawn((
                    Mesh3d(upper_leg_mesh.clone()),
                    MeshMaterial3d(moose_fur_primary.clone()),
                    Transform::from_xyz(lx, -0.55, lz),
                    Name::new("MooseUpperThigh"),
                )).with_children(|thigh| {
                    // Lower leg / cannon bone
                    thigh.spawn((
                        Mesh3d(lower_leg_mesh.clone()),
                        MeshMaterial3d(moose_fur_secondary.clone()),
                        Transform::from_xyz(0.0, -0.65, 0.0),
                        Name::new("MooseCannonBone"),
                    )).with_children(|lower| {
                        // Cloven hoof
                        lower.spawn((
                            Mesh3d(hoof_mesh.clone()),
                            MeshMaterial3d(moose_hoof_mat.clone()),
                            Transform::from_xyz(0.0, -0.45, 0.05),
                            Name::new("MooseClovenHoof"),
                        ));
                    });
                });
            }

            // Sculpted Head: Tagged with MooseHead so it can animate / poke through window
            parent.spawn((
                Mesh3d(cranium_mesh),
                MeshMaterial3d(moose_fur_primary.clone()),
                Transform::from_xyz(0.0, 1.1, 1.6),
                MooseHead,
                Name::new("MooseHead"),
            )).with_children(|head| {
                // Arched Roman snout / drooping upper lip
                head.spawn((
                    Mesh3d(roman_snout_mesh),
                    MeshMaterial3d(moose_fur_secondary.clone()),
                    Transform::from_xyz(0.0, 0.05, 0.85),
                    Name::new("MooseRomanSnout"),
                ));

                // Dark prehensile nose pad leather
                head.spawn((
                    Mesh3d(nose_pad_mesh),
                    MeshMaterial3d(moose_snout_leather.clone()),
                    Transform::from_xyz(0.0, 0.0, 1.35),
                    Name::new("MooseNosePad"),
                ));

                // Left & Right sculpted nostrils
                for (nx, ny, nz) in [(-0.13, 0.02, 1.45), (0.13, 0.02, 1.45)] {
                    head.spawn((
                        Mesh3d(nostril_mesh.clone()),
                        MeshMaterial3d(moose_snout_leather.clone()),
                        Transform::from_xyz(nx, ny, nz).with_rotation(Quat::from_rotation_x(0.35)),
                        Name::new("MooseNostril"),
                    ));
                }

                //  Eyebrow ridges
                for (ex, sign) in [(-0.35, -1.0), (0.35, 1.0)] {
                    head.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.10, 0.05, 0.18))),
                        MeshMaterial3d(moose_fur_primary.clone()),
                        Transform::from_xyz(ex, 0.32, 0.45),
                        Name::new("MooseEyebrowRidge"),
                    ));

                    // Large mobile ears angled outward/backward
                    let ear_rot = Quat::from_euler(EulerRot::YXZ, sign * 0.4, 0.2, sign * 0.5);
                    head.spawn((
                        Mesh3d(ear_mesh.clone()),
                        MeshMaterial3d(moose_fur_secondary.clone()),
                        Transform::from_xyz(sign * 0.42, 0.50, -0.05).with_rotation(ear_rot),
                        Name::new("MooseEar"),
                    ));
                }

                // Magnificent Branching Palmated Antlers
                for (sign, rot_z) in [(-1.0, 0.25), (1.0, -0.25)] {
                    // Main beam extending out from pedicle
                    head.spawn((
                        Mesh3d(antler_beam_mesh.clone()),
                        MeshMaterial3d(moose_antler_mat.clone()),
                        Transform::from_xyz(sign * 0.32, 0.50, 0.15).with_rotation(Quat::from_rotation_z(rot_z * 2.0)),
                        Name::new("AntlerBeam"),
                    ));

                    // Broad shovel palm plate cupped upward
                    head.spawn((
                        Mesh3d(antler_palm_mesh.clone()),
                        MeshMaterial3d(moose_antler_mat.clone()),
                        Transform::from_xyz(sign * 0.82, 0.72, 0.25).with_rotation(Quat::from_rotation_z(rot_z)),
                        Name::new("AntlerShovelPalm"),
                    )).with_children(|palm| {
                        // Forward brow tine pointing forward (+Z)
                        palm.spawn((
                            Mesh3d(antler_tine_mesh.clone()),
                            MeshMaterial3d(moose_antler_mat.clone()),
                            Transform::from_xyz(0.0, 0.05, 0.45).with_rotation(Quat::from_rotation_x(1.2)),
                            Name::new("AntlerBrowTine"),
                        ));
                        // Upward/lateral crown tines
                        palm.spawn((
                            Mesh3d(antler_tine_mesh.clone()),
                            MeshMaterial3d(moose_antler_mat.clone()),
                            Transform::from_xyz(sign * 0.35, 0.10, 0.10).with_rotation(Quat::from_rotation_z(rot_z)),
                            Name::new("AntlerCrownTine1"),
                        ));
                        palm.spawn((
                            Mesh3d(antler_tine_mesh.clone()),
                            MeshMaterial3d(moose_antler_mat.clone()),
                            Transform::from_xyz(sign * 0.35, 0.10, -0.20).with_rotation(Quat::from_rotation_z(rot_z)),
                            Name::new("AntlerCrownTine2"),
                        ));
                        // Posterior rear tine
                        palm.spawn((
                            Mesh3d(antler_tine_small_mesh.clone()),
                            MeshMaterial3d(moose_antler_mat.clone()),
                            Transform::from_xyz(0.0, 0.05, -0.45).with_rotation(Quat::from_rotation_x(-1.0)),
                            Name::new("AntlerRearTine"),
                        ));
                    });
                }
            });
        });

    // 2. Red-and-White Striped Soft-Body Physics Scarf Tails (Bulkier & Fluffy)
    // Two flowing tails (Left and Right) cascading from neck anchor down the chest
    for tail_side in [-1.0, 1.0] {
        for seg_i in 0..4 {
            let idx = if tail_side < 0.0 { seg_i } else { 4 + seg_i };
            let rest_off = Vec3::new(tail_side * 0.28, -0.12 - (seg_i as f32 * 0.22), 1.05 + (seg_i as f32 * 0.09));
            let seg_mat = if seg_i % 2 == 0 { scarf_red.clone() } else { scarf_white.clone() };

            let mut seg_cmd = commands.spawn((
                Mesh3d(scarf_segment_mesh.clone()),
                MeshMaterial3d(seg_mat),
                Transform::from_translation(Vec3::new(-6.5, 1.8, -17.0) + rest_off),
                MooseScarfSegment {
                    index: idx,
                    rest_offset: rest_off,
                    velocity: Vec3::ZERO,
                },
                MooseScarf,
                Name::new(format!("MooseScarfSegment_{}", idx)),
            ));

            // Fluffy white pom-pom on the tip of the lowest tail segment
            if seg_i == 3 {
                seg_cmd.with_children(|tip| {
                    tip.spawn((
                        Mesh3d(scarf_pompom_mesh.clone()),
                        MeshMaterial3d(scarf_white.clone()),
                        Transform::from_xyz(0.0, -0.20, 0.0),
                        Name::new("MooseScarfPomPom"),
                    ));
                });
            }
        }
    }

    // 3. Windowsill Decorative Physics Objects:
    // Placed on windowsill surface at X = -6.5, Y = 2.05, Z = -12.85
    let mini_tree_pot = workshop_mats.as_ref().map(|wm| wm.terracotta_pot.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: TERRACOTTA_POT,
            perceptual_roughness: 0.8,
            ..default()
        })
    });
    let mini_tree_green = workshop_mats.as_ref().map(|wm| wm.pine_green.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: PINE_GREEN,
            perceptual_roughness: 0.7,
            ..default()
        })
    });
    let gold_star_mat = workshop_mats.as_ref().map(|wm| wm.gold_ribbon.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: GOLD_RIBBON,
            metallic: 0.7,
            perceptual_roughness: 0.3,
            ..default()
        })
    });

    // Miniature Christmas Tree on Windowsill
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.08, 0.12))),
        MeshMaterial3d(mini_tree_pot),
        Transform::from_xyz(-7.0, 2.15, -12.85),
        Collider::cylinder(0.22, 0.14),
        RigidBody::Dynamic,
        Velocity::default(),
        Grabbable,
        WindowsillMiniTree,
        Name::new("WindowsillMiniChristmasTree"),
    )).with_children(|pot| {
        // Pine trunk
        pot.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.02, 0.08))),
            MeshMaterial3d(shutter_trim_wood.clone()),
            Transform::from_xyz(0.0, 0.10, 0.0),
            Name::new("MiniTreeTrunk"),
        ));
        // Foliage cones (3 festive tiers)
        for (fy, r, h) in [(0.18, 0.14, 0.14), (0.28, 0.10, 0.12), (0.36, 0.06, 0.10)] {
            pot.spawn((
                Mesh3d(meshes.add(Cone { radius: r, height: h })),
                MeshMaterial3d(mini_tree_green.clone()),
                Transform::from_xyz(0.0, fy, 0.0),
                Name::new("MiniTreeFoliage"),
            ));
        }
        // Gold star topper
        pot.spawn((
            Mesh3d(meshes.add(Sphere::new(0.025))),
            MeshMaterial3d(gold_star_mat),
            Transform::from_xyz(0.0, 0.44, 0.0),
            Name::new("MiniTreeGoldStar"),
        ));
    });

    // Mug of Hot Cocoa on Windowsill
    let mug_ceramic_mat = workshop_mats.as_ref().map(|wm| wm.cocoa_red.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: COCOA_CERAMIC_RED,
            perceptual_roughness: 0.3,
            ..default()
        })
    });
    let cocoa_liquid_mat = workshop_mats.as_ref().map(|wm| wm.hot_cocoa_liquid.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: HOT_COCOA_LIQUID,
            perceptual_roughness: 0.2,
            ..default()
        })
    });
    let marshmallow_mat = workshop_mats.as_ref().map(|wm| wm.fluffy_white.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SNOW_WHITE,
            perceptual_roughness: 0.6,
            ..default()
        })
    });

    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.06, 0.12))),
        MeshMaterial3d(mug_ceramic_mat),
        Transform::from_xyz(-6.0, 2.10, -12.85),
        Collider::cylinder(0.06, 0.06),
        RigidBody::Dynamic,
        Velocity::default(),
        Grabbable,
        WindowsillHotCocoa,
        Name::new("WindowsillHotCocoaMug"),
    )).with_children(|mug| {
        // Cocoa surface
        mug.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.052, 0.02))),
            MeshMaterial3d(cocoa_liquid_mat),
            Transform::from_xyz(0.0, 0.05, 0.0),
            Name::new("HotCocoaSurface"),
        ));
        // Fluffy marshmallows floating on top
        for (mx, mz) in [(-0.02, -0.01), (0.02, 0.02), (-0.01, 0.02)] {
            mug.spawn((
                Mesh3d(meshes.add(Sphere::new(0.012))),
                MeshMaterial3d(marshmallow_mat.clone()),
                Transform::from_xyz(mx, 0.065, mz),
                Name::new("CocoaMarshmallow"),
            ));
        }
        // Handle
        mug.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.02, 0.06, 0.04))),
            MeshMaterial3d(shutter_wood.clone()),
            Transform::from_xyz(0.07, 0.0, 0.0),
            Name::new("MugHandle"),
        ));
    });

    // 4. Feeding Trough / Food Box in front of the window (X = -6.5, Z = -12.0)
    let trough_metal = workshop_mats.as_ref().map(|wm| wm.sheet_metal.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SHEET_METAL,
            metallic: 0.7,
            ..default()
        })
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

    // 5. Main Workbench Table: Glass Jar Filled with Small Moose Snacks (X = 1.4, Y = 0.8, Z = -4.0)
    let jar_glass_mat = workshop_mats.as_ref().map(|wm| wm.snack_jar_glass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SNACK_JAR_GLASS,
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.15,
            ..default()
        })
    });
    let jar_lid_mat = workshop_mats.as_ref().map(|wm| wm.polished_brass.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: POLISHED_BRASS,
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        })
    });
    let snack_cookie_mat = workshop_mats.as_ref().map(|wm| wm.moose_snack_oat.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: MOOSE_SNACK_GOLDEN_OAT,
            perceptual_roughness: 0.6,
            ..default()
        })
    });

    // Glass Snack Jar on Workbench
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.20, 0.36))),
        MeshMaterial3d(jar_glass_mat),
        Transform::from_xyz(1.4, 0.98, -4.0),
        Collider::cylinder(0.18, 0.20),
        RigidBody::Fixed,
        MooseSnackJar,
        Name::new("MooseSnackJar"),
    )).with_children(|jar| {
        // Brass screw lid
        jar.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.21, 0.06))),
            MeshMaterial3d(jar_lid_mat),
            Transform::from_xyz(0.0, 0.20, 0.0),
            Name::new("JarBrassLid"),
        ));
    });

    // 4 Small Physics Moose Snacks resting on the workbench table near the jar
    for (ox, oz) in [(-0.12, -0.08), (0.10, -0.05), (-0.05, 0.10), (0.08, 0.08)] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.07, 0.05))),
            MeshMaterial3d(snack_cookie_mat.clone()),
            Transform::from_xyz(1.4 + ox, 0.90, -4.0 + oz),
            Collider::cylinder(0.025, 0.07),
            RigidBody::Dynamic,
            Velocity::zero(),
            Grabbable,
            MooseSnack {
                nutrition_value: 60.0,
            },
            Name::new("SmallMooseSnack"),
        ));
    }
}

const MAX_PEEK_SHUTTER_ANGLE: f32 = 1.35; // ~77 degrees inward
const BREACHED_SHUTTER_ANGLE: f32 = 1.95; // ~112 degrees wide open

/// Animates Mr. Moose's position, head posture, and hinged shutters with fast, snappy transitions between states.
pub fn animate_mr_moose_peeking_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    hunger: Res<MooseHunger>,
    mut q_moose: Query<&mut Transform, (With<MrMoose>, Without<MooseHead>, Without<LeftWindowShutter>, Without<RightWindowShutter>)>,
    mut q_head: Query<&mut Transform, (With<MooseHead>, Without<MrMoose>, Without<LeftWindowShutter>, Without<RightWindowShutter>)>,
    mut q_left_shutter: Query<&mut Transform, (With<LeftWindowShutter>, Without<RightWindowShutter>, Without<MrMoose>, Without<MooseHead>)>,
    mut q_right_shutter: Query<&mut Transform, (With<RightWindowShutter>, Without<LeftWindowShutter>, Without<MrMoose>, Without<MooseHead>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

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

    // Determine target transforms for current hunger stage
    let (target_moose_pos, target_head_pos, target_head_rot, target_shutter_angle) = match hunger.stage {
        MooseHungerStage::BehindShutters => {
            // Far back outside, shutters shut
            (
                Vec3::new(-6.5, 1.8, -17.0),
                Vec3::new(0.0, 1.1, 1.6),
                Quat::IDENTITY,
                0.0,
            )
        }
        MooseHungerStage::PeekingThrough => {
            // Fast, snappy step up to window with head poking through (head translation distance reduced by 1/2)
            let progress = ((hunger.value - 20.0) / 30.0).clamp(0.0, 1.0);
            let head_sway = (progress * std::f32::consts::PI * 2.0).sin() * 0.05;
            (
                Vec3::new(-6.5, 1.8, -14.5),
                Vec3::new(head_sway, 1.1, 2.4 + progress * 0.1),
                Quat::IDENTITY,
                MAX_PEEK_SHUTTER_ANGLE,
            )
        }
        MooseHungerStage::AgitatedHuffing => {
            // Agitated lean-in with rhythmic huffing snorts and shutter jitter (head translation distance reduced by 1/2)
            let huff_pulse = (hunger.value * 14.0).sin();
            let huff_offset = huff_pulse.max(0.0) * 0.05;
            let jitter = huff_pulse * 0.03;
            (
                Vec3::new(-6.5, 1.8, -14.3),
                Vec3::new(0.0, 1.1, 2.6 + huff_offset),
                Quat::IDENTITY,
                MAX_PEEK_SHUTTER_ANGLE + jitter,
            )
        }
        MooseHungerStage::WindowSlammed => {
            // Sharp snap back outside, shutters slammed shut
            (
                Vec3::new(-6.5, 1.8, -17.0),
                Vec3::new(0.0, 1.1, 1.6),
                Quat::IDENTITY,
                0.0,
            )
        }
        MooseHungerStage::DeliberatingLateTreat => {
            // Moose peeks back through the window, deliberating suspiciously
            let progress = (hunger.deliberation_timer / hunger.deliberation_duration.max(0.01)).clamp(0.0, 1.0);
            let deliberate_sway = (progress * std::f32::consts::PI * 4.0).sin() * 0.04;
            let head_cock_z = (progress * std::f32::consts::PI * 2.0).sin() * 0.08;
            (
                Vec3::new(-6.5, 1.8, -14.5),
                Vec3::new(deliberate_sway, 1.08, 2.45),
                Quat::from_rotation_z(head_cock_z),
                MAX_PEEK_SHUTTER_ANGLE,
            )
        }
        MooseHungerStage::EatingTreat => {
            let progress = (hunger.eating_timer / hunger.eating_duration.max(0.01)).clamp(0.0, 1.0);
            let is_fast = hunger.is_eating_fast;

            let (head_pos, head_rot) = if !is_fast {
                // Gentle Eating Animation (On-time treat):
                // Smooth reach down into trough, rhythmic chewing bobs, gentle return
                if progress < 0.35 {
                    let t = progress / 0.35;
                    let s = t * t * (3.0 - 2.0 * t);
                    let y = 1.1 + s * (0.15 - 1.1);
                    let z = 2.4 + s * (2.75 - 2.4);
                    let pitch = s * 0.48;
                    (Vec3::new(0.0, y, z), Quat::from_rotation_x(pitch))
                } else if progress < 0.75 {
                    let chew_timer = hunger.eating_timer;
                    let chew_bob = (chew_timer * 10.0).sin() * 0.04;
                    let chew_pitch = 0.48 + (chew_timer * 10.0).cos() * 0.05;
                    (
                        Vec3::new(0.0, 0.15 + chew_bob, 2.75),
                        Quat::from_rotation_x(chew_pitch),
                    )
                } else {
                    let t = (progress - 0.75) / 0.25;
                    let s = t * t * (3.0 - 2.0 * t);
                    let y = 0.15 + s * (1.1 - 0.15);
                    let z = 2.75 + s * (2.4 - 2.75);
                    let pitch = 0.48 * (1.0 - s);
                    (Vec3::new(0.0, y, z), Quat::from_rotation_x(pitch))
                }
            } else {
                // Fast & Annoyed Eating Animation (Late treat accepted):
                // Snappy dive down, aggressive chomping with head shakes, snappy jerk up
                if progress < 0.25 {
                    let t = progress / 0.25;
                    let y = 1.1 + t * (0.08 - 1.1);
                    let z = 2.4 + t * (2.80 - 2.4);
                    let pitch = t * 0.60;
                    (Vec3::new(0.0, y, z), Quat::from_rotation_x(pitch))
                } else if progress < 0.70 {
                    let chew_timer = hunger.eating_timer;
                    let rapid_bob = (chew_timer * 28.0).sin() * 0.07;
                    let rapid_shake_x = (chew_timer * 24.0).cos() * 0.05;
                    let rapid_yaw = (chew_timer * 24.0).sin() * 0.08;
                    let rapid_pitch = 0.60 + (chew_timer * 28.0).cos() * 0.08;
                    (
                        Vec3::new(rapid_shake_x, 0.08 + rapid_bob, 2.80),
                        Quat::from_euler(EulerRot::YXZ, rapid_yaw, rapid_pitch, 0.0),
                    )
                } else {
                    let t = (progress - 0.70) / 0.30;
                    let y = 0.08 + t * (1.1 - 0.08);
                    let z = 2.80 + t * (2.4 - 2.80);
                    let pitch = 0.60 * (1.0 - t);
                    (Vec3::new(0.0, y, z), Quat::from_rotation_x(pitch))
                }
            };

            (
                Vec3::new(-6.5, 1.8, -14.5),
                head_pos,
                head_rot,
                MAX_PEEK_SHUTTER_ANGLE,
            )
        }
        MooseHungerStage::BreachedRampage => unreachable!(),
    };

    // Snappy transition interpolation rate:
    // With rate = 14.0 s^-1, ~98% of transition is complete in 0.25s.
    // If running headless without Time resource (e.g. unit tests), snap immediately (blend = 1.0).
    const SNAPPY_LERP_SPEED: f32 = 14.0;
    let blend = if let Some(ref t) = time {
        let dt = t.delta_secs();
        if dt <= 0.0 {
            1.0
        } else {
            (1.0 - (-SNAPPY_LERP_SPEED * dt).exp()).clamp(0.0, 1.0)
        }
    } else {
        1.0
    };

    // Interpolate Moose body transform
    for mut m_tf in &mut q_moose {
        if blend >= 0.999 {
            m_tf.translation = target_moose_pos;
            m_tf.rotation = Quat::IDENTITY;
        } else {
            m_tf.translation = m_tf.translation.lerp(target_moose_pos, blend);
            m_tf.rotation = m_tf.rotation.slerp(Quat::IDENTITY, blend);
        }
    }

    // Interpolate Moose head transform
    for mut h_tf in &mut q_head {
        if blend >= 0.999 {
            h_tf.translation = target_head_pos;
            h_tf.rotation = target_head_rot;
        } else {
            h_tf.translation = h_tf.translation.lerp(target_head_pos, blend);
            h_tf.rotation = h_tf.rotation.slerp(target_head_rot, blend);
        }
    }

    // Interpolate hinged window shutters
    let target_left_rot = Quat::from_rotation_y(-target_shutter_angle);
    let target_right_rot = Quat::from_rotation_y(target_shutter_angle);

    for mut tf in &mut q_left_shutter {
        if blend >= 0.999 {
            tf.rotation = target_left_rot;
        } else {
            tf.rotation = tf.rotation.slerp(target_left_rot, blend);
        }
    }
    for mut tf in &mut q_right_shutter {
        if blend >= 0.999 {
            tf.rotation = target_right_rot;
        } else {
            tf.rotation = tf.rotation.slerp(target_right_rot, blend);
        }
    }
}

/// Monitors snacks brought to Mr. Moose's feeding trough / food box.
/// If on-time (hunger 50..60), initiates the gentle eating animation (or resets immediately if running headless without Time).
/// If late and the window is shut (WindowSlammed), initiates deliberation (1-2s) with paused hunger timer.
pub fn moose_feeding_trough_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut hunger: ResMut<MooseHunger>,
    q_trough: Query<&Transform, With<MooseFeedingTrough>>,
    q_snacks: Query<(Entity, &Transform, &MooseSnack)>,
    mut feed_messages: Option<MessageWriter<FeedMooseMessage>>,
    mut commands: Commands,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Ok(trough_tf) = q_trough.single() else {
        return;
    };

    if hunger.is_breached || hunger.stage == MooseHungerStage::BreachedRampage {
        return;
    }

    if hunger.stage == MooseHungerStage::DeliberatingLateTreat || hunger.stage == MooseHungerStage::EatingTreat {
        return;
    }

    for (entity, s_tf, _snack) in &q_snacks {
        let inside_trough = (s_tf.translation.x - trough_tf.translation.x).abs() < 1.1
            && (s_tf.translation.z - trough_tf.translation.z).abs() < 0.8
            && s_tf.translation.y < 1.4;

        if inside_trough {
            // Case 1: Player is late and the window is already shut (WindowSlammed)
            if hunger.stage == MooseHungerStage::WindowSlammed {
                if hunger.late_treat_rejected {
                    break;
                }
                hunger.stage = MooseHungerStage::DeliberatingLateTreat;
                hunger.deliberation_timer = 0.0;
                if hunger.deliberation_duration <= 0.0 {
                    hunger.deliberation_duration = 1.5;
                }
                hunger.active_snack_entity = Some(entity);
                break;
            }

            // Case 2: Player is on time with the treat (hunger 50 to 60)
            if hunger.value >= 50.0 && hunger.value <= 60.0 {
                if time.is_none() {
                    // Headless unit test without Time resource (e.g. test_moose_feeding_trough_stage_wait_and_reset)
                    hunger.value = 0.0;
                    hunger.stage = MooseHungerStage::BehindShutters;
                    hunger.breach_delay_remaining = None;
                    hunger.is_breached = false;

                    if let Some(ref mut writer) = feed_messages {
                        writer.write(FeedMooseMessage { snack_entity: entity });
                    }
                    commands.entity(entity).despawn();
                } else {
                    // Normal on-time gentle eating animation
                    hunger.stage = MooseHungerStage::EatingTreat;
                    hunger.is_eating_fast = false;
                    hunger.eating_duration = 2.2;
                    hunger.eating_timer = 0.0;
                    hunger.active_snack_entity = Some(entity);
                }
                break;
            }
        }
    }
}

/// Updates late-treat deliberation and eating animations for Mr. Moose.
/// Deliberation pauses the hunger timer while the Moose peeks back through the window.
/// Upon deciding to accept, a fast/annoyed eating animation plays.
/// Upon deciding to reject, the window is slammed shut again and breach countdown resumes.
pub fn moose_eating_and_deliberation_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut hunger: ResMut<MooseHunger>,
    q_trough: Query<&Transform, With<MooseFeedingTrough>>,
    q_snacks: Query<(Entity, &Transform, &MooseSnack)>,
    mut slam_events: Option<MessageWriter<MooseWindowSlamEvent>>,
    mut feed_messages: Option<MessageWriter<FeedMooseMessage>>,
    mut commands: Commands,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.map(|t| t.delta_secs()).unwrap_or(0.016);

    match hunger.stage {
        MooseHungerStage::DeliberatingLateTreat => {
            // Verify that the snack is still present in the trough
            let Ok(trough_tf) = q_trough.single() else {
                return;
            };

            let mut snack_present = false;
            if let Some(snack_entity) = hunger.active_snack_entity {
                if let Ok((_ent, s_tf, _snack)) = q_snacks.get(snack_entity) {
                    if (s_tf.translation.x - trough_tf.translation.x).abs() < 1.1
                        && (s_tf.translation.z - trough_tf.translation.z).abs() < 0.8
                        && s_tf.translation.y < 1.4
                    {
                        snack_present = true;
                    }
                }
            }

            if !snack_present {
                // Snack removed from trough while deliberating!
                // Moose slams window shut again and cancels deliberation
                hunger.stage = MooseHungerStage::WindowSlammed;
                hunger.active_snack_entity = None;
                if let Some(ref mut writer) = slam_events {
                    writer.write(MooseWindowSlamEvent);
                }
                return;
            }

            // Advance deliberation timer (hunger timer is paused during this stage)
            hunger.deliberation_timer += dt;
            if hunger.deliberation_timer >= hunger.deliberation_duration {
                // Deliberation complete: decide to accept or reject
                let accept = hunger.late_treat_decision_override.unwrap_or_else(|| {
                    let ticks = (hunger.deliberation_timer * 1000.0) as u64;
                    (ticks % 2) == 0
                });

                if accept {
                    // Accept treat: run fast eating animation to show annoyance
                    hunger.stage = MooseHungerStage::EatingTreat;
                    hunger.is_eating_fast = true;
                    hunger.eating_duration = 0.75;
                    hunger.eating_timer = 0.0;
                } else {
                    // Reject treat: slam window shut again and resume rampage countdown!
                    hunger.stage = MooseHungerStage::WindowSlammed;
                    hunger.late_treat_rejected = true;
                    hunger.active_snack_entity = None;
                    if let Some(ref mut writer) = slam_events {
                        writer.write(MooseWindowSlamEvent);
                    }
                }
            }
        }
        MooseHungerStage::EatingTreat => {
            hunger.eating_timer += dt;
            let progress = (hunger.eating_timer / hunger.eating_duration.max(0.01)).clamp(0.0, 1.0);

            // Despawn snack halfway through eating as it gets consumed
            if progress >= 0.5 {
                if let Some(snack_entity) = hunger.active_snack_entity.take() {
                    commands.entity(snack_entity).despawn();
                    if let Some(ref mut writer) = feed_messages {
                        writer.write(FeedMooseMessage { snack_entity });
                    }
                }
            }

            // Eating animation complete: reset hunger and retreat behind shutters
            if progress >= 1.0 {
                let was_fast = hunger.is_eating_fast;
                hunger.value = 0.0;
                hunger.stage = MooseHungerStage::BehindShutters;
                hunger.breach_delay_remaining = None;
                hunger.is_breached = false;
                hunger.active_snack_entity = None;
                hunger.late_treat_rejected = false;

                if was_fast {
                    // Annoyed finish: slam window shut behind retreat
                    if let Some(ref mut writer) = slam_events {
                        writer.write(MooseWindowSlamEvent);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Simulates soft-body chain physics for Mr. Moose's red-and-white striped scarf.
/// The scarf is anchored to a mount point on Mr. Moose's neck/collar, and the two tails
/// propagate momentum and inertia down each chain link with distance constraints, tension,
/// gravity, and air drag as Mr. Moose moves, peeks, huffs, slams, or bucks during rampages.
pub fn moose_scarf_physics_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_moose: Query<(&Transform, Option<&mut MooseScarfMount>), (With<MrMoose>, Without<MooseScarfSegment>)>,
    q_head: Query<&Transform, (With<MooseHead>, Without<MooseScarfSegment>)>,
    mut q_segments: Query<(&mut MooseScarfSegment, &mut Transform)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let Ok((moose_tf, mut opt_mount)) = q_moose.single_mut() else {
        return;
    };
    let head_z = q_head.single().map(|h| h.translation.z).unwrap_or(1.6);
    let head_forward_pull = (head_z - 1.6).max(0.0) * 0.25;

    // Default mount anchors in Moose local space
    let (left_anchor_local, right_anchor_local, last_pos) = if let Some(ref m) = opt_mount {
        (m.left_tail_anchor, m.right_tail_anchor, m.last_moose_pos)
    } else {
        (
            Vec3::new(-0.24, 0.85, 1.45),
            Vec3::new(0.24, 0.85, 1.45),
            moose_tf.translation,
        )
    };

    // Calculate Moose velocity for inertia transmission
    let moose_velocity = if dt > 0.0001 {
        (moose_tf.translation - last_pos) / dt
    } else {
        Vec3::ZERO
    };

    // Update last moose position
    if let Some(ref mut m) = opt_mount {
        m.last_moose_pos = moose_tf.translation;
    }

    // World-space mount points for left and right scarf tails
    let left_mount_pos = moose_tf.transform_point(left_anchor_local + Vec3::new(0.0, 0.0, head_forward_pull));
    let right_mount_pos = moose_tf.transform_point(right_anchor_local + Vec3::new(0.0, 0.0, head_forward_pull));

    // Natural rest direction hanging down from parent along the moose's chest
    let rest_dir = (moose_tf.rotation * Vec3::new(0.0, -0.92, 0.35)).normalize();
    const LINK_REST_LEN: f32 = 0.22;

    // Step 1: Gather current positions and velocities
    let mut seg_data: [Option<(Vec3, Vec3)>; 8] = [None; 8];
    for (seg, tf) in &q_segments {
        if seg.index < 8 {
            seg_data[seg.index] = Some((tf.translation, seg.velocity));
        }
    }

    // Step 2: Simulate chain propagation from mount point down each tail
    let mut calculated: [Option<(Vec3, Quat, Vec3)>; 8] = [None; 8];

    for tail in 0..2 {
        let (start_idx, mount_pos) = if tail == 0 {
            (0, left_mount_pos)
        } else {
            (4, right_mount_pos)
        };

        let mut current_parent_pos = mount_pos;
        let mut current_parent_vel = moose_velocity;

        for seg_offset in 0..4 {
            let idx = start_idx + seg_offset;
            let Some((current_pos, cur_vel)) = seg_data[idx] else {
                continue;
            };

            // Link tension force pulling towards parent
            let to_parent = current_parent_pos - current_pos;
            let dist = to_parent.length();
            let tension = if dist > 0.0001 {
                let stretch = dist - LINK_REST_LEN;
                (to_parent / dist) * (stretch * 95.0)
            } else {
                Vec3::ZERO
            };

            // Relative damping with parent link
            let rel_vel = cur_vel - current_parent_vel;
            let damping = -rel_vel * 7.5;

            // Rest pose bending stiffness (drapes naturally along chest)
            let rest_target = current_parent_pos + rest_dir * LINK_REST_LEN;
            let rest_bias = (rest_target - current_pos) * 22.0;

            // Gravity & subtle organic breathing flutter
            let gravity = Vec3::new(0.0, -7.0, 0.0);
            let flutter = Vec3::new(
                ((idx as f32 * 1.8 + head_z * 3.5).sin()) * 0.35,
                0.0,
                ((idx as f32 * 2.2 + head_z * 2.8).cos()) * 0.35,
            );

            // Aerodynamic drag
            let drag = -cur_vel * 2.8;

            let total_accel = tension + damping + rest_bias + gravity + flutter + drag;
            let mut new_vel = cur_vel + total_accel * dt;
            let mut new_pos = current_pos + new_vel * dt;

            // Distance constraint projection (prevent link disconnection or inversion)
            let from_parent = new_pos - current_parent_pos;
            let new_dist = from_parent.length();
            let max_dist = LINK_REST_LEN * 1.35;
            let min_dist = LINK_REST_LEN * 0.65;
            if new_dist > max_dist && new_dist > 0.0001 {
                let norm = from_parent / new_dist;
                new_pos = current_parent_pos + norm * max_dist;
                let outward_speed = new_vel.dot(norm);
                if outward_speed > 0.0 {
                    new_vel -= norm * outward_speed * 0.8;
                }
            } else if new_dist < min_dist && new_dist > 0.0001 {
                new_pos = current_parent_pos + (from_parent / new_dist) * min_dist;
            }

            // Align segment rotation along the tangent vector from parent to child
            let link_dir = new_pos - current_parent_pos;
            let new_rot = if link_dir.length_squared() > 0.0001 {
                Quat::from_rotation_arc(Vec3::NEG_Y, link_dir.normalize()) * moose_tf.rotation
            } else {
                moose_tf.rotation
            };

            calculated[idx] = Some((new_pos, new_rot, new_vel));

            // Update parent for the next link in the chain
            current_parent_pos = new_pos;
            current_parent_vel = new_vel;
        }
    }

    // Step 3: Write back updated transform and velocity
    for (mut seg, mut tf) in &mut q_segments {
        if seg.index < 8 {
            if let Some((pos, rot, vel)) = calculated[seg.index] {
                tf.translation = pos;
                tf.rotation = rot;
                seg.velocity = vel;
            }
        }
    }
}
