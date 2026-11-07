use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    core::WorkshopQuota,
    moose::{MooseBreachMessage, MooseHunger, MooseHungerStage},
    player::ElfMovement,
    wrapping::{Present, PresentDeliveredMessage},
    workshop::room::{BreakableWindowPane, PresentDeliveryChute},
    moose::toy_model::{MrMoose, WindowShutters},
};

#[derive(Component)]
pub struct MooseRampageTracker {
    pub current_waypoint_index: usize,
    pub speed: f32,
    pub waypoints: Vec<Vec3>,
}

impl Default for MooseRampageTracker {
    fn default() -> Self {
        Self {
            current_waypoint_index: 0,
            speed: 8.5,
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
            commands.entity(moose_entity).insert(MooseRampageTracker::default());
        }

        // Screen knockback impulse on player
        for mut movement in &mut q_player {
            movement.velocity.y = 6.0;
        }
    }
}

/// Follows the deterministic waypoint path through the workshop, faces forward along motion,
/// applies extreme impulse forces to objects, and retreats back behind window when complete.
pub fn moose_rampage_path_and_physics_impulse_system(
    time: Option<Res<Time>>,
    hunger: Option<ResMut<MooseHunger>>,
    mut q_moose: Query<(Entity, &mut Transform, &mut MooseRampageTracker), With<MrMoose>>,
    mut q_dynamic_bodies: Query<(&Transform, &mut ExternalImpulse), (Without<MrMoose>, Without<ElfMovement>)>,
    mut q_player: Query<(&Transform, &mut ElfMovement), (Without<MrMoose>, Without<ExternalImpulse>)>,
    mut q_shutters: Query<&mut Visibility, (With<WindowShutters>, Without<MrMoose>)>,
    mut q_window: Query<(&mut Visibility, &mut BreakableWindowPane), (Without<MrMoose>, Without<WindowShutters>)>,
    mut commands: Commands,
) {
    let (Some(time), Some(mut hunger)) = (time, hunger) else {
        return;
    };

    if hunger.stage != MooseHungerStage::BreachedRampage {
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

            // Close shutters
            for mut s_vis in &mut q_shutters {
                *s_vis = Visibility::Inherited;
            }

            // Restore window
            for (mut w_vis, mut pane) in &mut q_window {
                pane.is_broken = false;
                *w_vis = Visibility::Inherited;
            }

            // Face moose back toward the window (+Z)
            moose_tf.rotation = Quat::IDENTITY;
            moose_tf.translation = Vec3::new(-6.5, 1.8, -17.0);

            commands.entity(moose_entity).remove::<MooseRampageTracker>();
            continue;
        }

        let target = tracker.waypoints[tracker.current_waypoint_index];
        let diff = target - moose_tf.translation;
        let dist = diff.length();

        if dist < 0.6 {
            tracker.current_waypoint_index += 1;
        } else {
            let dir = diff / dist;
            moose_tf.translation += dir * tracker.speed * dt;

            // Face forward: Since Moose's head/antlers are at +Z, align +Z with movement direction
            let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            if flat_dir.length_squared() > 0.001 {
                moose_tf.rotation = Quat::from_rotation_y(flat_dir.x.atan2(flat_dir.z));
            }

            // Extreme Impulses: Launch any physics objects nearby into the air with chaotic force
            for (obj_tf, mut impulse) in &mut q_dynamic_bodies {
                let obj_dist = obj_tf.translation.distance(moose_tf.translation);
                if obj_dist < 2.5 {
                    let away = (obj_tf.translation - moose_tf.translation).normalize_or_zero();
                    // High exaggerated impulse (5x slapstick force)
                    impulse.impulse += away * 15.0 + Vec3::Y * 8.0;
                    impulse.torque_impulse += Vec3::new(5.0, 8.0, 3.0);
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
