// Textured renderer: the default `WorldRenderer`.
//
//  * walls: one texture column per ray (`wall_x` -> u), oriented so faces are
//    never mirrored, Y faces slightly darker;
//  * floor / ceiling: per-column floor casting below / above the wall with the
//    per-row distances computed once per frame;
//  * sky (outdoor presets): procedural gradient, or the preset `sky` texture;
//  * grids (`X`): see-through faces where the texture alpha decides;
//  * shading: `distance_shade` turned into a lookup table of integer ops
//    (indoor darkening, outdoor fog towards the sky colour).
//
// A missing texture slot falls back to a 1x1 texture of the slot's flat
// colour, so a preset without textures renders like `FlatRenderer`.
//
// Texture coordinates are fixed point (`FRAC_BITS` fractional bits) in the
// inner loops; nothing is allocated per column or pixel (buffers are reused).

use std::f64::consts::{PI, TAU};
use std::ops::Range;

use crate::assets::preset::{CommonAssets, Preset, PresetColors};
use crate::assets::texture::Texture;
use crate::config::{FOG_DISTANCE, MAX_RAY_DIST, SHADE_FACTOR, SHADE_STEP};
use crate::game::map::{Map, Tile};
use crate::render::color::{blend_u32, Color};
use crate::render::framebuffer::FrameBuffer;
use crate::render::raycaster::{Camera, ColumnHits, Side, View, WorldRenderer, SUN_ANGLE};

/// Shade lookup: entries per tile of distance, and table length (64 tiles).
const LUT_RES: f64 = 32.0;
const LUT_LEN: usize = 64 * 32;
/// Extra factor (/256) for faces hit on a Y side.
const SIDE_SHADE: u32 = 192;
/// Fractional bits of the vertical texture coordinate.
const FRAC_BITS: u32 = 24;
const FRAC_ONE: f64 = (1u64 << FRAC_BITS) as f64;
/// Fixed point scale of floor / ceiling world coordinates (16 fractional bits).
const PLANE_ONE: f64 = 65536.0;
const OPAQUE: u32 = 0xFF00_0000;
/// Alpha of the flat grid colour when the preset has no grid texture.
const FLAT_GRID_ALPHA: u8 = 160;

// Billboards, sizes and heights in tiles (floor at 0, eye at 0.5, ceiling at 1).
const COIN_SIZE: f64 = 0.4;
/// Height of the coin centre, bobbing by `COIN_BOB` at `COIN_BOB_SPEED` rad/s.
const COIN_HEIGHT: f64 = 0.32;
const COIN_BOB: f64 = 0.05;
const COIN_BOB_SPEED: f64 = 2.5;
/// Fake rotation: the width is scaled by cos(t * speed), mirrored on the back side.
const COIN_SPIN_SPEED: f64 = 1.8;
/// The end flag stands on the floor.
const END_SIZE: f64 = 0.8;
/// Sprites closer than this (tiles) are skipped (huge and behind the near plane).
const MIN_SPRITE_DEPTH: f64 = 0.05;

// Sky objects live at infinity: positions and sizes are angles (radians), so
// they turn with the camera and wrap over 360 degrees. Elevation is the angle
// above the horizon of the object's centre; on screen y = horizon - tan(e) * focal.
const SUN_ELEVATION: f64 = 0.30;
/// Angular diameter of the sun image (it includes a glow halo).
const SUN_SIZE: f64 = 0.26;
/// Sun and clouds are partly hidden by fog.
const SKY_FOG_ALPHA: u8 = 110;

struct CloudDef {
    /// Index into `common.clouds` (modulo their count).
    texture: usize,
    /// World angle at time 0.
    angle: f64,
    elevation: f64,
    /// Angular width (the height follows the image aspect ratio).
    width: f64,
    /// Drift in radians per second.
    speed: f64,
}

const CLOUDS: [CloudDef; 7] = [
    CloudDef { texture: 0, angle: 0.3, elevation: 0.26, width: 0.95, speed: 0.010 },
    CloudDef { texture: 1, angle: 1.3, elevation: 0.36, width: 0.75, speed: 0.014 },
    CloudDef { texture: 2, angle: 2.2, elevation: 0.18, width: 0.40, speed: 0.018 },
    CloudDef { texture: 0, angle: 3.1, elevation: 0.33, width: 0.80, speed: 0.008 },
    CloudDef { texture: 2, angle: 3.9, elevation: 0.40, width: 0.35, speed: 0.016 },
    CloudDef { texture: 1, angle: 4.6, elevation: 0.22, width: 0.85, speed: 0.011 },
    CloudDef { texture: 2, angle: 5.5, elevation: 0.30, width: 0.45, speed: 0.013 },
];

/// Per-pixel shading as integer ops: `rgb * mul / 256 + add / 256`.
/// Indoors `add` is 0 (darkening); with fog it is the sky colour weight.
#[derive(Clone, Copy, Default)]
struct ShadeOp {
    mul: u32,
    add_rb: u32,
    add_g: u32,
}

impl ShadeOp {
    /// Shades packed ARGB `p`, keeping its alpha.
    #[inline(always)]
    fn apply(self, p: u32) -> u32 {
        let rb = (((p & 0x00FF_00FF) * self.mul + self.add_rb) >> 8) & 0x00FF_00FF;
        let g = (((p & 0x0000_FF00) * self.mul + self.add_g) >> 8) & 0x0000_FF00;
        (p & 0xFF00_0000) | rb | g
    }
}

/// `distance_shade` as a table indexed by distance (front faces, then Y faces).
/// Same curve as the flat renderer but continuous instead of stepped.
#[derive(Default)]
struct Shading {
    key: Option<(bool, bool, Color)>,
    lut: Vec<ShadeOp>,
}

impl Shading {
    fn update(&mut self, preset: &Preset) {
        let key = (preset.outside, preset.fog, preset.colors.sky);
        if self.key == Some(key) {
            return;
        }
        self.key = Some(key);
        self.lut.clear();
        let fog = preset.outside && preset.fog;
        let sky = if fog { preset.colors.sky.to_u32() } else { 0 };
        for side in [256, SIDE_SHADE] {
            for i in 0..LUT_LEN {
                let d = i as f64 / LUT_RES;
                let f = if !preset.outside {
                    SHADE_FACTOR.powf(d / SHADE_STEP)
                } else if fog {
                    1.0 - d / FOG_DISTANCE
                } else {
                    1.0
                };
                let f = (f.clamp(0.0, 1.0) * 256.0).round() as u32;
                let inv = 256 - f;
                self.lut.push(ShadeOp { mul: f * side / 256, add_rb: (sky & 0x00FF_00FF) * inv, add_g: (sky & 0x0000_FF00) * inv });
            }
        }
    }

    #[inline]
    fn op(&self, dist: f64, y_side: bool) -> ShadeOp {
        let i = ((dist * LUT_RES) as usize).min(LUT_LEN - 1);
        self.lut[if y_side { i + LUT_LEN } else { i }]
    }
}

/// 1x1 textures of the preset flat colours, used for empty texture slots.
struct Fallbacks {
    colors: PresetColors,
    wall: Texture,
    floor: Texture,
    ceiling: Texture,
    grid: Texture,
    coin: Texture,
    end: Texture,
}

impl Fallbacks {
    fn new(c: &PresetColors) -> Fallbacks {
        let t = |color: Color| Texture::solid(1, 1, color);
        Fallbacks {
            colors: *c,
            wall: t(c.wall),
            floor: t(c.floor),
            ceiling: t(c.ceiling),
            grid: t(c.grid.with_alpha(FLAT_GRID_ALPHA)),
            coin: t(c.coin),
            end: t(c.end),
        }
    }
}

/// Sun and cloud images tinted with the preset colours (rebuilt when they change).
#[derive(Default)]
struct SkyArt {
    key: Option<(Color, Color)>,
    sun: Option<Texture>,
    clouds: Vec<Texture>,
}

impl SkyArt {
    fn update(&mut self, colors: &PresetColors, common: &CommonAssets) {
        let key = (colors.sun, colors.clouds);
        if self.key == Some(key) {
            return;
        }
        self.key = Some(key);
        // black is the "unset" default colour: keep the images as they are
        let tint = |c: Color| if c == Color::BLACK { Color::WHITE } else { c };
        self.sun = usable(&common.sun).map(|t| t.tinted(tint(colors.sun)));
        self.clouds = common.clouds.iter().filter(|t| t.width > 0 && t.height > 0).map(|t| t.tinted(tint(colors.clouds))).collect();
    }
}

/// Draws a sky image centred on world `angle` / `elevation`, `ang_w` radians
/// wide, into the sky rows `[0, rows)`. Columns are mapped by their own view
/// angle (`col_angle`), so the image wraps around and does not stretch at the
/// screen edges.
#[allow(clippy::too_many_arguments)]
fn draw_sky_object(px: &mut [u32], w: usize, rows: usize, cam: &Camera, col_angle: &[f64], tex: &Texture, angle: f64, elevation: f64, ang_w: f64, alpha: u8) {
    let hpx = ang_w * tex.height as f64 / tex.width as f64 * cam.focal;
    let top = cam.horizon - elevation.tan() * cam.focal - hpx / 2.0;
    let (y0, y1) = span(top, hpx, rows);
    if y0 >= y1 {
        return;
    }
    let (pos0, step) = tex_fixed(y0, top, hpx, tex.height);
    let (tw, last) = (tex.width, tex.height - 1);
    for (x, &a) in col_angle.iter().enumerate().take(w) {
        let rel = (cam.angle + a - angle + PI).rem_euclid(TAU) - PI;
        let u = rel / ang_w + 0.5;
        if !(0.0..1.0).contains(&u) {
            continue;
        }
        let tx = ((u * tw as f64) as usize).min(tw - 1);
        let mut pos = pos0;
        for p in px[y0 * w + x..].iter_mut().step_by(w).take(y1 - y0) {
            let ty = ((pos >> FRAC_BITS) as usize).min(last);
            *p = blend_u32(*p, tex.pixels[ty * tw + tx], alpha);
            pos += step;
        }
    }
}

fn usable(t: &Option<Texture>) -> Option<&Texture> {
    t.as_ref().filter(|t| t.width > 0 && t.height > 0 && t.pixels.len() >= t.width * t.height)
}

/// Texture of each slot for this frame (preset texture or flat fallback).
struct Slots<'a> {
    wall: &'a Texture,
    floor: &'a Texture,
    ceiling: &'a Texture,
    grid: &'a Texture,
    coin: &'a Texture,
    end: &'a Texture,
}

impl<'a> Slots<'a> {
    fn new(preset: &'a Preset, fallback: &'a Fallbacks) -> Slots<'a> {
        let t = &preset.textures;
        Slots {
            wall: usable(&t.wall).unwrap_or(&fallback.wall),
            floor: usable(&t.floor).unwrap_or(&fallback.floor),
            ceiling: usable(&t.ceiling).unwrap_or(&fallback.ceiling),
            grid: usable(&t.grid).unwrap_or(&fallback.grid),
            coin: usable(&t.coin).unwrap_or(&fallback.coin),
            end: usable(&t.end).unwrap_or(&fallback.end),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpriteKind {
    Coin,
    End,
}

/// A billboard clipped to the screen, ready to draw column by column.
struct Sprite {
    kind: SpriteKind,
    /// Perpendicular distance (sorting, occlusion).
    depth: f64,
    /// Visible columns `[x0, x1)` and rows `[y0, y1)`.
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
    /// Unclipped left edge and width in pixels (texture u per column).
    left: f64,
    width: f64,
    /// Back side of a spinning coin: texture mirrored.
    mirror: bool,
    pos: i64,
    step: i64,
    op: ShadeOp,
}

impl Sprite {
    /// The column `x` slice as a `Face` (same drawing code as grids).
    #[inline]
    fn column(&self, x: usize, tex: &Texture) -> Face {
        let u = (x as f64 + 0.5 - self.left) / self.width;
        Face { perp: self.depth, y0: self.y0, y1: self.y1, tx: face_tx(tex, u, self.mirror), pos: self.pos, step: self.step, op: self.op }
    }
}

/// Coins and end tiles within `MAX_RAY_DIST` of the camera, as visible
/// billboards sorted far to near.
fn collect_sprites(out: &mut Vec<Sprite>, view: &View, slots: &Slots, shading: &Shading, w: usize, h: usize) {
    out.clear();
    let cam = &view.camera;
    let map = view.map;
    if map.width == 0 || map.height == 0 {
        return;
    }
    let r = MAX_RAY_DIST.ceil() as i32;
    let (cx, cy) = (cam.x.floor() as i32, cam.y.floor() as i32);
    let (x_lo, x_hi) = ((cx - r).max(0), (cx + r).min(map.width as i32 - 1));
    let (y_lo, y_hi) = ((cy - r).max(0), (cy + r).min(map.height as i32 - 1));
    for ty in y_lo..=y_hi {
        for tx in x_lo..=x_hi {
            let (kind, tex) = match map.get(tx, ty) {
                Tile::Coin => (SpriteKind::Coin, slots.coin),
                Tile::End => (SpriteKind::End, slots.end),
                _ => continue,
            };
            let (wx, wy) = (tx as f64 + 0.5, ty as f64 + 0.5);
            let (sx, depth) = match cam.project(wx, wy) {
                Some(v) => v,
                None => continue,
            };
            if !(MIN_SPRITE_DEPTH..=MAX_RAY_DIST).contains(&depth) {
                continue;
            }
            // per-tile phase so coins do not all move in sync
            let phase = tx as f64 * 1.7 + ty as f64 * 2.9;
            let (size, z, spin) = match kind {
                SpriteKind::Coin => (
                    COIN_SIZE,
                    COIN_HEIGHT + COIN_BOB * (view.time * COIN_BOB_SPEED + phase).sin(),
                    (view.time * COIN_SPIN_SPEED + phase).cos(),
                ),
                SpriteKind::End => (END_SIZE, END_SIZE / 2.0, 1.0),
            };
            let scale = cam.focal / depth;
            let hpx = size * scale;
            let wpx = hpx * tex.width as f64 / tex.height as f64 * spin.abs();
            let left = sx - wpx / 2.0;
            let top = cam.horizon + (0.5 - z) * scale - hpx / 2.0;
            let (x0, x1) = span(left, wpx, w);
            let (y0, y1) = span(top, hpx, h);
            if x0 >= x1 || y0 >= y1 {
                continue;
            }
            let (pos, step) = tex_fixed(y0, top, hpx, tex.height);
            let dist = ((wx - cam.x).powi(2) + (wy - cam.y).powi(2)).sqrt();
            out.push(Sprite { kind, depth, x0, x1, y0, y1, left, width: wpx, mirror: spin < 0.0, pos, step, op: shading.op(dist, false) });
        }
    }
    out.sort_unstable_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));
}

/// A vertical textured face (wall or grid) ready to draw in one column.
#[derive(Clone, Copy)]
struct Face {
    /// Perpendicular distance (depth ordering).
    perp: f64,
    y0: usize,
    y1: usize,
    tx: usize,
    /// Fixed-point texture row at `y0` and its step per pixel.
    pos: i64,
    step: i64,
    op: ShadeOp,
}

/// Rows `[y0, y1)` whose pixel centres lie in `[start, start + len)`, clipped to `[0, max)`.
#[inline]
fn span(start: f64, len: f64, max: usize) -> (usize, usize) {
    let clip = |v: f64| (v - 0.5).ceil().clamp(0.0, max as f64) as usize;
    (clip(start), clip(start + len))
}

/// Fixed-point texture coordinate at the centre of pixel `first` and its step,
/// for `texels` texels stretched over `len` pixels starting at `origin`.
#[inline]
fn tex_fixed(first: usize, origin: f64, len: f64, texels: usize) -> (i64, i64) {
    let scale = texels as f64 / len;
    (((first as f64 + 0.5 - origin) * scale * FRAC_ONE) as i64, (scale * FRAC_ONE) as i64)
}

/// First row below the horizon (rows above it are sky / ceiling).
fn horizon_row(cam: &Camera, h: usize) -> usize {
    (cam.horizon - 0.5).ceil().clamp(0.0, h as f64) as usize
}

/// Texture column of a face. Faces are seen from outside: moving right on
/// screen must move right in the texture. Screen right is `plane`, i.e. +y when
/// looking towards +x, so the coordinate along the face runs backwards (and
/// must be flipped) on X faces hit with dx < 0 and Y faces hit with dy > 0.
#[inline]
fn face_tx(tex: &Texture, wall_x: f64, flip: bool) -> usize {
    let tx = ((wall_x * tex.width as f64) as usize).min(tex.width - 1);
    if flip {
        tex.width - 1 - tx
    } else {
        tx
    }
}

fn entry_flip(side: Side, dx: f64, dy: f64) -> bool {
    match side {
        Side::X => dx < 0.0,
        Side::Y => dy > 0.0,
    }
}

/// Back face of a grid tile: where the ray leaves it, as `(t, wall_x, side, flip)`.
/// None when the next tile is a wall or another grid (that face is drawn by
/// the wall, or as the next grid's front face). Texture u stays attached to
/// the world face, so it is the mirror image of the front-face rule.
// Not inlined: inlined into `draw_column`, LLVM (rustc 1.100, opt-level 2)
// used 6+ GB of memory and got OOM-killed. It runs once per grid hit only.
#[inline(never)]
fn grid_exit(map: &Map, cam: &Camera, map_x: i32, map_y: i32, dx: f64, dy: f64) -> Option<(f64, f64, Side, bool)> {
    let border = |cell: i32, o: f64, d: f64| {
        if d > 0.0 {
            (cell as f64 + 1.0 - o) / d
        } else if d < 0.0 {
            (cell as f64 - o) / d
        } else {
            f64::INFINITY
        }
    };
    let (tx, ty) = (border(map_x, cam.x, dx), border(map_y, cam.y, dy));
    let (t, side, next) = if tx < ty {
        (tx, Side::X, (map_x + if dx < 0.0 { -1 } else { 1 }, map_y))
    } else {
        (ty, Side::Y, (map_x, map_y + if dy < 0.0 { -1 } else { 1 }))
    };
    if !t.is_finite() || matches!(map.get(next.0, next.1), Tile::Wall | Tile::Grid) {
        return None;
    }
    let along = match side {
        Side::X => cam.y + t * dy,
        Side::Y => cam.x + t * dx,
    };
    Some((t, along - along.floor(), side, !entry_flip(side, dx, dy)))
}

#[allow(clippy::too_many_arguments)]
fn make_face(cam: &Camera, h: usize, tex: &Texture, shading: &Shading, perp: f64, dist: f64, wall_x: f64, side: Side, flip: bool) -> Face {
    let hpx = cam.wall_height(perp);
    let top = cam.horizon - hpx / 2.0;
    let (y0, y1) = span(top, hpx, h);
    let (pos, step) = tex_fixed(y0, top, hpx, tex.height);
    Face { perp, y0, y1, tx: face_tx(tex, wall_x, flip), pos, step, op: shading.op(dist, side == Side::Y) }
}

/// Opaque textured span (walls).
#[inline]
fn draw_face_opaque(px: &mut [u32], w: usize, x: usize, f: &Face, tex: &Texture) {
    if f.y0 >= f.y1 {
        return;
    }
    let (tw, last) = (tex.width, tex.height - 1);
    let mut pos = f.pos;
    for p in px[f.y0 * w + x..].iter_mut().step_by(w).take(f.y1 - f.y0) {
        let ty = ((pos >> FRAC_BITS) as usize).min(last);
        *p = f.op.apply(tex.pixels[ty * tw + f.tx]) | OPAQUE;
        pos += f.step;
    }
}

/// Alpha-keyed span: alpha 0 texels are skipped, partial alpha is blended.
#[inline]
fn draw_face_keyed(px: &mut [u32], w: usize, x: usize, f: &Face, tex: &Texture) {
    if f.y0 >= f.y1 {
        return;
    }
    let (tw, last) = (tex.width, tex.height - 1);
    let mut pos = f.pos;
    for p in px[f.y0 * w + x..].iter_mut().step_by(w).take(f.y1 - f.y0) {
        let ty = ((pos >> FRAC_BITS) as usize).min(last);
        let t = tex.pixels[ty * tw + f.tx];
        if t >> 24 == 0xFF {
            *p = f.op.apply(t);
        } else if t >> 24 != 0 {
            *p = blend_u32(*p, f.op.apply(t), 255);
        }
        pos += f.step;
    }
}

/// Floor or ceiling casting in column `x` for `rows`: the plane point of row
/// y is `origin + row_dist[y] * ray_dir` (ray_dir has a unit component along
/// the view direction, so row_dist is a perpendicular distance).
#[allow(clippy::too_many_arguments)]
fn cast_plane(px: &mut [u32], w: usize, x: usize, rows: Range<usize>, row_dist: &[f64], cam: &Camera, ray: (f64, f64), tex: &Texture, shading: &Shading) {
    if rows.is_empty() {
        return;
    }
    let (tw, th) = (tex.width, tex.height);
    let (rdx, rdy) = ray;
    // euclidean distance = perpendicular distance * |ray_dir|
    let lut_scale = (rdx * rdx + rdy * rdy).sqrt() * LUT_RES;
    let lut = &shading.lut[..LUT_LEN];
    let start = rows.start;
    for (p, &d) in px[start * w + x..].iter_mut().step_by(w).zip(&row_dist[rows]) {
        // 16.16 fixed point; `& 0xFFFF` is the wrapped fraction (also for negatives)
        let u = ((cam.x + d * rdx) * PLANE_ONE) as i64;
        let v = ((cam.y + d * rdy) * PLANE_ONE) as i64;
        let tx = ((u & 0xFFFF) as usize * tw) >> 16;
        let ty = ((v & 0xFFFF) as usize * th) >> 16;
        let op = lut[((d * lut_scale) as usize).min(LUT_LEN - 1)];
        *p = op.apply(tex.pixels[ty * tw + tx]) | OPAQUE;
    }
}

pub struct TexturedRenderer {
    shading: Shading,
    fallback: Fallbacks,
    sky_art: SkyArt,
    /// Perpendicular distance of the floor (below the horizon) / ceiling (above) per row.
    row_dist: Vec<f64>,
    /// View angle of each column relative to the camera direction.
    col_angle: Vec<f64>,
    /// Sky texture column of each screen column.
    col_tx: Vec<usize>,
    /// Grid faces of every column, far to near, drawn after the columns.
    faces: Vec<Face>,
    /// `faces[face_start[x]..face_start[x + 1]]` belong to column x.
    face_start: Vec<usize>,
    /// Visible billboards of the frame, far to near.
    sprites: Vec<Sprite>,
}

impl Default for TexturedRenderer {
    fn default() -> TexturedRenderer {
        TexturedRenderer::new()
    }
}

impl TexturedRenderer {
    pub fn new() -> TexturedRenderer {
        TexturedRenderer {
            shading: Shading::default(),
            fallback: Fallbacks::new(&PresetColors::default()),
            sky_art: SkyArt::default(),
            row_dist: Vec::new(),
            col_angle: Vec::new(),
            col_tx: Vec::new(),
            faces: Vec::new(),
            face_start: Vec::new(),
            sprites: Vec::new(),
        }
    }

    /// Rows above the horizon: the preset sky texture (u = view angle over
    /// 360 degrees, v = row over the sky height), or a gradient + sun + clouds.
    fn draw_sky(&mut self, fb: &mut FrameBuffer, view: &View) {
        let cam = &view.camera;
        let p = view.preset;
        let w = fb.width();
        let rows = horizon_row(cam, fb.height());
        let px = fb.pixels_mut();

        if let Some(tex) = usable(&p.textures.sky) {
            let tw = tex.width;
            self.col_tx.clear();
            self.col_tx.extend(self.col_angle.iter().map(|&a| (((cam.angle + a) / TAU).rem_euclid(1.0) * tw as f64) as usize % tw));
            for y in 0..rows {
                let ty = (y * tex.height / rows).min(tex.height - 1);
                let src = &tex.pixels[ty * tw..(ty + 1) * tw];
                for (d, &tx) in px[y * w..(y + 1) * w].iter_mut().zip(&self.col_tx) {
                    *d = src[tx] | OPAQUE;
                }
            }
            return;
        }

        let sky = p.colors.sky;
        // lighter towards the horizon; with fog the horizon is exactly the fog colour
        let (top, bottom) = if p.fog { (sky.scale(0.85), sky) } else { (sky.scale(0.8), sky.lerp(Color::WHITE, 0.3)) };
        for y in 0..rows {
            let c = top.lerp(bottom, (y as f64 + 0.5) / rows as f64).to_u32() | OPAQUE;
            px[y * w..(y + 1) * w].fill(c);
        }

        self.sky_art.update(&p.colors, view.common);
        let alpha = if p.fog { SKY_FOG_ALPHA } else { 255 };
        if let Some(sun) = &self.sky_art.sun {
            draw_sky_object(px, w, rows, cam, &self.col_angle, sun, SUN_ANGLE, SUN_ELEVATION, SUN_SIZE, alpha);
        }
        let clouds = &self.sky_art.clouds;
        if !clouds.is_empty() {
            for c in CLOUDS.iter() {
                let angle = c.angle + view.time * c.speed;
                draw_sky_object(px, w, rows, cam, &self.col_angle, &clouds[c.texture % clouds.len()], angle, c.elevation, c.width, alpha);
            }
        }
    }
}

impl WorldRenderer for TexturedRenderer {
    fn draw_background(&mut self, fb: &mut FrameBuffer, view: &View) {
        let p = view.preset;
        let cam = &view.camera;
        self.shading.update(p);
        if self.fallback.colors != p.colors {
            self.fallback = Fallbacks::new(&p.colors);
        }
        let (w, h) = (fb.width(), fb.height());
        self.row_dist.clear();
        self.row_dist.extend((0..h).map(|y| 0.5 * cam.focal / (y as f64 + 0.5 - cam.horizon).abs().max(1e-3)));
        // column x looks along dir + plane * cx, i.e. at atan((x + 0.5 - W/2) / focal) from dir
        self.col_angle.clear();
        self.col_angle.extend((0..w).map(|x| ((x as f64 + 0.5 - w as f64 / 2.0) / cam.focal).atan()));
        self.faces.clear();
        self.face_start.clear();
        self.face_start.resize(w + 1, 0);
        if p.outside {
            self.draw_sky(fb, view);
        }
    }

    fn draw_column(&mut self, fb: &mut FrameBuffer, view: &View, x: usize, hits: &ColumnHits) {
        let cam = &view.camera;
        let slots = Slots::new(view.preset, &self.fallback);
        let (w, h) = (fb.width(), fb.height());
        let px = fb.pixels_mut();
        let (dx, dy) = hits.ray_dir;

        let (wall_top, wall_bottom) = match hits.solid {
            Some(hit) => {
                let face = make_face(cam, h, slots.wall, &self.shading, hit.perp_dist, hit.dist, hit.wall_x, hit.side, entry_flip(hit.side, dx, dy));
                draw_face_opaque(px, w, x, &face, slots.wall);
                (face.y0, face.y1)
            }
            None => {
                let r = horizon_row(cam, h);
                (r, r)
            }
        };
        cast_plane(px, w, x, wall_bottom..h, &self.row_dist, cam, (dx, dy), slots.floor, &self.shading);
        if !view.preset.outside {
            cast_plane(px, w, x, 0..wall_top, &self.row_dist, cam, (dx, dy), slots.ceiling, &self.shading);
        }

        // grids are only recorded here (far to near: back face, then front
        // face of each tile) and drawn in `draw_sprites`, interleaved with the
        // billboards by depth
        self.face_start[x] = self.faces.len();
        let len = (dx * dx + dy * dy).sqrt();
        for hit in hits.transparent.iter().rev() {
            if let Some((t, along, side, flip)) = grid_exit(view.map, cam, hit.map_x, hit.map_y, dx, dy) {
                self.faces.push(make_face(cam, h, slots.grid, &self.shading, t, t * len, along, side, flip));
            }
            let flip = entry_flip(hit.side, dx, dy);
            self.faces.push(make_face(cam, h, slots.grid, &self.shading, hit.perp_dist, hit.dist, hit.wall_x, hit.side, flip));
        }
    }

    /// Billboards and the grid faces recorded by `draw_column`, merged far to
    /// near per column. Billboards behind the solid wall (`depth`) are skipped.
    fn draw_sprites(&mut self, fb: &mut FrameBuffer, view: &View, depth: &[f64]) {
        let (w, h) = (fb.width(), fb.height());
        self.face_start[w] = self.faces.len();
        let slots = Slots::new(view.preset, &self.fallback);
        collect_sprites(&mut self.sprites, view, &slots, &self.shading, w, h);
        let px = fb.pixels_mut();
        for x in 0..w {
            let faces = &self.faces[self.face_start[x]..self.face_start[x + 1]];
            let mut next = 0;
            for s in &self.sprites {
                if x < s.x0 || x >= s.x1 || s.depth >= depth[x] {
                    continue;
                }
                while next < faces.len() && faces[next].perp > s.depth {
                    draw_face_keyed(px, w, x, &faces[next], slots.grid);
                    next += 1;
                }
                let tex = match s.kind {
                    SpriteKind::Coin => slots.coin,
                    SpriteKind::End => slots.end,
                };
                draw_face_keyed(px, w, x, &s.column(x, tex), tex);
            }
            for f in &faces[next..] {
                draw_face_keyed(px, w, x, f, slots.grid);
            }
        }
    }

    fn items_as_sprites(&self) -> bool {
        true
    }
}
