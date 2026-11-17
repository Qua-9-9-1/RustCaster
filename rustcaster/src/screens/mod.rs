// One module per non-gameplay `AppState` (Playing is handled in app.rs).
// Screen contract:
//   fn update(&mut self, ctx: &mut AppContext, input: &Input, fb_size: (usize, usize)) -> Transition
//   fn draw(&self, ctx: &AppContext, fb: &mut FrameBuffer, input: &Input)

pub mod menu;
pub mod pause;
pub mod preset_editor;
