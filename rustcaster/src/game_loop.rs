// ooooooooo.                            .     .oooooo.                          .    o8o                         
// `888   `Y88.                        .o8    d8P'  `Y8b                       .o8    `"'                         
//  888   .d88' oooo  oooo   .oooo.o .o888oo 888           .oooo.    .oooo.o .o888oo oooo  ooo. .oo.    .oooooooo 
//  888ooo88P'  `888  `888  d88(  "8   888   888          `P  )88b  d88(  "8   888   `888  `888P"Y88b  888' `88b  
//  888`88b.     888   888  `"Y88b.    888   888           .oP"888  `"Y88b.    888    888   888   888  888   888  
//  888  `88b.   888   888  o.  )88b   888 . `88b    ooo  d8(  888  o.  )88b   888 .  888   888   888  `88bod8P'  
// o888o  o888o  `V88V"V8P' 8""888P'   "888"  `Y8bood8P'  `Y888""8o 8""888P'   "888" o888o o888o o888o `8oooooo.  
//                                                                                                     d"     YD  
//                                                                                                     "Y88888P'  

extern crate sdl2;

use crate::draw_minimap;
use crate::Player;
use crate::Game;
use crate::rustcaster::BLOCK_SIZE;
use crate::rustcaster::PLAYER_SIZE;
use crate::rustcaster::SCREEN_HEIGHT;
use crate::rustcaster::SCREEN_WIDTH;
use crate::graphic_tools::draw_filled_circle;
use crate::is_x_in_map;
use crate::is_y_in_map;

use std::time::Duration;
use std::process;
use sdl2::pixels::Color;
use sdl2::keyboard::Scancode;

fn draw_sky(game_t: &mut Game, player: &Player) {
    game_t.canvas.set_draw_color(Color::RGB(255, 255, 150));
    draw_filled_circle(&mut game_t.canvas, ((player.angle * (180.0 / std::f64::consts::PI))) as i32 * 10, 150, 100);
}

fn draw_screen(game_t: &mut Game, player: &Player) {
    game_t.canvas.set_draw_color(Color::RGB(50, 50, 50));
    game_t.canvas.clear();
    game_t.canvas.set_draw_color(game_t.environnement.floor_color);
    game_t.canvas.fill_rect(game_t.map_zone).unwrap();
    game_t.canvas.set_draw_color(Color::RGB(100, 100, 100));
    game_t.canvas.fill_rect(game_t.board).unwrap();
    game_t.canvas.set_draw_color(game_t.environnement.sky_color);
    game_t.canvas.fill_rect(game_t.screen).unwrap();
    game_t.canvas.set_draw_color(game_t.environnement.floor_color);
    game_t.canvas.fill_rect(sdl2::rect::Rect::new(300, 40 + SCREEN_HEIGHT / 2, SCREEN_WIDTH as u32, (SCREEN_HEIGHT / 2) as u32)).unwrap();
    if game_t.environnement.outside == true && game_t.environnement.fog == false
        { draw_sky(game_t, player); }
}

fn move_player(_player: &mut Player, _game_t: &Game, direction: i32) {
    let mut new_x = ((_player.pos.x + (_player.angle.cos() * _player.speed as f64 + _player.angle.cos() * PLAYER_SIZE as f64) * direction as f64) / BLOCK_SIZE as f64) - 0.5;
    let mut old_x = (_player.pos.x / BLOCK_SIZE as f64) - 0.5;
    let mut new_y = ((_player.pos.y + (_player.angle.sin() * _player.speed as f64 + _player.angle.sin() * PLAYER_SIZE as f64) * direction as f64) / BLOCK_SIZE as f64) - 0.5;
    let mut old_y = (_player.pos.y / BLOCK_SIZE as f64) - 0.5;

    new_x = new_x.round();
    new_y = new_y.round();
    old_x = old_x.round();
    old_y = old_y.round();
    if is_x_in_map(_game_t, new_x as i32) &&
    (_game_t.ascii_map[old_y as usize][new_x as usize] != '#' && _game_t.ascii_map[old_y as usize][new_x as usize] != 'X')
        {_player.pos.x += _player.angle.cos() * _player.speed as f64 * direction as f64;}
    if is_y_in_map(_game_t, new_y as i32) &&
    (_game_t.ascii_map[new_y as usize][old_x as usize] != '#' && _game_t.ascii_map[new_y as usize][old_x as usize] != 'X')
        {_player.pos.y += _player.angle.sin() * _player.speed as f64 * direction as f64;}
}

fn controls(player: &mut Player, game_t: &mut Game) {
    game_t.event_pump.pump_events();
    let keyboard_state = game_t.event_pump.keyboard_state();

    if keyboard_state.is_scancode_pressed(Scancode::Escape)
        {process::exit(0);}
    if keyboard_state.is_scancode_pressed(Scancode::A) {
        player.angle -= player.sensibility as f64 * (std::f64::consts::PI / 180.0);
        if player.angle < 0.0
            { player.angle += 360.0 * (std::f64::consts::PI / 180.0); }
    }
    if keyboard_state.is_scancode_pressed(Scancode::W)
        {move_player(player, game_t, 1);}
    if keyboard_state.is_scancode_pressed(Scancode::S)
        {move_player(player, game_t, -1);}
    if keyboard_state.is_scancode_pressed(Scancode::D) {
        player.angle += player.sensibility as f64 * (std::f64::consts::PI / 180.0);
        if player.angle >= 360.0 * (std::f64::consts::PI / 180.0)
            { player.angle -= 360.0 * (std::f64::consts::PI / 180.0); }
    }
    if keyboard_state.is_scancode_pressed(Scancode::U) && player.fov < 360 - 10
        {player.fov += 10;}
    if keyboard_state.is_scancode_pressed(Scancode::J) && player.fov > 10
        {player.fov -= 10;}
    if keyboard_state.is_scancode_pressed(Scancode::I) {
        if player.sensibility < 10.0 - 0.1 {
            player.sensibility += 0.1;
        }
    }
    if keyboard_state.is_scancode_pressed(Scancode::K) {
        if player.sensibility > 0.1 {
            player.sensibility -= 0.1;
        }
    }
}

pub fn game_loop(player: &mut Player, game_t: &mut Game) {
    loop {
        draw_screen(game_t, player);
        draw_minimap(game_t, &player);
        controls(player, game_t);
        if game_t.ascii_map[(player.pos.y as i32 / BLOCK_SIZE) as usize][(player.pos.x as i32 / BLOCK_SIZE) as usize] == '0' {
            player.coins += 1;
            game_t.ascii_map[(player.pos.y as i32 / BLOCK_SIZE) as usize][(player.pos.x as i32 / BLOCK_SIZE) as usize] = ' '
        }
        if game_t.ascii_map[(player.pos.y as i32 / BLOCK_SIZE) as usize][(player.pos.x as i32 / BLOCK_SIZE) as usize] == 'E'
            { break; }
        game_t.canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}