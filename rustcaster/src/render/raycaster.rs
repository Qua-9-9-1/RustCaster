// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// DDA raycasting of the 3D view into the framebuffer.
//
// Split in two layers so the look can be swapped without touching the maths:
//  * `Raycaster` owns the per-frame loop: one ray per framebuffer column,
//    cast with DDA (`cast_ray`), results collected in `ColumnHits`, plus a
//    depth buffer (`depth`) and hit points (`ray_ends`, used by the minimap).
//  * a `WorldRenderer` turns those hits into pixels. `FlatRenderer` (below)
//    reproduces the historical flat-colour look; a textured renderer only has
//    to implement the same trait (see ARCHITECTURE.md).
//
// Projection: camera-plane model. Column `x` shoots the ray
// `dir + plane * (2x/W - 1)` where |plane| = tan(fov/2), so `RayHit::perp_dist`
// is the fisheye-free distance. A wall of height 1 tile at distance d is
// `focal / d` pixels tall, centred on the horizon line `horizon`.
// Floor/ceiling casting: the floor pixel at row y > horizon is at distance
// `0.5 * focal / (y - horizon)` (eye at mid wall height).

use std::f64::consts::{PI, TAU};

use crate::assets::preset::{CommonAssets, Preset};
use crate::config::{FOG_DISTANCE, MAX_RAY_DIST, SHADE_FACTOR, SHADE_STEP};
use crate::game::map::{Map, Tile};
use crate::game::player::Player;
use crate::render::color::Color;
use crate::render::framebuffer::{FrameBuffer, Rect};

/// Max semi-transparent tiles remembered per column.
pub const MAX_TRANSPARENT_HITS: usize = 16;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    /// Unit view direction.
    pub dir_x: f64,
    pub dir_y: f64,
    /// Camera plane (perpendicular to dir, length tan(fov/2)), points to the right of the screen.
    pub plane_x: f64,
    pub plane_y: f64,
    pub angle: f64,
    pub fov: f64,
    /// Focal length in pixels: (W/2) / tan(fov/2).
    pub focal: f64,
    /// Screen row of the horizon (H/2 for now; a pitch/bob offset could move it).
    pub horizon: f64,
    pub width: usize,
    pub height: usize,
}

impl Camera {
    pub fn new(player: &Player, fov_rad: f64, width: usize, height: usize) -> Camera {
        let (dir_y, dir_x) = player.angle.sin_cos();
        let half = (fov_rad / 2.0).tan();
        Camera {
            x: player.x,
            y: player.y,
            dir_x,
            dir_y,
            plane_x: -dir_y * half,
            plane_y: dir_x * half,
            angle: player.angle,
            fov: fov_rad,
            focal: (width as f64 / 2.0) / half,
            horizon: height as f64 / 2.0,
            width,
            height,
        }
    }

    /// Ray direction of a framebuffer column (not normalised: its component
    /// along `dir` is 1, which is what makes DDA distances perpendicular).
    pub fn ray_dir(&self, column: usize) -> (f64, f64) {
        let cx = 2.0 * (column as f64 + 0.5) / self.width as f64 - 1.0;
        (self.dir_x + self.plane_x * cx, self.dir_y + self.plane_y * cx)
    }

    /// Projects a world point (tiles) for billboards: returns
    /// `(screen_x, depth)` with depth = perpendicular distance, or None when
    /// behind the camera.
    pub fn project(&self, wx: f64, wy: f64) -> Option<(f64, f64)> {
        let (rx, ry) = (wx - self.x, wy - self.y);
        let depth = rx * self.dir_x + ry * self.dir_y;
        if depth <= 1e-6 {
            return None;
        }
        let half = (self.fov / 2.0).tan();
        // signed distance to the right of the view axis
        let lateral = (rx * self.plane_x + ry * self.plane_y) / half;
        Some((self.width as f64 / 2.0 + lateral / depth * self.focal, depth))
    }

    /// On-screen x of a direction at world angle `angle` (sky objects at
    /// infinity), or None when outside the field of view.
    pub fn project_angle(&self, angle: f64) -> Option<f64> {
        let rel = (angle - self.angle + PI).rem_euclid(TAU) - PI;
        if rel.abs() >= PI / 2.0 {
            return None;
        }
        Some(self.width as f64 / 2.0 + rel.tan() * self.focal)
    }

    /// Height in pixels of a 1-tile-high wall at perpendicular distance `perp`.
    pub fn wall_height(&self, perp: f64) -> f64 {
        self.focal / perp.max(1e-6)
    }
}

/// Which kind of grid line the ray crossed to enter the tile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    /// Crossed a vertical line x = k: the hit face looks west or east.
    X,
    /// Crossed a horizontal line y = k: the hit face looks north or south.
    Y,
}

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub tile: Tile,
    pub map_x: i32,
    pub map_y: i32,
    pub side: Side,
    /// Perpendicular (fisheye-corrected) distance, use it for heights/depth.
    pub perp_dist: f64,
    /// Euclidean distance from the eye, use it for fog/shading.
    pub dist: f64,
    /// Where the face was hit, in [0, 1): texture `u` coordinate.
    pub wall_x: f64,
}

/// Everything one ray met, filled by `cast_ray`.
#[derive(Clone, Debug, Default)]
pub struct ColumnHits {
    /// First opaque tile (wall, map border, or coin/end when items block rays),
    /// None when nothing within `MAX_RAY_DIST`.
    pub solid: Option<RayHit>,
    /// Semi-transparent tiles (grids) in front of `solid`, nearest first.
    pub transparent: Vec<RayHit>,
    pub ray_dir: (f64, f64),
    /// Euclidean length travelled (to `solid`, or the max distance).
    pub ray_len: f64,
}

/// How tiles interact with rays.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RayInteraction {
    Pass,
    Transparent,
    Solid,
}

pub fn tile_interaction(tile: Tile, items_as_sprites: bool) -> RayInteraction {
    match tile {
        Tile::Wall => RayInteraction::Solid,
        Tile::Grid => RayInteraction::Transparent,
        Tile::Coin | Tile::End if !items_as_sprites => RayInteraction::Solid,
        _ => RayInteraction::Pass,
    }
}

/// DDA from `(ox, oy)` along `(dx, dy)` (tiles), jumping from one tile border
/// to the next. The cost is the number of tiles crossed.
pub fn cast_ray(map: &Map, ox: f64, oy: f64, dx: f64, dy: f64, max_dist: f64, items_as_sprites: bool, out: &mut ColumnHits) {
    out.solid = None;
    out.transparent.clear();
    out.ray_dir = (dx, dy);
    let len = (dx * dx + dy * dy).sqrt().max(1e-12);

    let mut map_x = ox.floor() as i32;
    let mut map_y = oy.floor() as i32;
    let step_x: i32 = if dx < 0.0 { -1 } else { 1 };
    let step_y: i32 = if dy < 0.0 { -1 } else { 1 };
    // ray parameter needed to cross one full tile on each axis
    let delta_x = if dx == 0.0 { f64::INFINITY } else { (1.0 / dx).abs() };
    let delta_y = if dy == 0.0 { f64::INFINITY } else { (1.0 / dy).abs() };
    // ray parameter to the first vertical / horizontal border
    let mut side_x = if dx == 0.0 {
        f64::INFINITY
    } else if dx < 0.0 {
        (ox - map_x as f64) * delta_x
    } else {
        (map_x as f64 + 1.0 - ox) * delta_x
    };
    let mut side_y = if dy == 0.0 {
        f64::INFINITY
    } else if dy < 0.0 {
        (oy - map_y as f64) * delta_y
    } else {
        (map_y as f64 + 1.0 - oy) * delta_y
    };

    loop {
        let (t, side) = if side_x < side_y {
            let t = side_x;
            side_x += delta_x;
            map_x += step_x;
            (t, Side::X)
        } else {
            let t = side_y;
            side_y += delta_y;
            map_y += step_y;
            (t, Side::Y)
        };
        let dist = t * len;
        if dist > max_dist {
            out.ray_len = max_dist;
            return;
        }
        let tile = map.get(map_x, map_y);
        let interaction = tile_interaction(tile, items_as_sprites);
        if interaction == RayInteraction::Pass {
            continue;
        }
        let wall_x = match side {
            Side::X => oy + t * dy,
            Side::Y => ox + t * dx,
        };
        let hit = RayHit { tile, map_x, map_y, side, perp_dist: t, dist, wall_x: wall_x - wall_x.floor() };
        if interaction == RayInteraction::Solid {
            out.solid = Some(hit);
            out.ray_len = dist;
            return;
        }
        if out.transparent.len() < MAX_TRANSPARENT_HITS {
            out.transparent.push(hit);
        }
    }
}

/// What a `WorldRenderer` gets to draw a frame.
pub struct View<'a> {
    pub camera: Camera,
    pub map: &'a Map,
    pub preset: &'a Preset,
    pub common: &'a CommonAssets,
    /// Seconds since the level started (animations).
    pub time: f64,
}

/// Pluggable drawing of the 3D view. Call order per frame:
/// `draw_background` once, `draw_column` for x in 0..W, then `draw_sprites`.
pub trait WorldRenderer {
    /// When true, coins / end tiles do not stop rays: they are expected to be
    /// drawn as billboards in `draw_sprites`. When false they are solid blocks.
    fn items_as_sprites(&self) -> bool {
        false
    }

    /// Sky / ceiling / floor for the whole view.
    fn draw_background(&mut self, fb: &mut FrameBuffer, view: &View);

    /// Draws column `x` from the ray results (solid hit, then transparent ones).
    fn draw_column(&mut self, fb: &mut FrameBuffer, view: &View, x: usize, hits: &ColumnHits);

    /// Billboards, after all columns. `depth[x]` = perpendicular distance of the
    /// solid hit in column x (`f64::INFINITY` if none) for occlusion.
    fn draw_sprites(&mut self, _fb: &mut FrameBuffer, _view: &View, _depth: &[f64]) {}
}

/// Reusable per-frame buffers + the column loop.
#[derive(Default)]
pub struct Raycaster {
    hits: ColumnHits,
    /// Perpendicular distance of the solid hit per column (INFINITY = none).
    pub depth: Vec<f64>,
    /// World point where each column's ray stopped (minimap rays).
    pub ray_ends: Vec<(f64, f64)>,
}

impl Raycaster {
    pub fn new() -> Raycaster {
        Raycaster::default()
    }

    pub fn render(&mut self, fb: &mut FrameBuffer, view: &View, renderer: &mut dyn WorldRenderer) {
        let w = fb.width();
        self.depth.resize(w, f64::INFINITY);
        self.ray_ends.resize(w, (0.0, 0.0));
        let items_as_sprites = renderer.items_as_sprites();
        let cam = view.camera;

        renderer.draw_background(fb, view);
        for x in 0..w {
            let (dx, dy) = cam.ray_dir(x);
            cast_ray(view.map, cam.x, cam.y, dx, dy, MAX_RAY_DIST, items_as_sprites, &mut self.hits);
            self.depth[x] = self.hits.solid.map_or(f64::INFINITY, |h| h.perp_dist);
            let len = (dx * dx + dy * dy).sqrt();
            let t = self.hits.ray_len / len;
            self.ray_ends[x] = (cam.x + dx * t, cam.y + dy * t);
            renderer.draw_column(fb, view, x, &self.hits);
        }
        renderer.draw_sprites(fb, view, &self.depth);
    }
}

/// Historical distance shading: indoors colours get darker by steps, outside
/// with fog they fade to the sky colour, outside without fog they stay flat.
pub fn distance_shade(color: Color, dist: f64, preset: &Preset) -> Color {
    if preset.outside {
        if preset.fog {
            color.lerp(preset.colors.sky, dist / FOG_DISTANCE)
        } else {
            color
        }
    } else {
        color.scale(SHADE_FACTOR.powi((dist / SHADE_STEP) as i32))
    }
}

/// Sun position in the sky (world angle, towards -y = up on the minimap).
pub const SUN_ANGLE: f64 = 1.5 * PI;
const SUN_COLOR: Color = Color::rgb(255, 255, 150);
const GRID_ALPHA: u8 = 160;

/// Flat colours, no textures: the look of the game before the refactor.
#[derive(Default)]
pub struct FlatRenderer;

impl WorldRenderer for FlatRenderer {
    fn draw_background(&mut self, fb: &mut FrameBuffer, view: &View) {
        let p = view.preset;
        let w = fb.width() as i32;
        let h = fb.height() as i32;
        let horizon = view.camera.horizon as i32;
        let top = if p.outside { p.colors.sky } else { p.colors.ceiling };
        fb.fill_rect(Rect::new(0, 0, w, horizon), top);
        fb.fill_rect(Rect::new(0, horizon, w, h - horizon), p.colors.floor);
        if p.outside && !p.fog {
            if let Some(sx) = view.camera.project_angle(SUN_ANGLE) {
                let r = (h as f64 * 0.1) as i32;
                fb.fill_circle(sx as i32, (h as f64 * 0.15) as i32, r, SUN_COLOR);
            }
        }
    }

    fn draw_column(&mut self, fb: &mut FrameBuffer, view: &View, x: usize, hits: &ColumnHits) {
        let p = view.preset;
        let cam = &view.camera;
        let span = |perp: f64| {
            let h = cam.wall_height(perp);
            ((cam.horizon - h / 2.0) as i32, (cam.horizon + h / 2.0).ceil() as i32)
        };
        if let Some(hit) = hits.solid {
            let color = match hit.tile {
                Tile::Coin => p.colors.coin,
                Tile::End => p.colors.end,
                _ => distance_shade(p.colors.wall, hit.dist, p),
            };
            let (y0, y1) = span(hit.perp_dist);
            fb.draw_vline(x as i32, y0, y1, color);
        }
        // far to near so nearer grids blend over farther ones
        for hit in hits.transparent.iter().rev() {
            let (y0, y1) = span(hit.perp_dist);
            fb.draw_vline(x as i32, y0, y1, p.colors.grid.with_alpha(GRID_ALPHA));
        }
    }
}
