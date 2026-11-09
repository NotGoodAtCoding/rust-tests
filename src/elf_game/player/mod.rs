use bevy::prelude::*;

pub mod controller;
pub mod hands;
pub mod bunny_ears;

pub use bunny_ears::{
    BunnyEarsEquipped, BunnyEarsItem, FloppyEar, ViewmodelBunnyEars,
    spawn_bunny_ears_item, spawn_viewmodel_bunny_ears,
};
pub use hands::HeavyItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BhopWindow {
    Sixtieth,   // 1/60s (~16.6ms)
    Thirtieth,  // 1/30s (~33.3ms)
    Tenth,      // 1/10s (~100.0ms)
}

impl BhopWindow {
    pub fn duration_secs(&self) -> f32 {
        match self {
            Self::Sixtieth => 1.0 / 60.0,
            Self::Thirtieth => 1.0 / 30.0,
            Self::Tenth => 1.0 / 10.0,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Sixtieth => Self::Thirtieth,
            Self::Thirtieth => Self::Tenth,
            Self::Tenth => Self::Sixtieth,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sixtieth => "1/60s (Strict)",
            Self::Thirtieth => "1/30s (Medium)",
            Self::Tenth => "1/10s (Forgiving)",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct BhopConfig {
    pub window: BhopWindow,
    pub max_boost_multiplier: f32, // sqrt(2) ~ 1.414
}

impl Default for BhopConfig {
    fn default() -> Self {
        Self {
            window: BhopWindow::Sixtieth,
            max_boost_multiplier: 1.41421356, // sqrt(2)
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct ElfMovement {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub base_speed: f32,
    pub current_max_speed: f32,
    pub time_since_landing: f32,
    pub consecutive_bhops: u32,
    pub ground_accel: f32,
    pub air_strain_accel: f32,
    pub has_bunny_ears: bool,
}

impl Default for ElfMovement {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            is_grounded: true,
            base_speed: 6.0,
            current_max_speed: 6.0,
            time_since_landing: 0.0,
            consecutive_bhops: 0,
            ground_accel: 40.0,
            air_strain_accel: 5.0, // ~12.5% of ground accel
            has_bunny_ears: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandSide {
    Left,
    Right,
}

#[derive(Component, Debug, Clone, Default)]
pub struct ElfHands {
    pub left_grabbed: Option<Entity>,
    pub right_grabbed: Option<Entity>,
    // Two-handed tape pulling state
    pub tape_pulling_active: bool,
    pub current_tape_length: f32, // meters, e.g. 0.05 to 0.40m
    pub tape_tangled: bool,
    // Two-handed ribbon shaking state
    pub ribbon_shake_energy: f32,
}

impl ElfHands {
    pub const MIN_TAPE_LENGTH: f32 = 0.05;
    pub const MAX_TAPE_LENGTH: f32 = 0.40;
    pub const TAPE_PULL_SPEED: f32 = 0.25; // meters per second
    pub const RIBBON_SHAKE_GOAL: f32 = 1.0;

    /// Update pulling tape when holding tape dispenser in one hand and pulling with the other
    pub fn update_tape_pull(&mut self, dt: f32, is_pulling: bool) -> Option<f32> {
        if is_pulling {
            self.tape_pulling_active = true;
            self.current_tape_length += Self::TAPE_PULL_SPEED * dt;
            if self.current_tape_length > Self::MAX_TAPE_LENGTH {
                self.tape_tangled = true;
            }
            None
        } else if self.tape_pulling_active {
            // Cut or released tape
            self.tape_pulling_active = false;
            let final_len = self.current_tape_length;
            let tangled = self.tape_tangled;
            self.current_tape_length = 0.0;
            self.tape_tangled = false;
            if !tangled && final_len >= Self::MIN_TAPE_LENGTH {
                Some(final_len)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Add shake impulse when holding ribbon with both hands. Returns true when a Bow is created.
    pub fn add_shake_energy(&mut self, delta_velocity: f32) -> bool {
        self.ribbon_shake_energy += delta_velocity;
        if self.ribbon_shake_energy >= Self::RIBBON_SHAKE_GOAL {
            self.ribbon_shake_energy = 0.0;
            true
        } else {
            false
        }
    }
}

/// Applies air straining vs ground acceleration and processes bunny hopping
pub fn update_elf_physics(
    dt: f32,
    wish_dir: Vec3,
    jump_just_pressed: bool,
    jump_held: bool,
    config: &BhopConfig,
    movement: &mut ElfMovement,
) {
    let effective_window_secs = if movement.has_bunny_ears {
        1.0 / 10.0 // 1/10s grace period when equipped
    } else {
        config.window.duration_secs()
    };

    let effective_jump_impulse = if movement.has_bunny_ears {
        7.0 * 1.10 // 10% higher jump
    } else {
        7.0
    };

    let effective_air_strain = if movement.has_bunny_ears {
        movement.air_strain_accel * 2.0 // 2x the in air straining vs normal limits
    } else {
        movement.air_strain_accel
    };

    if movement.is_grounded {
        movement.time_since_landing += dt;

        // Holding space never allows a bunny-hop
        if jump_just_pressed {
            if movement.time_since_landing <= effective_window_secs {
                // Perfect bunny hop!
                movement.consecutive_bhops += 1;
                if movement.consecutive_bhops >= 3 {
                    movement.current_max_speed = movement.base_speed * config.max_boost_multiplier;
                } else {
                    let progress = movement.consecutive_bhops as f32 / 3.0;
                    movement.current_max_speed = movement.base_speed * (1.0 + progress * (config.max_boost_multiplier - 1.0));
                }
            } else {
                // Too late for a bunny hop, normal jump resets streak
                movement.consecutive_bhops = 0;
                movement.current_max_speed = movement.base_speed;
            }
            movement.is_grounded = false;
            movement.velocity.y = effective_jump_impulse;
        } else {
            // Reset vertical velocity to 0.0 upon landing on ground after jumping
            movement.velocity.y = 0.0;

            if jump_held {
                // Held space on ground: does not trigger hop
                if movement.time_since_landing > effective_window_secs {
                    movement.consecutive_bhops = 0;
                    movement.current_max_speed = movement.base_speed;
                }
            } else {
                // Player lingered on ground without jumping, decay bhop combo
                if movement.time_since_landing > effective_window_secs {
                    movement.consecutive_bhops = 0;
                    movement.current_max_speed = movement.base_speed;
                }
            }
        }

        // Ground acceleration / deceleration
        if movement.is_grounded {
            let target_vel = wish_dir * movement.current_max_speed;
            let current_h = Vec3::new(movement.velocity.x, 0.0, movement.velocity.z);
            let diff = target_vel - current_h;
            let step = diff.clamp_length_max(movement.ground_accel * dt);
            movement.velocity.x += step.x;
            movement.velocity.z += step.z;
        }
    } else {
        // Airborne: reset landing timer
        movement.time_since_landing = 0.0;

        // Realistic air physics: cannot set velocity directly, only strain with low acceleration
        if wish_dir.length_squared() > 0.001 {
            let strain = wish_dir * effective_air_strain * dt;
            movement.velocity.x += strain.x;
            movement.velocity.z += strain.z;
            
            // Limit horizontal velocity to current max speed
            let h_vel = Vec2::new(movement.velocity.x, movement.velocity.z);
            if h_vel.length() > movement.current_max_speed {
                let clamped = h_vel.clamp_length_max(movement.current_max_speed);
                movement.velocity.x = clamped.x;
                movement.velocity.z = clamped.y;
            }
        }

        // Gravity
        movement.velocity.y -= 19.6 * dt;
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BhopConfig>()
            .init_resource::<controller::ElfInputSettings>()
            .add_systems(Startup, (controller::spawn_elf_player, hands::spawn_workshop_tools_and_materials))
            .add_systems(
                Update,
                (
                    controller::toggle_noclip_system,
                    controller::sync_noclip_collider_system,
                    (
                        controller::elf_look_system,
                        controller::elf_player_controller_system,
                        controller::viewmodel_bob,
                        controller::floaty_hands_system,
                        hands::dual_hand_interaction_system,
                        bunny_ears::player_bunny_ears_equip_system,
                        bunny_ears::floppy_ears_physics_bob_system,
                    ).chain(),
                ),
            );
    }
}
