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
    player::BhopConfig,
    wrapping::Present,
};

#[derive(Resource)]
pub struct MenuState {
    pub is_open: bool,
    pub show_fps: bool,
    pub show_controls_hud: bool,
    pub show_physics_stats: bool,
    pub show_crosshair: bool,
    pub show_wireframes: bool,
    pub mouse_sensitivity: f32,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            is_open: false,
            show_fps: true,
            show_controls_hud: true,
            show_physics_stats: true,
            show_crosshair: true,
            show_wireframes: true,
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

#[derive(Debug, Component)]
pub struct InteractionPromptText;

#[derive(Debug, Component, Clone, Copy, PartialEq, Eq)]
pub enum MenuButtonAction {
    ToggleFps,
    ToggleControlsHud,
    TogglePhysicsStats,
    ToggleCrosshair,
    ToggleWireframes,
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
                Text::new("[TAB]: Toggle Options & Debug Menu (Free Cursor)"),
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
            width: px(420),
            margin: UiRect {
                left: px(-210),
                top: px(-280),
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
                Text::new("PAUSED - OPTIONS & MENU"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::from(tailwind::AMBER_400)),
            ),
            (
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
                                SensitivitySliderTrack,
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
                                        SensitivitySliderFill,
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
                                        SensitivitySliderThumb,
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
            // Debug & Options Buttons
            menu_button("BHOP Timing: 1/60s (Strict)", MenuButtonAction::CycleBhopWindow),
            menu_button("FPS Meter: ON", MenuButtonAction::ToggleFps),
            menu_button("Controls Guide: ON", MenuButtonAction::ToggleControlsHud),
            menu_button("Physics Stats: ON", MenuButtonAction::TogglePhysicsStats),
            menu_button("Crosshair & Prompts: ON", MenuButtonAction::ToggleCrosshair),
            menu_button("Physics Wireframes: ON", MenuButtonAction::ToggleWireframes),
            menu_button("Resume Game", MenuButtonAction::ResumeGame),
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
            MenuButtonAction::CycleBhopWindow => {
                if let Some(ref mut config) = bhop_config {
                    config.window = config.window.next();
                }
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
            MenuButtonAction::ToggleWireframes => {
                menu_state.show_wireframes = !menu_state.show_wireframes;
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
) {
    let usable_width = SLIDER_TRACK_WIDTH - SLIDER_THUMB_SIZE;
    let delta_norm = event.delta.x / usable_width;
    let old_norm = sensitivity_to_norm(menu_state.mouse_sensitivity);
    let new_norm = (old_norm + delta_norm).clamp(0.0, 1.0);
    menu_state.mouse_sensitivity = norm_to_sensitivity(new_norm);
}

pub fn on_slider_thumb_over(
    _: On<PointerOver>,
    mut query: Query<&mut BackgroundColor, With<SensitivitySliderThumb>>,
) {
    for mut bg in query.iter_mut() {
        *bg = BackgroundColor(Color::from(tailwind::CYAN_300));
    }
}

pub fn on_slider_thumb_out(
    _: On<PointerOut>,
    mut query: Query<&mut BackgroundColor, With<SensitivitySliderThumb>>,
) {
    for mut bg in query.iter_mut() {
        *bg = BackgroundColor(Color::from(tailwind::CYAN_400));
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
        let is_resume = *action == MenuButtonAction::ResumeGame;
        if pressed {
            *bg = BackgroundColor(Color::from(tailwind::BLUE_900));
            border.set_all(Color::from(tailwind::BLUE_400));
        } else if hovered.get() {
            *bg = if is_resume {
                BackgroundColor(Color::from(tailwind::BLUE_600))
            } else {
                BackgroundColor(Color::from(tailwind::SLATE_600))
            };
            border.set_all(Color::from(tailwind::SLATE_400));
        } else {
            *bg = if is_resume {
                BackgroundColor(Color::from(tailwind::BLUE_700))
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
                    MenuButtonAction::ToggleWireframes => {
                        text.0 = format!("Physics Wireframes: {}", if menu_state.show_wireframes { "ON" } else { "OFF" });
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn sync_hud_visibility(
    menu_state: Res<MenuState>,
    debug_render_context: Option<ResMut<DebugRenderContext>>,
    mut menu_query: Query<&mut Visibility, (With<MenuRoot>, Without<FpsHud>, Without<ControlsHud>, Without<PhysicsHud>, Without<CrosshairHud>)>,
    mut fps_query: Query<&mut Visibility, (With<FpsHud>, Without<MenuRoot>, Without<ControlsHud>, Without<PhysicsHud>, Without<CrosshairHud>)>,
    mut controls_query: Query<&mut Visibility, (With<ControlsHud>, Without<MenuRoot>, Without<FpsHud>, Without<PhysicsHud>, Without<CrosshairHud>)>,
    mut physics_query: Query<&mut Visibility, (With<PhysicsHud>, Without<MenuRoot>, Without<FpsHud>, Without<ControlsHud>, Without<CrosshairHud>)>,
    mut crosshair_query: Query<&mut Visibility, (With<CrosshairHud>, Without<MenuRoot>, Without<FpsHud>, Without<ControlsHud>, Without<PhysicsHud>)>,
) {
    if !menu_state.is_changed() {
        return;
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

pub struct ElfUiPlugin;

impl Plugin for ElfUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_observer(on_button_activate)
            .add_observer(on_slider_thumb_drag)
            .add_observer(on_slider_thumb_over)
            .add_observer(on_slider_thumb_out)
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (
                    toggle_menu,
                    sync_pause_state_system,
                    handle_mouse_grab,
                    update_button_visuals,
                    update_menu_button_labels,
                    sync_hud_visibility,
                    sync_sensitivity_slider,
                    update_fps,
                    update_hud_stats,
                ),
            );
    }
}
