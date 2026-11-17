// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// Presets (formerly "ambiances"): a named look for a level = flat colours,
// `outside` / `fog` flags and optional textures.
//
// On disk a preset is a folder `assets/presets/<name>/` holding any of the
// conventional texture files (see `TextureSlot::file_name`) and an optional
// `preset.json` (colours + flags, see `PresetFile`). Without `preset.json`,
// colours and flags come from the built-in table below (the 8 historical
// ambiances) or from `PresetColors::default()` for unknown names.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assets::texture::Texture;
use crate::render::color::Color;

pub const PRESET_FILE: &str = "preset.json";

/// Names of the presets hardcoded in `Preset::builtin`, in menu order.
pub const BUILTIN_PRESETS: [&str; 8] = ["house", "cave", "garden", "halloween", "night", "nether", "retro", "snow"];

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetColors {
    pub sky: Color,
    /// Top half of the view when `outside == false`.
    pub ceiling: Color,
    pub floor: Color,
    pub wall: Color,
    pub grid: Color,
    pub coin: Color,
    pub end: Color,
    pub clouds: Color,
    pub sun: Color,
}

impl Default for PresetColors {
    fn default() -> PresetColors {
        PresetColors {
            sky: Color::rgb(100, 100, 100),
            ceiling: Color::rgb(100, 100, 100),
            floor: Color::rgb(100, 100, 100),
            wall: Color::rgb(50, 50, 50),
            grid: Color::WHITE,
            coin: Color::rgb(255, 215, 0),
            end: Color::rgb(0, 255, 0),
            clouds: Color::BLACK,
            sun: Color::BLACK,
        }
    }
}

/// Texture slots of a preset. File names keep the historical asset names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureSlot {
    Wall,
    Floor,
    Ceiling,
    Grid,
    Coin,
    End,
    Sky,
}

impl TextureSlot {
    pub const ALL: [TextureSlot; 7] = [
        TextureSlot::Wall,
        TextureSlot::Floor,
        TextureSlot::Ceiling,
        TextureSlot::Grid,
        TextureSlot::Coin,
        TextureSlot::End,
        TextureSlot::Sky,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            TextureSlot::Wall => "wall.png",
            TextureSlot::Floor => "ground.png",
            TextureSlot::Ceiling => "ceiling.png",
            TextureSlot::Grid => "grid.png",
            TextureSlot::Coin => "coin.png",
            TextureSlot::End => "end.png",
            TextureSlot::Sky => "sky.png",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TextureSlot::Wall => "Wall",
            TextureSlot::Floor => "Floor",
            TextureSlot::Ceiling => "Ceiling",
            TextureSlot::Grid => "Grid",
            TextureSlot::Coin => "Coin",
            TextureSlot::End => "End",
            TextureSlot::Sky => "Sky",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PresetTextures {
    pub wall: Option<Texture>,
    pub floor: Option<Texture>,
    pub ceiling: Option<Texture>,
    /// Semi-transparent block (`X`), alpha < 255 texels let the background through.
    pub grid: Option<Texture>,
    /// Billboard sprite.
    pub coin: Option<Texture>,
    /// Billboard sprite; falls back to `assets/common/end.png`.
    pub end: Option<Texture>,
    pub sky: Option<Texture>,
}

impl PresetTextures {
    pub fn get(&self, slot: TextureSlot) -> Option<&Texture> {
        match slot {
            TextureSlot::Wall => self.wall.as_ref(),
            TextureSlot::Floor => self.floor.as_ref(),
            TextureSlot::Ceiling => self.ceiling.as_ref(),
            TextureSlot::Grid => self.grid.as_ref(),
            TextureSlot::Coin => self.coin.as_ref(),
            TextureSlot::End => self.end.as_ref(),
            TextureSlot::Sky => self.sky.as_ref(),
        }
    }

    pub fn get_mut(&mut self, slot: TextureSlot) -> &mut Option<Texture> {
        match slot {
            TextureSlot::Wall => &mut self.wall,
            TextureSlot::Floor => &mut self.floor,
            TextureSlot::Ceiling => &mut self.ceiling,
            TextureSlot::Grid => &mut self.grid,
            TextureSlot::Coin => &mut self.coin,
            TextureSlot::End => &mut self.end,
            TextureSlot::Sky => &mut self.sky,
        }
    }
}

/// Serialized form of a preset (`preset.json`). Textures are not inside: they
/// are the conventional PNG files next to it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetFile {
    pub name: String,
    #[serde(default)]
    pub outside: bool,
    #[serde(default)]
    pub fog: bool,
    #[serde(default)]
    pub colors: PresetColors,
}

#[derive(Clone, Debug)]
pub struct Preset {
    pub name: String,
    pub colors: PresetColors,
    /// Open sky (sun/clouds, sky colour on top) instead of a ceiling.
    pub outside: bool,
    /// Distance fog towards the sky colour (only meaningful when `outside`).
    pub fog: bool,
    pub textures: PresetTextures,
    /// Folder the preset was loaded from, if any.
    pub dir: Option<PathBuf>,
}

impl Preset {
    /// Colours and flags only (no textures). Unknown names get the defaults.
    pub fn builtin(name: &str) -> Preset {
        let c = Color::rgb;
        let mut colors = PresetColors::default();
        let (outside, fog) = match name {
            "house" => {
                colors.sky = c(38, 98, 199);
                colors.clouds = Color::WHITE;
                colors.sun = Color::WHITE;
                colors.floor = c(0, 130, 71);
                colors.wall = c(100, 100, 100);
                (true, false)
            }
            "cave" => {
                colors.sky = c(100, 100, 100);
                colors.floor = c(100, 100, 100);
                colors.wall = c(50, 50, 50);
                (false, false)
            }
            "garden" => {
                colors.sky = c(135, 206, 235);
                colors.floor = c(20, 200, 71);
                colors.wall = c(100, 116, 52);
                colors.clouds = Color::WHITE;
                colors.sun = Color::WHITE;
                (true, false)
            }
            "halloween" => {
                colors.sky = c(170, 120, 20);
                colors.clouds = c(150, 140, 40);
                colors.sun = c(115, 65, 0);
                colors.floor = c(20, 20, 28);
                colors.wall = c(76, 44, 100);
                (true, false)
            }
            "night" => {
                colors.sky = c(12, 20, 69);
                colors.clouds = c(30, 30, 70);
                colors.sun = c(220, 220, 50);
                colors.floor = c(92, 60, 44);
                colors.wall = c(47, 45, 7);
                (true, true)
            }
            "nether" => {
                colors.sky = c(172, 32, 32);
                colors.floor = c(48, 24, 28);
                colors.wall = c(116, 52, 52);
                (true, true)
            }
            "retro" => {
                colors.sky = Color::BLACK;
                colors.floor = Color::BLACK;
                colors.wall = Color::WHITE;
                colors.clouds = Color::BLACK;
                colors.sun = Color::BLACK;
                (false, false)
            }
            "snow" => {
                colors.sky = c(240, 240, 240);
                colors.wall = c(200, 200, 200);
                colors.floor = Color::WHITE;
                colors.clouds = Color::WHITE;
                colors.sun = Color::WHITE;
                (true, true)
            }
            _ => (false, false),
        };
        // indoors, the old renderer painted the ceiling with the sky colour
        colors.ceiling = colors.sky;
        Preset { name: name.to_string(), colors, outside, fog, textures: PresetTextures::default(), dir: None }
    }

    /// Loads a preset folder: `preset.json` if present (else built-in colours
    /// for the folder name), then every conventional texture file found.
    /// Never fails: broken files are reported on stderr and skipped.
    pub fn load_dir(dir: &Path, common: &CommonAssets) -> Preset {
        let folder_name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut preset = Preset::builtin(&folder_name);
        let meta_path = dir.join(PRESET_FILE);
        if meta_path.is_file() {
            match fs::read_to_string(&meta_path).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<PresetFile>(&s).map_err(|e| e.to_string())) {
                Ok(meta) => {
                    preset.name = meta.name;
                    preset.outside = meta.outside;
                    preset.fog = meta.fog;
                    preset.colors = meta.colors;
                }
                Err(e) => eprintln!("Warning: {}: {}", meta_path.display(), e),
            }
        }
        for slot in TextureSlot::ALL {
            let path = dir.join(slot.file_name());
            if path.is_file() {
                match Texture::load(&path) {
                    Ok(t) => *preset.textures.get_mut(slot) = Some(t),
                    Err(e) => eprintln!("Warning: {}", e),
                }
            }
        }
        if preset.textures.end.is_none() {
            preset.textures.end = common.end.clone();
        }
        preset.dir = Some(dir.to_path_buf());
        preset
    }

    pub fn to_file(&self) -> PresetFile {
        PresetFile { name: self.name.clone(), outside: self.outside, fog: self.fog, colors: self.colors }
    }

    /// Writes `preset.json` and every present texture (as PNG) into `dir`,
    /// creating it if needed. Stale texture files of now-empty slots are removed.
    pub fn save_to_dir(&self, dir: &Path) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
        let json = serde_json::to_string_pretty(&self.to_file()).map_err(|e| e.to_string())?;
        fs::write(dir.join(PRESET_FILE), json).map_err(|e| format!("{}: {}", dir.display(), e))?;
        for slot in TextureSlot::ALL {
            let path = dir.join(slot.file_name());
            match self.textures.get(slot) {
                Some(t) => t.save_png(&path)?,
                None if path.is_file() => fs::remove_file(&path).map_err(|e| format!("{}: {}", path.display(), e))?,
                None => {}
            }
        }
        Ok(())
    }
}

/// Images shared by every preset (`assets/common/`).
#[derive(Clone, Debug, Default)]
pub struct CommonAssets {
    pub sun: Option<Texture>,
    pub clouds: Vec<Texture>,
    pub end: Option<Texture>,
}

impl CommonAssets {
    pub fn load(dir: &Path) -> CommonAssets {
        let load = |name: &str| {
            let path = dir.join(name);
            match Texture::load(&path) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("Warning: {}", e);
                    None
                }
            }
        };
        CommonAssets {
            sun: load("sun.png"),
            clouds: ["cloud1.png", "cloud2.png", "cloud3.png"].iter().filter_map(|n| load(n)).collect(),
            end: load("end.png"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PresetEntry {
    /// Folder name (or built-in name), used as the preset id in `Settings`.
    pub name: String,
    pub dir: Option<PathBuf>,
}

/// List of available presets: built-ins first, then any other folder of
/// `assets/presets/` (custom presets), alphabetically. Textures are only
/// loaded on `load`.
pub struct PresetLibrary {
    pub presets_dir: PathBuf,
    pub entries: Vec<PresetEntry>,
}

impl PresetLibrary {
    pub fn scan(presets_dir: &Path) -> PresetLibrary {
        let mut entries: Vec<PresetEntry> = BUILTIN_PRESETS
            .iter()
            .map(|n| {
                let d = presets_dir.join(n);
                PresetEntry { name: n.to_string(), dir: if d.is_dir() { Some(d) } else { None } }
            })
            .collect();
        let mut others: Vec<PresetEntry> = fs::read_dir(presets_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .filter_map(|p| {
                        let name = p.file_name()?.to_string_lossy().into_owned();
                        if BUILTIN_PRESETS.contains(&name.as_str()) {
                            None
                        } else {
                            Some(PresetEntry { name, dir: Some(p) })
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        others.sort_by(|a, b| a.name.cmp(&b.name));
        entries.extend(others);
        PresetLibrary { presets_dir: presets_dir.to_path_buf(), entries }
    }

    pub fn rescan(&mut self) {
        *self = PresetLibrary::scan(&self.presets_dir.clone());
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.name == name)
    }

    /// Loads entry `index` (clamped) with its textures.
    pub fn load(&self, index: usize, common: &CommonAssets) -> Preset {
        match self.entries.get(index.min(self.entries.len().saturating_sub(1))) {
            Some(PresetEntry { dir: Some(dir), .. }) => Preset::load_dir(dir, common),
            Some(PresetEntry { name, dir: None }) => {
                let mut p = Preset::builtin(name);
                p.textures.end = common.end.clone();
                p
            }
            None => Preset::builtin(""),
        }
    }

    /// Saves `preset` as `assets/presets/<sanitized name>/` and rescans.
    /// Returns the folder (its name is the new entry's id).
    pub fn save(&mut self, preset: &Preset) -> Result<PathBuf, String> {
        let folder: String = preset
            .name
            .trim()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '_' })
            .collect();
        if folder.is_empty() {
            return Err("preset name is empty".to_string());
        }
        let dir = self.presets_dir.join(&folder);
        preset.save_to_dir(&dir)?;
        self.rescan();
        Ok(dir)
    }
}
