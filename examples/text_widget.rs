/// File: examples/text_widget.rs
use std::{
    io::{self},
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode};
use quilltty::{
    prelude::*,
    ui::{TextWidget, Ui, UiEvent},
};

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let (width, height) = Terminal::size()?.into();

    let mut stdout = io::stdout().lock();
    let mut ui = Ui::new(width, height, Some(BorderKind::Double));

    // Root instructions.
    {
        let root = ui.canvas_mut().root_mut();
        root.write_str(
            Point::ZERO,
            "Click the text area to focus/click it.",
            Style::new(),
        );
        root.write_str(
            Point::ZERO.with_y(1),
            "Hover the text to see hover styling.",
            Style::new(),
        );
        root.write_str(
            Point::ZERO.with_y(2),
            "Drag the pane title/border to move it.",
            Style::new(),
        );
        root.write_str(
            Point::ZERO.with_y(3),
            "Drag the bottom-right corner to resize it.",
            Style::new(),
        );
        root.write_str(
            Point::ZERO.with_y(4),
            "Press 'q' or Esc to quit.",
            Style::new(),
        );
    }

    ui.set_pane_title(
        Canvas::ROOT_ID,
        Some("quilltty - examples/text_widget".to_string()),
    );

    let pane_rect = Rect::default()
        .width((width / 2).max(30))
        .height((height / 2).max(10))
        .center_on(width / 2, height / 2);

    let text_pane = ui
        .create_pane()
        .title("TextWidget Demo")
        .border(Some(BorderKind::Rounded))
        .movable(true)
        .layer(2)
        .rect(pane_rect)
        .build();

    let mut text = TextWidget::new()
        .with_hover_style(Style::new().with_fg(Color::Yellow).bold())
        .with_wrap(true);

    text.push("TextWidget renders multiple lines inside a widget rect.");
    text.push("It can still participate in hover, press, focus, and click flow.");
    text.push("This example updates the pane title whenever the text is clicked.");
    text.push("Because TextWidget has no cursor, focus is purely visual here.");
    text.push("Try moving or resizing the pane too.");

    let text_widget = ui
        .widget(text_pane)
        .with_layout(WidgetLayout::Fill)
        .with_widget(text)
        .build();

    ui.focus_pane(text_pane);
    ui.focus_widget(Some(text_widget));
    ui.render_to(&mut stdout)?;

    let input = Input::listen(25)?;
    let mut click_count = 0usize;

    'main: loop {
        for event in input.drain() {
            match event {
                Event::Mouse(mouse_event) => match ui.mouse(mouse_event) {
                    UiEvent::WidgetClicked(hit) if hit.widget_id == text_widget => {
                        click_count += 1;
                        ui.set_pane_title(
                            text_pane,
                            Some(format!("TextWidget Demo ({click_count} clicks)")),
                        );
                    }
                    _ => {}
                },

                Event::Key(key_event) => match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break 'main,
                    _ => {
                        let _ = ui.key(key_event);
                    }
                },

                _ => {}
            }
        }

        ui.render_to(&mut stdout)?;
        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
