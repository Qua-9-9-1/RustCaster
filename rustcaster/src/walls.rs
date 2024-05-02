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
use crate::rustcaster::BLOCK_SIZE;
use crate::rustcaster::SCREEN_WIDTH;
use crate::rustcaster::SCREEN_HEIGHT;
use crate::rustcaster::RAYS_NB;
use crate::rustcaster::FOG_INTENSITY;
use crate::rustcaster::RAYS_LENGTH;
use crate::is_in_map;
use crate::darker_color;
use crate::merge_colors;

use sdl2::pixels::Color;

pub fn draw_wall(game_t: &mut Game, player: &Player, hit_x: i32, hit_y: i32, ray_nb: i32, angle_temp: f64)
{
    let distance = f64::sqrt(f64::powi(hit_x as f64 - player.pos.x, 2) + f64::powi(hit_y as f64 - player.pos.y, 2));
    let vision_coefficient = f64::cos(player.angle - angle_temp);
    let wall_height = ((500.0 / (vision_coefficient * distance)) * 100.0) / 2.0;
    let wall_start = (SCREEN_HEIGHT / 2) - (wall_height.round() as i32 / 2) + 40;
    let ray_on_screen = ray_nb * (SCREEN_WIDTH / RAYS_NB) + 300;
    let wall_rect = sdl2::rect::Rect::new(ray_on_screen, wall_start, (SCREEN_WIDTH / RAYS_NB) as u32, wall_height.round() as u32);
    let map_hit_x = (hit_x / BLOCK_SIZE) as usize;
    let map_hit_y = (hit_y / BLOCK_SIZE) as usize;

    if game_t.environnement.outside == true {
        // if (game_t.ascii_map[map_hit_y + 1][map_hit_x] == '#' ||
        //     game_t.ascii_map[map_hit_y - 1][map_hit_x] == '#') &&
        //     (game_t.ascii_map[map_hit_y][map_hit_x + 1] == '#' ||
        //     game_t.ascii_map[map_hit_y][map_hit_x - 1] == '#')
        //     { game_t.canvas.set_draw_color(darker_color(game_t.environnement.walls_color, 2)); }
        // else
            { game_t.canvas.set_draw_color(game_t.environnement.walls_color); }
        if game_t.environnement.fog == true
            { game_t.canvas.set_draw_color(merge_colors(game_t.environnement.sky_color, game_t.environnement.walls_color, distance * FOG_INTENSITY as f64)); }
    } else {
        game_t.canvas.set_draw_color(darker_color(game_t.environnement.walls_color, (distance / 17.5) as i32));
    }
    game_t.canvas.fill_rect(wall_rect).unwrap();
}

pub fn draw_grid(game_t: &mut Game, player: &Player, hit_x: i32, hit_y: i32, ray_nb: i32, angle_temp: f64)
{
    let distance = f64::sqrt(f64::powi(hit_x as f64 - player.pos.x, 2) + f64::powi(hit_y as f64 - player.pos.y, 2));
    let vision_coefficient = f64::cos(angle_temp - player.angle);
    let wall_height = (50000.0 / (vision_coefficient * distance)) as i32;
    let wall_start = (SCREEN_HEIGHT / 2) - (wall_height / 2);
    let wall_end = ((SCREEN_HEIGHT / 2) + (wall_height / 2)) as u32;
    let ray_on_screen = ray_nb * (SCREEN_WIDTH / RAYS_NB) + 300;
    let wall_rect = sdl2::rect::Rect::new(ray_on_screen, wall_start, (SCREEN_WIDTH / RAYS_NB) as u32, wall_height as u32);

    game_t.canvas.set_draw_color(game_t.environnement.grid_color);
    game_t.canvas.fill_rect(wall_rect).unwrap();
}

