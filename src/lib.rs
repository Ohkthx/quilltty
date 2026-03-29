//! File: src/lib.rs

pub mod prelude {
    pub use crate::geom::{Point, Rect};
    pub use crate::style::{BorderKind, Color, Style};
    pub use crate::terminal::{Input, Terminal};
    pub use crate::ui::{
        ButtonWidget, Canvas, CheckboxWidget, InputWidget, Pane, PaneBuilder, PaneDragKind, PaneId,
        TextWidget, Ui, UiEvent, Widget, WidgetBuilder, WidgetHit, WidgetId, WidgetLayout,
        WidgetStore,
    };
}

pub use ui::{
    ButtonWidget, Canvas, CheckboxWidget, InputWidget, Pane, PaneBuilder, PaneDragKind, PaneId,
    TextWidget, Ui, UiEvent, Widget, WidgetBuilder, WidgetHit, WidgetId, WidgetLayout, WidgetStore,
};

mod surface;
pub mod terminal;
pub mod ui;

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
