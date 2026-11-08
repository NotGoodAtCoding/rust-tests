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
