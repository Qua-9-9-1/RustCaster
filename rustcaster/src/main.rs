// ooooooooo.                            .     .oooooo.                          .    o8o                         
// `888   `Y88.                        .o8    d8P'  `Y8b                       .o8    `"'                         
//  888   .d88' oooo  oooo   .oooo.o .o888oo 888           .oooo.    .oooo.o .o888oo oooo  ooo. .oo.    .oooooooo 
//  888ooo88P'  `888  `888  d88(  "8   888   888          `P  )88b  d88(  "8   888   `888  `888P"Y88b  888' `88b  
//  888`88b.     888   888  `"Y88b.    888   888           .oP"888  `"Y88b.    888    888   888   888  888   888  
//  888  `88b.   888   888  o.  )88b   888 . `88b    ooo  d8(  888  o.  )88b   888 .  888   888   888  `88bod8P'  
// o888o  o888o  `V88V"V8P' 8""888P'   "888"  `Y8bood8P'  `Y888""8o 8""888P'   "888" o888o o888o o888o `8oooooo.  
//                                                                                                     d"     YD  
//                                                                                                     "Y88888P'  

mod game_loop;
mod load_map;
mod minimap;
mod graphic_tools;
mod rays;
mod rustcaster;
mod walls;

use walls::draw_wall;
use walls::draw_grid;
use rays::draw_rays;
use graphic_tools::draw_filled_circle;
use graphic_tools::darker_color;
use graphic_tools::merge_colors;
use minimap::draw_minimap;
use minimap::is_in_map;
use game_loop::game_loop;
use load_map::load_map;
use rustcaster::SCREEN_WIDTH;
use rustcaster::SCREEN_HEIGHT;

use std::env;
use std::fs::File;
use std::io::Read;
use std::process;
use sdl2::pixels::Color;

pub struct Pos {
    x: f64,
    y: f64,
}

pub struct Player {
    pos: Pos,
    fov: i32,
    sensibility: f64,
    speed: f64,
    angle: f64,
}

pub struct Environnement  {
    // renderer: &'a sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    sky_color: Color,
    floor_color: Color,
    walls_color: Color,
    grid_color: Color,
    clouds_color: Color,
    sun_color: Color,
    fog: bool,
    outside: bool,
    //wall_texture: sdl2::image::LoadTexture,
    // ground_texture: Option<sdl2::render::Texture<'a>>,
    // ceiling_texture: Option<sdl2::render::Texture<'a>>,
    // grid_texture: Option<sdl2::render::Texture<'a>>,
    // coin_texture: Option<sdl2::render::Texture<'a>>,
    // exit_texture: Option<sdl2::render::Texture<'a>>,
}

pub struct Game  {
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: sdl2::EventPump,
    map_width: usize,
    map_height: usize,
    ascii_map: Vec<Vec<char>>,
    map_zone: sdl2::rect::Rect,
    screen: sdl2::rect::Rect,
    board: sdl2::rect::Rect,
    environnement: Environnement,
}

fn error_check(filepath: String, _len: usize) -> bool
{
    let mut contents = String::new();
    let mut file: File = match File::open(filepath.clone()) {
        Ok(f) => f,
        Err(_) => {
            println!("Error: {} File not found", filepath);
            return false;
        }
    };
    if let Err(_) = file.read_to_string(&mut contents) {
        println!("Error reading file: {}", filepath.clone());
        return false;
    }
    for c in contents.chars() {
        if c != '#' && c != 'P' && c != '\n' && c != ' ' && c != 'E' && c != 'X'
        && c != '0' && c != 'M' {
            println!("Error: {} Illegal character in file", filepath);
            return false;
        }
    }
    true
}

fn init_game() -> Game
{
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let struct_window = video_subsystem.window("RustCasting", 800, 600)
    .fullscreen_desktop()
    .position_centered()
    .build()
    .unwrap();
    let _canvas = struct_window.into_canvas().build().unwrap();
    let _renderer = _canvas.texture_creator();
    Game {
        canvas: _canvas,
        event_pump: sdl_context.event_pump().unwrap(),
        map_width: 0,
        map_height: 0,
        map_zone: sdl2::rect::Rect::new(20, 45, 260, 260),
        screen: sdl2::rect::Rect::new(300, 40, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32),
        board: sdl2::rect::Rect::new(10, 320, 280, 720),
        ascii_map: Vec::new(),
    }
}

fn main()
{
    let av: Vec<String> = env::args().collect();
    let ac: usize = av.len();
    let mut player = Player {
        pos: Pos { x: 0.0, y: 0.0 },
        fov: 80,
        sensibility: 4.0,
        speed: 3.0,
        angle: 0.0,
    };
    let mut game_t: Game = init_game();

    if ac < 2 {
        println!("Usage: ./rustcaster <map_file>");
        process::exit(1);
    }
    if error_check(av[1].clone(), ac) == false
        { process::exit(1); }
    load_map(av[1].clone(), &mut player, &mut game_t);
    // loop {
//        game_menu(&mut player, &mut game_t);
        load_ambiance("house", &mut game_t);
        game_loop(&mut player, &mut game_t);
    // }
    process::exit(0);
}
