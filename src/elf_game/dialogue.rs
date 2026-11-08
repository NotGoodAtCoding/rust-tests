use std::f32::consts::{FRAC_PI_2, PI};
use bevy::{
    color::palettes::tailwind,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use crate::elf_game::{
    physics_sandbox::{ElfAssistant, ElfJob, ElfMischiefTrait, ElfNpcState},
    player::controller::FirstPersonCamera,
};

/// High-level action triggered when a dialogue option is selected.
#[derive(Clone, Debug, PartialEq)]
pub enum DialogueAction {
    /// Opens a nested submenu with its own options and BACK item.
    Submenu(DialogueMenu),
    /// Assigns a specific job to the elf assistant.
    AssignJob(ElfJob),
    /// Removes any currently assigned job from the elf assistant.
    ClearJob,
    /// Elf speaks or provides feedback text inside the dialogue wheel.
    Speak(String),
    /// Closes the dialogue (or returns up one level if in a submenu).
    Close,
}

/// A single option entry inside a dialogue menu.
#[derive(Clone, Debug, PartialEq)]
pub struct DialogueOption {
    pub label: String,
    pub description: String,
    pub action: DialogueAction,
}

/// A structured menu containing options and contextual prompt text.
#[derive(Clone, Debug, PartialEq)]
pub struct DialogueMenu {
    pub title: String,
    pub prompt: String,
    pub options: Vec<DialogueOption>,
}

/// A visual and interactive sector on the circular dialogue wheel.
#[derive(Clone, Debug, PartialEq)]
pub struct WheelSection {
    pub index: usize,
    pub is_back: bool,
    pub label: String,
    pub description: String,
    pub action: DialogueAction,
    /// Center angle in radians (math coordinates: 0 is Right, +PI/2 is Top, -PI/2 is Bottom, PI is Left).
    pub angle_center: f32,
    /// Angular half-span of this slice in radians.
    pub angle_half_span: f32,
}

/// Computes the layout of the selection wheel sections based on the available menu options.
/// Rules:
/// - If 0 options: the bottom half is BACK.
/// - If 1 option: the top half is that option, and the bottom half is BACK.
/// - If 2 options (3 total): thirds of the selection wheel, with BACK centered at the bottom.
/// - If N options: BACK is centered at the bottom, and the other options split the remainder of the wheel evenly.
pub fn compute_wheel_sections(menu: &DialogueMenu) -> Vec<WheelSection> {
    let k = menu.options.len();
    if k == 0 {
        // Zero options: bottom half is BACK
        vec![WheelSection {
            index: 0,
            is_back: true,
            label: "↩ BACK".to_string(),
            description: "Exit dialogue".to_string(),
            action: DialogueAction::Close,
            angle_center: -FRAC_PI_2,
            angle_half_span: FRAC_PI_2,
        }]
    } else if k == 1 {
        // One option: top half is that option, bottom half is BACK
        vec![
            WheelSection {
                index: 0,
                is_back: false,
                label: menu.options[0].label.clone(),
                description: menu.options[0].description.clone(),
                action: menu.options[0].action.clone(),
                angle_center: FRAC_PI_2,
                angle_half_span: FRAC_PI_2,
            },
            WheelSection {
                index: 1,
                is_back: true,
                label: "↩ BACK".to_string(),
                description: "Return or Exit".to_string(),
                action: DialogueAction::Close,
                angle_center: -FRAC_PI_2,
                angle_half_span: FRAC_PI_2,
            },
        ]
    } else {
        // k >= 2: Total N = k + 1 sections.
        // BACK is always centered at the bottom (-FRAC_PI_2).
        // The other k options are evenly distributed around the rest of the wheel.
        let n = k + 1;
        let delta = 2.0 * PI / (n as f32);
        let angle_half_span = delta / 2.0;
        let mut sections = Vec::with_capacity(n);

        // Sections 0..k-1 for the options
        for i in 0..k {
            let angle = -FRAC_PI_2 + (i + 1) as f32 * delta;
            sections.push(WheelSection {
                index: i,
                is_back: false,
                label: menu.options[i].label.clone(),
                description: menu.options[i].description.clone(),
                action: menu.options[i].action.clone(),
                angle_center: angle,
                angle_half_span,
            });
        }

        // Section k for BACK (centered at bottom)
        sections.push(WheelSection {
            index: k,
            is_back: true,
            label: "↩ BACK".to_string(),
            description: "Return or Exit".to_string(),
            action: DialogueAction::Close,
            angle_center: -FRAC_PI_2,
            angle_half_span,
        });

        sections
    }
}

/// Returns the smallest angular difference between two angles in radians.
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % (2.0 * PI);
    if d > PI {
        2.0 * PI - d
    } else {
        d
    }
}

/// Finds the wheel section index closest to the given mouse aim angle `theta`.
pub fn find_section_for_angle(sections: &[WheelSection], theta: f32) -> Option<usize> {
    if sections.is_empty() {
        return None;
    }
    if sections.len() == 1 {
        return Some(0);
    }
    let mut best_idx = 0;
    let mut min_diff = f32::MAX;
    for (i, sec) in sections.iter().enumerate() {
        let diff = angle_diff(theta, sec.angle_center);
        if diff < min_diff {
            min_diff = diff;
            best_idx = i;
        }
    }
    Some(best_idx)
}

/// Finds the wheel section matching a keyboard directional vector (WASD).
/// Pressing S selects BACK (at the bottom).
/// Pressing W selects the top option.
/// Pressing A selects the left option.
/// Pressing D selects the right option.
pub fn find_section_for_wasd(sections: &[WheelSection], dir: Vec2) -> Option<usize> {
    if sections.is_empty() || dir.length_squared() < 0.1 {
        return None;
    }
    let dir = dir.normalize();
    let mut best_idx = 0;
    let mut max_dot = f32::NEG_INFINITY;
    for (i, sec) in sections.iter().enumerate() {
        let u = Vec2::new(sec.angle_center.cos(), sec.angle_center.sin());
        let dot = u.dot(dir);
        if dot > max_dot {
            max_dot = dot;
            best_idx = i;
        }
    }
    Some(best_idx)
}

/// Generates a rich, dynamic default dialogue menu tree for an elf assistant.
pub fn build_default_elf_dialogue(elf: &ElfAssistant) -> DialogueMenu {
    let job_label = match elf.assigned_job {
        Some(ElfJob::MooseFeeder) => "Moose Feeder",
        Some(ElfJob::PresentWrapper) => "Present Wrapper",
        Some(ElfJob::WorkshopMaintenance) => "Workshop Maintenance",
        None => "Unassigned",
    };

    let prompt = match elf.assigned_job {
        Some(ElfJob::MooseFeeder) => "On Moose duty! Need something, Boss?".to_string(),
        Some(ElfJob::PresentWrapper) => "Wrapping presents for the big night!".to_string(),
        Some(ElfJob::WorkshopMaintenance) => "Keeping the workshop benches tidy.".to_string(),
        None => match elf.mischief_trait {
            ElfMischiefTrait::Lazy => "*Yawn* Taking a quick break. What's up?".to_string(),
            ElfMischiefTrait::Klepto => "Just admiring the workshop presents!".to_string(),
            ElfMischiefTrait::Pyro => "Watching the warm holiday hearth glow.".to_string(),
        },
    };

    let assign_job_menu = DialogueMenu {
        title: format!("Jobs for {}", elf.name),
        prompt: "Assign a workshop responsibility:".to_string(),
        options: vec![
            DialogueOption {
                label: "Moose Feeder".to_string(),
                description: "Retrieves treats from the pantry and feeds Mr. Moose.".to_string(),
                action: DialogueAction::AssignJob(ElfJob::MooseFeeder),
            },
            DialogueOption {
                label: "Present Wrapper".to_string(),
                description: "Assists at the conveyor belt wrapping gifts.".to_string(),
                action: DialogueAction::AssignJob(ElfJob::PresentWrapper),
            },
            DialogueOption {
                label: "Workshop Maint.".to_string(),
                description: "Patrols workbench aisles and cleans clutter.".to_string(),
                action: DialogueAction::AssignJob(ElfJob::WorkshopMaintenance),
            },
            DialogueOption {
                label: "Relieve Duty".to_string(),
                description: "Clears assigned job and allows free wandering.".to_string(),
                action: DialogueAction::ClearJob,
            },
        ],
    };

    let tips_menu = DialogueMenu {
        title: "Workshop Advice".to_string(),
        prompt: "Ask for tips and tricks:".to_string(),
        options: vec![
            DialogueOption {
                label: "How to Bunnyhop?".to_string(),
                description: "Chain jumps with high air strain velocity.".to_string(),
                action: DialogueAction::Speak("Jump just as you hit the ground to preserve speed!".to_string()),
            },
            DialogueOption {
                label: "About Mr. Moose".to_string(),
                description: "Moose hunger mechanics and warnings.".to_string(),
                action: DialogueAction::Speak("Keep the feeding trough stocked with Moose Treats from the pantry!".to_string()),
            },
            DialogueOption {
                label: "High Five!".to_string(),
                description: "Holiday cheer high five with the elf.".to_string(),
                action: DialogueAction::Speak(format!("*High Five!* {} beams with holiday cheer!", elf.name)),
            },
        ],
    };

    // Top level menu has 2 options + BACK (Total = 3 sections: thirds of the wheel!)
    DialogueMenu {
        title: format!("{} ({})", elf.name, job_label),
        prompt,
        options: vec![
            DialogueOption {
                label: "Assign Job".to_string(),
                description: "Give this elf a task or duty.".to_string(),
                action: DialogueAction::Submenu(assign_job_menu),
            },
            DialogueOption {
                label: "Advice & Chat".to_string(),
                description: "Ask questions or share holiday cheer.".to_string(),
                action: DialogueAction::Submenu(tips_menu),
            },
        ],
    }
}

/// Global resource tracking the active elf dialogue session.
#[derive(Resource, Debug)]
pub struct ActiveDialogue {
    pub is_active: bool,
    pub elf_entity: Option<Entity>,
    pub elf_name: String,
    pub elf_trait: Option<ElfMischiefTrait>,
    pub elf_job: Option<ElfJob>,
    pub current_menu: Option<DialogueMenu>,
    pub menu_stack: Vec<DialogueMenu>,
    pub selected_index: usize,
    pub status_feedback: String,
    pub sections_cache: Vec<WheelSection>,
    pub dirty: bool,
}

impl Default for ActiveDialogue {
    fn default() -> Self {
        Self {
            is_active: false,
            elf_entity: None,
            elf_name: String::new(),
            elf_trait: None,
            elf_job: None,
            current_menu: None,
            menu_stack: Vec::new(),
            selected_index: 0,
            status_feedback: String::new(),
            sections_cache: Vec::new(),
            dirty: false,
        }
    }
}

impl ActiveDialogue {
    pub fn open_for_elf(&mut self, entity: Entity, elf: &ElfAssistant) {
        self.is_active = true;
        self.elf_entity = Some(entity);
        self.elf_name = elf.name.clone();
        self.elf_trait = Some(elf.mischief_trait);
        self.elf_job = elf.assigned_job;
        self.menu_stack.clear();
        let menu = build_default_elf_dialogue(elf);
        self.sections_cache = compute_wheel_sections(&menu);
        self.current_menu = Some(menu);
        self.selected_index = 0;
        self.status_feedback.clear();
        self.dirty = true;
    }

    pub fn open_custom_menu(&mut self, entity: Entity, name: &str, menu: DialogueMenu) {
        self.is_active = true;
        self.elf_entity = Some(entity);
        self.elf_name = name.to_string();
        self.elf_trait = None;
        self.elf_job = None;
        self.menu_stack.clear();
        self.sections_cache = compute_wheel_sections(&menu);
        self.current_menu = Some(menu);
        self.selected_index = 0;
        self.status_feedback.clear();
        self.dirty = true;
    }
}

#[derive(Component)]
pub struct DialogueWheelRoot;

#[derive(Component)]
pub struct DialogueWheelFrame;

#[derive(Component)]
pub struct DialogueWheelHub;

#[derive(Component)]
pub struct DialogueWheelTitleText;

#[derive(Component)]
pub struct DialogueWheelPromptText;

#[derive(Component)]
pub struct DialogueWheelStatusText;

#[derive(Component)]
pub struct DialoguePromptBox;

#[derive(Component)]
pub struct DialogueSliceButton {
    pub section_index: usize,
    pub is_back: bool,
}

#[derive(Component)]
pub struct DialogueReticlePrompt;

pub fn spawn_dialogue_ui(mut commands: Commands) {
    // 1. Center interaction prompt when aiming at an elf
    commands.spawn((
        DialogueReticlePrompt,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(280.0),
            left: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-150.0),
                ..default()
            },
            width: Val::Px(300.0),
            height: Val::Px(42.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BorderColor::all(Color::from(tailwind::AMBER_400)),
        BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.85)),
        Visibility::Hidden,
        children![(
            Text::new("[Q / E / Mouse1] Talk to Elf"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));

    // 2. Fullscreen Dialogue Wheel Root Overlay
    commands
        .spawn((
            DialogueWheelRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.40)),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            // Radial Wheel Frame (480x480) - Clear and transparent center to see the elf
            root.spawn((
                DialogueWheelFrame,
                Node {
                    width: Val::Px(480.0),
                    height: Val::Px(480.0),
                    position_type: PositionType::Relative,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(240.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.85, 0.70, 0.30, 0.35)),
                BackgroundColor(Color::NONE),
            ))
            .with_children(|frame| {
                // Center Hub (160x160 circular area - fully transparent and clear so player can see elf)
                frame.spawn((
                    DialogueWheelHub,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(160.0),
                        top: Val::Px(160.0),
                        width: Val::Px(160.0),
                        height: Val::Px(160.0),
                        border_radius: BorderRadius::all(Val::Px(80.0)),
                        border: UiRect::all(Val::Px(0.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::NONE),
                    BackgroundColor(Color::NONE),
                ));
            });

            // Rectangular prompt box at the bottom of the screen with larger prominent text
            root.spawn((
                DialoguePromptBox,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    left: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-340.0),
                        ..default()
                    },
                    width: Val::Px(680.0),
                    min_height: Val::Px(104.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(12.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.85, 0.70, 0.30, 0.75)),
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.92)),
            ))
            .with_children(|prompt_box| {
                prompt_box.spawn((
                    DialogueWheelTitleText,
                    Text::new("Elf Assistant"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::from(tailwind::AMBER_300)),
                ));

                prompt_box.spawn((
                    DialogueWheelPromptText,
                    Text::new("Choose an option:"),
                    TextFont {
                        font_size: FontSize::Px(22.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                prompt_box.spawn((
                    DialogueWheelStatusText,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::from(tailwind::EMERALD_300)),
                ));

                prompt_box.spawn((
                    Text::new("WASD / Mouse Aim • Q/E/M1 to Select • ESC/Backspace to Go Back"),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::from(tailwind::SLATE_400)),
                ));
            });
        });
}

/// Detects if the player is looking at an elf within interaction reach and updates the reticle prompt.
pub fn detect_elf_interaction_prompt_system(
    active_dialogue: Res<ActiveDialogue>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    q_elves: Query<(&Transform, &ElfAssistant)>,
    mut q_prompt: Query<(&mut Visibility, &Children), With<DialogueReticlePrompt>>,
    mut q_text: Query<&mut Text>,
) {
    let Ok((mut prompt_vis, children)) = q_prompt.single_mut() else {
        return;
    };

    if active_dialogue.is_active
        || crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref())
    {
        *prompt_vis = Visibility::Hidden;
        return;
    }

    let Ok(cam_gt) = q_camera.single() else {
        *prompt_vis = Visibility::Hidden;
        return;
    };

    let cam_pos = cam_gt.translation();
    let cam_fwd = cam_gt.forward();

    let mut best_elf: Option<(&ElfAssistant, f32)> = None;
    for (elf_tf, elf) in &q_elves {
        if elf.state == ElfNpcState::RagdollTumble {
            continue;
        }
        let elf_target = elf_tf.translation + Vec3::Y * 0.45;
        let dist = cam_pos.distance(elf_target);
        if dist > 3.2 {
            continue;
        }
        let to_elf = (elf_target - cam_pos).normalize();
        let dot = cam_fwd.dot(to_elf);
        if dot > 0.65 {
            if best_elf.is_none() || dot > best_elf.unwrap().1 {
                best_elf = Some((elf, dot));
            }
        }
    }

    if let Some((elf, _)) = best_elf {
        *prompt_vis = Visibility::Visible;
        for child in children.iter() {
            if let Ok(mut text) = q_text.get_mut(child) {
                text.0 = format!("[Q / E / Mouse1] Talk to {}", elf.name);
            }
        }
    } else {
        *prompt_vis = Visibility::Hidden;
    }
}

/// Handles player interaction to trigger dialogue, mouse angle aiming, WASD navigation, and Q/E selection.
pub fn elf_dialogue_interaction_system(
    keyboard_input: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    menu_state: Option<Res<crate::elf_game::ui::MenuState>>,
    game_state: Option<Res<State<crate::elf_game::core::GameState>>>,
    mut q_window: Query<&mut Window, With<PrimaryWindow>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    mut q_elves: Query<(Entity, &Transform, &mut ElfAssistant)>,
    mut cursor_query: Query<&mut CursorOptions, With<Window>>,
    mut active_dialogue: ResMut<ActiveDialogue>,
) {
    if crate::elf_game::core::is_game_paused(menu_state.as_deref(), game_state.as_deref()) {
        return;
    }

    let Some(keys) = keyboard_input else {
        return;
    };
    let Some(mouse) = mouse_buttons else {
        return;
    };

    // 1. If NOT in dialogue: Check for player interaction with Q, E, or Mouse1
    if !active_dialogue.is_active {
        let interact_pressed = keys.just_pressed(KeyCode::KeyE)
            || keys.just_pressed(KeyCode::KeyQ)
            || keys.just_pressed(KeyCode::KeyF)
            || mouse.just_pressed(MouseButton::Left);

        if !interact_pressed {
            return;
        }

        let Ok(cam_gt) = q_camera.single() else {
            return;
        };
        let cam_pos = cam_gt.translation();
        let cam_fwd = cam_gt.forward();

        let mut closest_match: Option<(Entity, f32)> = None;
        for (entity, elf_tf, elf) in &q_elves {
            if elf.state == ElfNpcState::RagdollTumble {
                continue;
            }
            let elf_target = elf_tf.translation + Vec3::Y * 0.45;
            let dist = cam_pos.distance(elf_target);
            if dist > 3.2 {
                continue;
            }
            let to_elf = (elf_target - cam_pos).normalize();
            let dot = cam_fwd.dot(to_elf);
            if dot > 0.65 {
                if closest_match.is_none() || dot > closest_match.unwrap().1 {
                    closest_match = Some((entity, dot));
                }
            }
        }

        if let Some((target_entity, _)) = closest_match {
            if let Ok((_, _, mut elf)) = q_elves.get_mut(target_entity) {
                elf.previous_state = elf.state;
                elf.state = ElfNpcState::InDialogue;
                active_dialogue.open_for_elf(target_entity, &elf);

                // Unlock cursor for dialogue interaction
                for mut cursor in &mut cursor_query {
                    cursor.visible = true;
                    cursor.grab_mode = CursorGrabMode::None;
                }
                if let Ok(mut window) = q_window.single_mut() {
                    let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
                    window.set_cursor_position(Some(center));
                }
            }
        }
        return;
    }

    // 2. In Dialogue:
    // A. Mouse cursor angle aiming
    if let Ok(window) = q_window.single() {
        if let Some(cursor_pos) = window.cursor_position() {
            let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
            let offset = cursor_pos - center;
            if offset.length() > 35.0 {
                let math_x = offset.x;
                let math_y = -offset.y; // In screen space Y is down; in math space Y is up
                let theta = math_y.atan2(math_x);
                if let Some(sec_idx) = find_section_for_angle(&active_dialogue.sections_cache, theta) {
                    active_dialogue.selected_index = sec_idx;
                }
            }
        }
    }

    // B. Keyboard WASD navigation
    let mut wasd_dir = Vec2::ZERO;
    if keys.just_pressed(KeyCode::KeyW) || keys.pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::ArrowUp) {
        wasd_dir.y += 1.0;
    }
    if keys.just_pressed(KeyCode::KeyS) || keys.pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::ArrowDown) {
        wasd_dir.y -= 1.0;
    }
    if keys.just_pressed(KeyCode::KeyA) || keys.pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::ArrowLeft) {
        wasd_dir.x -= 1.0;
    }
    if keys.just_pressed(KeyCode::KeyD) || keys.pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::ArrowRight) {
        wasd_dir.x += 1.0;
    }

    if wasd_dir.length_squared() > 0.1 {
        if let Some(sec_idx) = find_section_for_wasd(&active_dialogue.sections_cache, wasd_dir) {
            active_dialogue.selected_index = sec_idx;
            if let Ok(mut window) = q_window.single_mut() {
                let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
                window.set_cursor_position(Some(center));
            }
        }
    }

    // C. Selection input with Q, E, Enter, Space, or Left Mouse click
    let select_pressed = keys.just_pressed(KeyCode::KeyQ)
        || keys.just_pressed(KeyCode::KeyE)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Space)
        || mouse.just_pressed(MouseButton::Left);

    let back_shortcut = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Backspace);

    if back_shortcut {
        // Explicit BACK shortcut
        execute_back_action(&mut active_dialogue, &mut q_elves, &mut cursor_query);
    } else if select_pressed {
        if let Some(section) = active_dialogue.sections_cache.get(active_dialogue.selected_index).cloned() {
            match section.action {
                DialogueAction::Submenu(sub) => {
                    if let Some(current) = active_dialogue.current_menu.take() {
                        active_dialogue.menu_stack.push(current);
                    }
                    active_dialogue.sections_cache = compute_wheel_sections(&sub);
                    active_dialogue.current_menu = Some(sub);
                    active_dialogue.selected_index = 0;
                    active_dialogue.dirty = true;
                }
                DialogueAction::Close => {
                    execute_back_action(&mut active_dialogue, &mut q_elves, &mut cursor_query);
                }
                DialogueAction::AssignJob(job) => {
                    if let Some(elf_entity) = active_dialogue.elf_entity {
                        if let Ok((_, _, mut elf)) = q_elves.get_mut(elf_entity) {
                            elf.assigned_job = Some(job);
                            active_dialogue.elf_job = Some(job);
                            let job_name = match job {
                                ElfJob::MooseFeeder => "Moose Feeder",
                                ElfJob::PresentWrapper => "Present Wrapper",
                                ElfJob::WorkshopMaintenance => "Workshop Maintenance",
                            };
                            active_dialogue.status_feedback = format!("{} assigned to {}!", elf.name, job_name);
                            active_dialogue.dirty = true;
                        }
                    }
                }
                DialogueAction::ClearJob => {
                    if let Some(elf_entity) = active_dialogue.elf_entity {
                        if let Ok((_, _, mut elf)) = q_elves.get_mut(elf_entity) {
                            elf.assigned_job = None;
                            active_dialogue.elf_job = None;
                            active_dialogue.status_feedback = format!("{} relieved from duties.", elf.name);
                            active_dialogue.dirty = true;
                        }
                    }
                }
                DialogueAction::Speak(reply) => {
                    active_dialogue.status_feedback = reply;
                    active_dialogue.dirty = true;
                }
            }
        }
    }
}

fn execute_back_action(
    active_dialogue: &mut ActiveDialogue,
    q_elves: &mut Query<(Entity, &Transform, &mut ElfAssistant)>,
    cursor_query: &mut Query<&mut CursorOptions, With<Window>>,
) {
    if let Some(parent_menu) = active_dialogue.menu_stack.pop() {
        // Return up one level in the submenu stack
        active_dialogue.sections_cache = compute_wheel_sections(&parent_menu);
        active_dialogue.current_menu = Some(parent_menu);
        active_dialogue.selected_index = 0;
        active_dialogue.dirty = true;
    } else {
        // Pressing BACK at top level exits dialogue and resumes elf movement!
        active_dialogue.is_active = false;
        active_dialogue.current_menu = None;
        active_dialogue.sections_cache.clear();
        active_dialogue.dirty = true;

        if let Some(elf_e) = active_dialogue.elf_entity.take() {
            if let Ok((_, _, mut elf)) = q_elves.get_mut(elf_e) {
                if elf.state == ElfNpcState::InDialogue {
                    elf.state = if elf.assigned_job == Some(ElfJob::MooseFeeder) {
                        ElfNpcState::FetchingMooseTreat
                    } else {
                        match elf.previous_state {
                            ElfNpcState::InDialogue | ElfNpcState::RagdollTumble => ElfNpcState::Wandering,
                            other => other,
                        }
                    };
                }
            }
        }

        for mut cursor in cursor_query {
            cursor.visible = false;
            cursor.grab_mode = CursorGrabMode::Locked;
        }
    }
}

/// Synchronizes the dynamic dialogue wheel UI nodes, slice layout, and selection highlight.
pub fn sync_dialogue_ui_system(
    mut commands: Commands,
    mut active_dialogue: ResMut<ActiveDialogue>,
    mut q_root: Query<&mut Visibility, With<DialogueWheelRoot>>,
    q_frame: Query<Entity, With<DialogueWheelFrame>>,
    mut q_title: Query<&mut Text, (With<DialogueWheelTitleText>, Without<DialogueWheelPromptText>, Without<DialogueWheelStatusText>)>,
    mut q_prompt: Query<&mut Text, (With<DialogueWheelPromptText>, Without<DialogueWheelTitleText>, Without<DialogueWheelStatusText>)>,
    mut q_status: Query<&mut Text, (With<DialogueWheelStatusText>, Without<DialogueWheelTitleText>, Without<DialogueWheelPromptText>)>,
    q_existing_buttons: Query<Entity, With<DialogueSliceButton>>,
    mut q_slice_styles: Query<(
        &DialogueSliceButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let Ok(mut root_vis) = q_root.single_mut() else {
        return;
    };

    if !active_dialogue.is_active {
        *root_vis = Visibility::Hidden;
        return;
    }

    *root_vis = Visibility::Visible;

    // When dialogue menu updates (dirty), regenerate UI components
    if active_dialogue.dirty {
        active_dialogue.dirty = false;

        let title_str = active_dialogue
            .current_menu
            .as_ref()
            .map(|m| m.title.clone())
            .unwrap_or_else(|| active_dialogue.elf_name.clone());

        let prompt_str = active_dialogue
            .current_menu
            .as_ref()
            .map(|m| m.prompt.clone())
            .unwrap_or_default();

        let status_str = active_dialogue.status_feedback.clone();

        if let Ok(mut text) = q_title.single_mut() {
            text.0 = title_str;
        }
        if let Ok(mut text) = q_prompt.single_mut() {
            text.0 = prompt_str;
        }
        if let Ok(mut text) = q_status.single_mut() {
            text.0 = status_str;
        }

        // Despawn old slice buttons
        for btn_entity in &q_existing_buttons {
            commands.entity(btn_entity).despawn();
        }

        // Spawn new slice buttons into the wheel frame
        if let Ok(frame_entity) = q_frame.single() {
            let sections = active_dialogue.sections_cache.clone();
            let k = sections.len().saturating_sub(1);

            commands.entity(frame_entity).with_children(|frame| {
                if sections.len() == 1 {
                    // Zero options: bottom half is BACK
                    let sec = &sections[0];
                    frame.spawn((
                        DialogueSliceButton {
                            section_index: 0,
                            is_back: true,
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(80.0),
                            bottom: Val::Px(24.0),
                            width: Val::Px(320.0),
                            height: Val::Px(90.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(16.0)),
                            ..default()
                        },
                        BorderColor::all(Color::from(tailwind::ROSE_500)),
                        BackgroundColor(Color::srgba(0.85, 0.20, 0.25, 0.95)),
                        children![(
                            Text::new(&sec.label),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        )],
                    ));
                } else if sections.len() == 2 && k == 1 {
                    // One option: top half is option, bottom half is BACK
                    // Option 0 on top
                    let top_sec = &sections[0];
                    frame.spawn((
                        DialogueSliceButton {
                            section_index: 0,
                            is_back: false,
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(80.0),
                            top: Val::Px(24.0),
                            width: Val::Px(320.0),
                            height: Val::Px(85.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(16.0)),
                            ..default()
                        },
                        BorderColor::all(Color::from(tailwind::AMBER_400)),
                        BackgroundColor(Color::srgba(0.18, 0.65, 0.35, 0.95)),
                        children![(
                            Text::new(&top_sec.label),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        )],
                    ));

                    // BACK on bottom
                    let back_sec = &sections[1];
                    frame.spawn((
                        DialogueSliceButton {
                            section_index: 1,
                            is_back: true,
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(80.0),
                            bottom: Val::Px(24.0),
                            width: Val::Px(320.0),
                            height: Val::Px(85.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(16.0)),
                            ..default()
                        },
                        BorderColor::all(Color::from(tailwind::ROSE_500)),
                        BackgroundColor(Color::srgba(0.85, 0.20, 0.25, 0.95)),
                        children![(
                            Text::new(&back_sec.label),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        )],
                    ));
                } else {
                    // k >= 2: Thirds or N-sectors with BACK centered at bottom
                    let cx = 240.0;
                    let cy = 240.0;
                    let radius = 150.0;

                    for sec in &sections {
                        let btn_width = if sec.is_back { 210.0 } else { 165.0 };
                        let btn_height = if sec.is_back { 68.0 } else { 62.0 };

                        let x = cx + radius * sec.angle_center.cos();
                        let y = cy - radius * sec.angle_center.sin();

                        frame.spawn((
                            DialogueSliceButton {
                                section_index: sec.index,
                                is_back: sec.is_back,
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(x - btn_width / 2.0),
                                top: Val::Px(y - btn_height / 2.0),
                                width: Val::Px(btn_width),
                                height: Val::Px(btn_height),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                border_radius: BorderRadius::all(Val::Px(12.0)),
                                ..default()
                            },
                            BorderColor::all(if sec.is_back {
                                Color::from(tailwind::ROSE_500)
                            } else {
                                Color::from(tailwind::SLATE_600)
                            }),
                            BackgroundColor(if sec.is_back {
                                Color::srgba(0.4, 0.1, 0.15, 0.85)
                            } else {
                                Color::srgba(0.08, 0.12, 0.18, 0.85)
                            }),
                            children![(
                                Text::new(&sec.label),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            )],
                        ));
                    }
                }
            });
        }
    }

    // Dynamic selection highlighting
    let current_selected = active_dialogue.selected_index;
    for (btn, mut bg_color, mut border_color) in &mut q_slice_styles {
        let is_selected = btn.section_index == current_selected;
        if is_selected {
            if btn.is_back {
                *border_color = BorderColor::all(Color::from(tailwind::ROSE_300));
                *bg_color = BackgroundColor(Color::srgba(0.88, 0.22, 0.28, 0.98));
            } else {
                *border_color = BorderColor::all(Color::from(tailwind::AMBER_300));
                *bg_color = BackgroundColor(Color::srgba(0.18, 0.65, 0.35, 0.98));
            }
        } else {
            if btn.is_back {
                *border_color = BorderColor::all(Color::from(tailwind::ROSE_900));
                *bg_color = BackgroundColor(Color::srgba(0.40, 0.10, 0.15, 0.85));
            } else {
                *border_color = BorderColor::all(Color::from(tailwind::SLATE_600));
                *bg_color = BackgroundColor(Color::srgba(0.08, 0.12, 0.18, 0.85));
            }
        }
    }
}

pub struct ElfDialoguePlugin;

impl Plugin for ElfDialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveDialogue>()
            .add_systems(Startup, spawn_dialogue_ui)
            .add_systems(
                Update,
                (
                    detect_elf_interaction_prompt_system,
                    elf_dialogue_interaction_system,
                    sync_dialogue_ui_system,
                ),
            );
    }
}
