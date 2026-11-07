use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    player::hands::Grabbable,
    wrapping::{Present, WrappingSpecification},
};

#[derive(Component)]
pub struct WorkshopWorkbench;

#[derive(Component)]
pub struct ConveyorBelt {
    pub speed: f32,
    pub direction: Vec3,
}

#[derive(Component)]
pub struct PresentDeliveryChute;

#[derive(Component)]
pub struct ConveyorDespawnChute;

#[derive(Component)]
pub struct BreakableWindowPane {
    pub is_broken: bool,
}

#[derive(Resource)]
pub struct ConveyorSpawnerTimer {
    pub timer: Timer,
}

impl Default for ConveyorSpawnerTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.5, TimerMode::Repeating),
        }
    }
}

/// Spawns the Santa's Workshop room: walls, floor, tables, conveyors, delivery chute, windows, and outdoor snowy scene
pub fn setup_workshop_scene(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    // 1. Room Floor: Warm wooden workshop floor (26m x 26m, 60% of previous size)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(26.0, 26.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.35, 0.22),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider::cuboid(13.0, 0.1, 13.0),
        RigidBody::Fixed,
        Name::new("WorkshopFloor"),
    ));

    // Workshop Ceiling with wooden rafters (Height = 5.5m)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(26.0, 26.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.22, 0.14),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 5.5, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
        Collider::cuboid(13.0, 0.1, 13.0),
        RigidBody::Fixed,
        Name::new("WorkshopCeiling"),
    ));

    // 2. Workshop Walls (Festive Santa Red with Wood Baseboards)
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.20, 0.20),
        perceptual_roughness: 0.9,
        ..default()
    });

    let window_glass_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.92, 1.0, 0.35),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        metallic: 0.1,
        ..default()
    });

    let window_frame_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.2, 0.12),
        perceptual_roughness: 0.7,
        ..default()
    });

    // Back wall (South: Z = 13.0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(26.0, 5.5, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, 2.75, 13.0),
        Collider::cuboid(13.0, 2.75, 0.2),
        RigidBody::Fixed,
        Name::new("WallSouth"),
    ));

    // Left wall (West: X = -13.0) - with Side Snowy Observation Windows
    // Solid bottom wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 1.2, 26.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-13.0, 0.6, 0.0),
        Collider::cuboid(0.2, 0.6, 13.0),
        RigidBody::Fixed,
        Name::new("WallWestLower"),
    ));
    // West Windows
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 2.8, 18.0))),
        MeshMaterial3d(window_glass_material.clone()),
        Transform::from_xyz(-13.0, 2.6, 0.0),
        Collider::cuboid(0.1, 1.4, 9.0),
        RigidBody::Fixed,
        Name::new("WestWindowGlass"),
    ));
    // Solid top wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 1.5, 26.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-13.0, 4.75, 0.0),
        Collider::cuboid(0.2, 0.75, 13.0),
        RigidBody::Fixed,
        Name::new("WallWestUpper"),
    ));

    // Right wall (East: X = 13.0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 5.5, 26.0))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(13.0, 2.75, 0.0),
        Collider::cuboid(0.2, 2.75, 13.0),
        RigidBody::Fixed,
        Name::new("WallEast"),
    ));

    // North wall (Front: Z = -13.0):
    // 1. Left North Wall section (from X = -13.0 to X = -7.4, width 5.6m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(5.6, 5.5, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-10.2, 2.75, -13.0),
        Collider::cuboid(2.8, 2.75, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthLeft"),
    ));

    // 2. Mr. Moose Head-Sized Window Opening (X = -7.4 to X = -5.6, centered at X = -6.5, width 1.8m, height 1.8m)
    // Wall below window opening (Y = 0.0 to 1.95)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.8, 1.95, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-6.5, 0.975, -13.0),
        Collider::cuboid(0.9, 0.975, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthBelowWindow"),
    ));

    // Window Sill Shelf (Wooden ledge protruding slightly into room for decorations/props)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.1, 0.12, 0.6))),
        MeshMaterial3d(window_frame_material.clone()),
        Transform::from_xyz(-6.5, 1.95, -12.95),
        Collider::cuboid(1.05, 0.06, 0.3),
        RigidBody::Fixed,
        Name::new("MooseWindowSill"),
    ));

    // Window Casing / Frame Trim
    // Header trim (above opening at Y = 3.86)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.1, 0.12, 0.45))),
        MeshMaterial3d(window_frame_material.clone()),
        Transform::from_xyz(-6.5, 3.86, -13.0),
        Collider::cuboid(1.05, 0.06, 0.225),
        RigidBody::Fixed,
        Name::new("MooseWindowHeaderTrim"),
    ));

    // Left jamb trim
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.15, 1.8, 0.45))),
        MeshMaterial3d(window_frame_material.clone()),
        Transform::from_xyz(-7.45, 2.9, -13.0),
        Collider::cuboid(0.075, 0.9, 0.225),
        RigidBody::Fixed,
        Name::new("MooseWindowLeftJamb"),
    ));

    // Right jamb trim
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.15, 1.8, 0.45))),
        MeshMaterial3d(window_frame_material.clone()),
        Transform::from_xyz(-5.55, 2.9, -13.0),
        Collider::cuboid(0.075, 0.9, 0.225),
        RigidBody::Fixed,
        Name::new("MooseWindowRightJamb"),
    ));

    // Wall North above window opening (Y = 3.92 to 5.5, height 1.58m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.8, 1.58, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(-6.5, 4.71, -13.0),
        Collider::cuboid(0.9, 0.79, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthAboveWindow"),
    ));

    // Breakable window pane state entity for event integration
    commands.spawn((
        BreakableWindowPane { is_broken: false },
        Transform::from_xyz(-6.5, 2.9, -13.0),
        Visibility::default(),
        Name::new("MooseWindowBreachState"),
    ));

    // 3. Right & Center North Wall (Behind the workbench & monitor: X = -5.6 to X = 13.0, width 18.6m)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(18.6, 5.5, 0.4))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(3.7, 2.75, -13.0),
        Collider::cuboid(9.3, 2.75, 0.2),
        RigidBody::Fixed,
        Name::new("WallNorthMainSolid"),
    ));

    // 3. Outdoor Snowy Scene & Pine Trees (Outside the window around X = -6.5 and North)
    let snow_ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.96, 1.0),
        perceptual_roughness: 0.95,
        ..default()
    });

    let pine_foliage = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.35, 0.18), // Snowy dark evergreen
        perceptual_roughness: 0.8,
        ..default()
    });

    let pine_trunk = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.18, 0.1),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Outdoor snow landscape
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(snow_ground.clone()),
        Transform::from_xyz(0.0, -0.6, -35.0),
        Collider::cuboid(30.0, 0.2, 30.0),
        RigidBody::Fixed,
        Name::new("NorthSnowLandscape"),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 60.0))),
        MeshMaterial3d(snow_ground),
        Transform::from_xyz(-30.0, -0.6, 0.0),
        Collider::cuboid(20.0, 0.2, 30.0),
        RigidBody::Fixed,
        Name::new("WestSnowLandscape"),
    ));

    // Scenic Snowy Pine Trees outside
    let tree_coords = [
        (-10.0, -22.0), (-3.0, -25.0), (-14.0, -28.0), (6.0, -22.0),
        (-20.0, -14.0), (-24.0, -4.0), (-22.0, 8.0), (-6.0, -32.0),
    ];

    let trunk_mesh = meshes.add(Cylinder::new(0.3, 2.5));
    let foliage_lower = meshes.add(Cone { radius: 2.0, height: 3.0 });
    let foliage_upper = meshes.add(Cone { radius: 1.4, height: 2.2 });

    for (tx, tz) in tree_coords {
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(pine_trunk.clone()),
            Transform::from_xyz(tx, 0.7, tz),
            Collider::cylinder(1.25, 0.3),
            RigidBody::Fixed,
            Name::new("SnowyPineTrunk"),
        )).with_children(|trunk| {
            trunk.spawn((
                Mesh3d(foliage_lower.clone()),
                MeshMaterial3d(pine_foliage.clone()),
                Transform::from_xyz(0.0, 2.0, 0.0),
                Name::new("FoliageLower"),
            ));
            trunk.spawn((
                Mesh3d(foliage_upper.clone()),
                MeshMaterial3d(pine_foliage.clone()),
                Transform::from_xyz(0.0, 3.5, 0.0),
                Name::new("FoliageUpper"),
            ));
        });
    }

    // 4. Conveyor Belt: Spans 18m from X = -9.0 to X = 9.0 along Z = 3.0
    let conveyor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.25),
        perceptual_roughness: 0.5,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(18.0, 0.6, 1.2))),
        MeshMaterial3d(conveyor_material),
        Transform::from_xyz(0.0, 0.3, 3.0),
        Collider::cuboid(9.0, 0.3, 0.6),
        RigidBody::Fixed,
        ConveyorBelt {
            speed: 2.0,
            direction: Vec3::new(1.0, 0.0, 0.0),
        },
        Name::new("MainConveyorBelt"),
    ));

    // Despawning Chute at the end (+X) of the conveyor belt (X = 9.5, Z = 3.0)
    let despawn_chute_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.20),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.4, 0.9, 1.4))),
        MeshMaterial3d(despawn_chute_mat),
        Transform::from_xyz(9.6, 0.45, 3.0),
        Collider::cuboid(0.7, 0.45, 0.7),
        RigidBody::Fixed,
        ConveyorDespawnChute,
        Name::new("ConveyorDespawnChute"),
    ));

    // 5. Central Wrapping Workbench Table (Width = 4.0m, Depth = 1.8m, Z = -4.0)
    let table_wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.25, 0.15),
        perceptual_roughness: 0.6,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 0.8, 1.8))),
        MeshMaterial3d(table_wood),
        Transform::from_xyz(0.0, 0.4, -4.0),
        Collider::cuboid(2.0, 0.4, 0.9),
        RigidBody::Fixed,
        WorkshopWorkbench,
        Name::new("WrappingWorkbench"),
    ));

    // 6. Delivery Chute (Placed at X = 6.0, Z = -4.0)
    let chute_metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.5, 0.3),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.8, 1.0, 1.8))),
        MeshMaterial3d(chute_metal),
        Transform::from_xyz(6.0, 0.5, -4.0),
        Collider::cuboid(0.9, 0.5, 0.9),
        RigidBody::Fixed,
        PresentDeliveryChute,
        Name::new("DeliveryChute"),
    ));

    // 7. Lighting
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.95, 0.88),
        brightness: 400.0,
        ..default()
    });

    // Indoor workshop incandescent bulb
    commands.spawn((
        PointLight {
            intensity: 950_000.0,
            range: 24.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.90, 0.70),
            ..default()
        },
        Transform::from_xyz(0.0, 4.8, 0.0),
        Name::new("WorkshopCeilingLight"),
    ));

    // Outdoor snowy sunlight shining in through the windows
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadow_maps_enabled: true,
            color: Color::srgb(0.92, 0.96, 1.0),
            ..default()
        },
        Transform::default().looking_to(Vec3::new(0.5, -0.8, 0.6), Vec3::Y),
        Name::new("WinterSunlight"),
    ));
}

/// Spawns raw present cubes at the start of the conveyor belt (X = -8.5, Z = 3.0)
pub fn conveyor_cube_spawner_system(
    time: Option<Res<Time>>,
    spawner: Option<ResMut<ConveyorSpawnerTimer>>,
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(time), Some(mut spawner)) = (time, spawner) else {
        return;
    };
    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let cube_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.5, 0.8), // Unwrapped toy blue
        perceptual_roughness: 0.4,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.6, 0.6))),
        MeshMaterial3d(cube_mat),
        Transform::from_xyz(-8.5, 0.9, 3.0),
        Collider::cuboid(0.3, 0.3, 0.3),
        RigidBody::Dynamic,
        Velocity {
            linear: Vec3::new(2.0, 0.0, 0.0),
            angular: Vec3::ZERO,
        },
        Grabbable,
        Present::default(),
        WrappingSpecification::default(),
        Name::new("ConveyorToyCube"),
    ));
}

/// Moves cubes resting on the conveyor belt forward (+X)
pub fn conveyor_belt_physics_system(
    q_conveyor: Query<(&Transform, &ConveyorBelt)>,
    mut q_cubes: Query<(&Transform, &mut Velocity), (With<Present>, With<Grabbable>)>,
) {
    let Ok((c_tf, belt)) = q_conveyor.single() else {
        return;
    };

    for (p_tf, mut vel) in &mut q_cubes {
        if (p_tf.translation.z - c_tf.translation.z).abs() < 0.65
            && p_tf.translation.y > 0.45
            && p_tf.translation.y < 1.2
            && p_tf.translation.x > -9.0
            && p_tf.translation.x < 9.0
        {
            vel.linear.x = belt.speed;
        }
    }
}

/// Despawns present cubes that reach the end of the conveyor belt into the despawn chute
pub fn conveyor_despawn_chute_system(
    mut commands: Commands,
    q_chute: Query<&Transform, With<ConveyorDespawnChute>>,
    q_presents: Query<(Entity, &Transform), With<Present>>,
) {
    let Ok(chute_tf) = q_chute.single() else {
        return;
    };

    for (entity, p_tf) in &q_presents {
        if p_tf.translation.distance(chute_tf.translation) < 1.4
            || (p_tf.translation.x > 8.8 && (p_tf.translation.z - 3.0).abs() < 0.8)
        {
            commands.entity(entity).despawn();
        }
    }
}
