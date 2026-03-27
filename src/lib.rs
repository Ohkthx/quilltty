//! File: src/lib.rs

mod display;
mod terminal;

pub use display::{
    BorderKind, Canvas, Color, Compositor, Glyph, Pane, PaneBuilder, PaneElement, PaneHit, PaneId,
    Point, Rect, Renderer, Rune, Style,
};
pub use terminal::{Input, Terminal};
