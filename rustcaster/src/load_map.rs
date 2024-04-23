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

use std::fs::File;
use std::io::Read;
use std::process;

fn get_map_str(filepath: String) -> String
{
    let mut file: File = match File::open(filepath.clone()) {
        Ok(f) => f,
        Err(_) => {
            println!("Error: {} File not found", filepath);
            process::exit(1);
        }
    };
    let mut contents = String::new();
    if let Err(_) = file.read_to_string(&mut contents) {
        println!("Error reading file: {}", filepath.clone());
        process::exit(1);
    }
    contents
}

fn fill_map(grid: &mut Vec<Vec<char>>, map: &str, _width: usize, _height: usize)
{
    let mut x = 0;
    let mut y = 0;

    for c in map.chars() {
        if c == '\n' {
            x = 0;
            y += 1;
        } else {
            grid[y][x] = c;
            x += 1;
        }
    }
}

fn check_map(map: &str)
{
    let mut player_count = 0;
    let mut exit_count = 0;
    let mut width = 0;

    for c in map.chars() {
        if c == 'P' {
            player_count += 1;
        } else if c == 'E' {
            exit_count += 1;
        }
    }
    if player_count != 1 {
        println!("Error: Map must contain one player");
        process::exit(1);
    } else if exit_count < 1 {
        println!("Error: Map must contain at least one exit");
        process::exit(1);
    }
    for line in map.lines() {
        if width == 0 {
            width = line.len();
        } else if line.len() != width {
            println!("Error: Map must have the same length for each line");
            process::exit(1);
        }
    }
}

pub fn load_map(filepath: String, player: &mut Player, game_t: &mut Game)
{
    let map = get_map_str(filepath);
    let mut x = 0;
    let mut y = 0;
    
    check_map(&map);
    game_t.map_width = map.find('\n').unwrap_or(0);
    game_t.map_height = map.lines().count();
    game_t.ascii_map = vec![vec![' '; game_t.map_width]; game_t.map_height];
    fill_map(&mut game_t.ascii_map, &map, game_t.map_width, game_t.map_height);
    for c in map.chars() {
        if c == 'P' {
            player.pos.x = x as f64 * (BLOCK_SIZE + BLOCK_SIZE / 2) as f64;
            player.pos.y = y as f64 * (BLOCK_SIZE + BLOCK_SIZE / 2) as f64;
            break;
        } else if c == '\n' {
            x = 0;
            y += 1;
        } else {
            game_t.ascii_map[y][x] = c;
            x += 1;
        }
    }
}
