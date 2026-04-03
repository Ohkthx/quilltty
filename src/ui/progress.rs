//! File: src/ui/progress.rs

use crate::{
    HasInteractionStyle, InteractionStyle, WidgetRender,
    surface::{Glyph, Pane, Point, Rect, Style},
    ui::{
        traits::{HasWidgetState, WidgetBehavior},
        widget::WidgetState,
    },
};

/// Shows progress in the form of a bar.
pub struct ProgressWidget {
    pub(crate) state: WidgetState,     // Current state.
    pub interaction: InteractionStyle, // Style for interaction.
    label: Option<String>,             // Text to render on the progress bar.
    glyph: Glyph,                      // Glyph to render to show progress.
    min: f64,                          // Minimum value.
    max: f64,                          // Maximum value.
    value: f64,                        // Current progress.
}

impl ProgressWidget {
    /// Creates a new `ProgressWidget`.
    pub fn new<L>(label: Option<L>, min: f64, max: f64, value: f64) -> Self
    where
        L: Into<String>,
    {
        Self {
            state: WidgetState::default(),
            interaction: InteractionStyle::default(),
            label: label.map(Into::into),
            glyph: Glyph::from('█').with_style(Style::new()),
            min,
            max,
            value,
        }
    }

    /// Sets the Glyph that should be rendered as progress.
    pub fn with_glyph(mut self, glyph: Glyph) -> Self {
        self.glyph = glyph;
        self
    }

    /// Obtains the current value of the progress.
    pub fn value(&self) -> f64 {
        self.value.clamp(self.min, self.max)
    }

    /// Sets the progress to the specified amount.
    pub fn set(&mut self, value: f64) {
        let next = value.clamp(self.min, self.max);
        if self.value != next {
            self.value = next;
            self.state.damaged = true;
        }
    }
}

impl HasWidgetState for ProgressWidget {
    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

impl HasInteractionStyle for ProgressWidget {
    fn interaction(&self) -> &InteractionStyle {
        &self.interaction
    }

    fn interaction_mut(&mut self) -> &mut InteractionStyle {
        &mut self.interaction
    }
}

impl WidgetBehavior for ProgressWidget {}

impl WidgetRender for ProgressWidget {
    /// Renders the progress bar onto its parent `Pane`.
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

        let label = self.label.as_deref().unwrap_or("");
        let label_len = label.chars().count().min(rect.width);

        let frame_len = 2;
        let bar_len = rect.width.saturating_sub(label_len + frame_len);

        let range = (self.max - self.min).abs();
        let ratio = if range == 0.0 {
            1.0
        } else {
            ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
        };

        let filled = (ratio * bar_len as f64).round() as usize;

        let mut row = Vec::with_capacity(rect.width);

        row.extend(
            label
                .chars()
                .take(rect.width)
                .map(|ch| Glyph::from(ch).with_style(style)),
        );

        if row.len() < rect.width {
            row.push(Glyph::from('[').with_style(style));
        }

        for bar_x in 0..bar_len {
            if row.len() >= rect.width {
                break;
            }

            let glyph = if bar_x < filled {
                self.glyph
            } else {
                Glyph::from(' ').with_style(Style::new())
            };

            row.push(glyph);
        }

        if row.len() < rect.width && rect.width >= label_len + 2 {
            row.push(Glyph::from(']').with_style(style));
        }

        if !row.is_empty() {
            pane.write_glyphs(Point::new(rect.x, rect.y), &row);
        }

        self.state.damaged = false;
    }
}
