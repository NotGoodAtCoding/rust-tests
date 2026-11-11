use bevy::{
    color::palettes::tailwind,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Activate, Button},
    window::{CursorGrabMode, CursorOptions},
};
use bevy_rapier3d::prelude::*;

use crate::elf_game::{
    player::{BhopConfig, controller::FirstPersonCamera, hands::{ToolItem, Grabbable}},
    wrapping::{Present, WrappingTool},
    workshop::{
        stove::{GasFurnaceStove, GasHoseCouplerNozzle, PropaneTank, PropaneReceptaclePlatform},
        pantry::{PantryDoor, MooseTreatJar},
        stable_door::{StableDoorTopHalf, StableDoorBottomHalf},
        baking::BakingSheet,
    },
    moose::toy_model::MooseSnackJar,
    physics_sandbox::{HatRack, ElfAssistant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuSubmenu {
    #[default]
    Main,
    Debug,
}

#[derive(Resource)]
pub struct MenuState {
    pub is_open: bool,
    pub active_submenu: MenuSubmenu,
    pub show_fps: bool,
    pub show_controls_hud: bool,
    pub show_physics_stats: bool,
    pub show_crosshair: bool,
    pub show_wireframes: bool,
    pub debug_overlay: bool,
    pub show_debug_overlay: bool,
    pub noclip: bool,
    pub mouse_sensitivity: f32,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_submenu: MenuSubmenu::Main,
            show_fps: true,
            show_controls_hud: true,
            show_physics_stats: true,
            show_crosshair: true,
            show_wireframes: false,
            debug_overlay: false,
            show_debug_overlay: false,
            noclip: false,
            mouse_sensitivity: 1.0,
        }
    }
}

pub const MIN_SENSITIVITY: f32 = 0.2;
pub const MAX_SENSITIVITY: f32 = 2.5;
pub const DEFAULT_SENSITIVITY: f32 = 1.0;
pub const SLIDER_TRACK_WIDTH: f32 = 220.0;
pub const SLIDER_THUMB_SIZE: f32 = 18.0;

pub fn sensitivity_to_norm(sens: f32) -> f32 {
    ((sens - MIN_SENSITIVITY) / (MAX_SENSITIVITY - MIN_SENSITIVITY)).clamp(0.0, 1.0)
}

pub fn norm_to_sensitivity(norm: f32) -> f32 {
    (MIN_SENSITIVITY + norm.clamp(0.0, 1.0) * (MAX_SENSITIVITY - MIN_SENSITIVITY)).clamp(MIN_SENSITIVITY, MAX_SENSITIVITY)
}

pub fn thumb_left_offset(sens: f32) -> f32 {
    sensitivity_to_norm(sens) * (SLIDER_TRACK_WIDTH - SLIDER_THUMB_SIZE)
}

pub fn fill_bar_width(sens: f32) -> f32 {
    sensitivity_to_norm(sens) * (SLIDER_TRACK_WIDTH - SLIDER_THUMB_SIZE) + (SLIDER_THUMB_SIZE / 2.0)
}

#[derive(Debug, Component, Default)]
pub struct SensitivitySlider;

#[derive(Debug, Component)]
pub struct SensitivitySliderTrack;

#[derive(Debug, Component)]
pub struct SensitivitySliderThumb;

#[derive(Debug, Component)]
pub struct SensitivitySliderFill;

#[derive(Debug, Component)]
pub struct SensitivityLabel;

#[derive(Debug, Component)]
pub struct MenuRoot;

#[derive(Debug, Component)]
pub struct MainMenuPanel;

#[derive(Debug, Component)]
pub struct DebugSubmenuPanel;

#[derive(Debug, Component)]
pub struct MainMenuTabButton;

#[derive(Debug, Component)]
pub struct DebugMenuTabButton;

#[derive(Debug, Component)]
pub struct MenuTitleText;

#[derive(Debug, Component)]
pub struct MenuSubtitleText;

#[derive(Debug, Component)]
pub struct FpsHud;

#[derive(Debug, Component)]
pub struct ControlsHud;

#[derive(Debug, Component)]
pub struct PhysicsHud;

#[derive(Debug, Component)]
pub struct CrosshairHud;

#[derive(Debug, Component)]
pub struct FpsText;

#[derive(Debug, Component)]
pub struct PhysicsStatsText;

#[derive(Debug, Component, Clone)]
pub struct DebugOverlayHud;

#[derive(Debug, Component, Clone)]
pub struct DebugOverlayText;

#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct DebugOverlayData {
    pub player_position: Vec3,
    pub crosshair_raycast_hit: Option<Vec3>,
    pub is_active: bool,
}

#[derive(Debug, Component)]
pub struct InteractionPromptText;

/// Marker component for UI text elements that display the current interactive object name.
#[derive(Debug, Component, Clone)]
pub struct InteractiveObjectNameText;

/// Optional custom interactive name tag on entities (overrides procedural name lookup).
#[derive(Component, Debug, Clone)]
pub struct InteractiveObject {
    pub name: String,
}

impl InteractiveObject {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Tracks the active interactive object within interaction range of the player.
#[derive(Resource, Debug, Default, Clone)]
pub struct CurrentInteractiveTarget {
    pub entity: Option<Entity>,
    pub name: Option<String>,
    pub distance: f32,
}

#[derive(Debug, Component, Clone, Copy, PartialEq, Eq)]
pub enum MenuButtonAction {
    SwitchSubmenu(MenuSubmenu),
    ToggleFps,
    ToggleControlsHud,
    TogglePhysicsStats,
    ToggleCrosshair,
    ToggleWireframes,
    ToggleNoclip,
    ToggleDebugOverlay,
    CycleBhopWindow,
    DecreaseSensitivity,
    IncreaseSensitivity,
    ResetSensitivity,
    ResumeGame,
}

fn menu_button(label: &'static str, action: MenuButtonAction) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        action,
        Node {
            width: Val::Percent(100.0),
            height: px(38),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BorderColor::all(Color::from(tailwind::SLATE_600)),
        BackgroundColor(Color::from(tailwind::SLATE_700)),
        children![(
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

pub fn spawn_ui(mut commands: Commands) {
    let font = TextFont {
        font_size: FontSize::Px(14.0),
        ..default()
    };

    // 1. Bottom-left controls overlay
    commands.spawn((
        ControlsHud,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            left: px(16),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(8)),
            row_gap: px(4),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.8)),
        BorderColor::all(Color::from(tailwind::SLATE_700)),
        Visibility::Visible,
        children![
            (
                Text::new("Santa's Workshop: Elf Simulator"),
                TextFont {
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::CYAN_300)),
            ),
            (
                Text::new("WASD: Move | Space: Jump (Perfect landing timing = BHOP boost up to 1.41x)"),
                font.clone()
            ),
            (
                Text::new("[Q]: Left Hand Grab | [E]: Right Hand Grab (Two hands for tape & ribbons)"),
                font.clone()
            ),
            (
                Text::new("Keep Mr. Moose fed at the trough before hunger reaches 60!"),
                font.clone()
            ),
            (
                Text::new("[TAB]: Toggle Options & Debug Menu (Free Cursor) | [V]: Toggle Noclip | [F3]: Debug Overlay"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::YELLOW_300)),
            ),
        ],
    ));

    // 2. Top-left physics sleep & performance stats
    commands.spawn((
        PhysicsHud,
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            padding: UiRect::all(px(10)),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.8)),
        BorderColor::all(Color::from(tailwind::SLATE_700)),
        Visibility::Visible,
        children![
            (
                PhysicsStatsText,
                Text::new("Physics: Calculating..."),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::EMERALD_300)),
            ),
        ],
    ));

    // 2b. Debug Overlay HUD (Player XYZ & Crosshair Raycast Target)
    commands.spawn((
        DebugOverlayHud,
        Node {
            position_type: PositionType::Absolute,
            top: px(68),
            left: px(16),
            padding: UiRect::all(px(10)),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.07, 0.12, 0.85)),
        BorderColor::all(Color::from(tailwind::AMBER_600)),
        Visibility::Hidden,
        children![
            (
                DebugOverlayText,
                Text::new("Player Pos: X: 0.00, Y: 0.00, Z: 0.00\nCrosshair Raycast: No Hit"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::AMBER_300)),
            ),
        ],
    ));

    // 3. Top-right FPS Counter
    commands.spawn((
        FpsHud,
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            right: px(16),
            padding: UiRect::all(px(10)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.85)),
        BorderColor::all(Color::from(tailwind::SLATE_700)),
        Visibility::Visible,
        children![
            (
                FpsText,
                Text::new("FPS: -- (0.0 ms)"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::GREEN_400)),
            ),
        ],
    ));

    // 4. Center-bottom interaction prompt
    commands.spawn((
        CrosshairHud,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(160),
            left: px(16),
            padding: UiRect::all(px(8)),
            ..default()
        },
        Visibility::Visible,
        children![
            (
                InteractionPromptText,
                InteractiveObjectNameText,
                Text::new("[Q] / [E] to grab presents & tools"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::AMBER_300)),
            ),
        ],
    ));

    // Center crosshair dot
    commands.spawn((
        CrosshairHud,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: px(4),
            height: px(4),
            margin: UiRect {
                left: px(-2),
                top: px(-2),
                right: px(0),
                bottom: px(0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.6)),
        Visibility::Visible,
    ));

    // 5. Centered Tab Options Menu (Modal overlay)
    commands.spawn((
        MenuRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: px(440),
            margin: UiRect {
                left: px(-220),
                top: px(-260),
                right: px(0),
                bottom: px(0),
            },
            padding: UiRect::all(px(20)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(8),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.95)),
        BorderColor::all(Color::from(tailwind::SLATE_500)),
        Visibility::Hidden,
        children![
            (
                MenuTitleText,
                Text::new("PAUSED - OPTIONS & MENU"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::AMBER_400)),
            ),
            (
                MenuSubtitleText,
                Text::new("Press [ESC], [P], [TAB], or click Resume to return to workshop"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::SLATE_400)),
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    height: px(1),
                    margin: UiRect::vertical(px(2)),
                    ..default()
                },
                BackgroundColor(Color::from(tailwind::SLATE_700)),
            ),
            // Top Submenu Navigation Tab Bar
            (
                Node {
                    width: Val::Percent(100.0),
                    height: px(34),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(8),
                    margin: UiRect::vertical(px(2)),
                    ..default()
                },
                children![
                    (
                        Button,
                        Hovered::default(),
                        MenuButtonAction::SwitchSubmenu(MenuSubmenu::Main),
                        MainMenuTabButton,
                        Node {
                            flex_grow: 1.0,
                            height: px(32),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BorderColor::all(Color::from(tailwind::CYAN_400)),
                        BackgroundColor(Color::from(tailwind::CYAN_700)),
                        children![(
                            Text::new("OPTIONS"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        )],
                    ),
                    (
                        Button,
                        Hovered::default(),
                        MenuButtonAction::SwitchSubmenu(MenuSubmenu::Debug),
                        DebugMenuTabButton,
                        Node {
                            flex_grow: 1.0,
                            height: px(32),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BorderColor::all(Color::from(tailwind::SLATE_600)),
                        BackgroundColor(Color::from(tailwind::SLATE_800)),
                        children![(
                            Text::new("DEBUG"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::from(tailwind::SLATE_300)),
                        )],
                    ),
                ],
            ),
            // 1. MAIN OPTIONS PANEL
            (
                MainMenuPanel,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(8),
                    display: Display::Flex,
                    ..default()
                },
                children![
                    // Mouse Sensitivity Slider Card
                    (
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(6),
                            padding: UiRect::all(px(8)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        BorderColor::all(Color::from(tailwind::SLATE_700)),
                        BackgroundColor(Color::srgba(0.04, 0.06, 0.10, 0.7)),
                        children![
                            (
                                SensitivityLabel,
                                Text::new(format!("Mouse Sensitivity: {:.2}x", DEFAULT_SENSITIVITY)),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::from(tailwind::CYAN_300)),
                            ),
                            (
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: px(8),
                                    ..default()
                                },
                                children![
                                    (
                                        Button,
                                        Hovered::default(),
                                        MenuButtonAction::DecreaseSensitivity,
                                        Node {
                                            width: px(28),
                                            height: px(26),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(px(1)),
                                            border_radius: BorderRadius::all(px(4)),
                                            ..default()
                                        },
                                        BorderColor::all(Color::from(tailwind::SLATE_600)),
                                        BackgroundColor(Color::from(tailwind::SLATE_700)),
                                        children![(
                                            Text::new("-"),
                                            TextFont {
                                                font_size: FontSize::Px(15.0),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        )],
                                    ),
                                    (
                                        SensitivitySlider,
                                        SensitivitySliderTrack,
                                        Hovered::default(),
                                        Node {
                                            width: px(SLIDER_TRACK_WIDTH),
                                            height: px(10),
                                            position_type: PositionType::Relative,
                                            align_items: AlignItems::Center,
                                            border_radius: BorderRadius::all(px(5)),
                                            border: UiRect::all(px(1)),
                                            ..default()
                                        },
                                        BorderColor::all(Color::from(tailwind::SLATE_600)),
                                        BackgroundColor(Color::from(tailwind::SLATE_800)),
                                        children![
                                            (
                                                SensitivitySlider,
                                                SensitivitySliderFill,
                                                Hovered::default(),
                                                Node {
                                                    width: px(fill_bar_width(DEFAULT_SENSITIVITY)),
                                                    height: px(10),
                                                    position_type: PositionType::Absolute,
                                                    left: px(0),
                                                    border_radius: BorderRadius::all(px(5)),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::from(tailwind::CYAN_600)),
                                            ),
                                            (
                                                SensitivitySlider,
                                                SensitivitySliderThumb,
                                                Hovered::default(),
                                                Node {
                                                    width: px(SLIDER_THUMB_SIZE),
                                                    height: px(SLIDER_THUMB_SIZE),
                                                    position_type: PositionType::Absolute,
                                                    left: px(thumb_left_offset(DEFAULT_SENSITIVITY)),
                                                    top: px(-4),
                                                    border_radius: BorderRadius::all(px(SLIDER_THUMB_SIZE / 2.0)),
                                                    border: UiRect::all(px(2)),
                                                    ..default()
                                                },
                                                BorderColor::all(Color::WHITE),
                                                BackgroundColor(Color::from(tailwind::CYAN_400)),
                                            ),
                                        ],
                                    ),
                                    (
                                        Button,
                                        Hovered::default(),
                                        MenuButtonAction::IncreaseSensitivity,
                                        Node {
                                            width: px(28),
                                            height: px(26),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(px(1)),
                                            border_radius: BorderRadius::all(px(4)),
                                            ..default()
                                        },
                                        BorderColor::all(Color::from(tailwind::SLATE_600)),
                                        BackgroundColor(Color::from(tailwind::SLATE_700)),
                                        children![(
                                            Text::new("+"),
                                            TextFont {
                                                font_size: FontSize::Px(15.0),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        )],
                                    ),
                                    (
                                        Button,
                                        Hovered::default(),
                                        MenuButtonAction::ResetSensitivity,
                                        Node {
                                            width: px(46),
                                            height: px(26),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(px(1)),
                                            border_radius: BorderRadius::all(px(4)),
                                            ..default()
                                        },
                                        BorderColor::all(Color::from(tailwind::SLATE_600)),
                                        BackgroundColor(Color::from(tailwind::SLATE_700)),
                                        children![(
                                            Text::new("Reset"),
                                            TextFont {
                                                font_size: FontSize::Px(11.0),
                                                ..default()
                                            },
                                            TextColor(Color::from(tailwind::SLATE_300)),
                                        )],
                                    ),
                                ],
                            ),
                        ],
                    ),
                    menu_button("FPS Meter: ON", MenuButtonAction::ToggleFps),
                    menu_button("Controls Guide: ON", MenuButtonAction::ToggleControlsHud),
                    menu_button("Physics Stats: ON", MenuButtonAction::TogglePhysicsStats),
                    menu_button("Crosshair & Prompts: ON", MenuButtonAction::ToggleCrosshair),
                    menu_button("DEBUG Submenu (BHOP, Noclip, Wireframe) ->", MenuButtonAction::SwitchSubmenu(MenuSubmenu::Debug)),
                    menu_button("Resume Game", MenuButtonAction::ResumeGame),
                ],
            ),
            // 2. DEBUG SUBMENU PANEL
            (
                DebugSubmenuPanel,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(8),
                    display: Display::None,
                    ..default()
                },
                children![
                    (
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::all(px(6)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.8, 0.4, 0.0, 0.2)),
                        children![(
                            Text::new("DEBUG SUBMENU: Physics & Movement"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::from(tailwind::AMBER_300)),
                        )],
                    ),
                    menu_button("BHOP Timing: 1/60s (Strict)", MenuButtonAction::CycleBhopWindow),
                    menu_button("Noclip Mode: OFF", MenuButtonAction::ToggleNoclip),
                    menu_button("Wireframe Mode: OFF", MenuButtonAction::ToggleWireframes),
                    menu_button("Debug Overlay (XYZ / Raycast): OFF", MenuButtonAction::ToggleDebugOverlay),
                    menu_button("<- Back to Options", MenuButtonAction::SwitchSubmenu(MenuSubmenu::Main)),
                    menu_button("Resume Game", MenuButtonAction::ResumeGame),
                ],
            ),
        ],
    ));
}

pub fn toggle_menu(
    keyboard_input: Option<Res<ButtonInput<KeyCode>>>,
    mut menu_state: ResMut<MenuState>,
    mut cursor_query: Query<&mut CursorOptions, With<Window>>,
) {
    let Some(keys) = keyboard_input else {
        return;
    };
    if keys.any_just_pressed([KeyCode::Tab, KeyCode::Escape, KeyCode::KeyP, KeyCode::Pause]) {
        menu_state.is_open = !menu_state.is_open;

        for mut cursor in &mut cursor_query {
            if menu_state.is_open {
                cursor.visible = true;
                cursor.grab_mode = CursorGrabMode::None;
            } else {
                cursor.visible = false;
                cursor.grab_mode = CursorGrabMode::Locked;
            }
        }
    }
}

/// Synchronizes the menu open / pause state with virtual time, Rapier physics, and GameState.
pub fn sync_pause_state_system(
    menu_state: Res<MenuState>,
    mut next_game_state: Option<ResMut<NextState<crate::elf_game::core::GameState>>>,
    mut virtual_time: Option<ResMut<Time<Virtual>>>,
    mut rapier_configs: Query<&mut RapierConfiguration>,
) {
    if !menu_state.is_changed() {
        return;
    }

    let is_paused = menu_state.is_open;

    if let Some(ref mut next_state) = next_game_state {
        if is_paused {
            next_state.set(crate::elf_game::core::GameState::Paused);
        } else {
            next_state.set(crate::elf_game::core::GameState::InWorkshop);
        }
    }

    if let Some(ref mut time) = virtual_time {
        if is_paused {
            time.pause();
        } else {
            time.unpause();
        }
    }

    for mut config in &mut rapier_configs {
        config.physics_pipeline_active = !is_paused;
    }
}

pub fn handle_mouse_grab(
    mouse_button: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Res<MenuState>,
    mut cursor_query: Query<&mut CursorOptions, With<Window>>,
) {
    let Some(mouse) = mouse_button else {
        return;
    };
    if !menu_state.is_open && mouse.just_pressed(MouseButton::Left) {
        for mut cursor in &mut cursor_query {
            cursor.visible = false;
            cursor.grab_mode = CursorGrabMode::Locked;
        }
    }
}

pub fn on_button_activate(
    event: On<Activate>,
    action_query: Query<&MenuButtonAction, With<Button>>,
    mut menu_state: ResMut<MenuState>,
    mut bhop_config: Option<ResMut<BhopConfig>>,
    mut cursor_query: Query<&mut CursorOptions, With<Window>>,
) {
    if let Ok(action) = action_query.get(event.entity) {
        match action {
            MenuButtonAction::SwitchSubmenu(sub) => {
                menu_state.active_submenu = *sub;
            }
            MenuButtonAction::CycleBhopWindow => {
                if let Some(ref mut config) = bhop_config {
                    config.window = config.window.next();
                }
            }
            MenuButtonAction::ToggleNoclip => {
                menu_state.noclip = !menu_state.noclip;
            }
            MenuButtonAction::ToggleWireframes => {
                menu_state.show_wireframes = !menu_state.show_wireframes;
            }
            MenuButtonAction::ToggleFps => {
                menu_state.show_fps = !menu_state.show_fps;
            }
            MenuButtonAction::ToggleControlsHud => {
                menu_state.show_controls_hud = !menu_state.show_controls_hud;
            }
            MenuButtonAction::TogglePhysicsStats => {
                menu_state.show_physics_stats = !menu_state.show_physics_stats;
            }
            MenuButtonAction::ToggleCrosshair => {
                menu_state.show_crosshair = !menu_state.show_crosshair;
            }
            MenuButtonAction::ToggleDebugOverlay => {
                menu_state.debug_overlay = !menu_state.debug_overlay;
                menu_state.show_debug_overlay = menu_state.debug_overlay;
            }
            MenuButtonAction::DecreaseSensitivity => {
                menu_state.mouse_sensitivity = (menu_state.mouse_sensitivity - 0.1).max(MIN_SENSITIVITY);
            }
            MenuButtonAction::IncreaseSensitivity => {
                menu_state.mouse_sensitivity = (menu_state.mouse_sensitivity + 0.1).min(MAX_SENSITIVITY);
            }
            MenuButtonAction::ResetSensitivity => {
                menu_state.mouse_sensitivity = DEFAULT_SENSITIVITY;
            }
            MenuButtonAction::ResumeGame => {
                menu_state.is_open = false;
                for mut cursor in &mut cursor_query {
                    cursor.visible = false;
                    cursor.grab_mode = CursorGrabMode::Locked;
                }
            }
        }
    }
}

pub fn on_slider_thumb_drag(
    event: On<PointerDrag>,
    mut menu_state: ResMut<MenuState>,
    slider_query: Query<
        Entity,
        Or<(
            With<SensitivitySlider>,
            With<SensitivitySliderTrack>,
            With<SensitivitySliderThumb>,
            With<SensitivitySliderFill>,
        )>,
    >,
) {
    if !menu_state.is_open || menu_state.active_submenu != MenuSubmenu::Main {
        return;
    }

    if event.button != PointerButton::Primary {
        return;
    }

    if !slider_query.contains(event.entity) {
        return;
    }

    let usable_width = SLIDER_TRACK_WIDTH - SLIDER_THUMB_SIZE;
    let delta_norm = event.delta.x / usable_width;
    let old_norm = sensitivity_to_norm(menu_state.mouse_sensitivity);
    let new_norm = (old_norm + delta_norm).clamp(0.0, 1.0);
    menu_state.mouse_sensitivity = norm_to_sensitivity(new_norm);
}

pub fn on_slider_thumb_over(
    event: On<PointerOver>,
    slider_query: Query<
        Entity,
        Or<(
            With<SensitivitySlider>,
            With<SensitivitySliderTrack>,
            With<SensitivitySliderThumb>,
            With<SensitivitySliderFill>,
        )>,
    >,
    mut query: Query<&mut BackgroundColor, With<SensitivitySliderThumb>>,
) {
    if slider_query.contains(event.entity) {
        for mut bg in query.iter_mut() {
            *bg = BackgroundColor(Color::from(tailwind::CYAN_300));
        }
    }
}

pub fn on_slider_thumb_out(
    event: On<PointerOut>,
    slider_query: Query<
        Entity,
        Or<(
            With<SensitivitySlider>,
            With<SensitivitySliderTrack>,
            With<SensitivitySliderThumb>,
            With<SensitivitySliderFill>,
        )>,
    >,
    mut query: Query<&mut BackgroundColor, With<SensitivitySliderThumb>>,
) {
    if slider_query.contains(event.entity) {
        for mut bg in query.iter_mut() {
            *bg = BackgroundColor(Color::from(tailwind::CYAN_400));
        }
    }
}

pub fn sync_sensitivity_slider(
    menu_state: Res<MenuState>,
    mut thumb_query: Query<&mut Node, (With<SensitivitySliderThumb>, Without<SensitivitySliderFill>)>,
    mut fill_query: Query<&mut Node, (With<SensitivitySliderFill>, Without<SensitivitySliderThumb>)>,
    mut label_query: Query<&mut Text, With<SensitivityLabel>>,
) {
    if !menu_state.is_changed() {
        return;
    }

    for mut thumb in thumb_query.iter_mut() {
        thumb.left = px(thumb_left_offset(menu_state.mouse_sensitivity));
    }

    for mut fill in fill_query.iter_mut() {
        fill.width = px(fill_bar_width(menu_state.mouse_sensitivity));
    }

    for mut text in label_query.iter_mut() {
        text.0 = format!("Mouse Sensitivity: {:.2}x", menu_state.mouse_sensitivity);
    }
}

pub fn update_button_visuals(
    menu_state: Res<MenuState>,
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
            &MenuButtonAction,
        ),
        With<Button>,
    >,
) {
    for (hovered, pressed, mut bg, mut border, action) in &mut buttons {
        // Tab buttons styling is kept in sync with active submenu
        if let MenuButtonAction::SwitchSubmenu(sub) = *action {
            let is_active = sub == menu_state.active_submenu;
            if is_active {
                if sub == MenuSubmenu::Main {
                    *bg = BackgroundColor(Color::from(tailwind::CYAN_700));
                    border.set_all(Color::from(tailwind::CYAN_400));
                } else {
                    *bg = BackgroundColor(Color::from(tailwind::AMBER_700));
                    border.set_all(Color::from(tailwind::AMBER_400));
                }
            } else if hovered.get() {
                *bg = BackgroundColor(Color::from(tailwind::SLATE_700));
                border.set_all(Color::from(tailwind::SLATE_400));
            } else {
                *bg = BackgroundColor(Color::from(tailwind::SLATE_800));
                border.set_all(Color::from(tailwind::SLATE_600));
            }
            continue;
        }

        let is_resume = *action == MenuButtonAction::ResumeGame;
        let is_debug_entry = *action == MenuButtonAction::SwitchSubmenu(MenuSubmenu::Debug);

        if pressed {
            *bg = BackgroundColor(Color::from(tailwind::BLUE_900));
            border.set_all(Color::from(tailwind::BLUE_400));
        } else if hovered.get() {
            *bg = if is_resume {
                BackgroundColor(Color::from(tailwind::BLUE_600))
            } else if is_debug_entry {
                BackgroundColor(Color::from(tailwind::AMBER_600))
            } else {
                BackgroundColor(Color::from(tailwind::SLATE_600))
            };
            border.set_all(Color::from(tailwind::SLATE_400));
        } else {
            *bg = if is_resume {
                BackgroundColor(Color::from(tailwind::BLUE_700))
            } else if is_debug_entry {
                BackgroundColor(Color::from(tailwind::AMBER_800))
            } else {
                BackgroundColor(Color::from(tailwind::SLATE_700))
            };
            border.set_all(Color::from(tailwind::SLATE_600));
        }
    }
}

pub fn update_menu_button_labels(
    menu_state: Res<MenuState>,
    bhop_config: Option<Res<BhopConfig>>,
    button_query: Query<(&MenuButtonAction, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    let bhop_changed = bhop_config.as_ref().map(|c| c.is_changed()).unwrap_or(false);
    if !menu_state.is_changed() && !bhop_changed {
        return;
    }

    for (action, children) in button_query.iter() {
        if let Some(&child) = children.first() {
            if let Ok(mut text) = text_query.get_mut(child) {
                match action {
                    MenuButtonAction::CycleBhopWindow => {
                        if let Some(ref config) = bhop_config {
                            text.0 = format!("BHOP Timing: {}", config.window.label());
                        }
                    }
                    MenuButtonAction::ToggleNoclip => {
                        text.0 = format!("Noclip Mode: {}", if menu_state.noclip { "ON (Flying)" } else { "OFF" });
                    }
                    MenuButtonAction::ToggleWireframes => {
                        text.0 = format!("Wireframe Mode: {}", if menu_state.show_wireframes { "ON" } else { "OFF" });
                    }
                    MenuButtonAction::ToggleFps => {
                        text.0 = format!("FPS Meter: {}", if menu_state.show_fps { "ON" } else { "OFF" });
                    }
                    MenuButtonAction::ToggleControlsHud => {
                        text.0 = format!("Controls Guide: {}", if menu_state.show_controls_hud { "ON" } else { "OFF" });
                    }
                    MenuButtonAction::TogglePhysicsStats => {
                        text.0 = format!("Physics Stats: {}", if menu_state.show_physics_stats { "ON" } else { "OFF" });
                    }
                    MenuButtonAction::ToggleCrosshair => {
                        text.0 = format!("Crosshair & Prompts: {}", if menu_state.show_crosshair { "ON" } else { "OFF" });
                    }
                    MenuButtonAction::ToggleDebugOverlay => {
                        text.0 = format!("Debug Overlay (XYZ / Raycast): {}", if menu_state.debug_overlay || menu_state.show_debug_overlay { "ON" } else { "OFF" });
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn sync_submenu_visibility(
    menu_state: Res<MenuState>,
    mut title_query: Query<&mut Text, (With<MenuTitleText>, Without<MenuSubtitleText>)>,
    mut subtitle_query: Query<&mut Text, (With<MenuSubtitleText>, Without<MenuTitleText>)>,
    mut main_panel: Query<&mut Node, (With<MainMenuPanel>, Without<DebugSubmenuPanel>)>,
    mut debug_panel: Query<&mut Node, (With<DebugSubmenuPanel>, Without<MainMenuPanel>)>,
    mut main_tab: Query<(&mut BackgroundColor, &mut BorderColor), (With<MainMenuTabButton>, Without<DebugMenuTabButton>)>,
    mut debug_tab: Query<(&mut BackgroundColor, &mut BorderColor), (With<DebugMenuTabButton>, Without<MainMenuTabButton>)>,
) {
    if !menu_state.is_changed() {
        return;
    }

    let is_main = menu_state.active_submenu == MenuSubmenu::Main;

    for mut title in title_query.iter_mut() {
        title.0 = if is_main {
            "PAUSED - OPTIONS & MENU".to_string()
        } else {
            "PAUSED - DEBUG MENU".to_string()
        };
    }

    for mut sub in subtitle_query.iter_mut() {
        sub.0 = if is_main {
            "Press [ESC], [P], [TAB], or click Resume to return to workshop".to_string()
        } else {
            "Physics, Locomotion & Collision Debugging Tools".to_string()
        };
    }

    for mut node in main_panel.iter_mut() {
        node.display = if is_main { Display::Flex } else { Display::None };
    }
    for mut node in debug_panel.iter_mut() {
        node.display = if !is_main { Display::Flex } else { Display::None };
    }

    for (mut bg, mut border) in main_tab.iter_mut() {
        if is_main {
            *bg = BackgroundColor(Color::from(tailwind::CYAN_700));
            border.set_all(Color::from(tailwind::CYAN_400));
        } else {
            *bg = BackgroundColor(Color::from(tailwind::SLATE_800));
            border.set_all(Color::from(tailwind::SLATE_600));
        }
    }

    for (mut bg, mut border) in debug_tab.iter_mut() {
        if !is_main {
            *bg = BackgroundColor(Color::from(tailwind::AMBER_700));
            border.set_all(Color::from(tailwind::AMBER_400));
        } else {
            *bg = BackgroundColor(Color::from(tailwind::SLATE_800));
            border.set_all(Color::from(tailwind::SLATE_600));
        }
    }
}

pub fn sync_hud_visibility(
    menu_state: Res<MenuState>,
    wireframe_config: Option<ResMut<bevy::pbr::wireframe::WireframeConfig>>,
    debug_render_context: Option<ResMut<DebugRenderContext>>,
    mut menu_query: Query<&mut Visibility, (With<MenuRoot>, Without<FpsHud>, Without<ControlsHud>, Without<PhysicsHud>, Without<CrosshairHud>, Without<DebugOverlayHud>)>,
    mut fps_query: Query<&mut Visibility, (With<FpsHud>, Without<MenuRoot>, Without<ControlsHud>, Without<PhysicsHud>, Without<CrosshairHud>, Without<DebugOverlayHud>)>,
    mut controls_query: Query<&mut Visibility, (With<ControlsHud>, Without<MenuRoot>, Without<FpsHud>, Without<PhysicsHud>, Without<CrosshairHud>, Without<DebugOverlayHud>)>,
    mut physics_query: Query<&mut Visibility, (With<PhysicsHud>, Without<MenuRoot>, Without<FpsHud>, Without<ControlsHud>, Without<CrosshairHud>, Without<DebugOverlayHud>)>,
    mut crosshair_query: Query<&mut Visibility, (With<CrosshairHud>, Without<MenuRoot>, Without<FpsHud>, Without<ControlsHud>, Without<PhysicsHud>, Without<DebugOverlayHud>)>,
    mut debug_overlay_query: Query<&mut Visibility, (With<DebugOverlayHud>, Without<MenuRoot>, Without<FpsHud>, Without<ControlsHud>, Without<PhysicsHud>, Without<CrosshairHud>)>,
) {
    if !menu_state.is_changed() {
        return;
    }

    if let Some(mut wireframe) = wireframe_config {
        wireframe.global = menu_state.show_wireframes;
    }

    if let Some(mut debug_render) = debug_render_context {
        debug_render.enabled = menu_state.show_wireframes;
    }

    for mut vis in menu_query.iter_mut() {
        *vis = if menu_state.is_open { Visibility::Visible } else { Visibility::Hidden };
    }

    for mut vis in fps_query.iter_mut() {
        *vis = if menu_state.show_fps { Visibility::Visible } else { Visibility::Hidden };
    }

    for mut vis in controls_query.iter_mut() {
        *vis = if menu_state.show_controls_hud { Visibility::Visible } else { Visibility::Hidden };
    }

    for mut vis in physics_query.iter_mut() {
        *vis = if menu_state.show_physics_stats { Visibility::Visible } else { Visibility::Hidden };
    }

    for mut vis in crosshair_query.iter_mut() {
        *vis = if menu_state.show_crosshair { Visibility::Visible } else { Visibility::Hidden };
    }

    for mut vis in debug_overlay_query.iter_mut() {
        *vis = if menu_state.debug_overlay || menu_state.show_debug_overlay { Visibility::Visible } else { Visibility::Hidden };
    }
}

pub fn update_fps(
    diagnostics: Option<Res<DiagnosticsStore>>,
    mut fps_query: Query<&mut Text, With<FpsText>>,
) {
    let Some(diagnostics) = diagnostics else {
        return;
    };
    if let Some(fps_diag) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps_val) = fps_diag.smoothed() {
            let ft_val = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                .and_then(|ft| ft.smoothed())
                .unwrap_or(0.0);
            for mut text in &mut fps_query {
                text.0 = format!("FPS: {:.0} ({:.1} ms)", fps_val, ft_val);
            }
        }
    }
}

pub fn update_hud_stats(
    presents: Query<Option<&Sleeping>, With<Present>>,
    mut stats_query: Query<&mut Text, With<PhysicsStatsText>>,
) {
    let total = presents.iter().count();
    let sleeping_count = presents.iter().filter(|s| s.map(|sp| sp.sleeping).unwrap_or(false)).count();
    let active_count = total - sleeping_count;

    for mut text in &mut stats_query {
        text.0 = format!(
            "Presents: {} Active | {} Settled (CPU Idle)",
            active_count, sleeping_count
        );
    }
}

/// Keybinding system toggling debug overlay mode when F3 is pressed.
pub fn toggle_debug_overlay_system(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    menu_state: Option<ResMut<MenuState>>,
) {
    let (Some(keys), Some(mut menu_state)) = (keys, menu_state) else {
        return;
    };
    if keys.just_pressed(KeyCode::F3) {
        menu_state.debug_overlay = !menu_state.debug_overlay;
        menu_state.show_debug_overlay = menu_state.debug_overlay;
    }
}

/// Updates the debug overlay HUD, calculating the player's XYZ coordinates
/// and raycasting along the crosshair forward vector to find the collision point.
pub fn update_debug_overlay_system(
    menu_state: Option<Res<MenuState>>,
    mut overlay_data: Option<ResMut<DebugOverlayData>>,
    q_player: Query<
        &Transform,
        (
            Or<(With<crate::elf_game::player::controller::ElfPlayer>, With<crate::elf_game::player::ElfMovement>)>,
            Without<FirstPersonCamera>,
        ),
    >,
    q_camera: Query<(&GlobalTransform, Option<&Transform>), With<FirstPersonCamera>>,
    q_colliders: Query<
        (Entity, &Transform, Option<&GlobalTransform>, &Collider),
        (
            Without<crate::elf_game::player::controller::ElfPlayer>,
            Without<crate::elf_game::player::ElfMovement>,
            Without<Sensor>,
            Without<ColliderDisabled>,
            Without<FirstPersonCamera>,
        ),
    >,
    mut text_query: Query<&mut Text, With<DebugOverlayText>>,
) {
    let is_enabled = menu_state
        .as_ref()
        .map(|m| m.debug_overlay || m.show_debug_overlay)
        .unwrap_or(false);

    if let Some(ref mut data) = overlay_data {
        data.is_active = is_enabled;
    }

    if !is_enabled && text_query.is_empty() {
        return;
    }

    let player_pos = q_player.iter().next().map(|tf| tf.translation).unwrap_or(Vec3::ZERO);

    // Compute ray origin and direction from first person camera
    let (ray_origin, ray_dir) = if let Some((cam_gt, opt_tf)) = q_camera.iter().next() {
        let origin = cam_gt.translation();
        let fwd = cam_gt.forward();
        let dir = if fwd.length_squared() > 0.001 {
            Vec3::from(fwd).normalize()
        } else if let Some(tf) = opt_tf {
            Vec3::from(tf.forward()).normalize()
        } else {
            Vec3::NEG_Z
        };
        (origin, dir)
    } else {
        (player_pos + Vec3::new(0.0, 0.6, 0.0), Vec3::NEG_Z)
    };

    // Cast ray against colliders
    let max_reach = 100.0;
    let mut closest_hit: Option<(f32, Vec3)> = None;

    for (_e, tf, opt_gt, collider) in &q_colliders {
        let (pos, rot) = if let Some(gt) = opt_gt {
            let comp = gt.compute_transform();
            if comp.translation.length_squared() > 0.0001 || tf.translation.length_squared() < 0.0001 {
                (gt.translation(), comp.rotation)
            } else {
                (tf.translation, tf.rotation)
            }
        } else {
            (tf.translation, tf.rotation)
        };

        if let Some(toi) = collider.cast_ray(pos, rot, ray_origin, ray_dir, max_reach, true) {
            if toi > 0.001 {
                if closest_hit.as_ref().map(|(min_t, _)| toi < *min_t).unwrap_or(true) {
                    let hit_point = ray_origin + ray_dir * toi;
                    closest_hit = Some((toi, hit_point));
                }
            }
        }
    }

    let hit_point_opt = closest_hit.map(|(_, pt)| pt);

    if let Some(ref mut data) = overlay_data {
        data.player_position = player_pos;
        data.crosshair_raycast_hit = hit_point_opt;
    }

    for mut text in &mut text_query {
        let hit_str = if let Some(hit) = hit_point_opt {
            format!("X: {:.2}, Y: {:.2}, Z: {:.2}", hit.x, hit.y, hit.z)
        } else {
            "No Hit".to_string()
        };

        text.0 = format!(
            "Player Pos: X: {:.2}, Y: {:.2}, Z: {:.2}\nCrosshair Raycast: {}",
            player_pos.x, player_pos.y, player_pos.z, hit_str
        );
    }
}

pub fn clean_interactive_name(raw: &str) -> String {
    match raw {
        "WorkbenchToyPresent" | "Present" => "Present".to_string(),
        "GasFurnaceStove" | "Stove" | "Oven" | "StoveBodyIron" => "Oven".to_string(),
        "PantryPhysicsDoor" | "PantryDoor" => "Pantry Door".to_string(),
        "StableDoorTopHalf" | "StableDoorBottomHalf" | "StableDoor" => "Stable Door".to_string(),
        "YellowHoseCouplerNozzle" | "GasHose" => "Gas Hose".to_string(),
        "PropaneWallValve" | "PropaneGasValve" => "Propane Gas Valve".to_string(),
        "PropaneTank" | "HouseConnectedPropaneTank" => "Propane Tank".to_string(),
        "PantryMooseTreatJar" | "MooseSnackJar" | "MooseTreatJar" => "Moose Treat Jar".to_string(),
        "BakingSheet" | "CookieBakingSheet" => "Baking Sheet".to_string(),
        "HatRack" => "Hat Rack".to_string(),
        "VacuumActivationButton" | "VacuumButton" => "Vacuum Button".to_string(),
        "AutoWrapperTool" => "Auto Wrapper".to_string(),
        "TinselHandCannon" => "Tinsel Hand Cannon".to_string(),
        "BunnyEarsItem" => "Bunny Ears".to_string(),
        other => {
            let s = other
                .trim_start_matches("Pantry")
                .trim_start_matches("Workshop");
            s.to_string()
        }
    }
}

type InteractiveQueryData = (
    Entity,
    &'static Transform,
    (
        Option<&'static Name>,
        Option<&'static InteractiveObject>,
        Option<&'static Present>,
        Option<&'static GasFurnaceStove>,
    ),
    (
        Option<&'static PantryDoor>,
        Option<&'static StableDoorTopHalf>,
        Option<&'static StableDoorBottomHalf>,
        Option<&'static GasHoseCouplerNozzle>,
    ),
    (
        Option<&'static PropaneReceptaclePlatform>,
        Option<&'static PropaneTank>,
        Option<&'static BakingSheet>,
        Option<&'static MooseSnackJar>,
    ),
    (
        Option<&'static MooseTreatJar>,
        Option<&'static HatRack>,
        Option<&'static ElfAssistant>,
        Option<&'static ToolItem>,
        Option<&'static Grabbable>,
    ),
);

/// Detects interactive objects within reach of the player and displays the object's text name on the HUD.
pub fn update_interactive_object_name_display_system(
    menu_state: Option<Res<MenuState>>,
    active_dialogue: Option<Res<crate::elf_game::dialogue::ActiveDialogue>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut target_res: Option<ResMut<CurrentInteractiveTarget>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    q_player: Query<&Transform, With<crate::elf_game::player::controller::ElfPlayer>>,
    mut q_prompts: Query<&mut Text, Or<(With<InteractionPromptText>, With<InteractiveObjectNameText>)>>,
    q_interactive: Query<InteractiveQueryData>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }
    if active_dialogue.as_ref().map(|d| d.is_active).unwrap_or(false) {
        return;
    }

    // Determine ray origin and forward direction
    let (cam_pos, cam_fwd) = if let Ok(cam_gt) = q_camera.single() {
        (cam_gt.translation(), cam_gt.forward().as_vec3())
    } else if let Some(p_tf) = q_player.iter().next() {
        (p_tf.translation + Vec3::Y * 1.5, *p_tf.forward())
    } else {
        return;
    };

    let max_reach: f32 = 2.8;
    let mut best_target: Option<(Entity, String, f32)> = None;

    for (
        entity,
        tf,
        (opt_name, opt_custom, opt_present, opt_stove),
        (opt_pantry_door, opt_top_door, opt_bottom_door, opt_nozzle),
        (opt_platform, opt_tank, opt_baking, opt_snack_jar),
        (opt_treat_jar, opt_hat_rack, opt_elf, opt_tool, opt_grabbable),
    ) in &q_interactive {
        // Must be an interactive object
        let is_interactive = opt_custom.is_some()
            || opt_present.is_some()
            || opt_stove.is_some()
            || opt_pantry_door.is_some()
            || opt_top_door.is_some()
            || opt_bottom_door.is_some()
            || opt_nozzle.is_some()
            || opt_platform.is_some()
            || opt_tank.is_some()
            || opt_baking.is_some()
            || opt_snack_jar.is_some()
            || opt_treat_jar.is_some()
            || opt_hat_rack.is_some()
            || opt_elf.is_some()
            || opt_tool.is_some()
            || opt_grabbable.is_some();

        if !is_interactive {
            continue;
        }

        let obj_pos = tf.translation;
        let dist = cam_pos.distance(obj_pos);
        if dist > max_reach {
            continue;
        }

        let to_obj = if dist > 0.001 { (obj_pos - cam_pos) / dist } else { cam_fwd };
        let dot = cam_fwd.dot(to_obj);
        if dot < 0.20 && dist > 1.2 {
            continue;
        }

        // Determine object display name
        let name = if let Some(custom) = opt_custom {
            custom.name.clone()
        } else if opt_stove.is_some() {
            "Oven".to_string()
        } else if opt_present.is_some() {
            "Present".to_string()
        } else if opt_pantry_door.is_some() {
            "Pantry Door".to_string()
        } else if opt_top_door.is_some() || opt_bottom_door.is_some() {
            "Stable Door".to_string()
        } else if opt_nozzle.is_some() {
            "Gas Hose".to_string()
        } else if opt_platform.is_some() {
            "Propane Platform".to_string()
        } else if opt_tank.is_some() {
            "Propane Tank".to_string()
        } else if opt_baking.is_some() {
            "Baking Sheet".to_string()
        } else if opt_snack_jar.is_some() || opt_treat_jar.is_some() {
            "Moose Treat Jar".to_string()
        } else if opt_hat_rack.is_some() {
            "Hat Rack".to_string()
        } else if let Some(elf) = opt_elf {
            elf.name.clone()
        } else if let Some(tool) = opt_tool {
            match tool.tool {
                WrappingTool::Scissors => "Scissors".to_string(),
                WrappingTool::WrappingPaperRoll => "Wrapping Paper Roll".to_string(),
                WrappingTool::TapeDispenser => "Tape Dispenser".to_string(),
                WrappingTool::RibbonSpool => "Ribbon Spool".to_string(),
                WrappingTool::DecorativeBowBox => "Bow Box".to_string(),
                WrappingTool::GiftTagPen => "Gift Tag Pen".to_string(),
                WrappingTool::ForbiddenPlasmaShrink => "Plasma Shrink Wrapper".to_string(),
                WrappingTool::AutoWrapper => "Auto Wrapper".to_string(),
            }
        } else if let Some(n) = opt_name {
            clean_interactive_name(n.as_str())
        } else {
            "Item".to_string()
        };

        // Score prioritizing alignment and closer distance
        let score = dot + (1.0 - (dist / max_reach));
        let is_better = match &best_target {
            None => true,
            Some((_, _, best_score)) => score > *best_score,
        };

        if is_better {
            best_target = Some((entity, name, score));
        }
    }

    if let Some((entity, name, _)) = best_target {
        let dist = cam_pos.distance(q_interactive.get(entity).map(|(_, tf, ..)| tf.translation).unwrap_or(cam_pos));
        if let Some(ref mut res) = target_res {
            res.entity = Some(entity);
            res.name = Some(name.clone());
            res.distance = dist;
        }
        for mut prompt in &mut q_prompts {
            prompt.0 = name.clone();
        }
    } else {
        if let Some(ref mut res) = target_res {
            res.entity = None;
            res.name = None;
            res.distance = 0.0;
        }
        for mut prompt in &mut q_prompts {
            prompt.0 = "[Q] / [E] to grab presents & tools".to_string();
        }
    }
}

pub struct ElfUiPlugin;

impl Plugin for ElfUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .init_resource::<CurrentInteractiveTarget>()
            .init_resource::<DebugOverlayData>()
            .add_observer(on_button_activate)
            .add_observer(on_slider_thumb_drag)
            .add_observer(on_slider_thumb_over)
            .add_observer(on_slider_thumb_out)
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (
                    toggle_menu,
                    toggle_debug_overlay_system,
                    sync_pause_state_system,
                    sync_submenu_visibility,
                    handle_mouse_grab,
                    update_button_visuals,
                    update_menu_button_labels,
                    sync_hud_visibility,
                    sync_sensitivity_slider,
                    update_fps,
                    update_hud_stats,
                    update_debug_overlay_system,
                ),
            )
            .add_systems(Update, update_interactive_object_name_display_system);
    }
}
