//! File: examples/custom_widget.rs

use std::{
    io::{self, stdout},
    thread,
    time::Duration,
};

use quilltty::{
    InteractionStyle, Pane,
    prelude::*,
    style::Glyph,
    terminal::AppEvent,
    ui::{StylableWidgetExt, Widget, WidgetAction, WidgetState},
};

/// Returns `true` when the example should quit.
fn should_quit(event: &Event) -> bool {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
        }
        _ => false,
    }
}

/// Custom payload emitted by the widget through `WidgetAction::Custom`.
#[derive(Debug)]
enum CounterEvent {
    Incremented { value: u32 },
}

/// A minimal custom widget that increments an internal counter when activated.
struct CounterWidget {
    state: WidgetState,            // Hover/focus/pressed/damage tracking.
    interaction: InteractionStyle, // Resolved interaction styles.
    value: u32,                    // Current count.
}

impl CounterWidget {
    /// Creates a new counter widget.
    fn new(value: u32) -> Self {
        Self {
            state: WidgetState::default(),
            interaction: InteractionStyle::default(),
            value,
        }
    }

    /// Increments the count and emits a custom widget event.
    fn increment(&mut self) -> WidgetAction {
        self.value += 1;
        self.state_mut().set_damaged(true);

        WidgetAction::Custom(Box::new(CounterEvent::Incremented { value: self.value }))
    }
}

impl Widget for CounterWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn state(&self) -> &WidgetState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WidgetState {
        &mut self.state
    }

    fn interaction(&self) -> Option<&InteractionStyle> {
        Some(&self.interaction)
    }

    fn interaction_mut(&mut self) -> Option<&mut InteractionStyle> {
        Some(&mut self.interaction)
    }

    fn draw(&mut self, pane: &mut Pane, rect: Rect) {
        let style = self.interaction.style(self.state());

        let line_0 = self.glyph_row("Custom widget demo", style, rect.width);
        self.write_glyph_row(pane, rect, 0, &line_0);

        let line_1 = self.glyph_row("", style, rect.width);
        self.write_glyph_row(pane, rect, 1, &line_1);

        let line_2 = self.glyph_row("[ Click or press Enter / Space ]", style, rect.width);
        self.write_glyph_row(pane, rect, 2, &line_2);

        let line_3 = self.glyph_row(&format!("count = {}", self.value), style, rect.width);
        self.write_glyph_row(pane, rect, 3, &line_3);
    }

    fn activate_action(&mut self) -> WidgetAction {
        self.increment()
    }

    fn release_action(&mut self, focused: bool) -> WidgetAction {
        if focused {
            self.increment()
        } else {
            WidgetAction::Released
        }
    }
}

impl StylableWidgetExt for CounterWidget {}

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let input = Input::listen(16)?;
    let mut out = stdout();

    let term = Terminal::size()?;
    let width = term.x.max(24);
    let height = term.y.max(10);

    let mut ui = Ui::new(width, height, Some(Glyph::from(' ')));

    let pane = ui
        .create_pane()
        .rect(Rect {
            x: 1,
            y: 1,
            width: width.saturating_sub(2),
            height: height.saturating_sub(2),
        })
        .border(Some(BorderKind::Single))
        .title("Custom Widget")
        .build();

    let interaction = InteractionStyle {
        normal: Style::default(),
        hover: Style::default().with_fg(Color::Green),
        pressed: Style::default().with_fg(Color::Red),
        focused: Style::default().underline(),
    };

    let counter_id = ui
        .create_widget(pane, CounterWidget::new(0).with_interaction(interaction))
        .layout(WidgetLayout::Fixed(Rect {
            x: 1,
            y: 1,
            width: width.saturating_sub(6),
            height: 4,
        }))
        .build();

    ui.render_to(&mut out)?;

    loop {
        let mut dirty = false;

        if let Some(app_event) = input.try_read() {
            match app_event {
                AppEvent::Input(event) => {
                    if should_quit(&event) {
                        return Ok(());
                    }

                    let ui_event = ui.handle_event(event);

                    match ui_event {
                        UiEvent::WidgetCustom { widget_id, payload } if widget_id == counter_id => {
                            match payload.downcast::<CounterEvent>() {
                                Ok(event) => match *event {
                                    CounterEvent::Incremented { value } => {
                                        ui.set_pane_title(
                                            pane,
                                            Some(format!("Custom Widget · count = {}", value)),
                                        );
                                        dirty = true;
                                    }
                                },
                                Err(_) => {}
                            }
                        }

                        UiEvent::WidgetHovered(_)
                        | UiEvent::WidgetPressed(_)
                        | UiEvent::WidgetReleased(_)
                        | UiEvent::WidgetClicked(_)
                        | UiEvent::WidgetCustom { .. }
                        | UiEvent::PanePressed { .. }
                        | UiEvent::PaneDragStart { .. }
                        | UiEvent::PaneDragged { .. }
                        | UiEvent::PaneReleased { .. }
                        | UiEvent::PaneContentDragStart { .. }
                        | UiEvent::PaneContentDragged { .. }
                        | UiEvent::PaneContentHeld { .. }
                        | UiEvent::PaneContentDragEnd { .. }
                        | UiEvent::CheckboxChanged { .. }
                        | UiEvent::SliderChanged { .. }
                        | UiEvent::InputChanged { .. }
                        | UiEvent::InputSubmitted { .. }
                        | UiEvent::None => {}
                    }
                }

                AppEvent::Tick { .. } => {}
            }

            // Hover/focus/pressed changes also mark widgets dirty.
            dirty |= ui.with_widgets(|widgets| widgets.has_damage());

            if dirty {
                ui.render_to(&mut out)?;
            }
        } else {
            thread::sleep(Duration::from_millis(4));
        }
    }
}
