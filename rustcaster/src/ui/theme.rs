// UI theme: palette, spacing and the drawing primitives every screen shares
// (backgrounds, panels, titles). Buttons use it through `widgets::Button`.
//
// Sizes are expressed in "scale units" (`Metrics::s`, 2 at the 480 px
// framebuffer height) so layouts shrink with smaller windows.

use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::{draw_text, draw_text_shadow, text_size};
use crate::ui::widgets::text_scale;

// --- palette ---------------------------------------------------------------

pub const BG_TOP: Color = Color::rgb(14, 16, 30);
pub const BG_BOTTOM: Color = Color::rgb(40, 22, 48);
pub const PANEL_BG: Color = Color::rgba(16, 18, 32, 228);
pub const PANEL_BORDER: Color = Color::rgb(74, 82, 120);
pub const PANEL_HIGHLIGHT: Color = Color::rgba(255, 255, 255, 22);
pub const SHADOW: Color = Color::rgba(0, 0, 0, 120);
/// Darkens the 3D view behind overlays (pause, level complete).
pub const OVERLAY_DIM: Color = Color::rgba(6, 6, 14, 150);

/// Coin gold: titles, focus, primary buttons.
pub const ACCENT: Color = Color::rgb(255, 184, 56);
pub const ACCENT_LIGHT: Color = Color::rgb(255, 226, 140);
pub const ACCENT_DARK: Color = Color::rgb(176, 98, 22);
pub const SELECTION_BG: Color = Color::rgba(255, 184, 56, 44);

pub const TEXT: Color = Color::rgb(236, 236, 244);
pub const TEXT_DIM: Color = Color::rgb(146, 150, 176);
pub const TEXT_ERROR: Color = Color::rgb(255, 112, 112);
pub const TEXT_SUCCESS: Color = Color::rgb(130, 230, 140);
pub const TEXT_ON_ACCENT: Color = Color::rgb(44, 26, 6);
pub const TEXT_SHADOW: Color = Color::rgba(0, 0, 0, 160);

pub const BUTTON_BG: Color = Color::rgb(50, 56, 86);
pub const BUTTON_HOVER: Color = Color::rgb(72, 82, 126);
pub const BUTTON_PRESSED: Color = Color::rgb(36, 40, 64);
pub const BUTTON_DISABLED: Color = Color::rgb(36, 38, 50);
pub const BUTTON_BORDER: Color = Color::rgb(98, 108, 152);

// --- spacing -----------------------------------------------------------------

/// Spacing constants for a framebuffer height, all in pixels.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    /// Text scale (glyphs are `8 * s` px).
    pub s: i32,
    /// Inner padding of panels.
    pub pad: i32,
    /// Space between widgets.
    pub gap: i32,
    pub button_h: i32,
    /// Distance kept from the screen edges.
    pub margin: i32,
    /// Scale of secondary text (map info, hints): half size when possible.
    pub small: i32,
}

pub fn metrics(fb_height: usize) -> Metrics {
    let s = text_scale(fb_height);
    Metrics { s, pad: 6 * s, gap: 4 * s, button_h: 14 * s, margin: 8 * s, small: (s / 2).max(1) }
}

impl Metrics {
    /// Height of a line of text at scale `s`.
    pub fn line(&self) -> i32 {
        8 * self.s
    }

    /// Height of a line of small text.
    pub fn small_line(&self) -> i32 {
        8 * self.small
    }
}

// --- primitives --------------------------------------------------------------

/// Menu background: vertical gradient, a few stars and a perspective floor
/// grid scrolling towards the viewer (`phase` in "grid lines", grows over time).
pub fn draw_background(fb: &mut FrameBuffer, phase: f64) {
    let (w, h) = (fb.width() as i32, fb.height() as i32);
    for y in 0..h {
        let t = y as f64 / (h - 1).max(1) as f64;
        fb.fill_rect(Rect::new(0, y, w, 1), BG_TOP.lerp(BG_BOTTOM, t));
    }

    let horizon = h * 62 / 100;
    // stars (deterministic hash, slow twinkle)
    for i in 0..90u32 {
        let hash = i.wrapping_mul(2_654_435_761) ^ 0x9E37_79B9;
        let x = (hash % w.max(1) as u32) as i32;
        let y = ((hash >> 11) % horizon.max(1) as u32) as i32;
        let twinkle = ((phase * 0.7 + i as f64 * 1.7).sin() * 0.5 + 0.5) * 120.0 + 40.0;
        fb.put_pixel(x, y, Color::rgba(220, 220, 255, twinkle as u8));
    }

    // horizon glow
    for i in 0..14 {
        let a = (56 - i * 4) as u8;
        fb.fill_rect(Rect::new(0, horizon - i, w, 1), ACCENT_DARK.with_alpha(a));
    }
    fb.fill_rect(Rect::new(0, horizon, w, h - horizon), Color::rgba(0, 0, 0, 60));

    // floor grid: horizontal lines at depth z (screen y = horizon + k / z)
    let depth = (h - horizon) as f64;
    let scroll = phase.fract();
    for k in 0..24 {
        let z = (k as f64 + 1.0 - scroll) * 0.45 + 0.15;
        let y = horizon + (depth * 0.35 / z) as i32;
        if y >= h {
            continue;
        }
        let a = (90.0 * (y - horizon) as f64 / depth).clamp(6.0, 90.0) as u8;
        fb.fill_rect(Rect::new(0, y, w, 1), ACCENT.with_alpha(a));
    }
    // converging lines
    let cx = w / 2;
    let spacing = (w / 10).max(8);
    for i in -14..=14 {
        let x_bottom = cx + i * spacing * 3;
        let x_top = cx + i * spacing / 6;
        fb.draw_line(x_top, horizon + 1, x_bottom, h + 2 * depth as i32, ACCENT.with_alpha(36));
    }
}

/// Raised panel: drop shadow, translucent fill, border and a top highlight.
pub fn draw_panel(fb: &mut FrameBuffer, rect: Rect) {
    let off = (rect.h / 60).clamp(2, 6);
    fb.fill_rect(Rect::new(rect.x + off, rect.y + off, rect.w, rect.h), SHADOW);
    fb.fill_rect(rect, PANEL_BG);
    fb.draw_rect(rect, PANEL_BORDER);
    fb.fill_rect(Rect::new(rect.x + 1, rect.y + 1, rect.w - 2, 1), PANEL_HIGHLIGHT);
}

/// Panel with a heading line (accent text + separator). Returns the content
/// area below the heading, inside the padding. `right` is an optional dim text
/// aligned on the right of the heading (counter, hint).
pub fn draw_titled_panel(fb: &mut FrameBuffer, rect: Rect, title: &str, right: Option<&str>, m: &Metrics) -> Rect {
    draw_panel(fb, rect);
    let x = rect.x + m.pad;
    let y = rect.y + m.pad;
    draw_text(fb, title, x + m.small, y + m.small, m.s, TEXT_SHADOW);
    draw_text(fb, title, x, y, m.s, ACCENT);
    if let Some(r) = right {
        let (tw, th) = text_size(r, m.small);
        draw_text(fb, r, rect.right() - m.pad - tw, y + (m.line() - th) / 2, m.small, TEXT_DIM);
    }
    let sep_y = y + m.line() + 3 * m.s;
    fb.fill_rect(Rect::new(x, sep_y, rect.w - 2 * m.pad, 1), PANEL_BORDER);
    titled_panel_content(rect, m)
}

/// Content area `draw_titled_panel` returns, without drawing (for layouts).
pub fn titled_panel_content(rect: Rect, m: &Metrics) -> Rect {
    let top = rect.y + m.pad + m.line() + 3 * m.s + 1 + m.gap;
    Rect::new(rect.x + m.pad, top, rect.w - 2 * m.pad, rect.bottom() - m.pad - top)
}

/// Big game title centred in `area`: drop shadow, extruded dark gold copy and
/// a two-tone face (light top half, gold bottom half).
pub fn draw_title(fb: &mut FrameBuffer, text: &str, area: Rect, scale: i32) {
    let scale = scale.max(1);
    let (tw, th) = text_size(text, scale);
    let x = area.x + (area.w - tw) / 2;
    let y = area.y + (area.h - th) / 2;
    let d = (scale / 3).max(1);
    draw_text(fb, text, x + 2 * d, y + 3 * d, scale, Color::rgba(0, 0, 0, 150));
    for i in 1..=d {
        draw_text(fb, text, x, y + i, scale, ACCENT_DARK.scale(0.7));
    }
    let old = fb.clip();
    let split = y + 4 * scale;
    fb.set_clip(Some(old.intersect(&Rect::new(x - 1, y - 1, tw + 2, split - y + 1))));
    draw_text(fb, text, x, y, scale, ACCENT_LIGHT);
    fb.set_clip(Some(old.intersect(&Rect::new(x - 1, split, tw + 2, th))));
    draw_text(fb, text, x, y, scale, ACCENT);
    fb.set_clip(Some(old));
}

/// Largest scale <= `max_scale` at which `text` fits in `max_w` pixels (at least 1).
pub fn fit_scale(text: &str, max_w: i32, max_scale: i32) -> i32 {
    let mut s = max_scale.max(1);
    while s > 1 && text_size(text, s).0 > max_w {
        s -= 1;
    }
    s
}

/// `text` cut with "..." so it fits in `max_w` pixels at `scale`.
pub fn ellipsize(text: &str, max_w: i32, scale: i32) -> String {
    let max_chars = (max_w / (8 * scale.max(1))).max(0) as usize;
    let n = text.chars().count();
    if n <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return text.chars().take(max_chars).collect();
    }
    let mut out: String = text.chars().take(max_chars - 3).collect();
    out.push_str("...");
    out
}

/// Text with the theme shadow (see `font::draw_text_shadow`).
pub fn draw_text_soft(fb: &mut FrameBuffer, text: &str, x: i32, y: i32, scale: i32, color: Color) {
    draw_text_shadow(fb, text, x, y, scale, color);
}

/// `draw_text_soft` centred in `area`.
pub fn draw_text_soft_centered(fb: &mut FrameBuffer, text: &str, area: Rect, scale: i32, color: Color) {
    let (tw, th) = text_size(text, scale);
    draw_text_soft(fb, text, area.x + (area.w - tw) / 2, area.y + (area.h - th) / 2, scale, color);
}

/// Colour of a status message (`ctx.message`): errors in red, results in green.
pub fn message_color(msg: &str) -> Color {
    if msg.starts_with("Error") {
        TEXT_ERROR
    } else {
        TEXT_SUCCESS
    }
}

/// One-line message box: picks the largest scale <= `m.s` that fits, else
/// ellipsizes at the smallest one.
pub fn draw_message(fb: &mut FrameBuffer, msg: &str, area: Rect, m: &Metrics) {
    let inner = area.w - 2 * m.pad;
    let scale = fit_scale(msg, inner, m.s);
    let text = ellipsize(msg, inner, scale);
    draw_text_soft_centered(fb, &text, area, scale, message_color(msg));
}
