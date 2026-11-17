// Application state machine.
//
//   Menu ──> LevelSelect ──StartGame──> Playing <──Esc / HUD──> Paused ──> Menu
//    │  └──> Settings          │                        (Restart: RestartGame)
//    │                         └── exit reached ──> LevelComplete ──> Playing (next / replay) | Menu
//    └──> PresetEditor ──> Menu (presets rescanned)
//
// Each screen (see `screens/`) gets the shared `AppContext`, returns a
// `Transition`, and `App::apply` performs it. Adding a screen = a new
// `AppState` variant + a struct in `screens/` + two match arms below.

use std::fs;
use std::path::{Path, PathBuf};

use sdl2::keyboard::Scancode;

use crate::assets::preset::{CommonAssets, Preset, PresetLibrary};
use crate::config::{COMMON_ASSETS_DIR, MAPS_DIR, PRESETS_DIR};
use crate::game::GameState;
use crate::input::Input;
use crate::render::framebuffer::FrameBuffer;
use crate::render::minimap::draw_minimap;
use crate::render::raycaster::{Camera, FlatRenderer, Raycaster, View, WorldRenderer};
use crate::screens::level_complete::{LevelCompleteScreen, LevelResult};
use crate::screens::level_select::LevelSelectScreen;
use crate::screens::menu::MenuScreen;
use crate::screens::pause::PauseScreen;
use crate::screens::preset_editor::PresetEditorScreen;
use crate::screens::settings::SettingsScreen;
use crate::settings::Settings;
use crate::ui::hud::{draw_hud, update_hud};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppState {
    Menu,
    Playing,
    Paused,
    PresetEditor,
    LevelSelect,
    Settings,
    /// End-of-level overlay (the finished game stays loaded behind it).
    LevelComplete,
}

/// What a screen asks the app to do after its update.
#[derive(Clone, Debug)]
pub enum Transition {
    None,
    /// Switch screen. `Playing` / `Paused` are ignored when no game is loaded.
    To(AppState),
    /// Load the map and switch to `Playing` (errors go to `ctx.message` and stay in Menu).
    StartGame(PathBuf),
    /// Reload the current game's map from the start (menu if no game).
    RestartGame,
    /// Drop the current game and go back to the menu.
    QuitToMenu,
    Quit,
}

/// State shared by every screen.
pub struct AppContext {
    /// Folder containing `assets/` and `maps/`.
    pub data_root: PathBuf,
    pub settings: Settings,
    pub presets: PresetLibrary,
    /// Loaded preset matching `settings.preset` (textures included).
    pub preset: Preset,
    pub common: CommonAssets,
    /// `*.txt` files of `maps/`, sorted.
    pub maps: Vec<PathBuf>,
    /// One-line status shown by the menu (errors, level results).
    pub message: Option<String>,
}

impl AppContext {
    pub fn new(data_root: PathBuf) -> AppContext {
        let common = CommonAssets::load(&data_root.join(COMMON_ASSETS_DIR));
        let presets = PresetLibrary::scan(&data_root.join(PRESETS_DIR));
        let settings = Settings::default();
        let index = presets.index_of(&settings.preset).unwrap_or(0);
        let preset = presets.load(index, &common);
        let mut ctx = AppContext { data_root, settings, presets, preset, common, maps: Vec::new(), message: None };
        ctx.rescan_maps();
        ctx
    }

    pub fn rescan_maps(&mut self) {
        let dir = self.data_root.join(MAPS_DIR);
        self.maps = fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.is_file() && p.extension().map_or(false, |e| e.eq_ignore_ascii_case("txt")))
                    .collect()
            })
            .unwrap_or_default();
        self.maps.sort();
    }

    pub fn preset_index(&self) -> usize {
        self.presets.index_of(&self.settings.preset).unwrap_or(0)
    }

    /// Selects and loads a preset by library index (wraps around).
    pub fn select_preset(&mut self, index: usize) {
        if self.presets.len() == 0 {
            return;
        }
        let index = index % self.presets.len();
        self.settings.preset = self.presets.entries[index].name.clone();
        self.preset = self.presets.load(index, &self.common);
    }

    /// Next (+1) / previous (-1) preset.
    pub fn cycle_preset(&mut self, delta: i32) {
        let n = self.presets.len() as i32;
        if n == 0 {
            return;
        }
        let i = (self.preset_index() as i32 + delta).rem_euclid(n);
        self.select_preset(i as usize);
    }

    /// Resolves a map path given on the command line: as-is if it exists,
    /// else relative to the data root.
    pub fn resolve_map_path(&self, arg: &Path) -> PathBuf {
        if arg.exists() {
            arg.to_path_buf()
        } else {
            self.data_root.join(arg)
        }
    }
}

pub struct App {
    pub state: AppState,
    pub ctx: AppContext,
    pub game: Option<GameState>,
    raycaster: Raycaster,
    /// Swap this for a textured renderer (see ARCHITECTURE.md).
    world_renderer: Box<dyn WorldRenderer>,
    menu: MenuScreen,
    pause: PauseScreen,
    preset_editor: PresetEditorScreen,
    level_select: LevelSelectScreen,
    settings: SettingsScreen,
    level_complete: LevelCompleteScreen,
    quit: bool,
}

impl App {
    /// `cli_map`: optional map given on the command line, played immediately.
    pub fn new(data_root: PathBuf, cli_map: Option<PathBuf>) -> App {
        let mut app = App {
            state: AppState::Menu,
            ctx: AppContext::new(data_root),
            game: None,
            raycaster: Raycaster::new(),
            world_renderer: Box::new(FlatRenderer),
            menu: MenuScreen::new(),
            pause: PauseScreen::new(),
            preset_editor: PresetEditorScreen::new(),
            level_select: LevelSelectScreen::new(),
            settings: SettingsScreen::new(),
            level_complete: LevelCompleteScreen::new(),
            quit: false,
        };
        if let Some(arg) = cli_map {
            let path = app.ctx.resolve_map_path(&arg);
            app.apply(Transition::StartGame(path));
        }
        app
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn update(&mut self, input: &Input, dt: f64, fb_size: (usize, usize)) {
        let transition = match self.state {
            AppState::Menu => self.menu.update(&mut self.ctx, input, fb_size),
            AppState::PresetEditor => self.preset_editor.update(&mut self.ctx, input, fb_size),
            AppState::Paused => self.pause.update(&mut self.ctx, input, fb_size),
            AppState::Playing => self.update_playing(input, dt, fb_size),
            AppState::LevelSelect => self.level_select.update(&mut self.ctx, input, fb_size),
            AppState::Settings => self.settings.update(&mut self.ctx, input, fb_size),
            AppState::LevelComplete => self.level_complete.update(&mut self.ctx, input, fb_size),
        };
        self.apply(transition);
    }

    fn update_playing(&mut self, input: &Input, dt: f64, fb_size: (usize, usize)) -> Transition {
        let game = match self.game.as_mut() {
            Some(g) => g,
            None => return Transition::To(AppState::Menu),
        };
        let pause_clicked = update_hud(input, fb_size, &mut self.ctx.settings, game.map.total_coins);
        if input.was_pressed(Scancode::Escape) || pause_clicked {
            return Transition::To(AppState::Paused);
        }
        game.update(input, &self.ctx.settings, dt);
        if game.finished {
            let result = LevelResult::new(game, &self.ctx.maps);
            self.ctx.message = Some(result.summary());
            self.level_complete.open(result);
            return Transition::To(AppState::LevelComplete);
        }
        Transition::None
    }

    pub fn apply(&mut self, transition: Transition) {
        let previous = self.state;
        let transition = match transition {
            Transition::RestartGame => match &self.game {
                Some(g) => Transition::StartGame(g.map_path.clone()),
                None => Transition::QuitToMenu,
            },
            t => t,
        };
        match transition {
            Transition::None | Transition::RestartGame => {}
            Transition::To(state) => {
                let needs_game = matches!(state, AppState::Playing | AppState::Paused | AppState::LevelComplete);
                self.state = if needs_game && self.game.is_none() { AppState::Menu } else { state };
            }
            Transition::StartGame(path) => match GameState::load(&path) {
                Ok(game) => {
                    self.game = Some(game);
                    self.ctx.message = None;
                    self.state = AppState::Playing;
                }
                Err(e) => {
                    let msg = format!("Error: {}: {}", path.display(), e);
                    eprintln!("{}", msg);
                    self.ctx.message = Some(msg);
                    // show the error where maps are picked (title screen at startup)
                    self.game = None;
                    self.state = if previous == AppState::Menu { AppState::Menu } else { AppState::LevelSelect };
                }
            },
            Transition::QuitToMenu => {
                self.game = None;
                self.state = AppState::Menu;
            }
            Transition::Quit => self.quit = true,
        }
        if self.state != previous {
            self.enter_state(previous);
        }
    }

    /// Per-screen setup when `self.state` was just entered from `previous`.
    fn enter_state(&mut self, previous: AppState) {
        if previous == AppState::PresetEditor {
            crate::screens::refresh_presets(&mut self.ctx);
        }
        match self.state {
            AppState::LevelSelect => self.level_select.on_enter(&mut self.ctx),
            AppState::Paused => self.pause.on_enter(),
            _ => {}
        }
    }

    pub fn render(&mut self, fb: &mut FrameBuffer, input: &Input) {
        match self.state {
            AppState::Menu => self.menu.draw(&self.ctx, fb, input),
            AppState::PresetEditor => self.preset_editor.draw(&self.ctx, fb, input),
            AppState::LevelSelect => self.level_select.draw(&self.ctx, fb, input),
            AppState::Settings => self.settings.draw(&self.ctx, fb, input),
            AppState::Playing => self.render_world(fb, input),
            AppState::Paused => {
                self.render_world(fb, input);
                self.pause.draw(&self.ctx, fb, input);
            }
            AppState::LevelComplete => {
                self.render_world(fb, input);
                self.level_complete.draw(&self.ctx, fb, input);
            }
        }
    }

    /// 3D view + minimap + HUD of the current game.
    fn render_world(&mut self, fb: &mut FrameBuffer, input: &Input) {
        let game = match self.game.as_ref() {
            Some(g) => g,
            None => return,
        };
        let view = View {
            camera: Camera::new(&game.player, self.ctx.settings.fov_rad(), fb.width(), fb.height()),
            map: &game.map,
            preset: &self.ctx.preset,
            common: &self.ctx.common,
            time: game.time,
        };
        self.raycaster.render(fb, &view, self.world_renderer.as_mut());
        if self.ctx.settings.show_minimap {
            draw_minimap(fb, &game.map, &game.player, &self.ctx.preset, &self.raycaster.ray_ends);
        }
        draw_hud(fb, game, &self.ctx.settings, &self.ctx.preset, input);
    }
}
