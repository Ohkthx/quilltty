//! File: src/display/mod.rs

mod backend;
mod canvas;
mod geometry;
mod glyph;
mod pane;

pub use backend::{Compositor, Renderer};
pub use canvas::Canvas;
pub use geometry::Rect;
pub use glyph::{BorderKind, Color, Glyph, Rune, Style};
pub use pane::{Pane, PaneBuilder, PaneId};
