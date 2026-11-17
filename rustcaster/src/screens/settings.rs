// Settings: `SettingsPanel` is the reusable component (FOV and turn speed
// sliders with -/+ buttons, minimap toggle, Back), shown full screen by
// `SettingsScreen` (from the title screen) and inside the pause menu.
// Every change goes through the clamped `Settings` setters and applies at once.

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::config::{MAX_FOV_DEG, MAX_SENSITIVITY, MIN_FOV_DEG, MIN_SENSITIVITY};
use crate::input::{Input, Mouse};
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::settings::Settings;
use crate::ui::font::text_size;
use crate::ui::theme::{self, metrics, Metrics};
use crate::ui::widgets::{centered, Button};

const FOV_STEP: f64 = 5.0;
const SENSITIVITY_STEP: f64 = 0.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slider {
    Fov,
    Sensitivity,
}

impl Slider {
    fn range(self) -> (f64, f64) {
        match self {
            Slider::Fov => (MIN_FOV_DEG, MAX_FOV_DEG),
            Slider::Sensitivity => (MIN_SENSITIVITY, MAX_SENSITIVITY),
        }
    }

    fn value(self, s: &Settings) -> f64 {
        match self {
            Slider::Fov => s.fov_deg,
            Slider::Sensitivity => s.sensitivity,
        }
    }

    /// Sets the value from a 0..1 position on the track (rounded to the display precision).
    fn set_fraction(self, s: &mut Settings, t: f64) {
        let (lo, hi) = self.range();
        let v = lo + t.clamp(0.0, 1.0) * (hi - lo);
        match self {
            Slider::Fov => s.set_fov(v.round()),
            Slider::Sensitivity => s.set_sensitivity((v * 10.0).round() / 10.0),
        }
    }

    fn step(self, s: &mut Settings, dir: f64) {
        match self {
            Slider::Fov => s.adjust_fov(dir * FOV_STEP),
            Slider::Sensitivity => s.adjust_sensitivity(dir * SENSITIVITY_STEP),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Slider::Fov => "Field of view",
            Slider::Sensitivity => "Turn speed",
        }
    }

    fn value_text(self, s: &Settings) -> String {
        match self {
            Slider::Fov => format!("{:.0} deg", s.fov_deg),
            Slider::Sensitivity => format!("{:.1}", s.sensitivity),
        }
    }
}

struct SliderRow {
    kind: Slider,
    /// Line with the label (left) and the value (right).
    text: Rect,
    minus: Button,
    /// Full-height hit area of the track (drawn thinner).
    track: Rect,
    plus: Button,
}

struct PanelLayout {
    sliders: [SliderRow; 2],
    minimap_label: Rect,
    minimap: Button,
    back: Button,
}

/// The settings controls, laid out in a content rect given by the host.
pub struct SettingsPanel {
    dragging: Option<Slider>,
}

impl SettingsPanel {
    pub fn new() -> SettingsPanel {
        SettingsPanel { dragging: None }
    }

    fn group_h(m: &Metrics) -> i32 {
        m.line() + 3 * m.s + m.button_h
    }

    fn sep(m: &Metrics) -> i32 {
        6 * m.s
    }

    /// Height the panel content needs (hosts size their frame with it).
    pub fn content_height(m: &Metrics) -> i32 {
        2 * Self::group_h(m) + 3 * Self::sep(m) + 2 * m.button_h
    }

    /// Preferred content width.
    pub fn content_width(m: &Metrics) -> i32 {
        200 * m.s
    }

    fn layout(&self, ctx: &AppContext, area: Rect, m: &Metrics) -> PanelLayout {
        let slider_row = |kind: Slider, y: i32| {
            let text = Rect::new(area.x, y, area.w, m.line());
            let cy = y + m.line() + 3 * m.s;
            let minus = Button::new("-", Rect::new(area.x, cy, m.button_h, m.button_h), m.s);
            let plus = Button::new("+", Rect::new(area.right() - m.button_h, cy, m.button_h, m.button_h), m.s);
            let tx = minus.rect.right() + m.gap;
            let track = Rect::new(tx, cy, plus.rect.x - m.gap - tx, m.button_h);
            SliderRow { kind, text, minus, track, plus }
        };
        let mut y = area.y;
        let fov = slider_row(Slider::Fov, y);
        y += Self::group_h(m) + Self::sep(m);
        let sens = slider_row(Slider::Sensitivity, y);
        y += Self::group_h(m) + Self::sep(m);

        let toggle_w = 40 * m.s;
        let minimap_label = Rect::new(area.x, y, area.w - toggle_w - m.gap, m.button_h);
        let on = ctx.settings.show_minimap;
        let minimap = Button::new(if on { "ON" } else { "OFF" }, Rect::new(area.right() - toggle_w, y, toggle_w, m.button_h), m.s).selected(on);
        y += m.button_h + Self::sep(m);

        let bw = (80 * m.s).min(area.w);
        let back = Button::new("Back", Rect::new(area.x + (area.w - bw) / 2, y, bw, m.button_h), m.s);
        PanelLayout { sliders: [fov, sens], minimap_label, minimap, back }
    }

    /// Returns true when "Back" was clicked (Escape is left to the host).
    pub fn update(&mut self, ctx: &mut AppContext, input: &Input, area: Rect, m: &Metrics) -> bool {
        let l = self.layout(ctx, area, m);
        let (mx, my) = input.mouse_pos();
        for row in &l.sliders {
            if row.minus.clicked(input) {
                row.kind.step(&mut ctx.settings, -1.0);
            }
            if row.plus.clicked(input) {
                row.kind.step(&mut ctx.settings, 1.0);
            }
            if input.mouse_clicked(Mouse::Left) && row.track.contains(mx, my) {
                self.dragging = Some(row.kind);
            }
        }
        if !input.mouse_down(Mouse::Left) {
            self.dragging = None;
        }
        if let Some(kind) = self.dragging {
            if let Some(row) = l.sliders.iter().find(|r| r.kind == kind) {
                let t = (mx - row.track.x) as f64 / (row.track.w - 1).max(1) as f64;
                kind.set_fraction(&mut ctx.settings, t);
            }
        }
        if l.minimap.clicked(input) {
            ctx.settings.show_minimap = !ctx.settings.show_minimap;
        }
        l.back.clicked(input)
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input, area: Rect, m: &Metrics) {
        let l = self.layout(ctx, area, m);
        let s = &ctx.settings;
        let (mx, my) = input.mouse_pos();
        for row in &l.sliders {
            theme::draw_text_soft(fb, row.kind.label(), row.text.x, row.text.y, m.s, theme::TEXT);
            let value = row.kind.value_text(s);
            let vw = text_size(&value, m.s).0;
            theme::draw_text_soft(fb, &value, row.text.right() - vw, row.text.y, m.s, theme::ACCENT);
            row.minus.draw(fb, input);
            row.plus.draw(fb, input);

            let (lo, hi) = row.kind.range();
            let t = ((row.kind.value(s) - lo) / (hi - lo)).clamp(0.0, 1.0);
            let bar_h = 4 * m.s;
            let bar = Rect::new(row.track.x, row.track.y + (row.track.h - bar_h) / 2, row.track.w, bar_h);
            fb.fill_rect(bar, theme::BUTTON_PRESSED);
            let filled = (bar.w as f64 * t) as i32;
            fb.fill_rect(Rect::new(bar.x, bar.y, filled, bar.h), theme::ACCENT_DARK);
            fb.fill_rect(Rect::new(bar.x, bar.y, filled, (bar.h / 2).max(1)), theme::ACCENT);
            fb.draw_rect(bar, theme::BUTTON_BORDER);
            let hot = self.dragging == Some(row.kind) || row.track.contains(mx, my);
            let knob_w = 4 * m.s;
            let kx = bar.x + filled - knob_w / 2;
            let knob = Rect::new(kx.clamp(bar.x, bar.right() - knob_w), row.track.y + m.s, knob_w, row.track.h - 2 * m.s);
            fb.fill_rect(Rect::new(knob.x + 1, knob.y + 2, knob.w, knob.h), Color::rgba(0, 0, 0, 120));
            fb.fill_rect(knob, if hot { theme::ACCENT_LIGHT } else { theme::TEXT });
            fb.draw_rect(knob, theme::ACCENT_DARK);
        }
        let ly = l.minimap_label.y + (l.minimap_label.h - m.line()) / 2;
        theme::draw_text_soft(fb, "Minimap", l.minimap_label.x, ly, m.s, theme::TEXT);
        l.minimap.draw(fb, input);
        l.back.draw(fb, input);
    }
}

/// Full-screen settings page reached from the title screen.
pub struct SettingsScreen {
    panel: SettingsPanel,
    tick: u32,
}

struct ScreenLayout {
    m: Metrics,
    title: Rect,
    frame: Rect,
    content: Rect,
    hint: Rect,
}

impl SettingsScreen {
    pub fn new() -> SettingsScreen {
        SettingsScreen { panel: SettingsPanel::new(), tick: 0 }
    }

    fn layout(fb_size: (usize, usize)) -> ScreenLayout {
        let (w, h) = (fb_size.0 as i32, fb_size.1 as i32);
        let m = metrics(fb_size.1);
        let title = Rect::new(0, 2 * m.margin, w, 20 * m.s);
        let fw = (SettingsPanel::content_width(&m) + 2 * m.pad).min(w - 2 * m.margin);
        let fh = SettingsPanel::content_height(&m) + 2 * m.pad;
        let below = Rect::new(0, title.bottom(), w, h - title.bottom() - m.margin - m.small_line());
        let frame = centered(below, fw, fh);
        let content = frame.inset(m.pad);
        let hint = Rect::new(0, h - m.margin - m.small_line(), w, m.small_line());
        ScreenLayout { m, title, frame, content, hint }
    }

    pub fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        self.tick = self.tick.wrapping_add(1);
        let l = Self::layout(fb_size);
        if self.panel.update(ctx, input, l.content, &l.m) || input.was_pressed(Scancode::Escape) {
            return Transition::To(AppState::Menu);
        }
        Transition::None
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let l = Self::layout((fb.width(), fb.height()));
        theme::draw_background(fb, self.tick as f64 / 60.0 * 0.3);
        theme::draw_title(fb, "SETTINGS", l.title, 2 * l.m.s);
        theme::draw_panel(fb, l.frame);
        self.panel.draw(ctx, fb, input, l.content, &l.m);
        theme::draw_text_soft_centered(fb, "Changes apply immediately     Esc: back", l.hint, l.m.small, theme::TEXT_DIM);
    }
}
