//! File: src/ui/log.rs

use crate::{
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
    ui::{
        traits::{HasInteractionStyle, HasWidgetState, WidgetBehavior},
        widget::{
            InteractionStyle, WidgetState,
            text::{StyledLine, StyledSpan},
        },
    },
};

/// Renders a scroll-like log.
#[derive(Default)]
pub struct LogWidget {
    pub(crate) state: WidgetState, // Tracks hover/focus/pressed/damaged state.
    pub interaction: InteractionStyle, // Styles applied during widget interaction.
    lines: Vec<StyledLine>,        // Stored logical log lines.
    wrap: bool,                    // Wrap long lines across multiple rows when true.
    ascending: bool,               // Anchor newest visible rows at the bottom when true.
    max_entries: usize,            // Maximum number of logical lines to retain.
}

impl LogWidget {
    /// Creates an empty log widget with direction and retention settings.
    pub fn new(ascending: bool, max_entries: usize) -> Self {
        Self {
            ascending,
            max_entries,
            ..Default::default()
        }
    }

    /// Builds a log widget preloaded with styled lines.
    pub fn with_lines<I>(lines: I) -> Self
    where
        I: IntoIterator<Item = StyledLine>,
    {
        let mut log = Self::default();
        for line in lines {
            log.push_line(line);
        }

        log
    }

    /// Enables or disables line wrapping.
    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Appends a plain text line to the log.
    pub fn push(&mut self, log: impl Into<String>) {
        self.lines
            .push(StyledLine::with_spans([StyledSpan::new(log)]));

        // Drop the oldest lines once max_entries is exceeded.
        if self.max_entries > 0 && self.lines.len() > self.max_entries {
            let overflow = self.lines.len() - self.max_entries;
            self.lines.drain(0..overflow);
        }

        self.state.damaged = true;
    }

    /// Appends a pre-styled line to the log.
    pub fn push_line(&mut self, line: StyledLine) {
        self.lines.push(line);

        // Drop the oldest lines once the retention cap is exceeded.
        if self.max_entries > 0 && self.lines.len() > self.max_entries {
            let overflow = self.lines.len() - self.max_entries;
            self.lines.drain(0..overflow);
        }

        self.state.damaged = true;
    }

    /// Replaces all current log contents with a new line set.
    pub fn set_lines<I>(&mut self, lines: I)
    where
        I: IntoIterator<Item = StyledLine>,
    {
        self.lines = lines.into_iter().collect();
        self.state.damaged = true;
    }

    /// Removes every stored line from the log.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.state.damaged = true;
    }

    /// Returns all stored logical lines.
    pub fn lines(&self) -> &[StyledLine] {
        self.lines.as_slice()
    }

    /// Returns the count of stored logical lines.
    pub fn len_lines(&self) -> usize {
        self.lines.len()
    }

    /// Returns true when the log has no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Draws the visible portion of the log into the widget rectangle.
    pub(crate) fn render(&mut self, pane: &mut Pane, rect: Rect) {
        if !self.state.damaged {
            return;
        }

        if rect.width == 0 || rect.height == 0 {
            self.state.damaged = false;
            return;
        }

        let interaction_style = self.interaction.style(&self.state);
        self.clear_content(pane, rect, interaction_style);

        // Expand logical lines into physical rows after wrapping.
        let rows = self.layout_rows(rect.width, interaction_style);

        // Only as many rows as fit in the widget can be shown.
        // For ascending logs, show the newest rows; otherwise show from the top.
        let visible_len = rows.len().min(rect.height);
        let start = if self.ascending {
            rows.len().saturating_sub(visible_len)
        } else {
            0
        };

        // For ascending logs, bottom-align the visible rows inside the rect.
        let y_offset = if self.ascending {
            rect.height.saturating_sub(visible_len)
        } else {
            0
        };

        // Draw each visible row into the pane.
        for (row_idx, row) in rows[start..start + visible_len].iter().enumerate() {
            let y = rect.y + y_offset + row_idx;

            for (x, glyph) in row.iter().enumerate() {
                pane.set(Point::new(rect.x + x, y), *glyph);
            }
        }

        self.state.damaged = false;
    }

    /// Fills the widget area with spaces using the active background/style.
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

    /// Converts stored styled lines into physical rows that fit a given width.
    fn layout_rows(&self, width: usize, interaction_style: Style) -> Vec<Vec<Glyph>> {
        let mut rows = Vec::new();

        // Walk each logical line and split it into one or more physical rows.
        for line in &self.lines {
            let mut row = Vec::with_capacity(width);

            // Process each styled span in order.
            for span in line.spans() {
                // Use the interaction style when active, otherwise preserve span style.
                let span_style = if interaction_style == Style::default() {
                    span.style
                } else {
                    interaction_style
                };

                // Convert characters to glyphs and wrap as needed.
                for ch in span.text.chars() {
                    if row.len() >= width {
                        if self.wrap {
                            rows.push(std::mem::take(&mut row));
                        } else {
                            break; // Truncate the remaining line.
                        }
                    }

                    row.push(Glyph::from(ch).with_style(span_style));
                }

                // Stop early once the row is full when wrapping is disabled.
                if !self.wrap && row.len() >= width {
                    break;
                }
            }

            rows.push(row);
        }

        rows
    }
}

impl HasWidgetState for LogWidget {
    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

impl HasInteractionStyle for LogWidget {
    fn interaction(&self) -> &InteractionStyle {
        &self.interaction
    }

    fn interaction_mut(&mut self) -> &mut InteractionStyle {
        &mut self.interaction
    }
}

impl WidgetBehavior for LogWidget {}
