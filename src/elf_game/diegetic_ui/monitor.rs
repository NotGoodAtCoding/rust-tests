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
    let (casing_mat, screen_mat, scanline_mat, sweep_mat, wood_sign_mat, brass_sign_mat, _metal_sign_mat) = if let Some(ref wm) = workshop_mats {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;

    #[test]
    fn test_curved_crt_mesh_and_scanlines_geometry() {
        let crt_mesh = create_curved_crt_mesh(2.1, 1.1, 0.08, 24, 16);
        let positions = match crt_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(p) => p,
            _ => panic!("Expected Float32x3 positions"),
        };

        let center_idx = 8 * (24 + 1) + 12;
        let center_pos = positions[center_idx];
        assert!(
            (center_pos[2] - 0.08).abs() < 0.005,
            "CRT center must bulge forward ~0.08m, got: {}",
            center_pos[2]
        );

        let corner_pos = positions[0];
        assert!(
            corner_pos[2] < 0.01,
            "CRT corner must have flat border Z ~ 0.0m, got: {}",
            corner_pos[2]
        );

        let scanline_mesh = create_curved_crt_scanlines_mesh(2.1, 1.1, 0.08, 28, 0.014);
        let sl_positions = match scanline_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(p) => p,
            _ => panic!("Expected Float32x3 positions for scanlines"),
        };
        assert!(!sl_positions.is_empty(), "Scanline mesh must contain vertices");
        let max_z = sl_positions.iter().map(|p| p[2]).fold(0.0f32, f32::max);
        assert!(
            (max_z - 0.083).abs() < 0.005,
            "Peak scanline bulge should match curved glass + 3mm offset (0.083m), got: {}",
            max_z
        );
    }

    #[test]
    fn test_crt_monitor_cyclic_instruction_messages() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        app.insert_resource(time);
        app.insert_resource(WorkshopQuota {
            presents_delivered: 2,
            target_quota: 5,
            tier: 1,
        });
        app.insert_resource(MooseHunger {
            value: 15.0,
            is_breached: false,
            ..default()
        });

        app.add_systems(Startup, spawn_diegetic_screens_and_signs);
        app.add_systems(Update, update_diegetic_monitor_text);
        app.update();

        {
            let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
            let monitor = q_mon.single(app.world()).unwrap();
            assert_eq!(monitor.current_page, 0, "Monitor should start at Page 0");
            assert_eq!(monitor.total_pages, 4, "Monitor should have 4 instructional pages");
        }

        {
            let mut q_txt = app.world_mut().query::<&Text2d>();
            let text_page0 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 1/4")).expect("Must find Page 1 text");
            assert!(text_page0.0.contains("PRESENT INSTRUCTIONS"));
            assert!(text_page0.0.contains("GRAB RAW TOY CUBE FROM CONVEYOR BELT"));
            assert!(text_page0.0.contains("QUOTA: 2 / 5 [TIER 1]"));
        }

        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(4050));
        }
        app.update();

        {
            let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
            let monitor = q_mon.single(app.world()).unwrap();
            assert_eq!(monitor.current_page, 1, "Monitor should cycle to Page 1");
        }
        {
            let mut q_txt = app.world_mut().query::<&Text2d>();
            let text_page1 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 2/4")).expect("Must find Page 2 text");
            assert!(text_page1.0.contains("DISPATCH & VACUUM GUIDE"));
            assert!(text_page1.0.contains("CARRY WRAPPED PRESENTS TO THE CIRCULAR SEND PAD"));
        }

        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(4050));
        }
        app.update();

        {
            let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
            let monitor = q_mon.single(app.world()).unwrap();
            assert_eq!(monitor.current_page, 2, "Monitor should cycle to Page 2");
        }
        {
            let mut q_txt = app.world_mut().query::<&Text2d>();
            let text_page2 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 3/4")).expect("Must find Page 3 text");
            assert!(text_page2.0.contains("MR. MOOSE SAFETY PROTOCOL"));
            assert!(text_page2.0.contains("GRAB SMALL MOOSE SNACKS FROM WORKBENCH JAR"));
        }

        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(4050));
        }
        app.update();

        {
            let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
            let monitor = q_mon.single(app.world()).unwrap();
            assert_eq!(monitor.current_page, 3, "Monitor should cycle to Page 3");
        }
        {
            let mut q_txt = app.world_mut().query::<&Text2d>();
            let text_page3 = q_txt.iter(app.world()).find(|t| t.0.contains("PAGE 4/4")).expect("Must find Page 4 text");
            assert!(text_page3.0.contains("ELF CONTROLS & BHOP GUIDE"));
            assert!(text_page3.0.contains("TIME LANDING TO BHOP BOOST"));
        }

        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(4050));
        }
        app.update();

        {
            let mut q_mon = app.world_mut().query::<&CrtInstructionMonitor>();
            let monitor = q_mon.single(app.world()).unwrap();
            assert_eq!(monitor.current_page, 0, "Monitor must wrap back to Page 0 in an infinite loop");
        }

        {
            let mut moose = app.world_mut().resource_mut::<MooseHunger>();
            moose.is_breached = true;
        }
        app.update();

        {
            let mut q_txt = app.world_mut().query::<&Text2d>();
            let text_breach = q_txt.iter(app.world()).find(|t| t.0.contains("EMERGENCY ALERT")).expect("Must find Emergency Alert");
            assert!(text_breach.0.contains("MR. MOOSE HAS BREACHED THE WORKSHOP"));
        }
    }

    #[test]
    fn test_in_world_directional_signs_and_object_labels() {
        let mut app = App::new();
        app.add_systems(Startup, spawn_diegetic_screens_and_signs);
        app.update();

        let signs: Vec<(String, String)> = {
            let mut q_signs = app.world_mut().query::<&DirectionalSign>();
            q_signs
                .iter(app.world())
                .map(|s| (s.station.clone(), s.label.clone()))
                .collect()
        };

        assert!(signs.len() >= 2, "Expected at least 2 directional signs/labels, found: {}", signs.len());

        let stations: Vec<&str> = signs.iter().map(|(st, _)| st.as_str()).collect();
        assert!(stations.contains(&"WrappingWorkbench"), "Must have Workbench Wrapping plaque");
        assert!(stations.contains(&"PresentSendingPlatform"), "Must have Present Sending Platform sign");

        let workbench_sign = signs.iter().find(|(st, _)| st == "WrappingWorkbench").unwrap();
        assert!(workbench_sign.1.contains("WRAPPING STATION"));
        assert!(workbench_sign.1.contains("ASSEMBLE, TAPE & TIE"));

        let send_sign = signs.iter().find(|(st, _)| st == "PresentSendingPlatform").unwrap();
        assert!(send_sign.1.contains("PLACE WRAPPED GIFT ON CIRCLE"));
        assert!(send_sign.1.contains("HIT RED BUTTON TO SEND"));
    }

    #[test]
    fn test_diegetic_3d_quad_displays() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(50));
        app.insert_resource(time);
        app.insert_resource(WorkshopQuota {
            presents_delivered: 0,
            target_quota: 5,
            tier: 1,
        });
        app.insert_resource(MooseHunger {
            value: 10.0,
            is_breached: false,
            ..default()
        });

        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<Image>>();

        app.add_systems(Startup, spawn_diegetic_screens_and_signs);
        app.add_systems(Update, update_diegetic_monitor_text);
        app.update();

        let (crt_display, mesh3d, mat3d) = {
            let mut q_crt = app.world_mut().query::<(&CrtMonitorTextDisplay, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>();
            let (disp, m, mat) = q_crt.single(app.world()).expect("Must spawn CrtMonitor3dTextQuad with 3D text display");
            (disp.clone(), m.clone(), mat.clone())
        };
        assert!(crt_display.last_content.contains("PAGE 1/4"));
        assert!(crt_display.last_content.contains("PRESENT INSTRUCTIONS"));

        let meshes = app.world().resource::<Assets<Mesh>>();
        assert!(meshes.get(&mesh3d.0).is_some(), "CRT 3D text quad mesh must exist in Assets<Mesh>");
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let mat = materials.get(&mat3d.0).expect("CRT 3D text material must exist");
        assert!(mat.base_color_texture.is_some(), "Material must reference the rasterized text texture");
        let images = app.world().resource::<Assets<Image>>();
        let crt_image = images.get(&crt_display.image_handle).expect("CRT image asset must exist");
        let crt_data = crt_image.data.as_ref().unwrap();
        let crt_text_pixels = crt_data.chunks_exact(4).filter(|p| p[1] > 100).count();
        assert!(crt_text_pixels > 500, "CRT screen image must contain green phosphor text pixels");

        let sign_3d_count = {
            let mut q_signs = app.world_mut().query::<(&DirectionalSign3dText, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>();
            q_signs.iter(app.world()).count()
        };
        assert!(sign_3d_count >= 2, "Expected at least 2 in-world directional sign 3D text quads, found: {}", sign_3d_count);

        {
            let mut t = app.world_mut().resource_mut::<Time<()>>();
            t.advance_by(std::time::Duration::from_millis(4100));
        }
        app.update();

        let q_mon = {
            let mut q = app.world_mut().query::<&CrtInstructionMonitor>();
            q.single(app.world()).unwrap().clone()
        };
        assert_eq!(q_mon.current_page, 1, "Monitor should cycle to Page 1 (Page 2/4)");

        let q_display = {
            let mut q = app.world_mut().query::<&CrtMonitorTextDisplay>();
            q.single(app.world()).unwrap().clone()
        };
        assert!(q_display.last_content.contains("PAGE 2/4"), "CRT display component must update to Page 2 content");
        assert!(q_display.last_content.contains("DISPATCH & VACUUM GUIDE"));
    }
}
