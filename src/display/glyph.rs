//! File: src/display/glyph.rs

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
    pub(crate) const fn horizontal(self) -> BoxDraw {
        match self {
            BorderKind::Single | BorderKind::Rounded => BoxDraw::Horizontal,
            BorderKind::Double => BoxDraw::DoubleHorizontal,
        }
    }

    pub(crate) const fn vertical(self) -> BoxDraw {
        match self {
            BorderKind::Single | BorderKind::Rounded => BoxDraw::Vertical,
            BorderKind::Double => BoxDraw::DoubleVertical,
        }
    }

    pub(crate) const fn top_left(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::TopLeft,
            BorderKind::Rounded => BoxDraw::RoundedTopLeft,
            BorderKind::Double => BoxDraw::DoubleTopLeft,
        }
    }

    pub(crate) const fn top_right(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::TopRight,
            BorderKind::Rounded => BoxDraw::RoundedTopRight,
            BorderKind::Double => BoxDraw::DoubleTopRight,
        }
    }

    pub(crate) const fn bottom_left(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::BottomLeft,
            BorderKind::Rounded => BoxDraw::RoundedBottomLeft,
            BorderKind::Double => BoxDraw::DoubleBottomLeft,
        }
    }

    pub(crate) const fn bottom_right(self) -> BoxDraw {
        match self {
            BorderKind::Single => BoxDraw::BottomRight,
            BorderKind::Rounded => BoxDraw::RoundedBottomRight,
            BorderKind::Double => BoxDraw::DoubleBottomRight,
        }
    }

    pub(crate) const fn cross(self) -> BoxDraw {
        match self {
            BorderKind::Double => BoxDraw::DoubleCross,
            _ => BoxDraw::Cross,
        }
    }
}

/// Basic ANSI colors used for foreground and background styling.
#[repr(u8)]
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
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
    const fn from_u8(n: u8) -> Self {
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

/// Packed text style state, including flags and foreground/background colors.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Style(u16);

impl Style {
    // bits 0..5   flags
    // bits 6..9   fg (4 bits)
    // bits 10..13 bg (4 bits)

    pub const FLAG_BOLD: u16 = 1 << 0;
    pub const FLAG_DIM: u16 = 1 << 1;
    pub const FLAG_ITALIC: u16 = 1 << 2;
    pub const FLAG_UNDERLINE: u16 = 1 << 3;
    pub const FLAG_BLINK: u16 = 1 << 4;
    pub const FLAG_STRIKE: u16 = 1 << 5;

    const FLAG_MASK: u16 = 0x3F;
    const FG_SHIFT: u16 = 6;
    const BG_SHIFT: u16 = 10;

    const FG_MASK: u16 = 0xF << Self::FG_SHIFT;
    const BG_MASK: u16 = 0xF << Self::BG_SHIFT;

    /// Create a new style with no flags and colors set to default.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the flags for the style.
    #[inline]
    pub fn with_flags(mut self, flags: u16) -> Self {
        self.0 |= flags & Self::FLAG_MASK;
        self
    }

    /// Sets the foreground color.
    #[inline]
    pub fn with_fg(mut self, color: Color) -> Self {
        self.0 = (self.0 & !Self::FG_MASK) | ((color as u16) << Self::FG_SHIFT);
        self
    }

    /// Sets the background color.
    #[inline]
    pub fn with_bg(mut self, color: Color) -> Self {
        self.0 = (self.0 & !Self::BG_MASK) | ((color as u16) << Self::BG_SHIFT);
        self
    }

    /// Enables bold.
    #[inline]
    pub fn bold(mut self) -> Self {
        self.0 |= Self::FLAG_BOLD;
        self
    }

    /// Style will dim text.
    #[inline]
    pub fn dim(mut self) -> Self {
        self.0 |= Self::FLAG_DIM;
        self
    }

    /// Style will be italicized.
    #[inline]
    pub fn italic(mut self) -> Self {
        self.0 |= Self::FLAG_ITALIC;
        self
    }

    /// Style will be underlined.
    #[inline]
    pub fn underline(mut self) -> Self {
        self.0 |= Self::FLAG_UNDERLINE;
        self
    }

    /// Style will blink (if supported.)
    #[inline]
    pub fn blink(mut self) -> Self {
        self.0 |= Self::FLAG_BLINK;
        self
    }

    /// Style will have a strikethrough.
    #[inline]
    pub fn strike(mut self) -> Self {
        self.0 |= Self::FLAG_STRIKE;
        self
    }

    /// Sets the flags, fg color, and bg color of a style.
    #[inline]
    pub fn set(&mut self, flags: u16, fg: Color, bg: Color) {
        self.0 = (flags & Self::FLAG_MASK)
            | ((fg as u16) << Self::FG_SHIFT)
            | ((bg as u16) << Self::BG_SHIFT);
    }

    /// Sets specific flags.
    #[inline]
    pub fn add_flags(&mut self, flags: u16) {
        self.0 |= flags & Self::FLAG_MASK;
    }

    /// Sets the foreground color.
    #[inline]
    fn set_fg(&mut self, color: Color) {
        self.0 = (self.0 & !Self::FG_MASK) | ((color as u16) << Self::FG_SHIFT);
    }

    /// Sets the background color.
    #[inline]
    fn set_bg(&mut self, color: Color) {
        self.0 = (self.0 & !Self::BG_MASK) | ((color as u16) << Self::BG_SHIFT);
    }

    /// Removes all style flags, excludes colors.
    #[inline]
    pub fn clear_flags(&mut self) {
        self.0 &= !Self::FLAG_MASK;
    }

    /// Obtains the set flags.
    #[inline]
    pub fn flags(&self) -> u16 {
        self.0 & Self::FLAG_MASK
    }

    /// Obtains the foreground color that is currently set.
    #[inline]
    pub fn fg(&self) -> Color {
        Color::from_u8(((self.0 >> Self::FG_SHIFT) & 0xF) as u8)
    }

    /// Obtains the background color that is currently set.
    #[inline]
    pub fn bg(&self) -> Color {
        Color::from_u8(((self.0 >> Self::BG_SHIFT) & 0xF) as u8)
    }
}

impl Default for Style {
    fn default() -> Self {
        Self(0).with_fg(Color::Default).with_bg(Color::Default)
    }
}

/// Represents a printable "character" to the screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rune {
    pub(crate) bytes: [u8; 4],
    pub(crate) len: u8,
}

impl From<char> for Rune {
    fn from(value: char) -> Self {
        let mut bytes = [0u8; 4];
        let len = value.encode_utf8(&mut bytes).len() as u8;

        Self { bytes, len }
    }
}

impl Default for Rune {
    fn default() -> Self {
        Self {
            bytes: [b' '; 4],
            len: 1,
        }
    }
}

impl From<BoxDraw> for Rune {
    fn from(value: BoxDraw) -> Self {
        char::from(value).into()
    }
}

/// A single screen cell that can be rendered with a rune and style.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// Attributes that are applied such as bold, blink, underline, color, etc.
    pub style: Style,
    /// Rendered data.
    pub rune: Rune,
}

impl Glyph {
    /// Constructs using default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a style to the rendered data.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Replaces the default value with the user provided.
    pub fn with_rune(mut self, rune: impl Into<Rune>) -> Self {
        self.rune = rune.into();
        self
    }
}

impl From<char> for Glyph {
    fn from(c: char) -> Self {
        Self {
            rune: c.into(),
            ..Default::default()
        }
    }
}

impl From<BoxDraw> for Glyph {
    fn from(value: BoxDraw) -> Self {
        Self {
            rune: value.into(),
            ..Default::default()
        }
    }
}
