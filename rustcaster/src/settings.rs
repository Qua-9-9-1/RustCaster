// User-tweakable settings (replace the old U/J, I/K and numpad key hacks).
// The pause / settings UI should only go through the `adjust_*` / `set_*`
// methods so values stay clamped.

use crate::config::{
    DEFAULT_FOV_DEG, DEFAULT_PRESET, DEFAULT_SENSITIVITY, MAX_FOV_DEG, MAX_SENSITIVITY, MIN_FOV_DEG, MIN_SENSITIVITY,
};

#[derive(Clone, Debug)]
pub struct Settings {
    /// Horizontal field of view in degrees.
    pub fov_deg: f64,
    /// Turn speed in degrees per 60 FPS frame.
    pub sensitivity: f64,
    /// Name of the selected preset (an entry of `PresetLibrary`).
    pub preset: String,
    pub show_minimap: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            fov_deg: DEFAULT_FOV_DEG,
            sensitivity: DEFAULT_SENSITIVITY,
            preset: DEFAULT_PRESET.to_string(),
            show_minimap: true,
        }
    }
}

impl Settings {
    pub fn set_fov(&mut self, deg: f64) {
        self.fov_deg = deg.clamp(MIN_FOV_DEG, MAX_FOV_DEG);
    }

    pub fn adjust_fov(&mut self, delta_deg: f64) {
        self.set_fov(self.fov_deg + delta_deg);
    }

    pub fn set_sensitivity(&mut self, s: f64) {
        self.sensitivity = s.clamp(MIN_SENSITIVITY, MAX_SENSITIVITY);
    }

    pub fn adjust_sensitivity(&mut self, delta: f64) {
        // round to one decimal so repeated +0.1 steps display cleanly
        self.set_sensitivity(((self.sensitivity + delta) * 10.0).round() / 10.0);
    }

    pub fn fov_rad(&self) -> f64 {
        self.fov_deg.to_radians()
    }

    /// Radians per second.
    pub fn turn_speed_rad(&self) -> f64 {
        (self.sensitivity * 60.0).to_radians()
    }
}
