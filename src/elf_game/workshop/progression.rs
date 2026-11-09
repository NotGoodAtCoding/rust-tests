use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    moose::MooseHunger,
    player::hands::Grabbable,
    wrapping::{Present, WrappingFeature, WrappingSpecification},
    workshop::{room::ConveyorBelt, UnlockDoorMessage},
};

/// Operational state of a workshop progression door barrier
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorState {
    Locked,
    Opening { progress: f32, speed: f32 },
    Open,
}

/// A physical door barrier entity guarding access to Tier 2 (Storage Wing)
/// or Tier 3 (Forbidden Back Workshop).
#[derive(Component, Debug, Clone)]
pub struct WorkshopDoor {
    pub door_id: String,
    pub required_tier: u32,
    pub state: DoorState,
    pub closed_position: Vec3,
    pub open_position: Vec3,
}

/// Motorized wrapping turntable located in the Tier 2 Storage Wing.
/// Spins presents placed upon it and automatically applies paper wrapping.
#[derive(Component, Debug, Clone)]
pub struct MotorizedTurntable {
    pub angular_speed: f32,
    pub radius: f32,
}

impl Default for MotorizedTurntable {
    fn default() -> Self {
        Self {
            angular_speed: 2.5,
            radius: 1.25,
        }
    }
}

/// Bulk tape dispenser station in the Storage Wing.
#[derive(Component, Debug, Clone)]
pub struct BulkTapeDispenser {
    pub tape_strip_length: f32,
    pub strips_dispensed: u32,
}

impl Default for BulkTapeDispenser {
    fn default() -> Self {
        Self {
            tape_strip_length: 0.20,
            strips_dispensed: 0,
        }
    }
}

/// The Tier 3 Instant Plasma Gift Shrink-Wrapper.
/// Super-fast wrapping (1.5s) with a volatility timer / heat gauge.
/// Overheating (> 100%) triggers an explosive plasma blow-out shockwave!
#[derive(Component, Debug, Clone)]
pub struct PlasmaShrinkWrapper {
    pub heat: f32,
    pub max_heat: f32,
    pub cooling_rate: f32,
    pub heat_per_use: f32,
    pub is_overheated: bool,
    pub is_processing: bool,
    pub process_timer: f32,
    pub process_duration: f32,
    pub target_present: Option<Entity>,
}

impl Default for PlasmaShrinkWrapper {
    fn default() -> Self {
        Self {
            heat: 0.0,
            max_heat: 100.0,
            cooling_rate: 15.0,
            heat_per_use: 38.0,
            is_overheated: false,
            is_processing: false,
            process_timer: 0.0,
            process_duration: 1.5,
            target_present: None,
        }
    }
}

/// Fired when the plasma shrink wrapper overheats and explodes with chaotic force
#[derive(Message, Debug, Clone)]
pub struct PlasmaExplosionEvent {
    pub position: Vec3,
    pub blast_radius: f32,
    pub impulse_force: f32,
}

/// Request to activate the plasma shrink wrapper on a designated present
#[derive(Message, Debug, Clone)]
pub struct TriggerPlasmaWrapperMessage {
    pub wrapper_entity: Entity,
    pub present_entity: Entity,
}

/// Hydro-pneumatic ribbon cannon in the Tier 3 Forbidden Workshop.
/// Rapidly fires decorative bows onto presents placed in its target cradle.
#[derive(Component, Debug, Clone)]
pub struct RibbonCannon {
    pub cooldown_timer: f32,
    pub impulse_power: f32,
    pub bows_fired: u32,
}

impl Default for RibbonCannon {
    fn default() -> Self {
        Self {
            cooldown_timer: 0.0,
            impulse_power: 12.0,
            bows_fired: 0,
        }
    }
}

/// Request to fire the hydro-pneumatic ribbon cannon at a present
#[derive(Message, Debug, Clone)]
pub struct FireRibbonCannonMessage {
    pub cannon_entity: Entity,
    pub present_entity: Entity,
}

/// Cursed Candy-Cane Overclocker totem in the Forbidden Back Workshop.
/// High risk / high reward: doubles conveyor speed, but causes Mr. Moose to hunger 1.75x faster!
#[derive(Component, Debug, Clone)]
pub struct CandyCaneOverclocker {
    pub is_active: bool,
    pub speed_multiplier: f32,
    pub moose_rate_multiplier: f32,
}

impl Default for CandyCaneOverclocker {
    fn default() -> Self {
        Self {
            is_active: false,
            speed_multiplier: 2.0,
            moose_rate_multiplier: 1.75,
        }
    }
}

/// Request to toggle the Cursed Candy-Cane Overclocker state
#[derive(Message, Debug, Clone)]
pub struct ToggleOverclockerMessage;

/// Listens for `UnlockDoorMessage` and initiates opening for matching workshop doors
pub fn handle_door_unlock_messages(
    mut messages: MessageReader<UnlockDoorMessage>,
    mut q_doors: Query<(Entity, &mut WorkshopDoor)>,
) {
    for msg in messages.read() {
        for (_entity, mut door) in &mut q_doors {
            if (door.door_id == msg.door_id || door.required_tier == msg.tier)
                && door.state == DoorState::Locked
            {
                door.state = DoorState::Opening {
                    progress: 0.0,
                    speed: 0.65,
                };
            }
        }
    }
}

/// Animates opening doors by sliding their transform upward/away and removing their blocking collider on completion
pub fn animate_workshop_doors_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut commands: Commands,
    mut q_doors: Query<(Entity, &mut WorkshopDoor, &mut Transform)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.map(|t| t.delta_secs()).unwrap_or(0.016);
    for (entity, mut door, mut transform) in &mut q_doors {
        let (closed_pos, open_pos) = (door.closed_position, door.open_position);
        let mut mark_open = false;
        if let DoorState::Opening { ref mut progress, speed } = door.state {
            *progress = (*progress + speed * dt).min(1.0);
            let p = *progress;
            transform.translation = closed_pos.lerp(open_pos, p);
            if p >= 1.0 {
                mark_open = true;
            }
        }
        if mark_open {
            door.state = DoorState::Open;
            // Deactivate the physical blocking collider so the player and objects can walk through freely
            commands.entity(entity).remove::<Collider>();
        }
    }
}

/// Applies tangential velocity and rotation to presents on the motorized turntable,
/// automatically wrapping paper if sized
pub fn motorized_turntable_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_turntables: Query<(&Transform, &MotorizedTurntable)>,
    mut q_presents: Query<(&Transform, &mut Velocity, &mut Present)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let _dt = time.map(|t| t.delta_secs()).unwrap_or(0.016);
    for (t_tf, turntable) in &q_turntables {
        for (p_tf, mut vel, mut present) in &mut q_presents {
            let offset = p_tf.translation - t_tf.translation;
            let dist_xz = (offset.x * offset.x + offset.z * offset.z).sqrt();
            if dist_xz <= turntable.radius && (offset.y - 0.25).abs() < 0.75 {
                // Tangential linear velocity: v = omega x r = (-omega * z, 0, omega * x)
                vel.linear.x = -turntable.angular_speed * offset.z;
                vel.linear.z = turntable.angular_speed * offset.x;
                vel.angular.y = turntable.angular_speed;

                // Motorized auto-wrapping: if paper is sized, turntable smoothly wraps it
                if present.features.contains(&WrappingFeature::PaperSized) {
                    present.features.insert(WrappingFeature::PaperWrapped);
                }
            }
        }
    }
}

/// Updates the Plasma Shrink Wrapper: processes active wraps, increments/decays heat,
/// and triggers chaotic explosion events if max heat is exceeded
pub fn plasma_shrink_wrapper_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut messages: MessageReader<TriggerPlasmaWrapperMessage>,
    mut explosion_events: MessageWriter<PlasmaExplosionEvent>,
    mut q_wrapper: Query<(Entity, &Transform, &mut PlasmaShrinkWrapper)>,
    mut q_presents: Query<(&mut Present, Option<&WrappingSpecification>, &mut Velocity)>,
    mut q_dynamic_bodies: Query<(&Transform, &mut Velocity), (Without<Present>, With<Grabbable>)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.map(|t| t.delta_secs()).unwrap_or(0.016);

    // 1. Process new activation requests
    for msg in messages.read() {
        if let Ok((_w_entity, _w_tf, mut wrapper)) = q_wrapper.get_mut(msg.wrapper_entity) {
            if !wrapper.is_overheated && !wrapper.is_processing {
                wrapper.is_processing = true;
                wrapper.process_timer = 0.0;
                wrapper.target_present = Some(msg.present_entity);
                wrapper.heat += wrapper.heat_per_use;
            }
        }
    }

    // 2. Update active processing, heat dissipation, and overheat hazards
    for (_wrapper_entity, w_tf, mut wrapper) in &mut q_wrapper {
        // Overheat check
        if wrapper.heat >= wrapper.max_heat && !wrapper.is_overheated {
            wrapper.is_overheated = true;
            wrapper.is_processing = false;
            let target = wrapper.target_present.take();

            // Emit explosion event
            explosion_events.write(PlasmaExplosionEvent {
                position: w_tf.translation,
                blast_radius: 8.0,
                impulse_force: 22.0,
            });

            // Slapstick blast impulses to nearby presents
            if let Some(p_ent) = target {
                if let Ok((_p, _spec, mut vel)) = q_presents.get_mut(p_ent) {
                    vel.linear += Vec3::new(
                        (w_tf.translation.x * 3.7).sin() * 12.0,
                        14.0,
                        (w_tf.translation.z * 2.9).cos() * 12.0,
                    );
                    vel.angular += Vec3::new(6.0, 8.0, -5.0);
                }
            }

            // Push all nearby grabbable tools/props away
            for (body_tf, mut body_vel) in &mut q_dynamic_bodies {
                let diff = body_tf.translation - w_tf.translation;
                let dist = diff.length();
                if dist < 8.0 && dist > 0.05 {
                    let dir = diff / dist;
                    let strength = (1.0 - (dist / 8.0)).max(0.2) * 18.0;
                    body_vel.linear += dir * strength + Vec3::new(0.0, 10.0, 0.0);
                    body_vel.angular += Vec3::new(4.0, 4.0, 4.0);
                }
            }
        }

        // Active processing progression
        if wrapper.is_processing {
            wrapper.process_timer += dt;
            if wrapper.process_timer >= wrapper.process_duration {
                wrapper.is_processing = false;
                if let Some(p_ent) = wrapper.target_present.take() {
                    if let Ok((mut present, spec_opt, mut vel)) = q_presents.get_mut(p_ent) {
                        // Instant high-tech wrap: apply all fundamental wrapping features
                        present.features.insert(WrappingFeature::PaperSized);
                        present.features.insert(WrappingFeature::PaperWrapped);
                        present.features.insert(WrappingFeature::TapedCorners);
                        present.features.insert(WrappingFeature::TapedCenter);
                        present.features.insert(WrappingFeature::DecorativeBow);
                        present.features.insert(WrappingFeature::PlasmaShrinkSealed);
                        present.quality_score *= 1.5;

                        // Check specification satisfaction
                        if let Some(spec) = spec_opt {
                            if present.meets_specification(spec) {
                                present.is_completed = true;
                            }
                        }

                        // Gentle pneumatic ejection pop
                        vel.linear += Vec3::new(0.0, 3.5, -2.0);
                    }
                }
            }
        } else {
            // Heat dissipation when idle
            wrapper.heat = (wrapper.heat - wrapper.cooling_rate * dt).max(0.0);
            if wrapper.heat <= 30.0 {
                wrapper.is_overheated = false;
            }
        }
    }
}

/// Fires the hydro-pneumatic ribbon cannon to attach ribbons and bows at high velocity
pub fn ribbon_cannon_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut messages: MessageReader<FireRibbonCannonMessage>,
    mut q_cannons: Query<&mut RibbonCannon>,
    mut q_presents: Query<(&mut Present, Option<&WrappingSpecification>, &mut Velocity)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.map(|t| t.delta_secs()).unwrap_or(0.016);
    for mut cannon in &mut q_cannons {
        if cannon.cooldown_timer > 0.0 {
            cannon.cooldown_timer = (cannon.cooldown_timer - dt).max(0.0);
        }
    }

    for msg in messages.read() {
        if let Ok(mut cannon) = q_cannons.get_mut(msg.cannon_entity) {
            if cannon.cooldown_timer <= 0.0 {
                cannon.cooldown_timer = 1.0;
                cannon.bows_fired += 1;

                if let Ok((mut present, spec_opt, mut vel)) = q_presents.get_mut(msg.present_entity) {
                    present.features.insert(WrappingFeature::RibbonTied);
                    present.features.insert(WrappingFeature::DecorativeBow);

                    if let Some(spec) = spec_opt {
                        if present.meets_specification(spec) {
                            present.is_completed = true;
                        }
                    }

                    // Pneumatic recoil / launch impulse
                    vel.linear += Vec3::new(0.0, 4.5, -3.0);
                    vel.angular += Vec3::new(4.0, 0.0, 0.0);
                }
            }
        }
    }
}

/// Toggles the Cursed Candy-Cane Overclocker, modifying conveyor belt speeds and Moose hunger depletion rate
pub fn candy_cane_overclocker_system(
    mut messages: MessageReader<ToggleOverclockerMessage>,
    mut q_overclocker: Query<&mut CandyCaneOverclocker>,
    mut q_conveyors: Query<&mut ConveyorBelt>,
    mut moose_hunger: Option<ResMut<MooseHunger>>,
) {
    for _ in messages.read() {
        for mut overclocker in &mut q_overclocker {
            overclocker.is_active = !overclocker.is_active;
            let active = overclocker.is_active;
            let speed_mult = if active { overclocker.speed_multiplier } else { 1.0 };
            let moose_mult = if active { overclocker.moose_rate_multiplier } else { 1.0 };

            for mut belt in &mut q_conveyors {
                belt.speed = 2.0 * speed_mult;
            }

            if let Some(ref mut moose) = moose_hunger {
                moose.increment_rate = 1.0 * moose_mult;
            }
        }
    }
}

/// Spawns the physical doorway openings, door barriers, and newly unlocked wing rooms
/// (Storage Wing Annex at East wall, and Forbidden Back Workshop at South wall)
pub fn spawn_storage_wing_and_forbidden_workshop(
    commands: &mut Commands,
    meshes: &mut Option<ResMut<Assets<Mesh>>>,
    materials: &mut Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<&WorkshopMaterials>,
) {
    let _has_assets = meshes.is_some() && (materials.is_some() || workshop_mats.is_some());

    // Materials
    let door_wood_mat = workshop_mats.map(|wm| wm.workbench_wood.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: WORKBENCH_WOOD,
                perceptual_roughness: 0.8,
                ..default()
            })
        })
    });

    let vault_door_mat = workshop_mats.map(|wm| wm.industrial_dark.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: INDUSTRIAL_DARK,
                metallic: 0.8,
                perceptual_roughness: 0.4,
                ..default()
            })
        })
    });

    let _hazard_mat = workshop_mats.map(|wm| wm.hazard_yellow.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: HAZARD_YELLOW,
                perceptual_roughness: 0.5,
                ..default()
            })
        })
    });

    let iron_trim_mat = workshop_mats.map(|wm| wm.wrought_iron.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: WROUGHT_IRON,
                metallic: 0.7,
                perceptual_roughness: 0.6,
                ..default()
            })
        })
    });

    let floor_wood_mat = workshop_mats.map(|wm| wm.rustic_planks.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: RUSTIC_PLANKS,
                perceptual_roughness: 0.85,
                ..default()
            })
        })
    });

    let dark_stone_mat = workshop_mats.map(|wm| wm.cast_iron.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: CAST_IRON,
                perceptual_roughness: 0.9,
                ..default()
            })
        })
    });

    let brass_mat = workshop_mats.map(|wm| wm.polished_brass.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: POLISHED_BRASS,
                metallic: 0.85,
                perceptual_roughness: 0.3,
                ..default()
            })
        })
    });

    let candy_cane_mat = workshop_mats.map(|wm| wm.candy_cane_red.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: CANDY_CANE_RED,
                perceptual_roughness: 0.4,
                ..default()
            })
        })
    });

    let plasma_chamber_mat = workshop_mats.map(|wm| wm.plasma_chamber.clone()).or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: PLASMA_CHAMBER_PURPLE,
                emissive: LinearRgba::rgb(0.5, 0.1, 0.8),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })
        })
    });

    // =========================================================================
    // 1. Storage Wing Door (East Wall: X = 13.0, Z = 0.0)
    // =========================================================================
    let mut storage_door_cmd = commands.spawn((
        Transform::from_xyz(13.0, 1.7, 0.0),
        WorkshopDoor {
            door_id: "StorageWing".to_string(),
            required_tier: 2,
            state: DoorState::Locked,
            closed_position: Vec3::new(13.0, 1.7, 0.0),
            open_position: Vec3::new(13.0, 5.0, 0.0),
        },
        Collider::cuboid(0.25, 1.7, 1.95),
        RigidBody::KinematicPositionBased,
        Name::new("StorageWingDoor"),
    ));

    if let (Some(m), Some(mat)) = (meshes.as_mut(), door_wood_mat.clone()) {
        storage_door_cmd.insert((
            Mesh3d(m.add(Cuboid::new(0.35, 3.4, 3.9))),
            MeshMaterial3d(mat),
        ));
    }

    // Doorway Archway Header above Storage Wing Door
    let mut storage_header_cmd = commands.spawn((
        Transform::from_xyz(13.0, 4.45, 0.0),
        Collider::cuboid(0.3, 1.05, 2.0),
        RigidBody::Fixed,
        Name::new("StorageWingDoorHeader"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), door_wood_mat.clone()) {
        storage_header_cmd.insert((
            Mesh3d(m.add(Cuboid::new(0.6, 2.1, 4.0))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 2. Storage Wing Annex Geometry (X in [13.0, 25.0], Z in [-6.0, 6.0])
    // =========================================================================
    // Floor
    let mut storage_floor_cmd = commands.spawn((
        Transform::from_xyz(19.0, 0.0, 0.0),
        Collider::cuboid(6.0, 0.1, 6.0),
        RigidBody::Fixed,
        Name::new("StorageWingFloor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), floor_wood_mat.clone()) {
        storage_floor_cmd.insert((
            Mesh3d(m.add(Plane3d::default().mesh().size(12.0, 12.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Storage Wing Motorized Wrapping Turntable (X = 18.0, Y = 0.35, Z = 0.0)
    let mut turntable_cmd = commands.spawn((
        Transform::from_xyz(18.0, 0.35, 0.0),
        MotorizedTurntable {
            angular_speed: 2.5,
            radius: 1.25,
        },
        Collider::cylinder(0.2, 1.25),
        RigidBody::KinematicPositionBased,
        Name::new("MotorizedWrappingTurntable"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), brass_mat.clone()) {
        turntable_cmd.insert((
            Mesh3d(m.add(Cylinder::new(1.25, 0.4))),
            MeshMaterial3d(mat),
        ));
    }

    // Storage Wing Bulk Tape Dispenser (X = 18.0, Y = 0.85, Z = 3.2)
    let mut bulk_tape_cmd = commands.spawn((
        Transform::from_xyz(18.0, 0.85, 3.2),
        BulkTapeDispenser::default(),
        Collider::cuboid(0.4, 0.4, 0.4),
        RigidBody::Fixed,
        Name::new("BulkTapeDispenserStation"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), iron_trim_mat.clone()) {
        bulk_tape_cmd.insert((
            Mesh3d(m.add(Cuboid::new(0.8, 0.8, 0.8))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 3. Forbidden Back Workshop Door (South Wall: Z = 13.0, X = 0.0)
    // =========================================================================
    let mut forbidden_door_cmd = commands.spawn((
        Transform::from_xyz(0.0, 1.7, 13.0),
        WorkshopDoor {
            door_id: "ForbiddenBackWorkshop".to_string(),
            required_tier: 3,
            state: DoorState::Locked,
            closed_position: Vec3::new(0.0, 1.7, 13.0),
            open_position: Vec3::new(0.0, 5.0, 13.0),
        },
        Collider::cuboid(1.95, 1.7, 0.25),
        RigidBody::KinematicPositionBased,
        Name::new("ForbiddenBackWorkshopDoor"),
    ));

    if let (Some(m), Some(mat)) = (meshes.as_mut(), vault_door_mat.clone()) {
        forbidden_door_cmd.insert((
            Mesh3d(m.add(Cuboid::new(3.9, 3.4, 0.35))),
            MeshMaterial3d(mat),
        ));
    }

    // Doorway Archway Header above Forbidden Workshop Door
    let mut forbidden_header_cmd = commands.spawn((
        Transform::from_xyz(0.0, 4.3, 13.0),
        Collider::cuboid(2.0, 0.9, 0.3),
        RigidBody::Fixed,
        Name::new("ForbiddenWorkshopDoorHeader"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), vault_door_mat.clone()) {
        forbidden_header_cmd.insert((
            Mesh3d(m.add(Cuboid::new(4.0, 1.8, 0.6))),
            MeshMaterial3d(mat),
        ));
    }

    // =========================================================================
    // 4. Forbidden Back Workshop Geometry (Z in [13.0, 25.0], X in [-6.0, 6.0])
    // =========================================================================
    // Floor
    let mut forbidden_floor_cmd = commands.spawn((
        Transform::from_xyz(0.0, 0.0, 19.0),
        Collider::cuboid(6.0, 0.1, 6.0),
        RigidBody::Fixed,
        Name::new("ForbiddenWorkshopFloor"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), dark_stone_mat.clone()) {
        forbidden_floor_cmd.insert((
            Mesh3d(m.add(Plane3d::default().mesh().size(12.0, 12.0))),
            MeshMaterial3d(mat),
        ));
    }

    // Instant Plasma Gift Shrink-Wrapper (X = -3.2, Y = 0.85, Z = 18.0)
    let mut plasma_wrapper_cmd = commands.spawn((
        Transform::from_xyz(-3.2, 0.85, 18.0),
        PlasmaShrinkWrapper::default(),
        Collider::cuboid(0.8, 0.85, 0.8),
        RigidBody::Fixed,
        Name::new("InstantPlasmaShrinkWrapper"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), plasma_chamber_mat.clone()) {
        plasma_wrapper_cmd.insert((
            Mesh3d(m.add(Cuboid::new(1.6, 1.7, 1.6))),
            MeshMaterial3d(mat),
        ));
    }

    // Hydro-Pneumatic Ribbon Cannon (X = 3.2, Y = 0.85, Z = 18.0)
    let mut ribbon_cannon_cmd = commands.spawn((
        Transform::from_xyz(3.2, 0.85, 18.0),
        RibbonCannon::default(),
        Collider::cuboid(0.7, 0.85, 0.7),
        RigidBody::Fixed,
        Name::new("HydroPneumaticRibbonCannon"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), iron_trim_mat.clone()) {
        ribbon_cannon_cmd.insert((
            Mesh3d(m.add(Cylinder::new(0.65, 1.7))),
            MeshMaterial3d(mat),
        ));
    }

    // Cursed Candy-Cane Overclocker Totem (X = 0.0, Y = 1.2, Z = 22.0)
    let mut overclocker_cmd = commands.spawn((
        Transform::from_xyz(0.0, 1.2, 22.0),
        CandyCaneOverclocker::default(),
        Collider::cuboid(0.5, 1.2, 0.5),
        RigidBody::Fixed,
        Name::new("CursedCandyCaneOverclocker"),
    ));
    if let (Some(m), Some(mat)) = (meshes.as_mut(), candy_cane_mat.clone()) {
        overclocker_cmd.insert((
            Mesh3d(m.add(Cylinder::new(0.45, 2.4))),
            MeshMaterial3d(mat),
        ));
    }
}
