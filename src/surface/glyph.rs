//! File: src/surface/glyph.rs

#![allow(dead_code)]

use super::style::Style;

/// Characters used to draw boxes around panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoxDraw {
    Horizontal,         // ─
    Vertical,           // │
    TopLeft,            // ┌
    TopRight,           // ┐
    BottomLeft,         // └
    BottomRight,        // ┘
    Cross,              // ┼
    RoundedTopLeft,     // ╭
    RoundedTopRight,    // ╮
    RoundedBottomLeft,  // ╰
    RoundedBottomRight, // ╯
    DoubleHorizontal,   // ═
    DoubleVertical,     // ║
    DoubleTopLeft,      // ╔
    DoubleTopRight,     // ╗
    DoubleBottomLeft,   // ╚
    DoubleBottomRight,  // ╝
    DoubleCross,        // ╬
}

impl From<BoxDraw> for char {
    /// Converts a box-drawing marker into a Unicode character.
    fn from(value: BoxDraw) -> Self {
        match value {
            BoxDraw::Horizontal => '─',
            BoxDraw::Vertical => '│',
            BoxDraw::TopLeft => '┌',
            BoxDraw::TopRight => '┐',
            BoxDraw::BottomLeft => '└',
            BoxDraw::BottomRight => '┘',
            BoxDraw::Cross => '┼',
            BoxDraw::RoundedTopLeft => '╭',
            BoxDraw::RoundedTopRight => '╮',
            BoxDraw::RoundedBottomLeft => '╰',
            BoxDraw::RoundedBottomRight => '╯',
            BoxDraw::DoubleHorizontal => '═',
            BoxDraw::DoubleVertical => '║',
            BoxDraw::DoubleTopLeft => '╔',
            BoxDraw::DoubleTopRight => '╗',
            BoxDraw::DoubleBottomLeft => '╚',
            BoxDraw::DoubleBottomRight => '╝',
            BoxDraw::DoubleCross => '╬',
        }
    }
}

/// Type of border to be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderKind {
    #[default]
    Single,
    Rounded,
    Double,
}

impl BorderKind {
    /// Returns the horizontal box-drawing glyph for this border kind.
    pub(crate) const fn horizontal(self) -> BoxDraw {
        match self {
            BorderKind::Single | BorderKind::Rounded => BoxDraw::Horizontal,
            BorderKind::Double => BoxDraw::DoubleHorizontal,
        }
    }

    /// Returns the vertical box-drawing glyph for this border kind.
    pub(crate) const fn vertical(self) -> BoxDraw {
        match self {
            BorderKind::Single | BorderKind::Rounded => BoxDraw::Vertical,
            BorderKind::Double => BoxDraw::DoubleVertical,
        }
    }

    /// Returns the top-left corner glyph for this border kind.
    pub(crate) const fn top_left(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::TopLeft,
            BorderKind::Rounded => BoxDraw::RoundedTopLeft,
            BorderKind::Double => BoxDraw::DoubleTopLeft,
        }
    }

    /// Returns the top-right corner glyph for this border kind.
    pub(crate) const fn top_right(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::TopRight,
            BorderKind::Rounded => BoxDraw::RoundedTopRight,
            BorderKind::Double => BoxDraw::DoubleTopRight,
        }
    }

    /// Returns the bottom-left corner glyph for this border kind.
    pub(crate) const fn bottom_left(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::BottomLeft,
            BorderKind::Rounded => BoxDraw::RoundedBottomLeft,
            BorderKind::Double => BoxDraw::DoubleBottomLeft,
        }
    }

    /// Returns the bottom-right corner glyph for this border kind.
    pub(crate) const fn bottom_right(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::BottomRight,
            BorderKind::Rounded => BoxDraw::RoundedBottomRight,
            BorderKind::Double => BoxDraw::DoubleBottomRight,
        }
    }

    /// Returns the crossing glyph for this border kind.
    pub(crate) const fn cross(self) -> BoxDraw {
        match self {
            BorderKind::Double => BoxDraw::DoubleCross,
            _ => BoxDraw::Cross,
        }
    }

    /// Returns all border glyphs in the order:
    /// horizontal, vertical, top-left, top-right, bottom-left, bottom-right.
    pub(crate) const fn glyphs(self) -> (BoxDraw, BoxDraw, BoxDraw, BoxDraw, BoxDraw, BoxDraw) {
        match self {
            BorderKind::Single => (
                BoxDraw::Horizontal,
                BoxDraw::Vertical,
                BoxDraw::TopLeft,
                BoxDraw::TopRight,
                BoxDraw::BottomLeft,
                BoxDraw::BottomRight,
            ),
            BorderKind::Rounded => (
                BoxDraw::Horizontal,
                BoxDraw::Vertical,
                BoxDraw::RoundedTopLeft,
                BoxDraw::RoundedTopRight,
                BoxDraw::RoundedBottomLeft,
                BoxDraw::RoundedBottomRight,
            ),
            BorderKind::Double => (
                BoxDraw::DoubleHorizontal,
                BoxDraw::DoubleVertical,
                BoxDraw::DoubleTopLeft,
                BoxDraw::DoubleTopRight,
                BoxDraw::DoubleBottomLeft,
                BoxDraw::DoubleBottomRight,
            ),
        }
    }
}

/// Represents a printable "character" to the screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rune {
    pub(crate) bytes: [u8; 4],
    pub(crate) len: u8,
}

impl From<char> for Rune {
    /// Encodes a Unicode scalar value into the rune buffer.
    fn from(value: char) -> Self {
        let mut bytes = [0u8; 4];
        let len = value.encode_utf8(&mut bytes).len() as u8;
        Self { bytes, len }
    }
}

impl Default for Rune {
    /// Creates a default blank rune.
    fn default() -> Self {
        Self {
            bytes: [b' '; 4],
            len: 1,
        }
    }
}

impl From<BoxDraw> for Rune {
    /// Converts a box-drawing marker into a rune.
    fn from(value: BoxDraw) -> Self {
        char::from(value).into()
    }
}

/// A single screen cell that can be rendered with a rune and style.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// Attributes that are applied such as bold, blink, underline, and color.
    pub style: Style,

    /// Rendered data.
    pub rune: Rune,
}

impl Glyph {
    /// Constructs a default glyph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a style to the rendered glyph.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Replaces the glyph's rune.
    pub fn with_rune(mut self, rune: impl Into<Rune>) -> Self {
        self.rune = rune.into();
        self
    }
}

impl From<char> for Glyph {
    /// Creates a glyph from a character.
    fn from(c: char) -> Self {
        Self {
            rune: c.into(),
            ..Default::default()
        }
    }
}

impl From<BoxDraw> for Glyph {
    /// Creates a glyph from a box-drawing marker.
    fn from(value: BoxDraw) -> Self {
        Self {
            rune: value.into(),
            ..Default::default()
        }
    }
}
