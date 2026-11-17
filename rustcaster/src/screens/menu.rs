// Title screen: Play (-> level select), Create preset (-> preset editor),
// Settings, Quit, plus the last status message (errors, level result).

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::input::Input;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::text_size;
use crate::ui::theme::{self, metrics, Metrics};
use crate::ui::widgets::{column, Button, Focus};

const TITLE: &str = "RUSTCASTER";
const SUBTITLE: &str = "a raycasting maze";
const FOOTER: &str = "Mouse or arrows + Enter     Esc: quit";

const PLAY: usize = 0;
const CREATE: usize = 1;
const SETTINGS: usize = 2;
const QUIT: usize = 3;

pub struct MenuScreen {
    focus: Focus,
    /// Frames since start, animates the background.
    tick: u32,
}

struct Layout {
    m: Metrics,
    title: Rect,
    title_scale: i32,
    subtitle: Rect,
    buttons: Vec<Button>,
    message: Rect,
    footer: Rect,
}

impl MenuScreen {
    pub fn new() -> MenuScreen {
        MenuScreen { focus: Focus::default(), tick: 0 }
    }

    fn layout(&self, fb_size: (usize, usize)) -> Layout {
        let (w, h) = (fb_size.0 as i32, fb_size.1 as i32);
        let m = metrics(fb_size.1);
        let title_scale = (3 * m.s).min((w - 2 * m.margin) / text_size(TITLE, 1).0).max(1);
        let title = Rect::new(0, h * 15 / 100, w, 8 * title_scale + 4 * m.s);
        let subtitle = Rect::new(0, title.bottom() + m.gap, w, m.line());

        let bw = (110 * m.s).min(w - 2 * m.margin);
        let bh = m.button_h + 2 * m.s;
        let top = subtitle.bottom() + 14 * m.s;
        let rects = column(Rect::new((w - bw) / 2, top, bw, 0), bh, 5 * m.s, 4);
        let labels = ["Play", "Create preset", "Settings", "Quit"];
        let buttons = labels
            .iter()
            .zip(rects)
            .enumerate()
            .map(|(i, (label, r))| {
                let b = Button::new(*label, r, m.s).focused(self.focus.is(i));
                if i == PLAY {
                    b.primary()
                } else {
                    b
                }
            })
            .collect();

        let footer = Rect::new(0, h - m.margin - m.small_line(), w, m.small_line());
        let mw = (300 * m.s).min(w - 2 * m.margin);
        let mh = 18 * m.s;
        let message = Rect::new((w - mw) / 2, footer.y - m.gap - mh, mw, mh);
        Layout { m, title, title_scale, subtitle, buttons, message, footer }
    }

    pub fn update(&mut self, _ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        self.tick = self.tick.wrapping_add(1);
        if input.was_pressed(Scancode::Escape) {
            return Transition::Quit;
        }
        let l = self.layout(fb_size);
        let refs: Vec<&Button> = l.buttons.iter().collect();
        match self.focus.update(input, &refs) {
            Some(PLAY) => Transition::To(AppState::LevelSelect),
            Some(CREATE) => Transition::To(AppState::PresetEditor),
            Some(SETTINGS) => Transition::To(AppState::Settings),
            Some(QUIT) => Transition::Quit,
            _ => Transition::None,
        }
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let l = self.layout((fb.width(), fb.height()));
        let m = &l.m;
        theme::draw_background(fb, self.tick as f64 / 60.0 * 0.6);
        theme::draw_title(fb, TITLE, l.title, l.title_scale);
        theme::draw_text_soft_centered(fb, SUBTITLE, l.subtitle, m.s, theme::TEXT_DIM);
        for b in &l.buttons {
            b.draw(fb, input);
        }
        if let Some(msg) = &ctx.message {
            theme::draw_panel(fb, l.message);
            theme::draw_message(fb, msg, l.message, m);
        }
        theme::draw_text_soft_centered(fb, FOOTER, l.footer, m.small, theme::TEXT_DIM);
    }
}
