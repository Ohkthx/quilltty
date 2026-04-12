//! File: src/ui/store.rs

use std::collections::{HashMap, HashSet};

use crate::{
    Widget,
    surface::{
        Canvas, Layer, Pane, PaneId, Point, Rect,
        indexed_vec::{IndexedVec, Keyed},
    },
    ui::widget_render,
};

/// Unique identifier for a `Widget`.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub struct WidgetId(u32);

/// Hit information for a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetHit {
    /// Unique identifier for the widget.
    pub widget_id: WidgetId,
    /// Position local to the widget's rect.
    pub local: Point,
}

/// Layout for a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetLayout {
    Fixed(Rect), // Content-local rect; does not auto-resize with parent.
    Inset {
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
    },
    Line {
        left: usize,
        top: usize,
        right: usize,
    },
    Fill, // Reshapes as the parent changes, same as all 0 inset.
}

impl WidgetLayout {
    /// Resolves the widget layout against the pane's current content rect.
    #[inline]
    fn as_local(&self, content: &Rect) -> Rect {
        match *self {
            WidgetLayout::Fixed(rect) => rect,

            WidgetLayout::Fill => Rect {
                x: 0,
                y: 0,
                width: content.width,
                height: content.height,
            },

            WidgetLayout::Inset {
                left,
                top,
                right,
                bottom,
            } => Rect {
                x: left,
                y: top,
                width: content.width.saturating_sub(left + right),
                height: content.height.saturating_sub(top + bottom),
            },

            WidgetLayout::Line { left, top, right } => Rect {
                x: left,
                y: top,
                width: content.width.saturating_sub(left + right),
                height: usize::from(top < content.height),
            },
        }
    }
}

/// Stores one widget plus its visibility, layer, and layout metadata.
pub(crate) struct WidgetEntry {
    id: WidgetId,            // Unique identifier.
    layout: WidgetLayout,    // Bounds for the widget.
    z_layer: Layer,          // Layer the widget should be rendered at.
    visible: bool,           // If the widget can be seen.
    enabled: bool,           // If the widget is enabled.
    widget: Box<dyn Widget>, // Actual widget data.
}

impl Keyed<WidgetId> for WidgetEntry {
    fn key(&self) -> &WidgetId {
        &self.id
    }
}

/// Holds widgets for a single pane plus a pane-level damage marker.
pub(crate) struct PaneWidgets {
    full_damaged: bool,
    damaged_widgets: HashSet<WidgetId>,
    entries: IndexedVec<WidgetId, WidgetEntry>,
}

impl Default for PaneWidgets {
    fn default() -> Self {
        Self {
            full_damaged: false,
            damaged_widgets: HashSet::new(),
            entries: IndexedVec::new(),
        }
    }
}

/// Stores widgets by pane and tracks hover, pressed, and focused state.
#[derive(Default)]
pub struct WidgetStore {
    next_id: u32,
    by_pane: HashMap<PaneId, PaneWidgets>,
    index: HashMap<WidgetId, PaneId>,
    hovered: Option<WidgetId>,
    pressed: Option<WidgetId>,
    focused: Option<WidgetId>,
}

impl WidgetStore {
    /// Creates an empty widget store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a widget directly without routing through an internal builder.
    pub fn add_widget<W>(&mut self, pane_id: PaneId, widget: W, layout: WidgetLayout) -> WidgetId
    where
        W: Widget + 'static,
    {
        let widget_id = WidgetId(self.next_id);
        self.next_id += 1;

        let widget_entry = WidgetEntry {
            id: widget_id,
            layout,
            z_layer: Layer::default(),
            visible: true,
            enabled: true,
            widget: Box::new(widget),
        };

        let pane_widgets = self.by_pane.entry(pane_id).or_default();
        pane_widgets.full_damaged = true;
        pane_widgets.damaged_widgets.insert(widget_id);

        // Keeps entries ordered so hit-testing and rendering stay consistent.
        let idx = pane_widgets
            .entries
            .as_slice()
            .partition_point(|e| e.z_layer <= widget_entry.z_layer);

        pane_widgets.entries.insert_at(idx, widget_entry);
        self.index.insert(widget_id, pane_id);

        widget_id
    }

    /// Removes a widget from the store and invalidates its parent pane.
    pub fn remove_widget(&mut self, widget_id: WidgetId) -> bool {
        let Some(pane_id) = self.index.get(&widget_id).copied() else {
            return false;
        };

        let Some(pane_widgets) = self.by_pane.get_mut(&pane_id) else {
            self.index.remove(&widget_id);
            return false;
        };

        if pane_widgets.entries.remove(&widget_id).is_none() {
            self.index.remove(&widget_id);
            return false;
        }

        if self.hovered == Some(widget_id) {
            self.hovered = None;
        }

        if self.pressed == Some(widget_id) {
            self.pressed = None;
        }

        if self.focused == Some(widget_id) {
            self.focused = None;
        }

        pane_widgets.damaged_widgets.remove(&widget_id);
        pane_widgets.full_damaged = true;
        let should_remove_bucket = pane_widgets.entries.len() == 0;

        self.index.remove(&widget_id);

        if should_remove_bucket {
            self.by_pane.remove(&pane_id);
        }

        true
    }

    /// Removes all widgets owned by the pane and clears related widget state.
    pub fn remove_pane(&mut self, pane_id: PaneId) -> bool {
        let Some(pane_widgets) = self.by_pane.remove(&pane_id) else {
            return false;
        };

        for entry in pane_widgets.entries.iter() {
            self.index.remove(&entry.id);

            if self.hovered == Some(entry.id) {
                self.hovered = None;
            }

            if self.pressed == Some(entry.id) {
                self.pressed = None;
            }

            if self.focused == Some(entry.id) {
                self.focused = None;
            }
        }

        true
    }

    /// Returns true when any pane still has widget damage to flush.
    #[inline]
    pub fn has_damage(&self) -> bool {
        self.by_pane
            .values()
            .any(|pane| pane.full_damaged || !pane.damaged_widgets.is_empty())
    }

    /// Renders one widget entry using the cached pane content rect and returns the
    /// focused widget cursor position when this entry owns it.
    fn render_entry(
        entry: &mut WidgetEntry,
        pane: &mut Pane,
        content_rect: Rect,
        pane_is_focused: bool,
        focused_widget: Option<WidgetId>,
        force_damage: bool,
    ) -> Option<Point> {
        if !entry.visible || !entry.enabled {
            return None;
        }

        let rect = entry.layout.as_local(&content_rect);

        if force_damage {
            entry.widget.set_damaged(true); // Forced redraw for full pane damage.
        }

        widget_render(entry.widget.as_mut(), pane, rect);

        if pane_is_focused && focused_widget == Some(entry.id) {
            return entry.widget.cursor_pos(pane, rect);
        }

        None
    }

    /// Renders only dirty widgets unless the pane needs a full layout pass.
    pub fn render_into(&mut self, canvas: &mut Canvas) {
        let focused_widget = self.focused;

        for (&pane_id, widgets) in self.by_pane.iter_mut() {
            if !widgets.full_damaged && widgets.damaged_widgets.is_empty() {
                continue;
            }

            let Some(content_rect) = canvas.pane(pane_id).map(|p| p.content_rect()) else {
                continue;
            };

            let pane_is_focused = canvas.focused() == Some(pane_id);
            let full_damaged = widgets.full_damaged;
            let damaged_widgets = std::mem::take(&mut widgets.damaged_widgets);
            let mut focused_cursor = None;

            if let Some(pane) = canvas.pane_mut(pane_id) {
                // Preserve stable widget z-order during partial redraws by walking
                // the ordered entries and filtering by the dirty set.
                for entry in widgets.entries.iter_mut() {
                    if !full_damaged && !damaged_widgets.contains(&entry.id) {
                        continue;
                    }

                    if let Some(cursor) = Self::render_entry(
                        entry,
                        pane,
                        content_rect,
                        pane_is_focused,
                        focused_widget,
                        full_damaged,
                    ) {
                        focused_cursor = Some(cursor);
                    }
                }
            }

            if pane_is_focused {
                // Keep the canvas cursor synced to the focused widget. This also
                // clears stale cursors when the focused widget does not expose one.
                canvas.set_cursor(focused_cursor);
            }

            widgets.full_damaged = false;
        }
    }

    /// Assigns the current focused widget and updates focus flags.
    pub fn focus(&mut self, widget_id: Option<WidgetId>) {
        if self.focused == widget_id {
            return;
        }

        if let Some(old_id) = self.focused {
            let _ = self.edit(old_id, |w| w.set_focused(false));
        }

        self.focused = widget_id;

        if let Some(new_id) = self.focused {
            let _ = self.edit(new_id, |w| w.set_focused(true));
        }
    }

    /// Returns the currently focused widget.
    pub fn focused(&self) -> Option<WidgetId> {
        self.focused
    }

    /// Returns an immutable reference to a widget.
    pub fn get(&self, widget_id: WidgetId) -> Option<&dyn Widget> {
        let pane_id = self.index.get(&widget_id)?;
        let widgets = self.by_pane.get(pane_id)?;
        let entry = widgets.entries.get(&widget_id)?;
        Some(entry.widget.as_ref())
    }

    /// Returns an immutable reference to a widget of a specific concrete type.
    pub fn get_as<T>(&self, widget_id: WidgetId) -> Option<&T>
    where
        T: Widget + 'static,
    {
        let pane_id = self.index.get(&widget_id)?;
        let widgets = self.by_pane.get(pane_id)?;
        let entry = widgets.entries.get(&widget_id)?;
        entry.widget.as_any().downcast_ref::<T>()
    }

    /// Edits a widget and marks both the store entry and widget state as needing redraw.
    pub fn edit<R>(
        &mut self,
        widget_id: WidgetId,
        f: impl FnOnce(&mut dyn Widget) -> R,
    ) -> Option<R> {
        let pane_id = *self.index.get(&widget_id)?;
        let widgets = self.by_pane.get_mut(&pane_id)?;
        widgets.damaged_widgets.insert(widget_id);
        let entry = widgets.entries.get_mut(&widget_id)?;
        entry.widget.set_damaged(true);
        Some(f(entry.widget.as_mut()))
    }

    /// Hit-tests widgets using one cached pane content rect.
    pub fn widget_at(
        &self,
        canvas: &Canvas,
        pane_id: PaneId,
        content_local: Point,
    ) -> Option<WidgetHit> {
        let pane_widgets = self.by_pane.get(&pane_id)?;
        let pane = canvas.pane(pane_id)?;
        let content_rect = pane.content_rect();

        for entry in pane_widgets.entries.iter().rev() {
            if !entry.visible || !entry.enabled {
                continue;
            }

            let rect = entry.layout.as_local(&content_rect);
            if rect.contains_point(content_local) {
                return Some(WidgetHit {
                    widget_id: entry.id,
                    local: content_local.saturating_sub(rect.origin()),
                });
            }
        }

        None
    }

    /// Marks the widget under the cursor as pressed and focused.
    pub fn mouse_down(
        &mut self,
        canvas: &Canvas,
        pane_id: PaneId,
        content_local: Point,
    ) -> Option<WidgetHit> {
        let hit = self.widget_at(canvas, pane_id, content_local)?;

        if let Some(old_pressed) = self.pressed.replace(hit.widget_id) {
            let _ = self.edit(old_pressed, |w| w.set_pressed(false));
        }

        self.focus(Some(hit.widget_id));
        let _ = self.edit(hit.widget_id, |w| w.set_pressed(true));
        Some(hit)
    }

    /// Clears pressed state and preserves hover if release stays on the widget.
    pub fn mouse_up(
        &mut self,
        canvas: &Canvas,
        pane_id: PaneId,
        content_local: Point,
    ) -> Option<WidgetHit> {
        let hit = self.widget_at(canvas, pane_id, content_local);

        if let Some(pressed) = self.pressed.take() {
            let hovering_last = hit.as_ref().map(|h| h.widget_id) == Some(pressed);
            let _ = self.edit(pressed, |w| w.set_pressed(false));
            let _ = self.edit(pressed, |w| w.set_hovered(hovering_last));
        }

        hit
    }

    /// Updates hover state only when the hovered widget actually changes.
    pub fn hover(
        &mut self,
        canvas: &Canvas,
        pane_id: PaneId,
        content_local: Point,
    ) -> Option<WidgetHit> {
        let hit = self.widget_at(canvas, pane_id, content_local);
        let new_hovered = hit.as_ref().map(|h| h.widget_id);

        if self.hovered != new_hovered {
            if let Some(old) = self.hovered {
                let _ = self.edit(old, |w| w.set_hovered(false));
            }

            if let Some(new_id) = new_hovered {
                let _ = self.edit(new_id, |w| w.set_hovered(true));
            }

            self.hovered = new_hovered;
        }

        hit
    }

    /// Clears hover from the currently hovered widget, if any.
    pub fn clear_hover(&mut self) {
        if let Some(old) = self.hovered.take() {
            let _ = self.edit(old, |w| w.set_hovered(false));
        }
    }

    /// Returns the currently pressed widget, if any.
    pub fn pressed(&self) -> Option<WidgetId> {
        self.pressed
    }

    /// Clears the currently pressed widget even when release happens outside pane content.
    pub fn clear_pressed(&mut self) -> bool {
        let Some(pressed) = self.pressed.take() else {
            return false;
        };

        let _ = self.edit(pressed, |w| {
            w.set_pressed(false);
            w.set_hovered(false);
        });

        true
    }

    /// Resolves one widget rect using one cached pane content rect.
    pub fn widget_rect(&self, canvas: &Canvas, widget_id: WidgetId) -> Option<Rect> {
        let pane_id = *self.index.get(&widget_id)?;
        let widgets = self.by_pane.get(&pane_id)?;
        let entry = widgets.entries.get(&widget_id)?;
        let pane = canvas.pane(pane_id)?;
        let content_rect = pane.content_rect();
        Some(entry.layout.as_local(&content_rect))
    }

    /// Returns the parent pane for a widget.
    pub fn pane_id_of(&self, widget_id: WidgetId) -> Option<PaneId> {
        self.index.get(&widget_id).copied()
    }

    /// Marks every widget in a pane as needing redraw.
    pub fn invalidate_pane(&mut self, pane_id: PaneId) {
        if let Some(pane_widgets) = self.by_pane.get_mut(&pane_id) {
            pane_widgets.full_damaged = true;
        };
    }

    /// Marks one widget as needing redraw.
    pub fn invalidate_widget(&mut self, widget_id: WidgetId) {
        let Some(pane_id) = self.index.get(&widget_id).copied() else {
            return;
        };

        let Some(pane_widgets) = self.by_pane.get_mut(&pane_id) else {
            return;
        };

        pane_widgets.damaged_widgets.insert(widget_id);

        if let Some(entry) = pane_widgets.entries.get_mut(&widget_id) {
            entry.widget.set_damaged(true);
        }
    }

    /// Marks every widget in every pane as needing redraw.
    pub fn invalidate_all(&mut self) {
        for pane_widgets in self.by_pane.values_mut() {
            pane_widgets.full_damaged = true;
        }
    }
}
