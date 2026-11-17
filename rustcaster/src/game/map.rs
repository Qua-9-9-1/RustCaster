// ASCII map loading and validation.
//
//   '#' wall   'X' semi-transparent grid   '0' coin   'E' end
//   'P' player start (exactly one)   'M' monster (unused)   ' ' floor
//
// Rules: rectangular (every line the same length), one 'P', at least one 'E'.
// CRLF line endings and trailing blank lines are accepted. Cells outside the
// map behave like walls.

use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Empty,
    Wall,
    Grid,
    Coin,
    End,
    Monster,
}

impl Tile {
    pub fn from_char(c: char) -> Option<Tile> {
        match c {
            ' ' | 'P' => Some(Tile::Empty),
            '#' => Some(Tile::Wall),
            'X' => Some(Tile::Grid),
            '0' => Some(Tile::Coin),
            'E' => Some(Tile::End),
            'M' => Some(Tile::Monster),
            _ => None,
        }
    }

    /// Blocks player movement.
    pub fn is_solid(self) -> bool {
        matches!(self, Tile::Wall | Tile::Grid)
    }
}

#[derive(Debug)]
pub enum MapError {
    Io(String, std::io::Error),
    IllegalChar { line: usize, column: usize, c: char },
    Empty,
    NotRectangular { line: usize },
    PlayerCount(usize),
    NoExit,
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapError::Io(path, e) => write!(f, "{}: {}", path, e),
            MapError::IllegalChar { line, column, c } => write!(f, "illegal character {:?} at line {}, column {}", c, line, column),
            MapError::Empty => write!(f, "map is empty"),
            MapError::NotRectangular { line } => write!(f, "map must have the same length for each line (line {})", line),
            MapError::PlayerCount(n) => write!(f, "map must contain exactly one player, found {}", n),
            MapError::NoExit => write!(f, "map must contain at least one exit"),
        }
    }
}

impl std::error::Error for MapError {}

#[derive(Clone, Debug)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    /// Row-major, `tiles[y * width + x]`.
    pub tiles: Vec<Tile>,
    /// Tile coordinates of 'P'.
    pub player_start: (usize, usize),
    pub total_coins: usize,
}

impl Map {
    pub fn load(path: &Path) -> Result<Map, MapError> {
        let text = fs::read_to_string(path).map_err(|e| MapError::Io(path.display().to_string(), e))?;
        Map::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Map, MapError> {
        let text = text.replace('\r', "");
        let mut lines: Vec<&str> = text.split('\n').collect();
        while lines.last().map_or(false, |l| l.is_empty()) {
            lines.pop();
        }
        if lines.is_empty() || lines[0].is_empty() {
            return Err(MapError::Empty);
        }
        let width = lines[0].chars().count();
        let height = lines.len();
        let mut tiles = Vec::with_capacity(width * height);
        let mut players = Vec::new();
        let mut exits = 0;
        let mut total_coins = 0;
        for (y, line) in lines.iter().enumerate() {
            if line.chars().count() != width {
                return Err(MapError::NotRectangular { line: y + 1 });
            }
            for (x, c) in line.chars().enumerate() {
                let tile = Tile::from_char(c).ok_or(MapError::IllegalChar { line: y + 1, column: x + 1, c })?;
                match c {
                    'P' => players.push((x, y)),
                    'E' => exits += 1,
                    '0' => total_coins += 1,
                    _ => {}
                }
                tiles.push(tile);
            }
        }
        if players.len() != 1 {
            return Err(MapError::PlayerCount(players.len()));
        }
        if exits == 0 {
            return Err(MapError::NoExit);
        }
        Ok(Map { width, height, tiles, player_start: players[0], total_coins })
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    /// Tile at integer coordinates; outside the map is `Tile::Wall`.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Tile {
        if self.in_bounds(x, y) {
            self.tiles[y as usize * self.width + x as usize]
        } else {
            Tile::Wall
        }
    }

    /// Tile containing the world position (in tiles).
    pub fn at(&self, x: f64, y: f64) -> Tile {
        self.get(x.floor() as i32, y.floor() as i32)
    }

    pub fn set(&mut self, x: i32, y: i32, tile: Tile) {
        if self.in_bounds(x, y) {
            self.tiles[y as usize * self.width + x as usize] = tile;
        }
    }
}
