// End-of-level overlay drawn over the frozen 3D view: score, coins, time and
// Next map / Replay / Menu. `App` fills it with `open(LevelResult)` when the
// player reaches an exit.

use std::fs;
use std::path::{Path, PathBuf};

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, Transition};
use crate::game::GameState;
use crate::input::Input;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::text_size;
use crate::ui::preview::draw_coin_icon;
use crate::ui::theme::{self, fit_scale, metrics, Metrics};
use crate::ui::widgets::{centered, row, Button, Focus};

const NEXT: usize = 0;
const REPLAY: usize = 1;
const MENU: usize = 2;

/// Points per coin, and per second under `PAR_SECONDS`.
const COIN_POINTS: u64 = 100;
const PAR_SECONDS: f64 = 120.0;
const TIME_POINTS: f64 = 10.0;
const ALL_COINS_BONUS: u64 = 500;

#[derive(Clone, Debug)]
pub struct LevelResult {
    pub map_name: String,
    /// Map after this one in `ctx.maps`, if any.
    pub next_map: Option<PathBuf>,
    pub coins: usize,
    pub total_coins: usize,
    pub time: f64,
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b
        || match (fs::canonicalize(a), fs::canonicalize(b)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
}

/// "mm:ss.t"
fn format_time_precise(seconds: f64) -> String {
    let tenths = (seconds.max(0.0) * 10.0) as u64;
    format!("{:02}:{:02}.{}", tenths / 600, (tenths / 10) % 60, tenths % 10)
}

impl LevelResult {
    pub fn new(game: &GameState, maps: &[PathBuf]) -> LevelResult {
        let map_name = game.map_path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let next_map = maps.iter().position(|p| same_file(p, &game.map_path)).and_then(|i| maps.get(i + 1)).cloned();
        LevelResult {
            map_name,
            next_map,
            coins: game.coins_collected,
            total_coins: game.map.total_coins,
            time: game.time,
        }
    }

    pub fn all_coins(&self) -> bool {
        self.coins == self.total_coins
    }

    /// 100 per coin + 10 per second under two minutes + 500 for every coin.
    pub fn score(&self) -> u64 {
        let time_bonus = ((PAR_SECONDS - self.time).max(0.0) * TIME_POINTS) as u64;
        let bonus = if self.all_coins() && self.total_coins > 0 { ALL_COINS_BONUS } else { 0 };
        self.coins as u64 * COIN_POINTS + time_bonus + bonus
    }

    /// One line for `ctx.message`.
    pub fn summary(&self) -> String {
        format!(
            "{}: {}/{} coins, {}, score {}",
            self.map_name,
            self.coins,
            self.total_coins,
            format_time_precise(self.time),
            self.score()
        )
    }
}

pub struct LevelCompleteScreen {
    result: Option<LevelResult>,
    focus: Focus,
    /// Frames since `open` (score count-up animation).
    tick: u32,
}

struct Layout {
    m: Metrics,
    panel: Rect,
    title: Rect,
    title_scale: i32,
    map_name: Rect,
    stats: [Rect; 3],
    perfect: Rect,
    buttons: Vec<Button>,
}

impl LevelCompleteScreen {
    pub fn new() -> LevelCompleteScreen {
        LevelCompleteScreen { result: None, focus: Focus::default(), tick: 0 }
    }

    pub fn open(&mut self, result: LevelResult) {
        self.focus = Focus::new(Some(if result.next_map.is_some() { NEXT } else { REPLAY }));
        self.result = Some(result);
        self.tick = 0;
    }

    fn layout(&self, fb_size: (usize, usize)) -> Layout {
        let (w, h) = (fb_size.0 as i32, fb_size.1 as i32);
        let m = metrics(fb_size.1);
        let stat_h = 12 * m.s;
        let stat_gap = 2 * m.s;
        let title_h = 24 * m.s;
        let pw = (240 * m.s).min(w - 2 * m.margin);
        let ph = 2 * m.pad
            + title_h
            + m.line()
            + m.gap
            + 3 * stat_h
            + 2 * stat_gap
            + m.gap
            + m.small_line()
            + 6 * m.s
            + m.button_h;
        let panel = centered(Rect::new(0, 0, w, h), pw, ph);
        let inner = panel.inset(m.pad);
        let title = Rect::new(inner.x, inner.y, inner.w, title_h);
        let title_scale = (2 * m.s).min(inner.w / text_size("LEVEL COMPLETE", 1).0).max(1);
        let map_name = Rect::new(inner.x, title.bottom(), inner.w, m.line());
        let sy = map_name.bottom() + m.gap;
        let stat = |i: i32| Rect::new(inner.x, sy + i * (stat_h + stat_gap), inner.w, stat_h);
        let stats = [stat(0), stat(1), stat(2)];
        let perfect = Rect::new(inner.x, stats[2].bottom() + m.gap, inner.w, m.small_line());
        let cells = row(Rect::new(inner.x, inner.bottom() - m.button_h, inner.w, m.button_h), m.gap, 3);
        let has_next = self.result.as_ref().map_or(false, |r| r.next_map.is_some());
        let buttons = vec![
            Button::new("Next map", cells[0], m.s).primary().enabled(has_next).focused(self.focus.is(NEXT)),
            Button::new("Replay", cells[1], m.s).focused(self.focus.is(REPLAY)),
            Button::new("Menu", cells[2], m.s).focused(self.focus.is(MENU)),
        ];
        Layout { m, panel, title, title_scale, map_name, stats, perfect, buttons }
    }

    pub fn update(&mut self, _ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        self.tick = self.tick.saturating_add(1);
        if input.was_pressed(Scancode::Escape) {
            return Transition::QuitToMenu;
        }
        let l = self.layout(fb_size);
        let refs: Vec<&Button> = l.buttons.iter().collect();
        match self.focus.update(input, &refs) {
            Some(NEXT) => match self.result.as_ref().and_then(|r| r.next_map.clone()) {
                Some(path) => Transition::StartGame(path),
                None => Transition::None,
            },
            Some(REPLAY) => Transition::RestartGame,
            Some(MENU) => Transition::QuitToMenu,
            _ => Transition::None,
        }
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let result = match &self.result {
            Some(r) => r,
            None => return,
        };
        let l = self.layout((fb.width(), fb.height()));
        let m = &l.m;
        fb.fill_rect(fb.bounds(), theme::OVERLAY_DIM);
        theme::draw_panel(fb, l.panel);
        theme::draw_title(fb, "LEVEL COMPLETE", l.title, l.title_scale);
        theme::draw_text_soft_centered(fb, &result.map_name, l.map_name, m.s, theme::TEXT_DIM);

        let draw_stat = |fb: &mut FrameBuffer, r: Rect, label: &str, value: &str, color| {
            fb.fill_rect(r, theme::BUTTON_PRESSED.with_alpha(160));
            let ty = r.y + (r.h - m.line()) / 2;
            theme::draw_text_soft(fb, label, r.x + m.pad, ty, m.s, theme::TEXT_DIM);
            let scale = fit_scale(value, r.w / 2, m.s);
            let (vw, vh) = text_size(value, scale);
            theme::draw_text_soft(fb, value, r.right() - m.pad - vw, r.y + (r.h - vh) / 2, scale, color);
            r.right() - m.pad - vw
        };

        let coins = format!("{} / {}", result.coins, result.total_coins);
        let coin_color = if result.all_coins() { theme::TEXT_SUCCESS } else { theme::TEXT };
        let vx = draw_stat(fb, l.stats[0], "Coins", &coins, coin_color);
        let icon = l.stats[0].h - 4 * m.s;
        draw_coin_icon(fb, &ctx.preset, Rect::new(vx - m.gap - icon, l.stats[0].y + 2 * m.s, icon, icon));
        draw_stat(fb, l.stats[1], "Time", &format_time_precise(result.time), theme::TEXT);
        // score counts up during the first second
        let t = (self.tick as f64 / 50.0).min(1.0);
        let shown = (result.score() as f64 * t).round() as u64;
        draw_stat(fb, l.stats[2], "Score", &shown.to_string(), theme::ACCENT);

        let note = if result.total_coins == 0 {
            "No coin on this map"
        } else if result.all_coins() {
            "Perfect: every coin collected!"
        } else {
            "Some coins are still out there..."
        };
        let color = if result.all_coins() && result.total_coins > 0 { theme::TEXT_SUCCESS } else { theme::TEXT_DIM };
        theme::draw_text_soft_centered(fb, note, l.perfect, m.small, color);

        for b in &l.buttons {
            b.draw(fb, input);
        }
    }
}
