use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    core::WorkshopQuota,
    moose::{MooseBreachMessage, MooseHunger, MooseHungerStage},
    player::ElfMovement,
    wrapping::{Present, PresentDeliveredMessage},
    workshop::room::{BreakableWindowPane, PresentDeliveryChute},
    moose::toy_model::{MrMoose, MooseHead, WindowShutters},
};

#[derive(Component, Debug, Clone)]
pub struct MooseRampageTracker {
    pub current_waypoint_index: usize,
    pub speed: f32,
    pub base_speed: f32,
    pub waypoints: Vec<Vec3>,
    /// Timer accumulating seconds toward next bucking animation
    pub buck_timer: f32,
    /// Randomized duration threshold before triggering the next buck
    pub next_buck_interval: f32,
    /// Normalized progress within the active bucking animation [0.0, 1.0]
    pub buck_progress: f32,
    /// Duration of one complete buck cycle in seconds
    pub buck_duration: f32,
    /// Whether the Moose is actively bucking
    pub is_bucking: bool,
    /// Current pitch angle in radians of the bucking tilt
    pub current_buck_pitch: f32,
}

impl Default for MooseRampageTracker {
    fn default() -> Self {
        Self {
            current_waypoint_index: 0,
            speed: 8.5,
            base_speed: 8.5,
            waypoints: vec![
                Vec3::new(-6.5, 1.4, -10.0), // Leap into workshop through window
                Vec3::new(0.0, 1.4, -4.0),   // Slam into central workbench
                Vec3::new(5.0, 1.4, 3.0),    // Charge across conveyor belt
                Vec3::new(-3.0, 1.4, 6.0),   // Trample storage area
                Vec3::new(3.0, 1.4, 6.0),    // Trample south corner
                Vec3::new(-6.5, 1.4, -4.0),  // Charge back towards window
                Vec3::new(-6.5, 1.4, -10.0), // Leap back out window sill
                Vec3::new(-6.5, 1.8, -17.0), // Retreat to snowy hill behind shutters
            ],
            buck_timer: 0.0,
            next_buck_interval: 1.2,
            buck_progress: 0.0,
            buck_duration: 0.85,
            is_bucking: false,
            current_buck_pitch: 0.0,
        }
    }
}

/// Detects presents dropped inside the Delivery Chute and increments workshop quotas
pub fn monitor_present_delivery(
    mut q_presents: Query<(Entity, &Transform, &Present)>,
    q_chute: Query<&Transform, With<PresentDeliveryChute>>,
    mut quota: ResMut<WorkshopQuota>,
    mut delivery_messages: Option<MessageWriter<PresentDeliveredMessage>>,
    mut commands: Commands,
) {
    let Ok(chute_tf) = q_chute.single() else {
        return;
    };

    for (entity, p_tf, present) in &mut q_presents {
        if present.is_completed && p_tf.translation.distance(chute_tf.translation) < 1.2 {
            quota.presents_delivered += 1;
            if let Some(ref mut writer) = delivery_messages {
                writer.write(PresentDeliveredMessage {
                    present_entity: entity,
                    quality: present.quality_score,
                });
            }
            commands.entity(entity).despawn();
        }
    }
}

/// Handles Mr. Moose breach event: shatters the observation window and starts the rampage along the set path
pub fn handle_moose_window_breach(
    breach_messages: Option<MessageReader<MooseBreachMessage>>,
    mut q_window: Query<(&mut Visibility, &mut BreakableWindowPane)>,
    mut q_moose: Query<(Entity, &mut Transform), With<MrMoose>>,
    mut q_player: Query<&mut ElfMovement>,
    mut commands: Commands,
) {
    let Some(mut messages) = breach_messages else {
        return;
    };

    for _ in messages.read() {
        // Break window
        for (mut vis, mut pane) in &mut q_window {
            pane.is_broken = true;
            *vis = Visibility::Hidden;
        }

        // Add rampage tracker to Mr. Moose and leap into window opening at X = -6.5
        for (moose_entity, mut moose_tf) in &mut q_moose {
            moose_tf.translation = Vec3::new(-6.5, 1.4, -10.0);
            moose_tf.rotation = Quat::IDENTITY;
            commands.entity(moose_entity).insert(MooseRampageTracker::default());
        }

        // Screen knockback impulse on player
        for mut movement in &mut q_player {
            movement.velocity.y = 3.0;
        }
    }
}

/// Follows the deterministic waypoint path through the workshop, faces forward along motion,
/// applies extreme impulse forces to objects, and retreats back behind window when complete.
pub fn moose_rampage_path_and_physics_impulse_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    hunger: Option<ResMut<MooseHunger>>,
    mut q_moose: Query<
        (Entity, &mut Transform, &mut MooseRampageTracker),
        (
            With<MrMoose>,
            Without<ExternalImpulse>,
            Without<ElfMovement>,
            Without<WindowShutters>,
            Without<MooseHead>,
        ),
    >,
    mut q_dynamic_bodies: Query<
        (&Transform, &mut ExternalImpulse, Option<&mut crate::elf_game::physics_sandbox::SoftBodyLattice>),
        (
            Without<MrMoose>,
            Without<ElfMovement>,
            Without<WindowShutters>,
            Without<MooseHead>,
        ),
    >,
    mut q_player: Query<
        (&Transform, &mut ElfMovement),
        (
            Without<MrMoose>,
            Without<ExternalImpulse>,
            Without<WindowShutters>,
            Without<MooseHead>,
        ),
    >,
    mut q_shutters: Query<
        (&mut Transform, &mut Visibility),
        (
            With<WindowShutters>,
            Without<MrMoose>,
            Without<MooseHead>,
            Without<ExternalImpulse>,
            Without<ElfMovement>,
        ),
    >,
    mut q_head: Query<
        &mut Transform,
        (
            With<MooseHead>,
            Without<MrMoose>,
            Without<WindowShutters>,
            Without<ExternalImpulse>,
            Without<ElfMovement>,
        ),
    >,
    mut q_window: Query<
        (&mut Visibility, &mut BreakableWindowPane),
        (
            Without<MrMoose>,
            Without<WindowShutters>,
            Without<MooseHead>,
            Without<ExternalImpulse>,
            Without<ElfMovement>,
        ),
    >,
    mut commands: Commands,
) {
    let (Some(time), Some(mut hunger)) = (time, hunger) else {
        return;
    };

    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    if hunger.stage != MooseHungerStage::BreachedRampage {
        // Fallback cleanup if hunger was reset out-of-band while moose was still tracking
        for (moose_entity, mut moose_tf, _tracker) in &mut q_moose {
            moose_tf.translation = Vec3::new(-6.5, 1.8, -17.0);
            moose_tf.rotation = Quat::IDENTITY;
            moose_tf.scale = Vec3::ONE;

            for mut h_tf in &mut q_head {
                h_tf.translation = Vec3::new(0.0, 1.1, 1.6);
                h_tf.rotation = Quat::IDENTITY;
                h_tf.scale = Vec3::ONE;
            }

            for (mut s_tf, mut s_vis) in &mut q_shutters {
                s_tf.rotation = Quat::IDENTITY;
                *s_vis = Visibility::Inherited;
            }

            for (mut w_vis, mut pane) in &mut q_window {
                pane.is_broken = false;
                *w_vis = Visibility::Inherited;
            }

            commands.entity(moose_entity).remove::<MooseRampageTracker>();
        }
        return;
    }

    let dt = time.delta_secs();

    for (moose_entity, mut moose_tf, mut tracker) in &mut q_moose {
        if tracker.current_waypoint_index >= tracker.waypoints.len() {
            // Rampage finished! Moose has retreated back behind the window outside.
            hunger.value = 0.0;
            hunger.stage = MooseHungerStage::BehindShutters;
            hunger.is_breached = false;
            hunger.breach_delay_remaining = None;
            hunger.huff_sound_timer = 0.0;

            // Close shutters and reset rotation
            for (mut s_tf, mut s_vis) in &mut q_shutters {
                s_tf.rotation = Quat::IDENTITY;
                *s_vis = Visibility::Inherited;
            }

            // Restore window
            for (mut w_vis, mut pane) in &mut q_window {
                pane.is_broken = false;
                *w_vis = Visibility::Inherited;
            }

            // Reset Moose root transform to exact default
            moose_tf.translation = Vec3::new(-6.5, 1.8, -17.0);
            moose_tf.rotation = Quat::IDENTITY;
            moose_tf.scale = Vec3::ONE;

            // Reset Moose Head transform to exact default
            for mut h_tf in &mut q_head {
                h_tf.translation = Vec3::new(0.0, 1.1, 1.6);
                h_tf.rotation = Quat::IDENTITY;
                h_tf.scale = Vec3::ONE;
            }

            commands.entity(moose_entity).remove::<MooseRampageTracker>();
            continue;
        }

        let target = tracker.waypoints[tracker.current_waypoint_index];
        let flat_diff = Vec3::new(target.x - moose_tf.translation.x, 0.0, target.z - moose_tf.translation.z);
        let dist = flat_diff.length();

        if dist < 0.6 {
            tracker.current_waypoint_index += 1;
            tracker.is_bucking = false;
            tracker.current_buck_pitch = 0.0;
            tracker.speed = tracker.base_speed;
            if tracker.current_waypoint_index >= tracker.waypoints.len() {
                // Moose reached the final retreat waypoint behind the window!
                // Reset immediately so there is no 1-frame orientation lag.
                hunger.value = 0.0;
                hunger.stage = MooseHungerStage::BehindShutters;
                hunger.is_breached = false;
                hunger.breach_delay_remaining = None;
                hunger.huff_sound_timer = 0.0;

                for (mut s_tf, mut s_vis) in &mut q_shutters {
                    s_tf.rotation = Quat::IDENTITY;
                    *s_vis = Visibility::Inherited;
                }

                for (mut w_vis, mut pane) in &mut q_window {
                    pane.is_broken = false;
                    *w_vis = Visibility::Inherited;
                }

                moose_tf.translation = Vec3::new(-6.5, 1.8, -17.0);
                moose_tf.rotation = Quat::IDENTITY;
                moose_tf.scale = Vec3::ONE;

                for mut h_tf in &mut q_head {
                    h_tf.translation = Vec3::new(0.0, 1.1, 1.6);
                    h_tf.rotation = Quat::IDENTITY;
                    h_tf.scale = Vec3::ONE;
                }

                commands.entity(moose_entity).remove::<MooseRampageTracker>();
                continue;
            }
        } else {
            // Update "bucking" animation cycle at semi-randomized intervals inside workshop
            let can_buck = tracker.current_waypoint_index >= 1 && tracker.current_waypoint_index < tracker.waypoints.len() - 1;

            if can_buck {
                if !tracker.is_bucking {
                    tracker.buck_timer += dt;
                    if tracker.buck_timer >= tracker.next_buck_interval {
                        tracker.is_bucking = true;
                        tracker.buck_progress = 0.0;
                        if tracker.buck_duration <= 0.0 {
                            tracker.buck_duration = 0.85;
                        }
                    }
                } else {
                    tracker.buck_progress += dt / tracker.buck_duration.max(0.01);
                    if tracker.buck_progress >= 1.0 {
                        tracker.is_bucking = false;
                        tracker.buck_timer = 0.0;
                        tracker.next_buck_interval = 1.0 + (rand::random::<f32>() * 1.5);
                        tracker.speed = tracker.base_speed;
                        tracker.current_buck_pitch = 0.0;
                    } else {
                        let p = tracker.buck_progress;
                        if p <= 0.40 {
                            // Phase 1: Rotate upward from back feet, slow down slightly
                            let u = p / 0.40;
                            let pitch_norm = (u * std::f32::consts::FRAC_PI_2).sin();
                            tracker.current_buck_pitch = 0.60 * pitch_norm;
                            tracker.speed = tracker.base_speed * (1.0 - 0.45 * pitch_norm);
                        } else if p <= 0.80 {
                            // Phase 2: Rotate back to flat, speed up again (charging slam)
                            let v = (p - 0.40) / 0.40;
                            let pitch_norm = (v * std::f32::consts::FRAC_PI_2).cos();
                            tracker.current_buck_pitch = 0.60 * pitch_norm;
                            let surge_norm = (v * std::f32::consts::PI).sin();
                            tracker.speed = tracker.base_speed * (0.55 + 0.80 * surge_norm);
                        } else {
                            // Phase 3: Settle back to base speed
                            let w = (p - 0.80) / 0.20;
                            tracker.current_buck_pitch = 0.0;
                            tracker.speed = tracker.base_speed * (1.35 * (1.0 - w) + 1.0 * w);
                        }
                    }
                }
            } else {
                tracker.is_bucking = false;
                tracker.current_buck_pitch = 0.0;
                tracker.speed = tracker.base_speed;
            }

            let dir = flat_diff / dist;
            moose_tf.translation.x += dir.x * tracker.speed * dt;
            moose_tf.translation.z += dir.z * tracker.speed * dt;

            // Lift torso upwards proportionally so Moose pivots cleanly around rear feet on the floor
            let buck_height_lift = 0.95 * tracker.current_buck_pitch.sin() - 1.4 * (1.0 - tracker.current_buck_pitch.cos());
            moose_tf.translation.y = target.y + buck_height_lift.max(0.0);

            // Face forward along movement direction, then pitch upward around local X
            if dir.length_squared() > 0.001 {
                let yaw = Quat::from_rotation_y(dir.x.atan2(dir.z));
                // -current_buck_pitch rotates head & front upwards from back feet
                moose_tf.rotation = yaw * Quat::from_rotation_x(-tracker.current_buck_pitch);
            }

            // Extreme Impulses: Launch any physics objects nearby into the air with chaotic force
            for (obj_tf, mut impulse, mut opt_lattice) in &mut q_dynamic_bodies {
                let obj_dist = obj_tf.translation.distance(moose_tf.translation);
                if obj_dist < 2.5 {
                    let away = (obj_tf.translation - moose_tf.translation).normalize_or_zero();
                    // High exaggerated impulse (5x slapstick force)
                    impulse.impulse += away * 15.0 + Vec3::Y * 8.0;
                    impulse.torque_impulse += Vec3::new(5.0, 8.0, 3.0);
                    if let Some(ref mut lattice) = opt_lattice {
                        lattice.apply_directional_squash(away, 0.45);
                    }
                }
            }

            // Launch player in a random upwards direction on contact with rampaging Moose
            for (player_tf, mut movement) in &mut q_player {
                let p_dist = player_tf.translation.distance(moose_tf.translation);
                if p_dist < 2.2 && movement.velocity.y < 8.0 {
                    let angle = rand::random::<f32>() * std::f32::consts::TAU;
                    let h_speed = 6.0 + rand::random::<f32>() * 6.0;
                    let v_speed = 10.0 + rand::random::<f32>() * 6.0;

                    let launch_vel = Vec3::new(
                        angle.cos() * h_speed,
                        v_speed,
                        angle.sin() * h_speed,
                    );

                    movement.velocity = launch_vel;
                    movement.is_grounded = false;
                    movement.current_max_speed = movement.current_max_speed.max(h_speed);
                }
            }
        }
    }
}
