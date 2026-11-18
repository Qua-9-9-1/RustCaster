// Small pictures used by the menus and the HUD: preset preview "postcard",
// coin icon and map thumbnail. Textures are used when the preset has them,
// flat preset colours otherwise.

use crate::assets::preset::Preset;
use crate::assets::texture::Texture;
use crate::game::map::{Map, Tile};
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::theme;

/// Fills `area` with copies of `tex`, each `tile_w` x `tile_h` px.
fn tile_texture(fb: &mut FrameBuffer, tex: &Texture, area: Rect, tile_w: i32, tile_h: i32) {
    let (tw, th) = (tile_w.max(1), tile_h.max(1));
    let old = fb.clip();
    fb.set_clip(Some(old.intersect(&area)));
    let mut y = area.y;
    while y < area.bottom() {
        let mut x = area.x;
        while x < area.right() {
            fb.blit_texture(tex, Rect::new(x, y, tw, th));
            x += tw;
        }
        y += th;
    }
    fb.set_clip(Some(old));
}

/// Vertical gradient from `top` to `bottom` colour.
fn gradient(fb: &mut FrameBuffer, area: Rect, top: Color, bottom: Color) {
    for i in 0..area.h {
        let t = i as f64 / (area.h - 1).max(1) as f64;
        fb.fill_rect(Rect::new(area.x, area.y + i, area.w, 1), top.lerp(bottom, t));
    }
}

/// Coin pictogram: the preset's coin sprite, or a drawn gold disc.
pub fn draw_coin_icon(fb: &mut FrameBuffer, preset: &Preset, rect: Rect) {
    match &preset.textures.coin {
        Some(t) => fb.blit_texture(t, rect),
        None => {
            let (cx, cy) = rect.center();
            let r = (rect.w.min(rect.h) / 2 - 1).max(1);
            let c = preset.colors.coin;
            fb.fill_circle(cx, cy, r, c.scale(0.6));
            fb.fill_circle(cx, cy, (r - (r / 4).max(1)).max(1), c);
            fb.fill_circle(cx - r / 3, cy - r / 3, (r / 4).max(1), Color::rgba(255, 255, 255, 140));
        }
    }
}

/// Postcard of a preset: sky or ceiling, floor, a wall block and a coin.
pub fn draw_preset_preview(fb: &mut FrameBuffer, preset: &Preset, rect: Rect) {
    let old = fb.clip();
    fb.set_clip(Some(old.intersect(&rect)));
    let c = &preset.colors;
    let horizon = rect.y + rect.h * 48 / 100;
    let top = Rect::new(rect.x, rect.y, rect.w, horizon - rect.y);
    let bottom = Rect::new(rect.x, horizon, rect.w, rect.bottom() - horizon);

    if preset.outside {
        match &preset.textures.sky {
            Some(t) => fb.blit_texture(t, top),
            None => gradient(fb, top, c.sky.scale(0.8), c.sky),
        }
        let r = (rect.h / 12).max(2);
        if preset.sun && c.sun != c.sky {
            fb.fill_circle(rect.x + rect.w * 3 / 4, rect.y + top.h / 3, r, c.sun);
        }
        if preset.clouds && c.clouds != c.sky {
            let (x, y) = (rect.x + rect.w / 5, rect.y + top.h / 4);
            fb.fill_circle(x, y, r, c.clouds);
            fb.fill_circle(x + r, y - r / 3, r, c.clouds);
            fb.fill_circle(x + 2 * r, y, r * 3 / 4, c.clouds);
        }
    } else {
        match &preset.textures.ceiling {
            Some(t) => tile_texture(fb, t, top, top.h / 2, top.h / 2),
            None => fb.fill_rect(top, c.ceiling),
        }
    }
    match &preset.textures.floor {
        Some(t) => tile_texture(fb, t, bottom, bottom.h / 2, bottom.h / 2),
        None => fb.fill_rect(bottom, c.floor),
    }
    // depth: darken towards the horizon
    for i in 0..bottom.h / 2 {
        let a = (110 - 220 * i / bottom.h.max(1)).clamp(0, 255) as u8;
        fb.fill_rect(Rect::new(bottom.x, bottom.y + i, bottom.w, 1), Color::rgba(0, 0, 0, a));
    }

    // wall block standing on the horizon
    let wall_h = rect.h * 46 / 100;
    let wall_w = rect.w * 52 / 100;
    let wall = Rect::new(rect.x + (rect.w - wall_w) / 2, horizon - wall_h * 62 / 100, wall_w, wall_h);
    match &preset.textures.wall {
        Some(t) => tile_texture(fb, t, wall, wall.h / 2, wall.h / 2),
        None => fb.fill_rect(wall, c.wall),
    }
    let fog = preset.fog_amount();
    if fog > 0.0 {
        fb.fill_rect(wall, c.sky.with_alpha((70.0 * fog).round() as u8));
    }
    fb.draw_rect(wall, Color::rgba(0, 0, 0, 90));
    // floor shadow under the wall
    fb.fill_rect(Rect::new(wall.x, wall.bottom(), wall.w, (rect.h / 40).max(1)), Color::rgba(0, 0, 0, 90));

    // coin in front
    let coin = (rect.h / 5).max(6);
    draw_coin_icon(fb, preset, Rect::new(rect.x + (rect.w - coin) / 2, rect.bottom() - coin - rect.h / 14, coin, coin));

    fb.set_clip(Some(old));
    fb.draw_rect(rect, theme::PANEL_BORDER);
}

/// Colour chips of a preset (sky/ceiling, wall, floor, grid, coin, end) in a row.
pub fn draw_palette(fb: &mut FrameBuffer, preset: &Preset, rect: Rect, gap: i32) {
    let c = &preset.colors;
    let top = if preset.outside { c.sky } else { c.ceiling };
    let chips = [top, c.wall, c.floor, c.grid, c.coin, c.end];
    let n = chips.len() as i32;
    let w = (rect.w - gap * (n - 1)) / n;
    for (i, color) in chips.iter().enumerate() {
        let r = Rect::new(rect.x + i as i32 * (w + gap), rect.y, w, rect.h);
        fb.fill_rect(r, color.with_alpha(255));
        fb.draw_rect(r, theme::PANEL_BORDER);
    }
}

/// Top-down picture of a map in the preset colours, centred in `rect`.
pub fn draw_map_thumbnail(fb: &mut FrameBuffer, map: &Map, preset: &Preset, rect: Rect) {
    if map.width == 0 || map.height == 0 || rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let c = &preset.colors;
    let floor = c.floor.scale(0.55);
    let color_of = |t: Tile| match t {
        Tile::Wall => c.wall,
        Tile::Grid => c.grid.lerp(floor, 0.4),
        Tile::Coin => c.coin,
        Tile::End => c.end,
        Tile::Empty | Tile::Monster => floor,
    };
    let sx = rect.w as f64 / map.width as f64;
    let sy = rect.h as f64 / map.height as f64;
    let scale = sx.min(sy);
    let (w, h) = ((map.width as f64 * scale) as i32, (map.height as f64 * scale) as i32);
    let (x0, y0) = (rect.x + (rect.w - w) / 2, rect.y + (rect.h - h) / 2);
    if scale >= 1.0 {
        for ty in 0..map.height {
            for tx in 0..map.width {
                let px0 = x0 + (tx as f64 * scale) as i32;
                let py0 = y0 + (ty as f64 * scale) as i32;
                let px1 = x0 + ((tx + 1) as f64 * scale) as i32;
                let py1 = y0 + ((ty + 1) as f64 * scale) as i32;
                fb.fill_rect(Rect::new(px0, py0, px1 - px0, py1 - py0), color_of(map.get(tx as i32, ty as i32)));
            }
        }
    } else {
        // more tiles than pixels: sample one tile per pixel
        for py in 0..h {
            for px in 0..w {
                let tx = (px as f64 / scale) as i32;
                let ty = (py as f64 / scale) as i32;
                fb.put_pixel(x0 + px, y0 + py, color_of(map.get(tx, ty)));
            }
        }
    }
    let (sx0, sy0) = map.player_start;
    let p = (scale as i32).max(1);
    fb.fill_rect(Rect::new(x0 + (sx0 as f64 * scale) as i32, y0 + (sy0 as f64 * scale) as i32, p, p), theme::ACCENT);
    fb.draw_rect(Rect::new(x0 - 1, y0 - 1, w + 2, h + 2), Color::rgba(0, 0, 0, 120));
}
