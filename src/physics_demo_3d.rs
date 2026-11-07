use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::tailwind,
    input::mouse::AccumulatedMouseMotion,
    light::NotShadowCaster,
    prelude::*,
};
use bevy_rapier3d::prelude::*;

const DEFAULT_RENDER_LAYER: usize = 0;
const VIEW_MODEL_RENDER_LAYER: usize = 1;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy 3D - First-Person Viewmodel & Physics Demo".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(
            Startup,
            (
                setup_scene,
                setup_physics_objects,
                spawn_player_viewmodel,
                spawn_ui,
            ),
        )
        .add_systems(
            Update,
            (player_look, player_move, viewmodel_bob, change_fov),
        )
        .run();
}

#[derive(Debug, Component)]
pub struct Player;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct CameraSensitivity(pub Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(Vec2::new(0.003, 0.002))
    }
}

#[derive(Debug, Component)]
pub struct WorldModelCamera;

#[derive(Debug, Component)]
pub struct ViewModelArm {
    pub base_translation: Vec3,
}

fn spawn_player_viewmodel(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arm_mesh = meshes.add(Cuboid::new(0.12, 0.12, 0.5));
    let sleeve_mesh = meshes.add(Cuboid::new(0.15, 0.15, 0.2));
    let arm_material = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::TEAL_300),
        perceptual_roughness: 0.8,
        ..default()
    });
    let sleeve_material = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::SLATE_800),
        perceptual_roughness: 0.5,
        ..default()
    });

    let initial_arm_pos = Vec3::new(0.25, -0.2, -0.4);

    commands.spawn((
        Player,
        CameraSensitivity::default(),
        Transform::from_xyz(0.0, 1.8, 6.0),
        Visibility::default(),
        children![
            // World model camera (uses default layer 0, variable FOV)
            (
                WorldModelCamera,
                Camera3d::default(),
                Projection::from(PerspectiveProjection {
                    fov: 80.0_f32.to_radians(),
                    ..default()
                }),
            ),
            // View model camera (uses layer 1, fixed FOV 70 deg, renders on top)
            (
                Camera3d::default(),
                Camera {
                    order: 1,
                    ..default()
                },
                Projection::from(PerspectiveProjection {
                    fov: 70.0_f32.to_radians(),
                    ..default()
                }),
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            ),
            // View model arm
            (
                ViewModelArm {
                    base_translation: initial_arm_pos,
                },
                Transform::from_translation(initial_arm_pos),
                Visibility::default(),
                children![
                    (
                        Mesh3d(arm_mesh),
                        MeshMaterial3d(arm_material),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                        RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                        NotShadowCaster,
                    ),
                    (
                        Mesh3d(sleeve_mesh),
                        MeshMaterial3d(sleeve_material),
                        Transform::from_xyz(0.0, 0.0, 0.15),
                        RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                        NotShadowCaster,
                    ),
                ],
            ),
        ],
    ));
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground floor
    let ground_mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(25.0)));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::NEUTRAL_800),
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Name::new("Ground"),
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider::cuboid(25.0, 0.1, 25.0),
    ));

    // Environmental pillars
    let pillar_mesh = meshes.add(Cuboid::new(1.0, 3.0, 1.0));
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::INDIGO_600),
        perceptual_roughness: 0.4,
        ..default()
    });

    for &x in &[-5.0, 5.0] {
        for &z in &[-5.0, 5.0] {
            commands.spawn((
                Name::new("Pillar"),
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(pillar_mat.clone()),
                Transform::from_xyz(x, 1.5, z),
                Collider::cuboid(0.5, 1.5, 0.5),
            ));
        }
    }

    // Directional light illuminating both world model and view model
    commands.spawn((
        DirectionalLight {
            illuminance: 7500.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
        RenderLayers::from_layers(&[DEFAULT_RENDER_LAYER, VIEW_MODEL_RENDER_LAYER]),
    ));

    // Global ambient light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 150.0,
        ..default()
    });
}

fn setup_physics_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere_mesh = meshes.add(Sphere::new(0.5));
    let sphere_mat = materials.add(StandardMaterial {
        base_color: Color::from(tailwind::AMBER_400),
        metallic: 0.6,
        perceptual_roughness: 0.2,
        ..default()
    });

    // Spawn bouncing physics spheres
    for i in 0..5 {
        let offset = (i as f32 - 2.0) * 1.5;
        commands.spawn((
            Name::new(format!("Physics Ball {}", i)),
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(sphere_mat.clone()),
            Transform::from_xyz(offset, 4.0 + (i as f32 * 0.5), -2.0),
            RigidBody::Dynamic,
            Collider::ball(0.5),
            Restitution::coefficient(0.8),
            Velocity {
                linear: Vec3::new(0.0, 1.0 + (i as f32 * 0.2), 0.0),
                angular: Vec3::new(0.2, 0.1, 0.0),
            },
            GravityScale(1.0),
            Sleeping::disabled(),
            Ccd::enabled(),
        ));
    }
}

fn spawn_ui(mut commands: Commands) {
    let font = TextFont {
        font_size: FontSize::Px(16.0),
        ..default()
    };
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            left: px(16),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12)),
            ..default()
        },
        children![
            (
                Text::new("First-Person Viewmodel Demo (Bevy 0.20)"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                }
            ),
            (
                Text::new("WASD: Move | Shift/Space: Down/Up"),
                font.clone()
            ),
            (Text::new("Mouse: Look around"), font.clone()),
            (Text::new("Arrow Up / Down: Adjust World FOV"), font),
        ],
    ));
}

fn player_look(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    player: Single<(&mut Transform, &CameraSensitivity), With<Player>>,
) {
    let (mut transform, camera_sensitivity) = player.into_inner();
    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

fn player_move(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    player: Single<&mut Transform, With<Player>>,
) {
    let mut transform = player.into_inner();
    let mut direction = Vec3::ZERO;

    let forward = Vec3::new(transform.forward().x, 0.0, transform.forward().z).normalize_or_zero();
    let right = Vec3::new(transform.right().x, 0.0, transform.right().z).normalize_or_zero();

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction += right;
    }
    if keyboard_input.pressed(KeyCode::Space) {
        direction += Vec3::Y;
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        direction -= Vec3::Y;
    }

    if direction != Vec3::ZERO {
        const SPEED: f32 = 6.0;
        transform.translation += direction.normalize() * SPEED * time.delta_secs();
    }
}

fn viewmodel_bob(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    view_model: Single<(&mut Transform, &ViewModelArm)>,
) {
    let (mut transform, arm) = view_model.into_inner();
    let is_moving = keyboard_input.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyS,
        KeyCode::KeyA,
        KeyCode::KeyD,
    ]);

    if is_moving {
        let elapsed = time.elapsed_secs() * 8.0;
        let bob_x = (elapsed * 0.5).sin() * 0.015;
        let bob_y = elapsed.sin().abs() * 0.015;
        transform.translation = arm.base_translation + Vec3::new(bob_x, -bob_y, 0.0);
    } else {
        transform.translation = transform
            .translation
            .lerp(arm.base_translation, time.delta_secs() * 10.0);
    }
}

fn change_fov(
    input: Res<ButtonInput<KeyCode>>,
    mut world_model_projection: Single<&mut Projection, With<WorldModelCamera>>,
) {
    let Projection::Perspective(perspective) = world_model_projection.as_mut() else {
        return;
    };

    if input.pressed(KeyCode::ArrowUp) {
        perspective.fov -= 1.0_f32.to_radians();
        perspective.fov = perspective.fov.max(20.0_f32.to_radians());
    }
    if input.pressed(KeyCode::ArrowDown) {
        perspective.fov += 1.0_f32.to_radians();
        perspective.fov = perspective.fov.min(150.0_f32.to_radians());
    }
}
