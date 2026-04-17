//! File: src/surface/mod.rs

mod backend;
mod canvas;
pub(crate) mod color;
mod decor;
mod geometry;
mod glyph;
pub(crate) mod indexed_vec;
mod pane;
mod policy;
pub(crate) mod style;

pub use backend::{Compositor, Layer, Renderer};
pub use canvas::{Canvas, HitTarget, PaneHit};
pub use color::{Color, ColorAtlas, ColorPair, ColorSpec};
pub use decor::{Insets, PaneDecor, WindowDecor};
pub use geometry::{Point, Rect, Size};
pub use glyph::{BorderKind, Glyph, Rune};
pub use pane::{Pane, PaneBuilder, PaneElement, PaneId};
pub use policy::{PaneAction, PanePolicy};
pub use style::{Style, StylePatch};
