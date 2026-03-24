//! File: src/display/pane.rs

use crate::{
    Canvas, Glyph, Rect, Style,
    display::backend::{DamagedSpan, to_index},
};

/// Unique identifier for individual panes.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub struct PaneId(pub(crate) u32);

/// Builder for configuring and inserting a new `Pane` into the `Canvas`.
pub struct PaneBuilder<'a> {
    pub(crate) canvas: &'a mut Canvas, // Reference to the surface to write changes.
    pub(crate) rect: Rect,             // Position and size of the pane.
    pub(crate) z_layer: u16,           // Z positioning and order it will be drawn.

    pub(crate) visible: bool, // If true, `Pane` will render, otherwise it is hidden.
}

impl<'a> PaneBuilder<'a> {
    /// Assigns a position and dimensions.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = rect;
        self
    }

    /// Assigns the priority and rendering position.
    pub fn layer(mut self, z_layer: u16) -> Self {
        self.z_layer = z_layer;
        self
    }

    /// Assigns if the `Pane` will be visible or not.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
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

        assert!(
            pane_id != Canvas::ROOT_ID,
            "Cannot assign Root Pane identifier"
        );
        assert!(
            self.rect.width > 0 && self.rect.height > 0,
            "Pane size must be > 0"
        );

        let pane = Pane::new(pane_id)
            .with_rect(self.rect)
            .with_z_layer(self.z_layer)
            .with_visibility(self.visible)
            .with_data(vec![Glyph::default(); self.rect.width * self.rect.height]);

        let idx = canvas.panes.partition_point(|p| p.z_layer <= pane.z_layer);
        canvas.panes.insert(idx, pane);
        pane_id
    }
}

/// A drawable rectangular region with its own backing data and damage tracking.
pub struct Pane {
    pub(crate) id: PaneId,   // Unique identifier for the pane used for lookups.
    pub(crate) rect: Rect,   // Position (XY coordinates) and dimensions (Width x Height).
    pub(crate) z_layer: u16, // Priority and rendering position.

    pub(crate) visible: bool, // If true, `Pane` will render, otherwise it is hidden.

    pub(crate) data: Vec<Glyph>, // Render information owned by the `Pane`.
    pub(crate) damaged: Vec<DamagedSpan>, // Per-row spans used to track damage.
}

impl Pane {
    /// Constructs a new pane with defaults and the unique identifier.
    #[must_use]
    pub(crate) fn new(pane_id: PaneId) -> Self {
        Self {
            id: pane_id,
            rect: Rect::default(),
            z_layer: 1,

            visible: true,

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
    pub(crate) fn with_z_layer(mut self, z_layer: u16) -> Self {
        self.z_layer = z_layer;
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
        self.damaged = vec![DamagedSpan::default(); self.rect.height];
        self.mark_all_damaged();
        self
    }

    /// Unique identifier.
    pub fn id(&self) -> PaneId {
        self.id
    }

    /// Position (XY coordinate) with dimensions (Width x Height).
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Length in columns.
    pub fn width(&self) -> usize {
        self.rect.width
    }

    /// Length in rows.
    pub fn height(&self) -> usize {
        self.rect.height
    }

    /// Priority and rendering position.
    pub fn layer(&self) -> u16 {
        self.z_layer
    }

    /// If false, the `Pane` will not render.
    pub fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn hide(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        self.visible = false;
        true
    }

    pub(crate) fn show(&mut self) -> bool {
        if self.visible {
            return false;
        }

        self.visible = true;
        self.mark_all_damaged();
        true
    }

    pub(crate) fn toggle_visibility(&mut self) -> bool {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }

        self.visible
    }

    /// Writes a glyph at `(x, y)` if it lies within the pane.
    pub fn set(&mut self, x: usize, y: usize, glyph: impl Into<Glyph>) {
        let Rect { width, height, .. } = self.rect;
        if x >= width || y >= height {
            return;
        }

        let glyph = glyph.into();
        let idx = to_index(x, y, width);

        if self.data[idx] != glyph {
            self.data[idx] = glyph;
            self.damaged[y].mark(x);
        }
    }

    /// Writes `text` on a single row starting at `(x, y)` with `style`.
    pub fn write_str(&mut self, x: usize, y: usize, text: &str, style: Style) {
        let Rect { width, height, .. } = self.rect;
        if y >= height {
            return;
        }

        for (dx, ch) in text.chars().enumerate() {
            let px = x + dx;
            if px >= width {
                break;
            }

            self.set(px, y, Glyph::new().with_rune(ch).with_style(style));
        }
    }

    /// Fills the pane with `glyph`.
    pub fn fill(&mut self, glyph: Glyph) {
        let Rect { width, height, .. } = self.rect;
        if width == 0 || height == 0 {
            return;
        }

        for y in 0..height {
            let row_start = to_index(0, y, width);
            let row = &mut self.data[row_start..row_start + width];

            let mut changed = false;
            for cell in row.iter_mut() {
                if *cell != glyph {
                    *cell = glyph;
                    changed = true;
                }
            }

            if changed {
                self.damaged[y].mark_range(0, width - 1);
            }
        }
    }

    /// Resets the pane contents to the default glyph.
    pub fn clear(&mut self) {
        self.fill(Glyph::default());
    }

    pub(crate) fn mark_all_damaged(&mut self) {
        let Rect { width, height, .. } = self.rect;
        if width == 0 || height == 0 {
            return;
        }

        if self.damaged.len() != height {
            self.damaged.resize(height, DamagedSpan::default());
        }

        for span in &mut self.damaged {
            span.mark_range(0, width - 1);
        }
    }

    pub(crate) fn clear_damaged(&mut self) {
        for span in &mut self.damaged {
            span.clear();
        }
    }
}
