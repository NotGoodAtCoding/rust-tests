use bevy::prelude::*;
use bevy::text::FontSize;
use crate::elf_game::{
    core::WorkshopQuota,
    moose::MooseHunger,
};

#[derive(Component)]
pub struct DiegeticMonitor;

#[derive(Component)]
pub struct DiegeticText;

pub fn spawn_diegetic_screens_and_signs(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let casing_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        perceptual_roughness: 0.7,
        ..default()
    });

    let screen_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.15, 0.08), // Retro CRT green tint
        emissive: LinearRgba::rgb(0.02, 0.10, 0.04),
        perceptual_roughness: 0.3,
        ..default()
    });

    // 1. Workshop Task Monitor (Mounted above the workbench: X=0.0, Y=2.2, Z=-5.0)
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(2.4, 1.4, 0.3))),
            MeshMaterial3d(casing_mat.clone()),
            Transform::from_xyz(0.0, 2.2, -5.0),
            DiegeticMonitor,
            Name::new("WorkshopTaskCrtMonitor"),
        ))
        .with_children(|parent| {
            // Screen glass pane
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.1, 1.1, 0.05))),
                MeshMaterial3d(screen_mat),
                Transform::from_xyz(0.0, 0.0, 0.16),
                Name::new("CrtScreenGlass"),
            ));

            // In-world 2D Text Display rendered in 3D world
            parent.spawn((
                Text2d::new("SANTA OS v12.25\nQUOTA: 0 / 5\nMR MOOSE: CALM"),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(Color::srgb(0.2, 1.0, 0.4)),
                Transform::from_xyz(0.0, 0.0, 0.20).with_scale(Vec3::splat(0.008)),
                DiegeticText,
                Name::new("CrtMonitorText"),
            ));
        });

    // 2. Wooden Warning Sign next to Moose Window (X = -2.6, Y = 1.8, Z = -12.8)
    let sign_wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.4, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });

    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.6, 1.0, 0.1))),
            MeshMaterial3d(sign_wood),
            Transform::from_xyz(-2.6, 1.8, -12.8),
            Name::new("MooseWarningSignBoard"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text2d::new("WARNING:\nKEEP MR. MOOSE\nFED OR ELSE"),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.1, 0.1)),
                Transform::from_xyz(0.0, 0.0, 0.07).with_scale(Vec3::splat(0.009)),
                Name::new("WarningSignText"),
            ));
        });
}

/// Updates in-world diegetic monitors dynamically with live workshop stats
pub fn update_diegetic_monitor_text(
    quota: Res<WorkshopQuota>,
    moose: Res<MooseHunger>,
    mut q_text: Query<&mut Text2d, With<DiegeticText>>,
) {
    let moose_status = if moose.is_breached {
        "BREACH!! RUN!!"
    } else if moose.value >= 50.0 {
        "AGITATED HUFFING (WARNING!)"
    } else if moose.value >= 20.0 {
        "PEEKING THROUGH WINDOW"
    } else {
        "CALM (BEHIND SHUTTERS)"
    };

    let text_content = format!(
        "SANTA OS v12.25\nTIER: {}\nQUOTA: {} / {}\nMOOSE: {:.0}/60 ({})",
        quota.tier, quota.presents_delivered, quota.target_quota, moose.value, moose_status
    );

    for mut text in &mut q_text {
        text.0 = text_content.clone();
    }
}
