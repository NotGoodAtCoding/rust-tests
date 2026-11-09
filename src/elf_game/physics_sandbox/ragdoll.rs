use bevy::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    assets::WorkshopMaterials,
    moose::{toy_model::MrMoose, FeedMooseMessage, MooseHunger, MooseHungerStage, MooseSnack},
    player::{
        ElfHands, ElfMovement,
        hands::{Grabbable, HeldByPlayer},
    },
    workshop::pantry::MooseTreatJar,
    wrapping::Present,
};

/// High-level AI and physics behavior state for NPC elf assistants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfNpcState {
    /// Walking along patrol waypoints / nav graph edges between workstations.
    Wandering,
    /// Paused at a workbench or conveyor inspecting toys with curious head bobs or free roaming within node radius.
    Inspecting,
    /// Refusing to work until knocked down with a physics object or player tackle.
    Slacking,
    /// Klepto elf carrying a present to the secluded stash node.
    StashingPresent,
    /// Panicking and fleeing south with arms waving when Mr. Moose is agitated or breaching.
    Panicking,
    /// Knocked down into full 6-DOF dynamic ragdoll tumbling by Moose, player rush, or physics impacts.
    RagdollTumble,
    /// Standing back up, dusting off snow/sawdust, and restoring upright balance.
    Recovering,
    /// Autonomous Moose Feeder moving towards the main table snack jar to retrieve a treat.
    FetchingMooseTreat,
    /// Autonomous Moose Feeder holding a treat and moving towards the feeding trough.
    DeliveringMooseTreat,
    /// Interacting with the player in dialogue: stopped walking and smoothly facing the player.
    InDialogue,
    /// Respawning from the pneumatic present vacuum with reverse tube transit animation.
    VacuumRespawning,
}

/// Mischievous behavioral traits assigned to workshop elves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum ElfMischiefTrait {
    /// Refuses to do tasks until knocked down with a physics object.
    Lazy,
    /// Takes presents and relocates them to a secluded stash node.
    Klepto,
    /// Tagged for future flammable mischief; currently passive.
    Pyro,
}

/// Tasks that can be assigned to elves by equipping them with hats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfJob {
    MooseFeeder,
    PresentWrapper,
    WorkshopMaintenance,
}

/// Dynamic grabbable hat prop that assigns a job when placed on an elf.
#[derive(Component, Debug, Clone)]
pub struct ElfJobHat {
    pub job: ElfJob,
    pub worn_by: Option<Entity>,
}

/// Interactive wardrobe on the back wall dispensing job hats.
#[derive(Component, Debug, Clone)]
pub struct ElfHatCloset {
    pub spawn_point: Vec3,
    pub cooldown: f32,
}

/// Semantic roles and purposes for workshop navigation waypoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElfNodePurpose {
    Workbench,
    MainTableTreatJar,
    MooseFeedingStation,
    PantryShelves,
    HatCloset,
    KleptoStash,
}

/// Target option for dynamic waypoint navigation for Elves, Moose, and other NPCs.
#[derive(Component, Debug, Clone, PartialEq)]
pub enum DynamicWaypointTarget {
    Player,
    Npc(Entity),
    Position(Vec3),
}

/// Node in the workshop waypoint pathfinding graph.
#[derive(Debug, Clone)]
pub struct ElfNavNode {
    pub id: usize,
    pub name: String,
    pub position: Vec3,
    /// Free movement radius when arriving at this node.
    pub radius: f32,
    pub neighbors: Vec<usize>,
    pub purpose: Option<ElfNodePurpose>,
}

/// Topological pathfinding navigation graph around Santa's workshop obstacles.
#[derive(Resource, Debug, Clone)]
pub struct ElfNavGraph {
    pub nodes: Vec<ElfNavNode>,
    pub debug_visible: bool,
}

impl Default for ElfNavGraph {
    fn default() -> Self {
        Self::default_workshop_graph()
    }
}

impl ElfNavGraph {
    /// Constructs the 15-node navigation graph avoiding the central workbench and conveyor belt.
    pub fn default_workshop_graph() -> Self {
        let nodes = vec![
            // Node 0: Workbench Front (North aisle)
            ElfNavNode {
                id: 0,
                name: "WorkbenchFront".to_string(),
                position: Vec3::new(0.0, 0.45, -1.8),
                radius: 1.2,
                neighbors: vec![1, 2, 6],
                purpose: Some(ElfNodePurpose::Workbench),
            },
            // Node 1: Workbench West Pass
            ElfNavNode {
                id: 1,
                name: "WorkbenchWest".to_string(),
                position: Vec3::new(-3.8, 0.45, -4.0),
                radius: 1.2,
                neighbors: vec![0, 4, 5, 6, 7],
                purpose: Some(ElfNodePurpose::MainTableTreatJar),
            },
            // Node 2: Workbench East Pass
            ElfNavNode {
                id: 2,
                name: "WorkbenchEast".to_string(),
                position: Vec3::new(3.8, 0.45, -4.0),
                radius: 1.2,
                neighbors: vec![0, 3, 6, 9],
                purpose: Some(ElfNodePurpose::Workbench),
            },
            // Node 3: Present Sending Platform
            ElfNavNode {
                id: 3,
                name: "SendingPlatform".to_string(),
                position: Vec3::new(5.5, 0.45, -4.0),
                radius: 1.3,
                neighbors: vec![2, 9],
                purpose: None,
            },
            // Node 4: Moose Feeding Trough
            ElfNavNode {
                id: 4,
                name: "MooseTrough".to_string(),
                position: Vec3::new(-2.0, 0.45, -10.5),
                radius: 1.6,
                neighbors: vec![1, 5],
                purpose: Some(ElfNodePurpose::MooseFeedingStation),
            },
            // Node 5: Northwest Rafters Pass
            ElfNavNode {
                id: 5,
                name: "NorthwestPass".to_string(),
                position: Vec3::new(-6.5, 0.45, -5.5),
                radius: 1.4,
                neighbors: vec![1, 4, 7],
                purpose: None,
            },
            // Node 6: Central Crossing
            ElfNavNode {
                id: 6,
                name: "CentralCrossing".to_string(),
                position: Vec3::new(0.0, 0.45, 0.5),
                radius: 1.5,
                neighbors: vec![0, 1, 2, 7, 9],
                purpose: None,
            },
            // Node 7: West Conveyor Bypass (clears X = -8.5 belt start & spawner duct)
            ElfNavNode {
                id: 7,
                name: "WestConveyorBypass".to_string(),
                position: Vec3::new(-11.0, 0.45, 0.5),
                radius: 1.2,
                neighbors: vec![1, 5, 6, 8],
                purpose: None,
            },
            // Node 8: West South Corridor
            ElfNavNode {
                id: 8,
                name: "WestSouthCorridor".to_string(),
                position: Vec3::new(-11.0, 0.45, 5.5),
                radius: 1.2,
                neighbors: vec![7, 11, 12, 13, 15],
                purpose: None,
            },
            // Node 9: East Conveyor Bypass (clears X = 9.8 despawner chute & top rim)
            ElfNavNode {
                id: 9,
                name: "EastConveyorBypass".to_string(),
                position: Vec3::new(11.5, 0.45, 0.5),
                radius: 1.1,
                neighbors: vec![2, 3, 6, 10],
                purpose: None,
            },
            // Node 10: East South Corridor
            ElfNavNode {
                id: 10,
                name: "EastSouthCorridor".to_string(),
                position: Vec3::new(11.5, 0.45, 5.5),
                radius: 1.1,
                neighbors: vec![9, 11, 14],
                purpose: None,
            },
            // Node 11: South Central Hallway
            ElfNavNode {
                id: 11,
                name: "SouthCentralHall".to_string(),
                position: Vec3::new(0.0, 0.45, 7.0),
                radius: 1.6,
                neighbors: vec![8, 10, 12, 14],
                purpose: None,
            },
            // Node 12: Back Wall Hat Closet
            ElfNavNode {
                id: 12,
                name: "HatCloset".to_string(),
                position: Vec3::new(-4.5, 0.45, 10.5),
                radius: 1.4,
                neighbors: vec![8, 11, 13],
                purpose: Some(ElfNodePurpose::HatCloset),
            },
            // Node 13: Secluded Klepto Stash Node
            ElfNavNode {
                id: 13,
                name: "KleptoStash".to_string(),
                position: Vec3::new(-10.5, 0.45, 10.5),
                radius: 1.4,
                neighbors: vec![8, 12],
                purpose: Some(ElfNodePurpose::KleptoStash),
            },
            // Node 14: Southeast Rest Area
            ElfNavNode {
                id: 14,
                name: "SoutheastRest".to_string(),
                position: Vec3::new(5.5, 0.45, 10.0),
                radius: 1.4,
                neighbors: vec![10, 11],
                purpose: None,
            },
            // Node 15: Pantry Shelves (inside Pantry Storage Wing at X = -15.0, Z = 5.5)
            ElfNavNode {
                id: 15,
                name: "PantryShelves".to_string(),
                position: Vec3::new(-15.0, 0.45, 5.5),
                radius: 1.5,
                neighbors: vec![8],
                purpose: Some(ElfNodePurpose::PantryShelves),
            },
        ];

        Self {
            nodes,
            debug_visible: false,
        }
    }

    /// Finds the node index associated with a given semantic purpose.
    pub fn find_node_by_purpose(&self, purpose: ElfNodePurpose) -> Option<usize> {
        self.nodes.iter().find(|n| n.purpose == Some(purpose)).map(|n| n.id)
    }

    /// Finds the shortest sequence of node indices from `start_idx` to `goal_idx` using breadth-first search.
    pub fn find_path(&self, start_idx: usize, goal_idx: usize) -> Option<Vec<usize>> {
        if start_idx >= self.nodes.len() || goal_idx >= self.nodes.len() {
            return None;
        }
        if start_idx == goal_idx {
            return Some(vec![start_idx]);
        }
        use std::collections::{HashMap, VecDeque};
        let mut queue = VecDeque::new();
        let mut visited: HashMap<usize, Option<usize>> = HashMap::new();
        queue.push_back(start_idx);
        visited.insert(start_idx, None);

        while let Some(current) = queue.pop_front() {
            if current == goal_idx {
                let mut path = Vec::new();
                let mut curr = Some(goal_idx);
                while let Some(node) = curr {
                    path.push(node);
                    curr = visited[&node];
                }
                path.reverse();
                return Some(path);
            }

            for &neighbor in &self.nodes[current].neighbors {
                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, Some(current));
                    queue.push_back(neighbor);
                }
            }
        }
        None
    }

    /// Returns the index of the node nearest to `pos`.
    pub fn closest_node(&self, pos: Vec3) -> usize {
        let mut closest = 0;
        let mut min_dist_sq = f32::MAX;
        for (i, node) in self.nodes.iter().enumerate() {
            let d_sq = pos.distance_squared(node.position);
            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
                closest = i;
            }
        }
        closest
    }
}

/// Primary component identifying an autonomous Elf Assistant NPC in the workshop.
#[derive(Component, Debug, Clone)]
pub struct ElfAssistant {
    pub id: u32,
    pub name: String,
    pub state: ElfNpcState,
    pub previous_state: ElfNpcState,
    pub mischief_trait: ElfMischiefTrait,
    pub assigned_job: Option<ElfJob>,
    pub worn_hat_entity: Option<Entity>,

    // Pathfinding & Navigation
    pub current_node: usize,
    pub target_node: usize,
    pub path: Vec<usize>,
    pub path_step: usize,
    pub free_move_target: Option<Vec3>,
    pub free_move_timer: f32,

    // Direct / legacy waypoint targets for test compatibility
    pub target_waypoint: Vec3,
    pub waypoints: Vec<Vec3>,
    pub current_waypoint_idx: usize,

    // Dynamic Waypoint Targeting
    pub dynamic_target: Option<DynamicWaypointTarget>,
    pub wander_radius: f32,

    // Speeds & Timers
    pub walk_speed: f32,
    pub run_speed: f32,
    pub walk_timer: f32,
    pub state_timer: f32,
    pub tumble_timer: f32,
    pub tumble_duration: f32,
    pub recovery_timer: f32,
    pub recovery_duration: f32,
    pub node_navigation_timer: f32,
    pub navigation_failure_count: u32,
    pub vacuum_respawn_progress: f32,

    // Mischief specific state
    pub slacking_cooldown: f32,
    pub carried_present: Option<Entity>,

    // Job specific state (Moose Feeder loop)
    pub carried_treat: Option<Entity>,
    pub missing_treats_panic: bool,
}

impl ElfAssistant {
    pub fn new(id: u32, name: &str, waypoints: Vec<Vec3>) -> Self {
        let first = waypoints.first().copied().unwrap_or(Vec3::ZERO);
        let mischief_trait = match id % 3 {
            1 => ElfMischiefTrait::Lazy,
            2 => ElfMischiefTrait::Klepto,
            _ => ElfMischiefTrait::Pyro,
        };
        Self {
            id,
            name: name.to_string(),
            state: ElfNpcState::Wandering,
            previous_state: ElfNpcState::Wandering,
            mischief_trait,
            assigned_job: None,
            worn_hat_entity: None,
            current_node: 0,
            target_node: 0,
            path: Vec::new(),
            path_step: 0,
            free_move_target: None,
            free_move_timer: 0.0,
            target_waypoint: first,
            waypoints,
            current_waypoint_idx: 0,
            dynamic_target: None,
            wander_radius: 1.5,
            walk_speed: 1.8,
            run_speed: 3.6,
            walk_timer: 0.0,
            state_timer: 0.0,
            tumble_timer: 0.0,
            tumble_duration: 2.2,
            recovery_timer: 0.0,
            recovery_duration: 0.8,
            node_navigation_timer: 0.0,
            navigation_failure_count: 0,
            vacuum_respawn_progress: 0.0,
            slacking_cooldown: 8.0,
            carried_present: None,
            carried_treat: None,
            missing_treats_panic: false,
        }
    }

    /// Sets the dynamic navigation target to follow the Player.
    pub fn target_player(&mut self) {
        self.dynamic_target = Some(DynamicWaypointTarget::Player);
    }

    /// Sets the dynamic navigation target to follow another NPC.
    pub fn target_npc(&mut self, npc: Entity) {
        self.dynamic_target = Some(DynamicWaypointTarget::Npc(npc));
    }

    /// Sets the dynamic navigation target to a specific 3D coordinate.
    pub fn target_position(&mut self, pos: Vec3) {
        self.dynamic_target = Some(DynamicWaypointTarget::Position(pos));
    }

    /// Clears any active dynamic navigation target, returning to graph/waypoint navigation.
    pub fn clear_dynamic_target(&mut self) {
        self.dynamic_target = None;
    }
}

/// Distinct anatomical segments of an impressionist Elf assistant model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfLimbType {
    Torso,
    Head,
    ElfHat,
    HatPompom,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

/// Component attached to articulated child limbs to drive procedural animations and ragdoll flopping.
#[derive(Component, Debug, Clone)]
pub struct ElfRagdollLimb {
    pub elf_entity: Entity,
    pub elf_id: u32,
    pub limb_type: ElfLimbType,
    pub rest_local_translation: Vec3,
    pub rest_local_rotation: Quat,
    pub current_wobble: Vec3,
}

/// Spawns the impressionist NPC Elf assistants into Santa's Workshop.
pub fn setup_elf_assistants(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let wm = match workshop_mats {
        Some(mats) => mats.clone(),
        None => WorkshopMaterials::new(&mut materials),
    };

    let tunic_mat = wm.pine_green;
    let hat_arm_mat = wm.festive_red;
    let head_wood_mat = wm.birch_wood;
    let pompom_mat = wm.fluffy_white;
    let belt_boot_mat = wm.walnut_leather;
    let buckle_mat = wm.brass_buckle;

    // Grounded spawn positions: Y = 0.45 touches the floor directly
    let elf_configs = [
        (
            1,
            "Elf Sparky",
            Vec3::new(-1.5, 0.45, -2.5),
            vec![
                Vec3::new(-1.5, 0.45, -2.5),
                Vec3::new(1.5, 0.45, -2.5),
                Vec3::new(0.0, 0.45, -1.8),
                Vec3::new(-3.5, 0.45, -6.0),
            ],
            ElfMischiefTrait::Lazy,
            0,
        ),
        (
            2,
            "Elf Tinsel",
            Vec3::new(-3.5, 0.45, 1.8),
            vec![
                Vec3::new(-3.5, 0.45, 1.8),
                Vec3::new(0.0, 0.45, 1.8),
                Vec3::new(4.5, 0.45, 1.8),
                Vec3::new(-1.0, 0.45, 0.5),
            ],
            ElfMischiefTrait::Klepto,
            6,
        ),
        (
            3,
            "Elf Jingles",
            Vec3::new(3.8, 0.45, -3.2),
            vec![
                Vec3::new(3.8, 0.45, -3.2),
                Vec3::new(5.2, 0.45, -4.5),
                Vec3::new(2.0, 0.45, -4.0),
                Vec3::new(3.0, 0.45, 4.0),
            ],
            ElfMischiefTrait::Pyro,
            2,
        ),
    ];

    for (id, name, spawn_pos, waypoints, mischief, start_node) in elf_configs {
        let elf_e = spawn_single_elf(
            &mut commands,
            &mut meshes,
            &tunic_mat,
            &hat_arm_mat,
            &head_wood_mat,
            &pompom_mat,
            &belt_boot_mat,
            &buckle_mat,
            id,
            name,
            spawn_pos,
            waypoints,
        );
        commands.entity(elf_e).insert(mischief);
        commands.entity(elf_e).entry::<ElfAssistant>().and_modify(move |mut elf| {
            elf.mischief_trait = mischief;
            elf.current_node = start_node;
            elf.target_node = start_node;
        });
    }
}

/// Helper spawning a single impressionist articulated elf assistant entity with Rapier dynamic physics.
/// All limb offsets are translated down by 0.48m so boots rest flush with the floor planks at Y = 0.
pub fn spawn_single_elf(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    tunic_mat: &Handle<StandardMaterial>,
    hat_arm_mat: &Handle<StandardMaterial>,
    head_wood_mat: &Handle<StandardMaterial>,
    pompom_mat: &Handle<StandardMaterial>,
    belt_boot_mat: &Handle<StandardMaterial>,
    buckle_mat: &Handle<StandardMaterial>,
    id: u32,
    name: &str,
    spawn_pos: Vec3,
    waypoints: Vec<Vec3>,
) -> Entity {
    let root_entity = commands
        .spawn((
            ElfAssistant::new(id, name, waypoints),
            Transform::from_translation(spawn_pos),
            Visibility::Inherited,
            RigidBody::Dynamic,
            Collider::capsule_y(0.25, 0.20),
            Velocity::default(),
            ExternalImpulse::default(),
            Damping {
                linear_damping: 1.2,
                angular_damping: 2.5,
            },
            LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
            Name::new(name.to_string()),
        ))
        .id();

    // 1. Torso: Pine green tunic cuboid (translated down by 0.48m -> Y: -0.10)
    let torso_pos = Vec3::new(0.0, -0.10, 0.0);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.32, 0.42, 0.22))),
        MeshMaterial3d(tunic_mat.clone()),
        Transform::from_translation(torso_pos),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::Torso,
            rest_local_translation: torso_pos,
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_Torso", name)),
    ));

    // 1b. Belt band around waist (translated down by 0.48m -> Y: -0.20)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.33, 0.07, 0.23))),
        MeshMaterial3d(belt_boot_mat.clone()),
        Transform::from_xyz(0.0, -0.20, 0.0),
        ChildOf(root_entity),
        Name::new(format!("{}_Belt", name)),
    ));

    // 1c. Brass belt buckle (translated down by 0.48m -> Y: -0.20)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.09, 0.09, 0.03))),
        MeshMaterial3d(buckle_mat.clone()),
        Transform::from_xyz(0.0, -0.20, 0.12),
        ChildOf(root_entity),
        Name::new(format!("{}_Buckle", name)),
    ));

    // 2. Head: Turned birch wood sphere (translated down by 0.48m -> Y: 0.20)
    let head_pos = Vec3::new(0.0, 0.20, 0.0);
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.13))),
        MeshMaterial3d(head_wood_mat.clone()),
        Transform::from_translation(head_pos),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::Head,
            rest_local_translation: head_pos,
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_Head", name)),
    ));

    // 2b. Conical Elf Hat (translated down by 0.48m -> Y: 0.32)
    let hat_pos = Vec3::new(0.0, 0.32, -0.04);
    let hat_rot = Quat::from_rotation_x(-0.25);
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.12, 0.26))),
        MeshMaterial3d(hat_arm_mat.clone()),
        Transform::from_translation(hat_pos).with_rotation(hat_rot),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::ElfHat,
            rest_local_translation: hat_pos,
            rest_local_rotation: hat_rot,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_Hat", name)),
    ));

    // 2c. Fluffy White Hat Pom-Pom (translated down by 0.48m -> Y: 0.46)
    let pompom_pos = Vec3::new(0.0, 0.46, -0.09);
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.05))),
        MeshMaterial3d(pompom_mat.clone()),
        Transform::from_translation(pompom_pos),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::HatPompom,
            rest_local_translation: pompom_pos,
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_Pompom", name)),
    ));

    // 3. Left Arm: Red sleeve with mitten (translated down by 0.48m -> Y: 0.02)
    let l_arm_pos = Vec3::new(-0.21, 0.02, 0.0);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.09, 0.26, 0.09))),
        MeshMaterial3d(hat_arm_mat.clone()),
        Transform::from_translation(l_arm_pos),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::LeftArm,
            rest_local_translation: l_arm_pos,
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_LeftArm", name)),
    ));

    // 4. Right Arm: Red sleeve with mitten (translated down by 0.48m -> Y: 0.02)
    let r_arm_pos = Vec3::new(0.21, 0.02, 0.0);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.09, 0.26, 0.09))),
        MeshMaterial3d(hat_arm_mat.clone()),
        Transform::from_translation(r_arm_pos),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::RightArm,
            rest_local_translation: r_arm_pos,
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_RightArm", name)),
    ));

    // 5. Left Leg & Clog Boot (translated down by 0.48m -> Y: -0.32, bottom at -0.45m flush with floor)
    let l_leg_pos = Vec3::new(-0.09, -0.32, 0.0);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.10, 0.26, 0.12))),
        MeshMaterial3d(belt_boot_mat.clone()),
        Transform::from_translation(l_leg_pos),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::LeftLeg,
            rest_local_translation: l_leg_pos,
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_LeftLeg", name)),
    ));

    // 6. Right Leg & Clog Boot (translated down by 0.48m -> Y: -0.32, bottom at -0.45m flush with floor)
    let r_leg_pos = Vec3::new(0.09, -0.32, 0.0);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.10, 0.26, 0.12))),
        MeshMaterial3d(belt_boot_mat.clone()),
        Transform::from_translation(r_leg_pos),
        ElfRagdollLimb {
            elf_entity: root_entity,
            elf_id: id,
            limb_type: ElfLimbType::RightLeg,
            rest_local_translation: r_leg_pos,
            rest_local_rotation: Quat::IDENTITY,
            current_wobble: Vec3::ZERO,
        },
        ChildOf(root_entity),
        Name::new(format!("{}_RightLeg", name)),
    ));

    root_entity
}

/// Spawns the impressionist Hat Closet wardrobe on the workshop's back wall.
pub fn spawn_hat_closet(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let wm = match workshop_mats {
        Some(mats) => mats.clone(),
        None => WorkshopMaterials::new(&mut materials),
    };

    let closet_pos = Vec3::new(-4.5, 1.3, 12.3);

    // 1. Closet Wardrobe Frame (Cedar / Pine wood)
    let closet_entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.8, 2.4, 0.8))),
            MeshMaterial3d(wm.cabin_log.clone()),
            Transform::from_translation(closet_pos),
            Collider::cuboid(0.9, 1.2, 0.4),
            RigidBody::Fixed,
            ElfHatCloset {
                spawn_point: Vec3::new(-4.5, 0.95, 11.5),
                cooldown: 0.0,
            },
            Name::new("ElfHatCloset"),
        ))
        .id();

    // 2. Hat Shelf Countertop in front
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.6, 0.1, 0.6))),
        MeshMaterial3d(wm.birch_wood.clone()),
        Transform::from_xyz(0.0, -0.45, -0.6),
        ChildOf(closet_entity),
        Name::new("Closet_HatShelf"),
    ));

    // 3. Plaque / Sign above the closet
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.4, 0.25, 0.05))),
        MeshMaterial3d(wm.walnut_leather.clone()),
        Transform::from_xyz(0.0, 1.15, -0.42),
        ChildOf(closet_entity),
        Name::new("Closet_SignBoard"),
    ));
}

/// Spawns 3 dynamic job hats (one for each elf) on the closet counter when interacted with.
pub fn spawn_three_job_hats(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    wm: &WorkshopMaterials,
    spawn_origin: Vec3,
) {
    let hat_configs = [
        (
            ElfJob::MooseFeeder,
            "MooseFeederHat",
            spawn_origin + Vec3::new(-0.5, 0.15, 0.0),
            wm.walnut_leather.clone(),
            wm.brass_buckle.clone(),
        ),
        (
            ElfJob::PresentWrapper,
            "PresentWrapperHat",
            spawn_origin + Vec3::new(0.0, 0.15, 0.0),
            wm.festive_red.clone(),
            wm.gold_ribbon.clone(),
        ),
        (
            ElfJob::WorkshopMaintenance,
            "MaintenanceHat",
            spawn_origin + Vec3::new(0.5, 0.15, 0.0),
            wm.toy_blue.clone(),
            wm.sheet_metal.clone(),
        ),
    ];

    for (job, hat_name, pos, primary_mat, accent_mat) in hat_configs {
        let hat_entity = commands
            .spawn((
                Mesh3d(meshes.add(Cylinder::new(0.14, 0.20))),
                MeshMaterial3d(primary_mat),
                Transform::from_translation(pos),
                Collider::cylinder(0.10, 0.14),
                RigidBody::Dynamic,
                Velocity::default(),
                Grabbable,
                ElfJobHat {
                    job,
                    worn_by: None,
                },
                Name::new(hat_name.to_string()),
            ))
            .id();

        // Decorative accent brim / band
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.16, 0.04))),
            MeshMaterial3d(accent_mat),
            Transform::from_xyz(0.0, -0.08, 0.0),
            ChildOf(hat_entity),
            Name::new(format!("{}_Brim", hat_name)),
        ));
    }
}

/// Interacting with the Hat Closet dispenses three job hats onto the shelf.
pub fn hat_closet_interaction_system(
    time: Option<Res<Time>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_closet: Query<(&Transform, &mut ElfHatCloset), Without<ElfMovement>>,
    q_player: Query<&Transform, (With<ElfMovement>, Without<ElfHatCloset>)>,
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let is_interacting = keys
        .as_ref()
        .map(|k| k.any_just_pressed([KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyQ]))
        .unwrap_or(false)
        || mouse_buttons
            .as_ref()
            .map(|m| m.just_pressed(MouseButton::Left))
            .unwrap_or(false);

    let Ok(player_tf) = q_player.single() else {
        return;
    };

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let wm = match workshop_mats {
        Some(mats) => mats.clone(),
        None => WorkshopMaterials::new(&mut materials),
    };

    for (closet_tf, mut closet) in &mut q_closet {
        if closet.cooldown > 0.0 {
            closet.cooldown -= dt;
        }

        let dist = closet_tf.translation.distance(player_tf.translation);
        if dist < 2.5 && is_interacting && closet.cooldown <= 0.0 {
            closet.cooldown = 1.5;
            spawn_three_job_hats(&mut commands, &mut meshes, &wm, closet.spawn_point);
        }
    }
}

/// When a player holding a job hat interacts with an elf, the hat is placed on that elf.
pub fn elf_hat_assignment_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_player: Query<(&Transform, &mut ElfHands), (Without<ElfJobHat>, Without<ElfAssistant>)>,
    mut q_elves: Query<(Entity, &Transform, &mut ElfAssistant), (Without<ElfHands>, Without<ElfJobHat>)>,
    mut q_hats: Query<
        (&mut Transform, &mut ElfJobHat, Option<&mut Velocity>),
        (Without<ElfAssistant>, Without<ElfHands>),
    >,
    mut commands: Commands,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let is_interacting = keys
        .as_ref()
        .map(|k| k.any_just_pressed([KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyQ]))
        .unwrap_or(false)
        || mouse_buttons
            .as_ref()
            .map(|m| m.just_pressed(MouseButton::Left))
            .unwrap_or(false);

    if !is_interacting {
        return;
    }

    let Ok((player_tf, mut hands)) = q_player.single_mut() else {
        return;
    };

    // Check if player is holding a hat in either hand
    let held_hat_e = if let Some(e) = hands.right_grabbed {
        if q_hats.contains(e) {
            Some((e, true))
        } else {
            hands.left_grabbed.filter(|&le| q_hats.contains(le)).map(|le| (le, false))
        }
    } else if let Some(e) = hands.left_grabbed {
        if q_hats.contains(e) {
            Some((e, false))
        } else {
            None
        }
    } else {
        None
    };

    let Some((hat_entity, is_right)) = held_hat_e else {
        return;
    };

    // Find nearest elf within interaction range (< 2.2m)
    let mut nearest_elf: Option<(Entity, f32)> = None;
    for (elf_entity, elf_tf, _) in &q_elves {
        let dist = elf_tf.translation.distance(player_tf.translation);
        if dist < 2.2 && (nearest_elf.is_none() || dist < nearest_elf.unwrap().1) {
            nearest_elf = Some((elf_entity, dist));
        }
    }

    if let Some((target_elf_e, _)) = nearest_elf {
        // Release hat from player's hand
        if is_right {
            hands.right_grabbed = None;
        } else {
            hands.left_grabbed = None;
        }

        // Get hat job
        let mut hat_job = ElfJob::WorkshopMaintenance;
        if let Ok((mut hat_tf, mut hat, opt_vel)) = q_hats.get_mut(hat_entity) {
            hat.worn_by = Some(target_elf_e);
            hat_job = hat.job;

            // Stop physics velocity
            if let Some(mut vel) = opt_vel {
                vel.linear = Vec3::ZERO;
                vel.angular = Vec3::ZERO;
            }

            // Reparent hat onto elf head
            commands.entity(hat_entity).remove::<(HeldByPlayer, Sensor, RigidBody, Collider, Grabbable)>();
            commands.entity(hat_entity).insert(ChildOf(target_elf_e));

            // Position hat sitting on the elf's head (local Y = 0.32)
            hat_tf.translation = Vec3::new(0.0, 0.32, -0.04);
            hat_tf.rotation = Quat::IDENTITY;
        }

        // Assign job to elf
        if let Ok((_, _, mut elf)) = q_elves.get_mut(target_elf_e) {
            // Despawn old worn hat if present
            if let Some(old_hat) = elf.worn_hat_entity {
                if old_hat != hat_entity {
                    commands.entity(old_hat).despawn();
                }
            }
            elf.assigned_job = Some(hat_job);
            elf.worn_hat_entity = Some(hat_entity);
            if hat_job == ElfJob::MooseFeeder {
                elf.state = ElfNpcState::FetchingMooseTreat;
                elf.path.clear();
                elf.path_step = 0;
            }
        }
    }
}

/// AI system managing autonomous task navigation via `ElfNavGraph`, pathfinding,
/// Lazy slacking traits, Klepto present stashing, high-speed player collisions,
/// dynamic physics impacts, and dynamic ragdoll tumbling.
pub fn elf_assistant_ai_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut hunger: Option<ResMut<MooseHunger>>,
    nav_graph: Option<Res<ElfNavGraph>>,
    mut q_elves: Query<(
        Entity,
        &mut Transform,
        &mut Velocity,
        &mut ExternalImpulse,
        &mut LockedAxes,
        &mut ElfAssistant,
        Option<&DynamicWaypointTarget>,
    )>,
    q_player: Query<
        (&Transform, &ElfMovement),
        (Without<ElfAssistant>, Without<MrMoose>, Without<Present>),
    >,
    q_moose: Query<
        (&Transform, Option<&crate::elf_game::workshop::events::MooseRampageTracker>),
        (With<MrMoose>, Without<ElfAssistant>, Without<ElfMovement>, Without<Present>),
    >,
    q_dynamic_props: Query<
        (Entity, &Transform, &Velocity),
        (
            With<Grabbable>,
            Without<Present>,
            Without<MooseSnack>,
            Without<MooseTreatJar>,
            Without<ElfAssistant>,
            Without<MrMoose>,
            Without<ElfMovement>,
        ),
    >,
    mut q_presents: Query<
        (Entity, &mut Transform, &mut Velocity),
        (
            With<Present>,
            Without<MooseSnack>,
            Without<MooseTreatJar>,
            Without<ElfAssistant>,
            Without<MrMoose>,
            Without<ElfMovement>,
        ),
    >,
    mut q_jars: Query<
        (Entity, &Transform, &mut MooseTreatJar),
        (
            With<MooseTreatJar>,
            Without<MooseSnack>,
            Without<Present>,
            Without<ElfAssistant>,
            Without<ElfMovement>,
            Without<MrMoose>,
        ),
    >,
    mut q_snacks: Query<
        (Entity, &mut Transform, &mut Velocity),
        (
            With<MooseSnack>,
            Without<Present>,
            Without<MooseTreatJar>,
            Without<ElfAssistant>,
            Without<MrMoose>,
            Without<ElfMovement>,
        ),
    >,
    mut feed_messages: Option<MessageWriter<FeedMooseMessage>>,
    mut commands: Commands,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Some(time) = time else {
        return;
    };
    let dt = time.delta_secs();

    let moose_breached = hunger
        .as_ref()
        .map(|h| h.stage == MooseHungerStage::BreachedRampage)
        .unwrap_or(false);
    let moose_agitated = hunger.as_ref().map(|h| h.value >= 50.0).unwrap_or(false);

    let opt_moose_tf = q_moose.iter().next().map(|(tf, _)| *tf);
    let opt_player_pos = q_player.iter().next().map(|(tf, _)| tf.translation);
    let elf_positions: Vec<(Entity, Vec3)> = q_elves
        .iter()
        .map(|(e, tf, _, _, _, _, _)| (e, tf.translation))
        .collect();

    for (_elf_entity, mut elf_tf, mut vel, mut impulse, mut locked_axes, mut elf, opt_comp_target) in &mut q_elves {
        if let Some(comp_dyn) = opt_comp_target {
            elf.dynamic_target = Some(comp_dyn.clone());
        }
        // Carry present follow if Klepto is currently stashing
        if let Some(pres_e) = elf.carried_present {
            if let Ok((_, mut pres_tf, mut pres_vel)) = q_presents.get_mut(pres_e) {
                let forward = elf_tf.forward();
                pres_tf.translation = elf_tf.translation + forward * 0.45 + Vec3::Y * 0.1;
                pres_vel.linear = Vec3::ZERO;
                pres_vel.angular = Vec3::ZERO;
            }
        }

        // Carry treat follow if Moose Feeder is delivering
        if let Some(treat_e) = elf.carried_treat {
            if let Ok((_, mut treat_tf, mut treat_vel)) = q_snacks.get_mut(treat_e) {
                let forward = elf_tf.forward();
                treat_tf.translation = elf_tf.translation + forward * 0.42 + Vec3::Y * 0.05;
                treat_vel.linear = Vec3::ZERO;
                treat_vel.angular = Vec3::ZERO;
            }
        }

        match elf.state {
            ElfNpcState::Wandering
            | ElfNpcState::Inspecting
            | ElfNpcState::Slacking
            | ElfNpcState::StashingPresent
            | ElfNpcState::FetchingMooseTreat
            | ElfNpcState::DeliveringMooseTreat => {
                // If Moose is agitated or rampaging, panic immediately!
                if moose_breached || moose_agitated {
                    elf.state = ElfNpcState::Panicking;
                    elf.state_timer = 0.0;
                    let flee_dir = Vec3::new(
                        (elf.id as f32 % 3.0 - 1.0) * 0.4,
                        0.0,
                        1.0,
                    ).normalize_or_zero();
                    vel.linear.x = flee_dir.x * elf.run_speed;
                    vel.linear.z = flee_dir.z * elf.run_speed;
                    continue;
                }

                // Check for player collision: only ragdoll if player is going above default speed cap (sprinting or bunny hopping)
                for (player_tf, player_mov) in &q_player {
                    let player_dist = player_tf.translation.distance(elf_tf.translation);
                    let player_speed = player_mov.velocity.length();
                    let default_speed_cap = player_mov.base_speed;

                    if player_dist < 1.1 {
                        let mut knock_dir = player_mov.velocity;
                        knock_dir.y = 0.0;
                        let knock_dir = knock_dir.normalize_or_zero();

                        // Only ragdoll if going above the default speed cap (from sprinting or bunny hopping)
                        if player_speed > default_speed_cap + 0.1 {
                            elf.state = ElfNpcState::RagdollTumble;
                            elf.tumble_timer = 0.0;
                            *locked_axes = LockedAxes::empty();

                            impulse.impulse += knock_dir * 10.0 + Vec3::Y * 5.0;
                            impulse.torque_impulse += Vec3::new(6.0, 4.0, 5.0);

                            // Drop carried present or treat if knocked down
                            elf.carried_present = None;
                            elf.carried_treat = None;
                            break;
                        } else if player_speed > 0.5 {
                            // At or below default speed cap: do NOT ragdoll, apply only a small nudge (<= 1.0 Ns)
                            impulse.impulse += knock_dir * 0.8;
                        }
                    }
                }
                if elf.state == ElfNpcState::RagdollTumble {
                    continue;
                }

                // Check for knockdown by moving dynamic physics objects (thrown items, sliding presents, etc.)
                for (_prop_e, prop_tf, prop_vel) in &q_dynamic_props {
                    let dist = prop_tf.translation.distance(elf_tf.translation);
                    let prop_speed = prop_vel.linear.length();
                    if dist < 0.85 && prop_speed > 1.8 {
                        elf.state = ElfNpcState::RagdollTumble;
                        elf.tumble_timer = 0.0;
                        *locked_axes = LockedAxes::empty();

                        let knock_dir = prop_vel.linear.normalize_or_zero();
                        impulse.impulse += knock_dir * 8.0 + Vec3::Y * 4.5;
                        impulse.torque_impulse += Vec3::new(5.0, 3.0, 4.0);

                        // Drop carried present or treat if knocked down
                        elf.carried_present = None;
                        elf.carried_treat = None;
                        break;
                    }
                }
                if elf.state == ElfNpcState::RagdollTumble {
                    continue;
                }

                // 1. Lazy Trait Slacking Logic
                if elf.mischief_trait == ElfMischiefTrait::Lazy {
                    if elf.state != ElfNpcState::Slacking {
                        elf.slacking_cooldown -= dt;
                        if elf.slacking_cooldown <= 0.0 {
                            elf.state = ElfNpcState::Slacking;
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        }
                    } else {
                        // Refusing to do work: stays stationary until physics knockdown
                        vel.linear.x = 0.0;
                        vel.linear.z = 0.0;
                        continue;
                    }
                }

                // 2. Klepto Trait Present Finding & Stashing Logic
                if elf.mischief_trait == ElfMischiefTrait::Klepto && elf.carried_present.is_none() && elf.assigned_job.is_none() {
                    // Check if there is an unheld present nearby to steal
                    for (pres_e, pres_tf, _) in &q_presents {
                        let dist = pres_tf.translation.distance(elf_tf.translation);
                        if dist < 1.2 && pres_tf.translation.z < 8.0 {
                            // Pick up present!
                            elf.carried_present = Some(pres_e);
                            elf.state = ElfNpcState::StashingPresent;
                            if let Some(ref graph) = nav_graph {
                                elf.current_node = graph.closest_node(elf_tf.translation);
                                elf.target_node = 13; // Klepto Stash node
                                if let Some(path) = graph.find_path(elf.current_node, 13) {
                                    elf.path = path;
                                    elf.path_step = 0;
                                }
                            }
                            break;
                        }
                    }
                }

                // 3. Moose Feeder job activation from Wandering
                if elf.assigned_job == Some(ElfJob::MooseFeeder) && elf.state == ElfNpcState::Wandering {
                    elf.state = ElfNpcState::FetchingMooseTreat;
                    elf.path.clear();
                    elf.path_step = 0;
                }

                // Navigation execution
                if elf.state == ElfNpcState::FetchingMooseTreat {
                    elf.walk_timer += dt;
                    let jar_node_idx = nav_graph
                        .as_ref()
                        .and_then(|g| g.find_node_by_purpose(ElfNodePurpose::MainTableTreatJar))
                        .unwrap_or(1);

                    if elf.path.is_empty() || elf.target_node != jar_node_idx {
                        if let Some(ref graph) = nav_graph {
                            if let Some(path) = graph.find_path(elf.current_node, jar_node_idx) {
                                elf.path = path;
                                elf.path_step = 0;
                                elf.target_node = jar_node_idx;
                            }
                        }
                    }

                    let target_pos = if let Some(ref graph) = nav_graph {
                        if elf.path_step < elf.path.len() {
                            graph.nodes[elf.path[elf.path_step]].position
                        } else {
                            graph.nodes.get(jar_node_idx).map(|n| n.position).unwrap_or(elf.target_waypoint)
                        }
                    } else {
                        elf.target_waypoint
                    };

                    let flat_diff = Vec3::new(
                        target_pos.x - elf_tf.translation.x,
                        0.0,
                        target_pos.z - elf_tf.translation.z,
                    );
                    let dist = flat_diff.length();

                    let dest_pos = if let Some(ref graph) = nav_graph {
                        graph.nodes.get(jar_node_idx).map(|n| n.position).unwrap_or(elf.target_waypoint)
                    } else {
                        elf.target_waypoint
                    };
                    let dist_to_dest = Vec3::new(
                        dest_pos.x - elf_tf.translation.x,
                        0.0,
                        dest_pos.z - elf_tf.translation.z,
                    ).length();

                    if dist_to_dest < 0.8 || (dist < 0.6 && elf.path_step + 1 >= elf.path.len()) {
                        if let Some(ref graph) = nav_graph {
                            // Arrived at MainTableTreatJar node! Inspect node area
                            elf.current_node = jar_node_idx;
                            let node_pos = graph.nodes[jar_node_idx].position;
                            let scan_radius = (graph.nodes[jar_node_idx].radius + 1.5).max(5.5);

                            let mut found_jar: Option<(Entity, f32)> = None;
                            for (jar_e, jar_tf, jar) in &q_jars {
                                if jar.treat_count > 0 {
                                    let d = jar_tf
                                        .translation
                                        .distance(node_pos)
                                        .min(jar_tf.translation.distance(elf_tf.translation));
                                    if d <= scan_radius {
                                        if found_jar.is_none() || d < found_jar.unwrap().1 {
                                            found_jar = Some((jar_e, d));
                                        }
                                    }
                                }
                            }

                            if let Some((jar_e, _)) = found_jar {
                                if let Ok((_, _, mut jar)) = q_jars.get_mut(jar_e) {
                                    jar.treat_count = jar.treat_count.saturating_sub(1);
                                }

                                let forward = elf_tf.forward();
                                let treat_pos = elf_tf.translation + forward * 0.42 + Vec3::Y * 0.1;
                                let treat_entity = commands
                                    .spawn((
                                        Transform::from_translation(treat_pos),
                                        Velocity::default(),
                                        MooseSnack { nutrition_value: 60.0 },
                                        Grabbable,
                                        Name::new("CarriedMooseTreat"),
                                    ))
                                    .id();
                                elf.carried_treat = Some(treat_entity);
                                elf.missing_treats_panic = false;

                                // Pathfind to MooseFeedingStation
                                let feeding_node_idx = graph
                                    .find_node_by_purpose(ElfNodePurpose::MooseFeedingStation)
                                    .unwrap_or(4);
                                if let Some(path) = graph.find_path(jar_node_idx, feeding_node_idx) {
                                    elf.path = path;
                                    elf.path_step = 0;
                                    elf.target_node = feeding_node_idx;
                                }
                                elf.state = ElfNpcState::DeliveringMooseTreat;
                                vel.linear.x = 0.0;
                                vel.linear.z = 0.0;
                            } else {
                                // Panic Fallback: No jar with treats found in node area!
                                elf.state = ElfNpcState::Panicking;
                                elf.missing_treats_panic = true;
                                elf.state_timer = 0.0;
                                if let Some(node) = graph.nodes.get(jar_node_idx) {
                                    if !node.neighbors.is_empty() {
                                        let next_node = node.neighbors[(elf.id as usize) % node.neighbors.len()];
                                        if let Some(path) = graph.find_path(jar_node_idx, next_node) {
                                            elf.path = path;
                                            elf.path_step = 0;
                                            elf.target_node = next_node;
                                        }
                                    }
                                }
                            }
                        } else {
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        }
                    } else if dist < 0.6 {
                        if let Some(ref _graph) = nav_graph {
                            if elf.path_step + 1 < elf.path.len() {
                                elf.current_node = elf.path[elf.path_step];
                                elf.path_step += 1;
                            }
                        }
                    } else {
                        let dir = flat_diff / dist;
                        vel.linear.x = dir.x * elf.walk_speed;
                        vel.linear.z = dir.z * elf.walk_speed;
                        if dir.length_squared() > 0.001 {
                            let target_rot = Quat::from_rotation_y(dir.x.atan2(dir.z));
                            elf_tf.rotation = elf_tf.rotation.slerp(target_rot, (dt * 8.0).min(1.0));
                        }
                    }
                } else if elf.state == ElfNpcState::DeliveringMooseTreat {
                    elf.walk_timer += dt;
                    let feeding_node_idx = nav_graph
                        .as_ref()
                        .and_then(|g| g.find_node_by_purpose(ElfNodePurpose::MooseFeedingStation))
                        .unwrap_or(4);

                    if elf.path.is_empty() || elf.target_node != feeding_node_idx {
                        if let Some(ref graph) = nav_graph {
                            if let Some(path) = graph.find_path(elf.current_node, feeding_node_idx) {
                                elf.path = path;
                                elf.path_step = 0;
                                elf.target_node = feeding_node_idx;
                            }
                        }
                    }

                    let target_pos = if let Some(ref graph) = nav_graph {
                        if elf.path_step < elf.path.len() {
                            graph.nodes[elf.path[elf.path_step]].position
                        } else {
                            graph.nodes.get(feeding_node_idx).map(|n| n.position).unwrap_or(elf.target_waypoint)
                        }
                    } else {
                        elf.target_waypoint
                    };

                    let flat_diff = Vec3::new(
                        target_pos.x - elf_tf.translation.x,
                        0.0,
                        target_pos.z - elf_tf.translation.z,
                    );
                    let dist = flat_diff.length();

                    let dest_pos = if let Some(ref graph) = nav_graph {
                        graph.nodes.get(feeding_node_idx).map(|n| n.position).unwrap_or(elf.target_waypoint)
                    } else {
                        elf.target_waypoint
                    };
                    let dist_to_dest = Vec3::new(
                        dest_pos.x - elf_tf.translation.x,
                        0.0,
                        dest_pos.z - elf_tf.translation.z,
                    ).length();

                    let legacy_dest = Vec3::new(-6.5, 0.45, -10.5);
                    let dist_to_legacy = Vec3::new(
                        legacy_dest.x - elf_tf.translation.x,
                        0.0,
                        legacy_dest.z - elf_tf.translation.z,
                    ).length();

                    if dist_to_dest < 0.8 || dist_to_legacy < 0.8 || (dist < 0.8 && elf.path_step + 1 >= elf.path.len()) {
                        if let Some(ref graph) = nav_graph {
                            // Arrived at MooseFeedingStation! Deposit treat and reduce hunger
                            elf.current_node = feeding_node_idx;

                            if let Some(ref mut h) = hunger {
                                h.value = (h.value - 60.0).max(0.0);
                                h.breach_delay_remaining = None;
                                h.is_breached = false;
                                if h.value < 20.0 {
                                    h.stage = MooseHungerStage::BehindShutters;
                                } else if h.value < 50.0 {
                                    h.stage = MooseHungerStage::PeekingThrough;
                                } else {
                                    h.stage = MooseHungerStage::AgitatedHuffing;
                                }
                            }

                            if let Some(ref mut writer) = feed_messages {
                                if let Some(treat_e) = elf.carried_treat {
                                    writer.write(FeedMooseMessage { snack_entity: treat_e });
                                }
                            }

                            if let Some(treat_e) = elf.carried_treat.take() {
                                commands.entity(treat_e).despawn();
                            }

                            // Loop back to retrieve next treat from MainTableTreatJar
                            let jar_node_idx = graph
                                .find_node_by_purpose(ElfNodePurpose::MainTableTreatJar)
                                .unwrap_or(1);
                            if let Some(path) = graph.find_path(feeding_node_idx, jar_node_idx) {
                                elf.path = path;
                                elf.path_step = 0;
                                elf.target_node = jar_node_idx;
                            }
                            elf.state = ElfNpcState::FetchingMooseTreat;
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        } else {
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        }
                    } else if dist < 0.8 {
                        if let Some(ref _graph) = nav_graph {
                            if elf.path_step + 1 < elf.path.len() {
                                elf.current_node = elf.path[elf.path_step];
                                elf.path_step += 1;
                            }
                        }
                    } else {
                        let dir = flat_diff / dist;
                        vel.linear.x = dir.x * elf.walk_speed;
                        vel.linear.z = dir.z * elf.walk_speed;
                        if dir.length_squared() > 0.001 {
                            let target_rot = Quat::from_rotation_y(dir.x.atan2(dir.z));
                            elf_tf.rotation = elf_tf.rotation.slerp(target_rot, (dt * 8.0).min(1.0));
                        }
                    }
                } else if elf.state == ElfNpcState::StashingPresent {
                    // Pathfind towards Klepto stash node (Node 13: -10.5, 0.45, 10.5)
                    let stash_pos = Vec3::new(-10.5, 0.45, 10.5);
                    let target_pos = if let Some(ref graph) = nav_graph {
                        if elf.path_step < elf.path.len() {
                            graph.nodes[elf.path[elf.path_step]].position
                        } else {
                            stash_pos
                        }
                    } else {
                        stash_pos
                    };

                    let flat_diff = Vec3::new(
                        target_pos.x - elf_tf.translation.x,
                        0.0,
                        target_pos.z - elf_tf.translation.z,
                    );
                    let dist = flat_diff.length();

                    if dist < 0.6 {
                        if let Some(ref _graph) = nav_graph {
                            if elf.path_step + 1 < elf.path.len() {
                                elf.path_step += 1;
                            } else {
                                // Arrived at stash! Drop present
                                elf.carried_present = None;
                                elf.state = ElfNpcState::Inspecting;
                                elf.state_timer = 0.0;
                                vel.linear.x = 0.0;
                                vel.linear.z = 0.0;
                            }
                        } else {
                            elf.carried_present = None;
                            elf.state = ElfNpcState::Inspecting;
                            elf.state_timer = 0.0;
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        }
                    } else {
                        let dir = flat_diff / dist;
                        vel.linear.x = dir.x * elf.walk_speed;
                        vel.linear.z = dir.z * elf.walk_speed;
                        if dir.length_squared() > 0.001 {
                            let target_rot = Quat::from_rotation_y(dir.x.atan2(dir.z));
                            elf_tf.rotation = elf_tf.rotation.slerp(target_rot, (dt * 8.0).min(1.0));
                        }
                    }
                } else if elf.state == ElfNpcState::Wandering {
                    elf.walk_timer += dt;

                    // Navigation stuck prevention:
                    // If unable to reach target node in 10s, randomly choose a different node.
                    // If failed 3 times, respawn from present vacuum and play reverse animation!
                    elf.node_navigation_timer += dt;
                    if elf.node_navigation_timer >= 10.0 {
                        elf.node_navigation_timer = 0.0;
                        elf.navigation_failure_count += 1;
                        if elf.navigation_failure_count >= 3 {
                            elf.state = ElfNpcState::VacuumRespawning;
                            elf.vacuum_respawn_progress = 1.0;
                            vel.linear = Vec3::ZERO;
                            commands.spawn((
                                crate::elf_game::workshop::room::TubeTravellingLump {
                                    progress: 1.0,
                                    speed: -1.25,
                                    mass: 2.0,
                                    size: 0.6,
                                },
                                Name::new("ReverseVacuumLump"),
                            ));
                            continue;
                        } else if let Some(ref graph) = nav_graph {
                            let num_nodes = graph.nodes.len();
                            if num_nodes > 1 {
                                let mut next_goal = (elf.target_node + (elf.id as usize * 3 + 1) + elf.navigation_failure_count as usize) % num_nodes;
                                if next_goal == elf.target_node {
                                    next_goal = (next_goal + 1) % num_nodes;
                                }
                                elf.target_node = next_goal;
                                if let Some(path) = graph.find_path(elf.current_node, next_goal) {
                                    elf.path = path;
                                    elf.path_step = 0;
                                }
                            }
                        }
                    }

                    // Choose next target using DynamicWaypointTarget if set, otherwise NavGraph or waypoints
                    let (target_pos, is_dynamic) = if let Some(ref dyn_target) = elf.dynamic_target {
                        let pos = match dyn_target {
                            DynamicWaypointTarget::Player => {
                                opt_player_pos.unwrap_or(elf.target_waypoint)
                            }
                            DynamicWaypointTarget::Npc(npc_e) => {
                                if let Some((_, p)) = elf_positions.iter().find(|(e, _)| *e == *npc_e) {
                                    *p
                                } else if let Some(moose_tf) = opt_moose_tf {
                                    moose_tf.translation
                                } else {
                                    elf.target_waypoint
                                }
                            }
                            DynamicWaypointTarget::Position(p) => *p,
                        };
                        (pos, true)
                    } else if let Some(ref graph) = nav_graph {
                        if elf.path.is_empty() || elf.path_step >= elf.path.len() {
                            // Pick a new random goal node
                            let next_goal = (elf.target_node + (elf.id as usize * 3 + 2)) % graph.nodes.len();
                            elf.target_node = next_goal;
                            if let Some(path) = graph.find_path(elf.current_node, next_goal) {
                                elf.path = path;
                                elf.path_step = 0;
                            }
                        }
                        let pos = if elf.path_step < elf.path.len() {
                            graph.nodes[elf.path[elf.path_step]].position
                        } else {
                            elf.target_waypoint
                        };
                        (pos, false)
                    } else {
                        (elf.target_waypoint, false)
                    };

                    let flat_diff = Vec3::new(
                        target_pos.x - elf_tf.translation.x,
                        0.0,
                        target_pos.z - elf_tf.translation.z,
                    );
                    let dist = flat_diff.length();
                    let arrival_dist = if is_dynamic { 1.2 } else { 0.5 };

                    if dist < arrival_dist {
                        if is_dynamic {
                            elf.state = ElfNpcState::Inspecting;
                            elf.state_timer = 0.0;
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        } else if let Some(ref graph) = nav_graph {
                            if !elf.path.is_empty() && elf.path_step + 1 < elf.path.len() {
                                elf.current_node = elf.path[elf.path_step];
                                elf.path_step += 1;
                            } else {
                                // Arrived at final destination node: inspect surroundings and allow free movement
                                elf.current_node = elf.target_node;
                                elf.node_navigation_timer = 0.0;
                                elf.navigation_failure_count = 0;
                                elf.state = ElfNpcState::Inspecting;
                                elf.state_timer = 0.0;
                                vel.linear.x = 0.0;
                                vel.linear.z = 0.0;

                                // Pick free-movement target within node radius
                                let node_radius = graph.nodes[elf.current_node].radius.max(elf.wander_radius);
                                let angle = (elf.id as f32 * 1.7) + elf.walk_timer;
                                let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * (node_radius * 0.95);
                                elf.free_move_target = Some(graph.nodes[elf.current_node].position + offset);
                            }
                        } else {
                            elf.state = ElfNpcState::Inspecting;
                            elf.state_timer = 0.0;
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        }
                    } else {
                        let dir = flat_diff / dist;
                        vel.linear.x = dir.x * elf.walk_speed;
                        vel.linear.z = dir.z * elf.walk_speed;

                        if dir.length_squared() > 0.001 {
                            let target_rot = Quat::from_rotation_y(dir.x.atan2(dir.z));
                            elf_tf.rotation = elf_tf.rotation.slerp(target_rot, (dt * 8.0).min(1.0));
                        }
                    }
                } else if elf.state == ElfNpcState::Inspecting {
                    elf.state_timer += dt;

                    // Free movement around node radius
                    if let Some(free_tgt) = elf.free_move_target {
                        let diff = Vec3::new(free_tgt.x - elf_tf.translation.x, 0.0, free_tgt.z - elf_tf.translation.z);
                        let dist = diff.length();
                        if dist > 0.2 {
                            let dir = diff / dist;
                            vel.linear.x = dir.x * (elf.walk_speed * 0.5);
                            vel.linear.z = dir.z * (elf.walk_speed * 0.5);
                        } else {
                            vel.linear.x = 0.0;
                            vel.linear.z = 0.0;
                        }
                    }

                    if elf.state_timer > 2.5 {
                        if elf.dynamic_target.is_some() {
                            elf.state = ElfNpcState::Wandering;
                            elf.free_move_target = None;
                        } else if let Some(ref graph) = nav_graph {
                            // Done inspecting, choose next goal node
                            let next_goal = (elf.current_node + (elf.id as usize * 3 + 1)) % graph.nodes.len();
                            elf.target_node = next_goal;
                            if let Some(path) = graph.find_path(elf.current_node, next_goal) {
                                elf.path = path;
                                elf.path_step = 0;
                            }
                            elf.state = ElfNpcState::Wandering;
                            elf.free_move_target = None;
                        } else if !elf.waypoints.is_empty() {
                            elf.current_waypoint_idx = (elf.current_waypoint_idx + 1) % elf.waypoints.len();
                            elf.target_waypoint = elf.waypoints[elf.current_waypoint_idx];
                            elf.state = ElfNpcState::Wandering;
                            elf.free_move_target = None;
                        }
                    }
                }
            }

            ElfNpcState::Panicking => {
                elf.walk_timer += dt;
                elf.state_timer += dt;

                if let Some(moose_tf) = opt_moose_tf {
                    let moose_dist = moose_tf.translation.distance(elf_tf.translation);
                    if moose_breached && moose_dist < 2.8 {
                        elf.state = ElfNpcState::RagdollTumble;
                        elf.tumble_timer = 0.0;
                        *locked_axes = LockedAxes::empty();

                        let mut away = elf_tf.translation - moose_tf.translation;
                        away.y = 0.0;
                        let away = away.normalize_or_zero();
                        impulse.impulse += away * 16.0 + Vec3::Y * 9.0;
                        impulse.torque_impulse += Vec3::new(10.0, 12.0, 8.0);
                        elf.carried_present = None;
                        elf.carried_treat = None;
                        continue;
                    }
                }

                if elf.missing_treats_panic {
                    // Check if fresh MooseTreatJar with treats has arrived in the node area
                    let jar_node_idx = nav_graph
                        .as_ref()
                        .and_then(|g| g.find_node_by_purpose(ElfNodePurpose::MainTableTreatJar))
                        .unwrap_or(1);
                    let node_pos = nav_graph
                        .as_ref()
                        .map(|g| g.nodes[jar_node_idx].position)
                        .unwrap_or(Vec3::new(-3.8, 0.45, -4.0));
                    let scan_radius = nav_graph
                        .as_ref()
                        .map(|g| (g.nodes[jar_node_idx].radius + 1.5).max(5.5))
                        .unwrap_or(5.5);

                    let fresh_jar_found = q_jars.iter().any(|(_, jar_tf, jar)| {
                        jar.treat_count > 0
                            && (jar_tf.translation.distance(node_pos) <= scan_radius
                                || jar_tf.translation.distance(elf_tf.translation) <= scan_radius)
                    });

                    if fresh_jar_found {
                        // Fresh jar placed by player! Panic resolved!
                        elf.missing_treats_panic = false;
                        elf.state = ElfNpcState::FetchingMooseTreat;
                        if let Some(ref graph) = nav_graph {
                            if let Some(path) = graph.find_path(elf.current_node, jar_node_idx) {
                                elf.path = path;
                                elf.path_step = 0;
                                elf.target_node = jar_node_idx;
                            }
                        }
                        continue;
                    } else {
                        // Erratic pacing between neighbor nodes
                        let target_pos = if let Some(ref graph) = nav_graph {
                            if elf.path_step < elf.path.len() {
                                graph.nodes[elf.path[elf.path_step]].position
                            } else {
                                if let Some(node) = graph.nodes.get(elf.current_node) {
                                    if !node.neighbors.is_empty() {
                                        let next_node = node.neighbors[(elf.id as usize + (elf.state_timer as usize)) % node.neighbors.len()];
                                        if let Some(path) = graph.find_path(elf.current_node, next_node) {
                                            elf.path = path;
                                            elf.path_step = 0;
                                            elf.target_node = next_node;
                                        }
                                    }
                                }
                                if elf.path_step < elf.path.len() {
                                    graph.nodes[elf.path[elf.path_step]].position
                                } else {
                                    elf.target_waypoint
                                }
                            }
                        } else {
                            elf.target_waypoint
                        };

                        let flat_diff = Vec3::new(target_pos.x - elf_tf.translation.x, 0.0, target_pos.z - elf_tf.translation.z);
                        let dist = flat_diff.length();
                        if dist < 0.6 {
                            if elf.path_step + 1 < elf.path.len() {
                                elf.path_step += 1;
                                if let Some(ref _graph) = nav_graph {
                                    elf.current_node = elf.path[elf.path_step];
                                }
                            } else {
                                elf.path.clear();
                                elf.path_step = 0;
                            }
                        } else {
                            let dir = flat_diff / dist;
                            vel.linear.x = dir.x * (elf.run_speed * 0.7);
                            vel.linear.z = dir.z * (elf.run_speed * 0.7);
                            if dir.length_squared() > 0.001 {
                                let target_rot = Quat::from_rotation_y(dir.x.atan2(dir.z));
                                elf_tf.rotation = elf_tf.rotation.slerp(target_rot, (dt * 10.0).min(1.0));
                            }
                        }
                        continue;
                    }
                }

                if !moose_breached && !moose_agitated {
                    elf.state = if elf.assigned_job == Some(ElfJob::MooseFeeder) {
                        if elf.carried_treat.is_some() {
                            ElfNpcState::DeliveringMooseTreat
                        } else {
                            ElfNpcState::FetchingMooseTreat
                        }
                    } else {
                        ElfNpcState::Wandering
                    };
                    continue;
                }

                let flee_dir = Vec3::new(
                    (elf.id as f32 % 3.0 - 1.0) * 0.4,
                    0.0,
                    1.0,
                )
                .normalize_or_zero();

                vel.linear.x = flee_dir.x * elf.run_speed;
                vel.linear.z = flee_dir.z * elf.run_speed;

                if flee_dir.length_squared() > 0.001 {
                    let target_rot = Quat::from_rotation_y(flee_dir.x.atan2(flee_dir.z));
                    elf_tf.rotation = elf_tf.rotation.slerp(target_rot, (dt * 10.0).min(1.0));
                }
            }

            ElfNpcState::InDialogue => {
                // Stop walking immediately
                vel.linear.x = 0.0;
                vel.linear.z = 0.0;

                // Turn to face the player smoothly
                if let Some((player_tf, _)) = q_player.iter().next() {
                    let flat_diff = Vec3::new(
                        player_tf.translation.x - elf_tf.translation.x,
                        0.0,
                        player_tf.translation.z - elf_tf.translation.z,
                    );
                    if flat_diff.length_squared() > 0.001 {
                        let target_yaw = flat_diff.x.atan2(flat_diff.z);
                        let target_rot = Quat::from_rotation_y(target_yaw);
                        elf_tf.rotation = elf_tf.rotation.slerp(target_rot, (dt * 8.0).min(1.0));
                    }
                }
            }

            ElfNpcState::VacuumRespawning => {
                elf.vacuum_respawn_progress -= 1.0 * dt;
                vel.linear = Vec3::ZERO;

                let funnel_pos = Vec3::new(5.5, 4.0, -4.0);
                let platform_pos = Vec3::new(5.5, 0.45, -4.0);

                if elf.vacuum_respawn_progress > 0.3 {
                    elf_tf.translation = funnel_pos;
                } else if elf.vacuum_respawn_progress > 0.0 {
                    let descent_t = (0.3 - elf.vacuum_respawn_progress) / 0.3;
                    elf_tf.translation = funnel_pos.lerp(platform_pos, descent_t);
                } else {
                    elf_tf.translation = platform_pos;
                    elf.state = ElfNpcState::Inspecting;
                    elf.state_timer = 0.0;
                    elf.node_navigation_timer = 0.0;
                    elf.navigation_failure_count = 0;
                    elf.vacuum_respawn_progress = 0.0;

                    if let Some(ref graph) = nav_graph {
                        let nearest_node = 6.min(graph.nodes.len() - 1);
                        elf.current_node = nearest_node;
                        let next_goal = (nearest_node + (elf.id as usize * 3 + 2)) % graph.nodes.len();
                        elf.target_node = next_goal;
                        if let Some(path) = graph.find_path(nearest_node, next_goal) {
                            elf.path = path;
                            elf.path_step = 0;
                        }
                    }
                }
            }

            ElfNpcState::RagdollTumble => {
                elf.tumble_timer += dt;

                let current_speed = vel.linear.length();
                if elf.tumble_timer >= elf.tumble_duration && current_speed < 0.6 {
                    elf.state = ElfNpcState::Recovering;
                    elf.recovery_timer = 0.0;
                }
            }

            ElfNpcState::Recovering => {
                elf.recovery_timer += dt;
                let progress = (elf.recovery_timer / elf.recovery_duration).clamp(0.0, 1.0);

                let upright = Quat::from_rotation_y(elf_tf.rotation.to_euler(EulerRot::YXZ).0);
                elf_tf.rotation = elf_tf.rotation.slerp(upright, progress);

                if progress >= 1.0 {
                    *locked_axes = LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z;
                    // Reset slacking cooldown after recovering from a slapstick knockdown!
                    elf.slacking_cooldown = 12.0;
                    elf.state = if moose_breached || moose_agitated {
                        ElfNpcState::Panicking
                    } else if elf.missing_treats_panic {
                        ElfNpcState::Panicking
                    } else if elf.assigned_job == Some(ElfJob::MooseFeeder) {
                        if elf.carried_treat.is_some() {
                            ElfNpcState::DeliveringMooseTreat
                        } else {
                            ElfNpcState::FetchingMooseTreat
                        }
                    } else {
                        ElfNpcState::Wandering
                    };
                }
            }
        }
    }
}

/// Drives procedural limb animations for walking, panicking arm flails, slacking slouch, and dynamic ragdoll wobble.
pub fn elf_ragdoll_limb_animation_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_elves: Query<&ElfAssistant>,
    mut q_limbs: Query<(&ElfRagdollLimb, &mut Transform)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Some(time) = time else {
        return;
    };
    let total_time = time.elapsed_secs();

    for (limb, mut limb_tf) in &mut q_limbs {
        let Ok(elf) = q_elves.get(limb.elf_entity) else {
            continue;
        };

        match elf.state {
            ElfNpcState::Wandering
            | ElfNpcState::StashingPresent
            | ElfNpcState::FetchingMooseTreat
            | ElfNpcState::DeliveringMooseTreat => {
                let walk_t = elf.walk_timer;
                let gait_sin = (walk_t * 7.5).sin();

                match limb.limb_type {
                    ElfLimbType::Torso => {
                        let bounce = (walk_t * 15.0).sin().abs() * 0.03;
                        limb_tf.translation = limb.rest_local_translation + Vec3::Y * bounce;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                    ElfLimbType::LeftArm => {
                        if elf.carried_present.is_some() || elf.carried_treat.is_some() {
                            // Holding present or treat in front
                            limb_tf.translation = limb.rest_local_translation;
                            limb_tf.rotation = Quat::from_rotation_x(-0.8) * Quat::from_rotation_y(0.3);
                        } else {
                            limb_tf.translation = limb.rest_local_translation;
                            limb_tf.rotation = Quat::from_rotation_x(gait_sin * 0.45);
                        }
                    }
                    ElfLimbType::RightArm => {
                        if elf.carried_present.is_some() || elf.carried_treat.is_some() {
                            // Holding present or treat in front
                            limb_tf.translation = limb.rest_local_translation;
                            limb_tf.rotation = Quat::from_rotation_x(-0.8) * Quat::from_rotation_y(-0.3);
                        } else {
                            limb_tf.translation = limb.rest_local_translation;
                            limb_tf.rotation = Quat::from_rotation_x(-gait_sin * 0.45);
                        }
                    }
                    ElfLimbType::LeftLeg => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-gait_sin * 0.50);
                    }
                    ElfLimbType::RightLeg => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(gait_sin * 0.50);
                    }
                    ElfLimbType::Head => {
                        let bob = (walk_t * 7.5).sin() * 0.05;
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_z(bob);
                    }
                    ElfLimbType::ElfHat | ElfLimbType::HatPompom => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                }
            }

            ElfNpcState::Inspecting => {
                let inspect_sin = (total_time * 2.5).sin();
                match limb.limb_type {
                    ElfLimbType::Head => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(0.22 + inspect_sin * 0.08);
                    }
                    ElfLimbType::Torso => {
                        let breath = (total_time * 3.0).sin() * 0.01;
                        limb_tf.translation = limb.rest_local_translation + Vec3::Y * breath;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                    _ => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                }
            }

            ElfNpcState::Slacking => {
                // Slouched, drooping head and hanging arms
                let snore = (total_time * 2.0).sin();
                match limb.limb_type {
                    ElfLimbType::Head => {
                        limb_tf.translation = limb.rest_local_translation + Vec3::Y * -0.04;
                        limb_tf.rotation = Quat::from_rotation_x(0.35 + snore * 0.06);
                    }
                    ElfLimbType::Torso => {
                        let slouch = (total_time * 2.0).sin() * 0.015;
                        limb_tf.translation = limb.rest_local_translation + Vec3::Y * (slouch - 0.04);
                        limb_tf.rotation = Quat::from_rotation_x(0.12);
                    }
                    ElfLimbType::LeftArm => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(0.2) * Quat::from_rotation_z(0.1);
                    }
                    ElfLimbType::RightArm => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(0.2) * Quat::from_rotation_z(-0.1);
                    }
                    _ => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                }
            }

            ElfNpcState::Panicking => {
                let panic_sin = (total_time * 15.0).sin();
                let panic_cos = (total_time * 15.0).cos();

                match limb.limb_type {
                    ElfLimbType::LeftArm => {
                        limb_tf.translation = limb.rest_local_translation + Vec3::Y * 0.06;
                        limb_tf.rotation = Quat::from_rotation_x(-2.4 + panic_sin * 0.45)
                            * Quat::from_rotation_z(0.35 + panic_cos * 0.2);
                    }
                    ElfLimbType::RightArm => {
                        limb_tf.translation = limb.rest_local_translation + Vec3::Y * 0.06;
                        limb_tf.rotation = Quat::from_rotation_x(-2.4 - panic_sin * 0.45)
                            * Quat::from_rotation_z(-0.35 - panic_cos * 0.2);
                    }
                    ElfLimbType::Head => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_y(panic_sin * 0.3)
                            * Quat::from_rotation_x(-0.2);
                    }
                    ElfLimbType::LeftLeg => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-panic_sin * 0.6);
                    }
                    ElfLimbType::RightLeg => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(panic_sin * 0.6);
                    }
                    _ => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                }
            }

            ElfNpcState::InDialogue => {
                let breath = (total_time * 2.5).sin();
                match limb.limb_type {
                    ElfLimbType::Torso => {
                        limb_tf.translation = limb.rest_local_translation + Vec3::Y * (breath * 0.008);
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                    ElfLimbType::Head => {
                        let nod = (total_time * 3.0).sin() * 0.04;
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(nod);
                    }
                    ElfLimbType::LeftArm => {
                        let gesture = (total_time * 2.0).sin() * 0.12;
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-0.3 + gesture) * Quat::from_rotation_z(0.15);
                    }
                    ElfLimbType::RightArm => {
                        let gesture = (total_time * 2.0 + 1.0).sin() * 0.12;
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-0.3 + gesture) * Quat::from_rotation_z(-0.15);
                    }
                    ElfLimbType::LeftLeg | ElfLimbType::RightLeg => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::IDENTITY;
                    }
                    ElfLimbType::ElfHat | ElfLimbType::HatPompom => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                }
            }

            ElfNpcState::VacuumRespawning => {
                let flutter = (total_time * 12.0).sin();
                match limb.limb_type {
                    ElfLimbType::Torso => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                    ElfLimbType::Head => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-0.3 + flutter * 0.1);
                    }
                    ElfLimbType::LeftArm => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-2.2 + flutter * 0.2) * Quat::from_rotation_z(0.3);
                    }
                    ElfLimbType::RightArm => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-2.2 - flutter * 0.2) * Quat::from_rotation_z(-0.3);
                    }
                    ElfLimbType::LeftLeg => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(0.4 * flutter);
                    }
                    ElfLimbType::RightLeg => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = Quat::from_rotation_x(-0.4 * flutter);
                    }
                    ElfLimbType::ElfHat | ElfLimbType::HatPompom => {
                        limb_tf.translation = limb.rest_local_translation;
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                }
            }

            ElfNpcState::RagdollTumble => {
                let wobble = (total_time * 10.0 + limb.elf_id as f32).sin();
                match limb.limb_type {
                    ElfLimbType::LeftArm => {
                        limb_tf.rotation = Quat::from_rotation_z(1.2 + wobble * 0.4)
                            * Quat::from_rotation_x(0.6);
                    }
                    ElfLimbType::RightArm => {
                        limb_tf.rotation = Quat::from_rotation_z(-1.2 - wobble * 0.4)
                            * Quat::from_rotation_x(-0.6);
                    }
                    ElfLimbType::LeftLeg => {
                        limb_tf.rotation = Quat::from_rotation_z(0.4 + wobble * 0.3);
                    }
                    ElfLimbType::RightLeg => {
                        limb_tf.rotation = Quat::from_rotation_z(-0.4 - wobble * 0.3);
                    }
                    ElfLimbType::Head => {
                        limb_tf.rotation = Quat::from_rotation_x(-0.4 + wobble * 0.2);
                    }
                    _ => {
                        limb_tf.rotation = limb.rest_local_rotation;
                    }
                }
            }

            ElfNpcState::Recovering => {
                let progress = (elf.recovery_timer / elf.recovery_duration).clamp(0.0, 1.0);
                limb_tf.translation = limb.rest_local_translation;
                limb_tf.rotation = limb_tf.rotation.slerp(limb.rest_local_rotation, progress);
            }
        }
    }
}

/// Visualizes the navigation graph nodes, radii, and edge connections with `Gizmos` in debug mode.
pub fn debug_render_elf_nav_graph(
    graph: Option<Res<ElfNavGraph>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    mut gizmos: Gizmos,
) {
    let Some(graph) = graph else {
        return;
    };
    let show_debug = graph.debug_visible
        || menu_state.as_ref().map(|m| m.show_wireframes).unwrap_or(false);
    if !show_debug {
        return;
    }

    for node in &graph.nodes {
        // Draw node center sphere
        gizmos.sphere(node.position, 0.25, Color::srgb(0.2, 0.8, 1.0));
        // Draw free-roam radius circle on the floor
        gizmos.circle(
            Isometry3d::new(
                node.position + Vec3::Y * 0.05,
                Quat::from_rotation_x(1.5707963),
            ),
            node.radius,
            Color::srgba(0.2, 0.8, 1.0, 0.35),
        );

        // Draw connections to neighboring nodes
        for &neighbor in &node.neighbors {
            if neighbor > node.id && neighbor < graph.nodes.len() {
                let neighbor_pos = graph.nodes[neighbor].position;
                gizmos.line(
                    node.position + Vec3::Y * 0.1,
                    neighbor_pos + Vec3::Y * 0.1,
                    Color::srgb(0.9, 0.8, 0.2),
                );
            }
        }
    }
}
