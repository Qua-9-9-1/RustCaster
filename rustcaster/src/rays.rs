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
use crate::rustcaster::BLOCK_SIZE;
use crate::rustcaster::PLAYER_SIZE;
use crate::rustcaster::RAYS_NB;
use crate::rustcaster::RAYS_LENGTH;
use crate::draw_wall;
use crate::draw_grid;
use crate::is_in_map;

use sdl2::rect::Point;
use std::f64;

fn draw_single_ray(_game_t: &mut Game, _player: &Player, angle_temp: f64, ray_nb: i32) {

    let mut d_detect: i32 = 0;
    let mut hit_x: i32 = 0;
    let mut hit_y: i32 = 0;
    let center_x: i32 = _game_t.map_zone.center().x;
    let center_y: i32 = _game_t.map_zone.center().y;
    let mut distance: f64;

    while d_detect < RAYS_LENGTH {
        d_detect += 1;
        hit_x = (_player.pos.x + angle_temp.cos() * d_detect as f64) as i32;
        hit_y = (_player.pos.y + angle_temp.sin() * d_detect as f64) as i32;
        if is_in_map(_game_t, hit_x / BLOCK_SIZE, hit_y / BLOCK_SIZE, 2) {
            if _game_t.ascii_map[(hit_y / BLOCK_SIZE) as usize][(hit_x / BLOCK_SIZE) as usize] == '#' {
                draw_wall(_game_t, _player, hit_x, hit_y, ray_nb, angle_temp);
                break;
            }
            if _game_t.ascii_map[(hit_y / BLOCK_SIZE) as usize][(hit_x / BLOCK_SIZE) as usize] == 'X' {
                draw_grid(_game_t, _player, hit_x, hit_y, ray_nb, angle_temp);
            }
        }
    }
    // _game_t.canvas.set_draw_color(Color::RGB(255, 255, 0));
    // _game_t.canvas.draw_line(Point::new(center_x, center_y), Point::new(hit_x - _player.pos.x as i32 + center_x, hit_y - _player.pos.y as i32 + center_y)).unwrap();
}

pub fn draw_rays(game_t: &mut Game, player: &Player)
{
    let rays_nb: i32 = RAYS_NB;
    let mut ray_angle: f64 = player.angle - (player.fov as f64 * (std::f64::consts::PI / 180.0) / 2.0) as f64;
    let game_t_copy = game_t;
    let player_copy = player;
    
    for ray in 0..rays_nb {
        draw_single_ray(game_t_copy, player_copy, ray_angle, ray);
        ray_angle += player.fov as f64 * (std::f64::consts::PI / 180.0) / rays_nb as f64;
    }
}