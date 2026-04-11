//! File: src/lib.rs

mod surface;
pub mod terminal;
pub mod ui;

/// Re-exports of the `crossterm` event types used by Quilltty's public API.
pub mod crossterm {
    /// Event types re-exported from `crossterm`.
    pub mod event {
        pub use ::crossterm::event::{
            Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
            MouseEventKind,
        };
    }
}

pub mod style {
    pub use crate::surface::{BorderKind, Color, Glyph, Rune, Style};
}

pub mod geom {
    pub use crate::surface::{Insets, Point, Rect, Size};
}

pub mod render {
    pub use crate::surface::{Compositor, Layer, Renderer};
}

pub mod pane {
    pub use crate::surface::{Pane, PaneAction, PaneBuilder, PaneElement, PaneHit, PaneId};
    pub use crate::surface::{PaneDecor, PanePolicy, WindowDecor};
}

pub mod prelude {
    pub use crate::crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    };
    pub use crate::geom::{Point, Rect, Size};
    pub use crate::style::{BorderKind, Color, Glyph, Style};
    pub use crate::terminal::{Input, Terminal, TerminalOptions};
    pub use crate::ui::{
        ButtonWidget, CheckboxWidget, InputWidget, LogWidget, PaneDragKind, PaneId, ProgressWidget,
        SliderWidget, StylableWidgetExt, StyledLine, StyledSpan, TextWidget, Ui, UiEvent, Widget,
        WidgetHit, WidgetId, WidgetLayout,
    };
}

pub use crate::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use terminal::{Input, Terminal, TerminalOptions};
pub use ui::{
    ButtonWidget, Canvas, CheckboxWidget, InputWidget, InteractionStyle, LogWidget, Pane,
    PaneBuilder, PaneDragKind, PaneId, ProgressWidget, SliderWidget, StylableWidgetExt, StyledLine,
    StyledSpan, TextWidget, Ui, UiEvent, Widget, WidgetAction, WidgetHit, WidgetId, WidgetLayout,
    WidgetStore,
};
