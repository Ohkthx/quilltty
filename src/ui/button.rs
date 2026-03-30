//! File: src/ui/button.rs

use crate::{
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
    ui::{
        InteractionStyle, WidgetAction,
        traits::{HasInteractionStyle, HasWidgetState, WidgetBehavior},
        widget::WidgetState,
    },
};

/// A clickable widget.
pub struct ButtonWidget {
    pub(crate) state: WidgetState,     // Current state.
    pub interaction: InteractionStyle, // Style for interaction.
    label: Option<String>,             // Text to render on the button.
}

impl ButtonWidget {
    /// Creates a new `ButtonWidget`.
    pub fn new<L>(label: Option<L>) -> Self
    where
        L: Into<String>,
    {
        Self {
            state: WidgetState::default(),
            interaction: InteractionStyle::default(),
            label: label.map(Into::into),
        }
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

        let style = self.interaction.style(&self.state);
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
