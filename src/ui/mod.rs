//! File: src/ui/mod.rs

mod runtime;
pub mod widget;

pub(crate) use widget::{merge_style, widget_render};

pub use crate::surface::{Canvas, HitTarget, Pane, PaneBuilder, PaneElement, PaneHit, PaneId};

pub use runtime::{PaneDragKind, PointerDrag, Ui, UiEvent};
pub use widget::{
    ButtonWidget, CheckboxWidget, InputWidget, InteractionStyle, LogWidget, ProgressWidget,
    SliderWidget, StylableWidgetExt, StyledLine, StyledSpan, TextWidget, Widget, WidgetAction,
    WidgetBuilder, WidgetHit, WidgetId, WidgetLayout, WidgetState, WidgetStore,
};
