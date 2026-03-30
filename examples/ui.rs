/// File: examples/ui.rs
use std::{
    io::{self},
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode};
use quilltty::{
    Widget,
    prelude::*,
    style,
    ui::{CheckboxWidget, Ui, UiEvent},
};

fn apply_title(ui: &mut Ui, pane_id: PaneId, input_widget: WidgetId, value: String) {
    let title = if value.trim().is_empty() {
        "Text Box".to_string()
    } else {
        format!("Text Box: {value}")
    };

    ui.set_pane_title(pane_id, Some(title));
    ui.focus_pane(pane_id);
    ui.focus_widget(Some(input_widget));
}

fn submit_input(ui: &mut Ui, pane_id: PaneId, input_widget: WidgetId) {
    if let Some(value) = ui.edit_input(input_widget, |w| w.submit()) {
        apply_title(ui, pane_id, input_widget, value);
    }
}

fn focused_is_input(ui: &Ui) -> bool {
    ui.focused_widget()
        .is_some_and(|widget_id| matches!(ui.widget(widget_id), Some(Widget::Input(_))))
}

fn event_handler(
    ui: &mut Ui,
    event: UiEvent,
    main_pane: PaneId,
    input_widget: WidgetId,
    apply_button: WidgetId,
    shown_button: WidgetId,
) {
    match event {
        UiEvent::WidgetClicked(hit) if hit.widget_id == apply_button => {
            submit_input(ui, main_pane, input_widget);
        }

        UiEvent::InputSubmitted { widget_id, value } if widget_id == input_widget => {
            apply_title(ui, main_pane, input_widget, value);
        }

        UiEvent::CheckboxChanged { widget_id, checked } if widget_id == shown_button => {
            let pane_ids: Vec<_> = ui.pane_ids().collect();

            for pane_id in pane_ids {
                if checked {
                    ui.hide_pane(pane_id);
                } else {
                    ui.show_pane(pane_id);
                }
            }
        }

        UiEvent::PaneReleased { pane_id, kind } => {
            let _ = (pane_id, kind);
        }

        _ => {}
    }
}

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let (width, height) = Terminal::size()?.into();

    let mut stdout = io::stdout().lock();
    let mut ui = Ui::new(width, height, Some(BorderKind::Double));

    // Root instructions.
    ui.with_root_mut(|root| {
        root.write_str(
            Point::ZERO,
            "Drag the title or border to move the pane.",
            Style::new(),
        );
        root.write_str(
            Point::ZERO.with_y(1),
            "Drag the resize handle in the bottom-right corner to resize it.",
            Style::new(),
        );
        root.write_str(
            Point::ZERO.with_y(2),
            "Type in the input and press Enter or click the button.",
            Style::new(),
        );
        root.write_str(
            Point::ZERO.with_y(3),
            "Press 'q' to quit when not focused in the input.",
            Style::new(),
        );
    });

    ui.set_pane_title(Canvas::ROOT_ID, Some("quilltty - examples/ui".to_string()));

    let shown_button = ui.add_widget(
        Canvas::ROOT_ID,
        CheckboxWidget::new(Some("Hide All"), true, false)
            .with_hover_style(Style::new().with_fg(Color::Green).bold())
            .with_pressed_style(Style::new().inverse())
            .into(),
        WidgetLayout::Fixed(
            Rect::default()
                .with_origin((0usize, height - 3).into())
                .width(17)
                .height(1),
        ),
    );

    // Background pane to make layering more obvious.
    let back_pane = ui
        .add_pane()
        .rect(
            Rect::default()
                .with_origin((width / 8, height / 5).into())
                .width((width / 3).max(18))
                .height((height / 3).max(8)),
        )
        .layer(1)
        .border(Some(BorderKind::Single))
        .title("Background Pane")
        .build();

    ui.with_pane_mut(back_pane, |pane| {
        pane.fill(style::Glyph::from('·').with_style(Style::new().with_fg(Color::Blue)));
        pane.write_str(
            Point::new(2, 1),
            "This pane sits behind the draggable one.",
            Style::new().with_fg(Color::Yellow),
        );
    });

    // Main interactive pane.
    let main_rect = Rect::default()
        .width((width / 2).max(28))
        .height((height / 2).max(8))
        .center_on(width / 2, height / 2);

    let main_pane = ui
        .add_pane()
        .title("Text Box")
        .border(Some(BorderKind::Rounded))
        .movable(true)
        .layer(3)
        .rect(main_rect)
        .build();

    ui.with_pane_mut(main_pane, |pane| {
        pane.write_str(
            Point::new(0, 4),
            "Try moving and resizing this pane.",
            Style::new().with_fg(Color::Cyan),
        );
    });

    let input_widget = ui.add_widget(
        main_pane,
        InputWidget::new(Some("New Title"), Some("type here...")).into(),
        WidgetLayout::Fixed(
            Rect::default()
                .with_origin((0usize, 0usize).into())
                .width(main_rect.width / 2)
                .height(1),
        ),
    );

    let apply_button = ui.add_widget(
        main_pane,
        ButtonWidget::new(Some("[Apply Title]"))
            .with_hover_style(Style::new().with_fg(Color::Green).bold())
            .with_pressed_style(Style::new().inverse())
            .into(),
        WidgetLayout::Fixed(
            Rect::default()
                .with_origin((0usize, 2usize).into())
                .width(13)
                .height(1),
        ),
    );

    ui.focus_pane(main_pane);
    ui.focus_widget(Some(input_widget));
    ui.render_to(&mut stdout)?;

    let input = Input::listen(25)?;

    'main: loop {
        for event in input.drain() {
            match event {
                Event::Mouse(mouse_event) => {
                    let event = ui.mouse(mouse_event);
                    event_handler(
                        &mut ui,
                        event,
                        main_pane,
                        input_widget,
                        apply_button,
                        shown_button,
                    );
                }

                Event::Key(key_event) => {
                    match key_event.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') if !focused_is_input(&ui) => {
                            break 'main;
                        }
                        _ => {}
                    }

                    let event = ui.key(key_event);
                    event_handler(
                        &mut ui,
                        event,
                        main_pane,
                        input_widget,
                        apply_button,
                        shown_button,
                    );
                }

                _ => {}
            }
        }

        ui.render_to(&mut stdout)?;
        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
