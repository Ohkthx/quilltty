//! File: src/lib.rs

pub mod prelude {
    pub use crate::geom::{Point, Rect};
    pub use crate::style::{BorderKind, Color, Style};
    pub use crate::terminal::{Input, Terminal};
    pub use crate::ui::{Canvas, Pane, PaneBuilder, PaneId};
}

mod surface;
pub mod terminal;

pub mod ui {
    pub use crate::surface::{Canvas, Pane, PaneBuilder, PaneElement, PaneHit, PaneId};
}

pub mod style {
    pub use crate::surface::{BorderKind, Color, Glyph, Rune, Style};
}

pub mod geom {
    pub use crate::surface::{Point, Rect};
}

pub mod render {
    pub use crate::surface::{Compositor, Renderer};
}

pub use terminal::{Input, Terminal};
pub use ui::{Canvas, Pane, PaneBuilder, PaneId};
