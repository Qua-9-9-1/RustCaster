// Custom preset creator.
//
// Flow: on entry the working copy is the current preset (a user preset is
// edited in place, a built-in one is cloned under a new name). "Start from"
// + "Load" replaces it with another preset (or an empty one). Each texture
// slot has a card: thumbnail, flat colour, "Import" (native file dialog, see
// `import.rs`) and "Clear". Clicking a card selects its colour for the colour
// panel (RGB sliders, palette, "From texture" = average colour). "Save"
// writes `presets/<id>/` (see `PresetLibrary::save_as`), rescans the library
// and selects the new preset. "Delete" removes the user preset being edited.

mod import;
mod preview;
mod text_field;

use std::path::PathBuf;
use std::time::Instant;

use sdl2::keyboard::Scancode;

use crate::app::{AppContext, AppState, Transition};
use crate::assets::preset::{Preset, PresetColors, TextureSlot, FOG_STRENGTH_MIN, FOG_STRENGTH_STEP};
use crate::config::DEFAULT_PRESET;
use crate::input::{Input, Mouse};
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};
use crate::ui::font::{draw_text, draw_text_centered, text_size};
use crate::ui::widgets::{row, Button};

use self::text_field::{FieldColors, FieldEvent, TextField};

// --- Look (switch to the shared UI theme once merged) -----------------------
const BACKGROUND: Color = Color::rgb(20, 22, 32);
const TEXT: Color = Color::rgb(235, 235, 235);
const TEXT_DIM: Color = Color::rgb(150, 150, 150);
const TEXT_ERROR: Color = Color::rgb(255, 110, 110);
const TEXT_OK: Color = Color::rgb(120, 230, 140);
const CARD_BG: Color = Color::rgb(32, 35, 50);
const CARD_BORDER: Color = Color::rgb(70, 76, 100);
const SELECTED_BORDER: Color = Color::rgb(255, 200, 80);
const CHECKER_LIGHT: Color = Color::rgb(90, 90, 90);
const CHECKER_DARK: Color = Color::rgb(60, 60, 60);
const PREVIEW_BORDER: Color = Color::rgb(140, 150, 190);
const FIELD_COLORS: FieldColors = FieldColors {
    background: Color::rgb(12, 13, 20),
    border: Color::rgb(90, 96, 125),
    focus: Color::rgb(255, 200, 80),
    text: Color::rgb(235, 235, 235),
    placeholder: Color::rgb(110, 110, 110),
    placeholder_text: "type a name",
};
const CHANNEL_COLORS: [Color; 3] = [Color::rgb(220, 70, 70), Color::rgb(70, 200, 90), Color::rgb(80, 120, 235)];
const PALETTE: [Color; 16] = [
    Color::rgb(0, 0, 0),
    Color::rgb(255, 255, 255),
    Color::rgb(128, 128, 128),
    Color::rgb(64, 64, 64),
    Color::rgb(200, 40, 40),
    Color::rgb(240, 140, 30),
    Color::rgb(255, 215, 0),
    Color::rgb(30, 160, 60),
    Color::rgb(0, 130, 71),
    Color::rgb(40, 180, 200),
    Color::rgb(135, 206, 235),
    Color::rgb(38, 98, 199),
    Color::rgb(12, 20, 69),
    Color::rgb(120, 60, 170),
    Color::rgb(92, 60, 44),
    Color::rgb(180, 140, 90),
];

// --- Metrics (framebuffer pixels, the framebuffer is 480 px high) -----------
const TITLE_SCALE: i32 = 2;
const TEXT_SCALE: i32 = 1;
const MARGIN: i32 = 8;
const GAP: i32 = 6;
const ROW_H: i32 = 16;
const SMALL_H: i32 = 14;
const CARD_H: i32 = 92;
const THUMB: i32 = 52;
const CARD_COLUMNS: usize = 4;
const LEFT_RATIO: f64 = 0.6;
const COLOR_PANEL_H: i32 = 104;
/// Below this card width the card buttons get short labels.
const WIDE_CARD: i32 = 112;

const NAME_MAX_LEN: usize = 24;
/// Frames during which input is ignored after the blocking file dialog.
const FRAMES_AFTER_DIALOG: u8 = 3;

/// Which colour the colour panel edits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ColorTarget {
    Slot(TextureSlot),
    Sun,
    Clouds,
}

impl ColorTarget {
    fn label(self) -> &'static str {
        match self {
            ColorTarget::Slot(s) => s.label(),
            ColorTarget::Sun => "Sun",
            ColorTarget::Clouds => "Clouds",
        }
    }

    fn get_mut(self, c: &mut PresetColors) -> &mut Color {
        match self {
            ColorTarget::Slot(TextureSlot::Wall) => &mut c.wall,
            ColorTarget::Slot(TextureSlot::Floor) => &mut c.floor,
            ColorTarget::Slot(TextureSlot::Ceiling) => &mut c.ceiling,
            ColorTarget::Slot(TextureSlot::Grid) => &mut c.grid,
            ColorTarget::Slot(TextureSlot::Coin) => &mut c.coin,
            ColorTarget::Slot(TextureSlot::End) => &mut c.end,
            ColorTarget::Slot(TextureSlot::Sky) => &mut c.sky,
            ColorTarget::Sun => &mut c.sun,
            ColorTarget::Clouds => &mut c.clouds,
        }
    }

    fn get(self, c: &PresetColors) -> Color {
        let mut c = *c;
        *self.get_mut(&mut c)
    }
}

/// Actions that need a second click when they would lose something.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Confirm {
    Back,
    Load,
    Delete,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MsgKind {
    Info,
    Ok,
    Error,
}

pub struct PresetEditorScreen {
    /// False until the first update after entering the screen.
    active: bool,
    /// Working copy.
    preset: Preset,
    /// Id of the user preset being edited (saved over), `None` for a new one.
    editing: Option<String>,
    name: TextField,
    /// "Start from" choice: 0 = empty preset, `i + 1` = library entry `i`.
    base: usize,
    selected: ColorTarget,
    /// RGB channel being dragged on its slider.
    dragging: Option<usize>,
    message: Option<(String, MsgKind)>,
    dirty: bool,
    confirm: Option<Confirm>,
    ignore_frames: u8,
    /// Folder of the last imported image (the dialog reopens there).
    last_dir: Option<PathBuf>,
    started: Instant,
}

struct Card {
    slot: TextureSlot,
    rect: Rect,
    thumb: Rect,
    import: Button,
    clear: Button,
}

struct Channel {
    minus: Button,
    track: Rect,
    plus: Button,
}

/// Every widget of the screen for a framebuffer size (rebuilt each frame).
struct Layout {
    title: Rect,
    name_label: (i32, i32),
    name_field: Rect,
    name_hint: (i32, i32),
    base_label: (i32, i32),
    base_prev: Button,
    base_box: Rect,
    base_next: Button,
    base_load: Button,
    cards: Vec<Card>,
    extras: Rect,
    sun: Rect,
    clouds: Rect,
    sun_toggle: Button,
    clouds_toggle: Button,
    color_panel: Rect,
    from_texture: Button,
    channels: Vec<Channel>,
    big_swatch: Rect,
    palette: Vec<Rect>,
    outside: Button,
    fog: Button,
    fog_less: Button,
    fog_value: Rect,
    fog_more: Button,
    preview: Rect,
    hints_y: i32,
    message_y: i32,
    save: Button,
    delete: Button,
    open_folder: Button,
    back: Button,
}

fn text_x_after(x: i32, text: &str) -> i32 {
    x + text_size(text, TEXT_SCALE).0 + GAP
}

impl PresetEditorScreen {
    pub fn new() -> PresetEditorScreen {
        PresetEditorScreen {
            active: false,
            preset: Preset::builtin(""),
            editing: None,
            name: TextField::new("", NAME_MAX_LEN),
            base: 0,
            selected: ColorTarget::Slot(TextureSlot::Wall),
            dragging: None,
            message: None,
            dirty: false,
            confirm: None,
            ignore_frames: 0,
            last_dir: None,
            started: Instant::now(),
        }
    }

    // --- State helpers ------------------------------------------------------

    fn enter(&mut self, ctx: &AppContext) {
        self.active = true;
        let index = ctx.preset_index();
        self.base = index + 1;
        self.start_from(ctx, Some(index), ctx.preset.clone());
        self.message = Some(("Click a card to edit its colour, Import to use your own image".to_string(), MsgKind::Info));
    }

    /// Replaces the working copy. A user preset is edited in place, anything
    /// else becomes a new preset with a free name.
    fn start_from(&mut self, ctx: &AppContext, index: Option<usize>, preset: Preset) {
        match index.and_then(|i| ctx.presets.entries.get(i)) {
            Some(e) if e.user => {
                self.editing = Some(e.name.clone());
                self.name.set_text(preset.name.clone());
            }
            Some(e) => {
                self.editing = None;
                self.name.set_text(suggest_name(ctx, &e.name));
            }
            None => {
                self.editing = None;
                self.name.set_text(suggest_name(ctx, "preset"));
            }
        }
        self.preset = preset;
        self.name.focused = false;
        self.selected = ColorTarget::Slot(TextureSlot::Wall);
        self.dirty = false;
        self.confirm = None;
        self.dragging = None;
    }

    fn base_count(ctx: &AppContext) -> usize {
        ctx.presets.len() + 1
    }

    fn base_name(&self, ctx: &AppContext) -> String {
        match self.base.checked_sub(1).and_then(|i| ctx.presets.entries.get(i)) {
            Some(e) if e.user => format!("{} (yours)", e.name),
            Some(e) => e.name.clone(),
            None => "(empty)".to_string(),
        }
    }

    fn set_message(&mut self, text: impl Into<String>, kind: MsgKind) {
        self.message = Some((text.into(), kind));
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.confirm = None;
    }

    /// Second click of a destructive action? Otherwise arms the confirmation.
    fn confirmed(&mut self, action: Confirm, prompt: &str) -> bool {
        if self.confirm == Some(action) {
            self.confirm = None;
            true
        } else {
            self.confirm = Some(action);
            self.set_message(prompt, MsgKind::Error);
            false
        }
    }

    fn leave(&mut self) -> Transition {
        self.active = false;
        self.name.focused = false;
        Transition::To(AppState::Menu)
    }

    // --- Layout -------------------------------------------------------------

    fn layout(&self, fb_size: (usize, usize)) -> Layout {
        let (w, h) = (fb_size.0 as i32, fb_size.1 as i32);
        let left_w = ((w as f64 * LEFT_RATIO) as i32 - MARGIN).max(200);
        let lx = MARGIN;
        let rx = lx + left_w + 2 * MARGIN;
        let rw = (w - rx - MARGIN).max(80);

        let title = Rect::new(lx, MARGIN, left_w, 8 * TITLE_SCALE);

        // name row
        let y = title.bottom() + 8;
        let name_label = (lx, y + (ROW_H - 8) / 2);
        let fx = text_x_after(lx, "Start from:");
        let name_field = Rect::new(fx, y, ((left_w - (fx - lx)) * 11 / 20).max(80), ROW_H);
        let name_hint = (name_field.right() + GAP, y + (ROW_H - 8) / 2);

        // base row
        let y = name_field.bottom() + GAP;
        let base_label = (lx, y + (ROW_H - 8) / 2);
        let load_w = 48;
        let base_prev = Button::new("<", Rect::new(fx, y, ROW_H, ROW_H), TEXT_SCALE);
        let base_next_x = lx + left_w - load_w - GAP - ROW_H;
        let base_box = Rect::new(base_prev.rect.right() + 2, y, base_next_x - 2 - base_prev.rect.right() - 2, ROW_H);
        let base_next = Button::new(">", Rect::new(base_next_x, y, ROW_H, ROW_H), TEXT_SCALE);
        let base_load = Button::new("Load", Rect::new(lx + left_w - load_w, y, load_w, ROW_H), TEXT_SCALE);

        // texture cards (4 x 2, the 8th cell holds the sun/cloud colours)
        let cards_y = y + ROW_H + GAP + 2;
        let card_w = (left_w - GAP * (CARD_COLUMNS as i32 - 1)) / CARD_COLUMNS as i32;
        let cell = |i: usize| {
            let (c, r) = ((i % CARD_COLUMNS) as i32, (i / CARD_COLUMNS) as i32);
            Rect::new(lx + c * (card_w + GAP), cards_y + r * (CARD_H + GAP), card_w, CARD_H)
        };
        let wide = card_w >= WIDE_CARD;
        let cards = TextureSlot::ALL
            .iter()
            .enumerate()
            .map(|(i, &slot)| {
                let rect = cell(i);
                let t = THUMB.min(rect.w / 2 - 6).min(CARD_H - 16 - SMALL_H - 12).max(8);
                let thumb = Rect::new(rect.x + 4, rect.y + 16, t, t);
                let buttons = Rect::new(rect.x + 4, rect.bottom() - 4 - SMALL_H, rect.w - 8, SMALL_H);
                let import_w = buttons.w * 11 / 20;
                let import = Button::new(
                    if wide { "Import" } else { "Load" },
                    Rect::new(buttons.x, buttons.y, import_w, SMALL_H),
                    TEXT_SCALE,
                );
                let clear = Button::new(
                    if wide { "Clear" } else { "x" },
                    Rect::new(buttons.x + import_w + 4, buttons.y, buttons.w - import_w - 4, SMALL_H),
                    TEXT_SCALE,
                )
                .enabled(self.preset.textures.get(slot).is_some());
                Card { slot, rect, thumb, import, clear }
            })
            .collect();
        let extras = cell(TextureSlot::ALL.len());
        let sw = Rect::new(extras.x + 4, extras.y + 16, extras.w - 8, (CARD_H - 16 - 4 - 4) / 2);
        // each swatch shares its row with an on/off toggle
        let toggle_w = 32;
        let swatch_w = sw.w - toggle_w - 4;
        let sun = Rect::new(sw.x, sw.y, swatch_w, sw.h);
        let clouds = Rect::new(sw.x, sw.bottom() + 4, swatch_w, sw.h);
        let toggle = |r: Rect, on: bool| {
            Button::new(on_off(on), Rect::new(r.right() + 4, r.y, toggle_w, r.h), TEXT_SCALE).enabled(self.preset.outside)
        };
        let sun_toggle = toggle(sun, self.preset.sun);
        let clouds_toggle = toggle(clouds, self.preset.clouds);

        // colour panel
        let color_panel = Rect::new(lx, cell(CARD_COLUMNS).bottom() + GAP + 2, left_w, COLOR_PANEL_H);
        let p = color_panel.inset(6);
        let target_tex = match self.selected {
            ColorTarget::Slot(s) => self.preset.textures.get(s).is_some(),
            _ => false,
        };
        let from_texture =
            Button::new("From texture", Rect::new(p.right() - 104, p.y, 104, SMALL_H), TEXT_SCALE).enabled(target_tex);
        let sliders_w = p.w * 7 / 10;
        let channels = (0..3)
            .map(|i| {
                let y = p.y + 20 + i * 18;
                let minus = Button::new("-", Rect::new(p.x + 12, y, SMALL_H, SMALL_H), TEXT_SCALE);
                let plus_x = p.x + sliders_w - 32 - SMALL_H;
                let plus = Button::new("+", Rect::new(plus_x, y, SMALL_H, SMALL_H), TEXT_SCALE);
                let track = Rect::new(minus.rect.right() + 4, y + 2, plus_x - 4 - minus.rect.right() - 4, SMALL_H - 4);
                Channel { minus, track, plus }
            })
            .collect();
        let big_swatch = Rect::new(p.x + sliders_w + GAP, p.y + 20, p.w - sliders_w - GAP, 3 * 18 - 4);
        let palette_row = Rect::new(p.x, p.y + 20 + 3 * 18 + 4, p.w, SMALL_H);
        let palette = row(palette_row, 3, PALETTE.len());

        // right column: flags + preview
        let flags = row(Rect::new(rx, name_field.y, rw, ROW_H), GAP, 3);
        let outside = Button::new(format!("Outside: {}", on_off(self.preset.outside)), flags[0], TEXT_SCALE);
        let fog = Button::new(format!("Fog: {}", on_off(self.preset.fog)), flags[1], TEXT_SCALE).enabled(self.preset.outside);
        // fog strength: [-] value [+]
        let s = flags[2];
        let fog_on = self.preset.outside && self.preset.fog;
        let fog_less = Button::new("-", Rect::new(s.x, s.y, ROW_H, ROW_H), TEXT_SCALE)
            .enabled(fog_on && self.preset.fog_strength > FOG_STRENGTH_MIN + 1e-6);
        let fog_more = Button::new("+", Rect::new(s.right() - ROW_H, s.y, ROW_H, ROW_H), TEXT_SCALE)
            .enabled(fog_on && self.preset.fog_strength < 1.0 - 1e-6);
        let fog_value = Rect::new(fog_less.rect.right() + 2, s.y, fog_more.rect.x - 2 - fog_less.rect.right() - 2, ROW_H);
        let preview_y = base_box.y;
        let preview_h = (rw * 3 / 4).min(color_panel.bottom() - preview_y - 40).max(40);
        let preview = Rect::new(rx, preview_y, rw, preview_h);

        // bottom bar
        let bar_y = h - MARGIN - ROW_H;
        let bar = row(Rect::new(lx, bar_y, w - 2 * MARGIN, ROW_H), GAP, 4);
        let save = Button::new("Save", bar[0], TEXT_SCALE);
        let delete = Button::new("Delete", bar[1], TEXT_SCALE).enabled(self.editing.is_some());
        let open_folder = Button::new("Open presets folder", bar[2], TEXT_SCALE);
        let back = Button::new("Back", bar[3], TEXT_SCALE);

        Layout {
            title,
            name_label,
            name_field,
            name_hint,
            base_label,
            base_prev,
            base_box,
            base_next,
            base_load,
            cards,
            extras,
            sun,
            clouds,
            sun_toggle,
            clouds_toggle,
            color_panel,
            from_texture,
            channels,
            big_swatch,
            palette,
            outside,
            fog,
            fog_less,
            fog_value,
            fog_more,
            preview,
            hints_y: preview.bottom() + 6,
            message_y: bar_y - 8 - 8,
            save,
            delete,
            open_folder,
            back,
        }
    }

    // --- Update -------------------------------------------------------------

    pub fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition {
        if !self.active {
            self.enter(ctx);
        }
        if self.ignore_frames > 0 {
            // stray events from the file dialog (release of the double click, Enter...)
            self.ignore_frames -= 1;
            return Transition::None;
        }
        let l = self.layout(fb_size);
        let (mx, my) = input.mouse_pos();
        let clicked = input.mouse_clicked(Mouse::Left);

        // name field first: it owns the keyboard while focused
        let had_focus = self.name.focused;
        if self.name.update(input, l.name_field) == FieldEvent::Changed {
            self.changed();
        }
        if !had_focus && !self.name.focused {
            if input.was_pressed(Scancode::Escape) {
                return self.back(ctx);
            }
            let ctrl = input.is_down(Scancode::LCtrl) || input.is_down(Scancode::RCtrl);
            if ctrl && input.was_pressed(Scancode::S) {
                self.save(ctx);
            }
        }

        // start from
        let n = Self::base_count(ctx);
        if l.base_prev.clicked(input) {
            self.base = (self.base + n - 1) % n;
            self.confirm = None;
        }
        if l.base_next.clicked(input) {
            self.base = (self.base + 1) % n;
            self.confirm = None;
        }
        if l.base_load.clicked(input) && (!self.dirty || self.confirmed(Confirm::Load, "Unsaved changes: click Load again to discard them")) {
            let (index, preset) = match self.base.checked_sub(1) {
                Some(i) if i < ctx.presets.len() => (Some(i), ctx.presets.load(i, &ctx.common)),
                _ => {
                    let mut p = Preset::builtin("");
                    p.textures.end = ctx.common.end.clone();
                    (None, p)
                }
            };
            self.start_from(ctx, index, preset);
            self.set_message(format!("Loaded {}", self.base_name(ctx)), MsgKind::Info);
        }

        // flags
        if l.outside.clicked(input) {
            self.preset.outside = !self.preset.outside;
            self.changed();
        }
        if l.fog.clicked(input) {
            self.preset.fog = !self.preset.fog;
            self.changed();
        }
        for (button, delta) in [(&l.fog_less, -FOG_STRENGTH_STEP), (&l.fog_more, FOG_STRENGTH_STEP)] {
            if button.clicked(input) {
                // stay on the 0.1 grid
                let s = ((self.preset.fog_strength + delta) * 10.0).round() / 10.0;
                self.preset.fog_strength = s.clamp(FOG_STRENGTH_MIN, 1.0);
                self.changed();
            }
        }
        if l.fog_value.contains(mx, my) && input.wheel() != 0 && self.preset.outside && self.preset.fog {
            let s = ((self.preset.fog_strength + input.wheel() as f64 * FOG_STRENGTH_STEP) * 10.0).round() / 10.0;
            self.preset.fog_strength = s.clamp(FOG_STRENGTH_MIN, 1.0);
            self.changed();
        }
        if l.sun_toggle.clicked(input) {
            self.preset.sun = !self.preset.sun;
            self.selected = ColorTarget::Sun;
            self.changed();
        }
        if l.clouds_toggle.clicked(input) {
            self.preset.clouds = !self.preset.clouds;
            self.selected = ColorTarget::Clouds;
            self.changed();
        }

        // texture cards
        for card in &l.cards {
            if card.import.clicked(input) {
                self.import(card.slot);
                return Transition::None;
            }
            if card.clear.clicked(input) {
                *self.preset.textures.get_mut(card.slot) = None;
                self.selected = ColorTarget::Slot(card.slot);
                self.changed();
                let note = if card.slot == TextureSlot::End { " (the common flag will be used)" } else { "" };
                self.set_message(format!("{} texture cleared{}", card.slot.label(), note), MsgKind::Info);
            } else if clicked && card.rect.contains(mx, my) && !card.import.rect.contains(mx, my) {
                self.selected = ColorTarget::Slot(card.slot);
            }
        }
        if clicked && l.sun.contains(mx, my) {
            self.selected = ColorTarget::Sun;
        }
        if clicked && l.clouds.contains(mx, my) {
            self.selected = ColorTarget::Clouds;
        }

        self.update_color_panel(&l, input);

        // bottom bar
        if l.save.clicked(input) {
            self.save(ctx);
        }
        if l.delete.clicked(input) {
            self.delete(ctx);
        }
        if l.open_folder.clicked(input) {
            self.open_folder(ctx);
        }
        if l.back.clicked(input) {
            return self.back(ctx);
        }
        Transition::None
    }

    fn update_color_panel(&mut self, l: &Layout, input: &Input) {
        let (mx, my) = input.mouse_pos();
        let target = self.selected;
        let mut color = target.get(&self.preset.colors);
        let before = color;
        let fine = input.is_down(Scancode::LShift) || input.is_down(Scancode::RShift);
        let step = if fine { 1 } else { 8 };

        if !input.mouse_down(Mouse::Left) {
            self.dragging = None;
        }
        for (i, ch) in l.channels.iter().enumerate() {
            let value = channel_mut(&mut color, i);
            if ch.minus.clicked(input) {
                *value = value.saturating_sub(step);
            }
            if ch.plus.clicked(input) {
                *value = value.saturating_add(step);
            }
            let row = Rect::new(ch.minus.rect.x, ch.track.y - 2, ch.plus.rect.right() - ch.minus.rect.x, ch.track.h + 4);
            if row.contains(mx, my) && input.wheel() != 0 {
                *value = (*value as i32 + input.wheel() * if fine { 1 } else { 4 }).clamp(0, 255) as u8;
            }
            if input.mouse_clicked(Mouse::Left) && ch.track.inset(-2).contains(mx, my) {
                self.dragging = Some(i);
            }
            if self.dragging == Some(i) {
                let t = (mx - ch.track.x) as f64 / (ch.track.w - 1).max(1) as f64;
                *value = (t.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
        if input.mouse_clicked(Mouse::Left) {
            if let Some(i) = l.palette.iter().position(|r| r.contains(mx, my)) {
                color = PALETTE[i];
            }
        }
        if l.from_texture.clicked(input) {
            if let ColorTarget::Slot(s) = target {
                if let Some(t) = self.preset.textures.get(s) {
                    color = t.average_color();
                }
            }
        }
        if color != before {
            *target.get_mut(&mut self.preset.colors) = color;
            self.changed();
        }
    }

    /// Native file dialog (blocking) then load into `slot`.
    fn import(&mut self, slot: TextureSlot) {
        self.selected = ColorTarget::Slot(slot);
        self.name.focused = false;
        self.confirm = None;
        let path = import::pick_image(slot, self.last_dir.as_deref());
        self.ignore_frames = FRAMES_AFTER_DIALOG;
        let path = match path {
            Some(p) => p,
            None => return,
        };
        self.last_dir = path.parent().map(|p| p.to_path_buf());
        let fallback = ColorTarget::Slot(slot).get(&self.preset.colors);
        match import::load_for_slot(&path, slot, fallback) {
            Ok(imported) => {
                let t = &imported.texture;
                let mut msg = format!("{}: imported {}x{}", slot.label(), t.width, t.height);
                if let Some((w, h)) = imported.downscaled_from {
                    msg += &format!(" (downscaled from {}x{})", w, h);
                }
                // flat colour used where the texture is not drawn (minimap, fallbacks)
                *ColorTarget::Slot(slot).get_mut(&mut self.preset.colors) = t.average_color();
                if slot == TextureSlot::Sky {
                    msg += if self.preset.outside { "" } else { ", turn Outside on to see it" };
                }
                *self.preset.textures.get_mut(slot) = Some(imported.texture);
                self.changed();
                self.set_message(msg, MsgKind::Ok);
            }
            Err(e) => self.set_message(e, MsgKind::Error),
        }
    }

    fn save(&mut self, ctx: &mut AppContext) {
        self.name.focused = false;
        self.confirm = None;
        self.preset.name = self.name.text.trim().to_string();
        match ctx.presets.save_as(&self.preset, self.editing.as_deref()) {
            Ok(dir) => {
                let id = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if let Some(i) = ctx.presets.index_of(&id) {
                    ctx.select_preset(i);
                    self.base = i + 1;
                }
                self.editing = Some(id.clone());
                self.dirty = false;
                ctx.message = Some(format!("Preset '{}' saved", self.preset.name));
                self.set_message(format!("Saved to {}", dir.display()), MsgKind::Ok);
            }
            Err(e) => self.set_message(format!("Cannot save: {}", e), MsgKind::Error),
        }
    }

    fn delete(&mut self, ctx: &mut AppContext) {
        let id = match self.editing.clone() {
            Some(id) => id,
            None => return,
        };
        if !self.confirmed(Confirm::Delete, &format!("Delete '{}' from disk? Click Delete again", id)) {
            return;
        }
        match ctx.presets.delete(&id) {
            Ok(()) => {
                // keep `settings.preset` valid
                let fallback = ctx.presets.index_of(&ctx.settings.preset).or_else(|| ctx.presets.index_of(DEFAULT_PRESET)).unwrap_or(0);
                ctx.select_preset(fallback);
                self.editing = None;
                self.base = 0;
                self.dirty = true;
                self.set_message(format!("Deleted '{}' (still open here: Save to recreate it)", id), MsgKind::Info);
            }
            Err(e) => self.set_message(format!("Cannot delete: {}", e), MsgKind::Error),
        }
    }

    fn open_folder(&mut self, ctx: &AppContext) {
        let dir = match &self.editing {
            Some(id) => ctx.presets.user_dir.join(id),
            None => ctx.presets.user_dir.clone(),
        };
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.set_message(format!("{}: {}", dir.display(), e), MsgKind::Error);
            return;
        }
        match std::process::Command::new("explorer").arg(&dir).spawn() {
            Ok(_) => self.set_message(format!("Opened {}", dir.display()), MsgKind::Info),
            Err(e) => self.set_message(format!("Cannot open {}: {}", dir.display(), e), MsgKind::Error),
        }
    }

    fn back(&mut self, _ctx: &AppContext) -> Transition {
        if self.dirty && !self.confirmed(Confirm::Back, "Unsaved changes: press Back again to discard them") {
            return Transition::None;
        }
        self.leave()
    }

    // --- Draw ---------------------------------------------------------------

    pub fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input) {
        let l = self.layout((fb.width(), fb.height()));
        let time = self.started.elapsed().as_secs_f64();
        fb.clear(BACKGROUND);

        let title = match &self.editing {
            Some(id) => format!("Edit preset: {}", id),
            None => "New preset".to_string(),
        };
        draw_text(fb, &title, l.title.x, l.title.y, TITLE_SCALE, TEXT);
        if self.dirty {
            let x = l.title.x + text_size(&title, TITLE_SCALE).0 + GAP;
            draw_text(fb, "(unsaved)", x, l.title.y + 4, TEXT_SCALE, TEXT_DIM);
        }

        // name + validation
        draw_text(fb, "Name:", l.name_label.0, l.name_label.1, TEXT_SCALE, TEXT);
        self.name.draw(fb, l.name_field, TEXT_SCALE, FIELD_COLORS, time);
        let (hint, color) = match ctx.presets.check_name(&self.name.text, self.editing.as_deref()) {
            Ok(id) if self.editing.as_deref() == Some(id.as_str()) => (format!("updates presets/{}/", id), TEXT_DIM),
            // a new name while editing saves a copy, the original is kept
            Ok(id) => (format!("new: presets/{}/", id), TEXT_DIM),
            Err(e) => (e, TEXT_ERROR),
        };
        let hint_max = l.cards[CARD_COLUMNS - 1].rect.right() - l.name_hint.0;
        draw_text(fb, &fit_text(&hint, hint_max), l.name_hint.0, l.name_hint.1, TEXT_SCALE, color);

        // start from
        draw_text(fb, "Start from:", l.base_label.0, l.base_label.1, TEXT_SCALE, TEXT);
        l.base_prev.draw(fb, input);
        fb.fill_rect(l.base_box, FIELD_COLORS.background);
        draw_text_centered(fb, &fit_text(&self.base_name(ctx), l.base_box.w - 4), l.base_box, TEXT_SCALE, TEXT);
        l.base_next.draw(fb, input);
        l.base_load.draw(fb, input);

        for card in &l.cards {
            self.draw_card(ctx, fb, input, card);
        }
        self.draw_extras(fb, input, &l);
        self.draw_color_panel(fb, input, &l);

        // flags + preview
        l.outside.draw(fb, input);
        l.fog.draw(fb, input);
        l.fog_less.draw(fb, input);
        l.fog_more.draw(fb, input);
        fb.fill_rect(l.fog_value, FIELD_COLORS.background);
        let strength = format!("{}%", (self.preset.fog_strength * 100.0).round() as i32);
        let fog_on = self.preset.outside && self.preset.fog;
        draw_text_centered(fb, &fit_text(&strength, l.fog_value.w - 2), l.fog_value, TEXT_SCALE, if fog_on { TEXT } else { TEXT_DIM });
        preview::draw_preview(fb, l.preview, &self.preset, time);
        fb.draw_rect(l.preview.inset(-1), PREVIEW_BORDER);
        let hints = [
            "Images are scaled down to 256 px.",
            "Grid, coin and end keep transparency.",
            "Sky is shown when Outside is on.",
            "Shift + / - / wheel: fine colour steps.",
        ];
        for (i, h) in hints.iter().enumerate() {
            let y = l.hints_y + i as i32 * 11;
            if y + 8 < l.message_y {
                draw_text(fb, &fit_text(h, l.preview.w), l.preview.x, y, TEXT_SCALE, TEXT_DIM);
            }
        }

        // message + bottom bar
        if let Some((msg, kind)) = &self.message {
            let color = match kind {
                MsgKind::Info => TEXT_DIM,
                MsgKind::Ok => TEXT_OK,
                MsgKind::Error => TEXT_ERROR,
            };
            draw_text(fb, &fit_text(msg, fb.width() as i32 - 2 * MARGIN), MARGIN, l.message_y, TEXT_SCALE, color);
        }
        l.save.draw(fb, input);
        l.delete.draw(fb, input);
        l.open_folder.draw(fb, input);
        l.back.draw(fb, input);
    }

    fn draw_card(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input, card: &Card) {
        let target = ColorTarget::Slot(card.slot);
        let selected = self.selected == target;
        fb.fill_rect(card.rect, CARD_BG);
        fb.draw_rect(card.rect, if selected { SELECTED_BORDER } else { CARD_BORDER });
        draw_text(fb, card.slot.label(), card.rect.x + 4, card.rect.y + 4, TEXT_SCALE, if selected { SELECTED_BORDER } else { TEXT });

        let color = target.get(&self.preset.colors);
        let texture = self.preset.textures.get(card.slot);
        draw_checker(fb, card.thumb);
        match texture {
            Some(t) => fb.blit_texture(t, card.thumb),
            None => {
                fb.fill_rect(card.thumb, color);
                // the end flag falls back to the common image
                let fallback = if card.slot == TextureSlot::End { ctx.common.end.as_ref() } else { None };
                match fallback {
                    Some(t) => {
                        fb.blit_texture(t, card.thumb);
                        draw_text_centered(fb, "default", card.thumb, TEXT_SCALE, contrast(color));
                    }
                    None => draw_text_centered(fb, "none", card.thumb, TEXT_SCALE, contrast(color)),
                }
            }
        }
        fb.draw_rect(card.thumb.inset(-1), CARD_BORDER);

        // info column: colour swatch, size, alpha
        let ix = card.thumb.right() + 5;
        let iw = card.rect.right() - 4 - ix;
        let swatch = Rect::new(ix, card.thumb.y, iw, 12);
        fb.fill_rect(swatch, color);
        fb.draw_rect(swatch, CARD_BORDER);
        let mut y = swatch.bottom() + 4;
        let mut info = |fb: &mut FrameBuffer, s: &str, c: Color| {
            draw_text(fb, &fit_text(s, iw), ix, y, TEXT_SCALE, c);
            y += 11;
        };
        match texture {
            Some(t) => {
                info(fb, &format!("{}x{}", t.width, t.height), TEXT_DIM);
                if import::keeps_alpha(card.slot) && t.has_transparency() {
                    info(fb, "alpha", TEXT_DIM);
                }
            }
            None => info(fb, "colour", TEXT_DIM),
        }
        card.import.draw(fb, input);
        card.clear.draw(fb, input);
    }

    fn draw_extras(&self, fb: &mut FrameBuffer, input: &Input, l: &Layout) {
        fb.fill_rect(l.extras, CARD_BG);
        let selected = matches!(self.selected, ColorTarget::Sun | ColorTarget::Clouds);
        fb.draw_rect(l.extras, if selected { SELECTED_BORDER } else { CARD_BORDER });
        draw_text(fb, "Sky extras", l.extras.x + 4, l.extras.y + 4, TEXT_SCALE, TEXT);
        for (rect, target) in [(l.sun, ColorTarget::Sun), (l.clouds, ColorTarget::Clouds)] {
            let c = target.get(&self.preset.colors);
            fb.fill_rect(rect, c);
            fb.draw_rect(rect, if self.selected == target { SELECTED_BORDER } else { CARD_BORDER });
            draw_text_centered(fb, target.label(), rect, TEXT_SCALE, contrast(c));
        }
        l.sun_toggle.draw(fb, input);
        l.clouds_toggle.draw(fb, input);
    }

    fn draw_color_panel(&self, fb: &mut FrameBuffer, input: &Input, l: &Layout) {
        let panel = l.color_panel;
        fb.fill_rect(panel, CARD_BG);
        fb.draw_rect(panel, CARD_BORDER);
        let p = panel.inset(6);
        let color = self.selected.get(&self.preset.colors);
        draw_text(fb, &format!("{} colour  {}", self.selected.label(), color), p.x, p.y + 3, TEXT_SCALE, TEXT);
        l.from_texture.draw(fb, input);

        for (i, ch) in l.channels.iter().enumerate() {
            let value = channel(color, i);
            draw_text(fb, ["R", "G", "B"][i], p.x, ch.minus.rect.y + 3, TEXT_SCALE, CHANNEL_COLORS[i]);
            ch.minus.draw(fb, input);
            ch.plus.draw(fb, input);
            // gradient of this channel with the other two fixed
            for x in 0..ch.track.w {
                let mut c = color;
                *channel_mut(&mut c, i) = (x * 255 / (ch.track.w - 1).max(1)) as u8;
                fb.fill_rect(Rect::new(ch.track.x + x, ch.track.y, 1, ch.track.h), c);
            }
            fb.draw_rect(ch.track, CARD_BORDER);
            let kx = ch.track.x + value as i32 * (ch.track.w - 1) / 255;
            fb.fill_rect(Rect::new(kx - 1, ch.track.y - 2, 3, ch.track.h + 4), TEXT);
            draw_text(fb, &format!("{:3}", value), ch.plus.rect.right() + 4, ch.minus.rect.y + 3, TEXT_SCALE, TEXT);
        }
        fb.fill_rect(l.big_swatch, color);
        fb.draw_rect(l.big_swatch, CARD_BORDER);

        for (r, c) in l.palette.iter().zip(PALETTE.iter()) {
            fb.fill_rect(*r, *c);
            fb.draw_rect(*r, if *c == color { SELECTED_BORDER } else { CARD_BORDER });
        }
    }
}

/// First free display name based on `base` ("my cave", "my cave 2", ...).
fn suggest_name(ctx: &AppContext, base: &str) -> String {
    let base = format!("my {}", base);
    (1..100)
        .map(|i| if i == 1 { base.clone() } else { format!("{} {}", base, i) })
        .find(|n| ctx.presets.check_name(n, None).is_ok())
        .unwrap_or(base)
        .chars()
        .take(NAME_MAX_LEN)
        .collect()
}

fn on_off(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

fn channel(c: Color, i: usize) -> u8 {
    [c.r, c.g, c.b][i]
}

fn channel_mut(c: &mut Color, i: usize) -> &mut u8 {
    match i {
        0 => &mut c.r,
        1 => &mut c.g,
        _ => &mut c.b,
    }
}

/// Black or white, whichever reads best on `c`.
fn contrast(c: Color) -> Color {
    let luma = 299 * c.r as u32 + 587 * c.g as u32 + 114 * c.b as u32;
    if luma > 140_000 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

/// Truncates `s` with ".." so it fits in `max_w` pixels.
fn fit_text(s: &str, max_w: i32) -> String {
    let max_chars = (max_w / text_size("M", TEXT_SCALE).0).max(0) as usize;
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(2);
    s.chars().take(keep).collect::<String>() + ".."
}

/// Checkerboard behind thumbnails so transparency is visible.
fn draw_checker(fb: &mut FrameBuffer, r: Rect) {
    let step = 6;
    let mut y = r.y;
    while y < r.bottom() {
        let mut x = r.x;
        while x < r.right() {
            let dark = ((x - r.x) / step + (y - r.y) / step) % 2 == 0;
            let cell = Rect::new(x, y, step.min(r.right() - x), step.min(r.bottom() - y));
            fb.fill_rect(cell, if dark { CHECKER_DARK } else { CHECKER_LIGHT });
            x += step;
        }
        y += step;
    }
}
