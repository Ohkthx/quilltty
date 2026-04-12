# QuillTTY

**QuillTTY is a Rust crate for building interactive terminal interfaces with panes, widgets, styled rendering, and mouse + keyboard input.**

It gives you a practical middle ground between raw terminal drawing and heavyweight UI frameworks:

- low-level enough to stay flexible
- structured enough to build real interfaces
- ergonomic enough to get moving quickly

## Why QuillTTY?

Terminal apps often end up in one of two places:

- too bare-metal, where everything becomes manual rendering and input plumbing
- too framework-heavy, where simple interfaces feel overbuilt

QuillTTY is aimed at the useful middle:

- **pane-based composition** for windowed terminal layouts
- **widget-driven interaction** for common controls
- **builder-first APIs** for panes and widgets
- **incremental retained rendering** instead of redrawing everything blindly
- **direct access escape hatches** when you need lower-level control

## Features

- **Pane-based UI**
  - movable and resizable panes
  - configurable layer, visibility, policy, and window decoration
  - titles, borders, and pane hit detection

- **Widget system**
  - builder-driven widget creation
  - per-widget layout, layer, visibility, and enabled state
  - runtime widget removal and pane removal
  - widget querying, editing, and invalidation

- **Built-in widgets**
  - `TextWidget`
  - `ButtonWidget`
  - `CheckboxWidget`
  - `SliderWidget`
  - `ProgressWidget`
  - `LogWidget`
  - `InputWidget`

- **Input and interaction**
  - keyboard and mouse support
  - background input polling
  - high-level `UiEvent` values for widget and pane actions

- **Rendering**
  - styled glyph rendering
  - layered pane composition
  - damage-based updates
  - cursor support for focused widgets

## What QuillTTY is good for

QuillTTY is a strong fit for:

- terminal dashboards
- admin tools
- developer utilities
- game-adjacent terminal interfaces
- text-heavy control panels
- custom TUI tools that need window-like panes and simple widgets

## Quick example

```rust
use std::{io, thread, time::Duration};

use quilltty::prelude::*;
use quilltty::terminal::AppEvent;

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let input = Input::listen(Input::DEFAULT_POLL_INTERVAL_MS)?;

    let size = Terminal::size()?;
    let mut ui = Ui::new(size.x, size.y, None);

    let pane_id = ui
        .create_pane()
        .title("Hello QuillTTY")
        .rect(Rect {
            x: 2,
            y: 1,
            width: size.x.saturating_sub(4),
            height: size.y.saturating_sub(2),
        })
        .movable(true)
        .resizable(true)
        .border(Some(BorderKind::Single))
        .build();

    ui.create_widget(
        pane_id,
        TextWidget::with_lines([
            StyledLine::new().with_span(StyledSpan::new("QuillTTY is running.")),
            StyledLine::new().with_span(StyledSpan::new("Press Esc or q to quit.")),
        ]),
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
