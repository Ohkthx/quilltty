//! File: src/ui/mod.rs

#[macro_use]
mod macros;

mod runtime;
mod store;
mod traits;
pub mod widget;

pub use crate::surface::{Canvas, HitTarget, Pane, PaneBuilder, PaneElement, PaneHit, PaneId};

pub use runtime::{PaneDragKind, Ui, UiEvent};
pub use store::{WidgetHit, WidgetId, WidgetLayout, WidgetStore};
pub use traits::{HasInteractionStyle, WidgetRender};
pub use widget::{
    ButtonWidget, CheckboxWidget, InputWidget, InteractionStyle, LogWidget, ProgressWidget,
    SliderWidget, StyledLine, StyledSpan, TextWidget, Widget, WidgetAction,
};
