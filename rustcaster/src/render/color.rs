// RGBA colour used everywhere in the software renderer and in presets.
// Framebuffer / Texture pixels are packed `u32` in ARGB order (0xAARRGGBB).

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

/// Serialized as "#RRGGBB" (or "#RRGGBBAA" when not opaque) in preset files.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// Packs as 0xAARRGGBB (framebuffer / texture pixel format).
    #[inline]
    pub const fn to_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    #[inline]
    pub const fn from_u32(p: u32) -> Color {
        Color { a: (p >> 24) as u8, r: (p >> 16) as u8, g: (p >> 8) as u8, b: p as u8 }
    }

    pub fn with_alpha(self, a: u8) -> Color {
        Color { a, ..self }
    }

    /// Multiplies RGB by `factor` (0..1 darkens), alpha untouched.
    pub fn scale(self, factor: f64) -> Color {
        let f = |c: u8| (c as f64 * factor).clamp(0.0, 255.0) as u8;
        Color { r: f(self.r), g: f(self.g), b: f(self.b), a: self.a }
    }

    /// Linear mix: `t = 0` gives `self`, `t = 1` gives `other` (t is clamped).
    pub fn lerp(self, other: Color, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let m = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t) as u8;
        Color { r: m(self.r, other.r), g: m(self.g, other.g), b: m(self.b, other.b), a: m(self.a, other.a) }
    }
}

/// Alpha-blends packed ARGB `src` over opaque `dst`, using src alpha * `alpha`/255.
#[inline]
pub fn blend_u32(dst: u32, src: u32, alpha: u8) -> u32 {
    let a = ((src >> 24) & 0xFF) * alpha as u32 / 255;
    if a == 0 {
        return dst;
    }
    if a == 255 {
        return src | 0xFF00_0000;
    }
    let inv = 255 - a;
    let ch = |shift: u32| ((((src >> shift) & 0xFF) * a + ((dst >> shift) & 0xFF) * inv) / 255) << shift;
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            write!(f, "#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }
}

impl From<Color> for String {
    fn from(c: Color) -> String {
        c.to_string()
    }
}

impl TryFrom<String> for Color {
    type Error = String;

    fn try_from(s: String) -> Result<Color, String> {
        let hex = s.trim().trim_start_matches('#');
        if !hex.is_ascii() {
            return Err(format!("bad colour '{}'", s));
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| format!("bad colour '{}'", s));
        match hex.len() {
            6 => Ok(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Ok(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => Err(format!("bad colour '{}', expected #RRGGBB or #RRGGBBAA", s)),
        }
    }
}
