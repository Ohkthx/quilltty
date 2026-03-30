//! File: src/ui/traits.rs

use crossterm::event::KeyCode;

use crate::{
    prelude::*,
    ui::{InteractionStyle, WidgetAction, widget::WidgetState},
};

pub trait HasWidgetState {
    fn state(&self) -> &WidgetState;
    fn state_mut(&mut self) -> &mut WidgetState;

    fn set_hovered(&mut self, value: bool) {
        self.state_mut().set_hovered(value);
    }

    fn set_pressed(&mut self, value: bool) {
        self.state_mut().set_pressed(value);
    }

    fn set_focused(&mut self, value: bool) {
        self.state_mut().set_focused(value);
    }

    fn set_damaged(&mut self, value: bool) {
        self.state_mut().damaged = value;
    }
}

pub trait HasInteractionStyle: HasWidgetState {
    fn interaction(&self) -> &InteractionStyle;
    fn interaction_mut(&mut self) -> &mut InteractionStyle;

    fn with_hover_style(mut self, style: Style) -> Self
    where
        Self: Sized,
    {
        self.interaction_mut().hover = style;
        self.set_damaged(true);
        self
    }

    fn with_pressed_style(mut self, style: Style) -> Self
    where
        Self: Sized,
    {
        self.interaction_mut().pressed = style;
        self.set_damaged(true);
        self
    }

    fn with_focus_style(mut self, style: Style) -> Self
    where
        Self: Sized,
    {
        self.interaction_mut().focused = style;
        self.set_damaged(true);
        self
    }
}

pub trait WidgetBehavior {
    fn cursor_pos(&self, _pane: &Pane, _rect: Rect) -> Option<Point> {
        None
    }

    fn activate_action(&mut self) -> WidgetAction {
        WidgetAction::None
    }

    fn key_action(&mut self, _key: KeyCode) -> WidgetAction {
        WidgetAction::None
    }

    fn drag_action(&mut self, _local_x: usize, _width: usize) -> WidgetAction {
        WidgetAction::None
    }

    fn release_action(&mut self, focused: bool) -> WidgetAction {
        if focused {
            WidgetAction::Clicked
        } else {
            WidgetAction::Released
        }
    }
}
