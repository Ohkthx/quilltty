//! File: src/ui/button.rs

use crate::{
    geom::Rect,
    surface::Pane,
    ui::{
        InteractionStyle, WidgetAction,
        traits::{HasInteractionStyle, HasWidgetState, WidgetBehavior, WidgetRender},
        widget::WidgetState,
    },
};

/// A clickable widget.
pub struct ButtonWidget {
    pub(crate) state: WidgetState,     // Current state.
    pub interaction: InteractionStyle, // Style for interaction.
    label: String,                     // Text to render on the button.
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

impl HasWidgetState for ButtonWidget {
    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

impl HasInteractionStyle for ButtonWidget {
    fn interaction(&self) -> &InteractionStyle {
        &self.interaction
    }

    fn interaction_mut(&mut self) -> &mut InteractionStyle {
        &mut self.interaction
    }
}

impl WidgetBehavior for ButtonWidget {
    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::Clicked
    }
}

impl WidgetRender for ButtonWidget {
    /// Renders the button onto its parent `Pane`.
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

        let row = Self::glyph_row(&self.label, style, rect.width);
        self.write_glyph_row(pane, rect, 0, &row);

        self.state.damaged = false;
    }
}
