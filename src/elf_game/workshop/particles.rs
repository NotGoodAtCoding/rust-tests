use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader};
use crate::elf_game::{
    assets::palette::*,
    core::GameState,
    ui::MenuState,
};

/// Message triggering a celebratory confetti burst at a 3D location
#[derive(Message, Debug, Clone)]
pub struct SpawnConfettiEvent {
    pub position: Vec3,
    pub count: usize,
}

/// Message triggering physical glass shatter shard particles at a 3D location
#[derive(Message, Debug, Clone)]
pub struct SpawnGlassShardsEvent {
    pub position: Vec3,
    pub count: usize,
}

/// Dynamic 3D fluttering celebration confetti particle
#[derive(Component, Debug, Clone)]
pub struct ConfettiParticle {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub flutter_freq: f32,
}

/// Dynamic 3D tumbling sharp glass shard particle with bounce physics
#[derive(Component, Debug, Clone)]
pub struct GlassShardParticle {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

/// Spawns a radial burst of colorful celebratory confetti quads
pub fn spawn_confetti_burst(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    count: usize,
) {
    let colors = [
        FESTIVE_RED,
        PINE_GREEN,
        GOLD_RIBBON,
        SNOW_WHITE,
        CANDY_CANE_RED,
    ];

    let quad_mesh = meshes.add(Rectangle::new(0.08, 0.08));

    for i in 0..count {
        let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
        let spread_y = 1.8 + ((i * 7) % 11) as f32 * 0.25;
        let speed = 2.5 + ((i * 13) % 9) as f32 * 0.35;

        let vel = Vec3::new(
            angle.cos() * speed,
            spread_y,
            angle.sin() * speed,
        );

        let mat = materials.add(StandardMaterial {
            base_color: colors[i % colors.len()],
            double_sided: true,
            cull_mode: None,
            unlit: true,
            ..default()
        });

        let lifetime = 1.4 + ((i * 5) % 8) as f32 * 0.15;

        commands.spawn((
            Mesh3d(quad_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(origin + Vec3::new(
                ((i * 3) % 7) as f32 * 0.04 - 0.12,
                0.0,
                ((i * 5) % 7) as f32 * 0.04 - 0.12,
            )),
            ConfettiParticle {
                velocity: vel,
                angular_velocity: Vec3::new(
                    4.0 + (i % 5) as f32,
                    6.0 + (i % 7) as f32,
                    3.0 + (i % 4) as f32,
                ),
                lifetime,
                max_lifetime: lifetime,
                flutter_freq: 8.0 + (i % 6) as f32 * 1.5,
            },
            Name::new("ConfettiParticle"),
        ));
    }
}

/// Spawns physical sharp glass shard particles tumbling from a shattered window
pub fn spawn_glass_shards(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    count: usize,
) {
    let shard_mesh = meshes.add(Cuboid::new(0.07, 0.015, 0.09));
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.88, 0.94, 0.98, 0.75),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.1,
        perceptual_roughness: 0.05,
        reflectance: 0.8,
        ..default()
    });

    for i in 0..count {
        let angle_x = ((i * 17) % 31) as f32 * 0.2 - 3.1;
        let spread_y = 0.5 + ((i * 5) % 9) as f32 * 0.4;
        let spread_z = 2.0 + ((i * 11) % 13) as f32 * 0.35; // Shatters inward toward workshop (+Z)

        let vel = Vec3::new(angle_x, spread_y, spread_z);
        let lifetime = 2.2 + ((i * 3) % 7) as f32 * 0.2;

        commands.spawn((
            Mesh3d(shard_mesh.clone()),
            MeshMaterial3d(glass_mat.clone()),
            Transform::from_translation(origin + Vec3::new(
                ((i * 4) % 11) as f32 * 0.12 - 0.6,
                ((i * 6) % 11) as f32 * 0.12 - 0.6,
                0.0,
            )),
            GlassShardParticle {
                velocity: vel,
                angular_velocity: Vec3::new(
                    8.0 + (i % 9) as f32 * 2.0,
                    10.0 + (i % 7) as f32 * 2.0,
                    5.0 + (i % 5) as f32 * 2.0,
                ),
                lifetime,
                max_lifetime: lifetime,
            },
            Name::new("GlassShardParticle"),
        ));
    }
}

/// Message listener system spawning confetti bursts on event dispatch
pub fn confetti_spawner_listener_system(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    events: Option<MessageReader<SpawnConfettiEvent>>,
) {
    let (Some(mut meshes), Some(mut materials), Some(mut events)) = (
        meshes,
        materials,
        events,
    ) else {
        return;
    };

    for ev in events.read() {
        spawn_confetti_burst(&mut commands, &mut meshes, &mut materials, ev.position, ev.count);
    }
}

/// Message listener system spawning glass shards on window break
pub fn glass_shard_spawner_listener_system(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    events: Option<MessageReader<SpawnGlassShardsEvent>>,
) {
    let (Some(mut meshes), Some(mut materials), Some(mut events)) = (
        meshes,
        materials,
        events,
    ) else {
        return;
    };

    for ev in events.read() {
        spawn_glass_shards(&mut commands, &mut meshes, &mut materials, ev.position, ev.count);
    }
}

/// Updates 3D fluttering confetti dynamics: gravity, air drag, flutter sway, and despawn
pub fn confetti_particle_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<GameState>>>,
    mut commands: Commands,
    mut q_particles: Query<(Entity, &mut Transform, &mut ConfettiParticle)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    for (entity, mut tf, mut p) in &mut q_particles {
        p.lifetime -= dt;
        if p.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Gravity and air drag
        p.velocity.y -= 3.8 * dt;
        p.velocity.x *= (1.0 - 0.75 * dt).max(0.0);
        p.velocity.z *= (1.0 - 0.75 * dt).max(0.0);

        // Sinusoidal fluttering flutter
        let elapsed = p.max_lifetime - p.lifetime;
        let flutter_x = (elapsed * p.flutter_freq).sin() * 0.35;
        let flutter_z = (elapsed * (p.flutter_freq * 0.8)).cos() * 0.25;

        tf.translation += p.velocity * dt + Vec3::new(flutter_x, 0.0, flutter_z) * dt;
        tf.rotate_local_x(p.angular_velocity.x * dt);
        tf.rotate_local_y(p.angular_velocity.y * dt);
    }
}

/// Updates tumbling sharp glass shards: gravity, floor collision bouncing, and decay
pub fn glass_shard_particle_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<GameState>>>,
    mut commands: Commands,
    mut q_shards: Query<(Entity, &mut Transform, &mut GlassShardParticle)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    if dt <= 0.0 {
        return;
    }

    for (entity, mut tf, mut shard) in &mut q_shards {
        shard.lifetime -= dt;
        if shard.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        shard.velocity.y -= 9.8 * dt;
        tf.translation += shard.velocity * dt;

        // Ground floor bounce (at Y = 0.05)
        if tf.translation.y <= 0.05 && shard.velocity.y < 0.0 {
            tf.translation.y = 0.05;
            shard.velocity.y = -shard.velocity.y * 0.35;
            shard.velocity.x *= 0.65;
            shard.velocity.z *= 0.65;
        }

        tf.rotate_local_x(shard.angular_velocity.x * dt);
        tf.rotate_local_z(shard.angular_velocity.z * dt);
    }
}
