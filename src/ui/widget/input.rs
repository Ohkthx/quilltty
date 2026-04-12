//! File: src/ui/widget/input.rs

use crossterm::event::KeyCode;

use super::{InteractionStyle, StylableWidgetExt, Widget, WidgetAction, WidgetState, merge_style};
use crate::{
    geom::{Point, Rect},
    style::{Glyph, Style},
    surface::Pane,
};

/// A widget that allows text entry.
pub struct InputWidget {
    pub(crate) state: WidgetState,     // Current state.
    pub interaction: InteractionStyle, // Style for interaction.
    label: Option<String>,             // Label to display.
    label_style: Style,                // Style used to render the label of the widget.
    style: Style,                      // Style used to render the widget.
    placeholder: Option<String>,       // Placeholder text when empty.
    buffer: String,                    // Stores the text already entered.
    cursor: usize,                     // Cursor position in bytes.
}

impl InputWidget {
    /// Creates a new `InputWidget` with an optional label and placeholder.
    pub fn new<L, P>(label: Option<L>, placeholder: Option<P>) -> Self
    where
        L: Into<String>,
        P: Into<String>,
    {
        Self {
            state: WidgetState::default(),
            interaction: InteractionStyle::default(),
            label: label.map(Into::into),
            label_style: Style::default(),
            style: Style::new().underline(),
            placeholder: placeholder.map(Into::into),
            buffer: String::new(),
            cursor: 0,
        }
    }

    /// Sets the render style for this widget.
    #[must_use]
    pub fn with_label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self.state_mut().set_damaged(true);
        self
    }

    /// Sets the render style for this widget.
    #[must_use]
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self.state_mut().set_damaged(true);
        self
    }

    /// Returns the current input buffer.
    pub fn value(&self) -> &str {
        &self.buffer
    }

    /// Inserts a character into the text buffer.
    pub fn insert_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.state_mut().set_damaged(true);
    }

    /// Removes a character from the text buffer.
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let prev = self.buffer[..self.cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);

        self.buffer.drain(prev..self.cursor);
        self.cursor = prev;
        self.state_mut().set_damaged(true);
    }

    /// Moves the cursor left by one character.
    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }

        self.cursor = self.buffer[..self.cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);

        self.state_mut().set_damaged(true);
    }

    /// Moves the cursor right by one character.
    pub fn move_right(&mut self) {
        if self.cursor >= self.buffer.len() {
            return;
        }

        let next = self.buffer[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.buffer.len());

        self.cursor = next;
        self.state_mut().set_damaged(true);
    }

    /// Extracts the current input and clears the buffer.
    pub fn submit(&mut self) -> String {
        self.cursor = 0;
        self.state_mut().set_damaged(true);
        std::mem::take(&mut self.buffer)
    }

    /// Renders a multiline `InputWidget`.
    fn render_multiline(&self, pane: &mut Pane, rect: Rect) {
        let Rect { x, y, width, .. } = rect;

        if let Some(label) = self.label.as_deref() {
            self.draw_text(pane, Point::new(x, y), label, true, width);
        }

        let input_row = usize::from(self.label.is_some());
        let text = if self.buffer.is_empty() {
            self.placeholder.as_deref().unwrap_or("")
        } else {
            &self.buffer
        };

        self.draw_text(pane, Point::new(x, y + input_row), text, false, width);
    }

    /// Renders a single-line `InputWidget`.
    fn render_single_line(&self, pane: &mut Pane, rect: Rect) {
        let Rect {
            x: ox,
            y: oy,
            width,
            ..
        } = rect;
        let mut x = 0;

        if let Some(label) = self.label.as_deref() {
            let prefix = format!("{label}: ");
            x += self.draw_text(pane, Point::new(ox + x, oy), &prefix, true, width);
        }

        if x >= width {
            return;
        }

        let text = if self.buffer.is_empty() {
            self.placeholder.as_deref().unwrap_or("")
        } else {
            &self.buffer
        };

        self.draw_text(
            pane,
            Point::new(ox + x, oy),
            text,
            false,
            width.saturating_sub(x),
        );
    }

    /// Returns the current cursor position within the widget.
    pub fn cursor_pos(&self, pane: &Pane, rect: Rect) -> Option<Point> {
        let Rect {
            x,
            y,
            width,
            height,
        } = rect;
        if width == 0 || height == 0 {
            return None;
        }

        let label_inline_cols = if height > 1 {
            0
        } else {
            self.inline_label_width()
        };

        let col = (label_inline_cols + self.cursor_col()).min(width.saturating_sub(1));
        let row = usize::from(height > 1 && self.label.is_some());

        let content = pane.content_rect();
        Some(Point::new(content.x + x + col, content.y + y + row))
    }

    /// Returns the cursor column.
    fn cursor_col(&self) -> usize {
        self.buffer[..self.cursor].chars().count()
    }

    /// Returns the inline label width.
    fn inline_label_width(&self) -> usize {
        self.label
            .as_deref()
            .map(|label| label.chars().count() + 2)
            .unwrap_or(0)
    }

    /// Draws text into the `Pane`.
    fn draw_text(
        &self,
        pane: &mut Pane,
        origin: Point,
        text: &str,
        label: bool,
        width: usize,
    ) -> usize {
        let base_style = if label { self.label_style } else { self.style };
        let style = merge_style(base_style, self.interaction.style(&self.state));

        let glyphs: Vec<Glyph> = text
            .chars()
            .take(width)
            .map(|ch| Glyph::from(ch).with_style(style))
            .collect();

        let written = glyphs.len();
        if written > 0 {
            pane.write_glyphs(origin, &glyphs);
        }

        written
    }
}

impl Widget for InputWidget {
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

    fn draw(&mut self, pane: &mut Pane, rect: Rect) {
        if rect.height > 1 {
            self.render_multiline(pane, rect);
        } else {
            self.render_single_line(pane, rect);
        }
    }

    fn cursor_pos(&self, pane: &Pane, rect: Rect) -> Option<Point> {
        InputWidget::cursor_pos(self, pane, rect)
    }

    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::InputSubmitted(self.submit())
    }

    fn key_action(&mut self, key: KeyCode) -> WidgetAction {
        match key {
            KeyCode::Char(ch) => self.insert_char(ch),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            _ => return WidgetAction::None,
        }

        WidgetAction::InputChanged
    }
}

impl StylableWidgetExt for InputWidget {}
