use ab_glyph::{point, Font, FontRef, ScaleFont};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

/// Renders multi-line text into a Bevy `Image` texture with OpenType antialiased font
/// rasterization and dedicated procedural box-drawing / symbol support.
pub fn render_text_to_image(
    text: &str,
    font_scale: f32,
    text_color: Color,
    bg_color: Color,
    width: u32,
    height: u32,
    center_align: bool,
) -> Image {
    let mut data = vec![0u8; (width * height * 4) as usize];
    fill_text_pixel_data(
        text,
        font_scale,
        text_color,
        bg_color,
        width,
        height,
        center_align,
        &mut data,
    );

    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Updates an existing image data buffer in place with rasterized text
pub fn update_text_image_data(
    text: &str,
    font_scale: f32,
    text_color: Color,
    bg_color: Color,
    width: u32,
    height: u32,
    center_align: bool,
    data: &mut [u8],
) {
    fill_text_pixel_data(
        text,
        font_scale,
        text_color,
        bg_color,
        width,
        height,
        center_align,
        data,
    );
}

fn fill_text_pixel_data(
    text: &str,
    font_scale: f32,
    text_color: Color,
    bg_color: Color,
    width: u32,
    height: u32,
    center_align: bool,
    data: &mut [u8],
) {
    let font = match FontRef::try_from_slice(bevy::text::DEFAULT_FONT_DATA) {
        Ok(f) => f,
        Err(_) => return,
    };

    let bg_rgba = bg_color.to_srgba();
    let bg_r = (bg_rgba.red * 255.0).clamp(0.0, 255.0) as u8;
    let bg_g = (bg_rgba.green * 255.0).clamp(0.0, 255.0) as u8;
    let bg_b = (bg_rgba.blue * 255.0).clamp(0.0, 255.0) as u8;
    let bg_a = (bg_rgba.alpha * 255.0).clamp(0.0, 255.0) as u8;

    // Fill background
    for pixel in data.chunks_exact_mut(4) {
        pixel[0] = bg_r;
        pixel[1] = bg_g;
        pixel[2] = bg_b;
        pixel[3] = bg_a;
    }

    let text_rgba = text_color.to_srgba();
    let tr = text_rgba.red * 255.0;
    let tg = text_rgba.green * 255.0;
    let tb = text_rgba.blue * 255.0;
    let ta = text_rgba.alpha;

    let scaled_font = font.as_scaled(font_scale);
    let ascent = scaled_font.ascent();
    let descent = scaled_font.descent(); // negative
    let line_height = (ascent - descent).max(font_scale * 1.15);
    let char_w = scaled_font.h_advance(font.glyph_id('M')).max(font_scale * 0.6);

    let lines: Vec<&str> = text.lines().collect();
    let total_text_height = lines.len() as f32 * line_height;
    let start_y = ((height as f32 - total_text_height) * 0.5).max(4.0);

    for (line_idx, line) in lines.iter().enumerate() {
        let line_chars: Vec<char> = line.chars().collect();
        let line_width = line_chars.len() as f32 * char_w;
        let start_x = if center_align {
            ((width as f32 - line_width) * 0.5).max(4.0)
        } else {
            12.0
        };

        let y = start_y + line_idx as f32 * line_height;

        for (col_idx, &c) in line_chars.iter().enumerate() {
            if c == ' ' {
                continue;
            }
            let x = start_x + col_idx as f32 * char_w;

            // Dedicated procedural rendering for box drawing and symbols missing from font
            if is_procedural_char(c) {
                draw_procedural_char(c, x, y, char_w, line_height, tr, tg, tb, ta, width, height, data);
            } else {
                let glyph_id = font.glyph_id(c);
                if glyph_id.0 != 0 {
                    let glyph = glyph_id.with_scale_and_position(font_scale, point(x, y + ascent));
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        outlined.draw(|px, py, coverage| {
                            if px < width && py < height {
                                let idx = ((py * width + px) * 4) as usize;
                                if idx + 3 < data.len() {
                                    blend_pixel(&mut data[idx..idx + 4], tr, tg, tb, ta * coverage);
                                }
                            }
                        });
                    }
                } else {
                    // Fallback for missing unicode characters
                    draw_procedural_char(c, x, y, char_w, line_height, tr, tg, tb, ta, width, height, data);
                }
            }
        }
    }
}

fn is_procedural_char(c: char) -> bool {
    matches!(
        c,
        '─' | '│' | '┌' | '┐' | '└' | '┘' | '┬' | '┴' | '├' | '┤' | '┼'
            | '═' | '║' | '╔' | '╗' | '╚' | '╝' | '╦' | '╩' | '╠' | '╣' | '╬'
            | '▲' | '▼' | '►' | '◄' | '•'
    )
}

fn draw_procedural_char(
    c: char,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    tr: f32,
    tg: f32,
    tb: f32,
    ta: f32,
    tex_w: u32,
    tex_h: u32,
    data: &mut [u8],
) {
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let thick = (w * 0.12).clamp(1.5, 4.0);
    let half_t = thick * 0.5;

    let mut draw_rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
        let ix0 = (x0.max(0.0) as u32).min(tex_w);
        let ix1 = (x1.max(0.0) as u32).min(tex_w);
        let iy0 = (y0.max(0.0) as u32).min(tex_h);
        let iy1 = (y1.max(0.0) as u32).min(tex_h);

        for py in iy0..iy1 {
            for px in ix0..ix1 {
                let idx = ((py * tex_w + px) * 4) as usize;
                if idx + 3 < data.len() {
                    blend_pixel(&mut data[idx..idx + 4], tr, tg, tb, ta);
                }
            }
        }
    };

    match c {
        // Single-line horizontal
        '─' => draw_rect(x, cy - half_t, x + w, cy + half_t),
        // Single-line vertical
        '│' => draw_rect(cx - half_t, y, cx + half_t, y + h),
        // Corners
        '┌' => {
            draw_rect(cx - half_t, cy - half_t, x + w, cy + half_t);
            draw_rect(cx - half_t, cy - half_t, cx + half_t, y + h);
        }
        '┐' => {
            draw_rect(x, cy - half_t, cx + half_t, cy + half_t);
            draw_rect(cx - half_t, cy - half_t, cx + half_t, y + h);
        }
        '└' => {
            draw_rect(cx - half_t, cy - half_t, x + w, cy + half_t);
            draw_rect(cx - half_t, y, cx + half_t, cy + half_t);
        }
        '┘' => {
            draw_rect(x, cy - half_t, cx + half_t, cy + half_t);
            draw_rect(cx - half_t, y, cx + half_t, cy + half_t);
        }
        '┬' => {
            draw_rect(x, cy - half_t, x + w, cy + half_t);
            draw_rect(cx - half_t, cy - half_t, cx + half_t, y + h);
        }
        '┴' => {
            draw_rect(x, cy - half_t, x + w, cy + half_t);
            draw_rect(cx - half_t, y, cx + half_t, cy + half_t);
        }
        '├' => {
            draw_rect(cx - half_t, y, cx + half_t, y + h);
            draw_rect(cx - half_t, cy - half_t, x + w, cy + half_t);
        }
        '┤' => {
            draw_rect(cx - half_t, y, cx + half_t, y + h);
            draw_rect(x, cy - half_t, cx + half_t, cy + half_t);
        }
        '┼' => {
            draw_rect(x, cy - half_t, x + w, cy + half_t);
            draw_rect(cx - half_t, y, cx + half_t, y + h);
        }
        // Double-line horizontal
        '═' => {
            let offset = thick * 0.9;
            draw_rect(x, cy - offset - half_t, x + w, cy - offset + half_t);
            draw_rect(x, cy + offset - half_t, x + w, cy + offset + half_t);
        }
        // Double-line vertical
        '║' => {
            let offset = thick * 0.9;
            draw_rect(cx - offset - half_t, y, cx - offset + half_t, y + h);
            draw_rect(cx + offset - half_t, y, cx + offset + half_t, y + h);
        }
        // Double corners
        '╔' => {
            let offset = thick * 0.9;
            draw_rect(cx - offset - half_t, cy - offset - half_t, x + w, cy - offset + half_t);
            draw_rect(cx + offset - half_t, cy + offset - half_t, x + w, cy + offset + half_t);
            draw_rect(cx - offset - half_t, cy - offset - half_t, cx - offset + half_t, y + h);
            draw_rect(cx + offset - half_t, cy + offset - half_t, cx + offset + half_t, y + h);
        }
        '╗' => {
            let offset = thick * 0.9;
            draw_rect(x, cy - offset - half_t, cx + offset + half_t, cy - offset + half_t);
            draw_rect(x, cy + offset - half_t, cx - offset + half_t, cy + offset + half_t);
            draw_rect(cx + offset - half_t, cy - offset - half_t, cx + offset + half_t, y + h);
            draw_rect(cx - offset - half_t, cy + offset - half_t, cx - offset + half_t, y + h);
        }
        '╚' => {
            let offset = thick * 0.9;
            draw_rect(cx - offset - half_t, cy + offset - half_t, x + w, cy + offset + half_t);
            draw_rect(cx + offset - half_t, cy - offset - half_t, x + w, cy - offset + half_t);
            draw_rect(cx - offset - half_t, y, cx - offset + half_t, cy + offset + half_t);
            draw_rect(cx + offset - half_t, y, cx + offset + half_t, cy - offset + half_t);
        }
        '╝' => {
            let offset = thick * 0.9;
            draw_rect(x, cy + offset - half_t, cx + offset + half_t, cy + offset + half_t);
            draw_rect(x, cy - offset - half_t, cx - offset + half_t, cy - offset + half_t);
            draw_rect(cx + offset - half_t, y, cx + offset + half_t, cy + offset + half_t);
            draw_rect(cx - offset - half_t, y, cx - offset + half_t, cy - offset + half_t);
        }
        // Triangle pointing up ▲
        '▲' => {
            let top_y = cy - h * 0.35;
            let bot_y = cy + h * 0.35;
            let half_base = w * 0.38;
            let iy0 = (top_y.max(0.0) as u32).min(tex_h);
            let iy1 = (bot_y.max(0.0) as u32).min(tex_h);
            for py in iy0..iy1 {
                let progress = (py as f32 - top_y) / (bot_y - top_y).max(0.001);
                let cur_half = half_base * progress;
                let ix0 = ((cx - cur_half).max(0.0) as u32).min(tex_w);
                let ix1 = ((cx + cur_half).max(0.0) as u32).min(tex_w);
                for px in ix0..ix1 {
                    let idx = ((py * tex_w + px) * 4) as usize;
                    if idx + 3 < data.len() {
                        blend_pixel(&mut data[idx..idx + 4], tr, tg, tb, ta);
                    }
                }
            }
        }
        // Bullet •
        '•' => {
            let r = (w * 0.20).clamp(2.0, 8.0);
            let r_sq = r * r;
            let ix0 = ((cx - r).max(0.0) as u32).min(tex_w);
            let ix1 = ((cx + r + 1.0).max(0.0) as u32).min(tex_w);
            let iy0 = ((cy - r).max(0.0) as u32).min(tex_h);
            let iy1 = ((cy + r + 1.0).max(0.0) as u32).min(tex_h);
            for py in iy0..iy1 {
                for px in ix0..ix1 {
                    let dx = px as f32 + 0.5 - cx;
                    let dy = py as f32 + 0.5 - cy;
                    if dx * dx + dy * dy <= r_sq {
                        let idx = ((py * tex_w + px) * 4) as usize;
                        if idx + 3 < data.len() {
                            blend_pixel(&mut data[idx..idx + 4], tr, tg, tb, ta);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

#[inline(always)]
fn blend_pixel(pixel: &mut [u8], tr: f32, tg: f32, tb: f32, src_alpha: f32) {
    if src_alpha <= 0.005 {
        return;
    }
    let src_a = src_alpha.clamp(0.0, 1.0);
    let inv_a = 1.0 - src_a;

    let dst_r = pixel[0] as f32;
    let dst_g = pixel[1] as f32;
    let dst_b = pixel[2] as f32;
    let dst_a = pixel[3] as f32 / 255.0;

    let out_a = src_a + dst_a * inv_a;
    if out_a > 0.001 {
        let out_r = (tr * src_a + dst_r * dst_a * inv_a) / out_a;
        let out_g = (tg * src_a + dst_g * dst_a * inv_a) / out_a;
        let out_b = (tb * src_a + dst_b * dst_a * inv_a) / out_a;

        pixel[0] = out_r.clamp(0.0, 255.0) as u8;
        pixel[1] = out_g.clamp(0.0, 255.0) as u8;
        pixel[2] = out_b.clamp(0.0, 255.0) as u8;
        pixel[3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
    }
}
