//! File: src/surface/canvas.rs

use std::collections::HashMap;

use crate::{
    Pane, PaneBuilder, PaneId,
    geom::{Point, Rect, Size},
    render::{Compositor, Renderer},
    style::Glyph,
    surface::{
        PaneAction,
        backend::{DamagedRow, Layer},
        decor::PaneDecor,
        indexed_vec::IndexedVec,
        pane,
        policy::PanePolicy,
    },
    ui::PaneElement,
};

/// `Pane` hit information.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PaneHit {
    /// Element that was hit.
    pub element: PaneElement,
    /// Global `Point`.
    pub global: Point,
    /// Position local to the pane.
    pub local: Point,
    /// Position local to the pane content, only for content hits.
    pub content_local: Option<Point>,
}

/// Location that was hit during hit-testing.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum HitTarget {
    /// A visible pane was hit.
    Pane { pane_id: PaneId, hit: PaneHit },
    /// Empty canvas/background area was hit.
    Background { global: Point },
}

/// Manages panes, ordering, focus, and damage over the canvas.
pub struct Canvas {
    size: Size,          // Size of the canvas.
    clear_glyph: Glyph,  // Glyph used to clear uncovered cells.
    forced_redraw: bool, // Forces redraw next render.

    pub(crate) panes: IndexedVec<PaneId, Pane>, // All panes.
    pub(crate) policies: HashMap<PaneId, PanePolicy>,
    pub(crate) decor: HashMap<PaneId, PaneDecor>,

    pub(crate) damaged: Vec<DamagedRow>, // Damaged spans in canvas space.
    pub(crate) freed_ids: Vec<PaneId>,   // Reusable PaneIds.
    pub(crate) cursor: Option<Point>,    // Cursor position on the canvas.
    pub(crate) focused: Option<PaneId>,  // Currently focused pane.
}

impl Canvas {
    /// Creates a new instance with the given dimensions.
    pub fn new(size: Size, bg: Option<Glyph>) -> Self {
        assert!(size != Size::ZERO, "Canvas size must be > 0");

        Self {
            size,
            clear_glyph: bg.unwrap_or(Glyph::from(' ')),
            forced_redraw: true,

            panes: IndexedVec::new(),
            policies: HashMap::new(),
            decor: HashMap::new(),

            damaged: vec![DamagedRow::default(); size.height],
            freed_ids: Vec::new(),
            cursor: None,
            focused: None,
        }
    }

    /// Creates a new pane builder using default pane policy and decor.
    pub fn create_pane(&mut self) -> PaneBuilder<'_> {
        let rect = self.rect();

        PaneBuilder {
            canvas: self,
            rect,
            z_layer: Layer::default(),
            visible: true,
            policy: PanePolicy::default(),
            decor: PaneDecor::default(),
        }
    }

    /// Returns the canvas size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Full rectangle occupied by the canvas.
    pub fn rect(&self) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: self.size.width,
            height: self.size.height,
        }
    }

    /// Returns the glyph used to clear uncovered cells.
    pub fn clear_glyph(&self) -> Glyph {
        self.clear_glyph
    }

    /// Sets the glyph used to clear uncovered cells.
    pub fn set_clear_glyph(&mut self, glyph: Glyph) {
        if self.clear_glyph != glyph {
            self.clear_glyph = glyph;
            self.mark_damaged(self.rect());
        }
    }

    /// Forces the next render to perform a full redraw.
    pub fn force_redraw(&mut self) {
        self.forced_redraw = true;
    }

    /// Sets the cursor to specific coordinates on the canvas.
    pub fn set_cursor(&mut self, cursor: Option<Point>) {
        self.cursor = cursor;
    }

    /// Obtains an immutable pane from the managed pane list.
    pub fn pane(&self, pane_id: PaneId) -> Option<&Pane> {
        self.panes.get(&pane_id)
    }

    /// Obtains a mutable pane from the managed pane list.
    pub fn pane_mut(&mut self, pane_id: PaneId) -> Option<&mut Pane> {
        self.panes.get_mut(&pane_id)
    }

    /// Sets a pane title and marks the affected title row span as damaged.
    pub fn set_pane_title(&mut self, pane_id: PaneId, title: Option<String>) -> bool {
        let Some(decor) = self.decor.get_mut(&pane_id) else {
            return false;
        };

        if !decor.set_title(title) {
            return false;
        }

        self.sync_decor(pane_id);

        let Some(rect) = self.pane(pane_id).map(|pane| pane.rect()) else {
            return false;
        };

        if rect.width > 0 {
            let width = self.size.width;
            let y = rect.y;
            let x0 = rect.x;
            let x1 = rect.x.saturating_add(rect.width);
            Self::mark_canvas_span_in(&mut self.damaged, width, y, x0, x1);
        }

        true
    }

    /// Returns the policy for a pane.
    pub fn pane_policy(&self, pane_id: PaneId) -> Option<&PanePolicy> {
        self.policies.get(&pane_id)
    }

    /// Returns the mutable policy for a pane.
    pub fn pane_policy_mut(&mut self, pane_id: PaneId) -> Option<&mut PanePolicy> {
        self.policies.get_mut(&pane_id)
    }

    /// Returns the decoration for a pane.
    pub fn pane_decor(&self, pane_id: PaneId) -> Option<&PaneDecor> {
        self.decor.get(&pane_id)
    }

    /// Returns the mutable decoration for a pane.
    pub fn pane_decor_mut(&mut self, pane_id: PaneId) -> Option<&mut PaneDecor> {
        self.decor.get_mut(&pane_id)
    }

    /// Returns the cached content rect for a pane.
    pub fn content_rect(&self, pane_id: PaneId) -> Option<Rect> {
        self.pane(pane_id).map(Pane::content_rect)
    }

    /// Updates the cached content rect for a pane.
    fn sync_content_rect(&mut self, pane_id: PaneId) -> bool {
        let insets = self
            .decor
            .get(&pane_id)
            .map(PaneDecor::insets)
            .unwrap_or_default();

        let Some(pane) = self.pane_mut(pane_id) else {
            return false;
        };

        let rect = pane.rect();
        pane.set_content_rect(Rect {
            x: rect.x.saturating_add(insets.left),
            y: rect.y.saturating_add(insets.top),
            width: rect.width.saturating_sub(insets.left + insets.right),
            height: rect.height.saturating_sub(insets.top + insets.bottom),
        });

        true
    }

    /// Synchronizes the cached content rect and rerenders pane decoration.
    pub fn sync_decor(&mut self, pane_id: PaneId) {
        let focused = self.focused == Some(pane_id);
        if !self.sync_content_rect(pane_id) {
            return;
        }

        let Some(decor) = self.decor.remove(&pane_id) else {
            return;
        };

        let Some(policy) = self.policies.remove(&pane_id) else {
            return;
        };

        let pane_rect = if let Some(pane) = self.pane_mut(pane_id) {
            decor.render(pane, focused, policy.resizable);
            Some(pane.rect())
        } else {
            None
        };

        self.decor.insert(pane_id, decor);
        self.policies.insert(pane_id, policy);

        if let Some(rect) = pane_rect {
            self.mark_damaged(rect);
        }
    }

    /// Resolves the action to perform for a pane hit.
    pub fn action_for_hit(
        &self,
        pane_id: PaneId,
        element: PaneElement,
        local: Point,
    ) -> PaneAction {
        self.policies
            .get(&pane_id)
            .copied()
            .unwrap_or_default()
            .action_for_hit(element, local)
    }

    /// Returns the top-most target at the given canvas position.
    pub fn hit_at(&self, position: Point) -> HitTarget {
        for pane in self.panes.iter().rev() {
            if !pane.visible || !pane.rect.contains_point(position) {
                continue;
            }

            let local = position.saturating_sub(pane.rect.origin());
            let content = pane.content_rect();

            if content.contains_point(position) {
                return HitTarget::Pane {
                    pane_id: pane.id,
                    hit: PaneHit {
                        element: PaneElement::Content,
                        global: position,
                        local,
                        content_local: Some(position.saturating_sub(content.origin())),
                    },
                };
            }

            let element = self
                .decor
                .get(&pane.id)
                .map(|decor| decor.hit_test(pane, local))
                .unwrap_or(PaneElement::Content);

            return HitTarget::Pane {
                pane_id: pane.id,
                hit: PaneHit {
                    element,
                    global: position,
                    local,
                    content_local: None,
                },
            };
        }

        let min = Point::ZERO;
        let max = Point::new(
            self.size.width.saturating_sub(1),
            self.size.height.saturating_sub(1),
        );

        HitTarget::Background {
            global: position.clamp(min, max),
        }
    }

    /// Returns the currently focused pane, if any.
    pub fn focused(&self) -> Option<PaneId> {
        self.focused
    }

    /// Sets the focused pane. `None` clears pane focus.
    pub fn focus(&mut self, pane_id: Option<PaneId>) {
        if pane_id == self.focused {
            return;
        }

        if let Some(id) = pane_id
            && self.pane(id).is_none()
        {
            return;
        }

        let old_id = self.focused;
        self.focused = pane_id;

        if let Some(old_id) = old_id {
            self.sync_decor(old_id);
        }

        if let Some(new_id) = pane_id {
            self.sync_decor(new_id);
        }

        self.cursor = None;
    }

    /// Resizes a pane, clamped to the canvas bounds and decor minimum size.
    pub fn resize_pane(&mut self, pane_id: PaneId, width: usize, height: usize) -> bool {
        let bounds = self.rect();
        let focused = self.focused == Some(pane_id);

        let policy = self.policies.get(&pane_id).copied().unwrap_or_default();

        if !policy.resizable {
            return false;
        }

        let min = self
            .decor
            .get(&pane_id)
            .map(PaneDecor::min_outer_size)
            .unwrap_or(Size {
                width: 1,
                height: 1,
            });

        let Some(old_pane) = self.pane(pane_id) else {
            return false;
        };

        let max_width = bounds
            .x
            .saturating_add(bounds.width)
            .saturating_sub(old_pane.rect.x)
            .max(min.width);

        let max_height = bounds
            .y
            .saturating_add(bounds.height)
            .saturating_sub(old_pane.rect.y)
            .max(min.height);

        let width = width.clamp(min.width, max_width);
        let height = height.clamp(min.height, max_height);

        let old_rect = old_pane.rect;
        let old_visible = old_pane.visible;

        let insets = self
            .decor
            .get(&pane_id)
            .map(PaneDecor::insets)
            .unwrap_or_default();

        let new_rect = Rect {
            x: old_rect.x,
            y: old_rect.y,
            width,
            height,
        };

        let new_content = Rect {
            x: new_rect.x.saturating_add(insets.left),
            y: new_rect.y.saturating_add(insets.top),
            width: new_rect.width.saturating_sub(insets.left + insets.right),
            height: new_rect.height.saturating_sub(insets.top + insets.bottom),
        };

        let Some(pane) = self.pane_mut(pane_id) else {
            return false;
        };

        if !pane.resize(new_rect, new_content) {
            return false;
        }

        let _ = pane;

        self.sync_decor(pane_id);

        if old_visible {
            self.mark_damaged(old_rect);
            self.mark_damaged(new_rect);
        }

        if focused {
            self.cursor = None;
        }

        true
    }

    /// Moves a pane to the specified origin, optionally clamping to the canvas.
    pub fn move_pane(&mut self, pane_id: PaneId, position: Point, clamp: bool) -> bool {
        let bounds = self.rect();

        let policy = self.policies.get(&pane_id).copied().unwrap_or_default();

        if !policy.movable {
            return false;
        }

        let Some(pane) = self.pane_mut(pane_id) else {
            return false;
        };

        let old_rect = pane.rect;
        let old_visible = pane.visible;

        let rect = old_rect.with_origin(position);
        let new_rect = if clamp { rect.clamp_to(bounds) } else { rect };

        if old_rect == new_rect {
            return false;
        }

        pane.rect = new_rect;

        let _ = pane;

        let _ = self.sync_content_rect(pane_id);

        if old_visible {
            self.mark_damaged(old_rect);
            self.mark_damaged(new_rect);
        }

        true
    }

    /// Marks a pane as hidden.
    pub fn hide_pane(&mut self, pane_id: PaneId) -> bool {
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

    /// Marks a pane as visible.
    pub fn show_pane(&mut self, pane_id: PaneId) -> bool {
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

    /// Toggles pane visibility.
    pub fn toggle_pane_visibility(&mut self, pane_id: PaneId) -> bool {
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

    /// Composes damaged regions, renders the differences, and writes the output.
    pub fn render<W: std::io::Write>(
        &mut self,
        compositor: &mut Compositor,
        renderer: &mut Renderer,
        out: &mut W,
    ) -> std::io::Result<()> {
        self.collect_damage();
        if self.forced_redraw {
            self.mark_damaged(self.rect());
            self.forced_redraw = false;
        }

        let has_damage = self.damaged.iter().any(|row| row.is_damaged());
        if has_damage {
            compositor.flatten(self.clear_glyph, self.panes.as_slice(), &self.damaged);
        }

        renderer.render(compositor, &self.damaged, self.cursor, out)?;

        for pane in self.panes.iter_mut() {
            pane.clear_damaged();
        }

        self.clear_damaged();
        Ok(())
    }

    /// Returns all pane ids owned by the canvas.
    pub fn pane_ids(&self) -> impl DoubleEndedIterator<Item = PaneId> + '_ {
        self.panes.iter().map(|pane| pane.id())
    }

    /// Applies `f` to a pane and returns its rect/visibility before and after.
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
        let Rect { width, height, .. } = self.rect();
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

    /// Marks a horizontal span on a single canvas row as damaged.
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

    /// Projects pane-local damage into canvas-space damage.
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

    /// Collects visible pane damage into the canvas damage buffer.
    fn collect_damage(&mut self) {
        let Size { width, height } = self.size;

        for pane in self.panes.iter() {
            if pane.visible {
                Self::project_pane_damage_to_canvas(&mut self.damaged, width, height, pane);
            }
        }
    }

    /// Removes all tracked damage from the pane.
    pub(crate) fn clear_damaged(&mut self) {
        for row in &mut self.damaged {
            row.clear();
        }
    }
}
