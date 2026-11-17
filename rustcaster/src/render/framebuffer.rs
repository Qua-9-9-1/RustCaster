// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// Software framebuffer: everything (3D view, minimap, UI) is drawn into a
// `Vec<u32>` of ARGB pixels at an internal resolution, then uploaded once per
// frame to a streaming SDL texture and stretched to the window by `Presenter`.

use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{BlendMode, Canvas, Texture as SdlTexture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::assets::texture::Texture;
use crate::render::color::{blend_u32, Color};

/// Integer rectangle in framebuffer pixels (also used for UI layout).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    pub fn intersect(&self, o: &Rect) -> Rect {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = self.right().min(o.right());
        let y1 = self.bottom().min(o.bottom());
        Rect::new(x0, y0, (x1 - x0).max(0), (y1 - y0).max(0))
    }

    /// Shrinks the rect by `m` pixels on every side.
    pub fn inset(&self, m: i32) -> Rect {
        Rect::new(self.x + m, self.y + m, (self.w - 2 * m).max(0), (self.h - 2 * m).max(0))
    }
}

pub struct FrameBuffer {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    clip: Rect,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize) -> FrameBuffer {
        FrameBuffer {
            width,
            height,
            pixels: vec![0xFF00_0000; width * height],
            clip: Rect::new(0, 0, width as i32, height as i32),
        }
    }

    /// Reallocates (content is lost) and resets the clip rect.
    pub fn resize(&mut self, width: usize, height: usize) {
        if width == self.width && height == self.height {
            return;
        }
        *self = FrameBuffer::new(width, height);
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width as i32, self.height as i32)
    }

    /// Raw rows of ARGB pixels, `width` pixels per row. Fast path for renderers:
    /// index with `y * width + x`; does NOT honour the clip rect.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn pixels_mut(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    /// Restricts every drawing helper below to `r` (intersected with the buffer).
    pub fn set_clip(&mut self, r: Option<Rect>) {
        let full = self.bounds();
        self.clip = match r {
            Some(r) => r.intersect(&full),
            None => full,
        };
    }

    pub fn clip(&self) -> Rect {
        self.clip
    }

    pub fn clear(&mut self, color: Color) {
        let c = color.to_u32() | 0xFF00_0000;
        self.pixels.iter_mut().for_each(|p| *p = c);
    }

    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return None;
        }
        Some(self.pixels[y as usize * self.width + x as usize])
    }

    /// Writes a pixel (alpha-blended if `color.a < 255`), clipped.
    #[inline]
    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if !self.clip.contains(x, y) {
            return;
        }
        let i = y as usize * self.width + x as usize;
        self.pixels[i] = blend_u32(self.pixels[i], color.to_u32(), 255);
    }

    /// Fills a rectangle (alpha-blended if `color.a < 255`), clipped.
    pub fn fill_rect(&mut self, r: Rect, color: Color) {
        let r = r.intersect(&self.clip);
        if r.w <= 0 || r.h <= 0 {
            return;
        }
        let src = color.to_u32();
        for y in r.y..r.bottom() {
            let row = y as usize * self.width;
            let line = &mut self.pixels[row + r.x as usize..row + r.right() as usize];
            if color.a == 255 {
                line.iter_mut().for_each(|p| *p = src);
            } else {
                line.iter_mut().for_each(|p| *p = blend_u32(*p, src, 255));
            }
        }
    }

    /// 1px rectangle outline.
    pub fn draw_rect(&mut self, r: Rect, color: Color) {
        self.fill_rect(Rect::new(r.x, r.y, r.w, 1), color);
        self.fill_rect(Rect::new(r.x, r.bottom() - 1, r.w, 1), color);
        self.fill_rect(Rect::new(r.x, r.y + 1, 1, r.h - 2), color);
        self.fill_rect(Rect::new(r.right() - 1, r.y + 1, 1, r.h - 2), color);
    }

    /// Vertical span `[y0, y1)` in column `x`: the raycaster's basic primitive.
    pub fn draw_vline(&mut self, x: i32, y0: i32, y1: i32, color: Color) {
        self.fill_rect(Rect::new(x, y0, 1, y1 - y0), color);
    }

    /// Bresenham line, clipped per pixel.
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.put_pixel(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color) {
        for dy in -radius..=radius {
            let half = ((radius * radius - dy * dy) as f64).sqrt() as i32;
            self.fill_rect(Rect::new(cx - half, cy + dy, 2 * half + 1, 1), color);
        }
    }

    /// Draws `tex` scaled (nearest neighbour) into `dst`, honouring the texture
    /// alpha and the clip rect. Handy for UI previews and HUD icons.
    pub fn blit_texture(&mut self, tex: &Texture, dst: Rect) {
        if dst.w <= 0 || dst.h <= 0 || tex.width == 0 || tex.height == 0 {
            return;
        }
        let r = dst.intersect(&self.clip);
        for y in r.y..r.bottom() {
            let ty = ((y - dst.y) as usize * tex.height) / dst.h as usize;
            let row = y as usize * self.width;
            for x in r.x..r.right() {
                let tx = ((x - dst.x) as usize * tex.width) / dst.w as usize;
                let src = tex.pixels[ty * tex.width + tx];
                let i = row + x as usize;
                self.pixels[i] = blend_u32(self.pixels[i], src, 255);
            }
        }
    }
}

/// Owns the streaming SDL texture the framebuffer is uploaded into.
/// Created once in `main` (it borrows the `TextureCreator`), recreated
/// automatically when the framebuffer size changes.
pub struct Presenter<'a> {
    creator: &'a TextureCreator<WindowContext>,
    texture: SdlTexture<'a>,
    width: usize,
    height: usize,
}

impl<'a> Presenter<'a> {
    pub fn new(creator: &'a TextureCreator<WindowContext>, width: usize, height: usize) -> Result<Presenter<'a>, String> {
        let texture = Self::create(creator, width, height)?;
        Ok(Presenter { creator, texture, width, height })
    }

    fn create(creator: &'a TextureCreator<WindowContext>, width: usize, height: usize) -> Result<SdlTexture<'a>, String> {
        let mut t = creator
            .create_texture_streaming(PixelFormatEnum::ARGB8888, width as u32, height as u32)
            .map_err(|e| e.to_string())?;
        // framebuffer pixels are opaque: never let SDL blend them with the clear colour
        t.set_blend_mode(BlendMode::None);
        Ok(t)
    }

    /// Uploads the framebuffer and stretches it over the whole canvas.
    /// The caller still has to call `canvas.present()`.
    pub fn present(&mut self, canvas: &mut Canvas<Window>, fb: &FrameBuffer) -> Result<(), String> {
        if fb.width() != self.width || fb.height() != self.height {
            self.texture = Self::create(self.creator, fb.width(), fb.height())?;
            self.width = fb.width();
            self.height = fb.height();
        }
        let pixels = fb.pixels();
        // SAFETY: a &[u32] is always valid to view as 4x as many bytes; ARGB8888
        // is a packed native-endian u32 format, which is how the pixels are stored.
        let bytes = unsafe { std::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4) };
        self.texture.update(None, bytes, fb.width() * 4).map_err(|e| e.to_string())?;
        canvas.copy(&self.texture, None, None)
    }
}
