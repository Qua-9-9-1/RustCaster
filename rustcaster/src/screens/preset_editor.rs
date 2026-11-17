// Preset editor (stub). The custom preset task replaces this screen: import
// user textures into `Preset::textures` slots, edit colours/flags, then
// `ctx.presets.save(&preset)` and `ctx.select_preset(..)`. See ARCHITECTURE.md.

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::assets::preset::TextureSlot;
use crate::input::Input;
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::{draw_text, draw_text_centered};
use crate::ui::widgets::{text_scale, Button, TEXT_COLOR, TEXT_DIM};

const BACKGROUND: Color = Color::rgb(20, 22, 32);

pub struct PresetEditorScreen;

impl PresetEditorScreen {
    pub fn new() -> PresetEditorScreen {
        PresetEditorScreen
    }

    fn back_button(&self, fb_size: (usize, usize)) -> Button {
        let scale = text_scale(fb_size.1);
        let w = 60 * scale;
        let h = 12 * scale;
        Button::new("Back", Rect::new((fb_size.0 as i32 - w) / 2, fb_size.1 as i32 - h - 10 * scale, w, h), scale)
    }

    pub fn update(&mut self, _ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        if input.was_pressed(Scancode::Escape) || self.back_button(fb_size).clicked(input) {
            return Transition::To(AppState::Menu);
        }
        Transition::None
    }

    /// Shows the current preset's texture slots as a read-only preview.
    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let scale = text_scale(fb.height());
        let w = fb.width() as i32;
        fb.clear(BACKGROUND);
        draw_text_centered(fb, "Preset editor", Rect::new(0, 6 * scale, w, 16 * scale), scale * 2, TEXT_COLOR);
        draw_text_centered(fb, &format!("Preset: {} (not editable yet)", ctx.preset.name), Rect::new(0, 26 * scale, w, 10 * scale), scale, TEXT_DIM);

        let cell = 40 * scale;
        let n = TextureSlot::ALL.len() as i32;
        let x0 = (w - n * cell) / 2;
        for (i, slot) in TextureSlot::ALL.iter().enumerate() {
            let x = x0 + i as i32 * cell;
            let y = 50 * scale;
            let r = Rect::new(x + 4 * scale, y, cell - 8 * scale, cell - 8 * scale);
            fb.draw_rect(r, TEXT_DIM);
            match ctx.preset.textures.get(*slot) {
                Some(t) => fb.blit_texture(t, r.inset(1)),
                None => draw_text_centered(fb, "-", r, scale, TEXT_DIM),
            }
            draw_text(fb, slot.label(), r.x, r.bottom() + 2 * scale, 1.max(scale / 2), TEXT_COLOR);
        }
        self.back_button((fb.width(), fb.height())).draw(fb, input);
    }
}
