//! File: src/ui/text.rs

use crate::{
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
    ui::{
        WidgetRender,
        traits::{HasInteractionStyle, HasWidgetState, WidgetBehavior},
        widget::{InteractionStyle, WidgetState},
    },
};

/// A styled run of text within a line.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub(crate) text: String,
    pub(crate) style: Style,
}

impl StyledSpan {
    /// Creates a span with default styling.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
        }
    }

    /// Creates a span with the provided styling.
    pub fn with_style(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    /// Replaces the span text.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Replaces the span style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns the number of characters in the span.
    pub fn len_chars(&self) -> usize {
        self.text.chars().count()
    }

    /// Collects the span into styled glyphs.
    pub fn glyphs(&self) -> Vec<Glyph> {
        let mut glyphs = Vec::with_capacity(self.len_chars());
        self.extend_glyphs(&mut glyphs);
        glyphs
    }

    /// Appends the span's styled glyphs into `out`.
    pub fn extend_glyphs(&self, out: &mut Vec<Glyph>) {
        out.reserve(self.len_chars());
        for ch in self.text.chars() {
            out.push(Glyph::from(ch).with_style(self.style));
        }
    }
}

/// A line of text composed of one or more styled spans.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyledLine {
    pub(crate) spans: Vec<StyledSpan>,
}

impl StyledLine {
    /// Creates an empty styled line.
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Returns a line with `span` appended.
    pub fn with_span(mut self, span: StyledSpan) -> Self {
        self.spans.push(span);
        self
    }

    /// Creates a line from the provided spans.
    pub fn with_spans<I>(spans: I) -> Self
    where
        I: IntoIterator<Item = StyledSpan>,
    {
        Self {
            spans: spans.into_iter().collect(),
        }
    }

    /// Appends a span to the line.
    pub fn push(&mut self, span: StyledSpan) {
        self.spans.push(span);
    }

    /// Appends unstyled text to the line.
    pub fn push_text(&mut self, text: impl Into<String>) {
        self.spans.push(StyledSpan::new(text));
    }

    /// Appends styled text to the line.
    pub fn push_styled(&mut self, text: impl Into<String>, style: Style) {
        self.spans.push(StyledSpan::with_style(text, style));
    }

    /// Returns the spans in this line.
    pub fn spans(&self) -> &[StyledSpan] {
        &self.spans
    }

    /// Returns `true` if the line has no spans.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Returns the number of spans in the line.
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Returns the total character count across all spans.
    pub fn len_chars(&self) -> usize {
        self.spans.iter().map(StyledSpan::len_chars).sum()
    }

    /// Collects the line into styled glyphs.
    pub fn glyphs(&self) -> Vec<Glyph> {
        let mut glyphs = Vec::with_capacity(self.len_chars());
        self.extend_glyphs(&mut glyphs);
        glyphs
    }

    /// Appends the line's styled glyphs into `out`.
    pub fn extend_glyphs(&self, out: &mut Vec<Glyph>) {
        out.reserve(self.len_chars());
        for span in &self.spans {
            span.extend_glyphs(out);
        }
    }
}

/// A text widget that renders styled lines with optional wrapping.
#[derive(Default)]
pub struct TextWidget {
    pub(crate) state: WidgetState,     // Current state.
    pub interaction: InteractionStyle, // Style for interaction.
    lines: Vec<StyledLine>,            // Lines to be displayed.
    wrap: bool,                        // Wrapped text goes to the next line.
}

impl TextWidget {
    /// Creates an empty text widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a text widget from the provided lines.
    pub fn with_lines<I>(lines: I) -> Self
    where
        I: IntoIterator<Item = StyledLine>,
    {
        let mut text = Self::default();
        for line in lines {
            text.push_line(line);
        }

        text
    }

    /// Enables or disables line wrapping.
    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Appends a new unstyled line to the widget.
    pub fn push(&mut self, text: impl Into<String>) {
        self.lines
            .push(StyledLine::with_spans([StyledSpan::new(text)]));
        self.state.damaged = true
    }

    /// Appends a styled line to the widget.
    pub fn push_line(&mut self, line: StyledLine) {
        self.lines.push(line);
        self.state.damaged = true
    }

    /// Replaces the widget contents with the provided lines.
    pub fn set_lines<I>(&mut self, lines: I)
    where
        I: IntoIterator<Item = StyledLine>,
    {
        self.lines = lines.into_iter().collect();
        self.state.damaged = true
    }

    /// Removes all lines from the widget.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.state.damaged = true
    }

    /// Returns the widget's styled lines.
    pub fn lines(&self) -> &[StyledLine] {
        self.lines.as_slice()
    }

    /// Returns the number of lines in the widget.
    pub fn len_lines(&self) -> usize {
        self.lines.len()
    }

    /// Returns `true` if the widget has no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Resolves the effective style for a span.
    fn resolved_span_style(&self, interaction_style: Style, span_style: Style) -> Style {
        if interaction_style == Style::default() {
            span_style
        } else {
            interaction_style
        }
    }

    /// Flushes one prepared row into the pane.
    fn flush_row(&self, pane: &mut Pane, rect: Rect, row: usize, glyphs: &mut Vec<Glyph>) {
        if row < rect.height && !glyphs.is_empty() {
            pane.write_glyphs(Point::new(rect.x, rect.y + row), glyphs);
            glyphs.clear();
        }
    }
}

impl HasWidgetState for TextWidget {
    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }
}

impl HasInteractionStyle for TextWidget {
    fn interaction(&self) -> &InteractionStyle {
        &self.interaction
    }

    fn interaction_mut(&mut self) -> &mut InteractionStyle {
        &mut self.interaction
    }
}

impl WidgetBehavior for TextWidget {}

impl WidgetRender for TextWidget {
    /// Renders the widget into `rect` within the parent pane.
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

        let mut out_y = 0;
        let mut row = Vec::with_capacity(rect.width);

        'outer: for line in &self.lines {
            row.clear();

            for span in line.spans() {
                let span_style = self.resolved_span_style(interaction_style, span.style);

                for ch in span.text.chars() {
                    if row.len() >= rect.width {
                        if self.wrap {
                            self.flush_row(pane, rect, out_y, &mut row);
                            out_y += 1;

                            if out_y >= rect.height {
                                break 'outer;
                            }
                        } else {
                            break;
                        }
                    }

                    row.push(Glyph::from(ch).with_style(span_style));
                }

                if !self.wrap && row.len() >= rect.width {
                    break;
                }
            }

            self.flush_row(pane, rect, out_y, &mut row);
            out_y += 1;

            if out_y >= rect.height {
                break;
            }
        }

        self.state.damaged = false;
    }
}
