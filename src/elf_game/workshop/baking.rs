use std::collections::HashSet;
use bevy::prelude::*;
use bevy::ecs::message::{Message, MessageWriter};
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::elf_game::{
    assets::{palette::SHEET_METAL, WorkshopMaterials},
    core::is_game_paused,
    moose::MooseSnack,
    player::hands::Grabbable,
    workshop::{
        pantry::PantryDoor,
        stove::{GasFurnaceStove, GasHose, HoseAttachment, WorkshopGasLine},
    },
};

/// Tag component for freshly baked festive cookies.
#[derive(Component, Debug, Clone)]
pub struct BakedFestiveCookie;

/// The distinct baking ingredients stored in the Pantry Storage Wing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BakingIngredientType {
    Flour,
    Sugar,
    Butter,
    ChocolateChips,
}

impl BakingIngredientType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Flour => "Flour Sack",
            Self::Sugar => "Sugar Bag",
            Self::Butter => "Butter Block",
            Self::ChocolateChips => "Chocolate Chip Tin",
        }
    }

    pub fn default_color(&self) -> Color {
        match self {
            Self::Flour => Color::srgb(0.92, 0.90, 0.85),
            Self::Sugar => Color::srgb(0.98, 0.98, 0.98),
            Self::Butter => Color::srgb(1.0, 0.88, 0.45),
            Self::ChocolateChips => Color::srgb(0.32, 0.18, 0.10),
        }
    }
}

/// A grabbable baking ingredient prop spawned on the pantry shelves.
#[derive(Component, Debug, Clone)]
pub struct BakingIngredient {
    pub ingredient_type: BakingIngredientType,
    pub home_shelf_pos: Vec3,
}

/// The kitchen baking sheet tray used to combine ingredients into raw cookie dough
/// and bake them on the heated furnace stove.
#[derive(Component, Debug, Clone)]
pub struct BakingSheet {
    pub ingredients_added: HashSet<BakingIngredientType>,
    pub is_dough_ready: bool,
    pub bake_timer: f32,
    pub bake_duration: f32,
    pub is_baked: bool,
    pub cookies_spawned: bool,
    pub cookies_count: u32,
}

impl Default for BakingSheet {
    fn default() -> Self {
        Self {
            ingredients_added: HashSet::new(),
            is_dough_ready: false,
            bake_timer: 0.0,
            bake_duration: 3.5,
            is_baked: false,
            cookies_spawned: false,
            cookies_count: 4,
        }
    }
}

impl BakingSheet {
    /// Adds an ingredient to the baking sheet. If all 4 are present, the raw dough is ready.
    pub fn add_ingredient(&mut self, ingredient: BakingIngredientType) -> bool {
        let inserted = self.ingredients_added.insert(ingredient);
        if self.ingredients_added.len() >= 4 {
            self.is_dough_ready = true;
        }
        inserted
    }
}

/// Tag component for visual raw dough entity parented to the baking sheet.
#[derive(Component, Debug, Clone)]
pub struct RawCookieDoughVisual;

/// Event fired when a batch of festive cookies finishes baking.
#[derive(Message, Debug, Clone)]
pub struct CookieBakedEvent {
    pub position: Vec3,
    pub cookies_count: u32,
}

/// Spawns the initial BakingSheet on the Pantry staging counter and ingredients on the shelves.
pub fn spawn_baking_scene(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    let metal_mat = workshop_mats.as_ref().map(|wm| wm.sheet_metal.clone()).unwrap_or_else(|| {
        materials.add(StandardMaterial {
            base_color: SHEET_METAL,
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        })
    });

    // 1. Spawn BakingSheet on the Pantry Baking Staging Counter (X = -7.35, Y = 0.95, Z = 11.0)
    commands
        .spawn((
            BakingSheet::default(),
            Transform::from_xyz(-7.35, 0.95, 11.0),
            Visibility::default(),
            Collider::cuboid(0.25, 0.02, 0.35),
            RigidBody::Dynamic,
            Friction {
                coefficient: 1.2,
                combine_rule: CoefficientCombineRule::Max,
            },
            Damping {
                linear_damping: 1.2,
                angular_damping: 1.8,
            },
            Grabbable,
            Name::new("PantryBakingSheet"),
        ))
        .with_children(|sheet| {
            // Sheet metal tray mesh (Width: 0.50m, Thickness: 0.025m, Depth: 0.70m)
            sheet.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.50, 0.025, 0.70))),
                MeshMaterial3d(metal_mat.clone()),
                Name::new("BakingSheetTrayMesh"),
            ));
        });

    // 2. Spawn 4 Ingredients on West Supply Shelves (X = -12.65, Z in [10.0, 12.0])
    let ingredient_configs = [
        (BakingIngredientType::Flour, Vec3::new(-12.65, 0.95, 10.2), Vec3::new(0.24, 0.30, 0.20)),
        (BakingIngredientType::Sugar, Vec3::new(-12.65, 0.95, 11.0), Vec3::new(0.20, 0.26, 0.18)),
        (BakingIngredientType::Butter, Vec3::new(-12.65, 0.95, 11.8), Vec3::new(0.18, 0.12, 0.14)),
        (BakingIngredientType::ChocolateChips, Vec3::new(-12.65, 1.70, 11.0), Vec3::new(0.18, 0.16, 0.18)),
    ];

    for (ing_type, pos, dims) in ingredient_configs {
        let ing_mat = materials.add(StandardMaterial {
            base_color: ing_type.default_color(),
            perceptual_roughness: 0.6,
            ..default()
        });

        commands.spawn((
            BakingIngredient {
                ingredient_type: ing_type,
                home_shelf_pos: pos,
            },
            Transform::from_translation(pos),
            Visibility::default(),
            Mesh3d(meshes.add(Cuboid::new(dims.x, dims.y, dims.z))),
            MeshMaterial3d(ing_mat),
            Collider::cuboid(dims.x * 0.5, dims.y * 0.5, dims.z * 0.5),
            RigidBody::Dynamic,
            Friction {
                coefficient: 1.0,
                combine_rule: CoefficientCombineRule::Average,
            },
            Damping {
                linear_damping: 1.0,
                angular_damping: 1.5,
            },
            Grabbable,
            Name::new(format!("PantryIngredient_{}", ing_type.name().replace(' ', ""))),
        ));
    }
}

/// Detects when a baking ingredient is brought within proximity of the BakingSheet
/// and incorporates it into the raw cookie dough recipe.
pub fn ingredient_baking_sheet_assembly_system(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut q_sheet: Query<(Entity, &Transform, Option<&GlobalTransform>, &mut BakingSheet)>,
    q_ingredients: Query<(Entity, &Transform, Option<&GlobalTransform>, &BakingIngredient)>,
) {
    for (sheet_e, sheet_tf, sheet_gt, mut sheet) in &mut q_sheet {
        if sheet.is_dough_ready {
            continue;
        }

        let sheet_pos = sheet_gt.map(|g| g.translation()).unwrap_or(sheet_tf.translation);

        for (ing_e, ing_tf, ing_gt, ing) in &q_ingredients {
            let ing_pos = ing_gt.map(|g| g.translation()).unwrap_or(ing_tf.translation);
            // Distance threshold to deposit ingredient onto baking sheet (0.65m)
            if sheet_pos.distance(ing_pos) < 0.65 {
                if sheet.add_ingredient(ing.ingredient_type) {
                    // Despawn the added ingredient entity
                    commands.entity(ing_e).despawn();

                    // If dough is now complete, spawn dough visual on the tray
                    if sheet.is_dough_ready {
                        if let (Some(m), Some(mats)) = (&mut meshes, &mut materials) {
                            let dough_mat = mats.add(StandardMaterial {
                                base_color: Color::srgb(0.95, 0.88, 0.72), // Warm butter cookie dough
                                perceptual_roughness: 0.85,
                                ..default()
                            });

                            commands.entity(sheet_e).with_children(|parent| {
                                parent.spawn((
                                    RawCookieDoughVisual,
                                    Mesh3d(m.add(Cuboid::new(0.42, 0.03, 0.60))),
                                    MeshMaterial3d(dough_mat),
                                    Transform::from_xyz(0.0, 0.025, 0.0),
                                    Name::new("RawCookieDoughLayer"),
                                ));
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Bakes raw cookie dough on a BakingSheet when placed near/on the active, heated GasFurnaceStove.
pub fn stove_baking_system(
    time: Option<Res<Time>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    gas_line: Option<Res<WorkshopGasLine>>,
    q_hose: Query<&GasHose>,
    q_stove: Query<(&Transform, Option<&GlobalTransform>, &GasFurnaceStove)>,
    mut q_sheet: Query<(&Transform, Option<&GlobalTransform>, &mut BakingSheet)>,
    mut baked_events: MessageWriter<CookieBakedEvent>,
) {
    if is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    let gas_pressure = gas_line.as_ref().map(|g| g.pressure_psi).unwrap_or(100.0);
    let hose_attached_to_stove = q_hose.iter().any(|h| h.attached_to == HoseAttachment::Stove);

    let Some((stove_tf, stove_gt, stove)) = q_stove.iter().next() else {
        return;
    };

    let stove_active = stove.is_on && hose_attached_to_stove && gas_pressure > 0.0;
    if !stove_active {
        return;
    }

    let stove_pos = stove_gt.map(|g| g.translation()).unwrap_or(stove_tf.translation);
    let stove_cooking_surface = stove_pos + Vec3::new(0.0, 0.68, 0.0);

    for (sheet_tf, sheet_gt, mut sheet) in &mut q_sheet {
        if !sheet.is_dough_ready || sheet.is_baked {
            continue;
        }

        let sheet_pos = sheet_gt.map(|g| g.translation()).unwrap_or(sheet_tf.translation);
        // Check if baking sheet is resting on or very close to the stove cooking top (radius 1.6m)
        if sheet_pos.distance(stove_cooking_surface) < 1.6 {
            sheet.bake_timer += dt;
            if sheet.bake_timer >= sheet.bake_duration {
                sheet.is_baked = true;
                baked_events.write(CookieBakedEvent {
                    position: sheet_pos,
                    cookies_count: sheet.cookies_count,
                });
            }
        }
    }
}

/// Spawns golden baked festive cookies from a finished BakingSheet.
pub fn baking_sheet_cookie_harvest_system(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    workshop_mats: Option<Res<WorkshopMaterials>>,
    mut q_sheet: Query<(Entity, &Transform, Option<&GlobalTransform>, &mut BakingSheet)>,
    q_dough: Query<Entity, With<RawCookieDoughVisual>>,
) {
    let cookie_mat = workshop_mats.as_ref().map(|wm| wm.moose_snack_oat.clone()).unwrap_or_else(|| {
        materials.as_mut().map(|m| {
            m.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.68, 0.42),
                perceptual_roughness: 0.9,
                ..default()
            })
        }).unwrap_or_default()
    });

    for (_sheet_e, sheet_tf, sheet_gt, mut sheet) in &mut q_sheet {
        if !sheet.is_baked || sheet.cookies_spawned {
            continue;
        }

        sheet.cookies_spawned = true;
        let sheet_pos = sheet_gt.map(|g| g.translation()).unwrap_or(sheet_tf.translation);

        // Despawn the raw dough visual layer
        for dough_e in &q_dough {
            commands.entity(dough_e).despawn();
        }

        // Spawn 4 fresh baked festive cookies resting on the sheet
        let cookie_offsets = [
            Vec3::new(-0.12, 0.08, -0.15),
            Vec3::new(0.12, 0.08, -0.15),
            Vec3::new(-0.12, 0.08, 0.15),
            Vec3::new(0.12, 0.08, 0.15),
        ];

        for (i, offset) in cookie_offsets.iter().enumerate() {
            let cookie_pos = sheet_pos + *offset;
            let mut cookie_cmd = commands.spawn((
                MooseSnack { nutrition_value: 30.0 },
                BakedFestiveCookie,
                Transform::from_translation(cookie_pos),
                Visibility::default(),
                Collider::cylinder(0.02, 0.07),
                RigidBody::Dynamic,
                Friction {
                    coefficient: 2.5,
                    combine_rule: CoefficientCombineRule::Max,
                },
                Restitution {
                    coefficient: 0.05,
                    combine_rule: CoefficientCombineRule::Min,
                },
                Damping {
                    linear_damping: 1.0,
                    angular_damping: 1.5,
                },
                Grabbable,
                Name::new(format!("FreshBakedFestiveCookie_{}", i + 1)),
            ));

            if let (Some(m), mat) = (meshes.as_mut(), cookie_mat.clone()) {
                cookie_cmd.insert((
                    Mesh3d(m.add(Cylinder::new(0.07, 0.04))),
                    MeshMaterial3d(mat),
                ));
            }
        }
    }
}

/// Restocks consumed ingredients in the Pantry when the pantry door is closed and reopened.
pub fn pantry_ingredient_restock_system(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_door: Query<&PantryDoor, Changed<PantryDoor>>,
    q_existing_ingredients: Query<&BakingIngredient>,
) {
    for door in &q_door {
        // Trigger on door opening transition
        if door.current_angle > door.open_threshold && door.was_closed {
            let existing_types: HashSet<BakingIngredientType> = q_existing_ingredients
                .iter()
                .map(|ing| ing.ingredient_type)
                .collect();

            let ingredient_configs = [
                (BakingIngredientType::Flour, Vec3::new(-12.65, 0.95, 10.2), Vec3::new(0.24, 0.30, 0.20)),
                (BakingIngredientType::Sugar, Vec3::new(-12.65, 0.95, 11.0), Vec3::new(0.20, 0.26, 0.18)),
                (BakingIngredientType::Butter, Vec3::new(-12.65, 0.95, 11.8), Vec3::new(0.18, 0.12, 0.14)),
                (BakingIngredientType::ChocolateChips, Vec3::new(-12.65, 1.70, 11.0), Vec3::new(0.18, 0.16, 0.18)),
            ];

            for (ing_type, pos, dims) in ingredient_configs {
                if !existing_types.contains(&ing_type) {
                    let mut ing_cmd = commands.spawn((
                        BakingIngredient {
                            ingredient_type: ing_type,
                            home_shelf_pos: pos,
                        },
                        Transform::from_translation(pos),
                        Visibility::default(),
                        Collider::cuboid(dims.x * 0.5, dims.y * 0.5, dims.z * 0.5),
                        RigidBody::Dynamic,
                        Friction {
                            coefficient: 1.0,
                            combine_rule: CoefficientCombineRule::Average,
                        },
                        Damping {
                            linear_damping: 1.0,
                            angular_damping: 1.5,
                        },
                        Grabbable,
                        Name::new(format!("PantryIngredient_{}", ing_type.name().replace(' ', ""))),
                    ));

                    if let (Some(m), Some(mats)) = (meshes.as_mut(), materials.as_mut()) {
                        let ing_mat = mats.add(StandardMaterial {
                            base_color: ing_type.default_color(),
                            perceptual_roughness: 0.6,
                            ..default()
                        });
                        ing_cmd.insert((
                            Mesh3d(m.add(Cuboid::new(dims.x, dims.y, dims.z))),
                            MeshMaterial3d(ing_mat),
                        ));
                    }
                }
            }
        }
    }
}
