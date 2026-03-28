//! File: src/ui/mod.rs

mod store;
pub mod widget;

pub use crate::surface::{Canvas, Pane, PaneBuilder, PaneElement, PaneHit, PaneId};

pub use store::{WidgetBuilder, WidgetHit, WidgetId, WidgetStore};
pub use widget::{ButtonWidget, InputWidget, Widget};
