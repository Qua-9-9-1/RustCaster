// Application state machine.
//
//   Menu ──StartGame──> Playing <──Esc──> Paused ──> Menu
//    │                     └── level finished ──> Menu
//    └──> PresetEditor ──> Menu
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
use crate::screens::menu::MenuScreen;
use crate::screens::pause::PauseScreen;
use crate::screens::preset_editor::PresetEditorScreen;
use crate::settings::Settings;
use crate::ui::hud::draw_hud;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppState {
    Menu,
    Playing,
    Paused,
    PresetEditor,
}

/// What a screen asks the app to do after its update.
#[derive(Clone, Debug)]
pub enum Transition {
    None,
    /// Switch screen. `Playing` / `Paused` are ignored when no game is loaded.
    To(AppState),
    /// Load the map and switch to `Playing` (errors go to `ctx.message` and stay in Menu).
    StartGame(PathBuf),
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
            AppState::Playing => self.update_playing(input, dt),
        };
        self.apply(transition);
    }

    fn update_playing(&mut self, input: &Input, dt: f64) -> Transition {
        let game = match self.game.as_mut() {
            Some(g) => g,
            None => return Transition::To(AppState::Menu),
        };
        if input.was_pressed(Scancode::Escape) {
            return Transition::To(AppState::Paused);
        }
        game.update(input, &self.ctx.settings, dt);
        if game.finished {
            self.ctx.message = Some(format!(
                "Level complete! Coins {}/{}",
                game.coins_collected, game.map.total_coins
            ));
            return Transition::QuitToMenu;
        }
        Transition::None
    }

    pub fn apply(&mut self, transition: Transition) {
        match transition {
            Transition::None => {}
            Transition::To(state) => {
                let needs_game = matches!(state, AppState::Playing | AppState::Paused);
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
                    self.state = AppState::Menu;
                }
            },
            Transition::QuitToMenu => {
                self.game = None;
                self.state = AppState::Menu;
            }
            Transition::Quit => self.quit = true,
        }
    }

    pub fn render(&mut self, fb: &mut FrameBuffer, input: &Input) {
        match self.state {
            AppState::Menu => self.menu.draw(&self.ctx, fb, input),
            AppState::PresetEditor => self.preset_editor.draw(&self.ctx, fb, input),
            AppState::Playing => self.render_world(fb),
            AppState::Paused => {
                self.render_world(fb);
                self.pause.draw(&self.ctx, fb, input);
            }
        }
    }

    /// 3D view + minimap + HUD of the current game.
    fn render_world(&mut self, fb: &mut FrameBuffer) {
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
        draw_hud(fb, game, &self.ctx.settings);
    }
}
