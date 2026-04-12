//! File: src/ui/runtime.rs

use std::{
    any::Any,
    io::{self, Write},
    time::Duration,
};

use crate::{
    InputWidget, Widget, WidgetAction, WidgetHit, WidgetId, WidgetLayout, WidgetStore,
    crossterm::event::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    surface::{
        Canvas, Compositor, Glyph, HitTarget, Pane, PaneAction, PaneBuilder, PaneElement, PaneHit,
        PaneId, Point, Rect, Renderer, Size,
    },
};

/// Describes the kind of pane drag currently in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDragKind {
    Move,   // Pane is being repositioned.
    Resize, // Pane is being resized from the bottom-right.
}

/// Stores any active pointer drag captured by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerDrag {
    PaneMove {
        pane_id: PaneId,    // Pane currently being moved.
        grab_offset: Point, // Pointer offset captured on press.
    },
    PaneResize {
        pane_id: PaneId, // Pane currently being resized.
    },
    Content {
        pane_id: PaneId,     // Pane whose content owns the drag.
        button: MouseButton, // Mouse button that initiated the drag.
        anchor: Point,       // Original screen-space drag anchor.
        previous: Point,     // Previous screen-space pointer position.
        current: Point,      // Latest screen-space pointer position.
    },
}

/// High-level UI events emitted back to application code.
#[derive(Debug)]
pub enum UiEvent {
    None, // No UI action occurred.

    // Generic Pane events.
    PanePressed {
        pane_id: PaneId,
        hit: PaneHit,
    }, // Mouse pressed pane content with no widget hit.
    PaneDragStart {
        pane_id: PaneId,
        kind: PaneDragKind,
    }, // Pane drag has started.
    PaneDragged {
        pane_id: PaneId,
        kind: PaneDragKind,
    }, // Pane is actively being dragged.
    PaneReleased {
        pane_id: PaneId,
        kind: PaneDragKind,
    }, // Pane drag has ended.

    // Pane content drag events.
    PaneContentDragStart {
        pane_id: PaneId, // Pane whose content drag has started.
        anchor: Point,   // Original screen-space drag anchor.
        current: Point,  // Current screen-space pointer position.
    },
    PaneContentDragged {
        pane_id: PaneId, // Pane whose content is being dragged.
        anchor: Point,   // Original screen-space drag anchor.
        previous: Point, // Previous screen-space pointer position.
        current: Point,  // Current screen-space pointer position.
    },
    PaneContentHeld {
        pane_id: PaneId, // Pane whose content drag is still active.
        anchor: Point,   // Original screen-space drag anchor.
        current: Point,  // Current screen-space pointer position.
        dt: Duration,    // Tick duration since the last held update.
    },
    PaneContentDragEnd {
        pane_id: PaneId, // Pane whose content drag has ended.
        anchor: Point,   // Original screen-space drag anchor.
        current: Point,  // Final screen-space pointer position.
    },

    // Generic Widget interactions.
    WidgetHovered(WidgetHit),  // Pointer moved over a widget.
    WidgetPressed(WidgetHit),  // Widget received mouse press.
    WidgetReleased(WidgetHit), // Widget received mouse release without activation.
    WidgetClicked(WidgetHit),  // Widget was activated by click.

    // Widget-specific events.
    SliderChanged {
        widget_id: WidgetId,
        value: f64,
    }, // Slider value changed.
    CheckboxChanged {
        widget_id: WidgetId,
        checked: bool,
    }, // Checkbox toggled state.
    InputChanged {
        widget_id: WidgetId,
    }, // Input widget text changed.
    InputSubmitted {
        widget_id: WidgetId,
        value: String,
    }, // Input widget was submitted.

    // Custom widgets.
    WidgetCustom {
        widget_id: WidgetId,
        payload: Box<dyn Any + Send>,
    },
}

/// Coordinates pane rendering, widget state, and pointer-driven pane actions.
pub struct Ui {
    canvas: Canvas,            // Backing surface containing panes.
    compositor: Compositor,    // Flattens pane damage into a renderable frame.
    renderer: Renderer,        // Writes frame differences to the terminal.
    widgets: WidgetStore,      // Tracks widgets, focus, hover, and pressed state.
    drag: Option<PointerDrag>, // Active pointer drag session, if any.
}

impl Ui {
    // =========================================================================
    // Construction, Destruction, Rendering, and Building
    // =========================================================================

    /// Creates a new UI with the given size and optional background glyph.
    pub fn new(width: usize, height: usize, bg: Option<Glyph>) -> Self {
        Self {
            canvas: Canvas::new(Size { width, height }, bg),
            compositor: Compositor::new(width, height),
            renderer: Renderer::new(width, height, true),
            widgets: WidgetStore::new(),
            drag: None,
        }
    }

    /// Renders widgets into panes, then flushes the composed frame to `out`.
    pub fn render_to<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        self.widgets.render_into(&mut self.canvas);
        self.canvas
            .render(&mut self.compositor, &mut self.renderer, out)?;

        out.flush()?;
        Ok(())
    }

    /// Returns a builder for creating a new pane on the canvas.
    pub fn add_pane(&mut self) -> PaneBuilder<'_> {
        self.canvas.create_pane()
    }

    /// Removes a pane and all widget state associated with it.
    pub fn remove_pane(&mut self, pane_id: PaneId) -> bool {
        if self.drag_targets_pane(pane_id) {
            self.drag = None;
        }

        self.widgets.remove_pane(pane_id);
        self.canvas.remove_pane(pane_id)
    }

    /// Creates a widget inside the given pane.
    pub fn add_widget<W>(&mut self, pane_id: PaneId, widget: W, layout: WidgetLayout) -> WidgetId
    where
        W: Widget + 'static,
    {
        self.widgets.add_widget(pane_id, widget, layout)
    }

    /// Removes a widget from the UI at runtime.
    pub fn remove_widget(&mut self, widget_id: WidgetId) -> bool {
        let pane_id = self.widgets.pane_id_of(widget_id);
        let was_focused = self.widgets.focused() == Some(widget_id);

        let removed = self.widgets.remove_widget(widget_id);
        if !removed {
            return false;
        }

        if was_focused && self.canvas.focused() == pane_id {
            self.canvas.set_cursor(None);
        }

        true
    }
}

impl Ui {
    // =========================================================================
    // Input Handling
    // =========================================================================

    /// Dispatches a raw Crossterm event into the appropriate UI handler.
    pub fn handle_event(&mut self, event: Event) -> UiEvent {
        match event {
            Event::Key(key) => self.key(key),
            Event::Mouse(mouse) => self.mouse(mouse),
            _ => UiEvent::None,
        }
    }

    /// Advances tick-aware UI behavior such as held pane-content dragging.
    pub fn tick(&mut self, dt: Duration) -> UiEvent {
        match self.drag {
            Some(PointerDrag::Content {
                pane_id,
                anchor,
                current,
                ..
            }) => UiEvent::PaneContentHeld {
                pane_id,
                anchor,
                current,
                dt,
            },

            _ => UiEvent::None,
        }
    }

    /// Dispatches a raw mouse event into the appropriate UI handler.
    pub fn mouse(&mut self, mouse: MouseEvent) -> UiEvent {
        let pos: Point = (mouse.column, mouse.row).into();

        match mouse.kind {
            MouseEventKind::Moved if matches!(self.drag, Some(PointerDrag::Content { .. })) => {
                self.mouse_drag(pos)
            }

            MouseEventKind::Moved => self.mouse_move(pos),
            MouseEventKind::Down(MouseButton::Left) => self.mouse_down(pos),
            MouseEventKind::Drag(MouseButton::Left) => self.mouse_drag(pos),
            MouseEventKind::Up(MouseButton::Left) => self.mouse_up(pos),
            _ => UiEvent::None,
        }
    }

    /// Updates widget hover state when the pointer moves across pane content.
    pub fn mouse_move(&mut self, pos: Point) -> UiEvent {
        if self.drag.is_some() {
            return UiEvent::None;
        }

        match self.canvas.hit_at(pos) {
            HitTarget::Pane { pane_id, hit }
                if hit.element == PaneElement::Content && hit.content_local.is_some() =>
            {
                let content_local = hit.content_local.unwrap();

                self.widgets
                    .hover(&self.canvas, pane_id, content_local)
                    .map(UiEvent::WidgetHovered)
                    .unwrap_or(UiEvent::None)
            }

            _ => {
                self.widgets.clear_hover();
                UiEvent::None
            }
        }
    }

    /// Handles mouse press for panes, widgets, and drag start behavior.
    pub fn mouse_down(&mut self, pos: Point) -> UiEvent {
        match self.canvas.hit_at(pos) {
            HitTarget::Background { global: _ } => {
                self.clear_focus();
                self.clear_hover();
                self.drag = None;
                UiEvent::None
            }

            HitTarget::Pane { pane_id, hit } => {
                self.focus_pane(pane_id);

                match hit.element {
                    PaneElement::Content => {
                        let Some(content_local) = hit.content_local else {
                            self.focus_widget(None);
                            self.clear_hover();
                            return UiEvent::PanePressed { pane_id, hit };
                        };

                        let Some(widget_hit) =
                            self.widgets
                                .mouse_down(&self.canvas, pane_id, content_local)
                        else {
                            self.focus_widget(None);
                            self.clear_hover();
                            return UiEvent::PanePressed { pane_id, hit };
                        };

                        // Preserve slider behavior: clicking a slider immediately updates its value.
                        let Some(rect) =
                            self.widgets.widget_rect(&self.canvas, widget_hit.widget_id)
                        else {
                            return UiEvent::WidgetPressed(widget_hit);
                        };

                        let action = self
                            .widgets
                            .edit(widget_hit.widget_id, |w| {
                                w.drag_action(widget_hit.local.x, rect.width)
                            })
                            .unwrap_or(WidgetAction::None);

                        match action {
                            WidgetAction::None => UiEvent::WidgetPressed(widget_hit),
                            action => {
                                map_widget_action(widget_hit.widget_id, Some(widget_hit), action)
                            }
                        }
                    }

                    _ => match self.canvas.action_for_hit(pane_id, hit.element, hit.local) {
                        PaneAction::BeginMove { grab_offset } => {
                            self.begin_pointer_drag(PointerDrag::PaneMove {
                                pane_id,
                                grab_offset,
                            })
                        }

                        PaneAction::BeginResize => {
                            self.begin_pointer_drag(PointerDrag::PaneResize { pane_id })
                        }

                        PaneAction::FocusOnly | PaneAction::None => {
                            self.focus_widget(None);
                            self.clear_hover();
                            UiEvent::PanePressed { pane_id, hit }
                        }
                    },
                }
            }
        }
    }

    /// Applies an in-progress pane move, pane resize, or pane-content drag.
    pub fn mouse_drag(&mut self, pos: Point) -> UiEvent {
        if let Some(pointer_drag) = self.drag {
            match pointer_drag {
                PointerDrag::PaneMove {
                    pane_id,
                    grab_offset,
                } => {
                    let new_origin = pos.saturating_sub(grab_offset);
                    self.move_pane(pane_id, new_origin, true);

                    return UiEvent::PaneDragged {
                        pane_id,
                        kind: PaneDragKind::Move,
                    };
                }

                PointerDrag::PaneResize { pane_id } => {
                    if let Some(rect) = self.canvas.pane(pane_id).map(|p| p.rect()) {
                        let width = pos.x.saturating_sub(rect.x).saturating_add(1);
                        let height = pos.y.saturating_sub(rect.y).saturating_add(1);
                        self.resize_pane(pane_id, width, height);
                    }

                    return UiEvent::PaneDragged {
                        pane_id,
                        kind: PaneDragKind::Resize,
                    };
                }

                PointerDrag::Content {
                    pane_id,
                    button,
                    anchor,
                    previous: _,
                    current,
                } => {
                    self.drag = Some(PointerDrag::Content {
                        pane_id,
                        button,
                        anchor,
                        previous: current,
                        current: pos,
                    });

                    return UiEvent::PaneContentDragged {
                        pane_id,
                        anchor,
                        previous: current,
                        current: pos,
                    };
                }
            }
        }

        let Some(hit) = self.pressed_widget_hit_at(pos) else {
            return UiEvent::None;
        };

        let Some(rect) = self.widgets.widget_rect(&self.canvas, hit.widget_id) else {
            return UiEvent::None;
        };

        let action = self
            .widgets
            .edit(hit.widget_id, |w| w.drag_action(hit.local.x, rect.width))
            .unwrap_or(WidgetAction::None);

        map_widget_action(hit.widget_id, Some(hit), action)
    }

    /// Handles mouse release for dragging completion and widget activation.
    pub fn mouse_up(&mut self, pos: Point) -> UiEvent {
        if let Some(pointer_drag) = self.drag.take() {
            self.sync_hover(pos);

            return match pointer_drag {
                PointerDrag::PaneMove { pane_id, .. } => UiEvent::PaneReleased {
                    pane_id,
                    kind: PaneDragKind::Move,
                },

                PointerDrag::PaneResize { pane_id } => UiEvent::PaneReleased {
                    pane_id,
                    kind: PaneDragKind::Resize,
                },

                PointerDrag::Content {
                    pane_id, anchor, ..
                } => UiEvent::PaneContentDragEnd {
                    pane_id,
                    anchor,
                    current: pos,
                },
            };
        }

        let Some(widget_hit) = self.mouse_up_widget_hit(pos) else {
            let _ = self.widgets.clear_pressed();
            self.widgets.clear_hover();
            return UiEvent::None;
        };

        self.release_widget_event(widget_hit)
    }

    /// Handles keyboard interaction for the currently focused widget.
    pub fn key(&mut self, key: KeyEvent) -> UiEvent {
        let Some(widget_id) = self.widgets.focused() else {
            return UiEvent::None;
        };

        // Prevents accidental activation / submission for InputWidget.
        let is_input = self
            .widget(widget_id)
            .map(|w| w.as_any().is::<InputWidget>())
            .unwrap_or(false);

        let action = self
            .widgets
            .edit(widget_id, |w| match key.code {
                KeyCode::Enter => w.activate_action(),
                KeyCode::Char(' ') if !is_input => w.activate_action(),
                other => w.key_action(other),
            })
            .unwrap_or(WidgetAction::None);

        map_widget_action(widget_id, None, action)
    }
}

impl Ui {
    // =========================================================================
    // Focus and Hover State
    // =========================================================================

    /// Focuses a pane and refreshes widget state for the old and new pane.
    pub fn focus_pane(&mut self, pane_id: PaneId) {
        let old_pane = self.canvas.focused();
        if old_pane == Some(pane_id) {
            return;
        }

        self.canvas.focus(Some(pane_id));

        if let Some(old_pane) = old_pane {
            self.widgets.invalidate_pane(old_pane);
        }

        self.widgets.invalidate_pane(pane_id);
    }

    /// Focuses a widget and updates pane focus and cursor visibility.
    pub fn focus_widget(&mut self, widget_id: Option<WidgetId>) {
        let old_widget = self.widgets.focused();
        let old_pane = old_widget.and_then(|id| self.widgets.pane_id_of(id));
        let new_pane = widget_id.and_then(|id| self.widgets.pane_id_of(id));

        self.widgets.focus(widget_id);

        match new_pane {
            Some(pane_id) => self.canvas.focus(Some(pane_id)),
            None => {
                self.canvas.focus(None);
                self.canvas.set_cursor(None);
            }
        }

        if let Some(pane_id) = old_pane {
            self.widgets.invalidate_pane(pane_id);
        }

        if let Some(pane_id) = new_pane {
            self.widgets.invalidate_pane(pane_id);
        }
    }

    /// Returns the currently focused pane id.
    pub fn focused_pane(&self) -> Option<PaneId> {
        self.canvas.focused()
    }

    /// Returns the currently focused widget id, if any.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.widgets.focused()
    }

    /// Clears the current hovered widget, if any.
    pub fn clear_hover(&mut self) {
        self.widgets.clear_hover();
    }

    /// Clears widget focus and pane focus.
    pub fn clear_focus(&mut self) {
        let old_pane = self
            .widgets
            .focused()
            .and_then(|widget_id| self.widgets.pane_id_of(widget_id))
            .or(self.canvas.focused());

        self.widgets.focus(None);
        self.canvas.focus(None);
        self.canvas.set_cursor(None);

        if let Some(pane_id) = old_pane {
            self.widgets.invalidate_pane(pane_id);
        }
    }
}

impl Ui {
    // =========================================================================
    // Pane Management
    // =========================================================================

    /// Moves a pane and refreshes cursor state if a focused widget lives inside it.
    pub fn move_pane(&mut self, pane_id: PaneId, origin: Point, clamp: bool) {
        self.canvas.move_pane(pane_id, origin, clamp);

        // Moving a pane changes the screen-space cursor location for any focused widget
        // inside it, so invalidate that pane to force cursor recomputation.
        if self.canvas.focused() == Some(pane_id) && self.focused_widget_on_pane(pane_id) {
            self.widgets.invalidate_pane(pane_id);
        }
    }

    /// Resizes a pane and invalidates its widgets so they redraw against the new bounds.
    pub fn resize_pane(&mut self, pane_id: PaneId, width: usize, height: usize) -> bool {
        let changed = self.canvas.resize_pane(pane_id, width, height);
        if changed {
            self.widgets.invalidate_pane(pane_id);
        }
        changed
    }

    /// Toggles pane visibility while cleaning up focus and hover state.
    pub fn toggle_pane_visibility(&mut self, pane_id: PaneId) {
        let will_hide = self
            .canvas
            .pane(pane_id)
            .map(|pane| pane.visible)
            .unwrap_or(false);

        if will_hide {
            self.cleanup_hidden_pane_state(pane_id);
        }

        self.canvas.toggle_pane_visibility(pane_id);
        self.widgets.invalidate_pane(pane_id);
    }

    /// Marks a pane as visible and invalidates its widgets so they redraw.
    pub fn show_pane(&mut self, pane_id: PaneId) -> bool {
        let changed = self.canvas.show_pane(pane_id);
        if changed {
            self.widgets.invalidate_pane(pane_id);
        }
        changed
    }

    /// Marks a pane as hidden while cleaning up focus and hover state.
    pub fn hide_pane(&mut self, pane_id: PaneId) -> bool {
        let changed = self.canvas.hide_pane(pane_id);
        if !changed {
            return false;
        }

        self.cleanup_hidden_pane_state(pane_id);
        self.widgets.invalidate_pane(pane_id);
        true
    }

    /// Sets the title text for a pane.
    pub fn set_pane_title<S: Into<String>>(&mut self, pane_id: PaneId, title: Option<S>) {
        self.canvas.set_pane_title(pane_id, title.map(Into::into));
    }

    /// Returns an iterator over all current pane ids.
    pub fn pane_ids(&self) -> impl Iterator<Item = PaneId> + '_ {
        self.canvas.pane_ids()
    }

    /// Returns the pane if it exists.
    pub fn pane(&self, pane_id: PaneId) -> Option<&Pane> {
        self.canvas.pane(pane_id)
    }

    /// Returns the current rect for a pane if it exists.
    pub fn pane_rect(&self, pane_id: PaneId) -> Option<Rect> {
        self.canvas.pane(pane_id).map(|p| p.rect())
    }
}

impl Ui {
    // =========================================================================
    // Widget Queries and Editing
    // =========================================================================

    /// Returns an immutable reference to a widget by id.
    pub fn widget(&self, widget_id: WidgetId) -> Option<&dyn Widget> {
        self.widgets.get(widget_id)
    }

    /// Returns an immutable reference to a widget by id when it matches `T`.
    pub fn widget_as<T>(&self, widget_id: WidgetId) -> Option<&T>
    where
        T: Widget + 'static,
    {
        self.widgets.get_as::<T>(widget_id)
    }

    /// Returns the current content-local rect for a widget.
    pub fn widget_rect(&self, widget_id: WidgetId) -> Option<Rect> {
        self.widgets.widget_rect(&self.canvas, widget_id)
    }

    /// Returns the parent pane id for a widget.
    pub fn widget_pane(&self, widget_id: WidgetId) -> Option<PaneId> {
        self.widgets.pane_id_of(widget_id)
    }

    /// Edits a widget by id and returns the callback result when the widget exists.
    pub fn edit_widget<R>(
        &mut self,
        widget_id: WidgetId,
        f: impl FnOnce(&mut dyn Widget) -> R,
    ) -> Option<R> {
        self.widgets.edit(widget_id, f)
    }

    /// Edits a widget by id when it matches `T` and returns the callback result.
    pub fn edit_widget_as<T: 'static, R>(
        &mut self,
        widget_id: WidgetId,
        f: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        self.edit_widget(widget_id, |w| w.as_any_mut().downcast_mut::<T>().map(f))
            .flatten()
    }
}

impl Ui {
    // =========================================================================
    // Invalidation
    // =========================================================================

    /// Marks every widget in a pane as needing redraw.
    pub fn invalidate_pane(&mut self, pane_id: PaneId) {
        self.widgets.invalidate_pane(pane_id);
    }

    /// Marks a single widget as needing redraw.
    pub fn invalidate_widget(&mut self, widget_id: WidgetId) {
        self.widgets.invalidate_widget(widget_id);
    }

    /// Marks every widget as needing redraw.
    pub fn invalidate_all(&mut self) {
        self.widgets.invalidate_all();
    }
}

impl Ui {
    // =========================================================================
    // Direct Access
    // =========================================================================

    /// Borrows the backing canvas immutably for advanced read-only operations.
    pub fn with_canvas<R>(&self, f: impl FnOnce(&Canvas) -> R) -> R {
        f(&self.canvas)
    }

    /// Borrows the backing canvas mutably for advanced low-level operations.
    pub fn with_canvas_mut<R>(&mut self, f: impl FnOnce(&mut Canvas) -> R) -> R {
        f(&mut self.canvas)
    }

    /// Borrows a specific pane immutably when it exists for advanced read-only operations.
    pub fn with_pane<R>(&self, pane_id: PaneId, f: impl FnOnce(&Pane) -> R) -> Option<R> {
        self.canvas.pane(pane_id).map(f)
    }

    /// Borrows a specific pane mutably when it exists.
    pub fn with_pane_mut<R>(
        &mut self,
        pane_id: PaneId,
        f: impl FnOnce(&mut Pane) -> R,
    ) -> Option<R> {
        self.canvas.pane_mut(pane_id).map(f)
    }

    /// Borrows the widget store mutably for advanced widget manipulation.
    pub fn with_widgets_mut<R>(&mut self, f: impl FnOnce(&mut WidgetStore) -> R) -> R {
        f(&mut self.widgets)
    }

    /// Borrows the widget store immutably for advanced widget inspection.
    pub fn with_widgets<R>(&self, f: impl FnOnce(&WidgetStore) -> R) -> R {
        f(&self.widgets)
    }
}

impl Ui {
    // =========================================================================
    // Pointer Drag Control
    // =========================================================================

    /// Begins a captured content drag when the anchor is over the given pane's content.
    pub fn begin_content_drag(
        &mut self,
        pane_id: PaneId,
        anchor: Point,
        button: MouseButton,
    ) -> UiEvent {
        let Some((hit_pane_id, _, _)) = self.content_hit_at(anchor) else {
            return UiEvent::None;
        };

        if hit_pane_id != pane_id {
            return UiEvent::None;
        }

        self.begin_pointer_drag(PointerDrag::Content {
            pane_id,
            button,
            anchor,
            previous: anchor,
            current: anchor,
        })
    }

    /// Returns the active pointer drag, if any.
    pub fn active_pointer_drag(&self) -> Option<PointerDrag> {
        self.drag
    }

    /// Cancels any active pointer drag session.
    pub fn cancel_pointer_drag(&mut self) {
        self.drag = None;
    }
}

impl Ui {
    // =========================================================================
    // Internal Helpers
    // =========================================================================
    //
    /// Returns true when the active pointer drag belongs to the pane.
    fn drag_targets_pane(&self, pane_id: PaneId) -> bool {
        matches!(
            self.drag,
            Some(PointerDrag::PaneMove { pane_id: id, .. })
                | Some(PointerDrag::PaneResize { pane_id: id, .. })
                | Some(PointerDrag::Content { pane_id: id, .. })
            if id == pane_id
        )
    }

    /// Recomputes hover state after drag completion.
    fn sync_hover(&mut self, pos: Point) {
        match self.canvas.hit_at(pos) {
            HitTarget::Pane { pane_id, hit }
                if hit.element == PaneElement::Content && hit.content_local.is_some() =>
            {
                let content_local = hit.content_local.unwrap();
                let _ = self.widgets.hover(&self.canvas, pane_id, content_local);
            }

            _ => self.widgets.clear_hover(),
        }
    }

    /// Clears focus and hover state associated with a pane being hidden.
    #[inline]
    fn cleanup_hidden_pane_state(&mut self, pane_id: PaneId) {
        if self.focused_widget_on_pane(pane_id) {
            self.widgets.focus(None);
            self.canvas.set_cursor(None);
        }

        if self.canvas.focused() == Some(pane_id) {
            self.canvas.focus(None);
        }

        self.widgets.clear_hover();
    }

    /// Returns pane/content hit data when the position is over pane content.
    #[inline]
    fn content_hit_at(&self, pos: Point) -> Option<(PaneId, PaneHit, Point)> {
        match self.canvas.hit_at(pos) {
            HitTarget::Pane { pane_id, hit } if hit.element == PaneElement::Content => {
                let content_local = hit.content_local?;
                Some((pane_id, hit, content_local))
            }
            _ => None,
        }
    }

    /// Returns the widget hit at the given screen position.
    #[inline]
    fn widget_hit_at_pos(&self, pos: Point) -> Option<WidgetHit> {
        let (pane_id, _, content_local) = self.content_hit_at(pos)?;
        self.widgets.widget_at(&self.canvas, pane_id, content_local)
    }

    /// Returns the pressed widget hit when the pointer is still over the same widget.
    #[inline]
    fn pressed_widget_hit_at(&self, pos: Point) -> Option<WidgetHit> {
        let pressed_id = self.widgets.pressed()?;
        let widget_hit = self.widget_hit_at_pos(pos)?;

        (widget_hit.widget_id == pressed_id).then_some(widget_hit)
    }

    /// Returns the widget hit that should handle the current mouse release.
    #[inline]
    fn mouse_up_widget_hit(&mut self, pos: Point) -> Option<WidgetHit> {
        let (pane_id, _, content_local) = self.content_hit_at(pos)?;
        self.widgets.mouse_up(&self.canvas, pane_id, content_local)
    }

    /// Starts the provided pointer drag and emits the corresponding UI event.
    #[inline]
    fn begin_pointer_drag(&mut self, pointer_drag: PointerDrag) -> UiEvent {
        self.focus_widget(None);
        self.clear_hover();
        self.drag = Some(pointer_drag);

        match pointer_drag {
            PointerDrag::PaneMove { pane_id, .. } => UiEvent::PaneDragStart {
                pane_id,
                kind: PaneDragKind::Move,
            },

            PointerDrag::PaneResize { pane_id } => UiEvent::PaneDragStart {
                pane_id,
                kind: PaneDragKind::Resize,
            },

            PointerDrag::Content {
                pane_id,
                anchor,
                current,
                ..
            } => UiEvent::PaneContentDragStart {
                pane_id,
                anchor,
                current,
            },
        }
    }

    /// Converts a widget release into the matching public UI event.
    #[inline]
    fn release_widget_event(&mut self, widget_hit: WidgetHit) -> UiEvent {
        let focused = self.widgets.focused() == Some(widget_hit.widget_id);

        let action = self
            .widgets
            .edit(widget_hit.widget_id, |w| w.release_action(focused))
            .unwrap_or(WidgetAction::Released);

        map_widget_action(widget_hit.widget_id, Some(widget_hit), action)
    }

    /// Returns true when the currently focused widget belongs to the pane.
    #[inline]
    fn focused_widget_on_pane(&self, pane_id: PaneId) -> bool {
        self.widgets
            .focused()
            .and_then(|widget_id| self.widgets.pane_id_of(widget_id))
            == Some(pane_id)
    }
}

/// Converts a widget behavior action into the matching public UI event.
#[inline]
fn map_widget_action(widget_id: WidgetId, hit: Option<WidgetHit>, action: WidgetAction) -> UiEvent {
    match action {
        WidgetAction::None => UiEvent::None,

        WidgetAction::Clicked => UiEvent::WidgetClicked(hit.unwrap_or(WidgetHit {
            widget_id,
            local: Point::ZERO,
        })),
        WidgetAction::Released => UiEvent::WidgetReleased(hit.unwrap_or(WidgetHit {
            widget_id,
            local: Point::ZERO,
        })),

        WidgetAction::CheckboxChanged(checked) => UiEvent::CheckboxChanged { widget_id, checked },
        WidgetAction::InputChanged => UiEvent::InputChanged { widget_id },
        WidgetAction::InputSubmitted(value) => UiEvent::InputSubmitted { widget_id, value },
        WidgetAction::SliderChanged(value) => UiEvent::SliderChanged { widget_id, value },

        WidgetAction::Custom(payload) => UiEvent::WidgetCustom { widget_id, payload },
    }
}
