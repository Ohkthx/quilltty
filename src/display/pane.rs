//! File: src/display/pane.rs

use crate::{
    Canvas, Color, Glyph, Rect, Style,
    display::{
        Point,
        backend::{DamagedSpan, to_index},
        glyph::BorderKind,
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
    pub(crate) z_layer: u16,           // Z positioning and order it will be drawn.

    pub(crate) visible: bool, // If true, `Pane` will render, otherwise it is hidden.
    pub(crate) movable: bool, // If true, `Pane` can be moved.
    pub(crate) resizable: bool, // If true, `Pane` can be resized.

    pub(crate) border: Option<BorderKind>, // Marks if a border goes around the `Pane`.
    pub(crate) border_style: Style,        // Style for the border.

    pub(crate) title: Option<String>, // Optional title for the `Pane`.
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

    /// Assigns if the `Pane` will be movable or not.
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// Assigns if the `Pane` will be resizable or not.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Assigns if the `Pane` will be bordered or not.
    pub fn border(mut self, border: Option<BorderKind>) -> Self {
        self.border = border;
        self
    }

    /// Assigns if the `Pane` will have a specified border style.
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Assigns if the `Pane` will have a title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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
            .with_focus(false)
            .with_visibility(self.visible)
            .with_movability(self.movable)
            .with_resizability(self.resizable)
            .with_border(self.border)
            .with_border_style(self.border_style)
            .with_title(self.title)
            .with_data(vec![Glyph::default(); self.rect.width * self.rect.height])
            .build();

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

    pub(crate) focused: bool,   // If true, `Pane` will be marked as focused.
    pub(crate) visible: bool,   // If true, `Pane` will render, otherwise it is hidden.
    pub(crate) movable: bool,   // If true, `Pane` can be moved.
    pub(crate) resizable: bool, // If true, `Pane` can be resized.

    pub(crate) border: Option<BorderKind>, // Marks if a border goes around the `Pane`.
    pub(crate) border_style: Style,        // Marks the style for the border.

    pub(crate) title: Option<String>, // Optional title for the `Pane`.
    pub(crate) data: Vec<Glyph>,      // Render information owned by the `Pane`.
    pub(crate) damaged: Vec<DamagedSpan>, // Per-row spans used to track damage.
}

impl Pane {
    const TITLE_OFFSET: usize = 2; // Offset from the top-left side.

    /// Constructs a new pane with defaults and the unique identifier.
    #[must_use]
    pub(crate) fn new(pane_id: PaneId) -> Self {
        Self {
            id: pane_id,
            rect: Rect::default(),
            z_layer: 1,

            focused: false,
            visible: true,
            movable: true,
            resizable: true,

            border: None,
            border_style: Style::default().with_fg(Color::White),

            title: None,
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

    /// Assigns if the `Pane` will be movable or not.
    #[must_use]
    pub(crate) fn with_movability(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// Assigns if the `Pane` will be resizable or not.
    #[must_use]
    pub(crate) fn with_resizability(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Assigns if the `Pane` will be bordered or not.
    #[must_use]
    pub(crate) fn with_border(mut self, border: Option<BorderKind>) -> Self {
        self.border = border;
        self
    }

    /// Assigns if the `Pane` will have a specified border.
    pub fn with_border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Assigns if the `Pane` will have a title.
    #[must_use]
    pub(crate) fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    /// Assigns if the `Pane` will be focused.
    #[must_use]
    pub(crate) fn with_focus(mut self, is_focus: bool) -> Self {
        self.focused = is_focus;
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

    /// Performs final cleanup, setting any last touches.
    #[must_use]
    pub(crate) fn build(mut self) -> Self {
        self.draw_decorations();
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

    /// Returns the drawable content area of the pane, excluding borders and title space.
    pub fn content_rect(&self) -> Rect {
        let inset = usize::from(self.border.is_some());
        let title_rows = usize::from(self.title.is_some());
        let mut rect = self.rect;

        rect.x = rect.x.saturating_add(inset);
        rect.y = rect.y.saturating_add(inset.max(title_rows));
        rect.width = rect.width.saturating_sub(inset * 2);
        rect.height = rect.height.saturating_sub((inset * 2).max(title_rows));

        rect
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

    /// Current title of the `Pane` if it is set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Minimum width and height for the `Pane`.
    pub(crate) fn minimum_size(&self) -> Point {
        if self.border.is_some() {
            Point::new(3, 3)
        } else if self.title.is_some() {
            Point::new(1, 2)
        } else {
            Point::new(1, 1)
        }
    }

    /// Obtains the element at specific coordinates, returning None if outside the pane.
    pub(crate) fn element_at(&self, point: Point) -> Option<PaneElement> {
        let Rect { width, height, .. } = self.rect;
        let Point { x, y } = point;

        if x >= width || y >= height {
            return None;
        }

        let has_border = self.border.is_some();
        let has_title = self.title.is_some();

        let on_left = x == 0;
        let on_right = x + 1 == width;
        let on_top = y == 0;
        let on_bottom = y + 1 == height;

        // Bottom-right corner is the resize handle when bordered + resizable.
        if has_border && self.resizable && on_right && on_bottom {
            return Some(PaneElement::Resize);
        }

        // Treat the header row as title space.
        //
        // Bordered panes: the top row excluding the two corners.
        // Borderless panes: the full first row.
        if has_title {
            if has_border {
                if on_top && !on_left && !on_right {
                    return Some(PaneElement::Title);
                }
            } else if on_top {
                return Some(PaneElement::Title);
            }
        }

        if has_border && (on_left || on_right || on_top || on_bottom) {
            return Some(PaneElement::Border);
        }

        Some(PaneElement::Content)
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

    fn set_local(&mut self, x: usize, y: usize, glyph: Glyph) {
        let Rect { width, height, .. } = self.rect;
        debug_assert!(x < width && y < height);

        let index = to_index(x, y, width);
        if self.data[index] != glyph {
            self.data[index] = glyph;
            self.damaged[y].mark(x);
        }
    }

    fn raw_set(&mut self, x: usize, y: usize, glyph: Glyph) {
        self.set_local(x, y, glyph);
    }

    /// Writes a glyph at `(x, y)` if it lies within the pane. Expects the coordinates supplied to
    /// be local to the `Pane` scope.
    pub fn set(&mut self, x: usize, y: usize, glyph: impl Into<Glyph>) {
        let content = self.content_rect();
        let inset_x = content.x.saturating_sub(self.rect.x);
        let inset_y = content.y.saturating_sub(self.rect.y);

        if x >= content.width || y >= content.height {
            return;
        }

        let px = inset_x.saturating_add(x);
        let py = inset_y.saturating_add(y);

        self.raw_set(px, py, glyph.into());
    }

    /// Updates the title of the `Pane`.
    pub(crate) fn set_title(&mut self, title: impl Into<Option<String>>) {
        self.title = title.into();
        self.redraw_header();
    }

    /// Writes `text` on a single content row starting at `(x, y)` with `style`.
    pub fn write_str(&mut self, x: usize, y: usize, text: &str, style: Style) {
        let content = self.content_rect();
        let offset_x = content.x.saturating_sub(self.rect.x);
        let offset_y = content.y.saturating_sub(self.rect.y);

        if y >= content.height {
            return;
        }

        let pane_y = offset_y + y;
        for (dx, ch) in text.chars().enumerate() {
            let content_x = x + dx;
            if content_x >= content.width {
                break;
            }

            let pane_x = offset_x + content_x;
            self.raw_set(pane_x, pane_y, Glyph::new().with_rune(ch).with_style(style));
        }
    }

    /// Marks the `Pane` as being focused.
    pub(crate) fn set_focus(&mut self, is_focus: bool) {
        if self.focused == is_focus {
            return;
        }

        self.focused = is_focus;
        self.draw_decorations();
    }

    /// Resizes the `Pane` to the dimensions provided.
    pub(crate) fn resize(&mut self, width: usize, height: usize) {
        let min = self.minimum_size();
        if width < min.x || height < min.y {
            return;
        }

        if width == self.rect.width && height == self.rect.height {
            return;
        }

        let old_rect = self.rect;
        let old_content = self.content_rect();
        let old_offset = Point::new(
            old_content.x.saturating_sub(old_rect.x),
            old_content.y.saturating_sub(old_rect.y),
        );

        let old_data = std::mem::take(&mut self.data);

        self.rect.width = width;
        self.rect.height = height;

        let new_content = self.content_rect();
        let new_offset = Point::new(
            new_content.x.saturating_sub(self.rect.x),
            new_content.y.saturating_sub(self.rect.y),
        );

        let copy_width = old_content.width.min(new_content.width);
        let copy_height = old_content.height.min(new_content.height);

        let mut new_data = vec![Glyph::default(); width * height];

        for y in 0..copy_height {
            for x in 0..copy_width {
                let old_idx = to_index(old_offset.x + x, old_offset.y + y, old_rect.width);
                let new_idx = to_index(new_offset.x + x, new_offset.y + y, width);
                new_data[new_idx] = old_data[old_idx];
            }
        }

        self.data = new_data;
        self.damaged = vec![DamagedSpan::default(); height];

        self.draw_decorations();
        self.mark_all_damaged();
    }

    /// Fills the pane content area with `glyph`.
    pub fn fill(&mut self, glyph: Glyph) {
        let content = self.content_rect();
        let offset_x = content.x.saturating_sub(self.rect.x);
        let offset_y = content.y.saturating_sub(self.rect.y);
        let pane_width = self.rect.width;

        if content.width == 0 || content.height == 0 {
            return;
        }

        for y in 0..content.height {
            let pane_y = offset_y + y;
            let row_start = to_index(offset_x, pane_y, pane_width);
            let row = &mut self.data[row_start..row_start + content.width];

            let mut changed = false;
            for cell in row.iter_mut() {
                if *cell != glyph {
                    *cell = glyph;
                    changed = true;
                }
            }

            if changed {
                self.damaged[pane_y].mark_range(offset_x, offset_x + content.width - 1);
            }
        }
    }

    /// Resets the pane contents to the default glyph.
    pub fn clear(&mut self) {
        self.fill(Glyph::default());
    }

    /// Marks the entire pane as damaged.
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

    /// Remove all damage.
    pub(crate) fn clear_damaged(&mut self) {
        for span in &mut self.damaged {
            span.clear();
        }
    }

    /// Draws the border around the pane.
    fn draw_border(&mut self) {
        let Some(kind) = self.border else {
            return;
        };

        let Rect { width, height, .. } = self.rect;
        if width < 2 || height < 2 {
            return;
        }

        let style = if self.focused {
            self.border_style.with_fg(Color::Red).bold()
        } else {
            self.border_style
        };

        let horizontal = Glyph::from(kind.horizontal()).with_style(style);
        let vertical = Glyph::from(kind.vertical()).with_style(style);
        let top_left = Glyph::from(kind.top_left()).with_style(style);
        let top_right = Glyph::from(kind.top_right()).with_style(style);
        let bottom_left = Glyph::from(kind.bottom_left()).with_style(style);
        let bottom_right = if self.resizable {
            Glyph::from(kind.cross()).with_style(style)
        } else {
            Glyph::from(kind.bottom_right()).with_style(style)
        };

        self.raw_set(0, 0, top_left);
        self.raw_set(width - 1, 0, top_right);
        self.raw_set(0, height - 1, bottom_left);
        self.raw_set(width - 1, height - 1, bottom_right);

        for x in 1..width - 1 {
            self.raw_set(x, 0, horizontal);
            self.raw_set(x, height - 1, horizontal);
        }

        for y in 1..height - 1 {
            self.raw_set(0, y, vertical);
            self.raw_set(width - 1, y, vertical);
        }
    }

    /// Draws the title on the top of the pane.
    fn draw_title(&mut self) {
        let Some(title) = self.title.clone() else {
            return;
        };

        let width = self.rect.width;
        if width == 0 {
            return;
        }

        let style = Style::new().with_fg(Color::White).bold();
        let y = 0;
        let space = Glyph::from(' ').with_style(style);

        if self.border.is_some() {
            if width <= 2 {
                return;
            }

            let mut x = Self::TITLE_OFFSET;
            self.raw_set(x, y, space);
            x += 1;

            let max_title_width = width.saturating_sub(4);
            for ch in title.chars().take(max_title_width) {
                self.raw_set(x, y, Glyph::from(ch).with_style(style));
                x += 1;
            }

            if x < width - 1 {
                self.raw_set(x, y, space);
            }
        } else {
            for (x, ch) in title.chars().take(width).enumerate() {
                self.raw_set(x, y, Glyph::from(ch).with_style(style));
            }
        }
    }

    /// Redraws the title / top border.
    fn redraw_header(&mut self) {
        // Clear the top row first.
        if self.rect.height == 0 || self.rect.width == 0 {
            return;
        }

        if self.border.is_some() {
            self.draw_border();
        } else {
            for x in 0..self.rect.width {
                self.raw_set(x, 0, Glyph::default());
            }
        }

        if self.title.is_some() {
            self.draw_title();
        }
    }

    /// Draws all decorations for the pane.
    fn draw_decorations(&mut self) {
        if self.border.is_some() {
            self.draw_border();
        }

        if self.title.is_some() {
            self.draw_title();
        }
    }
}
