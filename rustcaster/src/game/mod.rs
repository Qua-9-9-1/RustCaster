// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// Game simulation, independent from rendering and SDL windowing.

pub mod map;
pub mod player;

use std::path::{Path, PathBuf};

use crate::input::Input;
use crate::settings::Settings;
use map::{Map, MapError, Tile};
use player::Player;

/// One level being played.
pub struct GameState {
    pub map_path: PathBuf,
    /// Mutable copy of the map (collected coins become `Tile::Empty`).
    pub map: Map,
    pub player: Player,
    pub coins_collected: usize,
    /// Set when the player steps on an 'E' tile.
    pub finished: bool,
    /// Seconds since the level started (for animations: bobbing coins, clouds...).
    pub time: f64,
}

impl GameState {
    pub fn load(path: &Path) -> Result<GameState, MapError> {
        let map = Map::load(path)?;
        Ok(GameState::new(map, path.to_path_buf()))
    }

    pub fn new(map: Map, map_path: PathBuf) -> GameState {
        let player = Player::spawn(&map);
        GameState { map_path, map, player, coins_collected: 0, finished: false, time: 0.0 }
    }

    pub fn update(&mut self, input: &Input, settings: &Settings, dt: f64) {
        if self.finished {
            return;
        }
        self.time += dt;
        self.player.update(&self.map, input, settings, dt);
        let (tx, ty) = (self.player.x.floor() as i32, self.player.y.floor() as i32);
        match self.map.get(tx, ty) {
            Tile::Coin => {
                self.coins_collected += 1;
                self.map.set(tx, ty, Tile::Empty);
            }
            Tile::End => self.finished = true,
            _ => {}
        }
    }
}
