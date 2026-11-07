use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct SpringPoint {
    pub position: Vec3,
    pub original_offset: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

#[derive(Component, Debug, Clone)]
pub struct SoftBodyLattice {
    pub points: Vec<SpringPoint>,
    pub stiffness: f32,
    pub damping: f32,
}

impl SoftBodyLattice {
    pub fn new_cube(half_extents: Vec3, stiffness: f32, damping: f32) -> Self {
        let mut points = Vec::new();
        for &x in &[-half_extents.x, half_extents.x] {
            for &y in &[-half_extents.y, half_extents.y] {
                for &z in &[-half_extents.z, half_extents.z] {
                    let offset = Vec3::new(x, y, z);
                    points.push(SpringPoint {
                        position: offset,
                        original_offset: offset,
                        velocity: Vec3::ZERO,
                        mass: 0.1,
                    });
                }
            }
        }
        Self {
            points,
            stiffness,
            damping,
        }
    }

    pub fn apply_impulse(&mut self, impulse: Vec3) {
        for pt in &mut self.points {
            pt.velocity += impulse / pt.mass;
        }
    }

    pub fn step(&mut self, dt: f32) {
        for pt in &mut self.points {
            // Hooke's Law: F = -k * displacement - damping * velocity
            let displacement = pt.position - pt.original_offset;
            let spring_force = -self.stiffness * displacement;
            let damping_force = -self.damping * pt.velocity;
            let total_force = spring_force + damping_force;

            let accel = total_force / pt.mass;
            pt.velocity += accel * dt;
            pt.position += pt.velocity * dt;
        }
    }
}

pub fn update_soft_bodies(time: Res<Time>, mut query: Query<&mut SoftBodyLattice>) {
    let dt = time.delta_secs();
    for mut lattice in &mut query {
        lattice.step(dt);
    }
}

pub struct PhysicsSandboxPlugin;

impl Plugin for PhysicsSandboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_soft_bodies);
    }
}
