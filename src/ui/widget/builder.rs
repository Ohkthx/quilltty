//! File: src/ui/widget/builder.rs

use super::{
    Widget,
    store::{WidgetId, WidgetLayout, WidgetStore},
};
use crate::{pane::PaneId, render::Layer};

/// Builder for configuring and inserting a new widget into a pane.
pub struct WidgetBuilder<'a, W>
where
    W: Widget + 'static,
{
    pub(crate) store: &'a mut WidgetStore, // Widget store receiving the widget.
    pub(crate) pane_id: PaneId,            // Parent pane for the widget.
    pub(crate) widget: W,                  // Widget being inserted.
    pub(crate) layout: Option<WidgetLayout>, // Bounds for the widget.
    pub(crate) z_layer: Layer,             // Rendering order for the widget.
    pub(crate) visible: bool,              // Whether the widget is visible.
    pub(crate) enabled: bool,              // Whether the widget is interactive.
}

impl<'a, W> WidgetBuilder<'a, W>
where
    W: Widget + 'static,
{
    /// Assigns the widget layout.
    #[must_use]
    pub fn layout(mut self, layout: WidgetLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Assigns the widget rendering layer.
    #[must_use]
    pub fn layer(mut self, z_layer: impl Into<Layer>) -> Self {
        self.z_layer = z_layer.into();
        self
    }

    /// Assigns whether the widget is visible.
    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Assigns whether the widget is enabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Builds the widget and inserts it into the widget store.
    pub fn build(self) -> WidgetId {
        let layout = self
            .layout
            .expect("Widget layout must be assigned before build()");

        self.store.insert_widget(
            self.pane_id,
            self.z_layer,
            self.visible,
            self.enabled,
            self.widget,
            layout,
        )
    }
}
