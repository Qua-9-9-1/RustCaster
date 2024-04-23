// ooooooooo.                            .     .oooooo.                          .    o8o                         
// `888   `Y88.                        .o8    d8P'  `Y8b                       .o8    `"'                         
//  888   .d88' oooo  oooo   .oooo.o .o888oo 888           .oooo.    .oooo.o .o888oo oooo  ooo. .oo.    .oooooooo 
//  888ooo88P'  `888  `888  d88(  "8   888   888          `P  )88b  d88(  "8   888   `888  `888P"Y88b  888' `88b  
//  888`88b.     888   888  `"Y88b.    888   888           .oP"888  `"Y88b.    888    888   888   888  888   888  
//  888  `88b.   888   888  o.  )88b   888 . `88b    ooo  d8(  888  o.  )88b   888 .  888   888   888  `88bod8P'  
// o888o  o888o  `V88V"V8P' 8""888P'   "888"  `Y8bood8P'  `Y888""8o 8""888P'   "888" o888o o888o o888o `8oooooo.  
//                                                                                                     d"     YD  
//                                                                                                     "Y88888P'  

use sdl2::rect::Point;
use sdl2::video::Window;
use sdl2::render::Canvas;

pub fn draw_filled_circle(canvas: &mut Canvas<Window>, center_x: i32, center_y: i32, radius: i32)
{
    let mut y = center_y - radius;

    while y <= center_y + radius {
        let x_max = ((radius as f64 * radius as f64 - (y - center_y) as f64 * (y - center_y) as f64).sqrt() + center_x as f64) as i32;
        let x_min = (2 * center_x - x_max) as i32;
        canvas.draw_line(Point::new(x_min, y), Point::new(x_max, y)).unwrap();
        y += 1;
    }
}

pub fn darker_color(color: sdl2::pixels::Color, coef: i32) -> sdl2::pixels::Color
{
    let mut r = color.r as f64;
    let mut g = color.g as f64;
    let mut b = color.b as f64;

    for _ in 0..coef {
        r *= 0.8;
        g *= 0.8;
        b *= 0.8;
    }
    sdl2::pixels::Color::RGB(r as u8, g as u8, b as u8)
}

pub fn merge_colors(color1: sdl2::pixels::Color, color2: sdl2::pixels::Color, coef: f64) -> sdl2::pixels::Color
{
    let mut r = color1.r as f64 * coef + color2.r as f64 * (1.0 - coef);
    let mut g = color1.g as f64 * coef + color2.g as f64 * (1.0 - coef);
    let mut b = color1.b as f64 * coef + color2.b as f64 * (1.0 - coef);
    
    if coef > 1.0
        { return color1; }
    sdl2::pixels::Color::RGB(r as u8, g as u8, b as u8)
}