use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum MovingPlatformType {
    SideToSide {
        origin: Vec3,
        amplitude: f32,
        speed: f32,
        axis: Vec3,
    },
    Vertical {
        origin: Vec3,
        amplitude: f32,
        speed: f32,
    },
    Spinning {
        origin: Vec3,
        speed: f32,
    },
}

#[derive(Debug, Component)]
pub struct MovingPlatform {
    pub platform_type: MovingPlatformType,
    pub previous_translation: Vec3,
    pub current_translation: Vec3,
    pub previous_rotation: Quat,
    pub current_rotation: Quat,
    pub delta_translation: Vec3,
    pub delta_rotation: Quat,
}

impl MovingPlatform {
    pub fn new(platform_type: MovingPlatformType, origin: Vec3) -> Self {
        Self {
            platform_type,
            previous_translation: origin,
            current_translation: origin,
            previous_rotation: Quat::IDENTITY,
            current_rotation: Quat::IDENTITY,
            delta_translation: Vec3::ZERO,
            delta_rotation: Quat::IDENTITY,
        }
    }
}

pub fn update_moving_platforms(
    time: Res<Time>,
    mut platforms: Query<(&mut Transform, &mut MovingPlatform, Option<&mut Velocity>)>,
) {
    let t = time.elapsed_secs();

    for (mut transform, mut platform, mut vel) in platforms.iter_mut() {
        platform.previous_translation = platform.current_translation;
        platform.previous_rotation = platform.current_rotation;

        let (new_pos, new_rot, lin_vel, ang_vel) = match platform.platform_type {
            MovingPlatformType::SideToSide {
                origin,
                amplitude,
                speed,
                axis,
            } => {
                let phase = speed * t;
                let pos = origin + axis * (amplitude * phase.sin());
                let rot = Quat::IDENTITY;
                let v = axis * (amplitude * speed * phase.cos());
                (pos, rot, v, Vec3::ZERO)
            }
            MovingPlatformType::Vertical {
                origin,
                amplitude,
                speed,
            } => {
                let phase = speed * t;
                let pos = origin + Vec3::new(0.0, amplitude * phase.sin(), 0.0);
                let rot = Quat::IDENTITY;
                let v_y = amplitude * speed * phase.cos();
                (pos, rot, Vec3::new(0.0, v_y, 0.0), Vec3::ZERO)
            }
            MovingPlatformType::Spinning { origin, speed } => {
                let rot = Quat::from_rotation_y(speed * t);
                (origin, rot, Vec3::ZERO, Vec3::new(0.0, speed, 0.0))
            }
        };

        platform.current_translation = new_pos;
        platform.current_rotation = new_rot;
        platform.delta_translation = new_pos - platform.previous_translation;
        platform.delta_rotation = new_rot * platform.previous_rotation.inverse();

        transform.translation = new_pos;
        transform.rotation = new_rot;

        if let Some(ref mut v) = vel {
            v.linear = lin_vel;
            v.angular = ang_vel;
        }
    }
}
