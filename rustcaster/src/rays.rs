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
use crate::rustcaster::RAYS_NB;
use crate::rustcaster::RAYS_LENGTH;
use crate::draw_wall;
use crate::draw_coin;
use crate::draw_end;
use crate::draw_grid;
use crate::is_in_map;

use sdl2::rect::Point;

// Ray casting by DDA (Digital Differential Analyzer):
// instead of advancing the ray pixel by pixel (up to RAYS_LENGTH steps with a
// cos/sin per step), we jump directly from one tile border to the next.
// The cost of a ray is now the number of tiles crossed, not the number of pixels.
fn draw_single_ray(game_t: &mut Game, player: &Player, angle_temp: f64, ray_nb: i32) {
    let center_x: i32 = game_t.map_zone.center().x;
    let center_y: i32 = game_t.map_zone.center().y;
    let block = BLOCK_SIZE as f64;
    let max_dist = RAYS_LENGTH as f64;
    let (dir_y, dir_x) = angle_temp.sin_cos();

    // current tile
    let mut map_x = (player.pos.x / block).floor() as i32;
    let mut map_y = (player.pos.y / block).floor() as i32;

    // direction of the steps on each axis
    let step_x: i32 = if dir_x < 0.0 { -1 } else { 1 };
    let step_y: i32 = if dir_y < 0.0 { -1 } else { 1 };

    // distance along the ray to cross one full tile on each axis
    let delta_x = if dir_x == 0.0 { f64::INFINITY } else { (block / dir_x).abs() };
    let delta_y = if dir_y == 0.0 { f64::INFINITY } else { (block / dir_y).abs() };

    // distance along the ray to the first vertical / horizontal tile border
    let mut side_x = if dir_x == 0.0 {
        f64::INFINITY
    } else if dir_x < 0.0 {
        (player.pos.x - map_x as f64 * block) / -dir_x
    } else {
        ((map_x + 1) as f64 * block - player.pos.x) / dir_x
    };
    let mut side_y = if dir_y == 0.0 {
        f64::INFINITY
    } else if dir_y < 0.0 {
        (player.pos.y - map_y as f64 * block) / -dir_y
    } else {
        ((map_y + 1) as f64 * block - player.pos.y) / dir_y
    };

    let mut distance: f64;
    loop {
        // move to the next tile
        if side_x < side_y {
            distance = side_x;
            side_x += delta_x;
            map_x += step_x;
        } else {
            distance = side_y;
            side_y += delta_y;
            map_y += step_y;
        }
        if distance > max_dist {
            distance = max_dist;
            break;
        }
        if !is_in_map(game_t, map_x, map_y) {
            draw_wall(game_t, player, distance, ray_nb, angle_temp);
            break;
        }
        match game_t.ascii_map[map_y as usize][map_x as usize] {
            '#' => {
                draw_wall(game_t, player, distance, ray_nb, angle_temp);
                break;
            },
            'X' => draw_grid(game_t, player, distance, ray_nb, angle_temp),
            '0' => {
                draw_coin(game_t, player, distance, ray_nb, angle_temp);
                break;
            },
            'E' => {
                draw_end(game_t, player, distance, ray_nb, angle_temp);
                break;
            },
            _ => (),
        }
    }
    let hit_x = (player.pos.x + dir_x * distance) as i32;
    let hit_y = (player.pos.y + dir_y * distance) as i32;
    game_t.canvas.set_draw_color(Color::RGB(0, 255, 255));
    game_t.canvas.draw_line(Point::new(center_x, center_y), Point::new(hit_x - player.pos.x as i32 + center_x, hit_y - player.pos.y as i32 + center_y)).unwrap();
}

pub fn draw_rays(game_t: &mut Game, player: &Player) {
    let fov_rad = player.fov as f64 * (std::f64::consts::PI / 180.0);
    let ray_step = fov_rad / RAYS_NB as f64;
    let mut ray_angle: f64 = player.angle - fov_rad / 2.0;

    for ray in 0..RAYS_NB {
        draw_single_ray(game_t, player, ray_angle, ray);
        ray_angle += ray_step;
    }
}
