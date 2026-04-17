//! File: src/ui/widget/traits.rs

use std::any::Any;

use crossterm::event::KeyCode;

use crate::{
    geom::{Point, Rect},
    style::{ColorAtlas, Glyph, Style},
    surface::{Pane, StylePatch},
};

/// High-level actions emitted by widgets during input handling.
#[derive(Debug)]
pub enum WidgetAction {
    None,

    Clicked,
    Released,

    CheckboxChanged(bool),
    InputChanged,
    InputSubmitted(String),
    SliderChanged(f64),

    Custom(Box<dyn Any + Send>),
}

/// Fully resolved interaction styles for simple widgets.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionStyle {
    /// Style used when the widget is idle.
    pub normal: Style,
    /// Style used while the widget is hovered.
    pub hover: Style,
    /// Style used while the widget is pressed.
    pub pressed: Style,
    /// Style used while the widget is focused.
    pub focused: Style,
}

impl InteractionStyle {
    /// Returns the resolved style for the provided widget state.
    #[inline]
    pub fn style(&self, state: &WidgetState) -> Style {
        if state.is_pressed() {
            self.pressed
        } else if state.is_hovered() {
            self.hover
        } else if state.is_focused() {
            self.focused
        } else {
            self.normal
        }
    }
}

/// State-driven sparse interaction patches for rich widgets.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RichInteractionStyle {
    /// Patch used when the widget is idle.
    pub normal: StylePatch,
    /// Patch used while the widget is hovered.
    pub hover: StylePatch,
    /// Patch used while the widget is pressed.
    pub pressed: StylePatch,
    /// Patch used while the widget is focused.
    pub focused: StylePatch,
}

impl RichInteractionStyle {
    /// Returns the sparse patch for the provided widget state.
    #[inline]
    pub fn patch(&self, state: &WidgetState) -> StylePatch {
        if state.is_pressed() {
            self.pressed
        } else if state.is_hovered() {
            self.hover
        } else if state.is_focused() {
            self.focused
        } else {
            self.normal
        }
    }
}

/// Resolves a sparse patch against an already resolved base style.
#[inline]
pub(crate) fn resolve_patched_style(
    colors: &mut ColorAtlas,
    base: Style,
    patch: StylePatch,
) -> Style {
    if patch.is_empty() {
        return base;
    }

    let pair = colors.resolve_pair(base.pair_id());
    let fg = patch.fg.unwrap_or(pair.fg);
    let bg = patch.bg.unwrap_or(pair.bg);

    colors.style(base.flags() | patch.add_flags, fg, bg)
}

/// Current interaction and damage state for a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetState {
    hovered: bool, // Hover marker.
    focused: bool, // Focus marker.
    pressed: bool, // Pressed marker.
    damaged: bool, // Damaged marker.
}

impl WidgetState {
    /// Sets the widget's hovered state.
    #[inline]
    pub fn set_hovered(&mut self, value: bool) {
        if self.hovered != value {
            self.hovered = value;
            self.set_damaged(true);
        }
    }

    /// Sets the widget's pressed state.
    #[inline]
    pub fn set_pressed(&mut self, value: bool) {
        if self.pressed != value {
            self.pressed = value;
            self.set_damaged(true);
        }
    }

    /// Sets the widget's focused state.
    #[inline]
    pub fn set_focused(&mut self, value: bool) {
        if self.focused != value {
            self.focused = value;
            self.set_damaged(true);
        }
    }

    /// Sets the widget's damaged state.
    #[inline]
    pub fn set_damaged(&mut self, value: bool) {
        if self.damaged != value {
            self.damaged = value;
        }
    }

    /// Returns `true` when the widget is hovered.
    #[inline]
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Returns `true` when the widget is pressed.
    #[inline]
    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// Returns `true` when the widget is focused.
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Returns `true` when the widget needs redraw.
    #[inline]
    fn is_damaged(&self) -> bool {
        self.damaged
    }
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            hovered: false,
            focused: false,
            pressed: false,
            damaged: true, // Force initial drawing.
        }
    }
}

/// Common behavior shared by all widgets.
pub trait Widget: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn state(&self) -> &WidgetState;
    fn state_mut(&mut self) -> &mut WidgetState;

    #[inline]
    fn interaction(&self) -> Option<&InteractionStyle> {
        None
    }

    #[inline]
    fn interaction_mut(&mut self) -> Option<&mut InteractionStyle> {
        None
    }

    #[inline]
    fn rich_interaction(&self) -> Option<&RichInteractionStyle> {
        None
    }

    #[inline]
    fn rich_interaction_mut(&mut self) -> Option<&mut RichInteractionStyle> {
        None
    }

    // Rendering

    /// Draws the widget when no color-atlas access is needed.
    #[inline]
    fn draw(&mut self, _pane: &mut Pane, _rect: Rect) {}

    /// Draws the widget with mutable access to the color atlas when needed.
    #[inline]
    fn draw_with_colors(&mut self, pane: &mut Pane, rect: Rect, _colors: &mut ColorAtlas) {
        self.draw(pane, rect);
    }

    #[inline]
    fn cursor_pos(&self, _pane: &Pane, _rect: Rect) -> Option<Point> {
        None
    }

    // Actions

    #[inline]
    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::None
    }

    #[inline]
    fn key_action(&mut self, _key: KeyCode) -> WidgetAction {
        WidgetAction::None
    }

    #[inline]
    fn drag_action(&mut self, _local_x: usize, _width: usize) -> WidgetAction {
        WidgetAction::None
    }

    #[inline]
    fn release_action(&mut self, focused: bool) -> WidgetAction {
        if focused {
            WidgetAction::Clicked
        } else {
            WidgetAction::Released
        }
    }

    // Status

    #[inline]
    fn set_hovered(&mut self, value: bool) {
        self.state_mut().set_hovered(value);
    }

    #[inline]
    fn set_pressed(&mut self, value: bool) {
        self.state_mut().set_pressed(value);
    }

    #[inline]
    fn set_focused(&mut self, value: bool) {
        self.state_mut().set_focused(value);
    }

    #[inline]
    fn set_damaged(&mut self, value: bool) {
        self.state_mut().set_damaged(value);
    }

    #[inline]
    fn is_hovered(&self) -> bool {
        self.state().is_hovered()
    }

    #[inline]
    fn is_pressed(&self) -> bool {
        self.state().is_pressed()
    }

    #[inline]
    fn is_focused(&self) -> bool {
        self.state().is_focused()
    }

    // Helpers

    /// Builds styled glyphs for a single row of text.
    fn glyph_row(&self, text: &str, style: Style, width: usize) -> Vec<Glyph> {
        text.chars()
            .take(width)
            .map(|ch| Glyph::from(ch).with_style(style))
            .collect()
    }

    /// Writes one glyph row into the widget rectangle.
    fn write_glyph_row(&self, pane: &mut Pane, rect: Rect, row: usize, glyphs: &[Glyph]) {
        if row >= rect.height || glyphs.is_empty() {
            return;
        }

        pane.write_glyphs(Point::new(rect.x, rect.y + row), glyphs);
    }

    #[inline]
    fn clear_before_draw(&self) -> bool {
        true
    }

    #[inline]
    fn clear_style(&self) -> Style {
        self.interaction()
            .map(|interaction| interaction.style(self.state()))
            .unwrap_or_default()
    }

    #[inline]
    fn clear_style_with_colors(&self, colors: &mut ColorAtlas) -> Style {
        if let Some(interaction) = self.rich_interaction() {
            resolve_patched_style(colors, Style::default(), interaction.patch(self.state()))
        } else {
            self.clear_style()
        }
    }

    #[inline]
    fn clear_content(&self, pane: &mut Pane, rect: Rect, style: Style) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }

        pane.fill(rect, Glyph::from(' ').with_style(style));
    }
}

/// Fluent interaction-style builders for widgets that expose
/// `interaction_mut() -> Some(...)`.
pub trait StylableWidgetExt: Widget + Sized {
    #[must_use]
    fn with_interaction(mut self, interaction: InteractionStyle) -> Self {
        *self
            .interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)") =
            interaction;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_normal_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .normal = style;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_hover_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .hover = style;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_pressed_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .pressed = style;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_focus_interaction_style(mut self, style: Style) -> Self {
        self.interaction_mut()
            .expect("StylableWidgetExt requires interaction_mut() to return Some(..)")
            .focused = style;
        self.set_damaged(true);
        self
    }
}

/// Fluent interaction-patch builders for widgets that expose
/// `rich_interaction_mut() -> Some(...)`.
pub trait RichStylableWidgetExt: Widget + Sized {
    #[must_use]
    fn with_rich_interaction(mut self, interaction: RichInteractionStyle) -> Self {
        *self
            .rich_interaction_mut()
            .expect("RichStylableWidgetExt requires rich_interaction_mut() to return Some(..)") =
            interaction;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_normal_interaction_patch(mut self, patch: StylePatch) -> Self {
        self.rich_interaction_mut()
            .expect("RichStylableWidgetExt requires rich_interaction_mut() to return Some(..)")
            .normal = patch;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_hover_interaction_patch(mut self, patch: StylePatch) -> Self {
        self.rich_interaction_mut()
            .expect("RichStylableWidgetExt requires rich_interaction_mut() to return Some(..)")
            .hover = patch;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_pressed_interaction_patch(mut self, patch: StylePatch) -> Self {
        self.rich_interaction_mut()
            .expect("RichStylableWidgetExt requires rich_interaction_mut() to return Some(..)")
            .pressed = patch;
        self.set_damaged(true);
        self
    }

    #[must_use]
    fn with_focus_interaction_patch(mut self, patch: StylePatch) -> Self {
        self.rich_interaction_mut()
            .expect("RichStylableWidgetExt requires rich_interaction_mut() to return Some(..)")
            .focused = patch;
        self.set_damaged(true);
        self
    }
}

/// Draws a widget when it has pending damage.
pub(crate) fn widget_render(
    widget: &mut dyn Widget,
    pane: &mut Pane,
    rect: Rect,
    colors: &mut ColorAtlas,
) {
    if !widget.state().is_damaged() {
        return;
    }

    if rect.width == 0 || rect.height == 0 {
        widget.set_damaged(false);
        return;
    }

    if widget.clear_before_draw() {
        widget.clear_content(pane, rect, widget.clear_style_with_colors(colors));
    }

    widget.draw_with_colors(pane, rect, colors);
    widget.set_damaged(false);
}
