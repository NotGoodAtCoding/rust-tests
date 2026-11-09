use bevy::prelude::*;

// =============================================================================
// SANTA'S WORKSHOP: MINIMAL & IMPRESSIONIST COLOR PALETTE
// Principle 4 (PLAN.md): Models should be minimal and impressionist.
// Bold silhouettes, clean iconic masses, and unified saturated palette.
// =============================================================================

// --- Festive Core ---
pub const FESTIVE_RED: Color = Color::srgb(0.88, 0.12, 0.14);
pub const PINE_GREEN: Color = Color::srgb(0.12, 0.42, 0.20);
pub const SNOW_WHITE: Color = Color::srgb(0.96, 0.96, 0.98);
pub const GOLD_RIBBON: Color = Color::srgb(0.95, 0.80, 0.15);
pub const HAZARD_YELLOW: Color = Color::srgb(0.92, 0.78, 0.12);
pub const CANDY_CANE_RED: Color = Color::srgb(0.85, 0.12, 0.12);

// --- Woods, Bark & Organic Materials ---
pub const CABIN_LOG: Color = Color::srgb(0.40, 0.25, 0.16);
pub const TIMBER_BEAM: Color = Color::srgb(0.26, 0.16, 0.10);
pub const WEATHERED_POST: Color = Color::srgb(0.22, 0.14, 0.08);
pub const RUSTIC_PLANKS: Color = Color::srgb(0.48, 0.30, 0.18);
pub const WORKBENCH_WOOD: Color = Color::srgb(0.40, 0.25, 0.15);
pub const BIRCH_WOOD: Color = Color::srgb(0.92, 0.76, 0.62);
pub const WALNUT_LEATHER: Color = Color::srgb(0.24, 0.16, 0.10);
pub const PINE_SIGN_BOARD: Color = Color::srgb(0.55, 0.36, 0.18);
pub const TREE_BARK: Color = Color::srgb(0.30, 0.18, 0.10);

// --- Metals, Iron & Industrial Surfaces ---
pub const POLISHED_BRASS: Color = Color::srgb(0.82, 0.68, 0.24);
pub const BRASS_PLAQUE: Color = Color::srgb(0.78, 0.62, 0.24);
pub const BRASS_BUCKLE: Color = Color::srgb(0.92, 0.78, 0.25);
pub const CAST_IRON: Color = Color::srgb(0.18, 0.18, 0.20);
pub const WROUGHT_IRON: Color = Color::srgb(0.12, 0.12, 0.14);
pub const SHEET_METAL: Color = Color::srgb(0.68, 0.72, 0.76);
pub const INDUSTRIAL_DARK: Color = Color::srgb(0.22, 0.22, 0.25);
pub const PLATFORM_METAL: Color = Color::srgb(0.25, 0.25, 0.30);
pub const CHUTE_PIT_VOID: Color = Color::srgb(0.06, 0.06, 0.08);

// --- Moose Toy Model Anatomy ---
pub const MOOSE_FUR: Color = Color::srgb(0.24, 0.16, 0.10);
pub const MOOSE_MUZZLE: Color = Color::srgb(0.36, 0.26, 0.18);
pub const MOOSE_NOSE: Color = Color::srgb(0.12, 0.10, 0.09);
pub const MOOSE_ANTLER: Color = Color::srgb(0.78, 0.72, 0.60);
pub const MOOSE_HOOF: Color = Color::srgb(0.10, 0.08, 0.07);
pub const MOOSE_OBSIDIAN_EYE: Color = Color::srgb(0.04, 0.04, 0.05);

// --- Diegetic UI, Gauges & CRT Displays ---
pub const GAUGE_GREEN: Color = Color::srgb(0.15, 0.72, 0.25);
pub const GAUGE_YELLOW: Color = Color::srgb(0.95, 0.82, 0.15);
pub const GAUGE_ORANGE: Color = Color::srgb(0.95, 0.48, 0.10);
pub const GAUGE_RED: Color = Color::srgb(0.90, 0.12, 0.14);
pub const IVORY_ENAMEL: Color = Color::srgb(0.96, 0.94, 0.88);
pub const CRT_PHOSPHOR_GREEN: Color = Color::srgb(0.20, 1.00, 0.40);
pub const CRT_TUBE_DARK: Color = Color::srgb(0.03, 0.12, 0.05);
pub const CRT_SCANLINE_SHADOW: Color = Color::srgba(0.01, 0.04, 0.02, 0.55);
pub const CRT_SWEEP_GLOW: Color = Color::srgba(0.20, 1.00, 0.40, 0.20);

// --- Translucent Glass, Illumination & Lighting ---
pub const WINDOW_PANE_GLASS: Color = Color::srgba(0.85, 0.92, 1.00, 0.35);
pub const TRACTOR_BEAM_GLOW: Color = Color::srgba(0.35, 0.85, 1.00, 0.18);
pub const VACUUM_TUBE_GLASS: Color = Color::srgba(0.85, 0.92, 1.00, 0.55);
pub const WARNING_LANTERN_RUBY: Color = Color::srgba(1.00, 0.25, 0.15, 0.80);
pub const CANDLE_TALLOW: Color = Color::srgb(0.94, 0.92, 0.82);
pub const CANDLE_FLAME: Color = Color::srgb(1.00, 0.82, 0.25);
pub const CANDLE_LIGHT_EMISSION: Color = Color::srgb(1.00, 0.78, 0.46);
pub const CANDLE_SMOKE: Color = Color::srgba(0.85, 0.85, 0.88, 0.35);
pub const CANDLE_SPARK: Color = Color::srgb(1.00, 0.72, 0.15);
pub const WINTER_SUNLIGHT: Color = Color::srgb(0.92, 0.96, 1.00);

// --- Toys, Props & Snacks ---
pub const UNWRAPPED_TOY_BLUE: Color = Color::srgb(0.20, 0.50, 0.80);
pub const TERRACOTTA_POT: Color = Color::srgb(0.72, 0.36, 0.20);
pub const COCOA_CERAMIC_RED: Color = Color::srgb(0.85, 0.15, 0.15);
pub const HOT_COCOA_LIQUID: Color = Color::srgb(0.22, 0.12, 0.08);
pub const MOOSE_SNACK_GOLDEN_OAT: Color = Color::srgb(0.85, 0.55, 0.15);
pub const SNACK_JAR_GLASS: Color = Color::srgba(0.85, 0.94, 1.00, 0.45);

// --- Stove, Oven & Propane System ---
pub const GAS_HOSE_YELLOW: Color = Color::srgb(0.95, 0.78, 0.12);
pub const SAFETY_RED: Color = Color::srgb(0.85, 0.15, 0.12);
pub const PROPANE_CYLINDER_BLUE: Color = Color::srgb(0.20, 0.45, 0.85);
pub const BULK_TANK_WHITE: Color = Color::srgb(0.88, 0.90, 0.92);
pub const STOVE_GLOWING_EMBER: Color = Color::srgb(1.0, 0.35, 0.05);
pub const STOVE_COOL_GRAY: Color = Color::srgb(0.12, 0.13, 0.14);
pub const STOVE_BLUE_FLAME: Color = Color::srgba(0.2, 0.5, 1.0, 0.85);

// --- Wearables & Progression Tech ---
pub const BUNNY_EAR_PINK: Color = Color::srgb(0.98, 0.65, 0.75);
pub const PLASMA_CHAMBER_PURPLE: Color = Color::srgba(0.65, 0.15, 0.95, 0.75);

