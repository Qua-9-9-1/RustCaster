// Pause / settings overlay (minimal stub) drawn over the frozen 3D view.
// Exposes the settings that used to be keyboard hacks (FOV U/J, sensitivity
// I/K, numpad presets) as buttons going through `Settings` / `AppContext`.

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::input::Input;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::draw_text_centered;
use crate::ui::widgets::{centered, draw_panel, row, text_scale, Button, TEXT_COLOR};

pub struct PauseScreen;

struct Layout {
    panel: Rect,
    title: Rect,
    resume: Button,
    fov_minus: Button,
    fov_label: Rect,
    fov_plus: Button,
    sens_minus: Button,
    sens_label: Rect,
    sens_plus: Button,
    preset_prev: Button,
    preset_label: Rect,
    preset_next: Button,
    minimap: Button,
    menu: Button,
    quit: Button,
    scale: i32,
}

/// `[-] label [+]` row.
fn stepper(area: Rect, gap: i32, scale: i32, minus: &str, plus: &str) -> (Button, Rect, Button) {
    let cells = row(area, gap, 4);
    let label = Rect::new(cells[1].x, area.y, cells[2].right() - cells[1].x, area.h);
    (Button::new(minus, cells[0], scale), label, Button::new(plus, cells[3], scale))
}

impl PauseScreen {
    pub fn new() -> PauseScreen {
        PauseScreen
    }

    fn layout(&self, ctx: &AppContext, fb_size: (usize, usize)) -> Layout {
        let scale = text_scale(fb_size.1);
        let item_h = 12 * scale;
        let gap = 3 * scale;
        let rows = 8;
        let w = (fb_size.0 as i32 / 2).max(200);
        let h = rows * (item_h + gap) + 8 * scale;
        let panel = centered(Rect::new(0, 0, fb_size.0 as i32, fb_size.1 as i32), w, h);
        let inner = panel.inset(4 * scale);
        let mut y = inner.y;
        let mut next_row = || {
            let r = Rect::new(inner.x, y, inner.w, item_h);
            y += item_h + gap;
            r
        };
        let title = next_row();
        let resume = Button::new("Resume", next_row(), scale);
        let (fov_minus, fov_label, fov_plus) = stepper(next_row(), gap, scale, "-", "+");
        let (sens_minus, sens_label, sens_plus) = stepper(next_row(), gap, scale, "-", "+");
        let (preset_prev, preset_label, preset_next) = stepper(next_row(), gap, scale, "<", ">");
        let minimap_label = if ctx.settings.show_minimap { "Minimap: on" } else { "Minimap: off" };
        let minimap = Button::new(minimap_label, next_row(), scale);
        let menu = Button::new("Main menu", next_row(), scale);
        let quit = Button::new("Quit", next_row(), scale);
        Layout {
            panel,
            title,
            resume,
            fov_minus,
            fov_label,
            fov_plus,
            sens_minus,
            sens_label,
            sens_plus,
            preset_prev,
            preset_label,
            preset_next,
            minimap,
            menu,
            quit,
            scale,
        }
    }

    pub fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        if input.was_pressed(Scancode::Escape) {
            return Transition::To(AppState::Playing);
        }
        let l = self.layout(ctx, fb_size);
        if l.resume.clicked(input) {
            return Transition::To(AppState::Playing);
        }
        if l.fov_minus.clicked(input) {
            ctx.settings.adjust_fov(-10.0);
        }
        if l.fov_plus.clicked(input) {
            ctx.settings.adjust_fov(10.0);
        }
        if l.sens_minus.clicked(input) {
            ctx.settings.adjust_sensitivity(-0.5);
        }
        if l.sens_plus.clicked(input) {
            ctx.settings.adjust_sensitivity(0.5);
        }
        if l.preset_prev.clicked(input) {
            ctx.cycle_preset(-1);
        }
        if l.preset_next.clicked(input) {
            ctx.cycle_preset(1);
        }
        if l.minimap.clicked(input) {
            ctx.settings.show_minimap = !ctx.settings.show_minimap;
        }
        if l.menu.clicked(input) {
            return Transition::QuitToMenu;
        }
        if l.quit.clicked(input) {
            return Transition::Quit;
        }
        Transition::None
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let l = self.layout(ctx, (fb.width(), fb.height()));
        draw_panel(fb, l.panel);
        draw_text_centered(fb, "Paused", l.title, l.scale, TEXT_COLOR);
        for b in [&l.resume, &l.fov_minus, &l.fov_plus, &l.sens_minus, &l.sens_plus, &l.preset_prev, &l.preset_next, &l.minimap, &l.menu, &l.quit] {
            b.draw(fb, input);
        }
        let s = &ctx.settings;
        draw_text_centered(fb, &format!("FOV {:.0}", s.fov_deg), l.fov_label, l.scale, TEXT_COLOR);
        draw_text_centered(fb, &format!("Sens {:.1}", s.sensitivity), l.sens_label, l.scale, TEXT_COLOR);
        draw_text_centered(fb, &s.preset, l.preset_label, l.scale, TEXT_COLOR);
    }
}
