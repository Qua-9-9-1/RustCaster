// Level select: scrollable list of `maps/*.txt` (name, size, coins and a
// thumbnail in the current style) on the left, style (preset) carousel with a
// preview on the right, Back / Start at the bottom.
//
//   +---------------------------------------------------+
//   |                  SELECT LEVEL                     |
//   | +- MAPS ------------- 6 maps -+ +- STYLE --- 3/10 -+ |
//   | | map1          [thumb]       | |   [ preview ]    | |
//   | | 11 x 8   7 coins            | | <    house     > | |
//   | | ...                       |#| |  outdoor - ...   | |
//   | +-----------------------------+ +------------------+ |
//   | [Back]        message / hint            [Start]    |
//   +---------------------------------------------------+

use std::path::Path;

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::assets::preset::BUILTIN_PRESETS;
use crate::game::map::Map;
use crate::input::{Input, Mouse};
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::{draw_text, text_size};
use crate::ui::preview::{draw_map_thumbnail, draw_palette, draw_preset_preview};
use crate::ui::theme::{self, ellipsize, fit_scale, metrics, Metrics};
use crate::ui::widgets::{draw_scrollbar, scrollbar, Button};

const TITLE: &str = "SELECT LEVEL";
const HINT: &str = "Up/Down: map   Left/Right: style   Enter: start";

/// What the list shows about a map file (parsed once when entering the screen).
struct MapInfo {
    name: String,
    map: Result<Map, String>,
}

impl MapInfo {
    fn load(path: &Path) -> MapInfo {
        let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        MapInfo { name, map: Map::load(path).map_err(|e| e.to_string()) }
    }
}

pub struct LevelSelectScreen {
    infos: Vec<MapInfo>,
    selected: usize,
    /// First visible list item.
    scroll: usize,
    tick: u32,
}

struct Layout {
    m: Metrics,
    title: Rect,
    title_scale: i32,
    maps_panel: Rect,
    list: Rect,
    /// (map index, rect) of the visible items.
    items: Vec<(usize, Rect)>,
    fit: usize,
    first: usize,
    track: Option<Rect>,
    style_panel: Rect,
    preview: Rect,
    prev: Button,
    next: Button,
    name: Rect,
    tags: Rect,
    palette: Rect,
    back: Button,
    start: Button,
    message: Rect,
}

impl LevelSelectScreen {
    pub fn new() -> LevelSelectScreen {
        LevelSelectScreen { infos: Vec::new(), selected: 0, scroll: 0, tick: 0 }
    }

    /// Called by `App` when the screen is shown: rescans `maps/` and parses them.
    pub fn on_enter(&mut self, ctx: &mut AppContext) {
        ctx.rescan_maps();
        self.reload(ctx);
    }

    fn reload(&mut self, ctx: &AppContext) {
        self.infos = ctx.maps.iter().map(|p| MapInfo::load(p)).collect();
        self.selected = self.selected.min(self.infos.len().saturating_sub(1));
    }

    fn item_h(m: &Metrics) -> i32 {
        4 * m.s + m.line() + 3 * m.s + m.small_line() + 4 * m.s
    }

    fn can_start(&self) -> bool {
        self.infos.get(self.selected).map_or(false, |i| i.map.is_ok())
    }

    fn layout(&self, fb_size: (usize, usize)) -> Layout {
        let (w, h) = (fb_size.0 as i32, fb_size.1 as i32);
        let m = metrics(fb_size.1);
        let cw = (440 * m.s).min(w - 2 * m.margin);
        let cx0 = (w - cw) / 2;

        let title_scale = (2 * m.s).min(cw / text_size(TITLE, 1).0).max(1);
        let title = Rect::new(0, m.margin, w, 20 * m.s);

        // bottom bar
        let by = h - m.margin - m.button_h;
        let back_w = (60 * m.s).min(cw / 4);
        let start_w = (70 * m.s).min(cw / 4);
        let back = Button::new("Back", Rect::new(cx0, by, back_w, m.button_h), m.s);
        let start = Button::new("Start", Rect::new(cx0 + cw - start_w, by, start_w, m.button_h), m.s).primary().enabled(self.can_start());
        let message = Rect::new(back.rect.right() + m.gap, by, start.rect.x - m.gap - back.rect.right() - m.gap, m.button_h);

        // panels
        let top = title.bottom() + m.gap;
        let ch = by - m.gap - top;
        let maps_w = (cw - m.gap) * 56 / 100;
        let style_w = cw - m.gap - maps_w;
        let maps_panel = Rect::new(cx0, top, maps_w, ch);
        let style_panel = Rect::new(maps_panel.right() + m.gap, top, style_w, ch);

        // map list
        let list = theme::titled_panel_content(maps_panel, &m);
        let n = self.infos.len();
        let item_h = Self::item_h(&m);
        let item_gap = 3 * m.s;
        let fit = ((list.h + item_gap) / (item_h + item_gap)).max(1) as usize;
        let first = self.scroll.min(n.saturating_sub(fit));
        let shown = n.saturating_sub(first).min(fit);
        let (track, item_w) = if n > fit {
            let sb = 3 * m.s;
            (Some(Rect::new(list.right() - sb, list.y, sb, list.h)), list.w - sb - 2 * sb)
        } else {
            (None, list.w)
        };
        let items = (0..shown)
            .map(|i| (first + i, Rect::new(list.x, list.y + i as i32 * (item_h + item_gap), item_w, item_h)))
            .collect();

        // style carousel, vertically centred in the panel content
        let inner = theme::titled_panel_content(style_panel, &m);
        let palette_h = 8 * m.s;
        let below = m.gap + m.button_h + m.gap + m.small_line() + m.gap + palette_h;
        let ph = ((inner.h - below).max(8)).min(inner.w * 3 / 4);
        let pw = (ph * 4 / 3).min(inner.w);
        let y0 = inner.y + (inner.h - ph - below).max(0) / 2;
        let preview = Rect::new(inner.x + (inner.w - pw) / 2, y0, pw, ph);
        let sy = preview.bottom() + m.gap;
        let prev = Button::new("<", Rect::new(inner.x, sy, m.button_h, m.button_h), m.s);
        let next = Button::new(">", Rect::new(inner.right() - m.button_h, sy, m.button_h, m.button_h), m.s);
        let name = Rect::new(prev.rect.right() + m.gap, sy, next.rect.x - m.gap - prev.rect.right() - m.gap, m.button_h);
        let tags = Rect::new(inner.x, name.bottom() + m.gap, inner.w, m.small_line());
        let palette = Rect::new(preview.x, tags.bottom() + m.gap, preview.w, palette_h);

        Layout {
            m,
            title,
            title_scale,
            maps_panel,
            list,
            items,
            fit,
            first,
            track,
            style_panel,
            preview,
            prev,
            next,
            name,
            tags,
            palette,
            back,
            start,
            message,
        }
    }

    fn start(&self, ctx: &AppContext) -> Transition {
        if self.can_start() {
            Transition::StartGame(ctx.maps[self.selected].clone())
        } else {
            Transition::None
        }
    }

    pub fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        self.tick = self.tick.wrapping_add(1);
        if self.infos.len() != ctx.maps.len() {
            self.reload(ctx);
        }
        if input.was_pressed(Scancode::Escape) {
            return Transition::To(AppState::Menu);
        }
        let l = self.layout(fb_size);
        let n = self.infos.len();
        let max_first = n.saturating_sub(l.fit);
        let (mx, my) = input.mouse_pos();

        // scrolling: wheel over the list, scrollbar drag
        self.scroll = l.first;
        if input.wheel() != 0 && l.maps_panel.contains(mx, my) {
            self.scroll = (self.scroll as i32 - input.wheel()).clamp(0, max_first as i32) as usize;
        }
        if let Some(track) = l.track {
            if let Some(first) = scrollbar(input, track, l.fit, n) {
                self.scroll = first;
            }
        }

        // selection: click, Up/Down (keeps the selection visible)
        if input.mouse_clicked(Mouse::Left) {
            if let Some((i, _)) = l.items.iter().find(|(_, r)| r.contains(mx, my)) {
                self.selected = *i;
            }
        }
        let mut moved = false;
        if input.was_pressed(Scancode::Up) && self.selected > 0 {
            self.selected -= 1;
            moved = true;
        }
        if input.was_pressed(Scancode::Down) && self.selected + 1 < n {
            self.selected += 1;
            moved = true;
        }
        if moved {
            if self.selected < self.scroll {
                self.scroll = self.selected;
            } else if self.selected >= self.scroll + l.fit {
                self.scroll = self.selected + 1 - l.fit;
            }
        }

        // style
        if l.prev.clicked(input) || input.was_pressed(Scancode::Left) {
            ctx.cycle_preset(-1);
        }
        if l.next.clicked(input) || input.was_pressed(Scancode::Right) {
            ctx.cycle_preset(1);
        }

        if l.back.clicked(input) {
            return Transition::To(AppState::Menu);
        }
        if l.start.clicked(input) || input.confirm_pressed() {
            return self.start(ctx);
        }
        Transition::None
    }

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let l = self.layout((fb.width(), fb.height()));
        let m = &l.m;
        let (mx, my) = input.mouse_pos();
        theme::draw_background(fb, self.tick as f64 / 60.0 * 0.3);
        theme::draw_title(fb, TITLE, l.title, l.title_scale);

        // maps
        let count = match self.infos.len() {
            1 => "1 map".to_string(),
            n => format!("{} maps", n),
        };
        theme::draw_titled_panel(fb, l.maps_panel, "MAPS", Some(&count), m);
        if self.infos.is_empty() {
            theme::draw_text_soft_centered(fb, "No map found in maps/", l.list, m.small, theme::TEXT_DIM);
        }
        for (i, r) in &l.items {
            self.draw_item(fb, ctx, &self.infos[*i], *r, *i == self.selected, r.contains(mx, my), m);
        }
        if let Some(track) = l.track {
            draw_scrollbar(fb, track, l.first, l.fit, self.infos.len());
        }

        // style
        let n = ctx.presets.len().max(1);
        let index = ctx.preset_index();
        let counter = format!("{}/{}", index + 1, n);
        theme::draw_titled_panel(fb, l.style_panel, "STYLE", Some(&counter), m);
        draw_preset_preview(fb, &ctx.preset, l.preview);
        l.prev.draw(fb, input);
        l.next.draw(fb, input);
        let name = &ctx.settings.preset;
        let scale = fit_scale(name, l.name.w, m.s);
        theme::draw_text_soft_centered(fb, &ellipsize(name, l.name.w, scale), l.name, scale, theme::ACCENT_LIGHT);
        let p = &ctx.preset;
        let mut tags = vec![if p.outside { "outdoor" } else { "indoor" }];
        if p.fog {
            tags.push("fog");
        }
        tags.push(if p.textures.wall.is_some() { "textured" } else { "flat colours" });
        if !BUILTIN_PRESETS.contains(&name.as_str()) {
            tags.push("custom");
        }
        let tags = ellipsize(&tags.join(" - "), l.tags.w, m.small);
        theme::draw_text_soft_centered(fb, &tags, l.tags, m.small, theme::TEXT_DIM);
        draw_palette(fb, p, l.palette, m.s);

        // bottom bar
        l.back.draw(fb, input);
        l.start.draw(fb, input);
        match &ctx.message {
            Some(msg) => theme::draw_message(fb, msg, l.message, m),
            None => {
                let text = ellipsize(HINT, l.message.w, m.small);
                theme::draw_text_soft_centered(fb, &text, l.message, m.small, theme::TEXT_DIM);
            }
        }
    }

    fn draw_item(&self, fb: &mut FrameBuffer, ctx: &AppContext, info: &MapInfo, r: Rect, selected: bool, hovered: bool, m: &Metrics) {
        if selected {
            fb.fill_rect(r, theme::SELECTION_BG);
            fb.draw_rect(r, theme::ACCENT);
            fb.fill_rect(Rect::new(r.x, r.y, 2 * m.s, r.h), theme::ACCENT);
        } else if hovered {
            fb.fill_rect(r, Color::rgba(255, 255, 255, 20));
            fb.draw_rect(r, theme::BUTTON_BORDER);
        } else {
            fb.fill_rect(r, Color::rgba(255, 255, 255, 8));
            fb.draw_rect(r, Color::rgba(255, 255, 255, 24));
        }

        let th = r.h - 6 * m.s;
        let tw = th * 16 / 10;
        let thumb = Rect::new(r.right() - 4 * m.s - tw, r.y + 3 * m.s, tw, th);
        let tx = r.x + 6 * m.s;
        let text_w = thumb.x - m.gap - tx;
        let ny = r.y + 4 * m.s;
        let iy = ny + m.line() + 3 * m.s;
        let name_color = if selected { theme::ACCENT_LIGHT } else { theme::TEXT };
        theme::draw_text_soft(fb, &ellipsize(&info.name, text_w, m.s), tx, ny, m.s, name_color);
        match &info.map {
            Ok(map) => {
                draw_map_thumbnail(fb, map, &ctx.preset, thumb);
                let coins = if map.total_coins == 1 { "coin" } else { "coins" };
                let text = format!("{} x {}   {} {}", map.width, map.height, map.total_coins, coins);
                draw_text(fb, &ellipsize(&text, text_w, m.small), tx, iy, m.small, theme::TEXT_DIM);
            }
            Err(e) => {
                let text = format!("Invalid: {}", e);
                draw_text(fb, &ellipsize(&text, thumb.right() - tx, m.small), tx, iy, m.small, theme::TEXT_ERROR);
            }
        }
    }
}
