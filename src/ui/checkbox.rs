//! File: src/ui/checkbox.rs

use crate::{
    geom::{Point, Rect},
    style::Glyph,
    surface::Pane,
    ui::{
        InteractionStyle, WidgetAction, WidgetRender,
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

impl WidgetRender for CheckboxWidget {
    /// Renders the checkbox onto its parent `Pane`.
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

        let row: Vec<Glyph> = label
            .chars()
            .take(rect.width)
            .map(|ch| Glyph::from(ch).with_style(style))
            .collect();

        if !row.is_empty() {
            pane.write_glyphs(Point::new(rect.x, rect.y), &row);
        }

        self.state.damaged = false;
    }
}
