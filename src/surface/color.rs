//! File: src/surface/color.rs

use std::collections::HashMap;

use super::style::{Style, StylePatch};

/// Basic ANSI colors used for foreground and background styling.
#[repr(u8)]
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Color {
    Black = 0,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    #[default]
    Default = 8,
}

impl Color {
    /// Reconstructs a packed inline color value.
    pub const fn from_u8(n: u8) -> Self {
        match n {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::White,
            _ => Color::Default,
        }
    }

    /// Converts to a numeric foreground color.
    pub const fn fg_code(self) -> u8 {
        match self {
            Color::Default => 39,
            _ => 30 + self as u8,
        }
    }

    /// Converts to a numeric background color.
    pub const fn bg_code(self) -> u8 {
        match self {
            Color::Default => 49,
            _ => 40 + self as u8,
        }
    }
}

/// Extended color representation resolved only by the renderer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpec {
    /// Reset back to the terminal default color.
    #[default]
    Default,

    /// Use one of the classic ANSI 16 colors.
    Ansi16(Color),

    /// Use one of the ANSI 256 indexed colors.
    Ansi256(u8),

    /// Use an explicit 24-bit RGB color.
    Rgb(u8, u8, u8),
}

impl From<Color> for ColorSpec {
    /// Converts a builtin color into a renderer-facing color spec.
    fn from(value: Color) -> Self {
        match value {
            Color::Default => ColorSpec::Default,
            other => ColorSpec::Ansi16(other),
        }
    }
}

/// Foreground/background combination interned by the color atlas.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorPair {
    /// Foreground color for the pair.
    pub fg: ColorSpec,

    /// Background color for the pair.
    pub bg: ColorSpec,
}

impl ColorPair {
    /// Creates a new foreground/background pair.
    pub const fn new(fg: ColorSpec, bg: ColorSpec) -> Self {
        Self { fg, bg }
    }
}

/// Interns foreground/background pairs for RGB and ANSI256 rendering.
#[derive(Debug, Default)]
pub struct ColorAtlas {
    /// Extended pairs stored out-of-line.
    pairs: Vec<ColorPair>,

    /// Reverse lookup for interning extended pairs.
    lookup: HashMap<ColorPair, u16>,
}

impl ColorAtlas {
    /// Creates a new empty color-pair atlas.
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a pair and returns its stable pair id.
    pub fn intern_pair(&mut self, pair: ColorPair) -> u16 {
        if let (
            ColorSpec::Default | ColorSpec::Ansi16(_),
            ColorSpec::Default | ColorSpec::Ansi16(_),
        ) = (pair.fg, pair.bg)
        {
            let fg = match pair.fg {
                ColorSpec::Default => Color::Default,
                ColorSpec::Ansi16(color) => color,
                _ => unreachable!(),
            };
            let bg = match pair.bg {
                ColorSpec::Default => Color::Default,
                ColorSpec::Ansi16(color) => color,
                _ => unreachable!(),
            };

            return Style::builtin_pair_id(fg, bg);
        }

        if let Some(&pair_id) = self.lookup.get(&pair) {
            return pair_id;
        }

        let extended_index = u16::try_from(self.pairs.len()).expect("color-pair atlas exhausted");
        let pair_id = Style::FIRST_EXTENDED_PAIR
            .checked_add(extended_index)
            .expect("color-pair atlas exhausted");

        self.pairs.push(pair);
        self.lookup.insert(pair, pair_id);
        pair_id
    }

    /// Applies a sparse patch to an already resolved style.
    #[inline]
    pub fn apply_patch(&mut self, base: Style, patch: StylePatch) -> Style {
        if patch.is_empty() {
            return base;
        }

        let pair = self.resolve_pair(base.pair_id());
        let fg = patch.fg.unwrap_or(pair.fg);
        let bg = patch.bg.unwrap_or(pair.bg);

        self.style(base.flags() | patch.add_flags, fg, bg)
    }

    /// Resolves a pair id into a foreground/background pair.
    pub fn resolve_pair(&self, pair_id: u16) -> ColorPair {
        if Style::is_inline_pair_id(pair_id) {
            return ColorPair {
                fg: ColorSpec::from(Style::inline_fg_from_pair_id(pair_id)),
                bg: ColorSpec::from(Style::inline_bg_from_pair_id(pair_id)),
            };
        }

        let index = (pair_id - Style::FIRST_EXTENDED_PAIR) as usize;
        self.pairs.get(index).copied().unwrap_or_default()
    }

    /// Creates a style from flags and a pair of renderer-facing colors.
    pub fn style(&mut self, flags: u32, fg: ColorSpec, bg: ColorSpec) -> Style {
        Style::new()
            .with_flags(flags)
            .with_pair(self.intern_pair(ColorPair::new(fg, bg)))
    }

    /// Rebuilds a style with a new foreground while preserving flags and bg.
    pub fn with_fg(&mut self, style: Style, fg: ColorSpec) -> Style {
        let mut pair = self.resolve_pair(style.pair_id());
        pair.fg = fg;
        Style::new()
            .with_flags(style.flags())
            .with_pair(self.intern_pair(pair))
    }

    /// Rebuilds a style with a new background while preserving flags and fg.
    pub fn with_bg(&mut self, style: Style, bg: ColorSpec) -> Style {
        let mut pair = self.resolve_pair(style.pair_id());
        pair.bg = bg;
        Style::new()
            .with_flags(style.flags())
            .with_pair(self.intern_pair(pair))
    }
}
