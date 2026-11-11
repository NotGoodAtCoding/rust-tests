use super::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy_rapier3d::prelude::*;
use crate::elf_game::{
    physics_sandbox::elf_assistant_ai_system,
    player::{
        controller::{elf_look_system, elf_player_controller_system, CameraPivot, ElfInputSettings, ElfPlayer},
        BhopConfig, ElfMovement,
    },
};

#[test]
fn test_dialogue_wheel_layout_zero_options() {
    let empty_menu = DialogueMenu {
        title: "Empty Menu".to_string(),
        prompt: "No options".to_string(),
        options: vec![],
    };

    let sections = compute_wheel_sections(&empty_menu);
    assert_eq!(sections.len(), 1, "Zero options must produce exactly 1 section (bottom half BACK)");
    let back = &sections[0];
    assert!(back.is_back);
    assert_eq!(back.action, DialogueAction::Close);
    assert!((back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");
    assert!((back.angle_half_span - FRAC_PI_2).abs() < 1e-4, "BACK must span the bottom half (half span PI/2)");
}

#[test]
fn test_dialogue_wheel_layout_one_option() {
    let single_opt_menu = DialogueMenu {
        title: "Single Option Menu".to_string(),
        prompt: "One option available".to_string(),
        options: vec![DialogueOption {
            label: "Only Option".to_string(),
            description: "A single choice".to_string(),
            action: DialogueAction::Speak("Selected".to_string()),
        }],
    };

    let sections = compute_wheel_sections(&single_opt_menu);
    assert_eq!(sections.len(), 2, "One option must produce 2 sections (top half option, bottom half BACK)");

    let top_opt = &sections[0];
    assert!(!top_opt.is_back);
    assert_eq!(top_opt.label, "Only Option");
    assert!((top_opt.angle_center - FRAC_PI_2).abs() < 1e-4, "Top option must be centered at top (+PI/2)");
    assert!((top_opt.angle_half_span - FRAC_PI_2).abs() < 1e-4, "Top option must span top half (half span PI/2)");

    let bottom_back = &sections[1];
    assert!(bottom_back.is_back);
    assert_eq!(bottom_back.action, DialogueAction::Close);
    assert!((bottom_back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");
    assert!((bottom_back.angle_half_span - FRAC_PI_2).abs() < 1e-4, "BACK must span bottom half (half span PI/2)");
}

#[test]
fn test_dialogue_wheel_layout_three_sections_thirds() {
    let two_opt_menu = DialogueMenu {
        title: "Two Options Menu".to_string(),
        prompt: "Choose between two".to_string(),
        options: vec![
            DialogueOption {
                label: "Option Alpha".to_string(),
                description: "First".to_string(),
                action: DialogueAction::Speak("A".to_string()),
            },
            DialogueOption {
                label: "Option Beta".to_string(),
                description: "Second".to_string(),
                action: DialogueAction::Speak("B".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&two_opt_menu);
    assert_eq!(sections.len(), 3, "2 options + BACK must produce 3 sections (thirds of the selection wheel)");

    let expected_half_span = PI / 3.0;
    for sec in &sections {
        assert!((sec.angle_half_span - expected_half_span).abs() < 1e-4, "Each third must have half-span PI/3");
    }

    let back = &sections[2];
    assert!(back.is_back);
    assert_eq!(back.action, DialogueAction::Close);
    assert!((back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");

    let opt_a = &sections[0];
    assert!(!opt_a.is_back);
    assert_eq!(opt_a.label, "Option Alpha");
    assert!((opt_a.angle_center - (PI / 6.0)).abs() < 1e-4, "Option Alpha must be at PI/6 (30 deg)");

    let opt_b = &sections[1];
    assert!(!opt_b.is_back);
    assert_eq!(opt_b.label, "Option Beta");
    assert!((opt_b.angle_center - (5.0 * PI / 6.0)).abs() < 1e-4, "Option Beta must be at 5*PI/6 (150 deg)");
}

#[test]
fn test_dialogue_wheel_layout_four_sections_quarters() {
    let three_opt_menu = DialogueMenu {
        title: "Three Options Menu".to_string(),
        prompt: "Choose between three".to_string(),
        options: vec![
            DialogueOption {
                label: "Right".to_string(),
                description: "East".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Top".to_string(),
                description: "North".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
            DialogueOption {
                label: "Left".to_string(),
                description: "West".to_string(),
                action: DialogueAction::Speak("3".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&three_opt_menu);
    assert_eq!(sections.len(), 4, "3 options + BACK must produce 4 sections (quarters of the wheel)");

    let expected_half_span = PI / 4.0;
    for sec in &sections {
        assert!((sec.angle_half_span - expected_half_span).abs() < 1e-4, "Each quarter must have half-span PI/4");
    }

    let back = &sections[3];
    assert!(back.is_back);
    assert!((back.angle_center - (-FRAC_PI_2)).abs() < 1e-4, "BACK must be centered at bottom (-PI/2)");

    assert!((sections[0].angle_center - 0.0).abs() < 1e-4, "Section 0 must be centered at 0 rad (East)");
    assert!((sections[1].angle_center - FRAC_PI_2).abs() < 1e-4, "Section 1 must be centered at PI/2 rad (North)");
    assert!((sections[2].angle_center - PI).abs() < 1e-4, "Section 2 must be centered at PI rad (West)");
}

#[test]
fn test_dialogue_mouse_angle_selection() {
    let menu = DialogueMenu {
        title: "Test".to_string(),
        prompt: "Test".to_string(),
        options: vec![
            DialogueOption {
                label: "Option 1".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Option 2".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&menu);
    assert_eq!(sections.len(), 3);

    let down_sec = find_section_for_angle(&sections, -FRAC_PI_2);
    assert_eq!(down_sec, Some(2), "Aiming straight down must select BACK");

    let tr_sec = find_section_for_angle(&sections, PI / 6.0);
    assert_eq!(tr_sec, Some(0), "Aiming top-right must select Option 1");

    let tl_sec = find_section_for_angle(&sections, 5.0 * PI / 6.0);
    assert_eq!(tl_sec, Some(1), "Aiming top-left must select Option 2");
}

#[test]
fn test_dialogue_keyboard_wasd_navigation() {
    let menu = DialogueMenu {
        title: "Test".to_string(),
        prompt: "Test".to_string(),
        options: vec![
            DialogueOption {
                label: "Right".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Top".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
            DialogueOption {
                label: "Left".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("3".to_string()),
            },
        ],
    };

    let sections = compute_wheel_sections(&menu);
    assert_eq!(sections.len(), 4);

    let s_sec = find_section_for_wasd(&sections, Vec2::new(0.0, -1.0));
    assert_eq!(s_sec, Some(3), "S key must always select BACK at bottom");

    let w_sec = find_section_for_wasd(&sections, Vec2::new(0.0, 1.0));
    assert_eq!(w_sec, Some(1), "W key must select Top option");

    let a_sec = find_section_for_wasd(&sections, Vec2::new(-1.0, 0.0));
    assert_eq!(a_sec, Some(2), "A key must select Left option");

    let d_sec = find_section_for_wasd(&sections, Vec2::new(1.0, 0.0));
    assert_eq!(d_sec, Some(0), "D key must select Right option");
}

#[test]
fn test_dialogue_submenu_navigation_and_back_resume_elf() {
    let mut active = ActiveDialogue::default();
    let mut elf = ElfAssistant::new(1, "Sparky", vec![Vec3::ZERO]);
    elf.state = ElfNpcState::Wandering;

    let sub_menu = DialogueMenu {
        title: "Submenu".to_string(),
        prompt: "Sub prompt".to_string(),
        options: vec![DialogueOption {
            label: "Sub Option".to_string(),
            description: "".to_string(),
            action: DialogueAction::AssignJob(ElfJob::MooseFeeder),
        }],
    };

    let top_menu = DialogueMenu {
        title: "Top Menu".to_string(),
        prompt: "Top prompt".to_string(),
        options: vec![DialogueOption {
            label: "Open Sub".to_string(),
            description: "".to_string(),
            action: DialogueAction::Submenu(sub_menu.clone()),
        }],
    };

    let dummy_elf_entity = Entity::from_raw_u32(100).unwrap();
    active.open_custom_menu(dummy_elf_entity, "Sparky", top_menu.clone());
    elf.previous_state = elf.state;
    elf.state = ElfNpcState::InDialogue;

    assert!(active.is_active);
    assert_eq!(elf.state, ElfNpcState::InDialogue);
    assert_eq!(active.menu_stack.len(), 0);

    if let Some(current) = active.current_menu.take() {
        active.menu_stack.push(current);
    }
    active.sections_cache = compute_wheel_sections(&sub_menu);
    active.current_menu = Some(sub_menu);
    assert_eq!(active.menu_stack.len(), 1, "Submenu must push parent menu to stack");
    assert_eq!(active.current_menu.as_ref().unwrap().title, "Submenu");

    let parent = active.menu_stack.pop().expect("Must have parent in stack");
    active.sections_cache = compute_wheel_sections(&parent);
    active.current_menu = Some(parent);
    assert_eq!(active.menu_stack.len(), 0);
    assert_eq!(active.current_menu.as_ref().unwrap().title, "Top Menu");
    assert!(active.is_active, "Selecting BACK in submenu must keep dialogue active");

    let parent_opt = active.menu_stack.pop();
    assert!(parent_opt.is_none());
    active.is_active = false;
    active.current_menu = None;

    if elf.state == ElfNpcState::InDialogue {
        elf.state = elf.previous_state;
    }
    assert_eq!(elf.state, ElfNpcState::Wandering, "Pressing BACK at top level must resume elf wandering");
}

#[test]
fn test_dialogue_camera_and_player_movement_locked() {
    let mut app = App::new();
    app.init_resource::<ActiveDialogue>();
    app.init_resource::<AccumulatedMouseMotion>();
    app.init_resource::<ElfInputSettings>();
    app.init_resource::<BhopConfig>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_systems(Update, (elf_look_system, elf_player_controller_system));

    let player_e = app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(0.0, 1.0, 0.0),
        ElfMovement::default(),
        KinematicCharacterController::default(),
    )).id();

    let pivot_e = app.world_mut().spawn((
        CameraPivot { pitch: 0.0 },
        Transform::IDENTITY,
    )).id();

    {
        let mut mouse_motion = app.world_mut().resource_mut::<AccumulatedMouseMotion>();
        mouse_motion.delta = Vec2::new(100.0, 50.0);
    }
    app.update();

    let player_rot_unlocked = app.world().get::<Transform>(player_e).unwrap().rotation;
    let pivot_unlocked = app.world().get::<CameraPivot>(pivot_e).unwrap().pitch;
    assert_ne!(player_rot_unlocked, Quat::IDENTITY, "Unlocked camera must rotate player yaw");
    assert_ne!(pivot_unlocked, 0.0, "Unlocked camera must rotate pitch");

    app.world_mut().resource_mut::<ActiveDialogue>().is_active = true;
    let saved_yaw = app.world().get::<Transform>(player_e).unwrap().rotation;
    let saved_pitch = app.world().get::<CameraPivot>(pivot_e).unwrap().pitch;

    {
        let mut mouse_motion = app.world_mut().resource_mut::<AccumulatedMouseMotion>();
        mouse_motion.delta = Vec2::new(500.0, -300.0);
    }
    app.update();

    let locked_yaw = app.world().get::<Transform>(player_e).unwrap().rotation;
    let locked_pitch = app.world().get::<CameraPivot>(pivot_e).unwrap().pitch;
    assert_eq!(locked_yaw, saved_yaw, "Camera yaw must be completely locked while in dialogue");
    assert_eq!(locked_pitch, saved_pitch, "Camera pitch must be completely locked while in dialogue");
}

#[test]
fn test_elf_in_dialogue_stops_and_faces_player_smoothly() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.init_resource::<crate::elf_game::physics_sandbox::ElfNavGraph>();
    app.add_systems(Update, elf_assistant_ai_system);

    app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(0.0, 0.45, -5.0),
        ElfMovement::default(),
    ));

    let elf_e = app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.45, 0.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        Velocity {
            linear: Vec3::new(2.0, 0.0, 1.0),
            angular: Vec3::ZERO,
        },
        ExternalImpulse::default(),
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ElfAssistant {
            state: ElfNpcState::InDialogue,
            ..ElfAssistant::new(1, "TestElf", vec![Vec3::ZERO])
        },
    )).id();

    for _ in 0..10 {
        app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(100));
        app.update();
    }

    let vel = app.world().get::<Velocity>(elf_e).unwrap();
    let tf = app.world().get::<Transform>(elf_e).unwrap();

    assert_eq!(vel.linear.x, 0.0, "Elf in dialogue must have zero linear X velocity");
    assert_eq!(vel.linear.z, 0.0, "Elf in dialogue must have zero linear Z velocity");

    let yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
    assert!(
        yaw.abs() > 2.0,
        "Elf must smoothly rotate towards player (expected yaw ~3.14 / -3.14, got {})",
        yaw
    );
}

#[test]
fn test_dialogue_ui_bottom_prompt_box_and_keyboard_cursor_reset() {
    let mut app = App::new();
    app.add_systems(Startup, spawn_dialogue_ui);
    app.update();

    let mut prompt_box_q = app.world_mut().query::<(&DialoguePromptBox, &Node, &BackgroundColor)>();
    let (_, box_node, box_bg) = prompt_box_q.iter(app.world()).next().expect("DialoguePromptBox must exist");
    assert_eq!(box_node.position_type, PositionType::Absolute);
    assert_eq!(box_node.bottom, Val::Px(24.0), "Prompt box must be anchored at the bottom of the screen");
    assert!(box_bg.0.alpha() > 0.8, "Prompt box should have dark solid/semi-solid background");

    let mut prompt_text_q = app.world_mut().query::<(&DialogueWheelPromptText, &TextFont)>();
    let (_, text_font) = prompt_text_q.iter(app.world()).next().expect("DialogueWheelPromptText must exist");
    match text_font.font_size {
        FontSize::Px(px) => assert!(px >= 20.0, "Elf prompt text must be larger (>= 20px, got {})", px),
        _ => panic!("Expected FontSize::Px"),
    }

    let mut frame_q = app.world_mut().query::<(&DialogueWheelFrame, &BackgroundColor)>();
    let (_, frame_bg) = frame_q.iter(app.world()).next().expect("DialogueWheelFrame must exist");
    assert_eq!(frame_bg.0, Color::NONE, "DialogueWheelFrame background must be clear/transparent (Color::NONE)");

    let mut hub_q = app.world_mut().query::<(&DialogueWheelHub, &BackgroundColor, Option<&Children>)>();
    let (_, hub_bg, hub_children) = hub_q.iter(app.world()).next().expect("DialogueWheelHub must exist");
    assert_eq!(hub_bg.0, Color::NONE, "DialogueWheelHub center must be clear/transparent (Color::NONE)");
    assert!(hub_children.is_none(), "Center hub must have no children blocking view of the elf");

    let mut sim_app = App::new();
    sim_app.init_resource::<ButtonInput<KeyCode>>();
    sim_app.init_resource::<ButtonInput<MouseButton>>();
    let mut active = ActiveDialogue::default();
    active.is_active = true;

    let menu = DialogueMenu {
        title: "Test".to_string(),
        prompt: "Choose".to_string(),
        options: vec![
            DialogueOption {
                label: "Right".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("1".to_string()),
            },
            DialogueOption {
                label: "Top".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("2".to_string()),
            },
            DialogueOption {
                label: "Left".to_string(),
                description: "".to_string(),
                action: DialogueAction::Speak("3".to_string()),
            },
        ],
    };
    active.sections_cache = compute_wheel_sections(&menu);
    active.current_menu = Some(menu);
    active.selected_index = 0;
    sim_app.insert_resource(active);

    let mut test_window = Window::default();
    test_window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
    let win_w = test_window.width();
    let win_h = test_window.height();
    let center = Vec2::new(win_w / 2.0, win_h / 2.0);

    let win_entity = sim_app.world_mut().spawn((
        test_window,
        PrimaryWindow,
        CursorOptions::default(),
    )).id();

    sim_app.add_systems(Update, elf_dialogue_interaction_system);

    sim_app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
    sim_app.update();

    let updated_active = sim_app.world().resource::<ActiveDialogue>();
    assert_eq!(updated_active.selected_index, 1, "W key must select Top option (index 1)");

    let updated_win = sim_app.world().get::<Window>(win_entity).unwrap();
    let cur_pos = updated_win.cursor_position().expect("Cursor position must be set");
    assert!((cur_pos.x - center.x).abs() < 1e-4, "Cursor X must be set to center");
    assert!((cur_pos.y - center.y).abs() < 1e-4, "Cursor Y must be set to center");

    sim_app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyW);
    sim_app.update();

    let after_release_active = sim_app.world().resource::<ActiveDialogue>();
    assert_eq!(
        after_release_active.selected_index, 1,
        "Subsequent frame with mouse at center must retain keyboard selection (index 1)"
    );
}

#[test]
fn test_philosopher_dialogue_tree_resolution_gets_elf_back_to_work() {
    let mut app = App::new();

    let mut phil_comp = ElfAssistant::new(1, "PlatoElf", vec![]);
    phil_comp.mischief_trait = ElfMischiefTrait::Philosopher;
    phil_comp.state = ElfNpcState::Philosophizing;
    phil_comp.is_recruiter = true;

    let phil_e = app.world_mut().spawn((
        Transform::from_xyz(-3.0, 0.45, 10.5),
        phil_comp,
    )).id();

    let mut helper_comp = ElfAssistant::new(2, "StudentElf", vec![]);
    helper_comp.state = ElfNpcState::Philosophizing;
    helper_comp.is_recruited = true;

    let helper_e = app.world_mut().spawn((
        Transform::from_xyz(-2.5, 0.45, 10.5),
        helper_comp,
    )).id();

    let mut active_dialogue = ActiveDialogue::default();

    {
        let elf = app.world().get::<ElfAssistant>(phil_e).unwrap();
        active_dialogue.open_for_elf(phil_e, elf);
    }

    assert!(active_dialogue.is_active);
    let root_menu = active_dialogue.current_menu.as_ref().unwrap();
    assert!(root_menu.prompt.contains("what IS joy?"));
    assert_eq!(root_menu.options.len(), 3);
    assert_eq!(root_menu.options[0].label, "Meaning of Toys");
    assert_eq!(root_menu.options[1].label, "Cogito Ergo Present");
    assert_eq!(root_menu.options[2].label, "Soul of Moose");

    let action1 = root_menu.options[0].action.clone();
    let DialogueAction::Submenu(sub1) = action1 else { panic!("Expected Submenu"); };
    assert_eq!(sub1.options.len(), 2);
    assert_eq!(sub1.options[0].label, "Creation is Joy");

    let action2 = sub1.options[0].action.clone();
    let DialogueAction::Submenu(sub2) = action2 else { panic!("Expected Submenu level 3"); };
    assert_eq!(sub2.options[0].label, "Shared Purpose");

    let action3 = sub2.options[0].action.clone();
    let DialogueAction::ConcludePhilosophy(msg) = action3 else { panic!("Expected ConcludePhilosophy"); };
    assert!(msg.contains("synthesized the thesis and antithesis"));

    {
        let mut q_elves = app.world_mut().query::<(Entity, &Transform, &mut ElfAssistant)>();
        if let Ok((_, _, mut elf)) = q_elves.get_mut(app.world_mut(), phil_e) {
            elf.mischief_trait = ElfMischiefTrait::Lazy;
            elf.state = ElfNpcState::Wandering;
            elf.is_recruiter = false;
        }
        for (_, _, mut other) in q_elves.iter_mut(app.world_mut()) {
            if other.is_recruited {
                other.is_recruited = false;
                other.state = ElfNpcState::Wandering;
                other.dynamic_target = None;
            }
        }
    }

    let phil_after = app.world().get::<ElfAssistant>(phil_e).unwrap();
    assert_eq!(phil_after.state, ElfNpcState::Wandering, "Philosopher elf must return to work");
    assert_ne!(phil_after.mischief_trait, ElfMischiefTrait::Philosopher, "Philosopher trait must be resolved");

    let helper_after = app.world().get::<ElfAssistant>(helper_e).unwrap();
    assert_eq!(helper_after.state, ElfNpcState::Wandering, "Recruited elf must return to work");
    assert!(!helper_after.is_recruited, "Recruited status must be cleared");
}
