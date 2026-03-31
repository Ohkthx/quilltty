//! File: src/surface/mod.rs

mod backend;
mod canvas;
mod geometry;
mod glyph;
pub(crate) mod indexed_vec;
mod pane;

pub use backend::{Compositor, Renderer};
pub use canvas::{Canvas, HitTarget, PaneHit};
pub use geometry::{Point, Rect, Size};
pub use glyph::{BorderKind, Color, Glyph, Rune, Style};
pub use pane::{Pane, PaneBuilder, PaneElement, PaneId};
