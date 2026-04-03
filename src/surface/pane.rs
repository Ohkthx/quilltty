//! File: src/surface/pane.rs

use crate::{
    Canvas,
    geom::{Point, Rect, Size},
    style::{BorderKind, Glyph, Style},
    surface::{
        backend::{DamagedRow, Layer},
        decor::PaneDecor,
        indexed_vec::Keyed,
        policy::PanePolicy,
    },
};

/// Unique identifier for individual panes.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub struct PaneId(pub(crate) u32);

/// Clickable elements within a `Pane`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum PaneElement {
    Title,
    Border,
    Content,
    Resize,
}

/// Builder for configuring and inserting a new `Pane` into the `Canvas`.
pub struct PaneBuilder<'a> {
    pub(crate) canvas: &'a mut Canvas, // Reference to the surface to write changes.
    pub(crate) rect: Rect,             // Position and size of the pane.
    pub(crate) z_layer: Layer,         // Z positioning and order it will be drawn.
    pub(crate) visible: bool,          // If true, `Pane` will render, otherwise it is hidden.

    pub(crate) policy: PanePolicy, // Policy for Pane.
    pub(crate) decor: PaneDecor,   // Decorations applied.
}

impl<'a> PaneBuilder<'a> {
    /// Assigns a position and dimensions.
    #[must_use]
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = rect;
        self
    }

    /// Assigns the priority and rendering position.
    #[must_use]
    pub fn layer(mut self, z_layer: impl Into<Layer>) -> Self {
        self.z_layer = z_layer.into();
        self
    }

    /// Assigns if the `Pane` will be visible or not.
    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Assigns the pane policy.
    #[must_use]
    pub fn policy(mut self, policy: PanePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Assigns the pane decoration.
    #[must_use]
    pub fn decor(mut self, decor: PaneDecor) -> Self {
        self.decor = decor;
        self
    }

    /// Assigns if the `Pane` will be movable or not.
    #[must_use]
    pub fn movable(mut self, movable: bool) -> Self {
        self.policy.movable = movable;
        self
    }

    /// Assigns if the `Pane` will be resizable or not.
    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.policy.resizable = resizable;
        self
    }

    /// Assigns if the `Pane` will be bordered or not.
    #[must_use]
    pub fn border(mut self, border: Option<BorderKind>) -> Self {
        if let PaneDecor::Window(window) = &mut self.decor {
            window.border = border;
        }
        self
    }

    /// Assigns if the `Pane` will have a specified border style.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        if let PaneDecor::Window(window) = &mut self.decor {
            window.border_style = style;
        }
        self
    }

    /// Assigns if the `Pane` will have a title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        if let PaneDecor::Window(window) = &mut self.decor {
            window.title = Some(title.into());
        }
        self
    }

    /// Builds the pane, assigns it a unique identifier, and inserts it into the canvas.
    pub fn build(self) -> PaneId {
        let canvas = self.canvas;
        let pane_id = if let Some(id) = canvas.freed_ids.pop() {
            id
        } else {
            PaneId(canvas.panes.len() as u32)
        };

        assert!(Size::from(self.rect) != Size::ZERO, "Pane size must be > 0");

        let pane = Pane::new(pane_id)
            .with_rect(self.rect)
            .with_z_layer(self.z_layer)
            .with_visibility(self.visible)
            .with_data(vec![Glyph::default(); self.rect.width * self.rect.height]);

        let idx = canvas
            .panes
            .as_slice()
            .partition_point(|p| p.z_layer <= pane.z_layer);

        canvas.panes.insert_at(idx, pane);
        canvas.policies.insert(pane_id, self.policy);
        canvas.decor.insert(pane_id, self.decor);
        canvas.sync_decor(pane_id);

        pane_id
    }
}

/// A drawable rectangular region with its own backing data and damage tracking.
pub struct Pane {
    pub(crate) id: PaneId,     // Unique identifier for the pane used for lookups.
    pub(crate) rect: Rect,     // Position (XY coordinates) and dimensions (Width x Height).
    pub(crate) z_layer: Layer, // Priority and rendering position.
    pub(crate) visible: bool,  // If true, `Pane` will render, otherwise it is hidden.

    pub(crate) content_rect: Rect, // Cached canvas-space content rect.
    pub(crate) data: Vec<Glyph>,   // Render information owned by the `Pane`.
    pub(crate) damaged: Vec<DamagedRow>, // Per-row spans used to track damage.
}

impl Pane {
    /// Constructs a new pane with defaults and the unique identifier.
    #[must_use]
    pub(crate) fn new(pane_id: PaneId) -> Self {
        Self {
            id: pane_id,
            rect: Rect::default(),
            z_layer: Layer::default(),
            visible: true,
            content_rect: Rect::default(),
            data: Vec::new(),
            damaged: Vec::new(),
        }
    }

    /// Assigns a position and dimensions.
    #[must_use]
    pub(crate) fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = rect;
        self
    }

    /// Assigns the priority and rendering position.
    #[must_use]
    pub(crate) fn with_z_layer(mut self, z_layer: impl Into<Layer>) -> Self {
        self.z_layer = z_layer.into();
        self
    }

    /// Assigns if the `Pane` will be visible or not.
    #[must_use]
    pub(crate) fn with_visibility(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Assigns the default data to be rendered.
    #[must_use]
    pub(crate) fn with_data(mut self, data: Vec<Glyph>) -> Self {
        debug_assert_eq!(data.len(), self.rect.width * self.rect.height);
        self.data = data;
        self.damaged = vec![DamagedRow::default(); self.rect.height];
        self.mark_all_damaged();
        self
    }

    /// Returns the unique identifier.
    pub fn id(&self) -> PaneId {
        self.id
    }

    /// Returns the pane rectangle in canvas space.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Returns the pane width.
    pub fn width(&self) -> usize {
        self.rect.width
    }

    /// Returns the pane height.
    pub fn height(&self) -> usize {
        self.rect.height
    }

    /// Returns the cached drawable content area in canvas space.
    pub fn content_rect(&self) -> Rect {
        self.content_rect
    }

    /// Updates the cached content rect.
    pub(crate) fn set_content_rect(&mut self, rect: Rect) {
        self.content_rect = rect;
    }

    /// Fills one pane-local row segment and marks one damaged span.
    fn raw_fill_row(&mut self, y: usize, x0: usize, x1: usize, glyph: Glyph) {
        if y >= self.rect.height {
            return;
        }

        let x0 = x0.min(self.rect.width);
        let x1 = x1.min(self.rect.width);

        if x0 >= x1 {
            return;
        }

        let start = Point::new(x0, y).as_index(self.rect.width);
        let end = start + (x1 - x0);
        let row = &mut self.data[start..end];

        let mut changed = false;
        for cell in row.iter_mut() {
            if *cell != glyph {
                *cell = glyph;
                changed = true;
            }
        }

        if changed {
            self.damaged[y].mark_range(x0, x1);
        }
    }

    /// Writes a glyph directly in pane-local coordinates for decoration rendering.
    pub(crate) fn decor_raw_set(&mut self, pos: Point, glyph: Glyph) {
        self.raw_set(pos, glyph);
    }

    /// Clears the specified pane-local row for decoration redraw.
    pub(crate) fn decor_clear_row(&mut self, y: usize, glyph: Glyph) {
        self.raw_fill_row(y, 0, self.rect.width, glyph);
    }

    /// Fills a content-local rectangle with one glyph using row-wise writes.
    pub fn fill(&mut self, rect: Rect, glyph: Glyph) {
        let content = self.content_rect();
        if rect.width == 0 || rect.height == 0 {
            return;
        }

        let x0 = rect.x.min(content.width);
        let y0 = rect.y.min(content.height);
        let x1 = rect.x.saturating_add(rect.width).min(content.width);
        let y1 = rect.y.saturating_add(rect.height).min(content.height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let inset_x = content.x - self.rect.x;
        let inset_y = content.y - self.rect.y;

        for y in y0..y1 {
            let local_y = inset_y + y;
            self.raw_fill_row(local_y, inset_x + x0, inset_x + x1, glyph);
        }
    }

    /// Writes a glyph in content-local coordinates.
    pub fn set(&mut self, pos: Point, glyph: impl Into<Glyph>) {
        let content = self.content_rect();
        let inset_x = content.x.saturating_sub(self.rect.x);
        let inset_y = content.y.saturating_sub(self.rect.y);

        if pos.x >= content.width || pos.y >= content.height {
            return;
        }

        self.raw_set(
            Point::new(inset_x.saturating_add(pos.x), inset_y.saturating_add(pos.y)),
            glyph.into(),
        );
    }

    /// Writes `text` on a single content row starting at `position` with `style`.
    #[inline]
    pub fn write_str(&mut self, position: Point, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }

        let glyphs: Vec<Glyph> = text
            .chars()
            .map(|ch| Glyph::from(ch).with_style(style))
            .collect();

        self.write_glyphs(position, &glyphs);
    }

    /// Writes a contiguous row of glyphs into content-local coordinates.
    pub fn write_glyphs(&mut self, position: Point, glyphs: &[Glyph]) {
        let content = self.content_rect();
        if position.y >= content.height || glyphs.is_empty() {
            return;
        }

        let inset_x = content.x - self.rect.x;
        let inset_y = content.y - self.rect.y;

        let available = content.width.saturating_sub(position.x);
        let len = glyphs.len().min(available);
        if len == 0 {
            return;
        }

        let local_x = inset_x + position.x;
        let local_y = inset_y + position.y;

        let start = Point::new(local_x, local_y).as_index(self.rect.width);
        let end = start + len;
        let dst = &mut self.data[start..end];

        let mut changed = false;
        for (d, s) in dst.iter_mut().zip(&glyphs[..len]) {
            if *d != *s {
                *d = *s;
                changed = true;
            }
        }

        if changed {
            self.damaged[local_y].mark_range(local_x, local_x + len);
        }
    }

    /// Marks the pane as hidden.
    pub(crate) fn hide(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        self.visible = false;
        true
    }

    /// Marks the pane as visible.
    pub(crate) fn show(&mut self) -> bool {
        if self.visible {
            return false;
        }

        self.visible = true;
        self.mark_all_damaged();
        true
    }

    /// Toggles pane visibility.
    pub(crate) fn toggle_visibility(&mut self) -> bool {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }

        self.visible
    }

    /// Resizes the pane backing storage and preserves overlapping content by row copy.
    pub(crate) fn resize(&mut self, rect: Rect, content_rect: Rect) -> bool {
        if rect.width == 0 || rect.height == 0 {
            return false;
        }

        if self.rect == rect && self.content_rect == content_rect {
            return false;
        }

        let old_rect = self.rect;
        let old_content = self.content_rect;
        let old_data = std::mem::take(&mut self.data);

        let old_offset = old_content.origin().saturating_sub(old_rect.origin());
        let new_offset = content_rect.origin().saturating_sub(rect.origin());

        let copy_width = old_content.width.min(content_rect.width);
        let copy_height = old_content.height.min(content_rect.height);

        let mut new_data = vec![Glyph::default(); rect.width * rect.height];

        for y in 0..copy_height {
            let old_y = old_offset.y + y;
            let new_y = new_offset.y + y;

            if old_y >= old_rect.height || new_y >= rect.height {
                continue;
            }

            // Copies one contiguous content row instead of copying cell-by-cell.
            let old_start = Point::new(old_offset.x, old_y).as_index(old_rect.width);
            let new_start = Point::new(new_offset.x, new_y).as_index(rect.width);

            let old_end = old_start + copy_width;
            let new_end = new_start + copy_width;

            if old_end <= old_data.len() && new_end <= new_data.len() {
                new_data[new_start..new_end].copy_from_slice(&old_data[old_start..old_end]);
            }
        }

        self.rect = rect;
        self.content_rect = content_rect;
        self.data = new_data;
        self.damaged = vec![DamagedRow::default(); rect.height];
        self.mark_all_damaged();

        true
    }

    /// Writes a glyph directly in pane-local coordinates.
    fn raw_set(&mut self, pos: Point, glyph: Glyph) {
        if pos.x >= self.rect.width || pos.y >= self.rect.height {
            return;
        }

        let index = pos.y.saturating_mul(self.rect.width).saturating_add(pos.x);
        if self.data[index] != glyph {
            self.data[index] = glyph;
            self.damaged[pos.y].mark_range(pos.x, pos.x.saturating_add(1));
        }
    }

    /// Removes all tracked damage from the pane.
    pub(crate) fn clear_damaged(&mut self) {
        for row in &mut self.damaged {
            row.clear();
        }
    }

    /// Marks the entire pane as damaged.
    pub(crate) fn mark_all_damaged(&mut self) {
        let Rect { width, height, .. } = self.rect;
        if width == 0 || height == 0 {
            return;
        }

        if self.damaged.len() != height {
            self.damaged.resize(height, DamagedRow::default());
        }

        for row in &mut self.damaged {
            row.mark_range(0, width);
        }
    }
}

impl Keyed<PaneId> for Pane {
    fn key(&self) -> &PaneId {
        &self.id
    }
}
