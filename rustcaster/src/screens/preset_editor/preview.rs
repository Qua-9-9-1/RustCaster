// Live preview of the edited preset: a tiny fixed-camera corridor drawn
// column by column (side walls, back wall, floor, ceiling or sky), plus a
// grid block, a coin and the end flag as billboards. Only an approximation of
// the in-game look, cheap enough to redraw every frame.

use crate::assets::preset::Preset;
use crate::assets::texture::Texture;
use crate::render::color::{blend_u32, Color};
use crate::render::framebuffer::{FrameBuffer, Rect};

/// Corridor half width and depth of the back wall, in tiles.
const HALF_WIDTH: f64 = 1.0;
const BACK_DEPTH: f64 = 3.0;
/// Camera focal length as a fraction of the preview width.
const FOCAL_RATIO: f64 = 0.7;
/// Indoor darkening: brightness is multiplied by `SHADE_BASE` per `SHADE_DIST` tiles.
const SHADE_BASE: f64 = 0.8;
const SHADE_DIST: f64 = 1.2;
/// Faces along the corridor are a bit darker (like `Side::X` in game).
const SIDE_SHADE: f64 = 0.8;
/// Outdoor fog reaches its maximum at this distance, never fully opaque.
const FOG_DIST: f64 = 4.0;
const FOG_MAX: f64 = 0.85;

/// Sprites: (world x, depth, size) in tiles.
const COIN: (f64, f64, f64) = (-0.45, 1.6, 0.35);
const END: (f64, f64, f64) = (0.5, 2.6, 0.7);
/// Grid block face: x range and depth.
const GRID_X: (f64, f64) = (0.15, 1.0);
const GRID_DEPTH: f64 = 2.1;

struct Cam {
    rect: Rect,
    cx: f64,
    horizon: f64,
    focal: f64,
}

impl Cam {
    fn screen_x(&self, wx: f64, z: f64) -> f64 {
        self.cx + wx * self.focal / z
    }

    /// Screen rows of a vertical span from the floor (`y = 0`) up to `height` tiles at depth `z`.
    fn span(&self, z: f64, height: f64) -> (f64, f64) {
        let bottom = self.horizon + 0.5 * self.focal / z;
        (bottom - height * self.focal / z, bottom)
    }
}

pub fn draw_preview(fb: &mut FrameBuffer, rect: Rect, preset: &Preset, time: f64) {
    let area = rect.intersect(&fb.bounds());
    if area.w <= 0 || area.h <= 0 {
        return;
    }
    let cam = Cam {
        rect,
        cx: rect.x as f64 + rect.w as f64 / 2.0,
        horizon: rect.y as f64 + rect.h as f64 / 2.0,
        focal: rect.w as f64 * FOCAL_RATIO,
    };
    draw_corridor(fb, &cam, area, preset, time);

    let old_clip = fb.clip();
    fb.set_clip(Some(area));
    draw_end(fb, &cam, preset);
    draw_grid(fb, &cam, preset);
    draw_coin(fb, &cam, preset, time);
    fb.set_clip(Some(old_clip));
}

fn shade(p: u32, factor: f64) -> u32 {
    Color::from_u32(p | 0xFF00_0000).scale(factor).to_u32()
}

/// Distance effect: darkening indoors, fog towards the sky colour outside.
fn apply_distance(p: u32, z: f64, preset: &Preset) -> u32 {
    if preset.outside {
        let fog = preset.fog_amount();
        if fog > 0.0 {
            let t = (fog * z / FOG_DIST).min(1.0) * FOG_MAX;
            return Color::from_u32(p | 0xFF00_0000).lerp(preset.colors.sky, t).to_u32();
        }
        p | 0xFF00_0000
    } else {
        shade(p, SHADE_BASE.powf(z / SHADE_DIST))
    }
}

fn sample_or(tex: Option<&Texture>, u: f64, v: f64, flat: Color) -> u32 {
    match tex {
        Some(t) => blend_u32(flat.to_u32(), t.sample(u, v), 255),
        None => flat.to_u32(),
    }
}

fn sky_pixel(preset: &Preset, cam: &Cam, x: i32, y: i32, time: f64) -> u32 {
    let r = cam.rect;
    let fx = (x - r.x) as f64 / r.w as f64;
    let fy = (y - r.y) as f64 / (cam.horizon - r.y as f64).max(1.0);
    if let Some(sky) = preset.textures.sky.as_ref() {
        return sky.sample(fx * 0.5 + time * 0.01, fy.clamp(0.0, 0.999)) | 0xFF00_0000;
    }
    // flat sky with a sun and two clouds, like the flat renderer
    let (px, py) = (x as f64, y as f64);
    let sun = (r.x as f64 + r.w as f64 * 0.78, r.y as f64 + r.h as f64 * 0.14, r.h as f64 * 0.06);
    if preset.sun && (px - sun.0).powi(2) + (py - sun.1).powi(2) < sun.2 * sun.2 {
        return preset.colors.sun.to_u32();
    }
    let drift = (time * 6.0) % r.w as f64;
    let clouds: &[(f64, f64, f64, f64)] = if preset.clouds { &[(0.2, 0.12, 0.09, 0.03), (0.5, 0.2, 0.07, 0.025)] } else { &[] };
    for &(cx, cy, rx, ry) in clouds {
        let ccx = r.x as f64 + (cx * r.w as f64 + drift) % r.w as f64;
        let dx = (px - ccx) / (rx * r.w as f64);
        let dy = (py - (r.y as f64 + cy * r.h as f64)) / (ry * r.h as f64);
        if dx * dx + dy * dy < 1.0 {
            return preset.colors.clouds.to_u32();
        }
    }
    preset.colors.sky.to_u32()
}

fn draw_corridor(fb: &mut FrameBuffer, cam: &Cam, area: Rect, preset: &Preset, time: f64) {
    let tex = &preset.textures;
    let colors = &preset.colors;
    let fb_w = fb.width();
    for x in area.x..area.right() {
        let sx = (x as f64 + 0.5 - cam.cx) / cam.focal;
        // wall hit for this column
        let (z, u, side) = if sx.abs() * BACK_DEPTH > HALF_WIDTH {
            let z = HALF_WIDTH / sx.abs();
            // seen from inside the corridor, u must grow to the right of the
            // screen: towards the back on the left wall, towards us on the right one
            (z, if sx > 0.0 { -z } else { z }, true)
        } else {
            (BACK_DEPTH, sx * BACK_DEPTH + HALF_WIDTH, false)
        };
        let (y0, y1) = cam.span(z, 1.0);
        for y in area.y..area.bottom() {
            let fy = y as f64 + 0.5;
            let p = if fy >= y0 && fy < y1 {
                let v = (fy - y0) / (y1 - y0);
                let p = sample_or(tex.wall.as_ref(), u.rem_euclid(1.0), v, colors.wall);
                let p = if side { shade(p, SIDE_SHADE) } else { p };
                apply_distance(p, z, preset)
            } else if fy >= y1 {
                let dist = 0.5 * cam.focal / (fy - cam.horizon);
                let (wx, wz) = (sx * dist + HALF_WIDTH, dist);
                apply_distance(sample_or(tex.floor.as_ref(), wx.fract(), wz.fract(), colors.floor), dist, preset)
            } else if preset.outside {
                sky_pixel(preset, cam, x, y, time)
            } else {
                let dist = 0.5 * cam.focal / (cam.horizon - fy);
                let (wx, wz) = (sx * dist + HALF_WIDTH, dist);
                apply_distance(sample_or(tex.ceiling.as_ref(), wx.fract(), wz.fract(), colors.ceiling), dist, preset)
            };
            fb.pixels_mut()[y as usize * fb_w + x as usize] = p | 0xFF00_0000;
        }
    }
}

/// Billboard standing on the floor at (wx, z), `size` tiles high and wide.
fn billboard(cam: &Cam, wx: f64, z: f64, size: f64, lift: f64) -> Rect {
    let (top, bottom) = cam.span(z, size + lift);
    let h = size * cam.focal / z;
    let cx = cam.screen_x(wx, z);
    Rect::new((cx - h / 2.0) as i32, top as i32, h as i32, (bottom - top - lift * cam.focal / z) as i32)
}

fn draw_end(fb: &mut FrameBuffer, cam: &Cam, preset: &Preset) {
    let r = billboard(cam, END.0, END.1, END.2, 0.0);
    match preset.textures.end.as_ref() {
        Some(t) => fb.blit_texture(t, r),
        None => {
            // flat fallback: pole + flag
            let pole = Rect::new(r.x + r.w / 4, r.y, (r.w / 10).max(1), r.h);
            fb.fill_rect(pole, Color::rgb(90, 90, 90));
            fb.fill_rect(Rect::new(pole.right(), r.y, r.w / 2, r.h / 3), preset.colors.end);
        }
    }
}

fn draw_grid(fb: &mut FrameBuffer, cam: &Cam, preset: &Preset) {
    let x0 = cam.screen_x(GRID_X.0, GRID_DEPTH) as i32;
    let x1 = cam.screen_x(GRID_X.1, GRID_DEPTH) as i32;
    let (top, bottom) = cam.span(GRID_DEPTH, 1.0);
    let r = Rect::new(x0, top as i32, x1 - x0, (bottom - top) as i32);
    match preset.textures.grid.as_ref() {
        // one block face = one texture, as in game
        Some(t) => fb.blit_texture(t, r),
        None => {
            let c = preset.colors.grid;
            let step = (r.h / 6).max(2);
            let mut x = r.x;
            while x < r.right() {
                fb.fill_rect(Rect::new(x, r.y, 1, r.h), c);
                x += step;
            }
            let mut y = r.y;
            while y < r.bottom() {
                fb.fill_rect(Rect::new(r.x, y, r.w, 1), c);
                y += step;
            }
        }
    }
}

fn draw_coin(fb: &mut FrameBuffer, cam: &Cam, preset: &Preset, time: f64) {
    let lift = 0.15 + 0.05 * (time * 3.0).sin();
    let r = billboard(cam, COIN.0, COIN.1, COIN.2, lift);
    match preset.textures.coin.as_ref() {
        Some(t) => fb.blit_texture(t, r),
        None => {
            let (cx, cy) = r.center();
            fb.fill_circle(cx, cy, r.w.min(r.h) / 2, preset.colors.coin);
        }
    }
}
