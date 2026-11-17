// ooooooooo.                            .     .oooooo.                          .    o8o
// `888   `Y88.                        .o8    d8P'  `Y8b                       .o8    `"'
//  888   .d88' oooo  oooo   .oooo.o .o888oo 888           .oooo.    .oooo.o .o888oo oooo  ooo. .oo.    .oooooooo
//  888ooo88P'  `888  `888  d88(  "8   888   888          `P  )88b  d88(  "8   888   `888  `888P"Y88b  888' `88b
//  888`88b.     888   888  `"Y88b.    888   888           .oP"888  `"Y88b.    888    888   888   888  888   888
//  888  `88b.   888   888  o.  )88b   888 . `88b    ooo  d8(  888  o.  )88b   888 .  888   888   888  `88bod8P'
// o888o  o888o  `V88V"V8P' 8""888P'   "888"  `Y8bood8P'  `Y888""8o 8""888P'   "888" o888o o888o o888o `8oooooo.
//                                                                                                     d"     YD
//                                                                                                     "Y88888P'
//
// Entry point: SDL init, App creation and the frame loop (60 FPS limiter).
// Usage: rustcaster [map_file]   (no map: start in the menu)

mod app;
mod assets;
mod config;
mod game;
mod input;
mod render;
mod screens;
mod settings;
mod ui;

use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use app::App;
use config::{MAX_FRAME_DT, RENDER_HEIGHT, TARGET_FPS, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH};
use input::Input;
use render::framebuffer::{FrameBuffer, Presenter};

/// Internal framebuffer size for a window size: fixed height, width following
/// the window aspect ratio (square pixels, no letterboxing).
fn render_size(window: (u32, u32)) -> (usize, usize) {
    let (ww, wh) = (window.0.max(1) as f64, window.1.max(1) as f64);
    let h = (RENDER_HEIGHT as f64).min(wh);
    let w = (h * ww / wh).round().max(1.0);
    (w as usize, h as usize)
}

fn run() -> Result<(), String> {
    let cli_map = std::env::args().nth(1).map(PathBuf::from);

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    // nearest-neighbour scaling of the framebuffer (crisp pixels)
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");
    let window = video
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .fullscreen_desktop()
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let mut event_pump = sdl.event_pump()?;

    let (fw, fh) = render_size(canvas.output_size()?);
    let mut fb = FrameBuffer::new(fw, fh);
    let mut presenter = Presenter::new(&texture_creator, fw, fh)?;
    let mut input = Input::new();
    let mut app = App::new(config::find_data_root(), cli_map);

    // target frame time: we only sleep for what is left of the frame,
    // instead of always sleeping 1/60s on top of the rendering time
    let frame_duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);
    let mut last_frame = Instant::now();
    loop {
        let frame_start = Instant::now();
        let dt = (frame_start - last_frame).as_secs_f64().min(MAX_FRAME_DT);
        last_frame = frame_start;

        let window_size = canvas.window().size();
        input.begin_frame(&mut event_pump, window_size, (fb.width(), fb.height()));
        if input.quit_requested() {
            break;
        }
        let (fw, fh) = render_size(canvas.output_size()?);
        fb.resize(fw, fh);

        app.update(&input, dt, (fb.width(), fb.height()));
        if app.should_quit() {
            break;
        }
        app.render(&mut fb, &input);
        presenter.present(&mut canvas, &fb)?;
        canvas.present();

        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
