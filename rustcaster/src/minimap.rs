
// ooooooooo.                            .     .oooooo.                          .    o8o                         
// `888   `Y88.                        .o8    d8P'  `Y8b                       .o8    `"'                         
//  888   .d88' oooo  oooo   .oooo.o .o888oo 888           .oooo.    .oooo.o .o888oo oooo  ooo. .oo.    .oooooooo 
//  888ooo88P'  `888  `888  d88(  "8   888   888          `P  )88b  d88(  "8   888   `888  `888P"Y88b  888' `88b  
//  888`88b.     888   888  `"Y88b.    888   888           .oP"888  `"Y88b.    888    888   888   888  888   888  
//  888  `88b.   888   888  o.  )88b   888 . `88b    ooo  d8(  888  o.  )88b   888 .  888   888   888  `88bod8P'  
// o888o  o888o  `V88V"V8P' 8""888P'   "888"  `Y8bood8P'  `Y888""8o 8""888P'   "888" o888o o888o o888o `8oooooo.  
//                                                                                                     d"     YD  
//                                                                                                     "Y88888P'  

use crate::Player;
use crate::Game;
use crate::Color;
use crate::draw_filled_circle;
use crate::draw_rays;
use crate::rustcaster::BLOCK_SIZE;
use crate::rustcaster::PLAYER_SIZE;

fn is_x_in_map(game_t: &Game, x: i32) -> bool
{
    if x < 0 || x >= game_t.map_width as i32
        { return false; }
    return true;
}

fn is_y_in_map(game_t: &Game, y: i32) -> bool
{
    if y < 0 || y >= game_t.map_height as i32
        { return false; }
    return true;
}

pub fn is_in_map(game_t: &Game, x: i32, y: i32, orientation: i32) -> bool
{
    if orientation == 0
        { return is_x_in_map(game_t, x); }
    else if orientation == 1
        { return is_y_in_map(game_t, y); }
    else
        { return is_x_in_map(game_t, x) && is_y_in_map(game_t, y); }
}

fn draw_minimap_walls(game_t: &mut Game, player: &Player, x: i32, y: i32)
{
    let point_color;
    let block_x_pos = x * BLOCK_SIZE - (player.pos.x) as i32 + game_t.map_zone.center().x;
    let block_y_pos = y * BLOCK_SIZE - (player.pos.y) as i32 + game_t.map_zone.center().y;

    if x < 0 || y < 0 || x >= game_t.map_width as i32 || y >= game_t.map_height as i32
        { return; }
    let c = game_t.ascii_map[y as usize][x as usize];
    if c == '#' || c == 'X' || c == 'E' {
        if c == '#'
            { point_color = game_t.environnement.walls_color; }
        else if c == 'X'
            { point_color = game_t.environnement.grid_color; }
        else
            { point_color = Color::RGB(0, 255, 0); }
        let rect = sdl2::rect::Rect::new(block_x_pos, block_y_pos, BLOCK_SIZE as u32, BLOCK_SIZE as u32);
        let intersection = rect.intersection(game_t.map_zone);
        if intersection.is_some() {
            game_t.canvas.set_draw_color(point_color);
            let _ = game_t.canvas.fill_rect(intersection);
        }
    }
}

pub fn draw_minimap(game_t: &mut Game, player: &Player)
{
    let start_x = (player.pos.x as i32 / BLOCK_SIZE) - 10;
    let end_x = (player.pos.x as i32 / BLOCK_SIZE) + 10;
    let start_y = (player.pos.y as i32 / BLOCK_SIZE ) - 10;
    let end_y = (player.pos.y as i32 / BLOCK_SIZE) + 10;

    for y in start_y..= end_y {
        for x in start_x..= end_x {
            draw_minimap_walls(game_t, player, x, y);
        }
    }
    draw_rays(game_t, player);
    game_t.canvas.set_draw_color(Color::RGB(255, 0, 0));
    draw_filled_circle(&mut game_t.canvas, game_t.map_zone.center().x, game_t.map_zone.center().y, PLAYER_SIZE);
}
