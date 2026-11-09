use bevy::prelude::*;
use super::palette::*;

/// Centralized pre-built StandardMaterial assets for Santa's Workshop.
/// Decouples rendering material definitions from game logic engines.
#[derive(Resource, Clone, Debug)]
pub struct WorkshopMaterials {
    // --- Woods & Cabin Architecture ---
    pub cabin_log: Handle<StandardMaterial>,
    pub timber_beam: Handle<StandardMaterial>,
    pub weathered_post: Handle<StandardMaterial>,
    pub rustic_planks: Handle<StandardMaterial>,
    pub workbench_wood: Handle<StandardMaterial>,
    pub birch_wood: Handle<StandardMaterial>,
    pub walnut_leather: Handle<StandardMaterial>,
    pub pine_sign: Handle<StandardMaterial>,
    pub tree_bark: Handle<StandardMaterial>,

    // --- Festive & Textiles ---
    pub festive_red: Handle<StandardMaterial>,
    pub pine_green: Handle<StandardMaterial>,
    pub fluffy_white: Handle<StandardMaterial>,
    pub snow_white: Handle<StandardMaterial>,
    pub gold_ribbon: Handle<StandardMaterial>,
    pub hazard_yellow: Handle<StandardMaterial>,
    pub candy_cane_red: Handle<StandardMaterial>,
    pub elf_skin: Handle<StandardMaterial>,

    // --- Metals, Iron & Hardware ---
    pub polished_brass: Handle<StandardMaterial>,
    pub brass_plaque: Handle<StandardMaterial>,
    pub brass_buckle: Handle<StandardMaterial>,
    pub cast_iron: Handle<StandardMaterial>,
    pub wrought_iron: Handle<StandardMaterial>,
    pub sheet_metal: Handle<StandardMaterial>,
    pub industrial_dark: Handle<StandardMaterial>,
    pub platform_metal: Handle<StandardMaterial>,
    pub chute_void: Handle<StandardMaterial>,

    // --- Mr. Moose Toy Anatomy ---
    pub moose_fur: Handle<StandardMaterial>,
    pub moose_muzzle: Handle<StandardMaterial>,
    pub moose_nose: Handle<StandardMaterial>,
    pub moose_antler: Handle<StandardMaterial>,
    pub moose_hoof: Handle<StandardMaterial>,
    pub moose_eye: Handle<StandardMaterial>,
    pub moose_scarf_red: Handle<StandardMaterial>,
    pub moose_scarf_white: Handle<StandardMaterial>,

    // --- Diegetic UI, Gauges & CRT Screens ---
    pub gauge_green: Handle<StandardMaterial>,
    pub gauge_yellow: Handle<StandardMaterial>,
    pub gauge_orange: Handle<StandardMaterial>,
    pub gauge_red: Handle<StandardMaterial>,
    pub ivory_enamel: Handle<StandardMaterial>,
    pub crt_screen: Handle<StandardMaterial>,
    pub crt_scanlines: Handle<StandardMaterial>,
    pub crt_sweep: Handle<StandardMaterial>,

    // --- Glass, Illumination & Lighting ---
    pub window_glass: Handle<StandardMaterial>,
    pub tractor_beam: Handle<StandardMaterial>,
    pub vacuum_tube_glass: Handle<StandardMaterial>,
    pub warning_lantern: Handle<StandardMaterial>,
    pub candle_wax: Handle<StandardMaterial>,
    pub candle_flame: Handle<StandardMaterial>,
    pub candle_smoke: Handle<StandardMaterial>,
    pub candle_spark: Handle<StandardMaterial>,

    // --- Toys, Props & Snacks ---
    pub toy_blue: Handle<StandardMaterial>,
    pub terracotta_pot: Handle<StandardMaterial>,
    pub cocoa_red: Handle<StandardMaterial>,
    pub hot_cocoa_liquid: Handle<StandardMaterial>,
    pub moose_snack_oat: Handle<StandardMaterial>,
    pub snack_jar_glass: Handle<StandardMaterial>,
    pub send_button_red: Handle<StandardMaterial>,

    // --- Stove, Oven & Propane System ---
    pub gas_hose: Handle<StandardMaterial>,
    pub gas_anchor_red: Handle<StandardMaterial>,
    pub propane_tank_blue: Handle<StandardMaterial>,
    pub propane_bulk_tank: Handle<StandardMaterial>,
    pub stove_glowing_ember: Handle<StandardMaterial>,
    pub stove_cool_dark_gray: Handle<StandardMaterial>,
    pub stove_blue_flame: Handle<StandardMaterial>,

    // --- Wearables & Tech Props ---
    pub bunny_ear_pink: Handle<StandardMaterial>,
    pub plasma_chamber: Handle<StandardMaterial>,
}

impl WorkshopMaterials {
    /// Factory initializing all StandardMaterial handles inside Bevy's asset storage.
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            // Woods & Cabin
            cabin_log: materials.add(StandardMaterial {
                base_color: CABIN_LOG,
                perceptual_roughness: 0.90,
                ..default()
            }),
            timber_beam: materials.add(StandardMaterial {
                base_color: TIMBER_BEAM,
                perceptual_roughness: 0.85,
                ..default()
            }),
            weathered_post: materials.add(StandardMaterial {
                base_color: WEATHERED_POST,
                perceptual_roughness: 0.95,
                ..default()
            }),
            rustic_planks: materials.add(StandardMaterial {
                base_color: RUSTIC_PLANKS,
                perceptual_roughness: 0.78,
                ..default()
            }),
            workbench_wood: materials.add(StandardMaterial {
                base_color: WORKBENCH_WOOD,
                perceptual_roughness: 0.70,
                ..default()
            }),
            birch_wood: materials.add(StandardMaterial {
                base_color: BIRCH_WOOD,
                perceptual_roughness: 0.50,
                ..default()
            }),
            walnut_leather: materials.add(StandardMaterial {
                base_color: WALNUT_LEATHER,
                perceptual_roughness: 0.60,
                ..default()
            }),
            pine_sign: materials.add(StandardMaterial {
                base_color: PINE_SIGN_BOARD,
                perceptual_roughness: 0.80,
                ..default()
            }),
            tree_bark: materials.add(StandardMaterial {
                base_color: TREE_BARK,
                perceptual_roughness: 0.90,
                ..default()
            }),

            // Festive & Textiles
            festive_red: materials.add(StandardMaterial {
                base_color: FESTIVE_RED,
                perceptual_roughness: 0.85,
                ..default()
            }),
            pine_green: materials.add(StandardMaterial {
                base_color: PINE_GREEN,
                perceptual_roughness: 0.85,
                ..default()
            }),
            fluffy_white: materials.add(StandardMaterial {
                base_color: SNOW_WHITE,
                perceptual_roughness: 0.90,
                ..default()
            }),
            snow_white: materials.add(StandardMaterial {
                base_color: SNOW_WHITE,
                perceptual_roughness: 0.95,
                ..default()
            }),
            gold_ribbon: materials.add(StandardMaterial {
                base_color: GOLD_RIBBON,
                metallic: 0.60,
                perceptual_roughness: 0.30,
                ..default()
            }),
            hazard_yellow: materials.add(StandardMaterial {
                base_color: HAZARD_YELLOW,
                perceptual_roughness: 0.30,
                ..default()
            }),
            candy_cane_red: materials.add(StandardMaterial {
                base_color: CANDY_CANE_RED,
                perceptual_roughness: 0.40,
                ..default()
            }),
            elf_skin: materials.add(StandardMaterial {
                base_color: BIRCH_WOOD,
                perceptual_roughness: 0.50,
                ..default()
            }),

            // Metals & Hardware
            polished_brass: materials.add(StandardMaterial {
                base_color: POLISHED_BRASS,
                metallic: 0.85,
                perceptual_roughness: 0.25,
                ..default()
            }),
            brass_plaque: materials.add(StandardMaterial {
                base_color: BRASS_PLAQUE,
                metallic: 0.80,
                perceptual_roughness: 0.30,
                ..default()
            }),
            brass_buckle: materials.add(StandardMaterial {
                base_color: BRASS_BUCKLE,
                metallic: 0.85,
                perceptual_roughness: 0.20,
                ..default()
            }),
            cast_iron: materials.add(StandardMaterial {
                base_color: CAST_IRON,
                metallic: 0.50,
                perceptual_roughness: 0.75,
                ..default()
            }),
            wrought_iron: materials.add(StandardMaterial {
                base_color: WROUGHT_IRON,
                metallic: 0.60,
                perceptual_roughness: 0.60,
                ..default()
            }),
            sheet_metal: materials.add(StandardMaterial {
                base_color: SHEET_METAL,
                metallic: 0.75,
                perceptual_roughness: 0.35,
                ..default()
            }),
            industrial_dark: materials.add(StandardMaterial {
                base_color: INDUSTRIAL_DARK,
                metallic: 0.80,
                perceptual_roughness: 0.30,
                ..default()
            }),
            platform_metal: materials.add(StandardMaterial {
                base_color: PLATFORM_METAL,
                metallic: 0.60,
                perceptual_roughness: 0.40,
                ..default()
            }),
            chute_void: materials.add(StandardMaterial {
                base_color: CHUTE_PIT_VOID,
                perceptual_roughness: 0.95,
                ..default()
            }),

            // Moose Toy Model Anatomy
            moose_fur: materials.add(StandardMaterial {
                base_color: MOOSE_FUR,
                perceptual_roughness: 0.85,
                ..default()
            }),
            moose_muzzle: materials.add(StandardMaterial {
                base_color: MOOSE_MUZZLE,
                perceptual_roughness: 0.75,
                ..default()
            }),
            moose_nose: materials.add(StandardMaterial {
                base_color: MOOSE_NOSE,
                perceptual_roughness: 0.55,
                ..default()
            }),
            moose_antler: materials.add(StandardMaterial {
                base_color: MOOSE_ANTLER,
                perceptual_roughness: 0.70,
                ..default()
            }),
            moose_hoof: materials.add(StandardMaterial {
                base_color: MOOSE_HOOF,
                perceptual_roughness: 0.80,
                ..default()
            }),
            moose_eye: materials.add(StandardMaterial {
                base_color: MOOSE_OBSIDIAN_EYE,
                metallic: 0.30,
                perceptual_roughness: 0.10,
                ..default()
            }),
            moose_scarf_red: materials.add(StandardMaterial {
                base_color: FESTIVE_RED,
                perceptual_roughness: 0.85,
                ..default()
            }),
            moose_scarf_white: materials.add(StandardMaterial {
                base_color: SNOW_WHITE,
                perceptual_roughness: 0.85,
                ..default()
            }),

            // UI Gauges & CRT Displays
            gauge_green: materials.add(StandardMaterial {
                base_color: GAUGE_GREEN,
                perceptual_roughness: 0.40,
                ..default()
            }),
            gauge_yellow: materials.add(StandardMaterial {
                base_color: GAUGE_YELLOW,
                perceptual_roughness: 0.40,
                ..default()
            }),
            gauge_orange: materials.add(StandardMaterial {
                base_color: GAUGE_ORANGE,
                perceptual_roughness: 0.40,
                ..default()
            }),
            gauge_red: materials.add(StandardMaterial {
                base_color: GAUGE_RED,
                perceptual_roughness: 0.40,
                ..default()
            }),
            ivory_enamel: materials.add(StandardMaterial {
                base_color: IVORY_ENAMEL,
                perceptual_roughness: 0.25,
                ..default()
            }),
            crt_screen: materials.add(StandardMaterial {
                base_color: CRT_TUBE_DARK,
                perceptual_roughness: 0.20,
                ..default()
            }),
            crt_scanlines: materials.add(StandardMaterial {
                base_color: CRT_SCANLINE_SHADOW,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.10,
                ..default()
            }),
            crt_sweep: materials.add(StandardMaterial {
                base_color: CRT_SWEEP_GLOW,
                alpha_mode: AlphaMode::Blend,
                emissive: LinearRgba::new(0.2, 1.0, 0.4, 0.5),
                perceptual_roughness: 0.10,
                ..default()
            }),

            // Glass, Illumination & Lighting
            window_glass: materials.add(StandardMaterial {
                base_color: WINDOW_PANE_GLASS,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.10,
                metallic: 0.10,
                ..default()
            }),
            tractor_beam: materials.add(StandardMaterial {
                base_color: TRACTOR_BEAM_GLOW,
                alpha_mode: AlphaMode::Blend,
                emissive: LinearRgba::new(0.08, 0.35, 0.55, 0.4),
                perceptual_roughness: 0.10,
                ..default()
            }),
            vacuum_tube_glass: materials.add(StandardMaterial {
                base_color: VACUUM_TUBE_GLASS,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.15,
                metallic: 0.20,
                ..default()
            }),
            warning_lantern: materials.add(StandardMaterial {
                base_color: WARNING_LANTERN_RUBY,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.15,
                ..default()
            }),
            candle_wax: materials.add(StandardMaterial {
                base_color: CANDLE_TALLOW,
                perceptual_roughness: 0.60,
                ..default()
            }),
            candle_flame: materials.add(StandardMaterial {
                base_color: CANDLE_FLAME,
                emissive: LinearRgba::new(1.0, 0.82, 0.25, 1.0),
                unlit: true,
                ..default()
            }),
            candle_smoke: materials.add(StandardMaterial {
                base_color: CANDLE_SMOKE,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                perceptual_roughness: 0.90,
                ..default()
            }),
            candle_spark: materials.add(StandardMaterial {
                base_color: CANDLE_SPARK,
                emissive: LinearRgba::new(1.0, 0.75, 0.15, 1.0),
                unlit: true,
                ..default()
            }),

            // Toys, Props & Snacks
            toy_blue: materials.add(StandardMaterial {
                base_color: UNWRAPPED_TOY_BLUE,
                perceptual_roughness: 0.40,
                ..default()
            }),
            terracotta_pot: materials.add(StandardMaterial {
                base_color: TERRACOTTA_POT,
                perceptual_roughness: 0.85,
                ..default()
            }),
            cocoa_red: materials.add(StandardMaterial {
                base_color: COCOA_CERAMIC_RED,
                perceptual_roughness: 0.20,
                ..default()
            }),
            hot_cocoa_liquid: materials.add(StandardMaterial {
                base_color: HOT_COCOA_LIQUID,
                perceptual_roughness: 0.30,
                ..default()
            }),
            moose_snack_oat: materials.add(StandardMaterial {
                base_color: MOOSE_SNACK_GOLDEN_OAT,
                perceptual_roughness: 0.70,
                ..default()
            }),
            snack_jar_glass: materials.add(StandardMaterial {
                base_color: SNACK_JAR_GLASS,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.15,
                metallic: 0.10,
                ..default()
            }),
            send_button_red: materials.add(StandardMaterial {
                base_color: FESTIVE_RED,
                emissive: LinearRgba::new(0.60, 0.05, 0.05, 1.0),
                perceptual_roughness: 0.20,
                ..default()
            }),

            // Stove, Oven & Propane System
            gas_hose: materials.add(StandardMaterial {
                base_color: GAS_HOSE_YELLOW,
                perceptual_roughness: 0.50,
                ..default()
            }),
            gas_anchor_red: materials.add(StandardMaterial {
                base_color: SAFETY_RED,
                perceptual_roughness: 0.40,
                ..default()
            }),
            propane_tank_blue: materials.add(StandardMaterial {
                base_color: PROPANE_CYLINDER_BLUE,
                perceptual_roughness: 0.35,
                metallic: 0.60,
                ..default()
            }),
            propane_bulk_tank: materials.add(StandardMaterial {
                base_color: BULK_TANK_WHITE,
                perceptual_roughness: 0.40,
                metallic: 0.50,
                ..default()
            }),
            stove_glowing_ember: materials.add(StandardMaterial {
                base_color: STOVE_GLOWING_EMBER,
                emissive: LinearRgba::new(2.5, 0.65, 0.05, 1.0),
                unlit: true,
                ..default()
            }),
            stove_cool_dark_gray: materials.add(StandardMaterial {
                base_color: STOVE_COOL_GRAY,
                perceptual_roughness: 0.85,
                ..default()
            }),
            stove_blue_flame: materials.add(StandardMaterial {
                base_color: STOVE_BLUE_FLAME,
                emissive: LinearRgba::new(0.4, 1.2, 3.0, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),

            // Wearables & Tech Props
            bunny_ear_pink: materials.add(StandardMaterial {
                base_color: BUNNY_EAR_PINK,
                perceptual_roughness: 0.60,
                ..default()
            }),
            plasma_chamber: materials.add(StandardMaterial {
                base_color: PLASMA_CHAMBER_PURPLE,
                emissive: LinearRgba::rgb(0.5, 0.1, 0.8),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        }
    }
}

/// Startup system populating the WorkshopMaterials resource.
pub fn init_workshop_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let workshop_mats = WorkshopMaterials::new(&mut materials);
    commands.insert_resource(workshop_mats);
}

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, init_workshop_materials);
        app.add_plugins(super::objects::AnchorPointPlugin);
    }
}
