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

fn load_ambiance(ambiance: String, game_t: &mut Game)
{
    game_t.environnement: Environnement {
        //renderer: &_renderer,
        sky_color: Color::RGB(38, 98, 199),
        floor_color: Color::RGB(0, 130, 71),
        walls_color: Color::RGB(255, 255, 255),
        grid_color: Color::RGB(255, 255, 255),
        clouds_color: Color::RGB(255, 255, 255),
        sun_color: Color::RGB(255, 255, 255),
        outside: true,
        fog: false,
        //wall_texture: sdl2::image::LoadTexture::from_file("assets/wall_texture.png").unwrap(),
    }

}