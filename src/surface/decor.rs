//! File: src/surface/decor.rs

use crate::{
    geom::{Point, Rect, Size},
    style::{BorderKind, Color, Glyph, Style},
    surface::pane::{Pane, PaneElement},
};

/// Insets reserved by pane decoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Insets {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
}

impl Insets {
    pub const ZERO: Self = Self {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
}

/// Decorations that may be applied to a pane.
#[derive(Debug, Clone)]
pub enum PaneDecor {
    None,
    Window(WindowDecor),
}

impl Default for PaneDecor {
    fn default() -> Self {
        Self::Window(WindowDecor::default())
    }
}

impl PaneDecor {
    /// Returns the insets contributed by the decoration.
    #[inline]
    pub fn insets(&self) -> Insets {
        match self {
            Self::None => Insets::ZERO,
            Self::Window(window) => window.insets(),
        }
    }

    /// Returns the minimum outer size contributed by the decoration.
    #[inline]
    pub fn min_outer_size(&self) -> Size {
        match self {
            Self::None => Size {
                width: 1,
                height: 1,
            },
            Self::Window(window) => window.min_outer_size(),
        }
    }

    /// Renders the decoration into the pane.
    #[inline]
    pub fn render(&self, pane: &mut Pane, focused: bool, resize: bool) {
        match self {
            Self::None => {}
            Self::Window(window) => window.render(pane, focused, resize),
        }
    }

    /// Hit-tests a pane-local position against the decoration.
    #[inline]
    pub fn hit_test(&self, pane: &Pane, local: Point) -> PaneElement {
        match self {
            Self::None => PaneElement::Content,
            Self::Window(window) => window.hit_test(pane, local),
        }
    }

    /// Sets the title when supported by the decoration.
    #[inline]
    pub fn set_title(&mut self, title: Option<String>) -> bool {
        match self {
            Self::None => false,
            Self::Window(window) => window.set_title(title),
        }
    }
}

/// Default window-like decoration used by the crate.
#[derive(Debug, Clone)]
pub struct WindowDecor {
    pub border: Option<BorderKind>,
    pub style: Style,
    pub title: Option<String>,
}

impl Default for WindowDecor {
    fn default() -> Self {
        Self {
            border: None,
            style: Style::default().with_fg(Color::White),
            title: None,
        }
    }
}

impl WindowDecor {
    /// Creates a new instance of WindowDecor with default behavior.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach or remove a border from the decor on creation.
    pub fn with_border(mut self, border: Option<BorderKind>) -> Self {
        self.border = border;
        self
    }

    /// Change the default style for the decor on creation.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Attach or remove title from the decor on creation.
    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    /// Returns the insets reserved by the window decoration.
    #[inline]
    pub fn insets(&self) -> Insets {
        if self.title.is_some() && self.border.is_none() {
            return Insets {
                top: 1,
                ..Insets::ZERO
            };
        }

        match self.border {
            Some(_) => Insets {
                left: 1,
                top: 1,
                right: 1,
                bottom: 1,
            },
            None => Insets::ZERO,
        }
    }

    /// Returns the minimum outer size for the window decoration.
    #[inline]
    pub fn min_outer_size(&self) -> Size {
        match self.border {
            Some(_) => Size {
                width: 2,
                height: 2,
            },
            None => Size {
                width: 1,
                height: 1,
            },
        }
    }

    /// Sets the title when it has changed.
    pub fn set_title(&mut self, title: Option<String>) -> bool {
        if self.title == title {
            return false;
        }

        self.title = title;
        true
    }

    /// Renders the decoration into the pane.
    pub fn render(&self, pane: &mut Pane, focused: bool, resize: bool) {
        if self.border.is_some() {
            self.draw_border(pane, focused, resize);
            self.draw_title(pane, focused);
        } else {
            self.clear_header_row(pane);
            self.draw_title(pane, focused);
        }
    }

    /// Hit-tests a pane-local position against the window decoration.
    pub fn hit_test(&self, pane: &Pane, local: Point) -> PaneElement {
        let Rect { width, height, .. } = pane.rect();

        if width == 0 || height == 0 {
            return PaneElement::Content;
        }

        if self.border.is_none() {
            if self.title.is_some() && local.y == 0 {
                return PaneElement::Title;
            }

            return PaneElement::Content;
        }

        let last_x = width.saturating_sub(1);
        let last_y = height.saturating_sub(1);

        if local.x == last_x && local.y == last_y {
            return PaneElement::Resize;
        }

        if local.y == 0 {
            return PaneElement::Title;
        }

        if local.x == 0 || local.y == 0 || local.x == last_x || local.y == last_y {
            return PaneElement::Border;
        }

        PaneElement::Content
    }

    /// Clears the header row before title rendering.
    fn clear_header_row(&self, pane: &mut Pane) {
        let Rect { width, height, .. } = pane.rect();
        if width == 0 || height == 0 {
            return;
        }

        for x in 0..width {
            pane.decor_raw_set(Point::new(x, 0), Glyph::default());
        }
    }

    /// Draws the border around the pane.
    fn draw_border(&self, pane: &mut Pane, focused: bool, resize: bool) {
        let Some(kind) = self.border else {
            return;
        };

        let Rect { width, height, .. } = pane.rect();
        if width < 2 || height < 2 {
            return;
        }

        let style = if focused {
            self.style.with_fg(Color::Red).bold()
        } else {
            self.style
        };

        let (h, v, tl, tr, bl, br) = kind.glyphs();
        let br = if resize { BorderKind::cross(kind) } else { br };

        pane.decor_raw_set(Point::new(0, 0), Glyph::from(tl).with_style(style));
        pane.decor_raw_set(Point::new(width - 1, 0), Glyph::from(tr).with_style(style));
        pane.decor_raw_set(Point::new(0, height - 1), Glyph::from(bl).with_style(style));
        pane.decor_raw_set(
            Point::new(width - 1, height - 1),
            Glyph::from(br).with_style(style),
        );

        for x in 1..width - 1 {
            pane.decor_raw_set(Point::new(x, 0), Glyph::from(h).with_style(style));
            pane.decor_raw_set(Point::new(x, height - 1), Glyph::from(h).with_style(style));
        }

        for y in 1..height - 1 {
            pane.decor_raw_set(Point::new(0, y), Glyph::from(v).with_style(style));
            pane.decor_raw_set(Point::new(width - 1, y), Glyph::from(v).with_style(style));
        }
    }

    /// Draws the title into the header area.
    fn draw_title(&self, pane: &mut Pane, focused: bool) {
        let Some(title) = self.title.as_deref() else {
            return;
        };

        let width = pane.width();
        if width == 0 {
            return;
        }

        let style = if focused {
            self.style.with_fg(Color::Red).bold()
        } else {
            self.style.bold()
        };

        let (start_x, max_len) = if self.border.is_some() {
            if width <= 2 {
                return;
            }

            (1, width - 2)
        } else {
            (0, width)
        };

        for (dx, ch) in title.chars().take(max_len).enumerate() {
            pane.decor_raw_set(
                Point::new(start_x + dx, 0),
                Glyph::from(ch).with_style(style),
            );
        }
    }
}
