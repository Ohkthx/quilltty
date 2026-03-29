//! File: src/ui/runtime.rs

use std::io::{self, Write};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    Canvas, PaneBuilder, PaneId, Widget, WidgetBuilder, WidgetHit, WidgetId, WidgetStore,
    geom::Point,
    render::{Compositor, Renderer},
    style::BorderKind,
    ui::{PaneElement, PaneHit},
};

/// Describes the kind of pane drag currently in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDragKind {
    Move,   // Pane is being repositioned.
    Resize, // Pane is being resized from the bottom-right.
}

/// High-level UI events emitted back to application code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    None,                                                   // No UI action occurred.
    PanePressed(PaneHit), // Mouse pressed pane content with no widget hit.
    PaneDragStart { pane_id: PaneId, kind: PaneDragKind }, // Pane drag has started.
    PaneDragged { pane_id: PaneId, kind: PaneDragKind }, // Pane is actively being dragged.
    PaneReleased { pane_id: PaneId, kind: PaneDragKind }, // Pane drag has ended.
    WidgetHovered(WidgetHit), // Pointer moved over a widget.
    WidgetPressed(WidgetHit), // Widget received mouse press.
    WidgetReleased(WidgetHit), // Widget received mouse release without activation.
    WidgetClicked(WidgetHit), // Widget was activated by click.
    CheckboxChanged { widget_id: WidgetId, checked: bool }, // Checkbox toggled state.
    InputChanged { widget_id: WidgetId }, // Input widget text changed.
    InputSubmitted { widget_id: WidgetId, value: String }, // Input widget was submitted.
}

/// Internal drag modes used while tracking pane drag state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragMode {
    Move { grab_offset: Point }, // Stores pointer offset while moving a pane.
    Resize,                      // Resizes the pane using the current pointer position.
}

impl DragMode {
    /// Converts the internal drag mode into the public drag kind.
    fn kind(self) -> PaneDragKind {
        match self {
            Self::Move { .. } => PaneDragKind::Move,
            Self::Resize => PaneDragKind::Resize,
        }
    }
}

/// Internal state for an active pane drag operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DragState {
    pane_id: PaneId, // Pane currently being dragged.
    mode: DragMode,  // Current drag behavior.
}

/// Coordinates pane rendering, widget state, and pointer-driven pane actions.
pub struct Ui {
    pub canvas: Canvas,       // Backing surface containing panes and root content.
    compositor: Compositor,   // Flattens pane damage into a renderable frame.
    renderer: Renderer,       // Writes frame differences to the terminal.
    pub widgets: WidgetStore, // Tracks widgets, focus, hover, and pressed state.
    drag: Option<DragState>,  // Active pane drag session, if any.
}

impl Ui {
    /// Creates a new UI with the given size and optional root border.
    pub fn new(width: usize, height: usize, border: Option<BorderKind>) -> Self {
        Self {
            canvas: Canvas::new(width, height, border),
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
    pub fn create_pane(&mut self) -> PaneBuilder<'_> {
        self.canvas.create_pane()
    }

    /// Returns a builder for creating a widget inside the given pane.
    pub fn widget(&mut self, pane_id: PaneId) -> WidgetBuilder<'_> {
        self.widgets.new_widget(pane_id)
    }

    /// Dispatches a raw mouse event into the appropriate UI handler.
    pub fn mouse(&mut self, mouse: MouseEvent) -> UiEvent {
        let pos: Point = (mouse.column, mouse.row).into();

        match mouse.kind {
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

        if let Some(hit) = self.canvas.pane_at(pos)
            && hit.element == PaneElement::Content
            && let Some(content_local) = hit.content_local
        {
            return self
                .widgets
                .hover(&self.canvas, hit.pane_id, content_local)
                .map(UiEvent::WidgetHovered)
                .unwrap_or(UiEvent::None);
        }

        self.widgets.clear_hover();
        UiEvent::None
    }

    /// Applies an in-progress pane move or resize using the current pointer position.
    pub fn mouse_drag(&mut self, pos: Point) -> UiEvent {
        let Some(drag) = self.drag else {
            return UiEvent::None;
        };

        match drag.mode {
            DragMode::Move { grab_offset } => {
                let new_origin = pos.saturating_sub(grab_offset);
                self.move_pane(drag.pane_id, new_origin, true);
            }

            DragMode::Resize => {
                if let Some(rect) = self.canvas.pane(drag.pane_id).map(|p| p.rect()) {
                    let width = pos.x.saturating_sub(rect.x).saturating_add(1);
                    let height = pos.y.saturating_sub(rect.y).saturating_add(1);
                    self.resize_pane(drag.pane_id, width, height);
                }
            }
        }

        UiEvent::PaneDragged {
            pane_id: drag.pane_id,
            kind: drag.mode.kind(),
        }
    }

    /// Handles mouse press for panes, widgets, and drag start behavior.
    pub fn mouse_down(&mut self, pos: Point) -> UiEvent {
        let Some(hit) = self.canvas.pane_at(pos) else {
            self.clear_focus();
            self.clear_hover();
            self.drag = None;
            return UiEvent::None;
        };

        self.focus_pane(hit.pane_id);

        match hit.element {
            PaneElement::Content => {
                if let Some(content_local) = hit.content_local
                    && let Some(widget_hit) =
                        self.widgets
                            .mouse_down(&self.canvas, hit.pane_id, content_local)
                {
                    return UiEvent::WidgetPressed(widget_hit);
                }

                self.focus_widget(None);
                self.clear_hover();
                UiEvent::PanePressed(hit)
            }

            PaneElement::Title | PaneElement::Border => {
                self.focus_widget(None);
                self.clear_hover();

                self.drag = Some(DragState {
                    pane_id: hit.pane_id,
                    mode: DragMode::Move {
                        grab_offset: hit.local,
                    },
                });

                UiEvent::PaneDragStart {
                    pane_id: hit.pane_id,
                    kind: PaneDragKind::Move,
                }
            }

            PaneElement::Resize => {
                self.focus_widget(None);
                self.clear_hover();

                self.drag = Some(DragState {
                    pane_id: hit.pane_id,
                    mode: DragMode::Resize,
                });

                UiEvent::PaneDragStart {
                    pane_id: hit.pane_id,
                    kind: PaneDragKind::Resize,
                }
            }
        }
    }

    /// Handles mouse release for pane drag completion and widget activation.
    pub fn mouse_up(&mut self, pos: Point) -> UiEvent {
        if let Some(drag) = self.drag.take() {
            self.sync_hover(pos);

            return UiEvent::PaneReleased {
                pane_id: drag.pane_id,
                kind: drag.mode.kind(),
            };
        }

        if let Some(hit) = self.canvas.pane_at(pos) {
            if hit.element == PaneElement::Content
                && let Some(content_local) = hit.content_local
            {
                if let Some(widget_hit) =
                    self.widgets
                        .mouse_up(&self.canvas, hit.pane_id, content_local)
                {
                    if self.widgets.focused() == Some(widget_hit.widget_id) {
                        if let Some(checked) = self
                            .widgets
                            .edit(widget_hit.widget_id, |w| {
                                w.as_checkbox_mut().map(|checkbox| checkbox.toggle())
                            })
                            .flatten()
                        {
                            return UiEvent::CheckboxChanged {
                                widget_id: widget_hit.widget_id,
                                checked,
                            };
                        }

                        return UiEvent::WidgetClicked(widget_hit);
                    }

                    return UiEvent::WidgetReleased(widget_hit);
                }

                return UiEvent::None;
            }

            self.widgets.clear_hover();
            return UiEvent::None;
        }

        self.widgets.clear_hover();
        UiEvent::None
    }

    /// Recomputes hover state after drag completion.
    fn sync_hover(&mut self, pos: Point) {
        if let Some(hit) = self.canvas.pane_at(pos)
            && hit.element == PaneElement::Content
            && let Some(content_local) = hit.content_local
        {
            let _ = self.widgets.hover(&self.canvas, hit.pane_id, content_local);
        } else {
            self.widgets.clear_hover();
        }
    }

    /// Handles keyboard interaction for the currently focused widget.
    pub fn key(&mut self, key: KeyEvent) -> UiEvent {
        let Some(widget_id) = self.widgets.focused() else {
            return UiEvent::None;
        };

        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(checked) = self
                    .widgets
                    .edit(widget_id, |w| {
                        w.as_checkbox_mut().map(|checkbox| checkbox.toggle())
                    })
                    .flatten()
                {
                    return UiEvent::CheckboxChanged { widget_id, checked };
                }

                if let Some(value) = self
                    .widgets
                    .edit(widget_id, |w| w.as_input_mut().map(|input| input.submit()))
                    .flatten()
                {
                    return UiEvent::InputSubmitted { widget_id, value };
                }

                if matches!(self.widgets.get(widget_id), Some(Widget::Button(_))) {
                    return UiEvent::WidgetClicked(WidgetHit {
                        widget_id,
                        local: Point::ZERO,
                    });
                }

                UiEvent::None
            }

            KeyCode::Char(ch) => {
                if self
                    .widgets
                    .edit(widget_id, |w| {
                        w.as_input_mut().map(|input| input.insert_char(ch))
                    })
                    .flatten()
                    .is_some()
                {
                    UiEvent::InputChanged { widget_id }
                } else {
                    UiEvent::None
                }
            }

            KeyCode::Backspace => {
                if self
                    .widgets
                    .edit(widget_id, |w| {
                        w.as_input_mut().map(|input| input.backspace())
                    })
                    .flatten()
                    .is_some()
                {
                    UiEvent::InputChanged { widget_id }
                } else {
                    UiEvent::None
                }
            }

            KeyCode::Left => {
                if self
                    .widgets
                    .edit(widget_id, |w| {
                        w.as_input_mut().map(|input| input.move_left())
                    })
                    .flatten()
                    .is_some()
                {
                    UiEvent::InputChanged { widget_id }
                } else {
                    UiEvent::None
                }
            }

            KeyCode::Right => {
                if self
                    .widgets
                    .edit(widget_id, |w| {
                        w.as_input_mut().map(|input| input.move_right())
                    })
                    .flatten()
                    .is_some()
                {
                    UiEvent::InputChanged { widget_id }
                } else {
                    UiEvent::None
                }
            }

            _ => UiEvent::None,
        }
    }

    /// Focuses a pane and refreshes widget state for the old and new pane.
    pub fn focus_pane(&mut self, pane_id: PaneId) {
        let old_pane = self.canvas.focused();
        if old_pane == pane_id {
            return;
        }

        self.canvas.focus(pane_id);

        // Recompute cursor/render state for both panes.
        self.widgets.invalidate_pane(old_pane);
        self.widgets.invalidate_pane(pane_id);
    }

    /// Focuses a widget and updates pane focus and cursor visibility.
    pub fn focus_widget(&mut self, widget_id: Option<WidgetId>) {
        let old_widget = self.widgets.focused();
        let old_pane = old_widget.and_then(|id| self.widgets.pane_id_of(id));
        let new_pane = widget_id.and_then(|id| self.widgets.pane_id_of(id));

        self.widgets.focus(widget_id);

        match new_pane {
            Some(pane_id) => self.canvas.focus(pane_id),
            None => self.canvas.set_cursor(None),
        }

        if let Some(pane_id) = old_pane {
            self.widgets.invalidate_pane(pane_id);
        }

        if let Some(pane_id) = new_pane {
            self.widgets.invalidate_pane(pane_id);
        }
    }

    /// Returns the currently focused pane id.
    pub fn focused_pane(&self) -> PaneId {
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

    /// Clears widget focus and returns focus to the root canvas pane.
    pub fn clear_focus(&mut self) {
        let old_pane = self
            .widgets
            .focused()
            .and_then(|widget_id| self.widgets.pane_id_of(widget_id));

        self.widgets.focus(None);
        self.canvas.focus(Canvas::ROOT_ID);
        self.canvas.set_cursor(None);

        if let Some(pane_id) = old_pane {
            self.widgets.invalidate_pane(pane_id);
        }
    }

    /// Moves a pane and refreshes cursor state if a focused widget lives inside it.
    pub fn move_pane(&mut self, pane_id: PaneId, origin: Point, clamp: bool) {
        self.canvas.move_pane(pane_id, origin, clamp);

        // Moving a pane changes the screen-space cursor location for any focused widget
        // inside it, so invalidate that pane to force cursor recomputation.
        let focused_on_this_pane = self
            .widgets
            .focused()
            .and_then(|widget_id| self.widgets.pane_id_of(widget_id))
            == Some(pane_id);

        if self.canvas.focused() == pane_id && focused_on_this_pane {
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
        let focused_on_this_pane = self
            .widgets
            .focused()
            .and_then(|widget_id| self.widgets.pane_id_of(widget_id))
            == Some(pane_id);

        if focused_on_this_pane {
            self.widgets.focus(None);
            self.canvas.set_cursor(None);
        }

        if self.canvas.focused() == pane_id {
            self.canvas.focus(Canvas::ROOT_ID);
        }

        self.widgets.clear_hover();
        self.canvas.toggle_pane_visibility(pane_id);
        self.widgets.invalidate_pane(pane_id);
    }

    /// Sets the title text for a pane.
    pub fn set_pane_title<S: Into<String>>(&mut self, pane_id: PaneId, title: Option<S>) {
        self.canvas.set_pane_title(pane_id, title.map(Into::into));
    }

    /// Marks every widget in a pane as needing redraw.
    pub fn invalidate_pane(&mut self, pane_id: PaneId) {
        self.widgets.invalidate_pane(pane_id);
    }

    /// Marks a single widget as needing redraw.
    pub fn invalidate_widget(&mut self, widget_id: WidgetId) {
        self.widgets.invalidate_widget(widget_id);
    }

    /// Marks all widgets across all panes as needing redraw.
    pub fn invalidate_all(&mut self) {
        self.widgets.invalidate_all();
    }

    /// Returns an immutable reference to the underlying canvas.
    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    /// Returns a mutable reference to the underlying canvas.
    pub fn canvas_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }

    /// Returns an immutable reference to the widget store.
    pub fn widgets(&self) -> &WidgetStore {
        &self.widgets
    }

    /// Returns a mutable reference to the widget store.
    pub fn widgets_mut(&mut self) -> &mut WidgetStore {
        &mut self.widgets
    }
}
