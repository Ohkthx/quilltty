macro_rules! widget_types {
    (
        $(
            $method:ident => $variant:ident($ty:ty)
        ),* $(,)?
    ) => {
        /// Type-erased `Widget` wrapper.
        pub enum Widget {
            $(
                $variant($ty),
            )*
        }

        impl Widget {
            /// Returns the cursor position for this widget within the given pane.
            pub fn cursor_pos(&self, pane: &Pane, rect: Rect) -> Option<Point> {
                match self {
                    $(
                        Self::$variant(widget) => widget.cursor_pos(pane, rect),
                    )*
                }
            }

            /// Renders this widget into the given pane within `rect`.
            pub fn render(&mut self, pane: &mut Pane, rect: Rect) {
                match self {
                    $(
                        Self::$variant(widget) => widget.render(pane, rect),
                    )*
                }
            }

            /// Updates the hovered state for this widget.
            pub fn set_hovered(&mut self, value: bool) {
                match self {
                    $(
                        Self::$variant(w) => w.set_hovered(value),
                    )*
                }
            }

            /// Updates the pressed state for this widget.
            pub fn set_pressed(&mut self, value: bool) {
                match self {
                    $(
                        Self::$variant(w) => w.set_pressed(value),
                    )*
                }
            }

            /// Updates the focused state for this widget.
            pub fn set_focused(&mut self, value: bool) {
                match self {
                    $(
                        Self::$variant(w) => w.set_focused(value),
                    )*
                }
            }

            /// Updates the damaged state for this widget.
            pub fn set_damaged(&mut self, damaged: bool) {
                match self {
                    $(
                        Self::$variant(w) => w.set_damaged(damaged),
                    )*
                }
            }

            /// Produces the action for keyboard activation (Enter / Space).
            pub fn activate_action(&mut self) -> WidgetAction {
                match self {
                    $(
                        Self::$variant(w) => w.activate_action(),
                    )*
                }
            }

            /// Produces the action for direct keyboard input.
            pub fn key_action(&mut self, key: KeyCode) -> WidgetAction {
                match self {
                    $(
                        Self::$variant(w) => w.key_action(key),
                    )*
                }
            }

            /// Produces the action for pointer dragging within the widget.
            pub fn drag_action(&mut self, local_x: usize, width: usize) -> WidgetAction {
                match self {
                    $(
                        Self::$variant(w) => w.drag_action(local_x, width),
                    )*
                }
            }

            /// Produces the action for pointer release.
            pub fn release_action(&mut self, focused: bool) -> WidgetAction {
                match self {
                    $(
                        Self::$variant(w) => w.release_action(focused),
                    )*
                }
            }
        }

        $(
            impl From<$ty> for Widget {
                fn from(value: $ty) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

macro_rules! impl_widget_store_editors {
    (
        $(
            $method:ident => $variant:ident($ty:ty)
        ),* $(,)?
    ) => {
        $(
            #[doc = concat!("Edits a ", stringify!($method), " widget by its `WidgetId`.")]
            pub fn $method<R>(
                &mut self,
                widget_id: WidgetId,
                f: impl FnOnce(&mut $ty) -> R,
            ) -> Option<R> {
                self.edit(widget_id, |widget| match widget {
                    Widget::$variant(inner) => Some(f(inner)),
                    _ => None,
                })
                .flatten()
            }
        )*
    };
}

macro_rules! impl_ui_widget_accessors {
    (
        $(
            $getter:ident, $editor:ident => $variant:ident($ty:ty)
        ),* $(,)?
    ) => {
        $(
            #[doc = concat!(
                "Returns the ",
                stringify!($getter),
                " widget for the given id, if it exists and has the correct type."
            )]
            pub fn $getter(&self, widget_id: WidgetId) -> Option<&$ty> {
                match self.widgets.get(widget_id)? {
                    Widget::$variant(w) => Some(w),
                    _ => None,
                }
            }

            #[doc = concat!("Edits a ", stringify!($getter), " widget by its `WidgetId`.")]
            pub fn $editor<R>(
                &mut self,
                widget_id: WidgetId,
                f: impl FnOnce(&mut $ty) -> R,
            ) -> Option<R> {
                self.widgets.$editor(widget_id, f)
            }
        )*
    };
}
