//! File: src/ui/widget.rs

#[path = "button.rs"]
mod button;

#[path = "input.rs"]
mod input;

pub use button::ButtonWidget;
pub use input::InputWidget;

use crate::{
    geom::{Point, Rect},
    surface::Pane,
};

/// Current state for a `Widget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetState {
    pub(crate) hovered: bool, // Hover marker.
    pub(crate) focused: bool, // Focus marker.
    pub(crate) pressed: bool, // Pressed marker.
    pub(crate) damaged: bool, // Damaged marker.
}

impl WidgetState {
    /// Sets the widget's hovered state.
    pub fn set_hovered(&mut self, value: bool) {
        if self.hovered != value {
            self.hovered = value;
            self.damaged = true;
        }
    }

    /// Sets the widget's pressed state.
    pub fn set_pressed(&mut self, value: bool) {
        if self.pressed != value {
            self.pressed = value;
            self.damaged = true;
        }
    }

    /// Sets the widget's focused state.
    pub fn set_focused(&mut self, value: bool) {
        if self.focused != value {
            self.focused = value;
            self.damaged = true;
        }
    }
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            hovered: false,
            focused: false,
            pressed: false,
            damaged: true,
        }
    }
}

/// Type-erased `Widget` wrapper.
pub enum Widget {
    /// Text-input widget.
    Input(InputWidget),
    /// Button widget.
    Button(ButtonWidget),
}

impl Widget {
    /// Returns the cursor position for this widget within the given pane.
    ///
    /// The returned `Point` is in canvas coordinates. Widgets that do not expose a
    /// cursor return `None`.
    pub fn cursor_pos(&self, pane: &Pane, rect: Rect) -> Option<Point> {
        match self {
            Self::Input(widget) => widget.cursor_pos(pane, rect),
            Self::Button(_) => None,
        }
    }

    /// Renders this widget into the given pane within `rect`.
    pub fn render(&mut self, pane: &mut Pane, rect: Rect) {
        match self {
            Self::Input(widget) => widget.render(pane, rect),
            Self::Button(widget) => widget.render(pane, rect),
        }
    }

    /// Updates the hovered state for this widget.
    pub fn set_hovered(&mut self, value: bool) {
        match self {
            Self::Input(w) => w.state.set_hovered(value),
            Self::Button(w) => w.state.set_hovered(value),
        }
    }

    /// Updates the pressed state for this widget.
    pub fn set_pressed(&mut self, value: bool) {
        match self {
            Self::Input(w) => w.state.set_pressed(value),
            Self::Button(w) => w.state.set_pressed(value),
        }
    }

    /// Updates the focused state for this widget.
    pub fn set_focused(&mut self, value: bool) {
        match self {
            Self::Input(w) => w.state.set_focused(value),
            Self::Button(w) => w.state.set_focused(value),
        }
    }

    /// Updates the damaged state for this widget.
    pub fn set_damaged(&mut self, damaged: bool) {
        match self {
            Self::Input(widget) => widget.state.damaged = damaged,
            Self::Button(widget) => widget.state.damaged = damaged,
        }
    }

    /// Backwards-compatible alias for `set_damaged`.
    pub fn damaged(&mut self, damaged: bool) {
        self.set_damaged(damaged);
    }

    /// Returns a mutable reference to the inner `InputWidget`, if this is an input widget.
    pub fn as_input_mut(&mut self) -> Option<&mut InputWidget> {
        match self {
            Self::Input(widget) => Some(widget),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner `ButtonWidget`, if this is a button widget.
    pub fn as_button_mut(&mut self) -> Option<&mut ButtonWidget> {
        match self {
            Self::Button(widget) => Some(widget),
            _ => None,
        }
    }
}

impl From<InputWidget> for Widget {
    fn from(value: InputWidget) -> Self {
        Self::Input(value)
    }
}

impl From<ButtonWidget> for Widget {
    fn from(value: ButtonWidget) -> Self {
        Self::Button(value)
    }
}
