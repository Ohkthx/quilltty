//! File: src/surface/style.rs

use super::color::{Color, ColorSpec};

/// Packed text style state, including flags and a color-pair id.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Style(u32);

impl Style {
    // bits 0..7   flags
    // bits 8..23  pair_id
    // bits 24..31 reserved

    /// Bold text flag.
    pub const FLAG_BOLD: u32 = 1 << 0;

    /// Dim text flag.
    pub const FLAG_DIM: u32 = 1 << 1;

    /// Italic text flag.
    pub const FLAG_ITALIC: u32 = 1 << 2;

    /// Underline text flag.
    pub const FLAG_UNDERLINE: u32 = 1 << 3;

    /// Blink text flag.
    pub const FLAG_BLINK: u32 = 1 << 4;

    /// Strikethrough text flag.
    pub const FLAG_STRIKE: u32 = 1 << 5;

    /// Inverse text flag.
    pub const FLAG_INVERSE: u32 = 1 << 6;

    /// Mask of all supported style flags.
    pub const FLAG_MASK: u32 = Self::FLAG_BOLD
        | Self::FLAG_DIM
        | Self::FLAG_ITALIC
        | Self::FLAG_UNDERLINE
        | Self::FLAG_BLINK
        | Self::FLAG_STRIKE
        | Self::FLAG_INVERSE;

    /// Shift of the packed pair id.
    pub const PAIR_SHIFT: u32 = 8;

    /// Mask of the packed pair id.
    pub const PAIR_MASK: u32 = 0xFFFF << Self::PAIR_SHIFT;

    /// Number of inline colors reserved for builtin ANSI/default styling.
    pub const INLINE_COLOR_COUNT: u16 = 9;

    /// First pair id owned by the color atlas.
    pub const FIRST_EXTENDED_PAIR: u16 = Self::INLINE_COLOR_COUNT * Self::INLINE_COLOR_COUNT;

    /// Inline pair id for terminal-default fg/bg.
    pub const DEFAULT_PAIR: u16 = Self::builtin_pair_id(Color::Default, Color::Default);

    /// Creates a new default style without calling `Default` in const context.
    #[inline]
    pub const fn new() -> Self {
        Self((Self::DEFAULT_PAIR as u32) << Self::PAIR_SHIFT)
    }

    /// Encodes a builtin ANSI/default pair into an inline pair id.
    #[inline]
    pub const fn builtin_pair_id(fg: Color, bg: Color) -> u16 {
        (bg as u16) * Self::INLINE_COLOR_COUNT + (fg as u16)
    }

    /// Returns true when the pair id is stored inline.
    #[inline]
    pub const fn is_inline_pair_id(pair_id: u16) -> bool {
        pair_id < Self::FIRST_EXTENDED_PAIR
    }

    /// Decodes the inline foreground color from a builtin pair id.
    #[inline]
    pub const fn inline_fg_from_pair_id(pair_id: u16) -> Color {
        Color::from_u8((pair_id % Self::INLINE_COLOR_COUNT) as u8)
    }

    /// Decodes the inline background color from a builtin pair id.
    #[inline]
    pub const fn inline_bg_from_pair_id(pair_id: u16) -> Color {
        Color::from_u8((pair_id / Self::INLINE_COLOR_COUNT) as u8)
    }

    /// Sets the packed pair id.
    #[inline]
    pub fn with_pair(mut self, pair_id: u16) -> Self {
        self.set_pair(pair_id);
        self
    }

    /// Adds style flags to the style.
    #[inline]
    pub fn with_flags(mut self, flags: u32) -> Self {
        self.0 |= flags & Self::FLAG_MASK;
        self
    }

    /// Sets the foreground color for inline ANSI/default pairs.
    ///
    /// This is only safe when the style already uses an inline pair id.
    #[inline]
    pub fn with_fg(mut self, color: Color) -> Self {
        self.set_fg(color);
        self
    }

    /// Sets the background color for inline ANSI/default pairs.
    ///
    /// This is only safe when the style already uses an inline pair id.
    #[inline]
    pub fn with_bg(mut self, color: Color) -> Self {
        self.set_bg(color);
        self
    }

    /// Enables bold.
    #[inline]
    pub fn bold(mut self) -> Self {
        self.0 |= Self::FLAG_BOLD;
        self
    }

    /// Enables dim text.
    #[inline]
    pub fn dim(mut self) -> Self {
        self.0 |= Self::FLAG_DIM;
        self
    }

    /// Enables italic text.
    #[inline]
    pub fn italic(mut self) -> Self {
        self.0 |= Self::FLAG_ITALIC;
        self
    }

    /// Enables underline text.
    #[inline]
    pub fn underline(mut self) -> Self {
        self.0 |= Self::FLAG_UNDERLINE;
        self
    }

    /// Enables blink text.
    #[inline]
    pub fn blink(mut self) -> Self {
        self.0 |= Self::FLAG_BLINK;
        self
    }

    /// Enables strikethrough text.
    #[inline]
    pub fn strike(mut self) -> Self {
        self.0 |= Self::FLAG_STRIKE;
        self
    }

    /// Enables inverse text.
    #[inline]
    pub fn inverse(mut self) -> Self {
        self.0 |= Self::FLAG_INVERSE;
        self
    }

    /// Sets the style from flags and builtin ANSI/default colors.
    #[inline]
    pub fn set(&mut self, flags: u32, fg: Color, bg: Color) {
        self.0 = 0;
        self.add_flags(flags);
        self.set_pair(Self::builtin_pair_id(fg, bg));
    }

    /// Adds specific flags to the style.
    #[inline]
    pub fn add_flags(&mut self, flags: u32) {
        self.0 |= flags & Self::FLAG_MASK;
    }

    /// Sets the pair id in place.
    #[inline]
    pub fn set_pair(&mut self, pair_id: u16) {
        self.0 = (self.0 & !Self::PAIR_MASK) | ((pair_id as u32) << Self::PAIR_SHIFT);
    }

    /// Sets the foreground color for inline ANSI/default pairs.
    #[inline]
    fn set_fg(&mut self, color: Color) {
        let bg = self.bg();
        self.set_pair(Self::builtin_pair_id(color, bg));
    }

    /// Sets the background color for inline ANSI/default pairs.
    #[inline]
    fn set_bg(&mut self, color: Color) {
        let fg = self.fg();
        self.set_pair(Self::builtin_pair_id(fg, color));
    }

    /// Removes all style flags while keeping the current pair id.
    #[inline]
    pub fn clear_flags(&mut self) {
        self.0 &= !Self::FLAG_MASK;
    }

    /// Obtains the set flags.
    #[inline]
    pub const fn flags(self) -> u32 {
        self.0 & Self::FLAG_MASK
    }

    /// Obtains the packed pair id.
    #[inline]
    pub const fn pair_id(self) -> u16 {
        ((self.0 & Self::PAIR_MASK) >> Self::PAIR_SHIFT) as u16
    }

    /// Obtains the inline foreground color or `Default` for atlas-backed pairs.
    #[inline]
    pub const fn fg(self) -> Color {
        if Self::is_inline_pair_id(self.pair_id()) {
            Self::inline_fg_from_pair_id(self.pair_id())
        } else {
            Color::Default
        }
    }

    /// Obtains the inline background color or `Default` for atlas-backed pairs.
    #[inline]
    pub const fn bg(self) -> Color {
        if Self::is_inline_pair_id(self.pair_id()) {
            Self::inline_bg_from_pair_id(self.pair_id())
        } else {
            Color::Default
        }
    }
}

impl Default for Style {
    /// Creates the default style.
    fn default() -> Self {
        Self::new()
    }
}

/// Sparse authoring-time style patch for rich widget drawing.
///
/// This patch is applied while widgets prepare final glyph styles. It is not a
/// replacement for the compact resolved `Style` stored in glyph buffers.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StylePatch {
    /// Flags to OR into the base style.
    pub add_flags: u32,
    /// Optional foreground replacement.
    pub fg: Option<ColorSpec>,
    /// Optional background replacement.
    pub bg: Option<ColorSpec>,
}

impl StylePatch {
    /// Creates an empty patch.
    #[inline]
    pub const fn new() -> Self {
        Self {
            add_flags: 0,
            fg: None,
            bg: None,
        }
    }

    /// Returns a copy with extra flags added.
    #[inline]
    pub const fn with_add_flags(mut self, flags: u32) -> Self {
        self.add_flags = flags;
        self
    }

    /// Returns a copy with a foreground replacement.
    #[inline]
    pub const fn with_fg(mut self, fg: ColorSpec) -> Self {
        self.fg = Some(fg);
        self
    }

    /// Returns a copy with a background replacement.
    #[inline]
    pub const fn with_bg(mut self, bg: ColorSpec) -> Self {
        self.bg = Some(bg);
        self
    }

    /// Returns `true` when the patch changes nothing.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.add_flags == 0 && self.fg.is_none() && self.bg.is_none()
    }
}
