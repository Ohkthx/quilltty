//! File: src/ui/store.rs

use std::collections::HashMap;

use crate::{
    Canvas, PaneId,
    geom::{Point, Rect},
    surface::indexed_vec::{IndexedVec, Keyed},
    ui::Widget,
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
pub enum WidgetLayout {
    Fixed(Rect), // Content-local rect; does not auto-resize with parent.
    Inset {
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
    },
    Fill, // Reshapes as the parent changes, same as all 0 inset.
}

impl WidgetLayout {
    /// Converts into a local rect.
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
        }
    }
}

/// An entry representing a widget instance, including geometry and render state.
pub(crate) struct WidgetEntry {
    id: WidgetId,         // Unique identifier.
    layout: WidgetLayout, // Bounds for the widget.
    z_layer: i16,         // Layer the widget should be rendered at.
    visible: bool,        // If the widget can be seen.
    enabled: bool,        // If the widget is enabled.
    widget: Widget,       // Actual widget data.
}

impl Keyed<WidgetId> for WidgetEntry {
    fn key(&self) -> &WidgetId {
        &self.id
    }
}

/// Holds the widgets for a specific `PaneId`.
pub(crate) struct PaneWidgets {
    damaged: bool,                              // Marker for changes.
    entries: IndexedVec<WidgetId, WidgetEntry>, // Widgets contained.
}

impl Default for PaneWidgets {
    fn default() -> Self {
        Self {
            damaged: false,
            entries: IndexedVec::new(),
        }
    }
}

/// Builds widgets and requires `.build()` to finalize insertion.
pub struct WidgetBuilder<'a> {
    store: &'a mut WidgetStore,   // Reference to the widget store.
    pane_id: PaneId,              // Identifier for the parent `Pane`.
    widget: Option<Widget>,       // Widget to insert.
    layout: Option<WidgetLayout>, // Pane content-local positioning.
}

impl<'a> WidgetBuilder<'a> {
    /// Creates a new builder for a `WidgetEntry`.
    fn new(pane_id: PaneId, store: &'a mut WidgetStore) -> Self {
        Self {
            store,
            pane_id,
            widget: None,
            layout: None,
        }
    }

    /// Assigns the widget to the builder.
    #[must_use]
    pub fn with_widget(mut self, widget: impl Into<Widget>) -> Self {
        self.widget = Some(widget.into());
        self
    }

    /// Positioning must be local to the Panes content area.
    #[must_use]
    pub fn with_layout(mut self, layout: WidgetLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Creates a new widget, assigning its parent as `PaneId`.
    #[must_use]
    pub fn build(self) -> WidgetId {
        let widget = self.widget.expect("Widget is required for WidgetBuilder.");
        let layout = self.layout.expect("Layout is required for WidgetBuilder.");

        let store = self.store;

        let pane_id = self.pane_id;
        let widget_id = WidgetId(store.next_id);
        store.next_id += 1;

        let widget_entry = WidgetEntry {
            id: widget_id,
            layout,
            z_layer: 1,
            visible: true,
            enabled: true,
            widget,
        };

        let pane_widgets = store.by_pane.entry(pane_id).or_default();
        pane_widgets.damaged = true;

        let z_layer = widget_entry.z_layer;
        let idx = pane_widgets
            .entries
            .as_slice()
            .partition_point(|e| e.z_layer <= z_layer);

        pane_widgets.entries.insert_at(idx, widget_entry);
        store.index.insert(widget_id, pane_id);

        widget_id
    }
}

/// Stores widgets for panes and orchestrates rendering, focus, and interactions.
#[derive(Default)]
pub struct WidgetStore {
    next_id: u32,                          // Next `WidgetId` to assign.
    by_pane: HashMap<PaneId, PaneWidgets>, // PaneId => widgets.
    index: HashMap<WidgetId, PaneId>,      // Ownership lookup.
    hovered: Option<WidgetId>,             // Currently hovered widget.
    pressed: Option<WidgetId>,             // Currently pressed widget.
    focused: Option<WidgetId>,             // Currently focused widget.
}

impl WidgetStore {
    /// Creates new storage for widgets mapped to parent panes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a builder to create a new widget entry.
    pub fn widget(&mut self, pane_id: PaneId) -> WidgetBuilder<'_> {
        WidgetBuilder::new(pane_id, self)
    }

    /// Backwards-compatible alias for `widget`.
    pub fn new_widget(&mut self, pane_id: PaneId) -> WidgetBuilder<'_> {
        self.widget(pane_id)
    }

    /// Renders all visible and enabled widgets into their parent panes.
    pub fn render_into(&mut self, canvas: &mut Canvas) {
        for (&pane_id, widgets) in self.by_pane.iter_mut() {
            if !widgets.damaged {
                continue;
            }

            let pane_is_focused = canvas.focused() == pane_id;
            let focused_widget = self.focused;
            let mut pane_cursor = None;

            for entry in widgets.entries.iter_mut() {
                if !entry.visible || !entry.enabled {
                    continue;
                }

                if let Some(pane) = canvas.pane_mut(pane_id) {
                    let rect = entry.layout.as_local(&pane.content_rect());
                    entry.widget.render(pane, rect);
                }

                if pane_is_focused
                    && Some(entry.id) == focused_widget
                    && let Some(pane) = canvas.pane(pane_id)
                {
                    let rect = entry.layout.as_local(&pane.content_rect());
                    pane_cursor = entry.widget.cursor_pos(pane, rect);
                }
            }

            if pane_is_focused {
                canvas.set_cursor(pane_cursor);
            }

            widgets.damaged = false;
        }
    }

    /// Assigns the current focused widget.
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

    /// Returns the currently focused `WidgetId`.
    pub fn focused(&self) -> Option<WidgetId> {
        self.focused
    }

    /// Returns an immutable reference to the widget.
    pub fn get(&self, widget_id: WidgetId) -> Option<&Widget> {
        let pane_id = self.index.get(&widget_id)?;
        let widgets = self.by_pane.get(pane_id)?;
        let entry = widgets.entries.get(&widget_id)?;
        Some(&entry.widget)
    }

    /// Edits a widget by its `WidgetId`.
    pub fn edit<R>(&mut self, widget_id: WidgetId, f: impl FnOnce(&mut Widget) -> R) -> Option<R> {
        let pane_id = *self.index.get(&widget_id)?;
        let widgets = self.by_pane.get_mut(&pane_id)?;
        widgets.damaged = true;

        let entry = widgets.entries.get_mut(&widget_id)?;
        Some(f(&mut entry.widget))
    }

    /// Returns the widget at a content-local position.
    pub fn widget_at(
        &self,
        canvas: &Canvas,
        pane_id: PaneId,
        content_local: Point,
    ) -> Option<WidgetHit> {
        let pane_widgets = self.by_pane.get(&pane_id)?;
        let pane = canvas.pane(pane_id)?;

        for entry in pane_widgets.entries.iter().rev() {
            if !entry.visible || !entry.enabled {
                continue;
            }

            let rect = entry.layout.as_local(&pane.content_rect());
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

    /// Clears the pressed state and updates hover for the previously pressed widget.
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

    /// Updates hover state and returns the widget hit, if any.
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

    /// Clears the hover flag from the currently hovered widget.
    pub fn clear_hover(&mut self) {
        if let Some(old) = self.hovered.take() {
            let _ = self.edit(old, |w| w.set_hovered(false));
        }
    }

    /// Lookup the parent `PaneId` for the widget.
    pub fn pane_id_of(&self, widget_id: WidgetId) -> Option<PaneId> {
        self.index.get(&widget_id).copied()
    }

    /// Marks all widgets for the `Pane` as damaged to force a redraw.
    pub fn invalidate_pane(&mut self, pane_id: PaneId) {
        let Some(pane_widgets) = self.by_pane.get_mut(&pane_id) else {
            return;
        };

        pane_widgets.damaged = true;
        for entry in pane_widgets.entries.iter_mut() {
            entry.widget.set_damaged(true);
        }
    }

    /// Invalids a single widget forcing a redraw.
    pub fn invalidate_widget(&mut self, widget_id: WidgetId) {
        let Some(pane_id) = self.index.get(&widget_id).copied() else {
            return;
        };

        let Some(pane_widgets) = self.by_pane.get_mut(&pane_id) else {
            return;
        };

        pane_widgets.damaged = true;
        if let Some(entry) = pane_widgets.entries.get_mut(&widget_id) {
            entry.widget.set_damaged(true);
        }
    }

    /// Invalids all widgets for all panes, forcing a full redraw.
    pub fn invalidate_all(&mut self) {
        for pane_widgets in self.by_pane.values_mut() {
            pane_widgets.damaged = true;

            for entry in pane_widgets.entries.iter_mut() {
                entry.widget.set_damaged(true);
            }
        }
    }
}
