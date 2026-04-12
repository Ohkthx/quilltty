use std::{io, thread, time::Duration};

use quilltty::prelude::*;
use quilltty::terminal::AppEvent;

fn push_log(ui: &mut Ui, log_id: WidgetId, message: impl Into<String>) {
    let message = message.into();
    let _ = ui.edit_widget_as::<LogWidget, _>(log_id, move |log| {
        log.push(message);
    });
}

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let input = Input::listen(Input::DEFAULT_POLL_INTERVAL_MS)?;

    let size = Terminal::size()?;
    let mut ui = Ui::new(size.x, size.y, None);

    let pane_id = ui
        .create_pane()
        .title("Submit to log")
        .rect(Rect {
            x: 2,
            y: 1,
            width: size.x.saturating_sub(4).min(80),
            height: size.y.saturating_sub(2).min(24),
        })
        .movable(true)
        .resizable(true)
        .border(Some(BorderKind::Single))
        .build();

    ui.create_widget(
        pane_id,
        TextWidget::with_lines([
            StyledLine::new().with_span(StyledSpan::new("Type into the input and press Enter.")),
            StyledLine::new().with_span(StyledSpan::new("Press q or Esc to quit.")),
        ]),
    )
    .layout(WidgetLayout::Inset {
        left: 1,
        top: 1,
        right: 1,
        bottom: 18,
    })
    .build();

    let input_id = ui
        .create_widget(
            pane_id,
            InputWidget::new(Some("Message"), Some("write something")),
        )
        .layout(WidgetLayout::Line {
            left: 1,
            top: 4,
            right: 1,
        })
        .build();

    let log_id = ui
        .create_widget(pane_id, LogWidget::new(true, 200).with_wrap(true))
        .layout(WidgetLayout::Inset {
            left: 1,
            top: 6,
            right: 1,
            bottom: 1,
        })
        .build();

    push_log(&mut ui, log_id, "ready");

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
                    UiEvent::InputSubmitted { widget_id, value } if widget_id == input_id => {
                        if !value.is_empty() {
                            push_log(&mut ui, log_id, format!("submitted: {value}"));
                            let _ = ui.edit_widget_as::<InputWidget, _>(input_id, |input| {
                                while !input.value().is_empty() {
                                    input.backspace();
                                }
                            });
                        }
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
