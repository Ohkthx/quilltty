//! File: src/ui/log.rs

use crate::{
    StyledLine, StyledSpan,
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
    ui::{InteractionStyle, StylableWidgetExt, Widget, WidgetState},
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

        if self.max_entries > 0 && self.lines.len() > self.max_entries {
            let overflow = self.lines.len() - self.max_entries;
            self.lines.drain(0..overflow);
        }

        self.state.damaged = true;
    }

    /// Appends a pre-styled line to the log.
    pub fn push_line(&mut self, line: StyledLine) {
        self.lines.push(line);

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

    /// Converts stored styled lines into physical rows that fit a given width.
    fn layout_rows(&self, width: usize, interaction_style: Style) -> Vec<Vec<Glyph>> {
        let mut rows = Vec::new();

        for line in &self.lines {
            let mut row = Vec::with_capacity(width);

            for span in line.spans() {
                let span_style = if interaction_style == Style::default() {
                    span.style
                } else {
                    interaction_style
                };

                for ch in span.text.chars() {
                    if row.len() >= width {
                        if self.wrap {
                            rows.push(std::mem::take(&mut row));
                        } else {
                            break;
                        }
                    }

                    row.push(Glyph::from(ch).with_style(span_style));
                }

                if !self.wrap && row.len() >= width {
                    break;
                }
            }

            rows.push(row);
        }

        rows
    }
}

impl Widget for LogWidget {
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

        let interaction_style = self.interaction.style(&self.state);
        self.clear_content(pane, rect, interaction_style);

        let rows = self.layout_rows(rect.width, interaction_style);

        let visible_len = rows.len().min(rect.height);
        let start = if self.ascending {
            rows.len().saturating_sub(visible_len)
        } else {
            0
        };

        let y_offset = if self.ascending {
            rect.height.saturating_sub(visible_len)
        } else {
            0
        };

        for (row_idx, row) in rows[start..start + visible_len].iter().enumerate() {
            let y = rect.y + y_offset + row_idx;
            if !row.is_empty() {
                pane.write_glyphs(Point::new(rect.x, y), row);
            }
        }

        self.state.damaged = false;
    }
}

impl StylableWidgetExt for LogWidget {}
