//! File: src/ui/checkbox.rs

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
pub struct CheckboxWidget {
    pub(crate) state: WidgetState,     // Current state.
    pub interaction: InteractionStyle, // Style for interaction.
    label: Option<String>,             // Text to render next to the checkbox.
    label_left: bool,                  // If the label is left or right of box.
    checked: bool,                     // Marked or not.
}

impl CheckboxWidget {
    /// Creates a new `CheckboxWidget`.
    pub fn new<L>(label: Option<L>, label_left: bool, checked: bool) -> Self
    where
        L: Into<String>,
    {
        Self {
            state: WidgetState::default(),
            interaction: InteractionStyle::default(),
            label: label.map(Into::into),
            label_left,
            checked,
        }
    }

    /// Renders the checkbox onto its parent `Pane`.
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

        let checked = if self.checked { "x" } else { " " };
        let label = if let Some(label) = self.label.as_deref() {
            if self.label_left {
                format!("{label}: [{checked}]")
            } else {
                format!("[{checked}]: {label}")
            }
        } else {
            format!("[{checked}]")
        };

        for (i, ch) in label.chars().take(rect.width).enumerate() {
            pane.set(
                Point::new(rect.x + i, rect.y),
                Glyph::from(ch).with_style(style),
            );
        }

        self.state.damaged = false;
    }

    /// Nullifies the drawn text.
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

    /// Gets the status of the checkbox.
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Sets the value of the checkbox.
    pub fn set_checked(&mut self, value: bool) {
        if self.checked != value {
            self.checked = value;
            self.state.damaged = true;
        }
    }

    /// Toggles the status of the checkbox.
    pub fn toggle(&mut self) -> bool {
        self.checked = !self.checked;
        self.state.damaged = true;
        self.checked
    }
}

impl HasWidgetState for CheckboxWidget {
    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

impl HasInteractionStyle for CheckboxWidget {
    fn interaction(&self) -> &InteractionStyle {
        &self.interaction
    }

    fn interaction_mut(&mut self) -> &mut InteractionStyle {
        &mut self.interaction
    }
}

impl WidgetBehavior for CheckboxWidget {
    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::CheckboxChanged(self.toggle())
    }

    fn release_action(&mut self, focused: bool) -> WidgetAction {
        if focused {
            WidgetAction::CheckboxChanged(self.toggle())
        } else {
            WidgetAction::Released
        }
    }
}
