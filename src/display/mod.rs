//! File: src/display/mod.rs

mod backend;
mod canvas;
mod geometry;
mod glyph;
mod indexed_vec;
mod pane;

pub use backend::{Compositor, Renderer};
pub use canvas::{Canvas, PaneHit};
pub use geometry::{Point, Rect};
pub use glyph::{BorderKind, Color, Glyph, Rune, Style};
pub use pane::{Pane, PaneBuilder, PaneElement, PaneId};
