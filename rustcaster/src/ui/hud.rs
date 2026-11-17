// In-game HUD drawn over the 3D view (placeholder: coin counter only).

use crate::config::MINIMAP_MARGIN;
use crate::game::GameState;
use crate::render::framebuffer::FrameBuffer;
use crate::render::minimap::minimap_rect;
use crate::settings::Settings;
use crate::ui::font::draw_text_shadow;
use crate::ui::widgets::{text_scale, TEXT_COLOR};

pub fn draw_hud(fb: &mut FrameBuffer, game: &GameState, settings: &Settings) {
    let scale = text_scale(fb.height());
    let y = if settings.show_minimap { minimap_rect(fb.height()).bottom() + MINIMAP_MARGIN } else { MINIMAP_MARGIN };
    let text = format!("Coins {}/{}", game.coins_collected, game.map.total_coins);
    draw_text_shadow(fb, &text, MINIMAP_MARGIN, y, scale, TEXT_COLOR);
}
