// Main menu (minimal stub): preset selector, one button per map, preset
// editor and quit. Meant to be replaced/restyled by the menus/UI task.

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::input::Input;
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::draw_text_centered;
use crate::ui::widgets::{column, row, text_scale, Button, Label, TEXT_COLOR, TEXT_DIM, TEXT_ERROR};

const BACKGROUND: Color = Color::rgb(20, 22, 32);

pub struct MenuScreen {
    /// First map shown when the list does not fit.
    scroll: usize,
}

/// Every widget of the screen for a given framebuffer size.
struct Layout {
    title: Rect,
    preset_prev: Button,
    preset_label: Rect,
    preset_next: Button,
    /// (index in ctx.maps, button)
    maps: Vec<(usize, Button)>,
    editor: Button,
    quit: Button,
    message_y: i32,
    scale: i32,
}

impl MenuScreen {
    pub fn new() -> MenuScreen {
        MenuScreen { scroll: 0 }
    }

    fn layout(&self, ctx: &AppContext, fb_size: (usize, usize)) -> Layout {
        let (w, h) = (fb_size.0 as i32, fb_size.1 as i32);
        let scale = text_scale(fb_size.1);
        let item_h = 12 * scale;
        let gap = 3 * scale;
        let col_w = (w / 2).max(160);
        let col_x = (w - col_w) / 2;

        let title = Rect::new(0, 6 * scale, w, 16 * scale);
        let preset_row = Rect::new(col_x, title.bottom() + 6 * scale, col_w, item_h);
        let cells = row(preset_row, gap, 4);
        let preset_prev = Button::new("<", cells[0], scale);
        let preset_next = Button::new(">", cells[3], scale);
        let preset_label = Rect::new(cells[1].x, preset_row.y, cells[2].right() - cells[1].x, item_h);

        let bottom_row = Rect::new(col_x, h - item_h - 18 * scale, col_w, item_h);
        let bottom = row(bottom_row, gap, 2);
        let editor = Button::new("Preset editor", bottom[0], scale);
        let quit = Button::new("Quit", bottom[1], scale);

        let list_top = preset_row.bottom() + 8 * scale;
        let list_h = bottom_row.y - 6 * scale - list_top;
        let fit = ((list_h + gap) / (item_h + gap)).max(1) as usize;
        let first = self.scroll.min(ctx.maps.len().saturating_sub(fit));
        let shown = ctx.maps.len().saturating_sub(first).min(fit);
        let rects = column(Rect::new(col_x, list_top, col_w, list_h), item_h, gap, shown);
        let maps = rects
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                let index = first + i;
                let name = ctx.maps[index].file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                (index, Button::new(name, r, scale))
            })
            .collect();

        Layout { title, preset_prev, preset_label, preset_next, maps, editor, quit, message_y: h - 14 * scale, scale }
    }

    pub fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        if input.was_pressed(Scancode::Escape) {
            return Transition::Quit;
        }
        if input.wheel() > 0 {
            self.scroll = self.scroll.saturating_sub(1);
        } else if input.wheel() < 0 && self.scroll + 1 < ctx.maps.len() {
            self.scroll += 1;
        }
        let l = self.layout(ctx, fb_size);
        if l.preset_prev.clicked(input) {
            ctx.cycle_preset(-1);
        }
        if l.preset_next.clicked(input) {
            ctx.cycle_preset(1);
        }
        for (index, b) in &l.maps {
            if b.clicked(input) {
                return Transition::StartGame(ctx.maps[*index].clone());
            }
        }
        if l.editor.clicked(input) {
            return Transition::To(AppState::PresetEditor);
        }
        if l.quit.clicked(input) {
            return Transition::Quit;
        }
        Transition::None
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let l = self.layout(ctx, (fb.width(), fb.height()));
        fb.clear(BACKGROUND);
        draw_text_centered(fb, "RustCaster", l.title, l.scale * 2, TEXT_COLOR);
        l.preset_prev.draw(fb, input);
        l.preset_next.draw(fb, input);
        draw_text_centered(fb, &format!("Preset: {}", ctx.settings.preset), l.preset_label, l.scale, TEXT_COLOR);
        if l.maps.is_empty() {
            Label::centered("No map found in maps/", fb.bounds(), fb.height() as i32 / 2, l.scale, TEXT_DIM).draw(fb);
        }
        for (_, b) in &l.maps {
            b.draw(fb, input);
        }
        l.editor.draw(fb, input);
        l.quit.draw(fb, input);
        if let Some(msg) = &ctx.message {
            let color = if msg.starts_with("Error") { TEXT_ERROR } else { TEXT_COLOR };
            // long errors (paths) would overflow at full scale
            let scale = if msg.len() as i32 * 8 * l.scale > fb.width() as i32 { 1 } else { l.scale };
            Label::centered(msg.clone(), fb.bounds(), l.message_y, scale, color).draw(fb);
        }
    }
}
