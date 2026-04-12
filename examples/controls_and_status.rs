use std::{io, thread, time::Duration};

use quilltty::prelude::*;
use quilltty::terminal::AppEvent;

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let input = Input::listen(Input::DEFAULT_POLL_INTERVAL_MS)?;

    let size = Terminal::size()?;
    let mut ui = Ui::new(size.x, size.y, None);

    let controls_pane = ui
        .create_pane()
        .title("Controls")
        .rect(Rect {
            x: 2,
            y: 1,
            width: 36,
            height: 14,
        })
        .movable(true)
        .resizable(true)
        .border(Some(BorderKind::Rounded))
        .build();

    let status_pane = ui
        .create_pane()
        .title("Status")
        .rect(Rect {
            x: 40,
            y: 1,
            width: size.x.saturating_sub(42),
            height: 14,
        })
        .movable(true)
        .resizable(true)
        .border(Some(BorderKind::Rounded))
        .build();

    ui.create_widget(
        controls_pane,
        TextWidget::with_lines([
            StyledLine::new().with_span(StyledSpan::new("Use the checkbox and slider.")),
            StyledLine::new().with_span(StyledSpan::new("Press q or Esc to quit.")),
        ]),
    )
    .layout(WidgetLayout::Inset {
        left: 1,
        top: 1,
        right: 1,
        bottom: 9,
    })
    .build();

    let visible_id = ui
        .create_widget(
            controls_pane,
            CheckboxWidget::new(Some("Show status pane"), false, true),
        )
        .layout(WidgetLayout::Line {
            left: 1,
            top: 4,
            right: 1,
        })
        .build();

    let slider_id = ui
        .create_widget(
            controls_pane,
            SliderWidget::new(Some("Level "), 0.0, 100.0, 35.0),
        )
        .layout(WidgetLayout::Line {
            left: 1,
            top: 6,
            right: 1,
        })
        .build();

    let progress_id = ui
        .create_widget(
            status_pane,
            ProgressWidget::new(Some("Level "), 0.0, 100.0, 35.0),
        )
        .layout(WidgetLayout::Line {
            left: 1,
            top: 1,
            right: 1,
        })
        .build();

    ui.create_widget(
        status_pane,
        TextWidget::with_lines([
            StyledLine::new().with_span(StyledSpan::new("The progress bar tracks the slider.")),
            StyledLine::new().with_span(StyledSpan::new("The checkbox toggles pane visibility.")),
        ]),
    )
    .layout(WidgetLayout::Inset {
        left: 1,
        top: 3,
        right: 1,
        bottom: 1,
    })
    .build();

    let mut stdout = io::stdout();
    ui.render_to(&mut stdout)?;

    loop {
        while let Some(app_event) = input.try_read() {
            let mut should_quit = false;

            match app_event {
                AppEvent::Input(Event::Key(key))
                    if key.kind == KeyEventKind::Press
                        && matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) =>
                {
                    should_quit = true;
                }
                AppEvent::Input(event) => match ui.handle_event(event) {
                    UiEvent::CheckboxChanged { widget_id, checked } if widget_id == visible_id => {
                        if checked {
                            let _ = ui.show_pane(status_pane);
                        } else {
                            let _ = ui.hide_pane(status_pane);
                        }
                    }
                    UiEvent::SliderChanged { widget_id, value } if widget_id == slider_id => {
                        let _ = ui.edit_widget_as::<ProgressWidget, _>(progress_id, |progress| {
                            progress.set(value);
                        });
                    }
                    _ => {}
                },
                AppEvent::Tick { dt } => {
                    let _ = ui.tick(dt);
                }
            }

            ui.render_to(&mut stdout)?;

            if should_quit {
                return Ok(());
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}
