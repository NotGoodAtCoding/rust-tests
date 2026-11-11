use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    core::WorkshopQuota,
    moose::{MooseBreachMessage, MooseHunger, MooseHungerStage},
    player::ElfMovement,
    wrapping::{Present, PresentDeliveredMessage},
    workshop::room::{BreakableWindowPane, PresentDeliveryChute},
    moose::toy_model::{MrMoose, MooseHead, WindowShutters},
    physics_sandbox::DynamicWaypointTarget,
};

pub use crate::elf_game::moose::toy_model::MoosePart;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct MooseNonCollision {
    pub timer: f32,
}

/// Candidate ground floor waypoints reorganized so they do not intersect
/// the new back wall subrooms (Pantry, Unused Room 1, Unused Room 2 at Z >= 9.0).
pub const MOOSE_GROUND_WAYPOINTS: &[Vec3] = &[
    Vec3::new(0.0, 1.4, -4.0),   // Central workbench
    Vec3::new(5.0, 1.4, 2.0),    // East conveyor belt
    Vec3::new(-4.0, 1.4, 4.0),   // West storage area
    Vec3::new(3.0, 1.4, 5.5),    // South-east open floor
    Vec3::new(-3.0, 1.4, 5.5),   // South-west open floor
    Vec3::new(-6.0, 1.4, -2.0),  // West open floor
    Vec3::new(7.0, 1.4, -3.0),   // East corridor
    Vec3::new(0.0, 1.4, 1.0),    // Workshop center floor
    Vec3::new(9.5, 1.4, 4.0),    // Base of mezzanine stairs
];

/// Candidate waypoints atop the elevated mezzanine (Floor at Y = 3.0, Moose at Y = 4.4).
pub const MOOSE_MEZZANINE_WAYPOINTS: &[Vec3] = &[
    Vec3::new(-8.0, 4.4, 11.0),  // Mezzanine west
    Vec3::new(-4.0, 4.4, 11.0),  // Mezzanine hat rack & mirror
    Vec3::new(0.0, 4.4, 11.0),   // Mezzanine center
    Vec3::new(4.5, 4.4, 11.0),   // Mezzanine east
    Vec3::new(8.5, 4.4, 10.5),   // Mezzanine top landing
];

/// Combined pool of all available rampage waypoints across the ground floor and mezzanine.
pub const MOOSE_ALL_WAYPOINTS: &[Vec3] = &[
    // Ground floor (reorganized away from back wall subrooms)
    Vec3::new(0.0, 1.4, -4.0),
    Vec3::new(5.0, 1.4, 2.0),
    Vec3::new(-4.0, 1.4, 4.0),
    Vec3::new(3.0, 1.4, 5.5),
    Vec3::new(-3.0, 1.4, 5.5),
    Vec3::new(-6.0, 1.4, -2.0),
    Vec3::new(7.0, 1.4, -3.0),
    Vec3::new(0.0, 1.4, 1.0),
    Vec3::new(9.5, 1.4, 4.0),
    // Top mezzanine
    Vec3::new(-8.0, 4.4, 11.0),
    Vec3::new(-4.0, 4.4, 11.0),
    Vec3::new(0.0, 4.4, 11.0),
    Vec3::new(4.5, 4.4, 11.0),
    Vec3::new(8.5, 4.4, 10.5),
];

#[derive(Component, Debug, Clone)]
pub struct MooseRampageTracker {
    pub current_waypoint_index: usize,
    pub speed: f32,
    pub base_speed: f32,
    pub waypoints: Vec<Vec3>,
    /// The 5 to 7 randomized waypoints chosen for this rampage
    pub chosen_waypoints: Vec<Vec3>,
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
    // Dynamic Player Targeting
    pub dynamic_target: Option<DynamicWaypointTarget>,
    pub is_targeting_player: bool,
    pub target_player_timer: f32,
    pub player_target_cooldown: f32,
}

impl Default for MooseRampageTracker {
    fn default() -> Self {
        Self {
            current_waypoint_index: 0,
            speed: 8.5,
            base_speed: 8.5,
            waypoints: vec![
                Vec3::new(-2.0, 1.4, -10.0), // Leap into workshop through stable door
                Vec3::new(0.0, 1.4, -4.0),   // Slam into central workbench
                Vec3::new(5.0, 1.4, 2.0),    // Charge across conveyor belt
                Vec3::new(-4.0, 1.4, 4.0),   // Trample west area
                Vec3::new(3.0, 1.4, 5.5),    // Trample south-east open floor (reorganized away from back rooms)
                Vec3::new(-2.0, 1.4, -4.0),  // Charge back towards stable door
                Vec3::new(-2.0, 1.4, -10.0), // Leap back out stable door
                Vec3::new(-2.0, 1.8, -17.0), // Retreat to resting area outside stable door
            ],
            chosen_waypoints: vec![
                Vec3::new(0.0, 1.4, -4.0),
                Vec3::new(5.0, 1.4, 2.0),
                Vec3::new(-4.0, 1.4, 4.0),
                Vec3::new(3.0, 1.4, 5.5),
                Vec3::new(0.0, 4.4, 11.0),
            ],
            buck_timer: 0.0,
            next_buck_interval: 1.2,
            buck_progress: 0.0,
            buck_duration: 0.85,
            is_bucking: false,
            current_buck_pitch: 0.0,
            dynamic_target: None,
            is_targeting_player: false,
            target_player_timer: 0.0,
            player_target_cooldown: 0.0,
        }
    }
}

impl MooseRampageTracker {
    /// Chooses 5 to 7 randomized waypoints across the workshop and top mezzanine.
    pub fn choose_rampage_waypoints(count: usize) -> Vec<Vec3> {
        let count = count.clamp(5, 7);
        let mut pool = MOOSE_ALL_WAYPOINTS.to_vec();
        for i in (1..pool.len()).rev() {
            let j = (rand::random::<u32>() as usize) % (i + 1);
            pool.swap(i, j);
        }
        pool.into_iter().take(count).collect()
    }

    /// Generates a complete randomized rampage path with 5 to 7 chosen waypoints.
    pub fn generate_rampage_path(moose_x: f32) -> (Vec<Vec3>, Vec<Vec3>) {
        let count = 5 + ((rand::random::<u32>() % 3) as usize); // 5, 6, or 7
        let chosen = Self::choose_rampage_waypoints(count);
        let mut path = Vec::with_capacity(chosen.len() + 4);
        path.push(Vec3::new(moose_x, 1.4, -10.0)); // Leap into workshop
        for wp in &chosen {
            path.push(*wp);
        }
        path.push(Vec3::new(moose_x, 1.4, -4.0));  // Charge back towards stable door
        path.push(Vec3::new(moose_x, 1.4, -10.0)); // Leap back out stable door
        path.push(Vec3::new(moose_x, 1.8, -17.0)); // Retreat to resting area outside
        (path, chosen)
    }

    /// Constructs a new tracker initialized with a randomized 5-7 waypoint path.
    pub fn new_randomized(moose_x: f32) -> Self {
        let (path, chosen) = Self::generate_rampage_path(moose_x);
        Self {
            current_waypoint_index: 0,
            speed: 8.5,
            base_speed: 8.5,
            waypoints: path,
            chosen_waypoints: chosen,
            ..default()
        }
    }

    /// Forces the rampaging moose to target the player if not on cooldown.
    pub fn target_player(&mut self) {
        if self.player_target_cooldown <= 0.0 {
            self.is_targeting_player = true;
            self.dynamic_target = Some(DynamicWaypointTarget::Player);
            self.target_player_timer = 0.0;
        }
    }

    /// Clears any active player targeting, returning to waypoints.
    pub fn clear_player_target(&mut self) {
        self.is_targeting_player = false;
        self.dynamic_target = None;
        self.target_player_timer = 0.0;
    }
}

/// Detects presents dropped inside the Delivery Chute and increments workshop quotas
pub fn monitor_present_delivery(
    mut q_presents: Query<(Entity, &Transform, &Present)>,
    q_chute: Query<&Transform, With<PresentDeliveryChute>>,
    mut quota: ResMut<WorkshopQuota>,
    mut delivery_messages: Option<MessageWriter<PresentDeliveredMessage>>,
    mut confetti_events: Option<MessageWriter<crate::elf_game::workshop::particles::SpawnConfettiEvent>>,
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
            if let Some(ref mut writer) = confetti_events {
                writer.write(crate::elf_game::workshop::particles::SpawnConfettiEvent {
                    position: chute_tf.translation + Vec3::Y * 0.5,
                    count: 24,
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
    mut glass_events: Option<MessageWriter<crate::elf_game::workshop::particles::SpawnGlassShardsEvent>>,
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

        if let Some(ref mut writer) = glass_events {
            writer.write(crate::elf_game::workshop::particles::SpawnGlassShardsEvent {
                position: Vec3::new(-6.5, 1.8, -13.0),
                count: 32,
            });
        }

        // Add rampage tracker to Mr. Moose and leap into opening
        for (moose_entity, mut moose_tf) in &mut q_moose {
            let moose_x = moose_tf.translation.x;
            moose_tf.translation = Vec3::new(moose_x, 1.4, -10.0);
            moose_tf.rotation = Quat::IDENTITY;
            let tracker = MooseRampageTracker::new_randomized(moose_x);
            commands.entity(moose_entity).insert(tracker);
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
        (
            Entity,
            &Transform,
            &mut ElfMovement,
            Option<&mut CollisionGroups>,
            Option<&mut KinematicCharacterController>,
            Option<&mut MooseNonCollision>,
        ),
        (
            Without<MrMoose>,
            Without<ExternalImpulse>,
            Without<WindowShutters>,
            Without<MooseHead>,
        ),
    >,
    q_moose_parts: Query<
        (Entity, &Transform, Option<&GlobalTransform>, Option<&ChildOf>),
        (
            With<MoosePart>,
            Without<MrMoose>,
            Without<MooseHead>,
            Without<WindowShutters>,
            Without<ElfMovement>,
            Without<ExternalImpulse>,
            Without<BreakableWindowPane>,
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
        for (moose_entity, mut moose_tf, tracker) in &mut q_moose {
            let retreat_pos = tracker.waypoints.last().copied().unwrap_or(Vec3::new(-2.0, 1.8, -17.0));
            moose_tf.translation = retreat_pos;
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

            // Reset Moose root transform to retreat destination
            let retreat_pos = tracker.waypoints.last().copied().unwrap_or(moose_tf.translation);
            moose_tf.translation = retreat_pos;
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

        // Update player targeting cooldown
        if tracker.player_target_cooldown > 0.0 {
            tracker.player_target_cooldown = (tracker.player_target_cooldown - dt).max(0.0);
        }

        let opt_player_pos = q_player.iter().next().map(|(_, tf, _, _, _, _)| tf.translation);
        if let Some(player_pos) = opt_player_pos {
            let flat_player_diff = Vec3::new(
                player_pos.x - moose_tf.translation.x,
                0.0,
                player_pos.z - moose_tf.translation.z,
            );
            let player_dist = flat_player_diff.length();

            // Mezzanine floor is elevated at Y = 3.0.
            // If the player is on the mezzanine (player Y >= 2.5 while moose is on ground < 2.5),
            // or vertical difference is large (> 1.5m), the moose CANNOT target the player.
            let player_on_mezzanine = player_pos.y >= 2.5 && moose_tf.translation.y < 2.5;
            let vertical_diff_too_large = (player_pos.y - moose_tf.translation.y).abs() > 1.5;
            let can_target_vertically = !player_on_mezzanine && !vertical_diff_too_large;

            // When the moose is rampaging, if the player gets within 5m of the moose, set target location to the player.
            if !tracker.is_targeting_player && tracker.player_target_cooldown <= 0.0 {
                if player_dist <= 5.0 && can_target_vertically {
                    tracker.is_targeting_player = true;
                    tracker.dynamic_target = Some(DynamicWaypointTarget::Player);
                    tracker.target_player_timer = 0.0;
                }
            }

            // As long as the player is within 6m of the moose once triggered, or 5 seconds have passed since the moose targeted the player,
            // keep updating the target location of the moose to the Player.
            // If the player escapes to the mezzanine above, clear targeting immediately.
            if tracker.is_targeting_player {
                if !can_target_vertically {
                    tracker.is_targeting_player = false;
                    tracker.dynamic_target = None;
                    tracker.target_player_timer = 0.0;
                } else {
                    tracker.target_player_timer += dt;
                    if player_dist <= 6.0 || tracker.target_player_timer < 5.0 {
                        tracker.dynamic_target = Some(DynamicWaypointTarget::Player);
                    } else {
                        tracker.is_targeting_player = false;
                        tracker.dynamic_target = None;
                        tracker.target_player_timer = 0.0;
                    }
                }
            }
        } else {
            tracker.is_targeting_player = false;
            tracker.dynamic_target = None;
        }

        let (target, is_player_target) = if tracker.is_targeting_player {
            if let Some(p_pos) = opt_player_pos {
                (p_pos, true)
            } else {
                (tracker.waypoints[tracker.current_waypoint_index], false)
            }
        } else {
            (tracker.waypoints[tracker.current_waypoint_index], false)
        };

        let flat_diff = Vec3::new(target.x - moose_tf.translation.x, 0.0, target.z - moose_tf.translation.z);
        let dist = flat_diff.length();

        if !is_player_target && dist < 0.6 {
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

                let retreat_pos = tracker.waypoints.last().copied().unwrap_or(moose_tf.translation);
                moose_tf.translation = retreat_pos;
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
            let can_buck = tracker.is_targeting_player || (tracker.current_waypoint_index >= 1 && tracker.current_waypoint_index < tracker.waypoints.len() - 1);

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

            let dir = if dist > 0.001 { flat_diff / dist } else { Vec3::ZERO };
            moose_tf.translation.x += dir.x * tracker.speed * dt;
            moose_tf.translation.z += dir.z * tracker.speed * dt;

            // Lift torso upwards proportionally so Moose pivots cleanly around rear feet on the floor
            let buck_height_lift = 0.95 * tracker.current_buck_pitch.sin() - 1.4 * (1.0 - tracker.current_buck_pitch.cos());
            let base_y = if is_player_target { 1.4 } else { target.y };
            moose_tf.translation.y = base_y + buck_height_lift.max(0.0);

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
            // All parts of the Moose should launch the player, and after contact have a short time of non-collision
            let mut part_positions: Vec<Vec3> = Vec::with_capacity(16);

            for h_tf in &q_head {
                part_positions.push(moose_tf.transform_point(h_tf.translation));
            }

            for (_part_e, part_tf, opt_gt, opt_parent) in &q_moose_parts {
                let pos = if let Some(gt) = opt_gt {
                    let t = gt.translation();
                    if t != Vec3::ZERO && t != part_tf.translation {
                        t
                    } else {
                        moose_tf.transform_point(part_tf.translation)
                    }
                } else if opt_parent.is_some() {
                    moose_tf.transform_point(part_tf.translation)
                } else {
                    part_tf.translation
                };
                part_positions.push(pos);
            }

            for (player_entity, player_tf, mut movement, opt_groups, opt_kcc, mut opt_non_coll) in &mut q_player {
                // If non-collision is active, tick it down and skip contact check
                if movement.moose_non_collision_timer > 0.0 {
                    movement.moose_non_collision_timer = (movement.moose_non_collision_timer - dt).max(0.0);
                    if let Some(ref mut nc) = opt_non_coll {
                        nc.timer = movement.moose_non_collision_timer;
                    }
                    if movement.moose_non_collision_timer <= 0.0 {
                        commands.entity(player_entity).remove::<MooseNonCollision>();
                        if let Some(mut groups) = opt_groups {
                            *groups = CollisionGroups::new(
                                crate::elf_game::player::hands::GROUP_PLAYER,
                                !crate::elf_game::player::hands::GROUP_HELD_ITEM,
                            );
                        }
                        if let Some(mut kcc) = opt_kcc {
                            kcc.filter_groups = Some(CollisionGroups::new(
                                crate::elf_game::player::hands::GROUP_PLAYER,
                                !crate::elf_game::player::hands::GROUP_HELD_ITEM,
                            ));
                        }
                    }
                    continue;
                }

                // Check distance to ANY part of the moose:
                // 1. Torso contact (broad 1.6m flat / 1.8m Y reach)
                let flat_torso_dist = Vec3::new(
                    player_tf.translation.x - moose_tf.translation.x,
                    0.0,
                    player_tf.translation.z - moose_tf.translation.z,
                ).length();
                let y_torso_diff = (player_tf.translation.y - moose_tf.translation.y).abs();
                let mut contacted = flat_torso_dist < 1.6 && y_torso_diff < 1.8;

                // 2. Anatomical parts contact (head, antlers, snout, hooves, flanks at 0.75m radius)
                if !contacted {
                    for part_pos in &part_positions {
                        if player_tf.translation.distance(*part_pos) < 0.75 {
                            contacted = true;
                            break;
                        }
                    }
                }

                if contacted && movement.velocity.y < 8.0 {
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

                    // Grant a short time (1.0s) of non-collision with the moose after contact
                    movement.moose_non_collision_timer = 1.0;
                    commands.entity(player_entity).insert(MooseNonCollision { timer: 1.0 });

                    if let Some(mut groups) = opt_groups {
                        *groups = CollisionGroups::new(
                            crate::elf_game::player::hands::GROUP_PLAYER,
                            !(crate::elf_game::player::hands::GROUP_HELD_ITEM | crate::elf_game::player::hands::GROUP_MOOSE),
                        );
                    }
                    if let Some(mut kcc) = opt_kcc {
                        kcc.filter_groups = Some(CollisionGroups::new(
                            crate::elf_game::player::hands::GROUP_PLAYER,
                            !(crate::elf_game::player::hands::GROUP_HELD_ITEM | crate::elf_game::player::hands::GROUP_MOOSE),
                        ));
                    }

                    // Once the moose contacts the player, the moose cannot target the player again for 3 seconds.
                    tracker.is_targeting_player = false;
                    tracker.dynamic_target = None;
                    tracker.target_player_timer = 0.0;
                    tracker.player_target_cooldown = 3.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf_game::{
        core::GameState,
        player::hands::{GROUP_HELD_ITEM, GROUP_MOOSE, GROUP_PLAYER},
        ui::MenuState,
    };

    #[test]
    fn test_moose_rampage_retreat_and_reset() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);
        app.insert_resource(MooseHunger {
            value: 60.0,
            stage: MooseHungerStage::BreachedRampage,
            is_breached: true,
            ..default()
        });
        app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

        // Moose retreating along last leg towards (-6.5, 1.8, -17.0), currently at (-6.5, 1.4, -16.5)
        // facing 180 degrees backward (away from the window, into -Z)
        let moose_entity = app.world_mut().spawn((
            MrMoose,
            MooseRampageTracker {
                current_waypoint_index: 1,
                speed: 8.5,
                waypoints: vec![
                    Vec3::new(-6.5, 1.4, -10.0),
                    Vec3::new(-6.5, 1.8, -17.0), // Final retreat destination
                ],
                ..default()
            },
            Transform::from_xyz(-6.5, 1.8, -16.6).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        )).id();

        // Moose Head stretched forward from peeking / breach
        let head_entity = app.world_mut().spawn((
            MooseHead,
            Transform::from_xyz(0.0, 1.1, 3.6).with_rotation(Quat::from_rotation_x(0.2)),
        )).id();

        // Shutters blown wide open
        let shutter_entity = app.world_mut().spawn((
            WindowShutters,
            Transform::from_xyz(-7.4, 2.9, -13.0).with_rotation(Quat::from_rotation_y(1.95)),
            Visibility::Hidden,
        )).id();

        // Broken window pane
        let window_entity = app.world_mut().spawn((
            BreakableWindowPane { is_broken: true },
            Transform::from_xyz(-6.5, 2.9, -13.0),
            Visibility::Hidden,
        )).id();

        // Update app: distance to final waypoint is 0.509m < 0.6m -> reaches destination!
        app.update();

        let hunger = app.world().resource::<MooseHunger>();
        assert_eq!(hunger.stage, MooseHungerStage::BehindShutters);
        assert_eq!(hunger.value, 0.0);
        assert!(!hunger.is_breached);
        assert!(app.world().get::<MooseRampageTracker>(moose_entity).is_none(), "Tracker should be removed on retreat");

        // 1. Assert Mr. Moose root transform snapped to default position and Quat::IDENTITY (facing +Z toward window)
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();
        assert_eq!(
            moose_tf.translation,
            Vec3::new(-6.5, 1.8, -17.0),
            "Moose translation must reset to default snowy hill position (-6.5, 1.8, -17.0)"
        );
        assert_eq!(
            moose_tf.rotation,
            Quat::IDENTITY,
            "Moose orientation must reset to Quat::IDENTITY facing +Z toward window"
        );

        // 2. Assert Moose Head transform reset to default (0.0, 1.1, 1.6) and Quat::IDENTITY
        let head_tf = app.world().get::<Transform>(head_entity).unwrap();
        assert_eq!(
            head_tf.translation,
            Vec3::new(0.0, 1.1, 1.6),
            "Moose head translation must reset to default (0.0, 1.1, 1.6)"
        );
        assert_eq!(
            head_tf.rotation,
            Quat::IDENTITY,
            "Moose head rotation must reset to Quat::IDENTITY"
        );

        // 3. Assert shutters reset to Quat::IDENTITY and Visibility::Inherited
        let shutter_tf = app.world().get::<Transform>(shutter_entity).unwrap();
        let shutter_vis = app.world().get::<Visibility>(shutter_entity).unwrap();
        assert_eq!(shutter_tf.rotation, Quat::IDENTITY, "Shutters rotation must reset to Quat::IDENTITY (shut)");
        assert_eq!(*shutter_vis, Visibility::Inherited, "Shutters visibility must be Inherited");

        // 4. Assert breakable window pane is restored
        let window_pane = app.world().get::<BreakableWindowPane>(window_entity).unwrap();
        let window_vis = app.world().get::<Visibility>(window_entity).unwrap();
        assert!(!window_pane.is_broken, "Window pane must be repaired");
        assert_eq!(*window_vis, Visibility::Inherited, "Window pane visibility must be Inherited");
    }

    #[test]
    fn test_player_launched_on_contact_with_rampaging_moose() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(0.016));
        app.insert_resource(time);
        app.insert_resource(MooseHunger {
            value: 60.0,
            stage: MooseHungerStage::BreachedRampage,
            is_breached: true,
            ..default()
        });
        app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

        // Moose rampaging at (0.0, 1.4, -4.0) towards next waypoint (5.0, 1.4, 3.0)
        app.world_mut().spawn((
            MrMoose,
            MooseRampageTracker {
                current_waypoint_index: 0,
                speed: 8.5,
                waypoints: vec![Vec3::new(5.0, 1.4, 3.0)],
                ..default()
            },
            Transform::from_xyz(0.0, 1.4, -4.0),
        ));

        // Player standing within contact distance (0.5m from Moose)
        let player_entity = app.world_mut().spawn((
            ElfMovement {
                velocity: Vec3::ZERO,
                is_grounded: true,
                ..default()
            },
            Transform::from_xyz(0.5, 1.2, -4.0),
        )).id();

        app.update();

        let movement = app.world().get::<ElfMovement>(player_entity).unwrap();
        assert!(
            movement.velocity.y >= 10.0,
            "Player should be launched upwards with high vertical speed (>= 10.0), got {}",
            movement.velocity.y
        );
        assert!(!movement.is_grounded, "Player must become airborne upon being launched");
        let h_speed = Vec2::new(movement.velocity.x, movement.velocity.z).length();
        assert!(
            h_speed >= 5.0,
            "Player should have strong horizontal momentum, got {}",
            h_speed
        );
    }

    #[test]
    fn test_moose_rampage_bucking_animation() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);

        app.insert_resource(MooseHunger {
            value: 60.0,
            stage: MooseHungerStage::BreachedRampage,
            is_breached: true,
            ..default()
        });
        app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

        // Spawn Moose inside workshop heading towards waypoint 2 (currently at (1.0, 1.4, -2.0))
        let moose_entity = app.world_mut().spawn((
            MrMoose,
            MooseRampageTracker {
                current_waypoint_index: 2,
                speed: 8.5,
                base_speed: 8.5,
                waypoints: vec![
                    Vec3::new(-6.5, 1.4, -10.0),
                    Vec3::new(0.0, 1.4, -4.0),
                    Vec3::new(5.0, 1.4, 3.0),
                    Vec3::new(-6.5, 1.8, -17.0),
                ],
                buck_timer: 1.15,
                next_buck_interval: 1.20,
                buck_duration: 0.80,
                is_bucking: false,
                current_buck_pitch: 0.0,
                ..default()
            },
            Transform::from_xyz(1.0, 1.4, -2.0),
        )).id();

        // 1. Step 1: Advance by 0.1s -> buck_timer exceeds next_buck_interval, bucking initiates!
        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(100));
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(tracker.is_bucking, "Moose must start bucking when buck timer reaches interval");

        // 2. Step into Phase 1 (Rearing upward from back feet, slowing down slightly)
        // Advance progress to ~0.30 of buck_duration (0.80s * 0.30 = ~0.24s)
        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(200));
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();

        assert!(
            tracker.current_buck_pitch > 0.30,
            "Moose should rotate upward from back feet (pitch > 0.30 rad / ~17 deg), got: {}",
            tracker.current_buck_pitch
        );
        assert!(
            tracker.speed < tracker.base_speed,
            "Moose should slow down slightly while rearing upward (speed {} < base {})",
            tracker.speed, tracker.base_speed
        );
        assert!(
            moose_tf.translation.y > 1.45,
            "Moose torso should elevate (Y > 1.45m) to pivot upward from rear feet, got: {}",
            moose_tf.translation.y
        );

        // 3. Step into Phase 2 (Rotating back to flat and speeding up again)
        // Advance progress to ~0.70 of buck_duration
        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(350));
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            tracker.speed > tracker.base_speed,
            "Moose should speed up again while rotating back to flat (speed {} > base {})",
            tracker.speed, tracker.base_speed
        );
        assert!(
            tracker.current_buck_pitch < 0.40,
            "Moose pitch should be rotating back down towards flat, got: {}",
            tracker.current_buck_pitch
        );

        // 4. Advance past buck completion (> 0.80s total)
        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(400));
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        let moose_tf = app.world().get::<Transform>(moose_entity).unwrap();

        assert!(!tracker.is_bucking, "Bucking cycle should finish and reset");
        assert_eq!(tracker.current_buck_pitch, 0.0, "Pitch must return to flat 0.0");
        assert!(
            (tracker.speed - tracker.base_speed).abs() < 0.01,
            "Speed must return to base speed after bucking recovery, got: {}",
            tracker.speed
        );
        assert!(
            (moose_tf.translation.y - 1.4).abs() < 0.05,
            "Moose height must return to flat ground level (1.4m), got: {}",
            moose_tf.translation.y
        );
    }

    #[test]
    fn test_pause_freezes_moose_rampage_movement_and_bucking_animation() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);

        app.insert_resource(MenuState::default());
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.insert_resource(MooseHunger {
            value: 60.0,
            stage: MooseHungerStage::BreachedRampage,
            is_breached: true,
            ..default()
        });
        app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

        let moose = app.world_mut().spawn((
            MrMoose,
            MooseRampageTracker::default(),
            Transform::from_xyz(-2.0, 1.4, -10.0),
        )).id();

        // Advance 10 frames while unpaused so the moose is actively moving and bucking
        for _ in 0..10 {
            app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(16));
            app.update();
        }

        let (pos_before_pause, rot_before_pause, tracker_before_pause) = {
            let tf = app.world().get::<Transform>(moose).unwrap();
            let tracker = app.world().get::<MooseRampageTracker>(moose).unwrap();
            (tf.translation, tf.rotation, tracker.clone())
        };

        assert!(
            tracker_before_pause.buck_timer > 0.0,
            "Moose should have accumulated bucking animation timer"
        );

        // Pause the game
        app.world_mut().resource_mut::<MenuState>().is_open = true;

        // Advance 30 frames with significant delta
        for _ in 0..30 {
            app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(33));
            app.update();
        }

        let (pos_after_pause, rot_after_pause, tracker_after_pause) = {
            let tf = app.world().get::<Transform>(moose).unwrap();
            let tracker = app.world().get::<MooseRampageTracker>(moose).unwrap();
            (tf.translation, tf.rotation, tracker.clone())
        };

        assert_eq!(
            pos_after_pause, pos_before_pause,
            "Moose rampage position must freeze in place while paused"
        );
        assert_eq!(
            rot_after_pause, rot_before_pause,
            "Moose rampage orientation must freeze in place while paused"
        );
        assert_eq!(
            tracker_after_pause.current_waypoint_index, tracker_before_pause.current_waypoint_index,
            "Moose waypoint index must remain frozen while paused"
        );
        assert_eq!(
            tracker_after_pause.buck_timer, tracker_before_pause.buck_timer,
            "Moose buck animation timer must remain frozen while paused"
        );
        assert_eq!(
            tracker_after_pause.buck_progress, tracker_before_pause.buck_progress,
            "Moose buck progress must remain frozen while paused"
        );
        assert_eq!(
            tracker_after_pause.current_buck_pitch, tracker_before_pause.current_buck_pitch,
            "Moose bucking animation pitch must remain frozen while paused"
        );
    }

    #[test]
    fn test_moose_rampage_dynamic_player_targeting_and_cooldown() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);

        app.insert_resource(MooseHunger {
            value: 60.0,
            stage: MooseHungerStage::BreachedRampage,
            is_breached: true,
            ..default()
        });
        app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

        // Spawn Moose at origin (0.0, 1.4, 0.0)
        let moose_entity = app.world_mut().spawn((
            MrMoose,
            MooseRampageTracker {
                current_waypoint_index: 1,
                speed: 8.5,
                base_speed: 8.5,
                waypoints: vec![
                    Vec3::new(0.0, 1.4, -100.0),
                    Vec3::new(0.0, 1.4, 100.0),
                    Vec3::new(0.0, 1.8, 500.0),
                ],
                ..default()
            },
            Transform::from_xyz(0.0, 1.4, 0.0),
        )).id();

        // Spawn Player far away at (10.0, 0.45, 0.0) (> 5m)
        let player_entity = app.world_mut().spawn((
            Transform::from_xyz(10.0, 0.45, 0.0),
            ElfMovement {
                velocity: Vec3::ZERO,
                base_speed: 6.0,
                ..default()
            },
        )).id();

        // 1. Player is > 5m away: Moose does NOT target player
        app.update();
        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(!tracker.is_targeting_player, "Moose must not target player when > 5m away");
        assert_eq!(tracker.dynamic_target, None);

        // 2. Player moves within 5m (4.5m from Moose)
        {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(4.5, 0.0, 0.0);
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(tracker.is_targeting_player, "Moose must target player when within 5m");
        assert_eq!(tracker.dynamic_target, Some(DynamicWaypointTarget::Player));

        // 3. Player moves to 5.8m (<= 6m): Moose keeps targeting player
        {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(5.8, 0.0, 0.0);
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(tracker.is_targeting_player, "Moose must keep targeting player while within 6m");

        // 4. Player runs away to 7.0m (> 6m), but only 1 second has elapsed (< 5s)
        for _ in 0..10 {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(7.0, 0.0, 0.0);
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(100));
            app.update();
        }

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            tracker.is_targeting_player,
            "Moose must keep targeting player for 5 seconds even if player steps beyond 6m"
        );

        // 5. Advance time by 4.5 more seconds (total > 5s, player still running away at 7.0m > 6m)
        for _ in 0..45 {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(7.0, 0.0, 0.0);
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(100));
            app.update();
        }

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            !tracker.is_targeting_player,
            "Moose must stop targeting player after 5s have elapsed and player is outside 6m"
        );
        assert_eq!(tracker.dynamic_target, None);

        // 6. Player comes back within 5m (4.0m): Moose targets player again
        {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(4.0, 0.0, 0.0);
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(tracker.is_targeting_player, "Moose must retarget player when entering 5m again");

        // 7. Contact: Player collides with Moose (< 1.6m)
        {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(0.8, 0.0, 0.0);
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        let player_mov = app.world().get::<ElfMovement>(player_entity).unwrap();

        assert!(player_mov.velocity.y >= 8.0, "Player must be launched on contact with Moose");
        assert!(!tracker.is_targeting_player, "Targeting must stop upon contact");
        assert_eq!(tracker.dynamic_target, None);
        assert!(
            tracker.player_target_cooldown >= 2.9,
            "Moose must enter 3-second cooldown upon contacting player, got {}",
            tracker.player_target_cooldown
        );

        // 8. While on cooldown, player being <= 5m (3.0m) does NOT trigger targeting
        for _ in 0..10 {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(3.0, 0.0, 0.0);
            let mut p_mov = app.world_mut().get_mut::<ElfMovement>(player_entity).unwrap();
            p_mov.velocity = Vec3::ZERO;
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(100)); // 1.0s elapsed, 2.0s cooldown remaining
            app.update();
        }

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            !tracker.is_targeting_player,
            "Moose must NOT target player while 3s cooldown is active (cooldown = {})",
            tracker.player_target_cooldown
        );

        // 9. Advance time past remaining cooldown (2.1s)
        for _ in 0..21 {
            let moose_pos = app.world().get::<Transform>(moose_entity).unwrap().translation;
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = moose_pos + Vec3::new(3.0, 0.0, 0.0);
            let mut p_mov = app.world_mut().get_mut::<ElfMovement>(player_entity).unwrap();
            p_mov.velocity = Vec3::ZERO;
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(100)); // Cooldown expires!
            app.update();
        }

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            tracker.is_targeting_player,
            "Moose should target player again once cooldown expires and player is within 5m"
        );
    }

    #[test]
    fn test_moose_rampage_randomized_waypoints_mezzanine_and_player_targeting() {
        // 1. Verify ground waypoints do not go through new back wall subrooms (Z >= 9.0)
        assert_eq!(
            MOOSE_ALL_WAYPOINTS.len(),
            MOOSE_GROUND_WAYPOINTS.len() + MOOSE_MEZZANINE_WAYPOINTS.len()
        );
        for wp in MOOSE_GROUND_WAYPOINTS {
            assert!(
                wp.z <= 6.5,
                "Ground waypoints must not enter back wall subrooms (Z >= 9.0), got Z = {}",
                wp.z
            );
            assert_eq!(wp.y, 1.4, "Ground waypoints must be at Moose ground height Y = 1.4");
        }

        // 2. Verify mezzanine waypoints are elevated at Y = 4.4 and in mezzanine bounds (Z >= 9.0)
        for wp in MOOSE_MEZZANINE_WAYPOINTS {
            assert_eq!(wp.y, 4.4, "Mezzanine waypoints must be elevated at Y = 4.4");
            assert!(wp.z >= 9.0, "Mezzanine waypoints must be on mezzanine (Z >= 9.0)");
        }

        // 3. Verify choose_rampage_waypoints chooses 5 to 7 waypoints
        for count in 5..=7 {
            let chosen = MooseRampageTracker::choose_rampage_waypoints(count);
            assert_eq!(chosen.len(), count, "Must choose requested count in 5..=7");
        }

        // 4. Verify randomized tracker path generation includes 5-7 chosen waypoints
        let randomized_tracker = MooseRampageTracker::new_randomized(-2.0);
        assert!(
            randomized_tracker.chosen_waypoints.len() >= 5 && randomized_tracker.chosen_waypoints.len() <= 7,
            "Randomized tracker must choose 5 to 7 waypoints, got {}",
            randomized_tracker.chosen_waypoints.len()
        );

        // 5. Verify Moose does NOT target player when player is on the mezzanine above (Y = 3.45m vs Moose Y = 1.4m)
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
        app.insert_resource(MooseHunger {
            value: 60.0,
            stage: MooseHungerStage::BreachedRampage,
            is_breached: true,
            ..default()
        });
        app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

        let moose_entity = app.world_mut().spawn((
            MrMoose,
            MooseRampageTracker {
                current_waypoint_index: 1,
                speed: 8.5,
                base_speed: 8.5,
                waypoints: vec![
                    Vec3::new(0.0, 1.4, -10.0),
                    Vec3::new(0.0, 1.4, 0.0),
                    Vec3::new(0.0, 1.8, -17.0),
                ],
                ..default()
            },
            Transform::from_xyz(0.0, 1.4, 0.0),
        )).id();

        // Spawn player on the mezzanine directly above Moose (X = 3.5, Y = 3.45, Z = 0.0, flat dist = 3.5m <= 5m)
        let player_entity = app.world_mut().spawn((
            Transform::from_xyz(3.5, 3.45, 0.0),
            ElfMovement {
                velocity: Vec3::ZERO,
                base_speed: 6.0,
                ..default()
            },
        )).id();

        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            !tracker.is_targeting_player,
            "Moose must NOT target player when player is elevated on the mezzanine above"
        );

        // If player drops down to main floor (Y = 0.45m), Moose targets player
        {
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = Vec3::new(3.5, 0.45, 0.0);
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            tracker.is_targeting_player,
            "Moose must target player once player is on ground floor within 5m"
        );

        // If player climbs back to mezzanine (Y = 3.45m), targeting is immediately cancelled
        {
            let mut p_tf = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            p_tf.translation = Vec3::new(3.5, 3.45, 0.0);
        }
        app.update();

        let tracker = app.world().get::<MooseRampageTracker>(moose_entity).unwrap();
        assert!(
            !tracker.is_targeting_player,
            "Moose targeting must immediately disengage when player escapes to mezzanine"
        );
    }

    #[test]
    fn test_moose_all_parts_launch_player_and_non_collision_window() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(0.016));
        app.insert_resource(time);
        app.insert_resource(MooseHunger {
            value: 60.0,
            stage: MooseHungerStage::BreachedRampage,
            is_breached: true,
            ..default()
        });
        app.add_systems(Update, moose_rampage_path_and_physics_impulse_system);

        // Spawn Moose at (0.0, 1.4, -4.0)
        let moose_tf = Transform::from_xyz(0.0, 1.4, -4.0);
        let moose_e = app.world_mut().spawn((
            MrMoose,
            MoosePart,
            MooseRampageTracker {
                current_waypoint_index: 0,
                speed: 8.5,
                waypoints: vec![Vec3::new(0.0, 1.4, 10.0)],
                ..default()
            },
            moose_tf,
        )).id();

        // 1. Antler part contact test:
        // Right antler is at offset (0.85, 1.8, 1.85) from Moose origin => world (0.85, 3.2, -2.15)
        let antler_offset = Vec3::new(0.85, 1.8, 1.85);
        let antler_world_pos = moose_tf.transform_point(antler_offset);
        app.world_mut().spawn((
            MoosePart,
            Transform::from_translation(antler_offset),
            ChildOf(moose_e),
        ));

        let player_e = app.world_mut().spawn((
            ElfMovement {
                velocity: Vec3::ZERO,
                is_grounded: true,
                ..default()
            },
            Transform::from_translation(antler_world_pos),
            CollisionGroups::new(GROUP_PLAYER, !GROUP_HELD_ITEM),
            KinematicCharacterController {
                filter_groups: Some(CollisionGroups::new(GROUP_PLAYER, !GROUP_HELD_ITEM)),
                ..default()
            },
        )).id();

        app.update();

        // Verify player is launched by antler contact
        {
            let mov = app.world().get::<ElfMovement>(player_e).unwrap();
            assert!(mov.velocity.y >= 10.0, "Antler must launch player upwards (>= 10.0), got {}", mov.velocity.y);
            assert!(!mov.is_grounded, "Player must become airborne on antler contact");
            let h_speed = Vec2::new(mov.velocity.x, mov.velocity.z).length();
            assert!(h_speed >= 5.0, "Player must have horizontal momentum from launch");

            // Verify short non-collision window is activated
            assert!(mov.moose_non_collision_timer > 0.8, "Non-collision timer must be initialized to ~1.0s, got {}", mov.moose_non_collision_timer);
            let nc = app.world().get::<MooseNonCollision>(player_e);
            assert!(nc.is_some(), "MooseNonCollision component must be attached to player");

            // Verify Rapier collision groups filter out GROUP_MOOSE during non-collision
            let groups = app.world().get::<CollisionGroups>(player_e).unwrap();
            assert_eq!(*groups, CollisionGroups::new(GROUP_PLAYER, !(GROUP_HELD_ITEM | GROUP_MOOSE)));
            let kcc = app.world().get::<KinematicCharacterController>(player_e).unwrap();
            assert_eq!(kcc.filter_groups, Some(CollisionGroups::new(GROUP_PLAYER, !(GROUP_HELD_ITEM | GROUP_MOOSE))));
        }

        // 2. Next frame: with non-collision active, reset velocity to zero and verify player is NOT re-launched
        {
            let mut mov = app.world_mut().get_mut::<ElfMovement>(player_e).unwrap();
            mov.velocity = Vec3::ZERO;
            mov.is_grounded = true;
        }

        app.update();

        {
            let mov = app.world().get::<ElfMovement>(player_e).unwrap();
            assert_eq!(mov.velocity, Vec3::ZERO, "Player must NOT be re-launched while in non-collision window");
            assert!(mov.is_grounded, "Player must remain grounded if non-collision ignores contact");
            assert!(mov.moose_non_collision_timer < 1.0 && mov.moose_non_collision_timer > 0.5, "Timer must tick down");
        }

        // 3. Fast-forward past non-collision duration (1.0s) -> collision is restored
        {
            let mut mov = app.world_mut().get_mut::<ElfMovement>(player_e).unwrap();
            mov.moose_non_collision_timer = 0.01;
        }

        app.update();

        {
            let mov = app.world().get::<ElfMovement>(player_e).unwrap();
            assert_eq!(mov.moose_non_collision_timer, 0.0, "Non-collision timer must expire to 0");
            assert!(app.world().get::<MooseNonCollision>(player_e).is_none(), "MooseNonCollision component must be removed");

            // Rapier collision restored
            let groups = app.world().get::<CollisionGroups>(player_e).unwrap();
            assert_eq!(*groups, CollisionGroups::new(GROUP_PLAYER, !GROUP_HELD_ITEM));
            let kcc = app.world().get::<KinematicCharacterController>(player_e).unwrap();
            assert_eq!(kcc.filter_groups, Some(CollisionGroups::new(GROUP_PLAYER, !GROUP_HELD_ITEM)));
        }
    }
}
