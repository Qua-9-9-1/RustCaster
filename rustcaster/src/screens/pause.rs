// Pause menu drawn over the (still rendered) 3D view: Resume, Settings,
// style (preset) picker applied immediately, Restart map, Main menu, Quit.
// "Settings" swaps the panel content for the shared `SettingsPanel`.

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::input::Input;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::text_size;
use crate::ui::preview::draw_preset_preview;
use crate::ui::theme::{self, ellipsize, fit_scale, metrics, Metrics};
use crate::ui::widgets::{centered, Button, Focus};

use super::settings::SettingsPanel;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Page {
    Main,
    Settings,
}

// focus order of the main page
const RESUME: usize = 0;
const SETTINGS: usize = 1;
const STYLE_PREV: usize = 2;
const STYLE_NEXT: usize = 3;
const RESTART: usize = 4;
const MENU: usize = 5;
const QUIT: usize = 6;

pub struct PauseScreen {
    page: Page,
    focus: Focus,
    settings: SettingsPanel,
}

struct Layout {
    m: Metrics,
    panel: Rect,
    title: Rect,
    /// Main page: buttons in focus order, and the style row middle cell.
    buttons: Vec<Button>,
    style: Rect,
    /// Settings page content.
    content: Rect,
}

impl PauseScreen {
    pub fn new() -> PauseScreen {
        PauseScreen { page: Page::Main, focus: Focus::default(), settings: SettingsPanel::new() }
    }

    /// Called by `App` every time the game gets paused.
    pub fn on_enter(&mut self) {
        self.page = Page::Main;
        self.focus = Focus::default();
    }

    fn layout(&self, fb_size: (usize, usize)) -> Layout {
        let (w, h) = (fb_size.0 as i32, fb_size.1 as i32);
        let m = metrics(fb_size.1);
        let screen = Rect::new(0, 0, w, h);
        let title_h = 22 * m.s;
        match self.page {
            Page::Main => {
                let rows = 6;
                let pw = (170 * m.s).min(w - 2 * m.margin);
                let ph = 2 * m.pad + title_h + rows * m.button_h + (rows - 1) * m.gap;
                let panel = centered(screen, pw, ph);
                let inner = panel.inset(m.pad);
                let title = Rect::new(inner.x, inner.y, inner.w, title_h);
                let mut y = title.bottom();
                let mut next = || {
                    let r = Rect::new(inner.x, y, inner.w, m.button_h);
                    y += m.button_h + m.gap;
                    r
                };
                let resume = next();
                let settings = next();
                let style_row = next();
                let restart = next();
                let menu = next();
                let quit = next();
                let bh = m.button_h;
                let prev = Rect::new(style_row.x, style_row.y, bh, bh);
                let nxt = Rect::new(style_row.right() - bh, style_row.y, bh, bh);
                let style = Rect::new(prev.right() + m.gap, style_row.y, nxt.x - m.gap - prev.right() - m.gap, bh);
                let f = |i: usize| self.focus.is(i);
                let buttons = vec![
                    Button::new("Resume", resume, m.s).primary().focused(f(RESUME)),
                    Button::new("Settings", settings, m.s).focused(f(SETTINGS)),
                    Button::new("<", prev, m.s).focused(f(STYLE_PREV)),
                    Button::new(">", nxt, m.s).focused(f(STYLE_NEXT)),
                    Button::new("Restart map", restart, m.s).focused(f(RESTART)),
                    Button::new("Main menu", menu, m.s).focused(f(MENU)),
                    Button::new("Quit game", quit, m.s).focused(f(QUIT)),
                ];
                Layout { m, panel, title, buttons, style, content: Rect::default() }
            }
            Page::Settings => {
                let pw = (SettingsPanel::content_width(&m) + 2 * m.pad).min(w - 2 * m.margin);
                let ph = 2 * m.pad + title_h + SettingsPanel::content_height(&m);
                let panel = centered(screen, pw, ph);
                let inner = panel.inset(m.pad);
                let title = Rect::new(inner.x, inner.y, inner.w, title_h);
                let content = Rect::new(inner.x, title.bottom(), inner.w, inner.bottom() - title.bottom());
                Layout { m, panel, title, buttons: Vec::new(), style: Rect::default(), content }
            }
        }
    }

    pub fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        let l = self.layout(fb_size);
        match self.page {
            Page::Settings => {
                if self.settings.update(ctx, input, l.content, &l.m) || input.was_pressed(Scancode::Escape) {
                    self.page = Page::Main;
                    self.focus = Focus::new(Some(SETTINGS));
                }
                Transition::None
            }
            Page::Main => {
                if input.was_pressed(Scancode::Escape) {
                    return Transition::To(AppState::Playing);
                }
                let refs: Vec<&Button> = l.buttons.iter().collect();
                match self.focus.update(input, &refs) {
                    Some(RESUME) => return Transition::To(AppState::Playing),
                    Some(SETTINGS) => {
                        self.page = Page::Settings;
                    }
                    Some(STYLE_PREV) => ctx.cycle_preset(-1),
                    Some(STYLE_NEXT) => ctx.cycle_preset(1),
                    Some(RESTART) => return Transition::RestartGame,
                    Some(MENU) => return Transition::QuitToMenu,
                    Some(QUIT) => return Transition::Quit,
                    _ => {}
                }
                Transition::None
            }
        }
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let l = self.layout((fb.width(), fb.height()));
        let m = &l.m;
        fb.fill_rect(fb.bounds(), theme::OVERLAY_DIM);
        theme::draw_panel(fb, l.panel);
        let title = if self.page == Page::Main { "PAUSED" } else { "SETTINGS" };
        theme::draw_title(fb, title, Rect::new(l.title.x, l.title.y, l.title.w, l.title.h - m.gap), 2 * m.s);
        match self.page {
            Page::Settings => self.settings.draw(ctx, fb, input, l.content, m),
            Page::Main => {
                for b in &l.buttons {
                    b.draw(fb, input);
                }
                // style cell: small preview + preset name
                let cell = l.style;
                fb.fill_rect(cell, theme::BUTTON_PRESSED);
                fb.draw_rect(cell, theme::BUTTON_BORDER);
                let thumb = Rect::new(cell.x + m.s, cell.y + m.s, (cell.h - 2 * m.s) * 4 / 3, cell.h - 2 * m.s);
                draw_preset_preview(fb, &ctx.preset, thumb);
                let text_x = thumb.right() + m.gap;
                let avail = cell.right() - m.gap - text_x;
                let name = &ctx.settings.preset;
                let scale = fit_scale(name, avail, m.s);
                let text = ellipsize(name, avail, scale);
                let th = text_size(&text, scale).1;
                theme::draw_text_soft(fb, &text, text_x, cell.y + (cell.h - th) / 2, scale, theme::TEXT);
            }
        }
    }
}
