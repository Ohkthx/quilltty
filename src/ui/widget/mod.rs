//! File: src/ui/widget/mod.rs

mod builder;
mod store;
mod traits;

mod button;
mod checkbox;
mod input;
mod log;
mod progress;
mod slider;
mod text;

pub(crate) use traits::{resolve_patched_style, widget_render};

pub use builder::WidgetBuilder;
pub use button::ButtonWidget;
pub use checkbox::CheckboxWidget;
pub use input::InputWidget;
pub use log::LogWidget;
pub use progress::ProgressWidget;
pub use slider::SliderWidget;
pub use store::{WidgetHit, WidgetId, WidgetLayout, WidgetStore};
pub use text::{StyledLine, StyledSpan, TextWidget};
pub use traits::{
    InteractionStyle, RichInteractionStyle, RichStylableWidgetExt, StylableWidgetExt, Widget,
    WidgetAction, WidgetState,
};
