//! File: src/ui/text.rs

use crate::{
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
    ui::widget::{InteractionStyle, WidgetState},
};

/// A styled run of text within a line.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    text: String,
    style: Style,
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
    spans: Vec<StyledSpan>,
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

    /// Sets the style used while the widget is hovered.
    #[must_use]
    pub fn with_hover_style(mut self, style: Style) -> Self {
        self.interaction.hover = style;
        self.state.damaged = true;
        self
    }

    /// Sets the style used while the widget is pressed.
    #[must_use]
    pub fn with_pressed_style(mut self, style: Style) -> Self {
        self.interaction.pressed = style;
        self.state.damaged = true;
        self
    }

    /// Sets the style used while the widget is focused.
    #[must_use]
    pub fn with_focus_style(mut self, style: Style) -> Self {
        self.interaction.focused = style;
        self.state.damaged = true;
        self
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

    /// Renders the widget into `rect` within the parent pane.
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

        let mut out_y = 0;

        'outer: for line in &self.lines {
            let mut out_x = 0;

            for span in line.spans() {
                let span_style = if interaction_style == Style::default() {
                    span.style
                } else {
                    interaction_style
                };

                for ch in span.text.chars() {
                    if out_x >= rect.width {
                        if self.wrap {
                            out_x = 0;
                            out_y += 1;
                        } else {
                            break;
                        }
                    }

                    if out_y >= rect.height {
                        break 'outer;
                    }

                    pane.set(
                        Point::new(rect.x + out_x, rect.y + out_y),
                        Glyph::from(ch).with_style(span_style),
                    );
                    out_x += 1;
                }

                if !self.wrap && out_x >= rect.width {
                    break;
                }
            }

            out_y += 1;
            if out_y >= rect.height {
                break;
            }
        }

        self.state.damaged = false;
    }

    /// Clears the widget's drawing area using `style`.
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
