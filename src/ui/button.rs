//! File: src/ui/button.rs

use crate::{
    geom::Rect,
    surface::Pane,
    ui::{InteractionStyle, StylableWidgetExt, Widget, WidgetAction, WidgetState},
};

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

    fn render(&mut self, pane: &mut Pane, rect: Rect) {
        if !self.state.damaged {
            return;
        }

        if rect.width == 0 || rect.height == 0 {
            self.state.damaged = false;
            return;
        }

        let style = self.interaction.style(&self.state);
        self.clear_content(pane, rect, style);

        let row = self.glyph_row(&self.label, style, rect.width);
        self.write_glyph_row(pane, rect, 0, &row);

        self.state.damaged = false;
    }

    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::Clicked
    }
}

impl StylableWidgetExt for ButtonWidget {}
