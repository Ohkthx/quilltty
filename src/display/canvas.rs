//! File: src/display/canvas.rs

use crate::display::backend::DamagedSpan;
use crate::{Compositor, Glyph, Pane, PaneBuilder, PaneId, Rect, Renderer};

/// Manages panes, their ordering, creation, and deletion.
pub struct Canvas {
    pub(crate) root: Pane,                // Main pane.
    pub(crate) panes: Vec<Pane>,          // Child panes to the root.
    pub(crate) damaged: Vec<DamagedSpan>, // Damaged spans for each canvas row.
    pub(crate) freed_ids: Vec<PaneId>,    // Reusable PaneIds.
}

impl Canvas {
    /// Reserved `PaneId` for the root pane.
    pub const ROOT_ID: PaneId = PaneId(u32::MAX - 1);

    /// Creates a new instance with a root pane covering the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
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
                .with_data(vec![Glyph::default(); width * height]),
            panes: Vec::new(),
            damaged: vec![DamagedSpan::default(); height],
            freed_ids: Vec::new(),
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
            return Some(&self.root);
        }

        self.panes.iter().find(|p| p.id == pane_id)
    }

    /// Obtains a mutable pane from currently managed panes.
    pub fn pane_mut(&mut self, pane_id: PaneId) -> Option<&mut Pane> {
        if pane_id == Self::ROOT_ID {
            return Some(&mut self.root);
        }

        self.panes.iter_mut().find(|p| p.id == pane_id)
    }

    /// Writes a `Glyph` to the root pane at `(x, y)`.
    pub fn set(&mut self, x: usize, y: usize, glyph: impl Into<Glyph>) {
        self.root.set(x, y, glyph);
    }

    /// Moves the `Pane` to the specified XY-coordinate, optionally clamping to the `Canvas`.
    pub fn move_pane(&mut self, pane_id: PaneId, x: usize, y: usize, clamp: bool) -> bool {
        if pane_id == Self::ROOT_ID {
            return false;
        }

        let bounds = self.root.rect;

        let Some((old_rect, old_visible, new_rect, _, _)) =
            self.with_pane_state_change(pane_id, |pane| {
                let rect = pane.rect.position(x, y);
                pane.rect = if clamp { rect.clamp_to(bounds) } else { rect };
            })
        else {
            return false;
        };

        if old_rect.x == new_rect.x && old_rect.y == new_rect.y {
            return true;
        }

        if old_visible {
            self.mark_rect_dirty(old_rect);
            self.mark_rect_dirty(new_rect);
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

        self.mark_rect_dirty(old_rect);
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

        self.mark_rect_dirty(new_rect);
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
            self.mark_rect_dirty(old_rect);
        }

        if new_visible {
            self.mark_rect_dirty(new_rect);
        }

        true
    }

    /// Collects damage, composes dirty regions, renders the differences, and writes the output.
    pub fn render<W: std::io::Write>(
        &mut self,
        compositor: &mut Compositor,
        renderer: &mut Renderer,
        out: &mut W,
    ) -> std::io::Result<()> {
        self.collect_damage();

        if self.damaged.iter().any(|span| span.dirty) {
            compositor.flatten(&self.root, &self.panes, &self.damaged);
            renderer.render(compositor, &self.damaged, out)?;
        }

        self.root.clear_damaged();
        for pane in &mut self.panes {
            pane.clear_damaged();
        }
        self.clear_damage();

        Ok(())
    }

    /// Locates the `Pane` index.
    fn pane_index(&self, pane_id: PaneId) -> Option<usize> {
        self.panes.iter().position(|p| p.id == pane_id)
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
        let idx = self.pane_index(pane_id)?;

        let old_rect = self.panes[idx].rect;
        let old_visible = self.panes[idx].visible;

        let result = {
            let pane = &mut self.panes[idx];
            f(pane)
        };

        let new_rect = self.panes[idx].rect;
        let new_visible = self.panes[idx].visible;

        Some((old_rect, old_visible, new_rect, new_visible, result))
    }

    /// Marks the visible portion of `rect` as damaged in canvas space.
    fn mark_rect_dirty(&mut self, rect: Rect) {
        let Rect { width, height, .. } = self.root.rect;
        if self.damaged.is_empty() || rect.width == 0 || rect.height == 0 {
            return;
        }

        if rect.x >= width || rect.y >= height {
            return;
        }

        let x0 = rect.x;
        let x1 = rect
            .x
            .saturating_add(rect.width)
            .saturating_sub(1)
            .min(width - 1);

        let y0 = rect.y;
        let y1 = rect
            .y
            .saturating_add(rect.height)
            .saturating_sub(1)
            .min(height - 1);

        for row in y0..=y1 {
            self.damaged[row].mark_range(x0, x1);
        }
    }

    /// Marks an inclusive horizontal span on a single canvas row as damaged.
    #[inline]
    fn mark_canvas_span_in(
        dirty: &mut [DamagedSpan],
        canvas_width: usize,
        y: usize,
        x0: usize,
        x1: usize,
    ) {
        if dirty.is_empty() || y >= dirty.len() || x0 >= canvas_width {
            return;
        }

        let x1 = x1.min(canvas_width - 1);
        if x0 > x1 {
            return;
        }

        dirty[y].mark_range(x0, x1);
    }

    /// Projects pane-local damaged spans into canvas-space damaged spans.
    fn project_pane_damage_to_canvas(
        dirty: &mut [DamagedSpan],
        canvas_width: usize,
        canvas_height: usize,
        pane: &Pane,
    ) {
        for local_y in 0..pane.height() {
            let span = pane.damaged[local_y];
            if !span.dirty {
                continue;
            }

            let canvas_y = pane.rect.y + local_y;
            if canvas_y >= canvas_height {
                continue;
            }

            let x0 = pane.rect.x.saturating_add(span.start);
            let x1 = pane.rect.x.saturating_add(span.end);

            Self::mark_canvas_span_in(dirty, canvas_width, canvas_y, x0, x1);
        }
    }

    /// Projects pane-local damaged spans into canvas-space damaged spans.
    fn collect_damage(&mut self) {
        let Rect { width, height, .. } = self.root.rect;
        Self::project_pane_damage_to_canvas(&mut self.damaged, width, height, &self.root);

        for pane in &self.panes {
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
