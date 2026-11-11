use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    player::{
        hands::HeldByPlayer,
        ElfHands,
        controller::{ElfPlayer, FirstPersonCamera},
    },
    core::is_game_paused,
    core::GameState,
    ui::MenuState,
    dialogue::ActiveDialogue,
};

/// Generic, reusable anchor point that can accept a physics object matching an accepted trait
/// and lock it in place until the player interacts with it again.
#[derive(Component, Debug, Clone)]
pub struct AnchorPoint {
    /// The trait identifier this anchor accepts (e.g. "propane_tank", "gas_hose", "tool", or "*" for any).
    pub accepted_trait: String,
    /// Currently locked object entity, if any.
    pub locked_object: Option<Entity>,
    /// Relative translation offset applied when locking the object to this anchor point.
    pub lock_offset: Vec3,
    /// Maximum distance within which the player can interact with this anchor point.
    pub interaction_radius: f32,
    /// Whether the locked object should also adopt the anchor's rotation.
    pub lock_rotation: bool,
}

impl Default for AnchorPoint {
    fn default() -> Self {
        Self {
            accepted_trait: "*".into(),
            locked_object: None,
            lock_offset: Vec3::ZERO,
            interaction_radius: 2.5,
            lock_rotation: true,
        }
    }
}

impl AnchorPoint {
    /// Creates a new AnchorPoint accepting objects marked with the given trait tag (or "*" for wildcard).
    pub fn new(accepted_trait: impl Into<String>) -> Self {
        Self {
            accepted_trait: accepted_trait.into(),
            ..default()
        }
    }

    /// Sets the lock offset translation relative to the anchor.
    pub fn with_offset(mut self, offset: Vec3) -> Self {
        self.lock_offset = offset;
        self
    }

    /// Sets the maximum interaction radius for the player.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.interaction_radius = radius;
        self
    }

    /// Sets whether locked objects adopt the rotation of the anchor point.
    pub fn with_rotation(mut self, lock_rotation: bool) -> Self {
        self.lock_rotation = lock_rotation;
        self
    }

    /// Checks if a trait matches what this anchor accepts.
    pub fn accepts_trait(&self, trait_id: &str) -> bool {
        self.accepted_trait == "*" || self.accepted_trait == trait_id
    }
}

/// Single trait tag attached to a physics object to indicate what types of anchors it can dock with.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct AnchorTrait(pub String);

impl AnchorTrait {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Multiple trait tags attached to a physics object.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct AnchorTraits(pub Vec<String>);

impl AnchorTraits {
    pub fn new(traits: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(traits.into_iter().map(Into::into).collect())
    }

    pub fn contains(&self, trait_id: &str) -> bool {
        self.0.iter().any(|t| t == trait_id)
    }
}

/// Marker component attached to an object while it is locked inside an AnchorPoint.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockedInAnchor {
    pub anchor: Entity,
}

/// Helper function to check if an object's trait(s) match the AnchorPoint's requirement.
pub fn matches_anchor(
    anchor: &AnchorPoint,
    single_trait: Option<&AnchorTrait>,
    multi_traits: Option<&AnchorTraits>,
) -> bool {
    if anchor.accepted_trait == "*" {
        return true;
    }
    if let Some(t) = single_trait {
        if anchor.accepts_trait(&t.0) {
            return true;
        }
    }
    if let Some(mt) = multi_traits {
        if mt.0.iter().any(|t| anchor.accepts_trait(t)) {
            return true;
        }
    }
    false
}

/// Handles player interaction with AnchorPoints using Q or E (or Left Mouse Button).
/// - If an object is being held, it DOES NOT automatically snap on proximity.
/// - If the player presses Q or E while facing an AnchorPoint holding an object matching its accepted trait,
///   the object is locked into the anchor (RigidBody::Fixed, player hold released, position snapped).
/// - If the anchor already has a locked object, interacting with Q or E unlocks it (RigidBody::Dynamic restored).
pub fn anchor_point_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<GameState>>>,
    dialogue_state: Option<Res<ActiveDialogue>>,
    mut commands: Commands,
    mut q_hands: Query<&mut ElfHands>,
    q_held: Query<Entity, With<HeldByPlayer>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    q_player: Query<&GlobalTransform, (With<ElfPlayer>, Without<FirstPersonCamera>)>,
    mut q_anchors: Query<(Entity, &mut AnchorPoint, &GlobalTransform)>,
    mut q_objects: Query<
        (
            Entity,
            &mut Transform,
            Option<&AnchorTrait>,
            Option<&AnchorTraits>,
            Option<&mut RigidBody>,
            Option<&mut Velocity>,
            Option<&LockedInAnchor>,
        ),
        Without<AnchorPoint>,
    >,
) {
    if is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    if dialogue_state.as_ref().map(|d| d.is_active).unwrap_or(false) {
        return;
    }

    let interact_pressed = keys.as_ref().map(|k| {
        k.just_pressed(KeyCode::KeyE)
            || k.just_pressed(KeyCode::KeyQ)
            || k.just_pressed(KeyCode::KeyF)
    }).unwrap_or(false)
    || mouse.as_ref().map(|m| m.just_pressed(MouseButton::Left)).unwrap_or(false);

    // CRITICAL REQUIREMENT: If the object is being held, it should NOT snap to the anchor point.
    // Only user interaction with Q or E (or interact key) allows locking!
    if !interact_pressed {
        return;
    }

    // Determine player/camera vantage
    let cam_data = q_camera.iter().next().map(|gt| (gt.translation(), gt.forward()));
    let player_pos = q_player.iter().next().map(|gt| gt.translation());

    // Find the candidate AnchorPoint the player is looking at / interacting with
    let mut best_anchor: Option<(Entity, f32)> = None;

    for (anchor_e, anchor, anchor_gt) in q_anchors.iter() {
        let anchor_pos = anchor_gt.translation();
        let dist = if let Some((cam_pos, _)) = cam_data {
            cam_pos.distance(anchor_pos)
        } else if let Some(p_pos) = player_pos {
            p_pos.distance(anchor_pos)
        } else {
            0.0
        };

        if dist > anchor.interaction_radius {
            continue;
        }

        let score = if let Some((cam_pos, cam_fwd)) = cam_data {
            let to_anchor = anchor_pos - cam_pos;
            let dir = to_anchor.normalize_or_zero();
            let dot = cam_fwd.dot(dir);
            // Allow close touch even with lower dot, but require facing if further away
            if dot < 0.20 && dist > 1.2 {
                continue;
            }
            dot / (dist + 0.1)
        } else {
            1.0 / (dist + 0.1)
        };

        if best_anchor.is_none() || score > best_anchor.unwrap().1 {
            best_anchor = Some((anchor_e, score));
        }
    }

    let Some((target_anchor_e, _)) = best_anchor else {
        return;
    };

    // Retrieve mutable anchor and its transform
    let Ok((_, mut anchor, anchor_gt)) = q_anchors.get_mut(target_anchor_e) else {
        return;
    };

    // Case 1: Anchor already has a locked object -> Unlock it!
    if let Some(locked_e) = anchor.locked_object.take() {
        commands.entity(locked_e).remove::<LockedInAnchor>();
        if let Ok((_, _, _, _, mut rb_opt, mut vel_opt, _)) = q_objects.get_mut(locked_e) {
            if let Some(ref mut rb) = rb_opt {
                **rb = RigidBody::Dynamic;
            }
            if let Some(ref mut vel) = vel_opt {
                **vel = Velocity::zero();
            }
        }
        return;
    }

    // Case 2: Anchor is empty -> Check if player is holding a matching object
    let mut held_candidates = Vec::new();
    for e in q_held.iter() {
        held_candidates.push(e);
    }
    for hands in q_hands.iter() {
        if let Some(l) = hands.left_grabbed {
            if !held_candidates.contains(&l) {
                held_candidates.push(l);
            }
        }
        if let Some(r) = hands.right_grabbed {
            if !held_candidates.contains(&r) {
                held_candidates.push(r);
            }
        }
    }

    for held_e in held_candidates {
        if let Ok((_, mut tf, single_trait, multi_traits, mut rb_opt, mut vel_opt, _)) = q_objects.get_mut(held_e) {
            if matches_anchor(&anchor, single_trait, multi_traits) {
                // LOCK IT IN!
                anchor.locked_object = Some(held_e);

                let target_pos = anchor_gt.translation() + anchor.lock_offset;
                tf.translation = target_pos;
                if anchor.lock_rotation {
                    tf.rotation = anchor_gt.to_scale_rotation_translation().1;
                }

                if let Some(ref mut rb) = rb_opt {
                    **rb = RigidBody::Fixed;
                }
                if let Some(ref mut vel) = vel_opt {
                    **vel = Velocity::zero();
                }

                commands.entity(held_e).remove::<HeldByPlayer>();
                commands.entity(held_e).remove::<Sensor>();
                commands.entity(held_e).insert(LockedInAnchor { anchor: target_anchor_e });

                for mut hands in &mut q_hands {
                    if hands.left_grabbed == Some(held_e) {
                        hands.left_grabbed = None;
                    }
                    if hands.right_grabbed == Some(held_e) {
                        hands.right_grabbed = None;
                    }
                }
                break;
            }
        }
    }
}

/// Keeps locked objects synchronised with their parent anchor points if the anchor moves.
pub fn sync_locked_anchor_objects_system(
    q_anchors: Query<(Entity, &AnchorPoint, &GlobalTransform)>,
    mut q_locked: Query<(&LockedInAnchor, &mut Transform)>,
) {
    for (anchor_e, anchor, anchor_gt) in &q_anchors {
        if let Some(locked_e) = anchor.locked_object {
            if let Ok((locked, mut tf)) = q_locked.get_mut(locked_e) {
                if locked.anchor == anchor_e {
                    let target_pos = anchor_gt.translation() + anchor.lock_offset;
                    if (tf.translation - target_pos).length_squared() > 0.0001 {
                        tf.translation = target_pos;
                    }
                }
            }
        }
    }
}

/// Spawns a generic visual AnchorPoint receptor in the world.
pub fn spawn_anchor_point(
    commands: &mut Commands,
    accepted_trait: &str,
    translation: Vec3,
    meshes: Option<&mut Assets<Mesh>>,
    materials: Option<&mut Assets<StandardMaterial>>,
) -> Entity {
    let mut e = commands.spawn((
        AnchorPoint::new(accepted_trait),
        Transform::from_translation(translation),
        GlobalTransform::from_translation(translation),
        Collider::cylinder(0.15, 0.25),
        Sensor,
        Name::new(format!("AnchorPoint_{}", accepted_trait)),
    ));
    if let (Some(meshes), Some(materials)) = (meshes, materials) {
        e.insert((
            Mesh3d(meshes.add(Cylinder::new(0.20, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.24, 0.28),
                metallic: 0.7,
                perceptual_roughness: 0.35,
                ..default()
            })),
        ));
    }
    e.id()
}

// =========================================================================
// MODULAR PHYSICS DOOR
// =========================================================================

/// Configurable, reusable physics door component with hinge rotation, angle limits,
/// spring return / auto-closing, and player grab & push interaction.
#[derive(Component, Debug, Clone)]
pub struct ModularPhysicsDoor {
    /// Rest angle (default 0.0 radians = fully closed).
    pub closed_angle: f32,
    /// Minimum allowed hinge angle in radians.
    pub min_angle: f32,
    /// Maximum allowed hinge angle in radians.
    pub max_angle: f32,
    /// Current hinge angle relative to closed orientation.
    pub current_angle: f32,
    /// Hinge rotation axis in local space (typically Vec3::Y).
    pub hinge_axis: Vec3,
    /// Anchor origin in world space around which the door rotates.
    pub hinge_origin: Vec3,
    /// Spring restoring stiffness toward closed_angle.
    pub spring_stiffness: f32,
    /// Angular damping to prevent jitter and excessive oscillation.
    pub angular_damping: f32,
    /// Whether the door should automatically pull itself shut when close to closed_angle.
    pub auto_close: bool,
    /// Angle threshold (in radians) below which auto_close engages.
    pub auto_close_threshold: f32,
    /// Whether the door is currently grabbed by the player.
    pub is_grabbed: bool,
    /// Entity currently grabbing the door, if any.
    pub grabbed_by: Option<Entity>,
    /// Hand index grabbing the door: 0 = left, 1 = right.
    pub grabbed_hand: Option<u8>,
}

impl Default for ModularPhysicsDoor {
    fn default() -> Self {
        Self {
            closed_angle: 0.0,
            min_angle: -std::f32::consts::FRAC_PI_2,
            max_angle: std::f32::consts::FRAC_PI_2,
            current_angle: 0.0,
            hinge_axis: Vec3::Y,
            hinge_origin: Vec3::ZERO,
            spring_stiffness: 8.0,
            angular_damping: 4.0,
            auto_close: true,
            auto_close_threshold: 0.35,
            is_grabbed: false,
            grabbed_by: None,
            grabbed_hand: None,
        }
    }
}

impl ModularPhysicsDoor {
    /// Creates a new modular physics door with specified hinge origin and angle limits.
    pub fn new(hinge_origin: Vec3, min_angle: f32, max_angle: f32) -> Self {
        Self {
            hinge_origin,
            min_angle,
            max_angle,
            ..default()
        }
    }

    /// Sets the hinge axis (default Vec3::Y).
    pub fn with_axis(mut self, axis: Vec3) -> Self {
        self.hinge_axis = axis.normalize_or_zero();
        self
    }

    /// Sets the auto-closing behavior and engagement angle threshold.
    pub fn with_auto_close(mut self, auto_close: bool, threshold: f32) -> Self {
        self.auto_close = auto_close;
        self.auto_close_threshold = threshold;
        self
    }

    /// Sets the spring stiffness and angular damping.
    pub fn with_spring_damping(mut self, stiffness: f32, damping: f32) -> Self {
        self.spring_stiffness = stiffness;
        self.angular_damping = damping;
        self
    }
}

/// Marker component for an interactive door handle associated with a ModularPhysicsDoor.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModularDoorHandle {
    pub door_entity: Entity,
}

/// Enforces hinge angle limits, auto-closing spring restoring torque, and angular damping
/// on ModularPhysicsDoor entities.
pub fn modular_door_physics_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<GameState>>>,
    mut q_doors: Query<(
        Entity,
        &mut ModularPhysicsDoor,
        &mut Transform,
        Option<&mut Velocity>,
    )>,
) {
    if is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let raw_dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(1.0 / 60.0);
    let dt = if raw_dt < 0.002 { 1.0 / 60.0 } else { raw_dt.min(0.1) };

    for (_e, mut door, mut tf, mut opt_vel) in &mut q_doors {
        if door.is_grabbed {
            continue;
        }

        // 1. Auto-close spring torque: if within auto_close_threshold, smoothly close the door
        let angle_diff = door.current_angle - door.closed_angle;
        if door.auto_close && angle_diff.abs() < door.auto_close_threshold {
            let step = -angle_diff * (door.spring_stiffness * dt).min(1.0);
            door.current_angle += step;

            if (door.current_angle - door.closed_angle).abs() < 0.005 {
                door.current_angle = door.closed_angle;
                if let Some(ref mut vel) = opt_vel {
                    vel.angular = Vec3::ZERO;
                }
            }
        }

        // 2. Clamp current angle within allowed hinge limits
        let clamped = door.current_angle.clamp(door.min_angle, door.max_angle);
        if clamped != door.current_angle {
            door.current_angle = clamped;
            if let Some(ref mut vel) = opt_vel {
                vel.angular = Vec3::ZERO;
            }
        }

        // 3. Angular damping
        if let Some(ref mut vel) = opt_vel {
            vel.angular *= (1.0 - door.angular_damping * dt).clamp(0.0, 1.0);
        }

        // 4. Update visual rotation around hinge axis
        tf.rotation = Quat::from_axis_angle(door.hinge_axis, door.current_angle);
    }
}

/// Handles player grabbing and swinging of ModularPhysicsDoors.
pub fn modular_door_interaction_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<GameState>>>,
    dialogue_state: Option<Res<ActiveDialogue>>,
    mut q_hands: Query<&mut ElfHands>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    q_player: Query<&GlobalTransform, (With<ElfPlayer>, Without<FirstPersonCamera>)>,
    mut q_doors: Query<(Entity, &mut ModularPhysicsDoor, &GlobalTransform)>,
) {
    if is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    if dialogue_state.as_ref().map(|d| d.is_active).unwrap_or(false) {
        return;
    }

    let q_just = keys.as_ref().map(|k| k.just_pressed(KeyCode::KeyQ)).unwrap_or(false);
    let e_just = keys.as_ref().map(|k| k.just_pressed(KeyCode::KeyE)).unwrap_or(false);

    let cam_data = q_camera.iter().next().map(|gt| (gt.translation(), gt.forward()));
    let player_pos = q_player.iter().next().map(|gt| gt.translation());

    for (door_e, mut door, door_gt) in &mut q_doors {
        let door_pos = door_gt.translation();
        let dist = if let Some((cam_pos, _)) = cam_data {
            cam_pos.distance(door_pos)
        } else if let Some(p_pos) = player_pos {
            p_pos.distance(door_pos)
        } else {
            0.0
        };

        // If door is grabbed, check if player moved too far away (> 2.8m) -> release grab
        if door.is_grabbed {
            if dist > 2.8 {
                door.is_grabbed = false;
                door.grabbed_by = None;
                door.grabbed_hand = None;
                for mut hands in &mut q_hands {
                    if hands.left_grabbed == Some(door_e) {
                        hands.left_grabbed = None;
                    }
                    if hands.right_grabbed == Some(door_e) {
                        hands.right_grabbed = None;
                    }
                }
            } else {
                // If the player presses the grab key again, release the grab
                if (door.grabbed_hand == Some(0) && q_just) || (door.grabbed_hand == Some(1) && e_just) {
                    door.is_grabbed = false;
                    door.grabbed_by = None;
                    door.grabbed_hand = None;
                    for mut hands in &mut q_hands {
                        if hands.left_grabbed == Some(door_e) {
                            hands.left_grabbed = None;
                        }
                        if hands.right_grabbed == Some(door_e) {
                            hands.right_grabbed = None;
                        }
                    }
                }
            }
            continue;
        }

        // If door is not grabbed, check for grab interaction
        if dist > 2.5 {
            continue;
        }

        if q_just || e_just {
            let hand_idx = if q_just { 0 } else { 1 };
            door.is_grabbed = true;
            door.grabbed_hand = Some(hand_idx);
            for mut hands in &mut q_hands {
                if hand_idx == 0 {
                    hands.left_grabbed = Some(door_e);
                } else {
                    hands.right_grabbed = Some(door_e);
                }
            }
        }
    }
}

/// Spawns a complete ModularPhysicsDoor entity with visual mesh and colliders.
pub fn spawn_modular_physics_door(
    commands: &mut Commands,
    config: ModularPhysicsDoor,
    width: f32,
    height: f32,
    thickness: f32,
    meshes: Option<&mut Assets<Mesh>>,
    materials: Option<&mut Assets<StandardMaterial>>,
) -> Entity {
    let half_w = width * 0.5;
    let half_h = height * 0.5;
    let half_t = thickness * 0.5;

    let mut e = commands.spawn((
        config.clone(),
        Transform::from_translation(config.hinge_origin),
        GlobalTransform::from_translation(config.hinge_origin),
        RigidBody::Dynamic,
        Collider::cuboid(half_w, half_h, half_t),
        Damping {
            linear_damping: 2.0,
            angular_damping: config.angular_damping,
        },
        Name::new("ModularPhysicsDoor"),
    ));

    if let (Some(meshes), Some(materials)) = (meshes, materials) {
        e.insert((
            Mesh3d(meshes.add(Cuboid::new(width, height, thickness))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.28, 0.18),
                perceptual_roughness: 0.8,
                ..default()
            })),
        ));
    }

    e.id()
}

// =========================================================================
// MODULAR PHYSICS ROPE
// =========================================================================

/// Configurable, reusable physics rope component for chains, ropes, and hoses.
#[derive(Component, Debug, Clone)]
pub struct ModularPhysicsRope {
    /// Number of links/segments in this rope.
    pub segment_count: usize,
    /// Distance between consecutive segment pivots.
    pub segment_length: f32,
    /// Cross-section radius of the rope segments.
    pub segment_radius: f32,
    /// Ordered list of link entities from start to end.
    pub segments: Vec<Entity>,
    /// Entity providing the starting anchor position (optional).
    pub anchor_start: Option<Entity>,
    /// Entity providing the ending anchor position (optional).
    pub anchor_end: Option<Entity>,
    /// Fixed start position if anchor_start is None.
    pub start_pos: Vec3,
    /// Fixed end position if anchor_end is None.
    pub end_pos: Vec3,
    /// Maximum allowed total chain length before tension pull/drop.
    pub max_chain_length: f32,
    /// Joint stiffness restoring distance constraints between segments.
    pub stiffness: f32,
    /// Velocity damping applied to rope segments.
    pub damping: f32,
}

impl Default for ModularPhysicsRope {
    fn default() -> Self {
        Self {
            segment_count: 8,
            segment_length: 0.25,
            segment_radius: 0.03,
            segments: Vec::new(),
            anchor_start: None,
            anchor_end: None,
            start_pos: Vec3::ZERO,
            end_pos: Vec3::new(0.0, 0.0, 2.0),
            max_chain_length: 3.5,
            stiffness: 15.0,
            damping: 3.0,
        }
    }
}

impl ModularPhysicsRope {
    /// Creates a new modular physics rope with the specified segment count and length.
    pub fn new(segment_count: usize, segment_length: f32) -> Self {
        let max_chain_length = (segment_count as f32) * segment_length * 1.35;
        Self {
            segment_count,
            segment_length,
            max_chain_length,
            ..default()
        }
    }

    /// Sets anchor entities for start and end of the rope.
    pub fn with_anchors(mut self, start: Option<Entity>, end: Option<Entity>) -> Self {
        self.anchor_start = start;
        self.anchor_end = end;
        self
    }

    /// Sets the fixed start and end coordinates.
    pub fn with_endpoints(mut self, start_pos: Vec3, end_pos: Vec3) -> Self {
        self.start_pos = start_pos;
        self.end_pos = end_pos;
        self
    }

    /// Sets the stiffness and velocity damping of the rope simulation.
    pub fn with_stiffness_damping(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }

    /// Sets the maximum chain length before drop or tension release.
    pub fn with_max_chain_length(mut self, max_length: f32) -> Self {
        self.max_chain_length = max_length;
        self
    }
}

/// Marker component for an individual segment in a ModularPhysicsRope.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModularRopeSegment {
    pub rope_entity: Entity,
    pub index: usize,
}

/// Enforces distance constraints between rope segments and pins anchors.
pub fn modular_rope_physics_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<MenuState>>,
    game_state: Option<Res<State<GameState>>>,
    q_anchors: Query<&GlobalTransform>,
    mut q_ropes: Query<(Entity, &mut ModularPhysicsRope)>,
    mut q_segments: Query<(&mut Transform, Option<&mut Velocity>), With<ModularRopeSegment>>,
) {
    if is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let raw_dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(1.0 / 60.0);
    let dt = if raw_dt < 0.002 { 1.0 / 60.0 } else { raw_dt.min(0.1) };

    for (_rope_e, rope) in &mut q_ropes {
        if rope.segments.is_empty() {
            continue;
        }

        // Determine start and end anchor positions
        let start_pos = if let Some(anchor_e) = rope.anchor_start {
            q_anchors.get(anchor_e).map(|gt| gt.translation()).unwrap_or(rope.start_pos)
        } else {
            rope.start_pos
        };

        let end_pos = if let Some(anchor_e) = rope.anchor_end {
            q_anchors.get(anchor_e).map(|gt| gt.translation()).unwrap_or(rope.end_pos)
        } else {
            rope.end_pos
        };

        // Pin first segment to start anchor
        if let Some(&first_e) = rope.segments.first() {
            if let Ok((mut tf, opt_vel)) = q_segments.get_mut(first_e) {
                let diff = start_pos - tf.translation;
                tf.translation += diff * (rope.stiffness * dt).min(1.0);
                if let Some(mut vel) = opt_vel {
                    vel.linear *= (1.0 - rope.damping * dt).clamp(0.0, 1.0);
                }
            }
        }

        // Enforce consecutive segment distance constraints
        for i in 0..rope.segments.len().saturating_sub(1) {
            let seg_a_e = rope.segments[i];
            let seg_b_e = rope.segments[i + 1];

            let pos_a = if let Ok((tf_a, _)) = q_segments.get(seg_a_e) {
                tf_a.translation
            } else {
                continue;
            };
            let pos_b = if let Ok((tf_b, _)) = q_segments.get(seg_b_e) {
                tf_b.translation
            } else {
                continue;
            };

            let delta = pos_b - pos_a;
            let dist = delta.length();
            if dist > 0.0001 && dist > rope.segment_length {
                let dir = delta / dist;
                let excess = dist - rope.segment_length;
                let correction = dir * (excess * 0.5 * (rope.stiffness * dt).min(1.0));

                if i > 0 {
                    if let Ok((mut tf_a, _)) = q_segments.get_mut(seg_a_e) {
                        tf_a.translation += correction;
                    }
                }
                if let Ok((mut tf_b, _)) = q_segments.get_mut(seg_b_e) {
                    tf_b.translation -= correction;
                }
            }
        }

        // Pin last segment to end anchor if specified
        if rope.anchor_end.is_some() {
            if let Some(&last_e) = rope.segments.last() {
                if let Ok((mut tf, opt_vel)) = q_segments.get_mut(last_e) {
                    let diff = end_pos - tf.translation;
                    tf.translation += diff * (rope.stiffness * dt).min(1.0);
                    if let Some(mut vel) = opt_vel {
                        vel.linear *= (1.0 - rope.damping * dt).clamp(0.0, 1.0);
                    }
                }
            }
        }

        // Calculate total chain length
        let mut total_len = 0.0;
        let mut prev_pos = start_pos;
        for &seg_e in &rope.segments {
            if let Ok((tf, _)) = q_segments.get(seg_e) {
                total_len += prev_pos.distance(tf.translation);
                prev_pos = tf.translation;
            }
        }
        total_len += prev_pos.distance(end_pos);

        // If exceeded max chain length, apply tension correction pulling segments closer
        if total_len > rope.max_chain_length {
            let stretch_ratio = (total_len - rope.max_chain_length) / total_len;
            for &seg_e in &rope.segments {
                if let Ok((mut tf, opt_vel)) = q_segments.get_mut(seg_e) {
                    let to_start = start_pos - tf.translation;
                    tf.translation += to_start * (stretch_ratio * (rope.stiffness * dt).min(0.5));
                    if let Some(mut vel) = opt_vel {
                        vel.linear *= 0.8;
                    }
                }
            }
        }
    }
}

/// Spawns a complete ModularPhysicsRope entity along with its chained segments.
pub fn spawn_modular_physics_rope(
    commands: &mut Commands,
    mut config: ModularPhysicsRope,
    start: Vec3,
    end: Vec3,
    mut meshes: Option<&mut Assets<Mesh>>,
    mut materials: Option<&mut Assets<StandardMaterial>>,
) -> Entity {
    config.start_pos = start;
    config.end_pos = end;

    let rope_entity = commands.spawn((
        config.clone(),
        Transform::from_translation(start),
        GlobalTransform::from_translation(start),
        Name::new("ModularPhysicsRope"),
    )).id();

    let seg_count = config.segment_count.max(2);
    let mut segment_entities = Vec::with_capacity(seg_count);

    let mesh_handle = if let Some(ref mut meshes) = meshes {
        Some(meshes.add(Cylinder::new(config.segment_radius, config.segment_length)))
    } else {
        None
    };

    let material_handle = if let Some(ref mut materials) = materials {
        Some(materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.65, 0.15),
            perceptual_roughness: 0.6,
            ..default()
        }))
    } else {
        None
    };

    for i in 0..seg_count {
        let t = (i as f32 + 0.5) / (seg_count as f32);
        let pos = start.lerp(end, t);

        let mut seg_cmd = commands.spawn((
            ModularRopeSegment {
                rope_entity,
                index: i,
            },
            Transform::from_translation(pos),
            GlobalTransform::from_translation(pos),
            RigidBody::Dynamic,
            Collider::capsule_y(config.segment_length * 0.45, config.segment_radius),
            Damping {
                linear_damping: config.damping,
                angular_damping: config.damping,
            },
            Name::new(format!("ModularRopeSegment_{}", i)),
        ));

        if let (Some(mesh), Some(mat)) = (&mesh_handle, &material_handle) {
            seg_cmd.insert((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
            ));
        }

        segment_entities.push(seg_cmd.id());
    }

    commands.entity(rope_entity).insert(ModularPhysicsRope {
        segments: segment_entities,
        ..config
    });

    rope_entity
}

// =========================================================================
// MODULAR PHYSICS OBJECTS PLUGIN
// =========================================================================

/// Comprehensive plugin registering all modular physics objects:
/// AnchorPoints (attachment points), ModularPhysicsDoors, and ModularPhysicsRopes.
pub struct ModularPhysicsObjectsPlugin;

impl Plugin for ModularPhysicsObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AnchorPointPlugin)
            .add_systems(
                Update,
                (
                    modular_door_physics_system,
                    modular_door_interaction_system,
                    modular_rope_physics_system,
                ),
            );
    }
}

pub struct AnchorPointPlugin;

impl Plugin for AnchorPointPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                anchor_point_interaction_system,
                sync_locked_anchor_objects_system,
            ),
        );
    }
}
