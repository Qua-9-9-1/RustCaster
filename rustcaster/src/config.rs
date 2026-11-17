// Global constants. Units: world distances are in tiles (1.0 = one map cell),
// angles in radians unless the name says `_DEG`, times in seconds.

use std::path::{Path, PathBuf};

pub const WINDOW_TITLE: &str = "RustCasting";
/// Initial window size (the window is then made fullscreen-desktop).
pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 720;

/// Height of the internal framebuffer in pixels. Its width follows the window
/// aspect ratio so pixels stay square; the framebuffer is scaled to the window.
pub const RENDER_HEIGHT: u32 = 480;

pub const TARGET_FPS: u64 = 60;
/// Longest simulated step: avoids teleporting through walls after a hitch.
pub const MAX_FRAME_DT: f64 = 0.1;

/// Player speed in tiles per second (the old game moved 3px of a 32px block per 60 FPS frame).
pub const PLAYER_SPEED: f64 = 5.625;
/// Collision look-ahead radius around the player, in tiles.
pub const PLAYER_RADIUS: f64 = 0.3125;

pub const DEFAULT_FOV_DEG: f64 = 80.0;
pub const MIN_FOV_DEG: f64 = 30.0;
pub const MAX_FOV_DEG: f64 = 150.0;
/// Turn speed: degrees per 60 FPS frame (so `sensitivity * 60` degrees per second).
pub const DEFAULT_SENSITIVITY: f64 = 4.0;
pub const MIN_SENSITIVITY: f64 = 0.5;
pub const MAX_SENSITIVITY: f64 = 10.0;
pub const DEFAULT_PRESET: &str = "house";

/// Rays further than this hit nothing (sky/floor visible), in tiles.
pub const MAX_RAY_DIST: f64 = 31.25;
/// Indoor shading: colour is multiplied by `SHADE_FACTOR` every `SHADE_STEP` tiles.
pub const SHADE_STEP: f64 = 17.5 / 32.0;
pub const SHADE_FACTOR: f64 = 0.8;
/// Fog: fully foggy (sky colour) at this distance, in tiles.
pub const FOG_DISTANCE: f64 = 100.0 / 32.0;

/// Minimap overlay: side as a fraction of the framebuffer height, tiles shown across.
pub const MINIMAP_SIZE_RATIO: f64 = 0.26;
pub const MINIMAP_TILES_ACROSS: f64 = 8.125;
pub const MINIMAP_MARGIN: i32 = 6;

pub const MAPS_DIR: &str = "maps";
pub const ASSETS_DIR: &str = "assets";
pub const PRESETS_DIR: &str = "assets/presets";
pub const COMMON_ASSETS_DIR: &str = "assets/common";

/// Directory holding `assets/` and `maps/`. Looked up in the current directory,
/// then next to the executable and in its parents (so running
/// `target/<triple>/debug/rustcaster.exe` from anywhere finds `rustcaster/`).
/// Falls back to the current directory.
pub fn find_data_root() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..5 {
            match dir {
                Some(d) => {
                    dir = d.parent().map(Path::to_path_buf);
                    candidates.push(d);
                }
                None => break,
            }
        }
    }
    for c in &candidates {
        if c.join(ASSETS_DIR).is_dir() && c.join(MAPS_DIR).is_dir() {
            return c.clone();
        }
    }
    candidates.into_iter().next().unwrap_or_else(|| PathBuf::from("."))
}
