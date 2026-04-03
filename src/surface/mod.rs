//! File: src/surface/mod.rs

mod backend;
mod canvas;
mod decor;
mod geometry;
mod glyph;
pub(crate) mod indexed_vec;
mod pane;
mod policy;

pub use backend::{Compositor, Layer, Renderer};
pub use canvas::{Canvas, HitTarget, PaneHit};
pub use decor::{Insets, PaneDecor, WindowDecor};
pub use geometry::{Point, Rect, Size};
pub use glyph::{BorderKind, Color, Glyph, Rune, Style};
pub use pane::{Pane, PaneBuilder, PaneElement, PaneId};
pub use policy::{PaneAction, PanePolicy};
