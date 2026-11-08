use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::render::mesh::{Mesh, PrimitiveTopology, Indices};
use bevy::asset::RenderAssetUsages;
use super::text_renderer::{render_text_to_image, update_text_image_data};
use crate::elf_game::{
    assets::{palette::*, WorkshopMaterials},
    core::WorkshopQuota,
    moose::MooseHunger,
};

#[derive(Component, Debug, Clone)]
pub struct DiegeticMonitor;

#[derive(Component, Debug, Clone)]
pub struct DiegeticText;

/// Marks the 3D text quad on the CRT monitor for dynamic content updates
#[derive(Component, Debug, Clone)]
pub struct CrtMonitorTextDisplay {
    pub image_handle: Handle<Image>,
    pub last_content: String,
}

/// Marks 3D text quads spawned on in-world directional signs
#[derive(Component, Debug, Clone)]
pub struct DirectionalSign3dText;

/// Tracks instruction message cycling on the curved CRT monitor
#[derive(Component, Debug, Clone)]
pub struct CrtInstructionMonitor {
    pub timer: Timer,
    pub current_page: usize,
    pub total_pages: usize,
}

impl Default for CrtInstructionMonitor {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(4.0, TimerMode::Repeating),
            current_page: 0,
            total_pages: 4,
        }
    }
}

/// Dynamic horizontal scanline sweep bar that travels down the CRT face
#[derive(Component, Debug, Clone)]
pub struct CrtScanlineSweep {
    pub speed: f32,
    pub min_y: f32,
    pub max_y: f32,
}

/// Identifies in-world signs and station labels that direct the player
#[derive(Component, Debug, Clone)]
pub struct DirectionalSign {
    pub station: String,
    pub label: String,
}

/// Generates a vintage curved CRT monitor glass mesh bulging forward (+Z) towards the player
pub fn create_curved_crt_mesh(width: f32, height: f32, bulge: f32, res_x: usize, res_y: usize) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((res_x + 1) * (res_y + 1));
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((res_x + 1) * (res_y + 1));
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((res_x + 1) * (res_y + 1));
    let mut indices: Vec<u32> = Vec::with_capacity(res_x * res_y * 6);

    let half_w = width * 0.5;
    let half_h = height * 0.5;

    for j in 0..=res_y {
        let v = j as f32 / res_y as f32;
        let y = half_h - v * height;
        let ny = (y / half_h).clamp(-1.0, 1.0);

        for i in 0..=res_x {
            let u = i as f32 / res_x as f32;
            let x = -half_w + u * width;
            let nx = (x / half_w).clamp(-1.0, 1.0);

            // Subtle spherical/cylindrical convex bulge in +Z (forward towards player)
            let curve_factor = (1.0 - 0.70 * nx * nx - 0.30 * ny * ny).max(0.0);
            let z = bulge * curve_factor;

            positions.push([x, y, z]);
            uvs.push([u, v]);

            // Normal vector pointing outward from the curved glass surface
            let nx_norm = 1.4 * nx * bulge / half_w.max(0.01);
            let ny_norm = 0.6 * ny * bulge / half_h.max(0.01);
            let normal = Vec3::new(nx_norm, ny_norm, 1.0).normalize();
            normals.push([normal.x, normal.y, normal.z]);
        }
    }

    let stride = (res_x + 1) as u32;
    for j in 0..res_y as u32 {
        for i in 0..res_x as u32 {
            let top_left = j * stride + i;
            let top_right = top_left + 1;
            let bottom_left = (j + 1) * stride + i;
            let bottom_right = bottom_left + 1;

            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Generates a curved scanline grille mesh conforming to the CRT glass curvature
pub fn create_curved_crt_scanlines_mesh(
    width: f32,
    height: f32,
    bulge: f32,
    num_lines: usize,
    line_thickness: f32,
) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let half_w = width * 0.5;
    let half_h = height * 0.5;
    let seg_x = 16;

    for line_idx in 0..num_lines {
        let v_center = (line_idx as f32 + 0.5) / num_lines as f32;
        let y_mid = half_h - v_center * height;
        let y_top = y_mid + line_thickness * 0.5;
        let y_bot = y_mid - line_thickness * 0.5;

        let base_vertex = positions.len() as u32;

        for s in 0..=seg_x {
            let u = s as f32 / seg_x as f32;
            let x = -half_w + u * width;
            let nx = (x / half_w).clamp(-1.0, 1.0);

            for &y in &[y_top, y_bot] {
                let ny = (y / half_h).clamp(-1.0, 1.0);
                let curve = (1.0 - 0.70 * nx * nx - 0.30 * ny * ny).max(0.0);
                // Placed 3mm in front of the curved glass face to avoid z-fighting
                let z = bulge * curve + 0.003;

                positions.push([x, y, z]);
                normals.push([0.0, 0.0, 1.0]);
                uvs.push([u, v_center]);
            }
        }

        for s in 0..seg_x as u32 {
            let i0 = base_vertex + s * 2;
            let i1 = i0 + 1;
            let i2 = i0 + 2;
            let i3 = i0 + 3;

            indices.push(i0);
            indices.push(i1);
            indices.push(i2);

            indices.push(i2);
            indices.push(i1);
            indices.push(i3);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Spawns in-world instruction signs and the curved CRT monitor with cycling instructions
pub fn spawn_diegetic_screens_and_signs(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut images: Option<ResMut<Assets<Image>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let has_assets = meshes.is_some() && (materials.is_some() || workshop_mats.is_some());

    // 1. Materials setup (when running in graphical environment)
    let (casing_mat, screen_mat, scanline_mat, sweep_mat, wood_sign_mat, brass_sign_mat, metal_sign_mat) = if let Some(ref wm) = workshop_mats {
        (
            Some(wm.cast_iron.clone()),
            Some(wm.crt_screen.clone()),
            Some(wm.crt_scanlines.clone()),
            Some(wm.crt_sweep.clone()),
            Some(wm.pine_sign.clone()),
            Some(wm.brass_plaque.clone()),
            Some(wm.sheet_metal.clone()),
        )
    } else if let Some(ref mut mats) = materials {
        (
            Some(mats.add(StandardMaterial {
                base_color: CAST_IRON,
                perceptual_roughness: 0.75,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: CRT_TUBE_DARK,
                emissive: LinearRgba::rgb(0.02, 0.09, 0.03),
                perceptual_roughness: 0.2,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: CRT_SCANLINE_SHADOW,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.4,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: CRT_SWEEP_GLOW,
                emissive: LinearRgba::rgb(0.05, 0.25, 0.08),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: PINE_SIGN_BOARD,
                perceptual_roughness: 0.85,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: BRASS_PLAQUE,
                metallic: 0.8,
                perceptual_roughness: 0.3,
                ..default()
            })),
            Some(mats.add(StandardMaterial {
                base_color: INDUSTRIAL_DARK,
                metallic: 0.7,
                perceptual_roughness: 0.45,
                ..default()
            })),
        )
    } else {
        (None, None, None, None, None, None, None)
    };

    // 2. Workshop Present Instruction Display (Curved CRT Monitor: X = 0.0, Y = 2.2, Z = -5.0)
    let mut monitor_cmd = commands.spawn((
        Transform::from_xyz(0.0, 2.2, -5.0),
        DiegeticMonitor,
        CrtInstructionMonitor::default(),
        Name::new("WorkshopTaskCrtMonitor"),
    ));

    if has_assets {
        if let (Some(m), Some(casing)) = (&mut meshes, casing_mat.clone()) {
            monitor_cmd.insert((
                Mesh3d(m.add(Cuboid::new(2.4, 1.4, 0.32))),
                MeshMaterial3d(casing),
            ));
        }
    }

    let initial_crt_text = "┌── PRESENT INSTRUCTIONS ─────────── [ PAGE 1/4 ] ──┐\n\
                            │ 1. GRAB RAW TOY CUBE FROM CONVEYOR BELT [Q / E]   │\n\
                            │ 2. CARRY TO MAIN WORKBENCH TABLE TO ASSEMBLE      │\n\
                            │ 3. APPLY WRAPPING PAPER, TAPE FLAPS, TIE BOW      │\n\
                            │ 4. PLACE FINISHED GIFT ON SENDING PLATFORM        │\n\
                            │ 5. PRESS BIG RED BUTTON TO VACUUM SEND            │\n\
                            └───────────────────────────────────────────────────┘\n\
                            ── CRT 15.75 kHz ────────────────────── REFRESH: 60Hz ─\n\
                            QUOTA: 0 / 5 [TIER 1]  |  MOOSE: 0/60 (CALM)";

    monitor_cmd.with_children(|parent| {
        // Curved CRT screen glass pane
        if let (Some(m), Some(screen)) = (&mut meshes, screen_mat) {
            let curved_mesh = create_curved_crt_mesh(2.1, 1.1, 0.08, 24, 16);
            parent.spawn((
                Mesh3d(m.add(curved_mesh)),
                MeshMaterial3d(screen),
                Transform::from_xyz(0.0, 0.0, 0.14),
                Name::new("CrtCurvedScreenGlass"),
            ));
        }

        // Curved CRT scanlines grille overlay
        if let (Some(m), Some(scanlines)) = (&mut meshes, scanline_mat) {
            let scanlines_mesh = create_curved_crt_scanlines_mesh(2.1, 1.1, 0.08, 28, 0.014);
            parent.spawn((
                Mesh3d(m.add(scanlines_mesh)),
                MeshMaterial3d(scanlines),
                Transform::from_xyz(0.0, 0.0, 0.14),
                Name::new("CrtCurvedScanlinesGrille"),
            ));
        }

        // Moving scanline sweep cathode refresh bar
        if let (Some(m), Some(sweep)) = (&mut meshes, sweep_mat) {
            parent.spawn((
                Mesh3d(m.add(Cuboid::new(2.1, 0.035, 0.01))),
                MeshMaterial3d(sweep),
                Transform::from_xyz(0.0, 0.45, 0.22),
                CrtScanlineSweep {
                    speed: 0.35,
                    min_y: -0.48,
                    max_y: 0.48,
                },
                Name::new("CrtScanlineSweepBar"),
            ));
        }

        // Genuine 3D Text Quad (rendered by Camera3d with CRT phosphor glowing texture)
        if let (Some(m), Some(mats), Some(imgs)) = (&mut meshes, &mut materials, &mut images) {
            let crt_img = render_text_to_image(
                initial_crt_text,
                26.0,
                CRT_PHOSPHOR_GREEN,
                Color::srgba(0.015, 0.05, 0.02, 0.95), // dark green phosphor tube background
                1024,
                512,
                false,
            );
            let img_handle = imgs.add(crt_img);

            let crt_text_mat = mats.add(StandardMaterial {
                base_color_texture: Some(img_handle.clone()),
                emissive: LinearRgba::rgb(0.04, 0.28, 0.08),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            parent.spawn((
                Mesh3d(m.add(Rectangle::new(2.0, 1.0))),
                MeshMaterial3d(crt_text_mat),
                Transform::from_xyz(0.0, 0.0, 0.225),
                CrtMonitorTextDisplay {
                    image_handle: img_handle,
                    last_content: initial_crt_text.to_string(),
                },
                Name::new("CrtMonitor3dTextQuad"),
            ));
        }

        // CRT Monospace Text Display (retained for headless test queries)
        parent.spawn((
            Text2d::new(initial_crt_text),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(CRT_PHOSPHOR_GREEN),
            Transform::from_xyz(0.0, 0.0, 0.23).with_scale(Vec3::splat(0.0075)),
            DiegeticText,
            Name::new("CrtMonitorText"),
        ));
    });

    // 3. Main Workbench Wrapping Station Plaque (Attached to front of workbench: X = 0.0, Y = 0.65, Z = -3.05)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        brass_sign_mat.clone(),
        Transform::from_xyz(0.0, 0.65, -3.05),
        Vec2::new(2.6, 0.28),
        "WRAPPING STATION\nASSEMBLE, TAPE & TIE PRESENTS HERE",
        Color::srgb(1.0, 0.92, 0.70),
        22.0,
        0.0065,
        "WrappingWorkbench",
        "WrappingStationPlaque",
    );

    // 4. Present Sending Platform Standing Sign (Standing post at X = 5.5, Y = 1.4, Z = -5.2)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        wood_sign_mat.clone(),
        Transform::from_xyz(5.5, 1.4, -5.2),
        Vec2::new(2.2, 0.65),
        "PRESENT SENDING PLATFORM\n1. PLACE WRAPPED GIFT ON CIRCLE\n2. HIT RED BUTTON TO SEND",
        Color::srgb(1.0, 0.85, 0.2),
        24.0,
        0.0075,
        "PresentSendingPlatform",
        "DispatchStationSign",
    );

    // 5. Present Sending Platform Floor Caution Border Text (X = 5.5, Y = 0.12, Z = -2.85, flat on floor)
    let mut floor_label_cmd = commands.spawn((
        Transform::from_xyz(5.5, 0.12, -2.85)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        DirectionalSign {
            station: "PresentSendingPlatform".to_string(),
            label: "▲ LOAD WRAPPED PRESENTS HERE ▲".to_string(),
        },
        Name::new("DispatchPadFloorLabel"),
    ));
    floor_label_cmd.with_children(|parent| {
        // Genuine 3D Text Quad on the floor pad
        if let (Some(m), Some(mats), Some(imgs)) = (&mut meshes, &mut materials, &mut images) {
            let img = render_text_to_image(
                "▲ LOAD WRAPPED PRESENTS HERE ▲",
                32.0,
                Color::srgb(1.0, 0.90, 0.1),
                Color::srgba(0.0, 0.0, 0.0, 0.0),
                1024,
                128,
                true,
            );
            let img_handle = imgs.add(img);

            let text_mat = mats.add(StandardMaterial {
                base_color_texture: Some(img_handle),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            parent.spawn((
                Mesh3d(m.add(Rectangle::new(2.4, 0.32))),
                MeshMaterial3d(text_mat),
                Transform::from_xyz(0.0, 0.0, 0.012),
                DirectionalSign3dText,
                Name::new("DispatchPadFloor3dText"),
            ));
        }

        parent.spawn((
            Text2d::new("▲ LOAD WRAPPED PRESENTS HERE ▲"),
            TextFont {
                font_size: FontSize::Px(26.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.90, 0.1)),
            Transform::from_xyz(0.0, 0.0, 0.01).with_scale(Vec3::splat(0.008)),
            Name::new("DispatchPadFloorLabelText"),
        ));
    });

    // 6. Pneumatic Vacuum Send Button Pedestal Sign (X = 1.8, Y = 0.72, Z = -1.62, angled towards player)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        brass_sign_mat.clone(),
        Transform::from_xyz(1.8, 0.72, -1.62).with_rotation(Quat::from_rotation_x(-0.4)),
        Vec2::new(0.55, 0.22),
        "PNEUMATIC VACUUM\n[ PRESS RED BUTTON ]",
        Color::srgb(1.0, 0.25, 0.25),
        20.0,
        0.006,
        "SendPlatformButton",
        "SendButtonSign",
    );

    // 7. Moose Snacks Jar Plaque on Table (X = 1.4, Y = 0.82, Z = -3.7, right in front of the jar)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        wood_sign_mat.clone(),
        Transform::from_xyz(1.4, 0.82, -3.7),
        Vec2::new(0.70, 0.20),
        "MOOSE SNACKS\nGRAB [Q/E] & CARRY TO TROUGH",
        Color::srgb(0.3, 1.0, 0.5),
        20.0,
        0.006,
        "MooseSnackJar",
        "MooseSnacksJarLabel",
    );

    // 8. Moose Feeding Trough Embossed Sign (X = -2.0, Y = 0.45, Z = -11.35, on front of trough facing +Z)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        metal_sign_mat.clone(),
        Transform::from_xyz(-2.0, 0.45, -11.35),
        Vec2::new(1.8, 0.25),
        "MOOSE FEEDING TROUGH\nDEPOSIT SNACKS HERE BEFORE HUNGER HITS 60",
        Color::srgb(0.4, 0.9, 1.0),
        22.0,
        0.0065,
        "MooseFeedingTrough",
        "MooseTroughSign",
    );

    // 9. Moose Warning Sign next to Window (X = -2.6, Y = 1.8, Z = -12.8, on wooden post)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        wood_sign_mat.clone(),
        Transform::from_xyz(-2.6, 1.8, -12.8),
        Vec2::new(1.8, 1.0),
        "WARNING: HUNGRY MR. MOOSE\n• TAKE SNACKS FROM WORKBENCH JAR\n• DROP IN TROUGH TO RESET HUNGER\n• IF HUNGER HITS 60: RAMPAGE BREACH!",
        Color::srgb(0.95, 0.2, 0.2),
        24.0,
        0.0075,
        "MooseWindow",
        "MooseWarningSignBoard",
    );

    // 10. Conveyor Belt Side Rail Sign (X = -4.0, Y = 1.35, Z = 2.45, facing -Z into room)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        metal_sign_mat.clone(),
        Transform::from_xyz(-4.0, 1.35, 2.45).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        Vec2::new(2.2, 0.32),
        "UNWRAPPED TOYS CONVEYOR\n[Q / E] PICK UP RAW TOY CUBES",
        Color::srgb(0.3, 0.9, 1.0),
        22.0,
        0.0065,
        "MainConveyorBelt",
        "ConveyorIntakeSign",
    );

    // 11. Recessed Floor Chute Funnel Sign (X = 9.6, Y = 0.50, Z = 2.1, facing -Z into room)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        metal_sign_mat.clone(),
        Transform::from_xyz(9.6, 0.50, 2.1).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        Vec2::new(1.8, 0.32),
        "OVERFLOW CHUTE\nUNWRAPPED TOYS ARE RECYCLED HERE",
        Color::srgb(1.0, 0.75, 0.2),
        20.0,
        0.0065,
        "ConveyorDespawnChute",
        "ChuteRecycleSign",
    );

    // 12. Overhead Suction Funnel Brass Sign (X = 5.5, Y = 3.5, Z = -3.9, facing +Z)
    spawn_directional_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        brass_sign_mat,
        Transform::from_xyz(5.5, 3.5, -3.9),
        Vec2::new(1.6, 0.24),
        "PNEUMATIC VACUUM FUNNEL\nSUCKS UP COMPLETED GIFTS",
        Color::srgb(1.0, 0.88, 0.5),
        20.0,
        0.006,
        "OverheadSuctionFunnel",
        "OverheadFunnelSign",
    );
}

/// Helper function to spawn directional signs with backing plate and child text
fn spawn_directional_sign(
    commands: &mut Commands,
    meshes: &mut Option<ResMut<Assets<Mesh>>>,
    materials: &mut Option<ResMut<Assets<StandardMaterial>>>,
    images: &mut Option<ResMut<Assets<Image>>>,
    board_mat: Option<Handle<StandardMaterial>>,
    transform: Transform,
    size: Vec2,
    text_content: &str,
    text_color: Color,
    font_size: f32,
    text_scale: f32,
    station: &str,
    name: &str,
) {
    let mut sign_cmd = commands.spawn((
        transform,
        DirectionalSign {
            station: station.to_string(),
            label: text_content.to_string(),
        },
        Name::new(name.to_string()),
    ));

    if let (Some(m), Some(mat)) = (meshes.as_mut(), board_mat) {
        sign_cmd.insert((
            Mesh3d(m.add(Cuboid::new(size.x, size.y, 0.05))),
            MeshMaterial3d(mat),
        ));
    }

    sign_cmd.with_children(|parent| {
        // 1. Genuine 3D Text Quad Mesh & Texture (rendered by Camera3d in the 3D pass)
        if let (Some(m), Some(mats), Some(imgs)) = (meshes.as_mut(), materials.as_mut(), images.as_mut()) {
            let aspect = size.x / size.y;
            let (tex_w, tex_h, font_pt) = if aspect > 5.0 {
                (1024, 128, 28.0)
            } else if aspect > 2.5 {
                (1024, 256, 28.0)
            } else if aspect > 1.4 {
                (512, 256, 26.0)
            } else {
                (512, 512, 28.0)
            };

            let img = render_text_to_image(
                text_content,
                font_pt,
                text_color,
                Color::srgba(0.0, 0.0, 0.0, 0.0), // transparent background so wood/brass/metal plate shows
                tex_w,
                tex_h,
                true, // center-aligned on sign
            );
            let img_handle = imgs.add(img);

            let text_mat = mats.add(StandardMaterial {
                base_color_texture: Some(img_handle),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.6,
                ..default()
            });

            parent.spawn((
                Mesh3d(m.add(Rectangle::new(size.x * 0.94, size.y * 0.88))),
                MeshMaterial3d(text_mat),
                Transform::from_xyz(0.0, 0.0, 0.028),
                DirectionalSign3dText,
                Name::new(format!("{}3dText", name)),
            ));
        }

        // 2. Retain Text2d for logical queries and headless test assertions
        parent.spawn((
            Text2d::new(text_content),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(text_color),
            Transform::from_xyz(0.0, 0.0, 0.035).with_scale(Vec3::splat(text_scale)),
            Name::new(format!("{}Text", name)),
        ));
    });
}

/// Updates in-world diegetic monitors dynamically, advancing instruction pages and displaying live stats
pub fn update_diegetic_monitor_text(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    quota: Option<Res<WorkshopQuota>>,
    moose: Option<Res<MooseHunger>>,
    mut q_monitor: Query<&mut CrtInstructionMonitor>,
    mut q_text: Query<&mut Text2d, With<DiegeticText>>,
    mut q_crt_display: Query<&mut CrtMonitorTextDisplay>,
    mut images: Option<ResMut<Assets<Image>>>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let mut page = 0;
    if let Ok(mut monitor) = q_monitor.single_mut() {
        if let Some(ref time) = time {
            monitor.timer.tick(time.delta());
            if monitor.timer.just_finished() {
                monitor.current_page = (monitor.current_page + 1) % monitor.total_pages;
            }
        }
        page = monitor.current_page;
    }

    let (delivered, target, tier) = quota
        .map(|q| (q.presents_delivered, q.target_quota, q.tier))
        .unwrap_or((0, 5, 1));

    let (moose_val, is_breached) = moose
        .map(|m| (m.value, m.is_breached))
        .unwrap_or((0.0, false));

    let moose_status = if is_breached {
        "BREACH!! RUN!!"
    } else if moose_val >= 50.0 {
        "AGITATED (FEED NOW!)"
    } else if moose_val >= 20.0 {
        "PEEKING IN WINDOW"
    } else {
        "CALM (FED)"
    };

    let text_content = if is_breached {
        format!(
            "╔═════════════════════════════════════════════════════╗\n\
             ║              *** EMERGENCY ALERT ***                ║\n\
             ║       MR. MOOSE HAS BREACHED THE WORKSHOP!          ║\n\
             ║       EVADE CHARGING MOOSE! STAY CLEAR!             ║\n\
             ║   REPAIR & PACIFY AFTER MOOSE WAYPOINT RETREAT      ║\n\
             ╚═════════════════════════════════════════════════════╝\n\
             ── CRT 15.75 kHz ────────────────────── REFRESH: 60Hz ─\n\
             QUOTA: {} / {} [TIER {}]  |  MOOSE: {:.0}/60 (BREACH!)",
            delivered, target, tier, moose_val
        )
    } else {
        match page {
            0 => format!(
                "┌── PRESENT INSTRUCTIONS ─────────── [ PAGE 1/4 ] ──┐\n\
                 │ 1. GRAB RAW TOY CUBE FROM CONVEYOR BELT [Q / E]   │\n\
                 │ 2. CARRY TO MAIN WORKBENCH TABLE TO ASSEMBLE      │\n\
                 │ 3. APPLY WRAPPING PAPER, TAPE FLAPS, TIE BOW      │\n\
                 │ 4. PLACE FINISHED GIFT ON SENDING PLATFORM        │\n\
                 │ 5. PRESS BIG RED BUTTON TO VACUUM SEND            │\n\
                 └───────────────────────────────────────────────────┘\n\
                 ── CRT 15.75 kHz ────────────────────── REFRESH: 60Hz ─\n\
                 QUOTA: {} / {} [TIER {}]  |  MOOSE: {:.0}/60 ({})",
                delivered, target, tier, moose_val, moose_status
            ),
            1 => format!(
                "┌── DISPATCH & VACUUM GUIDE ──────── [ PAGE 2/4 ] ──┐\n\
                 │ • CARRY WRAPPED PRESENTS TO THE CIRCULAR SEND PAD │\n\
                 │ • STAND CLEAR OF THE CEILING INTAKE FUNNEL        │\n\
                 │ • PRESS THE BIG RED BUTTON ON CONTROL PEDESTAL    │\n\
                 │ • PNEUMATIC VACUUM SUCKS GIFTS THROUGH LOOP TUBE  │\n\
                 │ • EACH DELIVERED PRESENT INCREMENTS YOUR QUOTA    │\n\
                 └───────────────────────────────────────────────────┘\n\
                 ── CRT 15.75 kHz ────────────────────── REFRESH: 60Hz ─\n\
                 QUOTA: {} / {} [TIER {}]  |  MOOSE: {:.0}/60 ({})",
                delivered, target, tier, moose_val, moose_status
            ),
            2 => format!(
                "┌── MR. MOOSE SAFETY PROTOCOL ────── [ PAGE 3/4 ] ──┐\n\
                 │ CAUTION: KEEP MR. MOOSE PACIFIED AT ALL TIMES!    │\n\
                 │ • HUNGER RISES CONTINUOUSLY OVER TIME             │\n\
                 │ • GRAB SMALL MOOSE SNACKS FROM WORKBENCH JAR      │\n\
                 │ • DROP SNACK IN FEEDING TROUGH OUTSIDE WINDOW     │\n\
                 │ • HE EATS AT 50-60 HUNGER; AT 60 HE RAMPAGES!     │\n\
                 └───────────────────────────────────────────────────┘\n\
                 ── CRT 15.75 kHz ────────────────────── REFRESH: 60Hz ─\n\
                 QUOTA: {} / {} [TIER {}]  |  MOOSE: {:.0}/60 ({})",
                delivered, target, tier, moose_val, moose_status
            ),
            _ => format!(
                "┌── ELF CONTROLS & BHOP GUIDE ────── [ PAGE 4/4 ] ──┐\n\
                 │ • [WASD]: MOVE / STRAFE  |  [MOUSE]: AIM LOOK     │\n\
                 │ • [Q] LEFT HAND TOGGLE   |  [E] RIGHT HAND TOGGLE │\n\
                 │ • [M1]: THROW HELD ITEM  |  [Q/E AGAIN]: DROP     │\n\
                 │ • [SPACE]: JUMP (TIME LANDING TO BHOP BOOST 1.4x) │\n\
                 │ • [TAB]: TOGGLE OPTIONS & TIMING DEBUG MENU       │\n\
                 └───────────────────────────────────────────────────┘\n\
                 ── CRT 15.75 kHz ────────────────────── REFRESH: 60Hz ─\n\
                 QUOTA: {} / {} [TIER {}]  |  MOOSE: {:.0}/60 ({})",
                delivered, target, tier, moose_val, moose_status
            ),
        }
    };

    // Update Text2d for test assertions & accessibility
    for mut text in &mut q_text {
        text.0 = text_content.clone();
    }

    // Update 3D CRT monitor texture in real-time
    for mut display in &mut q_crt_display {
        if display.last_content != text_content {
            display.last_content = text_content.clone();
            if let Some(ref mut img_assets) = images {
                if let Some(mut image) = img_assets.get_mut(&display.image_handle) {
                    if let Some(ref mut data) = image.data {
                        update_text_image_data(
                            &text_content,
                            26.0,
                            CRT_PHOSPHOR_GREEN,
                            Color::srgba(0.015, 0.05, 0.02, 0.95),
                            1024,
                            512,
                            false,
                            data,
                        );
                    }
                }
            }
        }
    }
}

/// Animates the CRT cathode ray scanline sweep bar downwards
pub fn animate_crt_scanlines(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_sweep: Query<(&mut Transform, &CrtScanlineSweep)>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    let Some(time) = time else { return };
    let dt = time.delta_secs();

    for (mut tf, sweep) in &mut q_sweep {
        tf.translation.y -= sweep.speed * dt;
        if tf.translation.y < sweep.min_y {
            tf.translation.y = sweep.max_y;
        }
    }
}
