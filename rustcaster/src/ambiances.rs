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

use crate::Game;
use crate::Color;

pub fn load_ambiance(game_t: &mut Game)
{
    let ambiance = &game_t.environnement.name as &str;
    match ambiance {
        "house" => {
            game_t.environnement.sky_color = Color::RGB(38, 98, 199);
            game_t.environnement.clouds_color = Color::RGB(255, 255, 255);
            game_t.environnement.sun_color = Color::RGB(255, 255, 255);
            game_t.environnement.floor_color = Color::RGB(0, 0, 0);
            game_t.environnement.walls_color = Color::RGB(100, 100, 100)
        },
        "garden" => {
            game_t.environnement.sky_color = Color::RGB(135, 206, 235);
            game_t.environnement.clouds_color = Color::RGB(255, 255, 255);
            game_t.environnement.sun_color = Color::RGB(255, 255, 255);
        },
        "halloween" => {
            game_t.environnement.sky_color = Color::RGB(220, 170, 20);
            game_t.environnement.clouds_color = Color::RGB(150, 140, 40);
            game_t.environnement.sun_color = Color::RGB(255, 255, 255);
        },
        "night" => {
            game_t.environnement.sky_color = Color::RGB(12, 20, 69);
            game_t.environnement.clouds_color = Color::RGB(30, 30, 70);
            game_t.environnement.sun_color = Color::RGB(220, 220, 50);
        },
        "nether" => game_t.environnement.sky_color = Color::RGB(172, 32, 32),
        "retro" => game_t.environnement.sky_color = Color::RGB(0, 0, 0),
        "snow" => game_t.environnement.sky_color = Color::RGB(240, 240, 240),
        _ => (),

    }
    if ambiance == "garden" || ambiance == "halloween" || ambiance == "nether"
    || ambiance == "night" || ambiance == "snow" || ambiance == "house" {
        game_t.environnement.outside = true;
    } else {
        game_t.environnement.outside = false;
        //load ceiling
    }
    if ambiance == "nether" || ambiance == "snow" || ambiance == "night" {
        game_t.environnement.fog = true;
    } else {
        game_t.environnement.fog = false;
    }
    let stre = "/assets/".to_owned() + ambiance + "/coin.png";
    //wall_texture: sdl2::image::LoadTexture::from_file("assets/wall_texture.png").unwrap(),
}