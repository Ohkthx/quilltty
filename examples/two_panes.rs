use std::{io, thread, time::Duration};

use quilltty::prelude::*;
use quilltty::terminal::AppEvent;

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let input = Input::listen(Input::DEFAULT_POLL_INTERVAL_MS)?;

    let size = Terminal::size()?;
    let mut ui = Ui::new(size.x, size.y, None);

    let left = ui
        .create_pane()
        .title("Overview")
        .rect(Rect {
            x: 2,
            y: 1,
            width: size.x.saturating_div(2).saturating_sub(3),
            height: size.y.saturating_sub(2),
        })
        .movable(true)
        .resizable(true)
        .border(Some(BorderKind::Double))
        .build();

    let right = ui
        .create_pane()
        .title("Details")
        .rect(Rect {
            x: size.x.saturating_div(2) + 1,
            y: 1,
            width: size.x.saturating_div(2).saturating_sub(3),
            height: size.y.saturating_sub(2),
        })
        .movable(true)
        .resizable(true)
        .border(Some(BorderKind::Double))
        .build();

    ui.create_widget(
        left,
        TextWidget::with_lines([
            StyledLine::new().with_span(StyledSpan::new("QuillTTY")),
            StyledLine::new().with_span(StyledSpan::new("")),
            StyledLine::new().with_span(StyledSpan::new("A retained terminal UI crate.")),
            StyledLine::new().with_span(StyledSpan::new("")),
            StyledLine::new().with_span(StyledSpan::new("Try this:")),
            StyledLine::new().with_span(StyledSpan::new("- Drag pane titles")),
            StyledLine::new().with_span(StyledSpan::new("- Resize pane corners")),
            StyledLine::new().with_span(StyledSpan::new("- Press q or Esc to quit")),
        ])
        .with_wrap(true),
    )
    .layout(WidgetLayout::Inset {
        left: 1,
        top: 1,
        right: 1,
        bottom: 1,
    })
    .build();

    ui.create_widget(
        right,
        TextWidget::with_lines([
            StyledLine::new().with_span(StyledSpan::new("This example demonstrates:")),
            StyledLine::new().with_span(StyledSpan::new("")),
            StyledLine::new().with_span(StyledSpan::new("- Pane builders")),
            StyledLine::new().with_span(StyledSpan::new("- Widget builders")),
            StyledLine::new().with_span(StyledSpan::new("- Styled text widgets")),
            StyledLine::new().with_span(StyledSpan::new("- Mouse-enabled terminal mode")),
        ])
        .with_wrap(true),
    )
    .layout(WidgetLayout::Inset {
        left: 1,
        top: 1,
        right: 1,
        bottom: 1,
    })
    .build();

    let mut stdout = io::stdout();
    ui.render_to(&mut stdout)?;

    loop {
        while let Some(app_event) = input.try_read() {
            match app_event {
                AppEvent::Input(Event::Key(key))
                    if key.kind == KeyEventKind::Press
                        && matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) =>
                {
                    return Ok(());
                }
                AppEvent::Input(event) => {
                    let _ = ui.handle_event(event);
                }
                AppEvent::Tick { dt } => {
                    let _ = ui.tick(dt);
                }
            }

            ui.render_to(&mut stdout)?;
        }

        thread::sleep(Duration::from_millis(1));
    }
}
