//! File: src/surface/canvas.rs

use crate::{
    Pane, PaneBuilder, PaneId,
    geom::{Point, Rect},
    render::{Compositor, Renderer},
    style::{BorderKind, Color, Glyph, Style},
    surface::{backend::DamagedRow, indexed_vec::IndexedVec},
    ui::PaneElement,
};

/// `Pane` and `Element` for hit detection.
pub struct PaneHit {
    /// Unique identifier for the `Pane`.
    pub pane_id: PaneId,
    /// Element that was hit.
    pub element: PaneElement,
    /// Local `Point`.
    pub local: Point,
    /// Global `Point`
    pub global: Point,
}

/// Manages panes, their ordering, creation, and deletion.
pub struct Canvas {
    pub(crate) root: Pane,                      // Main pane.
    pub(crate) panes: IndexedVec<PaneId, Pane>, // Child panes to the root.
    pub(crate) damaged: Vec<DamagedRow>,        // Damaged spans for each canvas row.
    pub(crate) freed_ids: Vec<PaneId>,          // Reusable PaneIds.
    pub(crate) cursor: Option<Point>,           // Cursor position on the Canvas.
    pub(crate) focus: PaneId,                   // Currently focused Pane.
}

impl Canvas {
    /// Reserved `PaneId` for the root pane.
    pub const ROOT_ID: PaneId = PaneId(u32::MAX - 1);

    /// Creates a new instance with a root pane covering the given dimensions.
    pub fn new(width: usize, height: usize, border: Option<BorderKind>) -> Self {
        assert!(width > 0 && height > 0, "Canvas size must be > 0");

        let rect = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };

        Self {
            root: Pane::new(Self::ROOT_ID)
                .with_rect(rect)
                .with_z_layer(0)
                .with_focus(false)
                .with_visibility(true)
                .with_movability(false)
                .with_resizability(false)
                .with_border(border)
                .with_border_style(Style::default().with_fg(Color::White))
                .with_title(None)
                .with_data(vec![Glyph::default(); width * height])
                .build(),
            panes: IndexedVec::new(),
            damaged: vec![DamagedRow::default(); height],
            freed_ids: Vec::new(),
            cursor: None,
            focus: Self::ROOT_ID,
        }
    }

    /// Creates a new pane, reusing previously freed `PaneId`s when available.
    pub fn create_pane(&mut self) -> PaneBuilder<'_> {
        let rect = self.root.rect;
        PaneBuilder {
            canvas: self,
            rect,
            z_layer: 1,
            visible: true,
            movable: true,
            resizable: true,
            border: None,
            border_style: Style::default().with_fg(Color::White),
            title: None,
        }
    }

    /// Immutable root pane for the display.
    pub fn root(&self) -> &Pane {
        &self.root
    }

    /// Mutable root pane used as the basis for all writes.
    pub fn root_mut(&mut self) -> &mut Pane {
        &mut self.root
    }

    /// Obtains an immutable pane from currently managed panes.
    pub fn pane(&self, pane_id: PaneId) -> Option<&Pane> {
        if pane_id == Self::ROOT_ID {
            Some(&self.root)
        } else {
            self.panes.get(&pane_id)
        }
    }

    /// Obtains a mutable pane from currently managed panes.
    pub fn pane_mut(&mut self, pane_id: PaneId) -> Option<&mut Pane> {
        if pane_id == Self::ROOT_ID {
            Some(&mut self.root)
        } else {
            self.panes.get_mut(&pane_id)
        }
    }

    /// Sets the cursor to specific coordinates on the `Canvas`.
    pub fn set_cursor(&mut self, cursor: Option<Point>) {
        self.cursor = cursor;
    }

    /// Writes a `Glyph` to the root pane at `(x, y)`.
    pub fn set(&mut self, position: Point, glyph: impl Into<Glyph>) {
        self.root.set(position, glyph);
    }

    /// Set the `Pane` title to a new value.
    pub fn set_pane_title(&mut self, pane_id: PaneId, title: Option<String>) -> bool {
        if pane_id == Self::ROOT_ID {
            self.root.set_title(title);

            let width = self.root.rect.width;
            let y = self.root.rect.y;

            if width > 0 {
                // Mark the entire top row as damaged.
                Self::mark_canvas_span_in(&mut self.damaged, width, y, 0, width);
            }

            return true;
        }

        let Some((rect, visible, _, _, _)) =
            self.with_pane_state_change(pane_id, |pane| pane.set_title(title))
        else {
            return false;
        };

        if visible && rect.width > 0 {
            let y = rect.y;
            let x0 = rect.x;
            let x1 = rect.x + rect.width;
            Self::mark_canvas_span_in(&mut self.damaged, self.root.rect.width, y, x0, x1);
        }

        true
    }

    /// Obtains the top-most `PaneId` and `PaneElement` at the `Point` provided.
    pub fn pane_at(&self, position: Point) -> Option<PaneHit> {
        for pane in self.panes.iter().rev() {
            if !pane.visible || !pane.rect.contains_point(position) {
                continue; // Ignore these panes.
            }

            // Extract the hit element.
            let local = position.saturating_sub(pane.rect.origin());
            if let Some(element) = pane.element_at(local) {
                return Some(PaneHit {
                    pane_id: pane.id,
                    element,
                    local,
                    global: position,
                });
            }
        }

        // Default to root.
        if self.root.rect().contains_point(position) {
            let local = position.saturating_sub(self.root.rect.origin());
            return self.root.element_at(local).map(|element| PaneHit {
                pane_id: Self::ROOT_ID,
                element,
                local,
                global: position,
            });
        }

        None
    }

    /// Returns the `PaneId` for the pane that is currently focused.
    pub fn focus_id(&self) -> PaneId {
        self.focus
    }

    /// Sets the `PaneId` to be the current focus.
    pub fn focus_pane(&mut self, pane_id: PaneId) -> bool {
        if pane_id == self.focus {
            return true; // Nothing to do.
        } else if self.pane(pane_id).is_none() {
            return false; // Pane not found.
        }

        let old_id = self.focus;
        self.focus = pane_id;

        if let Some(old) = self.pane_mut(old_id) {
            old.set_focus(false);
        }

        if let Some(new) = self.pane_mut(pane_id) {
            new.set_focus(true);
        }

        true
    }

    /// Resizes the `Pane`, extending or shrinking from the resize point.
    pub fn resize_pane(&mut self, pane_id: PaneId, width: usize, height: usize) -> bool {
        if pane_id == Self::ROOT_ID {
            return false;
        }

        let bounds = self.root.rect;
        let Some((old_rect, old_visible, new_rect, _, _)) =
            self.with_pane_state_change(pane_id, |pane| {
                if !pane.resizable {
                    return;
                }

                let mut width = width;
                let mut height = height;

                let min = pane.minimum_size();
                let max_width = bounds
                    .x
                    .saturating_add(bounds.width)
                    .saturating_sub(pane.rect.x)
                    .max(min.x);

                let max_height = bounds
                    .y
                    .saturating_add(bounds.height)
                    .saturating_sub(pane.rect.y)
                    .max(min.y);

                width = width.min(max_width);
                height = height.min(max_height);

                pane.resize(width, height);
            })
        else {
            return false;
        };

        if old_rect == new_rect {
            return false;
        }

        if old_visible {
            self.mark_damaged(old_rect);
            self.mark_damaged(new_rect);
        }

        true
    }

    /// Moves the `Pane` to the specified XY-coordinate, optionally clamping to the `Canvas`.
    pub fn move_pane(&mut self, pane_id: PaneId, position: Point, clamp: bool) -> bool {
        if pane_id == Self::ROOT_ID {
            return false;
        }

        let bounds = self.root.rect;
        let Some((old_rect, old_visible, new_rect, _, _)) =
            self.with_pane_state_change(pane_id, |pane| {
                if !pane.movable {
                    return;
                }

                let rect = pane.rect.with_origin(position);
                pane.rect = if clamp { rect.clamp_to(bounds) } else { rect };
            })
        else {
            return false;
        };

        if old_rect == new_rect {
            return false;
        }

        if old_visible {
            self.mark_damaged(old_rect);
            self.mark_damaged(new_rect);
        }

        true
    }

    /// Marks the `Pane` as invisible.
    pub fn hide_pane(&mut self, pane_id: PaneId) -> bool {
        assert!(pane_id != Self::ROOT_ID, "Cannot hide root pane.");

        let Some((old_rect, old_visible, _, new_visible, _)) =
            self.with_pane_state_change(pane_id, |pane| pane.hide())
        else {
            return false;
        };

        if old_visible == new_visible {
            return false;
        }

        self.mark_damaged(old_rect);
        true
    }

    /// Marks the `Pane` as visible.
    pub fn show_pane(&mut self, pane_id: PaneId) -> bool {
        assert!(pane_id != Self::ROOT_ID, "Cannot show root pane.");

        let Some((_, old_visible, new_rect, new_visible, _)) =
            self.with_pane_state_change(pane_id, |pane| pane.show())
        else {
            return false;
        };

        if old_visible == new_visible {
            return false;
        }

        self.mark_damaged(new_rect);
        true
    }

    /// Toggles the visibility of the `Pane` between shown and hidden.
    pub fn toggle_pane_visibility(&mut self, pane_id: PaneId) -> bool {
        assert!(
            pane_id != Self::ROOT_ID,
            "Cannot toggle root pane visibility."
        );

        let Some((old_rect, old_visible, new_rect, new_visible, _)) =
            self.with_pane_state_change(pane_id, |pane| pane.toggle_visibility())
        else {
            return false;
        };

        if old_visible == new_visible {
            return false;
        }

        if old_visible {
            self.mark_damaged(old_rect);
        }

        if new_visible {
            self.mark_damaged(new_rect);
        }

        true
    }

    /// Collects damage, composes damaged regions, renders the differences, and writes the output.
    pub fn render<W: std::io::Write>(
        &mut self,
        compositor: &mut Compositor,
        renderer: &mut Renderer,
        out: &mut W,
    ) -> std::io::Result<()> {
        self.collect_damage();

        let has_damage = self.damaged.iter().any(|span| span.is_damaged());
        if has_damage {
            compositor.flatten(&self.root, self.panes.as_slice(), &self.damaged);
        }

        renderer.render(compositor, &self.damaged, self.cursor, out)?;
        self.root.clear_damaged();
        for pane in self.panes.iter_mut() {
            pane.clear_damaged();
        }
        self.clear_damage();

        Ok(())
    }

    /// Applies `f` to the pane identified by `pane_id` and returns its rectangle and
    /// visibility state before and after the change, along with `f`'s result.
    ///
    /// Returns `None` if no pane with `pane_id` exists.
    fn with_pane_state_change<R>(
        &mut self,
        pane_id: PaneId,
        f: impl FnOnce(&mut Pane) -> R,
    ) -> Option<(Rect, bool, Rect, bool, R)> {
        let pane = self.panes.get_mut(&pane_id)?;

        let old_rect = pane.rect;
        let old_visible = pane.visible;
        let result = f(pane);
        let new_rect = pane.rect;
        let new_visible = pane.visible;

        Some((old_rect, old_visible, new_rect, new_visible, result))
    }

    /// Marks the visible portion of `rect` as damaged in canvas space.
    fn mark_damaged(&mut self, rect: Rect) {
        let Rect { width, height, .. } = self.root.rect;
        if self.damaged.is_empty() || rect.width == 0 || rect.height == 0 {
            return;
        }

        if rect.x >= width || rect.y >= height {
            return;
        }

        let x0 = rect.x;
        let x1 = rect.x.saturating_add(rect.width).min(width);

        let y0 = rect.y;
        let y1 = rect.y.saturating_add(rect.height).min(height);

        for row in y0..y1 {
            self.damaged[row].mark_range(x0, x1);
        }
    }

    /// Marks an inclusive horizontal span on a single canvas row as damaged.
    #[inline]
    fn mark_canvas_span_in(
        damaged: &mut [DamagedRow],
        canvas_width: usize,
        y: usize,
        x0: usize,
        x1: usize,
    ) {
        if damaged.is_empty() || y >= damaged.len() || x0 >= canvas_width {
            return;
        }

        let x1 = x1.min(canvas_width);
        if x0 >= x1 {
            return;
        }

        damaged[y].mark_range(x0, x1);
    }

    /// Projects pane-local damaged spans into canvas-space damaged spans.
    fn project_pane_damage_to_canvas(
        damaged: &mut [DamagedRow],
        canvas_width: usize,
        canvas_height: usize,
        pane: &Pane,
    ) {
        for local_y in 0..pane.height() {
            let row = &pane.damaged[local_y];
            if !row.is_damaged() {
                continue;
            }

            let canvas_y = pane.rect.y + local_y;
            if canvas_y >= canvas_height {
                continue;
            }

            for span in row.spans() {
                let x0 = pane.rect.x.saturating_add(span.start);
                let x1 = pane.rect.x.saturating_add(span.end);

                Self::mark_canvas_span_in(damaged, canvas_width, canvas_y, x0, x1);
            }
        }
    }

    /// Projects pane-local damaged spans into canvas-space damaged spans.
    fn collect_damage(&mut self) {
        let Rect { width, height, .. } = self.root.rect;
        Self::project_pane_damage_to_canvas(&mut self.damaged, width, height, &self.root);

        for pane in self.panes.iter() {
            if pane.visible {
                Self::project_pane_damage_to_canvas(&mut self.damaged, width, height, pane);
            }
        }
    }

    /// Collects root and visible pane damage into the canvas damage buffer.
    fn clear_damage(&mut self) {
        for span in &mut self.damaged {
            span.clear();
        }
    }
}
