use super::*;
use bevy::{
    camera::NormalizedRenderTarget,
    pbr::wireframe::WireframeConfig,
    picking::backend::HitData,
    picking::pointer::{Location, PointerId},
    window::WindowRef,
};
use crate::elf_game::{
    core::GameState,
    player::{
        controller::{
            elf_player_controller_system, sync_noclip_collider_system, toggle_noclip_system,
            CameraPivot, ElfPlayer,
        },
        BhopWindow, ElfMovement,
    },
};

#[test]
fn test_pause_syncs_virtual_time_rapier_physics_and_game_state() {
    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_resource(Time::<Virtual>::default());

    let rapier_entity = app.world_mut().spawn(RapierConfiguration::new(1.0)).id();

    app.add_systems(Update, sync_pause_state_system);
    app.update();

    assert_eq!(*app.world().resource::<State<GameState>>().get(), GameState::InWorkshop);
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert!(app.world().get::<RapierConfiguration>(rapier_entity).unwrap().physics_pipeline_active);

    app.world_mut().resource_mut::<MenuState>().is_open = true;
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Paused,
        "GameState must transition to Paused"
    );
    assert!(
        app.world().resource::<Time<Virtual>>().is_paused(),
        "Virtual time must be paused"
    );
    assert!(
        !app.world().get::<RapierConfiguration>(rapier_entity).unwrap().physics_pipeline_active,
        "Rapier physics pipeline must be deactivated when paused"
    );

    app.world_mut().resource_mut::<MenuState>().is_open = false;
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::InWorkshop,
        "GameState must return to InWorkshop"
    );
    assert!(
        !app.world().resource::<Time<Virtual>>().is_paused(),
        "Virtual time must be unpaused"
    );
    assert!(
        app.world().get::<RapierConfiguration>(rapier_entity).unwrap().physics_pipeline_active,
        "Rapier physics pipeline must be reactivated when unpaused"
    );
}

#[test]
fn test_toggle_menu_keybindings_and_resume_game() {
    let mut app = App::new();
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyP);
    app.insert_resource(keys);
    app.insert_resource(MenuState::default());

    app.add_systems(Update, toggle_menu);
    app.add_observer(on_button_activate);

    app.update();
    assert!(
        app.world().resource::<MenuState>().is_open,
        "Pressing KeyCode::KeyP must open pause menu"
    );

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.press(KeyCode::Escape);
    }
    app.update();
    assert!(
        !app.world().resource::<MenuState>().is_open,
        "Pressing KeyCode::Escape must close pause menu"
    );

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.press(KeyCode::Tab);
    }
    app.update();
    assert!(app.world().resource::<MenuState>().is_open);

    let resume_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::ResumeGame,
    )).id();
    app.world_mut().trigger(Activate { entity: resume_btn });

    assert!(
        !app.world().resource::<MenuState>().is_open,
        "Activating ResumeGame button action must close menu and unpause"
    );
}

#[test]
fn test_menu_debug_submenu_navigation_and_panels() {
    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.add_observer(on_button_activate);
    app.add_systems(Update, sync_submenu_visibility);

    let title_e = app.world_mut().spawn((
        MenuTitleText,
        Text::new("PAUSED - OPTIONS & MENU"),
    )).id();
    let _subtitle_e = app.world_mut().spawn((
        MenuSubtitleText,
        Text::new("Options Subtitle"),
    )).id();
    let main_panel_e = app.world_mut().spawn((
        MainMenuPanel,
        Node { display: Display::Flex, ..default() },
    )).id();
    let debug_panel_e = app.world_mut().spawn((
        DebugSubmenuPanel,
        Node { display: Display::None, ..default() },
    )).id();

    assert_eq!(app.world().resource::<MenuState>().active_submenu, MenuSubmenu::Main);

    let switch_to_debug_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::SwitchSubmenu(MenuSubmenu::Debug),
    )).id();
    app.world_mut().trigger(Activate { entity: switch_to_debug_btn });

    assert_eq!(app.world().resource::<MenuState>().active_submenu, MenuSubmenu::Debug);

    app.update();

    let main_panel_node = app.world().get::<Node>(main_panel_e).unwrap();
    let debug_panel_node = app.world().get::<Node>(debug_panel_e).unwrap();
    assert_eq!(main_panel_node.display, Display::None);
    assert_eq!(debug_panel_node.display, Display::Flex);

    let title_text = app.world().get::<Text>(title_e).unwrap();
    assert_eq!(title_text.0, "PAUSED - DEBUG MENU");

    let switch_to_main_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::SwitchSubmenu(MenuSubmenu::Main),
    )).id();
    app.world_mut().trigger(Activate { entity: switch_to_main_btn });

    assert_eq!(app.world().resource::<MenuState>().active_submenu, MenuSubmenu::Main);

    app.update();

    let main_panel_node = app.world().get::<Node>(main_panel_e).unwrap();
    let debug_panel_node = app.world().get::<Node>(debug_panel_e).unwrap();
    assert_eq!(main_panel_node.display, Display::Flex);
    assert_eq!(debug_panel_node.display, Display::None);

    let title_text = app.world().get::<Text>(title_e).unwrap();
    assert_eq!(title_text.0, "PAUSED - OPTIONS & MENU");
}

#[test]
fn test_debug_submenu_bhop_window_adjustment() {
    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.insert_resource(BhopConfig::default());
    app.add_observer(on_button_activate);
    app.add_systems(Update, update_menu_button_labels);

    let text_child = app.world_mut().spawn(Text::new("BHOP Timing: Initial")).id();
    let bhop_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::CycleBhopWindow,
    )).add_child(text_child).id();

    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Sixtieth);

    app.world_mut().trigger(Activate { entity: bhop_btn });
    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Thirtieth);

    app.update();
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "BHOP Timing: 1/30s (Medium)");

    app.world_mut().trigger(Activate { entity: bhop_btn });
    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Tenth);

    app.update();
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "BHOP Timing: 1/10s (Forgiving)");

    app.world_mut().trigger(Activate { entity: bhop_btn });
    assert_eq!(app.world().resource::<BhopConfig>().window, BhopWindow::Sixtieth);

    app.update();
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "BHOP Timing: 1/60s (Strict)");
}

#[test]
fn test_debug_submenu_noclip_mode_and_collider_bypass() {
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(50));
    app.insert_resource(time);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(BhopConfig::default());
    app.insert_resource(MenuState::default());

    app.add_observer(on_button_activate);
    app.add_systems(
        Update,
        (
            toggle_noclip_system,
            sync_noclip_collider_system,
            elf_player_controller_system,
            update_menu_button_labels,
        ).chain(),
    );

    let player_e = app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(0.0, 1.5, 0.0),
        ElfMovement::default(),
        KinematicCharacterController::default(),
        Collider::capsule_y(0.5, 0.3),
    )).id();

    app.world_mut().spawn(CameraPivot { pitch: 0.0 });

    let text_child = app.world_mut().spawn(Text::new("Noclip Mode: OFF")).id();
    let noclip_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::ToggleNoclip,
    )).add_child(text_child).id();

    assert!(!app.world().resource::<MenuState>().noclip);
    assert!(!app.world().entity(player_e).contains::<ColliderDisabled>());

    app.world_mut().trigger(Activate { entity: noclip_btn });
    assert!(app.world().resource::<MenuState>().noclip);

    app.update();
    assert!(
        app.world().entity(player_e).contains::<ColliderDisabled>(),
        "Player must have ColliderDisabled inserted when noclip is enabled"
    );
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Noclip Mode: ON (Flying)");

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyW);
    }
    app.update();

    let player_tf = app.world().get::<Transform>(player_e).unwrap();
    let controller = app.world().get::<KinematicCharacterController>(player_e).unwrap();
    let movement = app.world().get::<ElfMovement>(player_e).unwrap();

    assert!(controller.translation.is_none(), "Controller translation must be None during noclip");
    assert!(player_tf.translation.z < 0.0, "Player must have translated forward in Z, was {}", player_tf.translation.z);
    assert_eq!(movement.velocity, Vec3::ZERO, "Velocity must remain zero with no gravity during noclip");

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.press(KeyCode::KeyV);
    }
    app.update();

    assert!(!app.world().resource::<MenuState>().noclip, "KeyCode::KeyV must toggle noclip OFF");
    assert!(
        !app.world().entity(player_e).contains::<ColliderDisabled>(),
        "ColliderDisabled must be removed when noclip is disabled"
    );
    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Noclip Mode: OFF");
}

#[test]
fn test_debug_submenu_wireframe_toggle() {
    let mut app = App::new();
    app.insert_resource(MenuState::default());
    app.insert_resource(WireframeConfig::default());
    app.insert_resource(DebugRenderContext::default());
    app.add_observer(on_button_activate);
    app.add_systems(Update, (sync_hud_visibility, update_menu_button_labels));

    let text_child = app.world_mut().spawn(Text::new("Wireframe Mode: OFF")).id();
    let wireframe_btn = app.world_mut().spawn((
        Button,
        MenuButtonAction::ToggleWireframes,
    )).add_child(text_child).id();

    assert!(!app.world().resource::<MenuState>().show_wireframes);

    app.world_mut().trigger(Activate { entity: wireframe_btn });
    assert!(app.world().resource::<MenuState>().show_wireframes);

    app.update();

    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Wireframe Mode: ON");
    assert!(app.world().resource::<DebugRenderContext>().enabled);
    assert!(app.world().resource::<WireframeConfig>().global);

    app.world_mut().trigger(Activate { entity: wireframe_btn });
    assert!(!app.world().resource::<MenuState>().show_wireframes);

    app.update();

    let text = app.world().get::<Text>(text_child).unwrap();
    assert_eq!(text.0, "Wireframe Mode: OFF");
    assert!(!app.world().resource::<DebugRenderContext>().enabled);
    assert!(!app.world().resource::<WireframeConfig>().global);
}

#[test]
fn test_mouse_sensitivity_slider_activation_and_hit_filtering() {
    let mut app = App::new();
    app.init_resource::<MenuState>();
    app.add_observer(on_slider_thumb_drag);
    app.add_observer(on_slider_thumb_over);
    app.add_observer(on_slider_thumb_out);

    let track_e = app.world_mut().spawn((
        SensitivitySlider,
        SensitivitySliderTrack,
    )).id();

    let fill_e = app.world_mut().spawn((
        SensitivitySlider,
        SensitivitySliderFill,
    )).id();

    let thumb_e = app.world_mut().spawn((
        SensitivitySlider,
        SensitivitySliderThumb,
        BackgroundColor(Color::from(tailwind::CYAN_400)),
    )).id();

    let unrelated_e = app.world_mut().spawn(()).id();

    let dummy_pointer = Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(WindowRef::Entity(Entity::PLACEHOLDER).normalize(None).unwrap()),
            position: Vec2::ZERO,
        },
    );

    let dummy_hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);

    assert!(!app.world().resource::<MenuState>().is_open);
    assert_eq!(app.world().resource::<MenuState>().mouse_sensitivity, DEFAULT_SENSITIVITY);

    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Holding Mouse1 when menu is closed must not move sensitivity slider"
    );

    app.world_mut().trigger(PointerDrag {
        entity: unrelated_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Holding Mouse1 on unrelated entity when menu is closed must not move slider"
    );

    app.world_mut().resource_mut::<MenuState>().is_open = true;
    app.world_mut().resource_mut::<MenuState>().active_submenu = MenuSubmenu::Main;

    app.world_mut().trigger(PointerDrag {
        entity: unrelated_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Holding Mouse1 on unrelated entity (not slider) must not move sensitivity slider"
    );

    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Secondary,
        distance: Vec2::new(50.0, 0.0),
        delta: Vec2::new(50.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        DEFAULT_SENSITIVITY,
        "Right-clicking/dragging must not move sensitivity slider"
    );

    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(20.2, 0.0),
        delta: Vec2::new(20.2, 0.0),
    });
    let sens_after_thumb = app.world().resource::<MenuState>().mouse_sensitivity;
    assert!(
        sens_after_thumb > DEFAULT_SENSITIVITY,
        "Dragging slider thumb to the right must increase mouse sensitivity"
    );

    app.world_mut().trigger(PointerDrag {
        entity: track_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(-10.1, 0.0),
        delta: Vec2::new(-10.1, 0.0),
    });
    let sens_after_track = app.world().resource::<MenuState>().mouse_sensitivity;
    assert!(
        sens_after_track < sens_after_thumb,
        "Dragging on slider track to the left must decrease mouse sensitivity"
    );

    app.world_mut().trigger(PointerDrag {
        entity: fill_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(10.1, 0.0),
        delta: Vec2::new(10.1, 0.0),
    });
    assert!(
        app.world().resource::<MenuState>().mouse_sensitivity > sens_after_track,
        "Dragging on slider fill must adjust mouse sensitivity"
    );

    app.world_mut().resource_mut::<MenuState>().active_submenu = MenuSubmenu::Debug;
    let sens_before_debug = app.world().resource::<MenuState>().mouse_sensitivity;
    app.world_mut().trigger(PointerDrag {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(30.0, 0.0),
        delta: Vec2::new(30.0, 0.0),
    });
    assert_eq!(
        app.world().resource::<MenuState>().mouse_sensitivity,
        sens_before_debug,
        "Dragging slider when debug submenu is active must not move slider"
    );

    app.world_mut().resource_mut::<MenuState>().active_submenu = MenuSubmenu::Main;
    app.world_mut().trigger(PointerOver {
        entity: unrelated_e,
        pointer: dummy_pointer.clone(),
        hit: dummy_hit.clone(),
    });
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb_e).unwrap().0,
        Color::from(tailwind::CYAN_400),
        "Hovering unrelated entity must not highlight slider thumb"
    );

    app.world_mut().trigger(PointerOver {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        hit: dummy_hit.clone(),
    });
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb_e).unwrap().0,
        Color::from(tailwind::CYAN_300),
        "Hovering slider thumb must highlight it to CYAN_300"
    );

    app.world_mut().trigger(PointerOut {
        entity: thumb_e,
        pointer: dummy_pointer.clone(),
        hit: dummy_hit,
    });
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb_e).unwrap().0,
        Color::from(tailwind::CYAN_400),
        "PointerOut must restore thumb to CYAN_400"
    );
}

#[test]
fn test_interactive_object_text_name_display() {
    let mut app = App::new();
    app.init_resource::<CurrentInteractiveTarget>();
    app.add_systems(Update, update_interactive_object_name_display_system);

    let prompt_e = app.world_mut().spawn((
        InteractionPromptText,
        InteractiveObjectNameText,
        Text::new("[Q] / [E] to grab presents & tools"),
    )).id();

    let cam_e = app.world_mut().spawn((
        FirstPersonCamera,
        Transform::from_xyz(0.0, 1.5, 0.0).looking_at(Vec3::new(0.0, 1.5, -5.0), Vec3::Y),
        GlobalTransform::from(Transform::from_xyz(0.0, 1.5, 0.0).looking_at(Vec3::new(0.0, 1.5, -5.0), Vec3::Y)),
    )).id();

    app.update();
    assert_eq!(
        app.world().get::<Text>(prompt_e).unwrap().0,
        "[Q] / [E] to grab presents & tools"
    );
    assert!(app.world().resource::<CurrentInteractiveTarget>().name.is_none());

    let oven_e = app.world_mut().spawn((
        GasFurnaceStove {
            is_on: true,
            is_lit: true,
            heat_output: 1.5,
            is_heated: true,
            internal_temp: 20.0,
        },
        Transform::from_xyz(0.0, 1.5, -2.0),
    )).id();

    app.update();
    assert_eq!(
        app.world().get::<Text>(prompt_e).unwrap().0,
        "Oven",
        "Must display 'Oven' when near stove/oven"
    );
    assert_eq!(
        app.world().resource::<CurrentInteractiveTarget>().name.as_deref(),
        Some("Oven")
    );

    app.world_mut().entity_mut(oven_e).despawn();

    let _present_e = app.world_mut().spawn((
        Present::default(),
        Transform::from_xyz(0.0, 1.5, -1.5),
    )).id();

    app.update();
    assert_eq!(
        app.world().get::<Text>(prompt_e).unwrap().0,
        "Present",
        "Must display 'Present' when near a present"
    );
    assert_eq!(
        app.world().resource::<CurrentInteractiveTarget>().name.as_deref(),
        Some("Present")
    );

    {
        let mut cam_tf = app.world_mut().get_mut::<Transform>(cam_e).unwrap();
        cam_tf.translation = Vec3::new(0.0, 1.5, 10.0);
    }
    let cam_tf = *app.world().get::<Transform>(cam_e).unwrap();
    let mut cam_gt = app.world_mut().get_mut::<GlobalTransform>(cam_e).unwrap();
    *cam_gt = GlobalTransform::from(cam_tf);
    app.update();
    assert_eq!(
        app.world().get::<Text>(prompt_e).unwrap().0,
        "[Q] / [E] to grab presents & tools",
        "Must revert to default text when no interactive object is within interaction distance"
    );
}

#[test]
fn test_debug_overlay_player_xyz_and_crosshair_raycast_collision_point() {
    let mut app = App::new();
    app.init_resource::<MenuState>();
    app.init_resource::<DebugOverlayData>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_systems(
        Update,
        (
            toggle_debug_overlay_system,
            sync_hud_visibility,
            update_debug_overlay_system,
        ).chain(),
    );

    let hud_entity = app.world_mut().spawn((
        DebugOverlayHud,
        Visibility::Hidden,
    )).id();

    let text_entity = app.world_mut().spawn((
        DebugOverlayText,
        Text::new(""),
    )).id();

    app.update();
    assert_eq!(*app.world().get::<Visibility>(hud_entity).unwrap(), Visibility::Hidden);
    assert!(!app.world().resource::<DebugOverlayData>().is_active);

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::F3);
    }
    app.update();

    assert!(app.world().resource::<MenuState>().debug_overlay, "F3 must enable debug_overlay in MenuState");
    assert_eq!(*app.world().get::<Visibility>(hud_entity).unwrap(), Visibility::Visible, "DebugOverlayHud must become Visible");
    assert!(app.world().resource::<DebugOverlayData>().is_active, "DebugOverlayData must become active");

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset(KeyCode::F3);
    }

    let player_entity = app.world_mut().spawn((
        ElfPlayer,
        Transform::from_xyz(2.5, 1.2, -4.0),
    )).id();

    let camera_entity = app.world_mut().spawn((
        FirstPersonCamera,
        Transform::from_xyz(2.5, 1.8, -4.0),
        GlobalTransform::from(Transform::from_xyz(2.5, 1.8, -4.0)),
    )).id();

    let wall_entity = app.world_mut().spawn((
        Transform::from_xyz(2.5, 1.8, -10.0),
        GlobalTransform::from(Transform::from_xyz(2.5, 1.8, -10.0)),
        Collider::cuboid(5.0, 5.0, 0.2),
    )).id();

    app.update();

    {
        let data = app.world().resource::<DebugOverlayData>();
        assert_eq!(data.player_position, Vec3::new(2.5, 1.2, -4.0), "Player position must match player XYZ");
        assert!(data.crosshair_raycast_hit.is_some(), "Raycast must hit the wall directly in front");
        let hit = data.crosshair_raycast_hit.unwrap();
        assert!((hit.x - 2.5).abs() < 0.01, "Hit X must be 2.5, got {}", hit.x);
        assert!((hit.y - 1.8).abs() < 0.01, "Hit Y must be 1.8, got {}", hit.y);
        assert!((hit.z - (-9.8)).abs() < 0.02, "Hit Z must hit front face (-9.8), got {}", hit.z);

        let text = app.world().get::<Text>(text_entity).unwrap();
        assert!(text.0.contains("Player Pos: X: 2.50, Y: 1.20, Z: -4.00"), "Text must display player XYZ coordinates: {}", text.0);
        assert!(text.0.contains("Crosshair Raycast: X: 2.50, Y: 1.80, Z: -9.80"), "Text must display raycast hit XYZ coordinates: {}", text.0);
    }

    {
        let up_tf = Transform::from_xyz(2.5, 1.8, -4.0).looking_to(Vec3::Y, Vec3::NEG_Z);
        let mut cam_tf = app.world_mut().get_mut::<Transform>(camera_entity).unwrap();
        *cam_tf = up_tf;
        let mut cam_gt = app.world_mut().get_mut::<GlobalTransform>(camera_entity).unwrap();
        *cam_gt = GlobalTransform::from(up_tf);
    }

    app.update();

    {
        let data = app.world().resource::<DebugOverlayData>();
        assert_eq!(data.crosshair_raycast_hit, None, "Raycast into empty sky must return None");
        let text = app.world().get::<Text>(text_entity).unwrap();
        assert!(text.0.contains("Crosshair Raycast: No Hit"), "Text must display 'No Hit' when raycast misses: {}", text.0);
    }

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset(KeyCode::F3);
        keys.press(KeyCode::F3);
    }
    app.update();

    assert!(!app.world().resource::<MenuState>().debug_overlay, "F3 must disable debug_overlay in MenuState");
    assert_eq!(*app.world().get::<Visibility>(hud_entity).unwrap(), Visibility::Hidden, "DebugOverlayHud must become Hidden");
    assert!(!app.world().resource::<DebugOverlayData>().is_active, "DebugOverlayData must become inactive");

    app.world_mut().despawn(player_entity);
    app.world_mut().despawn(camera_entity);
    app.world_mut().despawn(wall_entity);
}
