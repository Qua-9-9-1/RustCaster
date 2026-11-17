// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// Minimal retained widgets drawn into the framebuffer.
//
// Pattern used by every screen: rebuild the widgets from a layout function
// each frame (cheap), query them in `update` (`clicked`) and draw them in
// `draw`. Coordinates are framebuffer pixels, like `Input::mouse_pos`.

use crate::input::{Input, Mouse};
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::{self, draw_text, text_size};

pub const TEXT_COLOR: Color = Color::rgb(235, 235, 235);
pub const TEXT_DIM: Color = Color::rgb(150, 150, 150);
pub const TEXT_ERROR: Color = Color::rgb(255, 110, 110);
pub const PANEL_BG: Color = Color::rgba(15, 15, 25, 210);
pub const BUTTON_BG: Color = Color::rgb(55, 60, 80);
pub const BUTTON_HOVER: Color = Color::rgb(85, 95, 135);
pub const BUTTON_DISABLED: Color = Color::rgb(40, 40, 45);
pub const BUTTON_BORDER: Color = Color::rgb(140, 150, 190);

/// Text scale suited to the framebuffer height (2 at 480px: 16px glyphs).
pub fn text_scale(fb_height: usize) -> i32 {
    ((fb_height as i32) / 240).max(1)
}

#[derive(Clone, Debug)]
pub struct Button {
    pub rect: Rect,
    pub label: String,
    pub enabled: bool,
    pub scale: i32,
}

impl Button {
    pub fn new(label: impl Into<String>, rect: Rect, scale: i32) -> Button {
        Button { rect, label: label.into(), enabled: true, scale }
    }

    pub fn enabled(mut self, enabled: bool) -> Button {
        self.enabled = enabled;
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

    pub fn draw(&self, fb: &mut FrameBuffer, input: &Input) {
        let bg = if !self.enabled {
            BUTTON_DISABLED
        } else if self.hovered(input) {
            BUTTON_HOVER
        } else {
            BUTTON_BG
        };
        fb.fill_rect(self.rect, bg);
        fb.draw_rect(self.rect, if self.enabled { BUTTON_BORDER } else { BUTTON_DISABLED });
        let color = if self.enabled { TEXT_COLOR } else { TEXT_DIM };
        font::draw_text_centered(fb, &self.label, self.rect, self.scale, color);
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
        draw_text(fb, &self.text, self.x, self.y, self.scale, self.color);
    }
}

/// Translucent background box.
pub fn draw_panel(fb: &mut FrameBuffer, rect: Rect) {
    fb.fill_rect(rect, PANEL_BG);
    fb.draw_rect(rect, BUTTON_BORDER);
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
