//! File: src/ui/slider.rs

use crate::{
    prelude::*,
    style::{Glyph, Style},
    ui::{
        InteractionStyle, WidgetAction,
        traits::{HasInteractionStyle, HasWidgetState, WidgetBehavior},
        widget::WidgetState,
    },
};

/// Shows slider in the form of a bar.
pub struct SliderWidget {
    pub(crate) state: WidgetState,     // Current state.
    pub interaction: InteractionStyle, // Style for interaction.
    label: Option<String>,             // Text to render on the slider.
    glyph: Glyph,                      // Glyph to render to show slider.
    min: f64,                          // Minimum value.
    max: f64,                          // Maximum value.
    value: f64,                        // Current slider.
}

impl SliderWidget {
    /// Creates a new `SliderWidget`.
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

    /// Sets the Glyph that should be rendered as slider.
    pub fn with_glyph(mut self, glyph: Glyph) -> Self {
        self.glyph = glyph;
        self
    }

    /// Obtains the current value of the slider.
    pub fn value(&self) -> f64 {
        self.value.clamp(self.min, self.max)
    }

    /// Sets the slider to the specified amount.
    pub fn set(&mut self, value: f64) {
        let next = value.clamp(self.min, self.max);
        if self.value != next {
            self.value = next;
            self.state.damaged = true;
        }
    }

    /// Updates the slider using a widget-local x position. Returns `Some(new_value)` only when the slider changes.
    pub fn set_from_local_x(&mut self, local_x: usize, total_width: usize) -> Option<f64> {
        let label_len = self
            .label
            .as_deref()
            .unwrap_or("")
            .chars()
            .count()
            .min(total_width);
        let frame_len = 2;
        let bar_len = total_width.saturating_sub(label_len + frame_len);

        // No usable track.
        if bar_len <= 1 {
            return None;
        }

        // Track begins immediately after the opening '['.
        let bar_start = label_len + 1;
        let slot = local_x.saturating_sub(bar_start).min(bar_len - 1);
        let ratio = slot as f64 / (bar_len - 1) as f64;
        let next = self.min + (self.max - self.min) * ratio;

        let old = self.value();
        self.set(next);
        let new = self.value();

        (new != old).then_some(new)
    }
}

impl HasWidgetState for SliderWidget {
    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

impl HasInteractionStyle for SliderWidget {
    fn interaction(&self) -> &InteractionStyle {
        &self.interaction
    }

    fn interaction_mut(&mut self) -> &mut InteractionStyle {
        &mut self.interaction
    }
}

impl WidgetBehavior for SliderWidget {
    fn drag_action(&mut self, local_x: usize, width: usize) -> WidgetAction {
        self.set_from_local_x(local_x, width)
            .map(WidgetAction::SliderChanged)
            .unwrap_or(WidgetAction::None)
    }

    fn release_action(&mut self, _focused: bool) -> WidgetAction {
        WidgetAction::Released
    }
}

impl WidgetRender for SliderWidget {
    /// Renders the slider onto its parent `Pane`.
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

        let position = if bar_len == 0 {
            0
        } else {
            ((ratio * (bar_len.saturating_sub(1)) as f64).round() as usize)
                .min(bar_len.saturating_sub(1))
        };

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

            let glyph = if bar_x == position {
                self.glyph
            } else {
                Glyph::from('-').with_style(style)
            };

            row.push(glyph);
        }

        if row.len() < rect.width && rect.width >= label_len + 2 {
            row.push(Glyph::from(']').with_style(style));
        }

        self.write_glyph_row(pane, rect, 0, &row);

        self.state.damaged = false;
    }
}
