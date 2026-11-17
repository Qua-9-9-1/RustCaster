// In-game HUD drawn over the 3D view:
//
//   [minimap]                         [coin 3/7] [01:23] [map] [||]
//
//   map name                 (controls hint for the first seconds)
//
// The two buttons on the right are clickable: minimap toggle and pause
// (`update_hud`, called by `App` while playing).

use crate::assets::preset::Preset;
use crate::config::MINIMAP_MARGIN;
use crate::game::GameState;
use crate::input::Input;
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::settings::Settings;
use crate::ui::font::{draw_text, text_size};
use crate::ui::preview::draw_coin_icon;
use crate::ui::theme::{self, metrics, Metrics};
use crate::ui::widgets::{Button, Icon};

/// Seconds the controls hint stays on screen (then fades out over one second).
const HINT_SECONDS: f64 = 5.0;

struct HudLayout {
    pause: Button,
    minimap: Button,
    timer: Rect,
    coins: Rect,
}

fn layout(fb_size: (usize, usize), settings: &Settings, total_coins: usize, m: &Metrics) -> HudLayout {
    let w = fb_size.0 as i32;
    let y = MINIMAP_MARGIN;
    let h = m.button_h;
    let pause = Button::new("", Rect::new(w - m.margin - h, y, h, h), m.s).icon(Icon::Pause);
    let minimap = Button::new("", Rect::new(pause.rect.x - m.gap - h, y, h, h), m.s).icon(Icon::Map).selected(settings.show_minimap);
    let timer_w = text_size("00:00", m.s).0 + 2 * m.pad;
    let timer = Rect::new(minimap.rect.x - m.gap - timer_w, y, timer_w, h);
    let widest = format!("{}/{}", total_coins, total_coins);
    let coins_w = m.pad + (h - 2 * m.s) + m.gap + text_size(&widest, m.s).0 + m.pad;
    let coins = Rect::new(timer.x - m.gap - coins_w, y, coins_w, h);
    HudLayout { pause, minimap, timer, coins }
}

/// Handles the HUD buttons. Returns true when the pause button was clicked.
pub fn update_hud(input: &Input, fb_size: (usize, usize), settings: &mut Settings, total_coins: usize) -> bool {
    let m = metrics(fb_size.1);
    let l = layout(fb_size, settings, total_coins, &m);
    if l.minimap.clicked(input) {
        settings.show_minimap = !settings.show_minimap;
    }
    l.pause.clicked(input)
}

/// "mm:ss" (minutes keep growing past 59).
pub fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Small translucent pill behind HUD counters.
fn draw_pill(fb: &mut FrameBuffer, r: Rect) {
    fb.fill_rect(Rect::new(r.x + 2, r.y + 2, r.w, r.h), Color::rgba(0, 0, 0, 70));
    fb.fill_rect(r, Color::rgba(14, 16, 28, 190));
    fb.draw_rect(r, theme::PANEL_BORDER);
}

pub fn draw_hud(fb: &mut FrameBuffer, game: &GameState, settings: &Settings, preset: &Preset, input: &Input) {
    let m = metrics(fb.height());
    let l = layout((fb.width(), fb.height()), settings, game.map.total_coins, &m);

    // coins
    draw_pill(fb, l.coins);
    let icon = l.coins.h - 2 * m.s;
    draw_coin_icon(fb, preset, Rect::new(l.coins.x + m.pad, l.coins.y + m.s, icon, icon));
    let text = format!("{}/{}", game.coins_collected, game.map.total_coins);
    let all = game.map.total_coins > 0 && game.coins_collected == game.map.total_coins;
    let tx = l.coins.x + m.pad + icon + m.gap;
    let ty = l.coins.y + (l.coins.h - m.line()) / 2;
    theme::draw_text_soft(fb, &text, tx, ty, m.s, if all { theme::ACCENT } else { theme::TEXT });

    // timer
    draw_pill(fb, l.timer);
    theme::draw_text_soft_centered(fb, &format_time(game.time), l.timer, m.s, theme::TEXT);

    l.minimap.draw(fb, input);
    l.pause.draw(fb, input);

    // map name, bottom-left
    let name = game.map_path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let h = fb.height() as i32;
    theme::draw_text_soft(fb, &name, m.margin, h - m.margin - m.small_line(), m.small, theme::TEXT_DIM);

    // controls hint, fading out
    if game.time < HINT_SECONDS + 1.0 {
        let fade = (HINT_SECONDS + 1.0 - game.time).clamp(0.0, 1.0);
        let hint = "W/S or arrows: move   A/D: turn   Esc: pause";
        let scale = theme::fit_scale(hint, fb.width() as i32 - 2 * m.margin, m.s);
        let (tw, th) = text_size(hint, scale);
        let x = (fb.width() as i32 - tw) / 2;
        let y = h - m.margin * 3 - th;
        let a = |base: u8| (base as f64 * fade) as u8;
        fb.fill_rect(Rect::new(x - m.pad, y - m.s * 3, tw + 2 * m.pad, th + m.s * 6), Color::rgba(0, 0, 0, a(120)));
        draw_text(fb, hint, x, y, scale, theme::TEXT.with_alpha(a(255)));
    }
}
