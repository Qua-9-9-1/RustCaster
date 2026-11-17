// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// Per-frame input snapshot built from SDL events (`begin_frame` once per
// frame, before updating the app). "pressed"/"clicked" are edges: true only
// during the frame the key/button went down. Mouse coordinates are in
// framebuffer pixels (same space as UI drawing), not window pixels.

use std::collections::HashSet;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use sdl2::EventPump;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mouse {
    Left,
    Middle,
    Right,
}

impl Mouse {
    fn index(self) -> usize {
        match self {
            Mouse::Left => 0,
            Mouse::Middle => 1,
            Mouse::Right => 2,
        }
    }

    fn from_sdl(b: MouseButton) -> Option<Mouse> {
        match b {
            MouseButton::Left => Some(Mouse::Left),
            MouseButton::Middle => Some(Mouse::Middle),
            MouseButton::Right => Some(Mouse::Right),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Input {
    held: HashSet<Scancode>,
    pressed: HashSet<Scancode>,
    released: HashSet<Scancode>,
    mouse_x: i32,
    mouse_y: i32,
    mouse_held: [bool; 3],
    mouse_pressed: [bool; 3],
    mouse_released: [bool; 3],
    wheel: i32,
    text: String,
    quit: bool,
}

impl Input {
    pub fn new() -> Input {
        Input::default()
    }

    /// Clears the per-frame edges and drains every pending SDL event.
    /// `window_size` / `fb_size` convert mouse positions to framebuffer pixels.
    pub fn begin_frame(&mut self, pump: &mut EventPump, window_size: (u32, u32), fb_size: (usize, usize)) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_pressed = [false; 3];
        self.mouse_released = [false; 3];
        self.wheel = 0;
        self.text.clear();

        let sx = fb_size.0 as f64 / window_size.0.max(1) as f64;
        let sy = fb_size.1 as f64 / window_size.1.max(1) as f64;
        let to_fb = |x: i32, y: i32| ((x as f64 * sx) as i32, (y as f64 * sy) as i32);

        for event in pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.quit = true,
                Event::KeyDown { scancode: Some(sc), repeat, .. } => {
                    if !repeat && self.held.insert(sc) {
                        self.pressed.insert(sc);
                    }
                }
                Event::KeyUp { scancode: Some(sc), .. } => {
                    self.held.remove(&sc);
                    self.released.insert(sc);
                }
                Event::MouseMotion { x, y, .. } => {
                    let (fx, fy) = to_fb(x, y);
                    self.mouse_x = fx;
                    self.mouse_y = fy;
                }
                Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                    let (fx, fy) = to_fb(x, y);
                    self.mouse_x = fx;
                    self.mouse_y = fy;
                    if let Some(b) = Mouse::from_sdl(mouse_btn) {
                        self.mouse_held[b.index()] = true;
                        self.mouse_pressed[b.index()] = true;
                    }
                }
                Event::MouseButtonUp { mouse_btn, x, y, .. } => {
                    let (fx, fy) = to_fb(x, y);
                    self.mouse_x = fx;
                    self.mouse_y = fy;
                    if let Some(b) = Mouse::from_sdl(mouse_btn) {
                        self.mouse_held[b.index()] = false;
                        self.mouse_released[b.index()] = true;
                    }
                }
                Event::MouseWheel { y, .. } => self.wheel += y,
                Event::TextInput { text, .. } => self.text.push_str(&text),
                Event::Window { win_event: WindowEvent::FocusLost, .. } => {
                    // avoid keys stuck "down" after alt-tab
                    self.held.clear();
                    self.mouse_held = [false; 3];
                }
                _ => {}
            }
        }
    }

    /// Window close / Alt+F4 requested.
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// Key currently held down.
    pub fn is_down(&self, sc: Scancode) -> bool {
        self.held.contains(&sc)
    }

    /// Key went down this frame (no auto-repeat).
    pub fn was_pressed(&self, sc: Scancode) -> bool {
        self.pressed.contains(&sc)
    }

    pub fn was_released(&self, sc: Scancode) -> bool {
        self.released.contains(&sc)
    }

    /// Mouse position in framebuffer pixels.
    pub fn mouse_pos(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    pub fn mouse_down(&self, b: Mouse) -> bool {
        self.mouse_held[b.index()]
    }

    /// Button went down this frame.
    pub fn mouse_clicked(&self, b: Mouse) -> bool {
        self.mouse_pressed[b.index()]
    }

    pub fn mouse_released(&self, b: Mouse) -> bool {
        self.mouse_released[b.index()]
    }

    /// Wheel notches this frame (positive = away from the user / up).
    pub fn wheel(&self) -> i32 {
        self.wheel
    }

    /// Text typed this frame (SDL text input, already layout/IME aware).
    pub fn text(&self) -> &str {
        &self.text
    }
}
