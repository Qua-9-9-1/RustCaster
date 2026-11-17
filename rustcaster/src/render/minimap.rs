// Minimap overlay in the top-left corner of the view, centred on the player,
// with the rays of the last raycast, drawn as a framed HUD panel.

use crate::assets::preset::Preset;
use crate::config::{MINIMAP_MARGIN, MINIMAP_SIZE_RATIO, MINIMAP_TILES_ACROSS, PLAYER_RADIUS};
use crate::game::map::{Map, Tile};
use crate::game::player::Player;
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::theme;

const RAY_COLOR: Color = Color::rgba(120, 230, 255, 70);
const PLAYER_COLOR: Color = theme::ACCENT;
/// About this many rays are drawn, whatever the framebuffer width.
const RAYS_SHOWN: usize = 80;

/// Screen rectangle of the minimap for a framebuffer of this height.
pub fn minimap_rect(fb_height: usize) -> Rect {
    let size = (fb_height as f64 * MINIMAP_SIZE_RATIO) as i32;
    Rect::new(MINIMAP_MARGIN, MINIMAP_MARGIN, size, size)
}

/// `ray_ends`: world points where each column's ray stopped (`Raycaster::ray_ends`).
pub fn draw_minimap(fb: &mut FrameBuffer, map: &Map, player: &Player, preset: &Preset, ray_ends: &[(f64, f64)]) {
    let area = minimap_rect(fb.height());
    let tile = area.w as f64 / MINIMAP_TILES_ACROSS;
    let (cx, cy) = area.center();
    let to_screen = |wx: f64, wy: f64| ((cx as f64 + (wx - player.x) * tile).floor() as i32, (cy as f64 + (wy - player.y) * tile).floor() as i32);

    // frame: shadow + outer border, map inside
    fb.fill_rect(Rect::new(area.x + 3, area.y + 3, area.w + 2, area.h + 2), theme::SHADOW);
    fb.draw_rect(Rect::new(area.x - 2, area.y - 2, area.w + 4, area.h + 4), theme::PANEL_BORDER);
    fb.draw_rect(Rect::new(area.x - 1, area.y - 1, area.w + 2, area.h + 2), Color::rgba(0, 0, 0, 200));

    fb.set_clip(Some(area));
    fb.fill_rect(area, preset.colors.floor.scale(0.55).with_alpha(255));

    let reach = (MINIMAP_TILES_ACROSS / 2.0).ceil() as i32 + 1;
    let (px, py) = (player.x.floor() as i32, player.y.floor() as i32);
    for ty in py - reach..=py + reach {
        for tx in px - reach..=px + reach {
            let (color, inset) = match map.get(tx, ty) {
                Tile::Wall => (preset.colors.wall, 0.0),
                Tile::Grid => (preset.colors.grid, 0.15),
                Tile::Coin => (preset.colors.coin, 0.3),
                Tile::End => (preset.colors.end, 0.2),
                Tile::Empty | Tile::Monster => continue,
            };
            let (x0, y0) = to_screen(tx as f64 + inset, ty as f64 + inset);
            let (x1, y1) = to_screen(tx as f64 + 1.0 - inset, ty as f64 + 1.0 - inset);
            fb.fill_rect(Rect::new(x0, y0, x1 - x0, y1 - y0), color.with_alpha(255));
        }
    }

    let step = (ray_ends.len() / RAYS_SHOWN).max(1);
    for &(ex, ey) in ray_ends.iter().step_by(step) {
        let (x1, y1) = to_screen(ex, ey);
        fb.draw_line(cx, cy, x1, y1, RAY_COLOR);
    }
    // player: dot + heading tick
    let r = ((PLAYER_RADIUS * tile) as i32).max(2);
    let (dx, dy) = player.dir();
    let (hx, hy) = (cx + (dx * r as f64 * 2.2) as i32, cy + (dy * r as f64 * 2.2) as i32);
    fb.draw_line(cx, cy, hx, hy, PLAYER_COLOR);
    fb.fill_circle(cx, cy, r + 1, Color::rgba(0, 0, 0, 160));
    fb.fill_circle(cx, cy, r, PLAYER_COLOR);

    // vignette on the inner edge
    fb.draw_rect(area, Color::rgba(0, 0, 0, 90));
    fb.draw_rect(area.inset(1), Color::rgba(0, 0, 0, 50));
    fb.set_clip(None);
}
