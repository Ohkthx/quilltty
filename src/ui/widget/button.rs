//! File: src/ui/widget/button.rs

use super::{InteractionStyle, StylableWidgetExt, Widget, WidgetAction, WidgetState};
use crate::{geom::Rect, surface::Pane};

/// A clickable widget.
pub struct ButtonWidget {
    pub(crate) state: WidgetState, // Current state.
    interaction: InteractionStyle, // Style for interaction.
    label: String,                 // Text to render on the button.
}

impl ButtonWidget {
    /// Creates a new `ButtonWidget`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            state: WidgetState::default(),
            interaction: InteractionStyle::default(),
            label: label.into(),
        }
    }
}

impl Widget for ButtonWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }

    fn interaction(&self) -> Option<&InteractionStyle> {
        Some(&self.interaction)
    }

    fn interaction_mut(&mut self) -> Option<&mut InteractionStyle> {
        Some(&mut self.interaction)
    }

    fn draw(&mut self, pane: &mut Pane, rect: Rect) {
        let style = self.interaction.style(self.state());
        let row = self.glyph_row(&self.label, style, rect.width);
        self.write_glyph_row(pane, rect, 0, &row);
    }

    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::Clicked
    }
}

impl StylableWidgetExt for ButtonWidget {}
