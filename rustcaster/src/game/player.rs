// Player position, orientation, keyboard movement and wall collision.

use std::f64::consts::TAU;

use sdl2::keyboard::Scancode;

use crate::config::{PLAYER_RADIUS, PLAYER_SPEED};
use crate::game::map::Map;
use crate::input::Input;
use crate::settings::Settings;

#[derive(Clone, Debug)]
pub struct Player {
    /// World position in tiles (tile (i, j) covers [i, i+1) x [j, j+1)).
    pub x: f64,
    pub y: f64,
    /// Radians in [0, 2pi). 0 looks towards +x, angles grow clockwise on
    /// screen because +y points down the map.
    pub angle: f64,
}

impl Player {
    /// Centre of the map's 'P' tile, looking towards +x.
    pub fn spawn(map: &Map) -> Player {
        let (x, y) = map.player_start;
        Player { x: x as f64 + 0.5, y: y as f64 + 0.5, angle: 0.0 }
    }

    /// Unit view direction.
    pub fn dir(&self) -> (f64, f64) {
        (self.angle.cos(), self.angle.sin())
    }

    /// W/S (or Up/Down) move, A/D (or Left/Right) turn.
    pub fn update(&mut self, map: &Map, input: &Input, settings: &Settings, dt: f64) {
        let turn = settings.turn_speed_rad() * dt;
        if input.is_down(Scancode::A) || input.is_down(Scancode::Left) {
            self.angle -= turn;
        }
        if input.is_down(Scancode::D) || input.is_down(Scancode::Right) {
            self.angle += turn;
        }
        self.angle = self.angle.rem_euclid(TAU);

        let mut forward = 0.0;
        if input.is_down(Scancode::W) || input.is_down(Scancode::Up) {
            forward += 1.0;
        }
        if input.is_down(Scancode::S) || input.is_down(Scancode::Down) {
            forward -= 1.0;
        }
        if forward != 0.0 {
            let (dx, dy) = self.dir();
            let step = forward * PLAYER_SPEED * dt;
            self.try_move(map, dx * step, dy * step);
        }
    }

    /// Moves each axis independently (wall sliding), refusing a step when the
    /// tile `PLAYER_RADIUS` ahead on that axis is solid.
    pub fn try_move(&mut self, map: &Map, dx: f64, dy: f64) {
        if dx != 0.0 {
            let probe = self.x + dx + PLAYER_RADIUS * dx.signum();
            if !map.at(probe, self.y).is_solid() {
                self.x += dx;
            }
        }
        if dy != 0.0 {
            let probe = self.y + dy + PLAYER_RADIUS * dy.signum();
            if !map.at(self.x, probe).is_solid() {
                self.y += dy;
            }
        }
    }
}
