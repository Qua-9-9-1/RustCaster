// Part of this public API is only used by the follow-up tasks (see ARCHITECTURE.md).
#![allow(dead_code)]

// CPU-side image used by the software renderer: decoded once with the `image`
// crate into packed ARGB `u32` pixels (same format as the framebuffer).

use std::fmt;
use std::path::Path;

use crate::render::color::Color;

#[derive(Clone)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    /// Row-major ARGB pixels (0xAARRGGBB), `width * height` entries.
    pub pixels: Vec<u32>,
}

impl fmt::Debug for Texture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Texture({}x{})", self.width, self.height)
    }
}

impl Texture {
    /// Loads any format the `image` crate decodes (PNG at least).
    pub fn load(path: &Path) -> Result<Texture, String> {
        let img = image::open(path).map_err(|e| format!("{}: {}", path.display(), e))?.to_rgba8();
        let (w, h) = img.dimensions();
        let pixels = img
            .pixels()
            .map(|p| Color::rgba(p[0], p[1], p[2], p[3]).to_u32())
            .collect();
        Ok(Texture { width: w as usize, height: h as usize, pixels })
    }

    /// Plain-colour texture (placeholder, tests, editor defaults).
    pub fn solid(width: usize, height: usize, color: Color) -> Texture {
        Texture { width, height, pixels: vec![color.to_u32(); width * height] }
    }

    /// Writes the texture as a PNG (used when saving custom presets).
    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        let mut buf = image::RgbaImage::new(self.width as u32, self.height as u32);
        for (i, p) in buf.pixels_mut().enumerate() {
            let c = Color::from_u32(self.pixels[i]);
            *p = image::Rgba([c.r, c.g, c.b, c.a]);
        }
        buf.save(path).map_err(|e| format!("{}: {}", path.display(), e))
    }

    /// Texel at integer coordinates, wrapped (repeating texture).
    #[inline]
    pub fn get(&self, x: i64, y: i64) -> u32 {
        let x = x.rem_euclid(self.width as i64) as usize;
        let y = y.rem_euclid(self.height as i64) as usize;
        self.pixels[y * self.width + x]
    }

    /// Nearest-neighbour sample with normalised, wrapping coordinates:
    /// `u` left->right and `v` top->bottom, `(0,0)` is the top-left texel and
    /// 1.0 wraps back to 0. Returns packed ARGB (alpha < 255 = transparent).
    #[inline]
    pub fn sample(&self, u: f64, v: f64) -> u32 {
        self.get((u * self.width as f64).floor() as i64, (v * self.height as f64).floor() as i64)
    }

    /// Average colour, handy as a flat-colour fallback / minimap colour.
    pub fn average_color(&self) -> Color {
        let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
        for &p in &self.pixels {
            let c = Color::from_u32(p);
            if c.a > 0 {
                r += c.r as u64;
                g += c.g as u64;
                b += c.b as u64;
                n += 1;
            }
        }
        if n == 0 {
            return Color::BLACK;
        }
        Color::rgb((r / n) as u8, (g / n) as u8, (b / n) as u8)
    }
}
