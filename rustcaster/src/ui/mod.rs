// Immediate-ish UI toolkit drawn into the framebuffer (no SDL_ttf):
// embedded bitmap font, theme (palette, spacing, panels, titles), widgets,
// previews (preset postcard, coin icon, map thumbnail), HUD.
// Screens using it live in `screens/`.

pub mod font;
pub mod hud;
pub mod preview;
pub mod theme;
pub mod widgets;
