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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_body_spring_oscillation_and_damping() {
        let mut lattice = SoftBodyLattice::new_cube(Vec3::splat(0.5), 50.0, 5.0);

        lattice.apply_impulse(Vec3::new(2.0, 0.0, 0.0));

        for _ in 0..10 {
            lattice.step(0.01);
        }

        let pt = &lattice.points[0];
        assert!(pt.position.x > pt.original_offset.x);

        for _ in 0..200 {
            lattice.step(0.01);
        }
        let pt_rested = &lattice.points[0];
        assert!(
            (pt_rested.position.x - pt_rested.original_offset.x).abs() < 0.05,
            "Damped lattice should settle back near rest position"
        );
    }

    #[test]
    fn test_soft_body_mesh_deformation_renderer() {
        let base_cuboid = Cuboid::new(1.0, 1.0, 1.0);
        let mut mesh = Mesh::from(base_cuboid);
        let half_extents = Vec3::splat(0.5);

        let deformable = SoftBodyDeformableMesh::from_mesh(&mesh, half_extents)
            .expect("Deformable mesh extraction must succeed");

        assert_eq!(deformable.original_positions.len(), 24, "Cube mesh has 24 vertices (6 faces * 4 vertices)");
        assert_eq!(deformable.half_extents, half_extents);

        let mut lattice = SoftBodyLattice::new_cube(half_extents, 120.0, 8.0);
        assert_eq!(lattice.points.len(), 8, "Cube lattice must have 8 corner spring points");
        assert_eq!(lattice.edge_springs.len(), 12, "Cube must have 12 edge springs");

        lattice.apply_directional_squash(Vec3::Y, 0.4);

        for pt in &lattice.points {
            if pt.original_offset.y > 0.0 {
                assert!(
                    pt.position.y < pt.original_offset.y,
                    "Top points must be compressed downward: got {}, original {}",
                    pt.position.y,
                    pt.original_offset.y
                );
            } else {
                assert!(
                    pt.position.y > pt.original_offset.y,
                    "Bottom points must be compressed upward: got {}, original {}",
                    pt.position.y,
                    pt.original_offset.y
                );
            }
            let orig_lat = (pt.original_offset.x.powi(2) + pt.original_offset.z.powi(2)).sqrt();
            let cur_lat = (pt.position.x.powi(2) + pt.position.z.powi(2)).sqrt();
            assert!(
                cur_lat > orig_lat,
                "Lateral radius must expand for volume preservation: got {cur_lat}, original {orig_lat}"
            );
        }

        deformable.apply_deformation(&lattice, &mut mesh);

        let pos_attr = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let VertexAttributeValues::Float32x3(deformed_positions) = pos_attr else {
            panic!("ATTRIBUTE_POSITION must be Float32x3");
        };

        for (orig, def) in deformable.original_positions.iter().zip(deformed_positions.iter()) {
            if orig.y > 0.49 {
                assert!(
                    def[1] < orig.y,
                    "Top vertices must be squashed downwards: deformed {}, original {}",
                    def[1],
                    orig.y
                );
            }
        }

        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let handle = meshes.add(mesh);
        app.insert_resource(meshes);

        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);

        let entity = app.world_mut().spawn((
            Mesh3d(handle.clone()),
            lattice,
            deformable,
        )).id();

        app.add_systems(Update, (update_soft_bodies_system, update_soft_body_meshes_system).chain());

        for _ in 0..10 {
            app.update();
        }

        let lat = app.world().get::<SoftBodyLattice>(entity).unwrap();
        assert!(lat.max_displacement() > 0.0, "Lattice must still be oscillating with elastic restitution");

        let mesh_assets = app.world().resource::<Assets<Mesh>>();
        let updated_mesh = mesh_assets.get(&handle).unwrap();
        assert!(updated_mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
    }
}
