use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;

/// Represents a single point mass within a soft-body lattice.
#[derive(Component, Debug, Clone)]
pub struct SpringPoint {
    pub position: Vec3,
    pub original_offset: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

/// Spring-mass lattice simulating volumetric squish, bounce, and elastic recovery.
#[derive(Component, Debug, Clone)]
pub struct SoftBodyLattice {
    pub points: Vec<SpringPoint>,
    pub stiffness: f32,
    pub damping: f32,
    pub half_extents: Vec3,
    /// Edge indices (i, j, rest_length) connecting adjacent corners of the cube lattice
    pub edge_springs: Vec<(usize, usize, f32)>,
    /// Shear / diagonal springs (i, j, rest_length) to resist diagonal shear
    pub shear_springs: Vec<(usize, usize, f32)>,
}

impl SoftBodyLattice {
    /// Creates an 8-point cuboid spring-mass lattice with structural edge and cross-shear springs.
    pub fn new_cube(half_extents: Vec3, stiffness: f32, damping: f32) -> Self {
        let mut points = Vec::with_capacity(8);
        for &x in &[-half_extents.x, half_extents.x] {
            for &y in &[-half_extents.y, half_extents.y] {
                for &z in &[-half_extents.z, half_extents.z] {
                    let offset = Vec3::new(x, y, z);
                    points.push(SpringPoint {
                        position: offset,
                        original_offset: offset,
                        velocity: Vec3::ZERO,
                        mass: 0.1,
                    });
                }
            }
        }

        // Generate edge springs (pairs differing in exactly 1 axis)
        let mut edge_springs = Vec::new();
        let mut shear_springs = Vec::new();
        for i in 0..8 {
            for j in (i + 1)..8 {
                let diff_bits = ((i as u32) ^ (j as u32)).count_ones();
                let rest_len = points[i].original_offset.distance(points[j].original_offset);
                if diff_bits == 1 {
                    edge_springs.push((i, j, rest_len));
                } else if diff_bits == 2 || diff_bits == 3 {
                    shear_springs.push((i, j, rest_len));
                }
            }
        }

        Self {
            points,
            stiffness,
            damping,
            half_extents,
            edge_springs,
            shear_springs,
        }
    }

    /// Applies a uniform linear impulse across all lattice point masses.
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        for pt in &mut self.points {
            pt.velocity += impulse / pt.mass;
        }
    }

    /// Applies an impulse to a specific corner point.
    pub fn apply_point_impulse(&mut self, point_index: usize, impulse: Vec3) {
        if let Some(pt) = self.points.get_mut(point_index) {
            pt.velocity += impulse / pt.mass;
        }
    }

    /// Applies cartoon squash along the given axis and complementary stretch perpendicular to it (preserving volume).
    pub fn apply_directional_squash(&mut self, axis: Vec3, squash_amount: f32) {
        let norm_axis = axis.normalize_or_zero();
        if norm_axis == Vec3::ZERO {
            return;
        }

        for pt in &mut self.points {
            let axial_dist = pt.original_offset.dot(norm_axis);
            let perp_vec = pt.original_offset - norm_axis * axial_dist;

            // Compress along axial direction
            let axial_disp = -norm_axis * (axial_dist * squash_amount);
            // Bulge outwards perpendicular to axis to preserve volume (isochoric squash & stretch)
            let perp_disp = perp_vec * (squash_amount * 0.5);

            pt.position += axial_disp + perp_disp;
            pt.velocity += (axial_disp + perp_disp) * 4.0;
        }
    }

    /// Calculates the maximum displacement of any corner point from its rest position.
    pub fn max_displacement(&self) -> f32 {
        self.points
            .iter()
            .map(|pt| (pt.position - pt.original_offset).length())
            .fold(0.0, f32::max)
    }

    /// Steps the internal spring-mass physics forward in time.
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        // Clamp large dt to prevent numerical explosion in low-framerate environments
        let sub_steps = 4;
        let sub_dt = (dt / sub_steps as f32).min(0.016);

        for _ in 0..sub_steps {
            // 1. Accumulate forces per point
            let mut forces = vec![Vec3::ZERO; self.points.len()];

            // Anchor forces back to original local offset (shape retention)
            for (i, pt) in self.points.iter().enumerate() {
                let disp = pt.position - pt.original_offset;
                let anchor_force = -self.stiffness * disp;
                let damp_force = -self.damping * pt.velocity;
                forces[i] += anchor_force + damp_force;
            }

            // Edge springs
            let edge_k = self.stiffness * 0.4;
            let edge_c = self.damping * 0.2;
            for &(i, j, rest_len) in &self.edge_springs {
                let delta = self.points[j].position - self.points[i].position;
                let current_len = delta.length();
                if current_len > 1e-5 {
                    let dir = delta / current_len;
                    let stretch = current_len - rest_len;
                    let rel_vel = self.points[j].velocity - self.points[i].velocity;
                    let f = dir * (edge_k * stretch + edge_c * rel_vel.dot(dir));
                    forces[i] += f;
                    forces[j] -= f;
                }
            }

            // Shear / diagonal cross springs
            let shear_k = self.stiffness * 0.15;
            for &(i, j, rest_len) in &self.shear_springs {
                let delta = self.points[j].position - self.points[i].position;
                let current_len = delta.length();
                if current_len > 1e-5 {
                    let dir = delta / current_len;
                    let stretch = current_len - rest_len;
                    let f = dir * (shear_k * stretch);
                    forces[i] += f;
                    forces[j] -= f;
                }
            }

            // 2. Integrate velocities and positions
            for (i, pt) in self.points.iter_mut().enumerate() {
                let accel = forces[i] / pt.mass;
                pt.velocity += accel * sub_dt;
                pt.position += pt.velocity * sub_dt;
            }
        }
    }
}

/// Stores reference un-deformed geometry for dynamic vertex deformation of a 3D mesh.
#[derive(Component, Debug, Clone)]
pub struct SoftBodyDeformableMesh {
    pub original_positions: Vec<Vec3>,
    pub original_normals: Vec<Vec3>,
    pub half_extents: Vec3,
    pub deformation_scale: f32,
}

impl SoftBodyDeformableMesh {
    /// Extracts initial rest positions and normals from a Bevy Mesh.
    pub fn from_mesh(mesh: &Mesh, half_extents: Vec3) -> Option<Self> {
        let pos_attr = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?;
        let norm_attr = mesh.attribute(Mesh::ATTRIBUTE_NORMAL);

        let positions: Vec<Vec3> = match pos_attr {
            VertexAttributeValues::Float32x3(slice) => {
                slice.iter().map(|&[x, y, z]| Vec3::new(x, y, z)).collect()
            }
            _ => return None,
        };

        let normals: Vec<Vec3> = if let Some(VertexAttributeValues::Float32x3(slice)) = norm_attr {
            slice.iter().map(|&[x, y, z]| Vec3::new(x, y, z)).collect()
        } else {
            vec![Vec3::Y; positions.len()]
        };

        Some(Self {
            original_positions: positions,
            original_normals: normals,
            half_extents,
            deformation_scale: 1.0,
        })
    }

    /// Evaluates trilinear free-form deformation (FFD) mapping the lattice corner displacements
    /// onto each mesh vertex, writing the results into the target Mesh's vertex buffers.
    pub fn apply_deformation(&self, lattice: &SoftBodyLattice, mesh: &mut Mesh) {
        if lattice.points.len() < 8 {
            return;
        }

        // Cache corner displacements
        // Corner mapping: index = (xi << 2) | (yi << 1) | zi
        let mut displacements = [Vec3::ZERO; 8];
        for i in 0..8 {
            displacements[i] = lattice.points[i].position - lattice.points[i].original_offset;
        }

        let hx = self.half_extents.x.max(0.001);
        let hy = self.half_extents.y.max(0.001);
        let hz = self.half_extents.z.max(0.001);

        let mut deformed_positions = Vec::with_capacity(self.original_positions.len());
        let mut deformed_normals = Vec::with_capacity(self.original_normals.len());

        for (v0, n0) in self.original_positions.iter().zip(self.original_normals.iter()) {
            // Normalized coordinate [0, 1] within bounding box
            let u = ((v0.x + hx) / (2.0 * hx)).clamp(0.0, 1.0);
            let v = ((v0.y + hy) / (2.0 * hy)).clamp(0.0, 1.0);
            let w = ((v0.z + hz) / (2.0 * hz)).clamp(0.0, 1.0);

            // Trilinear interpolation of corner displacements
            let mut interp_disp = Vec3::ZERO;
            for xi in 0..=1 {
                let wx = if xi == 1 { u } else { 1.0 - u };
                for yi in 0..=1 {
                    let wy = if yi == 1 { v } else { 1.0 - v };
                    for zi in 0..=1 {
                        let wz = if zi == 1 { w } else { 1.0 - w };
                        let idx = (xi << 2) | (yi << 1) | zi;
                        interp_disp += displacements[idx] * (wx * wy * wz);
                    }
                }
            }

            let new_pos = *v0 + interp_disp * self.deformation_scale;
            deformed_positions.push([new_pos.x, new_pos.y, new_pos.z]);

            // Perturb normal slightly with the deformation gradient
            let perturbed_normal = (*n0 + interp_disp * 0.3).normalize_or_zero();
            let final_norm = if perturbed_normal != Vec3::ZERO {
                perturbed_normal
            } else {
                *n0
            };
            deformed_normals.push([final_norm.x, final_norm.y, final_norm.z]);
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, deformed_positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, deformed_normals);
    }
}

/// System advancing internal spring-mass physics for all active soft bodies.
pub fn update_soft_bodies_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut query: Query<&mut SoftBodyLattice>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Some(time) = time else {
        return;
    };
    let dt = time.delta_secs();
    for mut lattice in &mut query {
        lattice.step(dt);
    }
}

/// System applying trilinear mesh vertex deformation to Bevy Mesh assets.
pub fn update_soft_body_meshes_system(
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    query: Query<(&SoftBodyLattice, &SoftBodyDeformableMesh, &Mesh3d)>,
    meshes: Option<ResMut<Assets<Mesh>>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Some(mut meshes) = meshes else {
        return;
    };

    for (lattice, deformable, mesh_handle) in &query {
        if let Some(mut mesh) = meshes.get_mut(mesh_handle) {
            deformable.apply_deformation(lattice, &mut *mesh);
        }
    }
}
