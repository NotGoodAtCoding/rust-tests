use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::tailwind,
    prelude::*,
};
use bevy_rapier3d::prelude::*;

use super::{
    platforms::{MovingPlatform, MovingPlatformType},
    player::HeldByPlayer,
    DEFAULT_RENDER_LAYER, VIEW_MODEL_RENDER_LAYER,
};

#[derive(Debug, Component)]
pub struct SphereObject {
    pub name: &'static str,
    pub mass: f32,
    pub radius: f32,
    pub settle_cooldown: f32,
}

pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(32.0)));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::NEUTRAL_800),
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Name::new("Ground Visual"),
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Thick bedrock collider below y = 0.0 (top surface at y = 0.0, depth 4.0m)
    // NeverRender prevents the 64-meter bounding edges from causing near-plane gizmo projection glitches
    commands.spawn((
        Name::new("Ground Collider"),
        Transform::from_xyz(0.0, -2.0, 0.0),
        RigidBody::Fixed,
        Collider::cuboid(32.0, 2.0, 32.0),
        ColliderDebug::NeverRender,
        Friction {
            coefficient: 0.7,
            combine_rule: CoefficientCombineRule::Average,
        },
    ));

    let pillar_mesh = meshes.add(Cuboid::new(1.0, 3.5, 1.0));
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::INDIGO_600),
        perceptual_roughness: 0.4,
        ..default()
    });

    for &x in &[-11.0, 11.0] {
        for &z in &[-11.0, 11.0] {
            commands.spawn((
                Name::new("Pillar"),
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(pillar_mat.clone()),
                Transform::from_xyz(x, 1.75, z),
                RigidBody::Fixed,
                Collider::cuboid(0.5, 1.75, 0.5),
            ));
        }
    }

    let platform_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::EMERALD_700),
        perceptual_roughness: 0.3,
        ..default()
    });

    let step_heights = [0.4, 0.8, 1.2, 1.6];
    for (i, &height) in step_heights.iter().enumerate() {
        let step_mesh = meshes.add(Cuboid::new(2.0, height, 2.0));
        let z_pos = -3.0 - (i as f32 * 2.2);
        commands.spawn((
            Name::new(format!("Platform Step {}", i + 1)),
            Mesh3d(step_mesh),
            MeshMaterial3d(platform_mat.clone()),
            Transform::from_xyz(-4.0, height / 2.0, z_pos),
            RigidBody::Fixed,
            Collider::cuboid(1.0, height / 2.0, 1.0),
        ));
    }

    // Dynamic Moving Platforms
    // 1. Side-to-Side
    let s2s_origin = Vec3::new(7.0, 1.2, 0.0);
    let s2s_mesh = meshes.add(Cuboid::new(3.2, 0.4, 3.2));
    let s2s_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::BLUE_600),
        metallic: 0.5,
        perceptual_roughness: 0.2,
        ..default()
    });
    commands.spawn((
        Name::new("Side-to-Side Platform"),
        MovingPlatform::new(
            MovingPlatformType::SideToSide {
                origin: s2s_origin,
                amplitude: 4.5,
                speed: 1.2,
                axis: Vec3::Z,
            },
            s2s_origin,
        ),
        Mesh3d(s2s_mesh),
        MeshMaterial3d(s2s_mat),
        Transform::from_translation(s2s_origin),
        RigidBody::KinematicPositionBased,
        Collider::cuboid(1.6, 0.2, 1.6),
        Friction {
            coefficient: 1.0,
            combine_rule: CoefficientCombineRule::Max,
        },
        Velocity {
            linear: Vec3::ZERO,
            angular: Vec3::ZERO,
        },
    ));

    // 2. Vertical Elevator
    let vert_origin = Vec3::new(-7.5, 2.2, 0.0);
    let vert_mesh = meshes.add(Cuboid::new(3.2, 0.4, 3.2));
    let vert_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::PURPLE_600),
        metallic: 0.5,
        perceptual_roughness: 0.2,
        ..default()
    });
    commands.spawn((
        Name::new("Vertical Elevator Platform"),
        MovingPlatform::new(
            MovingPlatformType::Vertical {
                origin: vert_origin,
                amplitude: 1.8,
                speed: 1.0,
            },
            vert_origin,
        ),
        Mesh3d(vert_mesh),
        MeshMaterial3d(vert_mat),
        Transform::from_translation(vert_origin),
        RigidBody::KinematicPositionBased,
        Collider::cuboid(1.6, 0.2, 1.6),
        Friction {
            coefficient: 1.0,
            combine_rule: CoefficientCombineRule::Max,
        },
        Velocity {
            linear: Vec3::ZERO,
            angular: Vec3::ZERO,
        },
    ));

    // 3. Spinning Carousel
    let spin_origin = Vec3::new(0.0, 0.6, -9.0);
    let spin_mesh = meshes.add(Cylinder::new(2.6, 0.4));
    let spin_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::AMBER_600),
        metallic: 0.6,
        perceptual_roughness: 0.2,
        ..default()
    });
    commands.spawn((
        Name::new("Spinning Platform"),
        MovingPlatform::new(
            MovingPlatformType::Spinning {
                origin: spin_origin,
                speed: 0.9,
            },
            spin_origin,
        ),
        Mesh3d(spin_mesh),
        MeshMaterial3d(spin_mat),
        Transform::from_translation(spin_origin),
        RigidBody::KinematicPositionBased,
        Collider::cylinder(0.2, 2.6),
        Friction {
            coefficient: 1.0,
            combine_rule: CoefficientCombineRule::Max,
        },
        Velocity {
            linear: Vec3::ZERO,
            angular: Vec3::new(0.0, 0.9, 0.0),
        },
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
        RenderLayers::from_layers(&[DEFAULT_RENDER_LAYER, VIEW_MODEL_RENDER_LAYER]),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 150.0,
        ..default()
    });
}

pub fn setup_physics_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let light_radius = 0.35;
    let light_mesh = meshes.add(Sphere::new(light_radius));
    let light_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::AMBER_400),
        metallic: 0.4,
        perceptual_roughness: 0.3,
        ..default()
    });

    let medium_radius = 0.5;
    let medium_mesh = meshes.add(Sphere::new(medium_radius));
    let medium_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::CYAN_500),
        metallic: 0.6,
        perceptual_roughness: 0.25,
        ..default()
    });

    let heavy_radius = 0.65;
    let heavy_mesh = meshes.add(Sphere::new(heavy_radius));
    let heavy_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::ROSE_700),
        metallic: 0.85,
        perceptual_roughness: 0.2,
        ..default()
    });

    let sphere_configs = [
        ("Light Sphere", 4.0, light_radius, light_mesh.clone(), light_mat.clone(), 0.80, Vec3::new(-2.0, 2.5, 2.0)),
        ("Light Sphere", 4.0, light_radius, light_mesh.clone(), light_mat.clone(), 0.80, Vec3::new(0.0, 3.0, 2.5)),
        ("Light Sphere", 4.0, light_radius, light_mesh, light_mat, 0.80, Vec3::new(2.0, 3.5, 2.0)),
        ("Medium Sphere", 18.0, medium_radius, medium_mesh.clone(), medium_mat.clone(), 0.60, Vec3::new(-1.5, 3.0, 0.0)),
        ("Medium Sphere", 18.0, medium_radius, medium_mesh.clone(), medium_mat.clone(), 0.60, Vec3::new(1.5, 3.5, 0.0)),
        ("Medium Sphere", 18.0, medium_radius, medium_mesh, medium_mat, 0.60, Vec3::new(0.0, 4.0, -2.0)),
        ("Heavy Sphere", 60.0, heavy_radius, heavy_mesh.clone(), heavy_mat.clone(), 0.30, Vec3::new(-3.0, 3.0, -1.0)),
        ("Heavy Sphere", 60.0, heavy_radius, heavy_mesh, heavy_mat, 0.30, Vec3::new(3.0, 3.5, -1.0)),
    ];

    for (name, mass, radius, mesh, mat, restitution, pos) in sphere_configs {
        commands.spawn((
            Name::new(name),
            SphereObject {
                name,
                mass,
                radius,
                settle_cooldown: 3.5,
            },
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
            RigidBody::Dynamic,
            Collider::ball(radius),
            ColliderMassProperties::Mass(mass),
            Restitution::coefficient(restitution),
            Friction {
                coefficient: 0.6,
                combine_rule: CoefficientCombineRule::Average,
            },
            Damping {
                linear_damping: 0.3,
                angular_damping: 0.3,
            },
            Velocity {
                linear: Vec3::ZERO,
                angular: Vec3::ZERO,
            },
            GravityScale(1.0),
            Sleeping::default(),
            Ccd::enabled(),
        ));
    }
}

pub fn apply_low_velocity_damping(
    time: Res<Time>,
    mut spheres: Query<
        (&mut Transform, &mut Velocity, &Sleeping, &mut SphereObject),
        Without<HeldByPlayer>,
    >,
) {
    const LOW_SPEED_THRESHOLD: f32 = 0.45;
    const REST_LINEAR_THRESHOLD: f32 = 0.04;
    const REST_ANGULAR_THRESHOLD: f32 = 0.05;

    let dt = time.delta_secs();

    for (mut transform, mut velocity, sleeping, mut sphere) in spheres.iter_mut() {
        if sphere.settle_cooldown > 0.0 {
            sphere.settle_cooldown -= dt;
            continue;
        }

        // Safety clamp: ensure spheres above the main ground area never sink through the floor
        if transform.translation.x.abs() <= 32.0 && transform.translation.z.abs() <= 32.0 {
            let min_y = sphere.radius;
            if transform.translation.y < min_y - 0.02 {
                transform.translation.y = min_y;
                if velocity.linear.y < 0.0 {
                    velocity.linear.y = 0.0;
                }
            }
        }

        if sleeping.sleeping {
            continue;
        }

        // Only damp when nearly settled vertically (not bouncing high in the air)
        if velocity.linear.y.abs() > 0.18 {
            continue;
        }

        let horiz_speed = velocity.linear.xz().length();
        let ang_speed = velocity.angular.length();

        if horiz_speed < REST_LINEAR_THRESHOLD && ang_speed < REST_ANGULAR_THRESHOLD {
            velocity.linear.x = 0.0;
            velocity.linear.z = 0.0;
            velocity.angular = Vec3::ZERO;
            // Rapier's built-in Sleeping system will naturally sleep the body
            // once horizontal and vertical velocities remain below threshold,
            // avoiding freezing during active penetration resolution.
        } else if horiz_speed < LOW_SPEED_THRESHOLD {
            let damp = (1.0 - dt * 3.5).max(0.0);
            // Only damp planar horizontal motion and spin; never damp vertical contact response!
            velocity.linear.x *= damp;
            velocity.linear.z *= damp;
            velocity.angular *= damp;
        }
    }
}
