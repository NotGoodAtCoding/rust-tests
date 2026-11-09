use std::collections::HashMap;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use crate::elf_game::{
    player::{
        ElfHands, ElfMovement, HandSide,
        controller::{ElfPlayer, FirstPersonCamera, LeftHandVisual},
        hands::Grabbable,
    },
};

/// 2D Sprite viewmodel hand poses reflecting player locomotion and interaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum HandPose {
    #[default]
    Idle,
    Walking,
    Sprinting,
    Grabbing,
    Holding,
}

/// Dynamic state machine component attached to each viewmodel hand
#[derive(Component, Debug, Clone)]
pub struct HandPoseState {
    pub side: HandSide,
    pub current_pose: HandPose,
    pub previous_pose: HandPose,
    pub pose_timer: f32,
    pub forward_reach: f32,
    pub tilt_angle: f32,
}

impl HandPoseState {
    pub fn new(side: HandSide) -> Self {
        Self {
            side,
            current_pose: HandPose::Idle,
            previous_pose: HandPose::Idle,
            pose_timer: 0.0,
            forward_reach: 0.0,
            tilt_angle: 0.0,
        }
    }
}

/// Tag component for the 2D billboard sprite quad rendering the hand glove texture
#[derive(Component, Debug, Clone)]
pub struct ViewmodelHandSprite {
    pub side: HandSide,
}

/// Resource caching pre-generated procedural materials for all (HandSide, HandPose) pairs
#[derive(Resource, Default, Clone)]
pub struct HandSpriteMaterials {
    pub materials: HashMap<(HandSide, HandPose), Handle<StandardMaterial>>,
}

impl HandSpriteMaterials {
    pub fn get(&self, side: HandSide, pose: HandPose) -> Option<Handle<StandardMaterial>> {
        self.materials.get(&(side, pose)).cloned()
    }
}

/// Generates procedural 32x32 RGBA sprite textures for elf mittens
pub fn generate_hand_sprite_image(pose: HandPose, side: HandSide) -> Image {
    const W: u32 = 32;
    const H: u32 = 32;
    let mut data = vec![0u8; (W * H * 4) as usize];

    let set_pixel = |x: i32, y: i32, r: u8, g: u8, b: u8, a: u8, data: &mut [u8]| {
        if x >= 0 && x < W as i32 && y >= 0 && y < H as i32 {
            let idx = ((y as u32 * W + x as u32) * 4) as usize;
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = a;
        }
    };

    let is_left = matches!(side, HandSide::Left);

    // Mitten colors: Festive holiday emerald green body with warm gold stitch and fluffy white cuff
    let (body_r, body_g, body_b) = if is_left {
        (28, 148, 68) // Emerald Green
    } else {
        (198, 38, 48) // Festive Red
    };
    let (cuff_r, cuff_g, cuff_b) = (245, 245, 240); // Fluffy White
    let (gold_r, gold_g, gold_b) = (235, 195, 45); // Gold Trim
    let (shadow_r, shadow_g, shadow_b) = (body_r / 2, body_g / 2, body_b / 2);

    // 1. Base fluffy cuff at bottom (y = 23..29)
    for y in 23..30 {
        let span = if y == 23 || y == 29 { 12 } else { 14 };
        for x in (16 - span / 2)..(16 + span / 2) {
            set_pixel(x, y, cuff_r, cuff_g, cuff_b, 255, &mut data);
        }
    }

    // Gold trim band above cuff (y = 21..23)
    for x in 10..22 {
        set_pixel(x, 22, gold_r, gold_g, gold_b, 255, &mut data);
    }

    // 2. Mitten Body & Pose Silhouettes
    match pose {
        HandPose::Idle => {
            // Rounded relaxed palm (y = 9..22, x = 10..22)
            for y in 10..22 {
                for x in 10..22 {
                    let d_center = (x - 16) * (x - 16) + (y - 16) * (y - 16);
                    if d_center <= 36 {
                        set_pixel(x, y, body_r, body_g, body_b, 255, &mut data);
                    }
                }
            }
            // Gentle resting thumb
            let thumb_x_start = if is_left { 20 } else { 8 };
            let thumb_x_end = if is_left { 25 } else { 13 };
            for y in 14..19 {
                for x in thumb_x_start..thumb_x_end {
                    set_pixel(x, y, body_r, body_g, body_b, 255, &mut data);
                }
            }
        }
        HandPose::Walking => {
            // Slightly swaying mitten tilted along stride
            let offset_x = if is_left { -1 } else { 1 };
            for y in 10..22 {
                for x in 10..22 {
                    let ox = x + (22 - y) * offset_x / 8;
                    let d_center = (ox - 16) * (ox - 16) + (y - 16) * (y - 16);
                    if d_center <= 38 {
                        set_pixel(ox, y, body_r, body_g, body_b, 255, &mut data);
                    }
                }
            }
            // Thumb
            let thumb_x = if is_left { 21 } else { 9 };
            for y in 15..20 {
                set_pixel(thumb_x, y, body_r, body_g, body_b, 255, &mut data);
                set_pixel(thumb_x + if is_left { 1 } else { -1 }, y, body_r, body_g, body_b, 255, &mut data);
            }
        }
        HandPose::Sprinting => {
            // Forward raked streamlined mitten with high-speed wind streaks
            for y in 8..22 {
                let slant = (22 - y) * 2 / 5;
                for x in (11 + slant)..(21 + slant) {
                    set_pixel(x, y, body_r, body_g, body_b, 255, &mut data);
                }
            }
            // Motion streaks
            for x in 3..8 {
                set_pixel(x, 14, 255, 255, 230, 200, &mut data);
                set_pixel(x + 2, 18, 255, 255, 230, 180, &mut data);
            }
        }
        HandPose::Grabbing => {
            // Reaching anticipation pinch pose: fingers flare forward with wide gap before thumb
            for y in 6..20 {
                let finger_x_start = if is_left { 9 } else { 13 };
                let finger_x_end = if is_left { 17 } else { 21 };
                for x in finger_x_start..finger_x_end {
                    set_pixel(x, y, body_r, body_g, body_b, 255, &mut data);
                }
            }
            // Open extended reaching thumb
            let thumb_x_start = if is_left { 20 } else { 5 };
            let thumb_x_end = if is_left { 27 } else { 12 };
            for y in 12..17 {
                for x in thumb_x_start..thumb_x_end {
                    set_pixel(x, y, body_r, body_g, body_b, 255, &mut data);
                }
            }
            // Highlight pinch focus point
            let focus_x = if is_left { 18 } else { 13 };
            set_pixel(focus_x, 9, gold_r, gold_g, gold_b, 255, &mut data);
        }
        HandPose::Holding => {
            // Tight clenched fist wrapped around object with knuckles and grip fold
            for y in 11..22 {
                for x in 10..22 {
                    let d_center = (x - 16) * (x - 16) + (y - 16) * (y - 16);
                    if d_center <= 32 {
                        set_pixel(x, y, body_r, body_g, body_b, 255, &mut data);
                    }
                }
            }
            // Clamped curled thumb over palm
            for x in 12..20 {
                set_pixel(x, 14, shadow_r, shadow_g, shadow_b, 255, &mut data);
                set_pixel(x, 15, body_r, body_g, body_b, 255, &mut data);
            }
            // Knuckle line highlights
            for x in 11..21 {
                if x % 3 == 0 {
                    set_pixel(x, 11, gold_r, gold_g, gold_b, 255, &mut data);
                }
            }
        }
    }

    Image::new(
        Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Initializes and builds `HandSpriteMaterials` resource with procedural textures
pub fn create_hand_sprite_materials(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> HandSpriteMaterials {
    let mut map = HashMap::new();
    let poses = [
        HandPose::Idle,
        HandPose::Walking,
        HandPose::Sprinting,
        HandPose::Grabbing,
        HandPose::Holding,
    ];
    let sides = [HandSide::Left, HandSide::Right];

    for &side in &sides {
        for &pose in &poses {
            let img = generate_hand_sprite_image(pose, side);
            let img_handle = images.add(img);
            let mat_handle = materials.add(StandardMaterial {
                base_color_texture: Some(img_handle),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            });
            map.insert((side, pose), mat_handle);
        }
    }
    HandSpriteMaterials { materials: map }
}

/// State machine evaluating locomotion, input, and interaction queries to drive hand poses
pub fn hand_pose_state_machine_system(
    time: Option<Res<Time>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    q_player: Query<(&Transform, &ElfMovement, &ElfHands), With<ElfPlayer>>,
    q_camera: Query<&GlobalTransform, With<FirstPersonCamera>>,
    q_grabbable: Query<&GlobalTransform, (With<Grabbable>, Without<ElfPlayer>)>,
    mut q_hand_states: Query<&mut HandPoseState>,
    mut q_sprites: Query<(&ViewmodelHandSprite, &mut MeshMaterial3d<StandardMaterial>)>,
    sprite_mats: Option<Res<HandSpriteMaterials>>,
) {
    let Ok((_, movement, elf_hands)) = q_player.single() else {
        return;
    };

    let dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);

    // Calculate reticle proximity to grabbable objects
    let mut is_near_grabbable = false;
    if let Ok(cam_gtf) = q_camera.single() {
        let cam_pos = cam_gtf.translation();
        let cam_forward = cam_gtf.forward().as_vec3();
        for gtf in &q_grabbable {
            let to_obj = gtf.translation() - cam_pos;
            let dist_sq = to_obj.length_squared();
            if dist_sq <= 1.85 * 1.85 {
                let dir = to_obj.normalize_or_zero();
                if dir.dot(cam_forward) > 0.55 {
                    is_near_grabbable = true;
                    break;
                }
            }
        }
    }

    let q_pressed = keys.as_ref().map(|k| k.pressed(KeyCode::KeyQ)).unwrap_or(false);
    let e_pressed = keys.as_ref().map(|k| k.pressed(KeyCode::KeyE)).unwrap_or(false);

    let h_speed = Vec2::new(movement.velocity.x, movement.velocity.z).length();
    let is_sprinting = h_speed >= 7.5 || movement.consecutive_bhops >= 2;
    let is_walking = movement.is_grounded && h_speed > 0.5;

    for mut hand_state in &mut q_hand_states {
        let side = hand_state.side;
        let is_holding = match side {
            HandSide::Left => elf_hands.left_grabbed.is_some(),
            HandSide::Right => elf_hands.right_grabbed.is_some(),
        };
        let is_grabbing_input = match side {
            HandSide::Left => q_pressed,
            HandSide::Right => e_pressed,
        };

        let target_pose = if is_holding {
            HandPose::Holding
        } else if is_grabbing_input || is_near_grabbable {
            HandPose::Grabbing
        } else if is_sprinting {
            HandPose::Sprinting
        } else if is_walking {
            HandPose::Walking
        } else {
            HandPose::Idle
        };

        if target_pose != hand_state.current_pose {
            hand_state.previous_pose = hand_state.current_pose;
            hand_state.current_pose = target_pose;
            hand_state.pose_timer = 0.0;

            // Update sprite quad material handle
            if let Some(ref mats) = sprite_mats {
                if let Some(new_mat) = mats.get(side, target_pose) {
                    for (sprite, mut mat_handle) in &mut q_sprites {
                        if sprite.side == side {
                            *mat_handle = MeshMaterial3d(new_mat.clone());
                        }
                    }
                }
            }
        } else {
            hand_state.pose_timer += dt;
        }
    }
}

/// Applies dynamic procedural viewmodel motion and forward-reach transforms based on pose
pub fn hand_pose_animation_system(
    time: Option<Res<Time>>,
    mut q_hands: Query<(&mut Transform, &mut HandPoseState, Has<LeftHandVisual>)>,
) {
    let mut dt = time.as_ref().map(|t| t.delta_secs()).unwrap_or(0.016);
    if dt <= 0.0 {
        dt = 0.016;
    }
    let elapsed = time.as_ref().map(|t| t.elapsed_secs()).unwrap_or(0.0);

    for (mut tf, mut state, is_left) in &mut q_hands {
        match state.current_pose {
            HandPose::Idle => {
                state.forward_reach = state.forward_reach.lerp(0.0, dt * 10.0);
                state.tilt_angle = state.tilt_angle.lerp(0.0, dt * 8.0);
            }
            HandPose::Walking => {
                state.forward_reach = state.forward_reach.lerp(0.0, dt * 8.0);
                let sway = (elapsed * 8.0 + if is_left { 0.0 } else { std::f32::consts::PI }).sin() * 0.08;
                state.tilt_angle = state.tilt_angle.lerp(sway, dt * 10.0);
            }
            HandPose::Sprinting => {
                state.forward_reach = state.forward_reach.lerp(0.03, dt * 12.0);
                state.tilt_angle = state.tilt_angle.lerp(0.24, dt * 12.0);
            }
            HandPose::Grabbing => {
                // Reach outward and forward towards targeted item
                state.forward_reach = state.forward_reach.lerp(0.075, dt * 14.0);
                let inward = if is_left { 0.12 } else { -0.12 };
                state.tilt_angle = state.tilt_angle.lerp(inward, dt * 10.0);
            }
            HandPose::Holding => {
                // Clamped firmly in viewmodel carry slot
                state.forward_reach = state.forward_reach.lerp(0.0, dt * 12.0);
                let inward_clamp = if is_left { 0.16 } else { -0.16 };
                state.tilt_angle = state.tilt_angle.lerp(inward_clamp, dt * 10.0);
            }
        }

        // Apply forward reach offset along local viewmodel Z
        tf.translation.z -= state.forward_reach * dt * 5.0;
        tf.rotation = Quat::from_rotation_z(state.tilt_angle);
    }
}
