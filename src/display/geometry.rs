//! File: src/display/geometry.rs

/// A 2D XY-coordinate within a space.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    /// X-coordinate (# of columns) within a space.
    pub x: usize,
    /// Y-coordinate (# of rows) within a space.
    pub y: usize,
}

impl Point {
    /// The origin point `(0, 0)`.
    pub const ZERO: Self = Self { x: 0, y: 0 };

    /// Creates a new instance of a `Point`.
    #[inline]
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    /// Returns this point as a tuple.
    #[inline]
    pub const fn into_tuple(self) -> (usize, usize) {
        (self.x, self.y)
    }

    /// Returns a copy of this point with a different `x`.
    #[inline]
    pub const fn with_x(self, x: usize) -> Self {
        Self { x, ..self }
    }

    /// Returns a copy of this point with a different `y`.
    #[inline]
    pub const fn with_y(self, y: usize) -> Self {
        Self { y, ..self }
    }

    /// Returns a new point offset by the given delta.
    #[inline]
    pub const fn offset(self, dx: usize, dy: usize) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    /// Returns a new point offset by the given delta, saturating on overflow.
    #[inline]
    pub const fn saturating_offset(self, dx: usize, dy: usize) -> Self {
        Self::new(self.x.saturating_add(dx), self.y.saturating_add(dy))
    }

    /// Returns a new point after saturating subtraction.
    #[inline]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self::new(
            self.x.saturating_sub(other.x),
            self.y.saturating_sub(other.y),
        )
    }

    /// Returns the component-wise minimum.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    /// Returns the component-wise maximum.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }
}

impl From<(usize, usize)> for Point {
    #[inline]
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(u16, u16)> for Point {
    fn from((x, y): (u16, u16)) -> Self {
        Self::new(x as usize, y as usize)
    }
}

impl From<Point> for (usize, usize) {
    #[inline]
    fn from(value: Point) -> Self {
        (value.x, value.y)
    }
}

impl std::ops::Add<Point> for Point {
    type Output = Point;

    #[inline]
    fn add(self, rhs: Point) -> Self::Output {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::AddAssign<Point> for Point {
    #[inline]
    fn add_assign(&mut self, rhs: Point) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub<Point> for Point {
    type Output = Point;

    #[inline]
    fn sub(self, rhs: Point) -> Self::Output {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::SubAssign<Point> for Point {
    #[inline]
    fn sub_assign(&mut self, rhs: Point) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

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
    /// Assigns the XY-coordinates to the `Point`.
    #[must_use]
    pub fn with_origin(mut self, origin: Point) -> Self {
        self.x = origin.x;
        self.y = origin.y;
        self
    }

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

    /// XY Coordinates for the `Rect`.
    #[inline]
    pub const fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Checks whether the point `(x, y)` lies within this `Rect`.
    #[inline]
    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.x
            && x < self.x.saturating_add(self.width)
            && y >= self.y
            && y < self.y.saturating_add(self.height)
    }

    /// Checks if a `Point` is within the `Rect`.
    #[inline]
    pub fn contains_point(&self, point: Point) -> bool {
        self.contains(point.x, point.y)
    }

    /// Center point of the Rect.
    #[inline]
    pub fn center(&self) -> Point {
        Point::new(
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
