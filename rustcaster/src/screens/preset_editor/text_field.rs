// Single-line text input drawn into the framebuffer. Click to focus, type
// (SDL text input), Backspace deletes, Enter / Escape / click outside unfocus.
// Kept generic so it can move to `ui/` later.

use sdl2::keyboard::Scancode;

use crate::input::{Input, Mouse};
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::{draw_text, text_size, GLYPH_W};

pub struct TextField {
    pub text: String,
    pub max_len: usize,
    pub focused: bool,
}

/// What happened to the field this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldEvent {
    None,
    Changed,
    /// Focus left with Enter.
    Submitted,
}

impl TextField {
    pub fn new(text: impl Into<String>, max_len: usize) -> TextField {
        TextField { text: text.into(), max_len, focused: false }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into().chars().take(self.max_len).collect();
    }

    /// Handles focus clicks and typing. `rect` is where the field is drawn.
    pub fn update(&mut self, input: &Input, rect: Rect) -> FieldEvent {
        if input.mouse_clicked(Mouse::Left) {
            let (mx, my) = input.mouse_pos();
            self.focused = rect.contains(mx, my);
        }
        if !self.focused {
            return FieldEvent::None;
        }
        let mut event = FieldEvent::None;
        for c in input.text().chars() {
            // the bitmap font only has printable ASCII
            if (' '..='~').contains(&c) && self.text.chars().count() < self.max_len {
                self.text.push(c);
                event = FieldEvent::Changed;
            }
        }
        if input.was_pressed(Scancode::Backspace) && self.text.pop().is_some() {
            event = FieldEvent::Changed;
        }
        if input.was_pressed(Scancode::Delete) && !self.text.is_empty() {
            self.text.clear();
            event = FieldEvent::Changed;
        }
        if input.was_pressed(Scancode::Return) || input.was_pressed(Scancode::KpEnter) {
            self.focused = false;
            event = FieldEvent::Submitted;
        } else if input.was_pressed(Scancode::Escape) {
            self.focused = false;
        }
        event
    }

    /// `time` (seconds) blinks the caret.
    pub fn draw(&self, fb: &mut FrameBuffer, rect: Rect, scale: i32, colors: FieldColors, time: f64) {
        fb.fill_rect(rect, colors.background);
        fb.draw_rect(rect, if self.focused { colors.focus } else { colors.border });
        let (_, th) = text_size("M", scale);
        let pad = 3 * scale;
        let x = rect.x + pad;
        let y = rect.y + (rect.h - th) / 2;
        // show the end of the text when it does not fit
        let max_chars = ((rect.w - 2 * pad) / (GLYPH_W * scale) - 1).max(1) as usize;
        let count = self.text.chars().count();
        let shown: String = self.text.chars().skip(count.saturating_sub(max_chars)).collect();
        if shown.is_empty() && !self.focused {
            draw_text(fb, colors.placeholder_text, x, y, scale, colors.placeholder);
        } else {
            draw_text(fb, &shown, x, y, scale, colors.text);
        }
        if self.focused && (time * 2.0).fract() < 0.5 {
            let (w, _) = text_size(&shown, scale);
            fb.fill_rect(Rect::new(x + w + 1, y, scale.max(1), th), colors.text);
        }
    }
}

#[derive(Clone, Copy)]
pub struct FieldColors {
    pub background: Color,
    pub border: Color,
    pub focus: Color,
    pub text: Color,
    pub placeholder: Color,
    pub placeholder_text: &'static str,
}
