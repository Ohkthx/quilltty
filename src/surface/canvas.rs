//! File: src/surface/canvas.rs

use crate::{
    Pane, PaneBuilder, PaneId,
    geom::{Point, Rect, Size},
    render::{Compositor, Renderer},
    style::{Color, Glyph, Style},
    surface::{backend::DamagedRow, indexed_vec::IndexedVec},
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
    size: Size,                                 // Size of the canvas.
    clear_glyph: Glyph,                         // Glyph used to clear uncovered cells.
    forced_redraw: bool,                        // Forces redraw next render.
    pub(crate) panes: IndexedVec<PaneId, Pane>, // Visible/hidden child panes.
    pub(crate) damaged: Vec<DamagedRow>,        // Damaged spans in canvas space.
    pub(crate) freed_ids: Vec<PaneId>,          // Reusable PaneIds.
    pub(crate) cursor: Option<Point>,           // Cursor position on the canvas.
    pub(crate) focused: Option<PaneId>,         // Currently focused pane.
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
            damaged: vec![DamagedRow::default(); size.height],
            freed_ids: Vec::new(),
            cursor: None,
            focused: None,
        }
    }

    /// Creates a new pane builder using the current canvas rect as the default rect.
    pub fn create_pane(&mut self) -> PaneBuilder<'_> {
        let rect = self.rect();
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

    /// Obtains an immutable pane from the managed pane list.
    pub fn pane(&self, pane_id: PaneId) -> Option<&Pane> {
        self.panes.get(&pane_id)
    }

    /// Obtains a mutable pane from the managed pane list.
    pub fn pane_mut(&mut self, pane_id: PaneId) -> Option<&mut Pane> {
        self.panes.get_mut(&pane_id)
    }

    /// Sets the cursor to specific coordinates on the canvas.
    pub fn set_cursor(&mut self, cursor: Option<Point>) {
        self.cursor = cursor;
    }

    /// Sets a pane title and marks the affected title row span as damaged.
    pub fn set_pane_title(&mut self, pane_id: PaneId, title: Option<String>) -> bool {
        let focused = self.focused == Some(pane_id);
        let Some((rect, visible, _, _, _)) =
            self.with_pane_state_change(pane_id, |pane| pane.set_title(title, focused))
        else {
            return false;
        };

        if visible && rect.width > 0 {
            let width = self.size.width;
            let y = rect.y;
            let x0 = rect.x;
            let x1 = rect.x.saturating_add(rect.width);
            Self::mark_canvas_span_in(&mut self.damaged, width, y, x0, x1);
        }

        true
    }

    /// Returns the top-most target at the given canvas position.
    pub fn hit_at(&self, position: Point) -> HitTarget {
        for pane in self.panes.iter().rev() {
            if !pane.visible || !pane.rect.contains_point(position) {
                continue;
            }

            let pane_origin = pane.rect.origin();
            let content_origin = pane.content_rect().origin();
            let local = position.saturating_sub(pane_origin);

            if let Some(element) = pane.element_at(local) {
                let content_local = if element == PaneElement::Content {
                    Some(position.saturating_sub(content_origin))
                } else {
                    None
                };

                return HitTarget::Pane {
                    pane_id: pane.id,
                    hit: PaneHit {
                        element,
                        global: position,
                        local,
                        content_local,
                    },
                };
            }
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

        if let Some(old_id) = old_id
            && let Some(old) = self.pane_mut(old_id)
        {
            old.draw_decorations(false);
        }

        if let Some(new_id) = pane_id
            && let Some(new) = self.pane_mut(new_id)
        {
            new.draw_decorations(true);
        }

        self.cursor = None;
    }

    /// Resizes a pane, clamped to the canvas bounds and pane minimum size.
    pub fn resize_pane(&mut self, pane_id: PaneId, width: usize, height: usize) -> bool {
        let bounds = self.rect();
        let focused = self.focused == Some(pane_id);

        let Some((old_rect, old_visible, new_rect, _, _)) =
            self.with_pane_state_change(pane_id, |pane| {
                if !pane.resizable {
                    return;
                }

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

                pane.resize(width.min(max_width), height.min(max_height), focused);
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

    /// Moves a pane to the specified origin, optionally clamping to the canvas.
    pub fn move_pane(&mut self, pane_id: PaneId, position: Point, clamp: bool) -> bool {
        let bounds = self.rect();

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

        self.clear_damage();
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

    /// Clears the accumulated canvas damage buffer.
    fn clear_damage(&mut self) {
        for row in &mut self.damaged {
            row.clear();
        }
    }
}
