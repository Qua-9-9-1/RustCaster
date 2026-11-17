// One module per non-gameplay `AppState` (Playing is handled in app.rs).
// Screen contract:
//   fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition
//   fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input)

pub mod level_complete;
pub mod level_select;
pub mod menu;
pub mod pause;
pub mod preset_editor;
pub mod settings;

use crate::app::AppContext;

/// Rescans `assets/presets/` (new / edited custom presets) and reloads the
/// selected preset from disk, falling back to the first one if it vanished.
pub fn refresh_presets(ctx: &mut AppContext) {
    ctx.presets.rescan();
    let index = ctx.presets.index_of(&ctx.settings.preset).unwrap_or(0);
    ctx.select_preset(index);
}
