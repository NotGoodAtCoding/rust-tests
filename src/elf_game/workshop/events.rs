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
