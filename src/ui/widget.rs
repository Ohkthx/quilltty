//! File: src/ui/widget.rs

#[path = "button.rs"]
mod button;

#[path = "input.rs"]
mod input;

#[path = "checkbox.rs"]
mod checkbox;

#[path = "text.rs"]
mod text;

#[path = "progress.rs"]
mod progress;

#[path = "slider.rs"]
mod slider;

#[path = "log.rs"]
mod log;

use crossterm::event::KeyCode;

pub use button::ButtonWidget;
pub use checkbox::CheckboxWidget;
pub use input::InputWidget;
pub use log::LogWidget;
pub use progress::ProgressWidget;
pub use slider::SliderWidget;
pub use text::TextWidget;

use crate::{
    geom::{Point, Rect},
    style::Style,
    surface::Pane,
    ui::traits::{HasWidgetState, WidgetBehavior, WidgetRender},
};

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetAction {
    None,
    Clicked,
    Released,
    CheckboxChanged(bool),
    InputChanged,
    InputSubmitted(String),
    SliderChanged(f64),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionStyle {
    pub hover: Style,
    pub pressed: Style,
    pub focused: Style,
}

impl InteractionStyle {
    /// Obtains the style for the state provided, otherwise default style provided.
    pub fn style(&self, state: &WidgetState) -> Style {
        if state.pressed {
            self.pressed
        } else if state.hovered {
            self.hover
        } else if state.focused {
            self.focused
        } else {
            Style::new()
        }
    }
}

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
            damaged: true, // Force initial drawing.
        }
    }
}

widget_types! {
    input    => Input(InputWidget),
    button   => Button(ButtonWidget),
    checkbox => Checkbox(CheckboxWidget),
    text     => Text(TextWidget),
    progress => Progress(ProgressWidget),
    slider   => Slider(SliderWidget),
    log      => Log(LogWidget),
}
