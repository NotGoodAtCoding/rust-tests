use bevy::prelude::*;

pub mod soft_body;
pub mod ragdoll;

pub use soft_body::{
    SoftBodyLattice, SpringPoint, SoftBodyDeformableMesh,
    update_soft_bodies_system, update_soft_body_meshes_system,
    update_soft_bodies_system as update_soft_bodies,
};
pub use ragdoll::{
    ElfAssistant, ElfNpcState, ElfLimbType, ElfRagdollLimb,
    ElfMischiefTrait, ElfJob, ElfJobHat, ElfHatCloset, PhilosopherPipe,
    ElfNavNode, ElfNavGraph, ElfNodePurpose, DynamicWaypointTarget,
    setup_elf_assistants, spawn_hat_closet, spawn_three_job_hats, spawn_philosopher_pipe,
    elf_assistant_ai_system, elf_ragdoll_limb_animation_system,
    hat_closet_interaction_system, elf_hat_assignment_system, philosopher_pipe_system,
    debug_render_elf_nav_graph, spawn_single_elf,
};

pub struct PhysicsSandboxPlugin;

impl Plugin for PhysicsSandboxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ElfNavGraph>()
            .add_systems(Startup, (setup_elf_assistants, spawn_hat_closet))
            .add_systems(
                Update,
                (
                    update_soft_bodies_system,
                    update_soft_body_meshes_system,
                    elf_assistant_ai_system,
                    elf_ragdoll_limb_animation_system,
                    hat_closet_interaction_system,
                    elf_hat_assignment_system,
                    philosopher_pipe_system,
                    debug_render_elf_nav_graph,
                ),
            );
    }
}
