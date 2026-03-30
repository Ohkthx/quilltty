//! File: src/lib.rs

pub mod prelude {
    pub use crate::geom::{Point, Rect};
    pub use crate::style::{BorderKind, Color, Style};
    pub use crate::terminal::{Input, Terminal};
    pub use crate::ui::{
        ButtonWidget, Canvas, CheckboxWidget, HasInteractionStyle, InputWidget, LogWidget, Pane,
        PaneBuilder, PaneDragKind, PaneId, ProgressWidget, SliderWidget, TextWidget, Ui, UiEvent,
        Widget, WidgetHit, WidgetId, WidgetLayout, WidgetStore,
    };
}

pub use ui::{
    ButtonWidget, Canvas, CheckboxWidget, InputWidget, LogWidget, Pane, PaneBuilder, PaneDragKind,
    PaneId, ProgressWidget, SliderWidget, TextWidget, Ui, UiEvent, Widget, WidgetHit, WidgetId,
    WidgetLayout, WidgetStore,
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
