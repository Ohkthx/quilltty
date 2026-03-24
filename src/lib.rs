//! File: src/lib.rs

mod display;
mod terminal;

pub use display::{
    Canvas, Color, Compositor, Glyph, Pane, PaneBuilder, PaneId, Rect, Renderer, Rune, Style,
};
pub use terminal::{Input, Terminal};
