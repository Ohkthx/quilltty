//! File: src/display/geometry.rs

/// Bounds of an area including position that is based off of a top-left coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rect {
    /// Leftmost X-coordinate (column).
    pub x: usize,
    /// Topmost Y-coordinate (row).
    pub y: usize,
    /// Width extending right.
    pub width: usize,
    /// Height extending down.
    pub height: usize,
}

impl Rect {
    /// Assigns the position for the `Rect`.
    #[must_use]
    pub fn position(mut self, x: usize, y: usize) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Assigns the width in columns.
    #[must_use]
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Assigns the height in rows.
    #[must_use]
    pub fn height(mut self, height: usize) -> Self {
        self.height = height;
        self
    }

    /// Returns a new `Rect` centered on `(x, y)`.
    pub fn center_on(mut self, x: usize, y: usize) -> Self {
        self.x = x.saturating_sub(self.width / 2);
        self.y = y.saturating_sub(self.height / 2);
        self
    }

    /// Returns a new `Rect` clamped to the given bounds.
    pub fn clamp_to(mut self, bounds: Rect) -> Self {
        let max_x = bounds
            .x
            .saturating_add(bounds.width.saturating_sub(self.width));
        let max_y = bounds
            .y
            .saturating_add(bounds.height.saturating_sub(self.height));
        self.x = self.x.clamp(bounds.x, max_x);
        self.y = self.y.clamp(bounds.y, max_y);
        self
    }

    /// Checks whether the point `(x, y)` lies within this `Rect`.
    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.x
            && x < self.x.saturating_add(self.width)
            && y >= self.y
            && y < self.y.saturating_add(self.height)
    }

    /// Center point of the Rect.
    pub fn center(&self) -> (usize, usize) {
        (
            self.x.saturating_add(self.width / 2),
            self.y.saturating_add(self.height / 2),
        )
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        }
    }
}
