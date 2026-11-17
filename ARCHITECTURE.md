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
    render/        color.rs, framebuffer.rs (FrameBuffer, Rect, Presenter), raycaster.rs, textured.rs, minimap.rs
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

`AppState::{Menu, LevelSelect, Settings, Playing, Paused, LevelComplete, PresetEditor}`. A screen's
`update` returns a `Transition` (`None`, `To(AppState)`, `StartGame(PathBuf)`, `RestartGame`,
`QuitToMenu`, `Quit`), applied by `App::apply`.
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
in `Menu`. Reaching an exit opens `LevelComplete` (score, Next map / Replay / Menu).

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

Implemented in `render/textured.rs` (`TexturedRenderer`, the default in `App::new`; `FlatRenderer` stays
available):
- Empty texture slots use a 1x1 texture of the slot's flat colour (grid: alpha 160), so missing textures
  degrade per slot to the flat look.
- Shading: `distance_shade` as a lookup table (1/32 tile steps) of integer ops (`ShadeOp`: `rgb * mul + add`),
  continuous instead of stepped; Y faces x0.75. Euclidean distance everywhere (floor: row distance * |ray_dir|).
- Walls / grids: `Face` = one textured column span (fixed-point v). Texture u is flipped on X faces hit with
  dx < 0 and Y faces hit with dy > 0, so faces are never mirrored seen from outside.
- Floor / ceiling: cast per column below / above the wall with per-row distances computed once per frame
  (16.16 fixed-point world coordinates). Indoors the ceiling is cast too; outdoors `draw_background` paints the sky.
- Sky: `textures.sky` if present (u = view angle over 360 deg, v = row over the sky height), else a gradient
  from `colors.sky` + `common.sun` / clouds placed by world angle and elevation (they wrap and turn with the
  camera, clouds drift with `view.time`), tinted with `colors.sun` / `colors.clouds` (black = untinted),
  half-hidden with fog.
- Grids: `draw_column` only records the faces (back face where the ray leaves the tile, unless a wall/grid
  follows, then front face); `draw_sprites` draws them merged far to near with the billboards, per column, so
  coins behind grids are correct. Texel alpha decides (0 = see-through, partial = blended).
- Sprites (`items_as_sprites() == true`): coins / end tiles within `MAX_RAY_DIST` (tile scan around the
  camera), coins float and bob, fake-spin by scaling their width; the end flag stands on the floor. Tunables
  are the constants at the top of the file.

### (b) Menus, UI, HUD (`screens/`, `ui/`)

Navigation (extra states `LevelSelect`, `Settings`, `LevelComplete`; extra transition `RestartGame`):

```
Menu (title) ──Play──> LevelSelect ──Start──> Playing <──Esc / HUD pause──> Paused
  │  ├──Settings──> Settings                     │                          ├─ Settings page (same panel)
  │  └──Create preset──> PresetEditor ──> Menu   │                          ├─ style < > (applies at once)
  │                    (presets rescanned)       │                          └─ Restart / Main menu / Quit
  └──Quit                                        └── exit reached ──> LevelComplete ──> Next map | Replay | Menu
```

- `ui/theme.rs`: the look. Palette constants (`ACCENT` gold, `TEXT*`, `BUTTON_*`, `PANEL_*`, `OVERLAY_DIM`),
  `Metrics` (`metrics(fb_height)`: `s` text scale, `pad`, `gap`, `button_h`, `margin`, `small`), and primitives
  `draw_background` (animated gradient + perspective grid), `draw_panel` (shadow, translucent fill, border),
  `draw_titled_panel` / `titled_panel_content`, `draw_title` (big two-tone shadowed text), `draw_text_soft`,
  `draw_message` (`ctx.message`, red for `Error...`, green otherwise), `fit_scale`, `ellipsize`.
  Lay out in multiples of `m.s` so screens work at any framebuffer height; widths follow the aspect ratio.
- `ui/widgets.rs`: `Button` (`.primary()`, `.selected(b)`, `.focused(b)`, `.icon(Icon)`, `.enabled(b)`; draws
  normal / hover / pressed / disabled faces, shrinks its label to fit), `Focus` (Up/Down/Left/Right/Tab +
  Enter/Space keyboard navigation over a button list, returns the activated index, clicks included),
  `scrollbar` / `draw_scrollbar`, `Label`, `column`, `row`, `centered`. Old constants (`TEXT_COLOR`...) remain.
- `ui/preview.rs`: `draw_preset_preview` (sky/ceiling, floor, wall, coin with the preset textures or colours),
  `draw_palette`, `draw_coin_icon` (coin texture or drawn disc), `draw_map_thumbnail`.
- Screens: `menu.rs` (title), `level_select.rs` (map list parsed with `Map::load` on enter: size, coins,
  thumbnail; wheel / scrollbar / Up-Down; style carousel with Left-Right), `settings.rs` (`SettingsPanel`
  component: FOV and turn speed sliders with -/+ through the clamped `Settings` setters, minimap toggle; and the
  full-screen `SettingsScreen`), `pause.rs` (main page + settings page), `level_complete.rs` (`LevelResult`:
  coins, time, score = 100/coin + 10/s under 2 min + 500 for all coins; next map = following entry of `ctx.maps`).
- `App::enter_state` runs per-screen setup: `LevelSelect` rescans maps, `Paused` resets to its main page, and
  leaving `PresetEditor` calls `screens::refresh_presets` (rescan + reload the selected preset from disk).
- HUD (`ui/hud.rs`): coin counter with icon, timer, minimap toggle and pause buttons (top-right,
  `update_hud` handles clicks), map name, fading controls hint. `render/minimap.rs` draws the framed minimap.
- No keyboard shortcut changes parameters: everything is in the settings / pause menus.
- Persisting settings (e.g. `settings.json` with serde, already a dependency) is not done yet.

### (c) Custom preset creator (`screens/preset_editor.rs`, `assets/preset.rs`)

- Storage: shipped presets live in `assets/presets/` and are never written by the game. User presets
  live in `<data root>/presets/<id>/` (`USER_PRESETS_DIR`, next to `assets/` and `maps/`), same layout
  (conventional PNGs + `preset.json`). `PresetLibrary::scan` lists built-ins, then other shipped folders,
  then user folders (`PresetEntry::user == true`); a user folder named like a shipped preset is ignored.
  The id is `folder_name_for(name)` (lowercase ASCII, `-`, `_`, max 32 chars, Windows reserved names
  suffixed). `check_name(name, replacing)` validates; `save(&preset)` creates a new user preset and
  refuses any taken id; `save_as(&preset, Some(id))` may overwrite user preset `id` (a different name
  saves a copy); `delete(id)` removes a user preset (built-ins refused). All of them rescan.
- Screen (`screens/preset_editor.rs` + `preset_editor/{text_field, import, preview}.rs`): on entry the
  working copy is `ctx.preset` (a user preset is edited in place, a shipped one is cloned under a free
  name like "my cave"); "Start from" + "Load" switches base (or "(empty)"). One card per `TextureSlot`:
  thumbnail, flat colour, "Import" (blocking native dialog via `rfd`, owned by the SDL window through
  `GetActiveWindow`; input is ignored for a few frames afterwards) and "Clear". Imports are capped at
  64 MB, downscaled to 256 px max (sky 512), flattened to opaque for wall/floor/ceiling/sky (grid, coin,
  end keep alpha) and set the slot colour to `average_color`. Clicking a card selects its colour for the
  RGB sliders / palette / "From texture" panel; the "Sky extras" card holds sun and cloud colours.
  Outside/Fog toggles, live preview (`preview.rs`, small fixed corridor raycaster), Save (Ctrl+S),
  Delete, "Open presets folder" (explorer), Back/Escape (asks again when unsaved). After saving, the
  preset is selected with `ctx.select_preset` and `ctx.message` is set for the menu.
- Texture helpers used by the import: `Texture::resized`, `fit_within(max_side)`, `flattened(bg)`,
  `has_transparency`.
- `preset.json` format (`PresetFile`): `{"name", "outside", "fog", "colors": {"sky": "#RRGGBB", "ceiling",
  "floor", "wall", "grid", "coin", "end", "clouds", "sun"}}`; missing colours default to
  `PresetColors::default()`. Folders without it use `Preset::builtin(folder_name)` colours.

## Known gaps

- `Tile::Monster` is parsed but ignored (walkable, not drawn), like before.
- The data root falls back to the cwd if no `assets/` + `maps/` pair is found; errors then show in the menu.
