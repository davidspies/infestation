//! Colors: per-region board tints and the shared UI palette.

use macroquad::color::Color;

use crate::world_map::Theme;

pub(crate) const fn rgb(hex: u32) -> Color {
    Color::new(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    )
}

/// Dark outline used for cartoon edges, matching the sprites.
pub(crate) const INK: Color = rgb(0x1d1726);
pub(crate) const TEXT: Color = rgb(0xf1eadb);
pub(crate) const TEXT_DIM: Color = rgb(0xa79fb5);
pub(crate) const GOLD: Color = rgb(0xf2c14e);
pub(crate) const DANGER: Color = rgb(0xe8505b);
pub(crate) const SUCCESS: Color = rgb(0x7bd389);
pub(crate) const PANEL: Color = Color::new(0.094, 0.082, 0.125, 0.94);
pub(crate) const PANEL_LIGHT: Color = rgb(0x2b2538);
pub(crate) const PANEL_EDGE: Color = rgb(0x4a4058);
pub(crate) const PLAYER1: Color = rgb(0x5b8cff);
pub(crate) const PLAYER2: Color = rgb(0xffa94d);

/// Trigger colors by digit, so matching triggers are easy to spot.
pub(crate) fn trigger(digit: u8) -> Color {
    rgb(match digit {
        1 => 0x4fd1ff,
        2 => 0xff6bd6,
        3 => 0xffd23f,
        4 => 0x7cf27c,
        5 => 0xff8f40,
        6 => 0xb48cff,
        7 => 0xff5d5d,
        8 => 0x5dffd8,
        _ => 0xf4f4f4,
    })
}

/// How a region's levels are tinted.
pub(crate) struct Palette {
    /// Multiplies the neutral floor texture.
    pub(crate) floor: Color,
    /// Multiplies the neutral wall textures.
    pub(crate) wall: Color,
    /// Behind the board: gradient from the center to the screen edges.
    pub(crate) backdrop: (Color, Color),
    /// Floating dust motes.
    pub(crate) motes: Color,
}

pub(crate) fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Cellar => Palette {
            floor: Color::new(0.80, 0.85, 1.0, 1.0),
            wall: Color::new(0.48, 0.50, 0.59, 1.0),
            backdrop: (rgb(0x1e2234), rgb(0x08090f)),
            motes: rgb(0xb8c8ff),
        },
        Theme::Powder => Palette {
            floor: Color::new(1.0, 0.86, 0.68, 1.0),
            wall: Color::new(0.60, 0.50, 0.42, 1.0),
            backdrop: (rgb(0x2e1f14), rgb(0x0e0906)),
            motes: rgb(0xffc27a),
        },
        Theme::Hall => Palette {
            floor: Color::new(1.0, 0.97, 0.88, 1.0),
            wall: Color::new(0.59, 0.56, 0.50, 1.0),
            backdrop: (rgb(0x2a2219), rgb(0x0d0b08)),
            motes: rgb(0xffe6a8),
        },
        Theme::Lab => Palette {
            floor: Color::new(0.72, 0.95, 0.90, 1.0),
            wall: Color::new(0.44, 0.54, 0.55, 1.0),
            backdrop: (rgb(0x0f2622), rgb(0x040b0a)),
            motes: rgb(0x8dffe6),
        },
        Theme::Towers => Palette {
            floor: Color::new(0.86, 1.0, 0.78, 1.0),
            wall: Color::new(0.53, 0.58, 0.50, 1.0),
            backdrop: (rgb(0x1a2a1c), rgb(0x070b07)),
            motes: rgb(0xd8ffa8),
        },
        Theme::Archive => Palette {
            floor: Color::new(1.0, 0.88, 0.82, 1.0),
            wall: Color::new(0.56, 0.50, 0.52, 1.0),
            backdrop: (rgb(0x281c26), rgb(0x0b080b)),
            motes: rgb(0xffd0e8),
        },
    }
}
