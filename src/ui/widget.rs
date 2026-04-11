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

use std::any::Any;

use crossterm::event::KeyCode;

pub use button::ButtonWidget;
pub use checkbox::CheckboxWidget;
pub use input::InputWidget;
pub use log::LogWidget;
pub use progress::ProgressWidget;
pub use slider::SliderWidget;
pub use text::{StyledLine, StyledSpan, TextWidget};

use crate::{
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
};

#[derive(Debug)]
pub enum WidgetAction {
    None,

    Clicked,
    Released,

    CheckboxChanged(bool),
    InputChanged,
    InputSubmitted(String),
    SliderChanged(f64),

    Custom(Box<dyn Any + Send>),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionStyle {
    pub normal: Style,
    pub hover: Style,
    pub pressed: Style,
    pub focused: Style,
}

impl InteractionStyle {
    /// Obtains the style for the state provided, otherwise default style provided.
    #[inline]
    pub fn style(&self, state: &WidgetState) -> Style {
        if state.pressed {
            self.pressed
        } else if state.hovered {
            self.hover
        } else if state.focused {
            self.focused
        } else {
            self.normal
        }
    }
}

/// Current state for a `Widget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetState {
    hovered: bool, // Hover marker.
    focused: bool, // Focus marker.
    pressed: bool, // Pressed marker.
    damaged: bool, // Damaged marker.
}

impl WidgetState {
    /// Sets the widget's hovered state.
    #[inline]
    pub fn set_hovered(&mut self, value: bool) {
        if self.hovered != value {
            self.hovered = value;
            self.set_damaged(true);
        }
    }

    /// Sets the widget's pressed state.
    #[inline]
    pub fn set_pressed(&mut self, value: bool) {
        if self.pressed != value {
            self.pressed = value;
            self.set_damaged(true);
        }
    }

    /// Sets the widget's focused state.
    #[inline]
    pub fn set_focused(&mut self, value: bool) {
        if self.focused != value {
            self.focused = value;
            self.set_damaged(true);
        }
    }

    /// Sets the widget's damaged state.
    #[inline]
    pub fn set_damaged(&mut self, value: bool) {
        if self.damaged != value {
            self.damaged = value;
        }
    }

    /// Gets the hovered state for the widget.
    #[inline]
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Gets the pressed state for the widget.
    #[inline]
    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// Gets the focused state for the widget.
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Gets the damaged state for the widget.
    #[inline]
    fn is_damaged(&self) -> bool {
        self.damaged
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

pub trait Widget: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn state(&self) -> &WidgetState;
    fn state_mut(&mut self) -> &mut WidgetState;

    #[inline]
    fn interaction(&self) -> Option<&InteractionStyle> {
        None
    }

    #[inline]
    fn interaction_mut(&mut self) -> Option<&mut InteractionStyle> {
        None
    }

    // Rendering

    fn draw(&mut self, pane: &mut Pane, rect: Rect);

    #[inline]
    fn cursor_pos(&self, _pane: &Pane, _rect: Rect) -> Option<Point> {
        None
    }

    // Actions

    #[inline]
    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::None
    }

    #[inline]
    fn key_action(&mut self, _key: KeyCode) -> WidgetAction {
        WidgetAction::None
    }

    #[inline]
    fn drag_action(&mut self, _local_x: usize, _width: usize) -> WidgetAction {
        WidgetAction::None
    }

    #[inline]
    fn release_action(&mut self, focused: bool) -> WidgetAction {
        if focused {
            WidgetAction::Clicked
        } else {
            WidgetAction::Released
        }
    }

    // Status

    #[inline]
    fn set_hovered(&mut self, value: bool) {
        self.state_mut().set_hovered(value);
    }

    #[inline]
    fn set_pressed(&mut self, value: bool) {
        self.state_mut().set_pressed(value);
    }

    #[inline]
    fn set_focused(&mut self, value: bool) {
        self.state_mut().set_focused(value);
    }

    #[inline]
    fn set_damaged(&mut self, value: bool) {
        self.state_mut().set_damaged(value);
    }

    #[inline]
    fn is_hovered(&self) -> bool {
        self.state().is_hovered()
    }

    #[inline]
    fn is_pressed(&self) -> bool {
        self.state().is_pressed()
    }

    #[inline]
    fn is_focused(&self) -> bool {
        self.state().is_focused()
    }

    // Helpers

    /// Builds styled glyphs for a single row of text.
    fn glyph_row(&self, text: &str, style: Style, width: usize) -> Vec<Glyph> {
        text.chars()
            .take(width)
            .map(|ch| Glyph::from(ch).with_style(style))
            .collect()
    }

    /// Writes one glyph row into the widget rectangle.
    fn write_glyph_row(&self, pane: &mut Pane, rect: Rect, row: usize, glyphs: &[Glyph]) {
        if row >= rect.height || glyphs.is_empty() {
            return;
        }

        pane.write_glyphs(Point::new(rect.x, rect.y + row), glyphs);
    }

    #[inline]
    fn clear_before_draw(&self) -> bool {
        true
    }

    #[inline]
    fn clear_style(&self) -> Style {
        self.interaction()
            .map(|i| i.style(self.state()))
            .unwrap_or_default()
    }

    #[inline]
    fn clear_content(&self, pane: &mut Pane, rect: Rect, style: Style) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }

        pane.fill(rect, Glyph::from(' ').with_style(style));
    }
}

/// Fluent interaction-style builders for widgets that expose
/// `interaction_mut() -> Some(...)`.
pub trait StylableWidgetExt: Widget + Sized {
    #[must_use]
    fn with_interaction(mut self, interaction: InteractionStyle) -> Self {
        *self
            .interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)") =
            interaction;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_normal_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .normal = style;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_hover_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .hover = style;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_pressed_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .pressed = style;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_focus_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .focused = style;
        self.set_damaged(true);
        self
    }
}

pub(crate) fn widget_render(widget: &mut dyn Widget, pane: &mut Pane, rect: Rect) {
    if !widget.state().is_damaged() {
        return;
    }

    if rect.width == 0 || rect.height == 0 {
        widget.set_damaged(false);
        return;
    }

    if widget.clear_before_draw() {
        widget.clear_content(pane, rect, widget.clear_style());
    }

    widget.draw(pane, rect);
    widget.set_damaged(false);
}
