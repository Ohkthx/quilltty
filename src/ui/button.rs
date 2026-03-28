//! File: src/ui/button.rs

use crate::{
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
    ui::widget::WidgetState,
};

/// A clickable widget.
pub struct ButtonWidget {
    label: Option<String>,         // Text to render on the button.
    hover_style: Style,            // Style when hovered.
    hit_style: Style,              // Style when pressed.
    focus_style: Style,            // Style when focused.
    pub(crate) state: WidgetState, // Current state.
}

impl ButtonWidget {
    /// Creates a new `ButtonWidget`.
    pub fn new<L>(label: Option<L>) -> Self
    where
        L: Into<String>,
    {
        Self {
            label: label.map(Into::into),
            hover_style: Style::new().bold(),
            hit_style: Style::new().inverse(),
            focus_style: Style::new().underline(),
            state: WidgetState::default(),
        }
    }

    /// Sets the hover style.
    #[must_use]
    pub fn with_hover_style(mut self, style: Style) -> Self {
        self.hover_style = style;
        self.state.damaged = true;
        self
    }

    /// Sets the pressed style.
    #[must_use]
    pub fn with_pressed_style(mut self, style: Style) -> Self {
        self.hit_style = style;
        self.state.damaged = true;
        self
    }

    /// Sets the focus style.
    #[must_use]
    pub fn with_focus_style(mut self, style: Style) -> Self {
        self.focus_style = style;
        self.state.damaged = true;
        self
    }

    /// Renders the button onto its parent `Pane`.
    pub(crate) fn render(&mut self, pane: &mut Pane, rect: Rect) {
        if !self.state.damaged {
            return;
        }

        if rect.width == 0 || rect.height == 0 {
            self.state.damaged = false;
            return;
        }

        let style = if self.state.pressed {
            self.hit_style
        } else if self.state.hovered {
            self.hover_style
        } else if self.state.focused {
            self.focus_style
        } else {
            Style::new()
        };

        self.clear_content(pane, rect, style);

        let label = self.label.as_deref().unwrap_or("");
        for (i, ch) in label.chars().take(rect.width).enumerate() {
            pane.set(
                Point::new(rect.x + i, rect.y),
                Glyph::from(ch).with_style(style),
            );
        }

        self.state.damaged = false;
    }

    fn clear_content(&self, pane: &mut Pane, rect: Rect, style: Style) {
        for y in 0..rect.height {
            for x in 0..rect.width {
                pane.set(
                    Point::new(rect.x + x, rect.y + y),
                    Glyph::from(' ').with_style(style),
                );
            }
        }
    }
}
