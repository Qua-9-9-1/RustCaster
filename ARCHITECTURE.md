# Architecture

Software-rendered raycaster: every frame is drawn on the CPU into a `FrameBuffer`
(ARGB `u32` pixels, fixed 480 px height, width following the window aspect ratio),
uploaded once to a streaming SDL texture and stretched to the fullscreen window.
SDL is only used for the window, events and that final upload.

```
rustcaster/
  assets/presets/<name>/   wall.png ground.png [ceiling.png] grid.png coin.png [end.png sky.png preset.json]
  assets/common/           sun.png cloud1-3.png end.png (shared by all presets)
  maps/                    *.txt levels (listed by the menu)
  src/
    main.rs        SDL init, window, frame loop (60 FPS limiter, dt), Presenter
    app.rs         App, AppState, Transition, AppContext (shared state)
    config.rs      constants + data root lookup (cwd, then exe dir and parents)
    settings.rs    Settings: fov, sensitivity, preset, minimap (clamped setters)
    input.rs       Input: per-frame snapshot from SDL events
    game/          map.rs (Map, Tile, MapError), player.rs (movement/collision), mod.rs (GameState)
    render/        color.rs, framebuffer.rs (FrameBuffer, Rect, Presenter), raycaster.rs, minimap.rs
    assets/        texture.rs (Texture), preset.rs (Preset, PresetLibrary, CommonAssets)
    ui/            font.rs (embedded 8x8 font), widgets.rs (Button, Label, layout), hud.rs
    screens/       menu.rs, pause.rs, preset_editor.rs (minimal stubs)
```

Assets and maps are found from the current directory or next to / above the
executable (`config::find_data_root`), so `run.ps1` (cwd `rustcaster/`) and a
double-clicked `target/.../rustcaster.exe` both work.

## Conventions

- World units are **tiles** (`1.0` = one map cell); tile `(i, j)` covers `[i, i+1) x [j, j+1)`.
  Angles in radians, `+y` points down the map, angles grow clockwise.
- Pixels are `u32` `0xAARRGGBB` everywhere (framebuffer and textures); `Color::to_u32` / `from_u32`.
- Everything draws into the framebuffer, including UI and text (no SDL canvas drawing, no SDL_ttf).
  Mouse coordinates from `Input` are already in framebuffer pixels.
- `config::MAX_FRAME_DT` caps `dt`; movement is time-based.

## Frame flow

`main`: `input.begin_frame(..)` -> `app.update(&input, dt, fb_size)` -> `app.render(&mut fb, &input)`
-> `presenter.present(&mut canvas, &fb)` -> `canvas.present()` -> sleep the rest of the 1/60 s.

## App states

`AppState::{Menu, Playing, Paused, PresetEditor}`. A screen's `update` returns a `Transition`
(`None`, `To(AppState)`, `StartGame(PathBuf)`, `QuitToMenu`, `Quit`), applied by `App::apply`.
`Playing` is handled in `app.rs` (`update_playing`, `render_world`); the others are structs in
`screens/` with:

```rust
fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition
fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input)
```

`AppContext` holds what every screen may read/change: `settings`, `presets` (library),
`preset` (loaded, with textures), `common` assets, `maps` list, `message` (status line), `data_root`.
Use `ctx.select_preset(i)` / `ctx.cycle_preset(+-1)` to change preset (keeps `settings.preset` and the
loaded `preset` in sync). Adding a screen: new `AppState` variant, a struct in `screens/`, two match
arms in `App::update` / `App::render`.

The CLI argument (`rustcaster.exe maps/data.txt`) goes straight to `Playing`; without it the app starts
in `Menu`. Reaching an exit returns to the menu with the score in `ctx.message`.

## Core APIs

- `FrameBuffer`: `width()`, `height()`, `bounds()`, `pixels()` / `pixels_mut()` (raw rows, ignores clip),
  `set_clip(Option<Rect>)`, `clear`, `put_pixel`, `fill_rect`, `draw_rect`, `draw_vline(x, y0, y1, c)`,
  `draw_line`, `fill_circle`, `blit_texture(&Texture, Rect)`. Colours with `a < 255` are blended.
- `Texture { width, height, pixels: Vec<u32> }`: `load(&Path)`, `solid`, `save_png`, `get(x, y)` (wrapping),
  `sample(u, v)` (normalised, wrapping, nearest), `average_color`.
- `Preset { name, colors: PresetColors, outside, fog, textures: PresetTextures, dir }`;
  `PresetTextures { wall, floor, ceiling, grid, coin, end, sky: Option<Texture> }` + `get/get_mut(TextureSlot)`;
  `Preset::builtin(name)`, `Preset::load_dir(dir, &common)`, `preset.save_to_dir(dir)`.
  `PresetLibrary { presets_dir, entries }`: `scan`, `rescan`, `index_of`, `load(i, &common)`, `save(&preset)`.
- `Input`: `is_down(Scancode)`, `was_pressed`, `was_released`, `mouse_pos()`, `mouse_down(Mouse)`,
  `mouse_clicked(Mouse)`, `mouse_released`, `wheel()`, `text()` (typed text), `quit_requested()`.
- UI: `Button::new(label, rect, scale)` + `.clicked(&input)`, `.hovered`, `.draw(fb, &input)`;
  `Label::new/centered(..).draw(fb)`; `font::draw_text / draw_text_shadow / draw_text_centered / text_size`;
  layout helpers `widgets::{column, row, centered, text_scale, draw_panel}`.
  Pattern: build widgets in a `layout(ctx, fb_size)` fn called by both `update` and `draw`.

## Extension points

### (a) Textured renderer (`render/`)

`render/raycaster.rs` separates the DDA from the drawing:

- `Raycaster::render(fb, &View, &mut dyn WorldRenderer)` casts one ray per column, fills
  `depth[x]` (perpendicular distance of the solid hit, `INFINITY` if none) and `ray_ends` (minimap).
- `ColumnHits { solid: Option<RayHit>, transparent: Vec<RayHit> /* grids, nearest first */, ray_dir, ray_len }`,
  `RayHit { tile, map_x, map_y, side: Side::{X,Y}, perp_dist, dist, wall_x /* texture u in [0,1) */ }`.
- `View { camera: Camera, map, preset, common, time }`. `Camera` gives `dir`, `plane`, `focal`, `horizon`,
  `ray_dir(x)`, `wall_height(perp)`, `project(wx, wy) -> (screen_x, depth)` for billboards and
  `project_angle(a)` for sky objects. Floor row `y` is at distance `0.5 * focal / (y - horizon)`.
- Implement `WorldRenderer` in e.g. `render/textured.rs`:
  - `draw_background`: sky (`preset.textures.sky`, `common.sun/clouds` scrolling with `camera.angle`) when
    `preset.outside`, otherwise floor/ceiling casting with `floor` / `ceiling` textures (pixels are faster
    through `fb.pixels_mut()`).
  - `draw_column`: textured wall from `solid` (`wall_x` -> u, `side` for darker faces / flipping), then the
    `transparent` grid hits far to near, skipping texels with alpha 0.
  - `items_as_sprites() -> true`: coins and end tiles no longer stop rays; draw them in `draw_sprites`
    (iterate `view.map` for `Tile::Coin` / `Tile::End`, project with `camera.project`, sort far to near,
    occlude with `depth`, bob coins with `view.time`). Note grids are not in `depth`.
  - Fall back to `FlatRenderer` behaviour when a texture slot is `None`; reuse `distance_shade`.
- Plug it in `App::new` (`world_renderer: Box::new(...)`); a settings toggle can switch renderers.

### (b) Menus, UI, HUD (`screens/`, `ui/`)

- `screens/menu.rs` is a stub: map buttons from `ctx.maps`, preset `<`/`>` (`ctx.cycle_preset`), preset editor,
  quit. Replace freely (thumbnails, map preview from `Map::load`, preset preview with `blit_texture`).
- `screens/pause.rs` is the in-game settings stub: FOV / sensitivity steppers (`Settings::adjust_fov`,
  `adjust_sensitivity`, which clamp), preset, minimap toggle, main menu (`Transition::QuitToMenu`), quit.
  The old keyboard hacks (U/J, I/K, numpad presets) were removed in favour of these.
- `ui/hud.rs::draw_hud(fb, game, settings)` draws the coin counter under the minimap: extend it (timer
  from `game.time`, crosshair, end-of-level screen instead of returning to the menu).
- Persisting settings (e.g. `settings.json` with serde, already a dependency) is not done yet.

### (c) Custom preset creator (`screens/preset_editor.rs`, `assets/preset.rs`)

- The screen is a stub showing the current preset's texture slots. Expected flow: start from
  `ctx.preset.clone()` or `Preset::builtin(..)`, set `textures.get_mut(slot) = Some(Texture::load(path)?)`
  for user images (paths typed with `input.text()`, or dropped files: add `Event::DropFile` handling to
  `Input`), edit `colors` / `outside` / `fog`, then `ctx.presets.save(&preset)` (writes
  `assets/presets/<sanitized name>/preset.json` + PNGs and rescans) and `ctx.select_preset(index)`.
- `preset.json` format (`PresetFile`): `{"name", "outside", "fog", "colors": {"sky": "#RRGGBB", "ceiling",
  "floor", "wall", "grid", "coin", "end", "clouds", "sun"}}`; missing colours default to
  `PresetColors::default()`. Folders without it use `Preset::builtin(folder_name)` colours.

## Known gaps

- `Tile::Monster` is parsed but ignored (walkable, not drawn), like before.
- `PresetLibrary::save` overwrites an existing folder of the same name, built-ins included.
- The data root falls back to the cwd if no `assets/` + `maps/` pair is found; errors then show in the menu.
