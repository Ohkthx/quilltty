//! File: src/ui/mod.rs

mod runtime;
mod store;
pub mod widget;

pub use crate::surface::{Canvas, HitTarget, Pane, PaneBuilder, PaneElement, PaneHit, PaneId};

pub use runtime::{PaneDragKind, PointerDrag, Ui, UiEvent};
pub use store::{WidgetHit, WidgetId, WidgetLayout, WidgetStore};
pub use widget::{
    ButtonWidget, CheckboxWidget, InputWidget, InteractionStyle, LogWidget, ProgressWidget,
    SliderWidget, StylableWidgetExt, StyledLine, StyledSpan, TextWidget, Widget, WidgetAction,
    WidgetState,
};
