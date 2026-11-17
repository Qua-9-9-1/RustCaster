// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// Minimal retained widgets drawn into the framebuffer.
//
// Pattern used by every screen: rebuild the widgets from a layout function
// each frame (cheap), query them in `update` (`clicked`, or `Focus::update`
// for keyboard navigation) and draw them in `draw`. Coordinates are
// framebuffer pixels, like `Input::mouse_pos`. Colours come from `ui::theme`.

use sdl2::keyboard::Scancode;

use crate::input::{Input, Mouse};
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::{draw_text, text_size};
use crate::ui::theme::{self, ellipsize, fit_scale};

// Kept for screens written against the first UI version.
pub const TEXT_COLOR: Color = theme::TEXT;
pub const TEXT_DIM: Color = theme::TEXT_DIM;
pub const TEXT_ERROR: Color = theme::TEXT_ERROR;
pub const PANEL_BG: Color = theme::PANEL_BG;
pub const BUTTON_BG: Color = theme::BUTTON_BG;
pub const BUTTON_HOVER: Color = theme::BUTTON_HOVER;
pub const BUTTON_DISABLED: Color = theme::BUTTON_DISABLED;
pub const BUTTON_BORDER: Color = theme::BUTTON_BORDER;

/// Text scale suited to the framebuffer height (2 at 480px: 16px glyphs).
pub fn text_scale(fb_height: usize) -> i32 {
    ((fb_height as i32) / 240).max(1)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonStyle {
    Normal,
    /// Gold call-to-action (Play, Start, Resume...).
    Primary,
}

/// Small pictograms drawn instead of / next to the label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Pause,
    Map,
}

#[derive(Clone, Debug)]
pub struct Button {
    pub rect: Rect,
    pub label: String,
    pub enabled: bool,
    pub scale: i32,
    pub style: ButtonStyle,
    /// Keyboard focus: drawn like a hovered button.
    pub focused: bool,
    /// Toggle "on" / chosen item: gold border.
    pub selected: bool,
    pub icon: Option<Icon>,
}

impl Button {
    pub fn new(label: impl Into<String>, rect: Rect, scale: i32) -> Button {
        Button {
            rect,
            label: label.into(),
            enabled: true,
            scale,
            style: ButtonStyle::Normal,
            focused: false,
            selected: false,
            icon: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Button {
        self.enabled = enabled;
        self
    }

    pub fn primary(mut self) -> Button {
        self.style = ButtonStyle::Primary;
        self
    }

    pub fn focused(mut self, focused: bool) -> Button {
        self.focused = focused;
        self
    }

    pub fn selected(mut self, selected: bool) -> Button {
        self.selected = selected;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Button {
        self.icon = Some(icon);
        self
    }

    pub fn hovered(&self, input: &Input) -> bool {
        let (x, y) = input.mouse_pos();
        self.enabled && self.rect.contains(x, y)
    }

    /// Left button went down over the button this frame.
    pub fn clicked(&self, input: &Input) -> bool {
        self.hovered(input) && input.mouse_clicked(Mouse::Left)
    }

    /// Normal / hover (or focus) / pressed / disabled faces, with a bottom lip
    /// that disappears when pressed so the button looks pushed in.
    pub fn draw(&self, fb: &mut FrameBuffer, input: &Input) {
        let s = self.scale.max(1);
        let r = self.rect;
        let over = self.hovered(input);
        let hot = self.enabled && (over || self.focused);
        let pressed = over && input.mouse_down(Mouse::Left);
        let primary = self.style == ButtonStyle::Primary;

        let mut base = match (self.enabled, primary) {
            (false, _) => theme::BUTTON_DISABLED,
            (true, true) if pressed => theme::ACCENT_DARK,
            (true, true) if hot => theme::ACCENT_LIGHT,
            (true, true) => theme::ACCENT,
            (true, false) if pressed => theme::BUTTON_PRESSED,
            (true, false) if hot => theme::BUTTON_HOVER,
            (true, false) => theme::BUTTON_BG,
        };
        if self.selected && self.enabled && !primary {
            base = base.lerp(theme::ACCENT_DARK, 0.35);
        }

        let depth = s.min(r.h / 4).max(1);
        let face = if !self.enabled {
            r
        } else if pressed {
            fb.fill_rect(Rect::new(r.x, r.y, r.w, depth), base.scale(0.5));
            Rect::new(r.x, r.y + depth, r.w, r.h - depth)
        } else {
            fb.fill_rect(Rect::new(r.x, r.bottom() - depth, r.w, depth), base.scale(0.55));
            Rect::new(r.x, r.y, r.w, r.h - depth)
        };
        fb.fill_rect(face, base);
        if self.enabled {
            fb.fill_rect(Rect::new(face.x + 1, face.y + 1, face.w - 2, 1), Color::rgba(255, 255, 255, 50));
        }
        let border = if !self.enabled {
            theme::BUTTON_BORDER.scale(0.5)
        } else if hot && primary {
            theme::TEXT
        } else if hot || self.selected {
            theme::ACCENT
        } else if primary {
            theme::ACCENT_DARK
        } else {
            theme::BUTTON_BORDER
        };
        fb.draw_rect(r, border);

        let color = if !self.enabled {
            theme::TEXT_DIM
        } else if primary {
            theme::TEXT_ON_ACCENT
        } else {
            theme::TEXT
        };
        if let Some(icon) = self.icon {
            draw_icon(fb, icon, face, color);
        }
        if !self.label.is_empty() {
            let avail = face.w - 4 * s;
            let ts = fit_scale(&self.label, avail, self.scale);
            let text = ellipsize(&self.label, avail, ts);
            let (tw, th) = text_size(&text, ts);
            let (tx, ty) = (face.x + (face.w - tw) / 2, face.y + (face.h - th) / 2);
            if self.enabled && !primary {
                let d = (ts / 2).max(1);
                draw_text(fb, &text, tx + d, ty + d, ts, theme::TEXT_SHADOW);
            }
            draw_text(fb, &text, tx, ty, ts, color);
        }
    }
}

/// Draws `icon` centred in `area` (about half its height).
pub fn draw_icon(fb: &mut FrameBuffer, icon: Icon, area: Rect, color: Color) {
    let size = (area.h / 2).max(4);
    let (cx, cy) = area.center();
    let x0 = cx - size / 2;
    let y0 = cy - size / 2;
    match icon {
        Icon::Pause => {
            let bar = (size / 3).max(1);
            fb.fill_rect(Rect::new(x0, y0, bar, size), color);
            fb.fill_rect(Rect::new(x0 + size - bar, y0, bar, size), color);
        }
        Icon::Map => {
            // folded map: three panels with a darker middle one
            let third = (size / 3).max(1);
            fb.fill_rect(Rect::new(x0, y0, third, size), color);
            fb.fill_rect(Rect::new(x0 + third, y0 + 1, third, size - 1), color.scale(0.6));
            fb.fill_rect(Rect::new(x0 + 2 * third, y0, size - 2 * third, size), color);
        }
    }
}

/// Keyboard / pointer focus over an ordered list of buttons.
///
/// Up/Left and Down/Right/Tab move the focus (skipping disabled buttons),
/// Enter/Space activates the focused button, moving the mouse over a button
/// focuses it. Build the buttons with `.focused(focus.is(i))`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Focus {
    pub index: Option<usize>,
}

impl Focus {
    pub fn new(index: Option<usize>) -> Focus {
        Focus { index }
    }

    pub fn is(&self, i: usize) -> bool {
        self.index == Some(i)
    }

    /// Returns the index of the button activated this frame (click or Enter).
    pub fn update(&mut self, input: &Input, buttons: &[&Button]) -> Option<usize> {
        let n = buttons.len();
        if n == 0 {
            self.index = None;
            return None;
        }
        if input.mouse_moved() {
            if let Some(i) = buttons.iter().position(|b| b.hovered(input)) {
                self.index = Some(i);
            }
        }
        let back = input.was_pressed(Scancode::Up) || input.was_pressed(Scancode::Left);
        let fwd = input.was_pressed(Scancode::Down) || input.was_pressed(Scancode::Right) || input.was_pressed(Scancode::Tab);
        let step: i32 = if fwd { 1 } else if back { -1 } else { 0 };
        if step != 0 {
            let mut i = match self.index {
                Some(i) => i as i32,
                None if step > 0 => -1,
                None => n as i32,
            };
            for _ in 0..n {
                i = (i + step).rem_euclid(n as i32);
                if buttons[i as usize].enabled {
                    break;
                }
            }
            self.index = Some(i as usize);
        }
        if self.index.map_or(false, |i| i >= n) {
            self.index = None;
        }
        if let Some(i) = buttons.iter().position(|b| b.clicked(input)) {
            self.index = Some(i);
            return Some(i);
        }
        match self.index {
            Some(i) if input.confirm_pressed() && buttons[i].enabled => Some(i),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Label {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub scale: i32,
    pub color: Color,
}

impl Label {
    pub fn new(text: impl Into<String>, x: i32, y: i32, scale: i32, color: Color) -> Label {
        Label { text: text.into(), x, y, scale, color }
    }

    /// Label horizontally centred in `area`, at row `y`.
    pub fn centered(text: impl Into<String>, area: Rect, y: i32, scale: i32, color: Color) -> Label {
        let text = text.into();
        let (w, _) = text_size(&text, scale);
        Label { x: area.x + (area.w - w) / 2, y, text, scale, color }
    }

    pub fn draw(&self, fb: &mut FrameBuffer) {
        theme::draw_text_soft(fb, &self.text, self.x, self.y, self.scale, self.color);
    }
}

/// Themed panel (shadow, translucent fill, border). See `theme::draw_panel`.
pub fn draw_panel(fb: &mut FrameBuffer, rect: Rect) {
    theme::draw_panel(fb, rect);
}

/// Rect of size `w` x `h` centred in `outer`.
pub fn centered(outer: Rect, w: i32, h: i32) -> Rect {
    Rect::new(outer.x + (outer.w - w) / 2, outer.y + (outer.h - h) / 2, w, h)
}

/// `count` rows of height `item_h` stacked from the top of `area`, `gap` apart,
/// each as wide as `area`.
pub fn column(area: Rect, item_h: i32, gap: i32, count: usize) -> Vec<Rect> {
    (0..count as i32).map(|i| Rect::new(area.x, area.y + i * (item_h + gap), area.w, item_h)).collect()
}

/// Splits `area` horizontally into `count` equal cells separated by `gap`.
pub fn row(area: Rect, gap: i32, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let n = count as i32;
    let w = (area.w - gap * (n - 1)) / n;
    (0..n).map(|i| Rect::new(area.x + i * (w + gap), area.y, w, area.h)).collect()
}

/// Vertical scrollbar of a list showing `shown` of `total` items. Returns the
/// first item to show while the user presses / drags on the track.
pub fn scrollbar(input: &Input, track: Rect, shown: usize, total: usize) -> Option<usize> {
    if total <= shown || shown == 0 {
        return None;
    }
    let (mx, my) = input.mouse_pos();
    let grab = Rect::new(track.x - track.w, track.y, track.w * 3, track.h);
    if input.mouse_down(Mouse::Left) && grab.contains(mx, my) {
        let max_first = total - shown;
        let t = ((my - track.y) as f64 / track.h.max(1) as f64).clamp(0.0, 1.0);
        return Some(((t * max_first as f64).round() as usize).min(max_first));
    }
    None
}

/// Track + thumb for a list showing `shown` of `total` items from `first`.
pub fn draw_scrollbar(fb: &mut FrameBuffer, track: Rect, first: usize, shown: usize, total: usize) {
    if total <= shown || total == 0 {
        return;
    }
    fb.fill_rect(track, Color::rgba(0, 0, 0, 90));
    let thumb_h = (track.h * shown as i32 / total as i32).max(track.w * 2).min(track.h);
    let max_first = (total - shown) as i32;
    let y = track.y + (track.h - thumb_h) * first as i32 / max_first.max(1);
    fb.fill_rect(Rect::new(track.x, y, track.w, thumb_h), theme::ACCENT_DARK);
    fb.fill_rect(Rect::new(track.x, y, track.w, 1), theme::ACCENT);
}
