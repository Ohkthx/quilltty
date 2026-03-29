//! File: examples/widget.rs

use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use quilltty::{
    prelude::*,
    render::{Compositor, Renderer},
    ui::{PaneElement, Widget},
};

struct AppData {
    widgets: WidgetStore,
}

impl AppData {
    fn new() -> Self {
        Self {
            widgets: WidgetStore::new(),
        }
    }

    // Edit a focused input widget.
    fn input_mut<R>(
        &mut self,
        widget_id: WidgetId,
        f: impl FnOnce(&mut InputWidget) -> R,
    ) -> Option<R> {
        self.widgets.edit(widget_id, |w| w.as_input_mut().map(f))?
    }
}

// Submit input and show the result in the pane title.
fn submit_input(app: &mut AppData, canvas: &mut Canvas, pane_id: PaneId, input_widget: WidgetId) {
    if let Some(submitted) = app.input_mut(input_widget, |input| input.submit()) {
        let title = if submitted.is_empty() {
            "Text Box".to_string()
        } else {
            format!("Text Box: {submitted}")
        };

        canvas.set_pane_title(pane_id, Some(title));
        app.widgets.focus(Some(input_widget));
    }
}

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let (width, height) = Terminal::size()?.into();

    let mut stdout = io::stdout().lock();

    let mut canvas = Canvas::new(width, height, Some(BorderKind::Double));
    let mut compositor = Compositor::new(width, height);
    let mut renderer = Renderer::new(width, height, true);

    // Root layer content.
    let root = canvas.root_mut();
    root.write_str(Point::ZERO, "Press 'q' to quit.", Style::new());
    canvas.set_pane_title(Canvas::ROOT_ID, Some("quilltty - examples/widget".into()));

    let text_rect = Rect::default()
        .width(width / 2)
        .height(height / 2)
        .center_on(width / 2, height / 2);

    // Main demo pane.
    let text_pane = canvas
        .create_pane()
        .title("Input and Button Testing Pane")
        .border(Some(BorderKind::Rounded))
        .layer(3)
        .rect(text_rect)
        .build();

    let mut app = AppData::new();

    let input_widget = app
        .widgets
        .widget(text_pane)
        .with_layout(WidgetLayout::Fixed(
            Rect::default()
                .with_origin((0usize, 0usize).into())
                .width(text_rect.width / 2)
                .height(1),
        ))
        .with_widget(InputWidget::new(Some("New Title"), Some("type here...")))
        .build();

    let button_widget = app
        .widgets
        .widget(text_pane)
        .with_layout(WidgetLayout::Fixed(
            Rect::default()
                .with_origin((0usize, 1usize).into())
                .width(13)
                .height(1),
        ))
        .with_widget(
            ButtonWidget::new(Some("[Reset Title]"))
                .with_hover_style(Style::new().with_fg(Color::Red).bold())
                .with_pressed_style(Style::new().inverse()),
        )
        .build();

    app.widgets.focus(Some(input_widget));
    canvas.focus(text_pane);

    app.widgets.render_into(&mut canvas);
    canvas.render(&mut compositor, &mut renderer, &mut stdout)?;
    stdout.flush()?;

    let input = Input::listen(25)?;

    'main: loop {
        for event in input.drain() {
            match event {
                Event::Mouse(mouse_event) => {
                    let pos: Point = (mouse_event.column, mouse_event.row).into();

                    match mouse_event.kind {
                        // Track hover.
                        MouseEventKind::Moved => {
                            if let Some(hit) = canvas.pane_at(pos)
                                && hit.element == PaneElement::Content
                                && let Some(content_local) = hit.content_local
                            {
                                app.widgets.hover(&canvas, hit.pane_id, content_local);
                            } else {
                                app.widgets.clear_hover();
                            }
                        }

                        // Focus pane and press widget.
                        MouseEventKind::Down(MouseButton::Left) => {
                            if let Some(hit) = canvas.pane_at(pos) {
                                canvas.focus(hit.pane_id);

                                if hit.element == PaneElement::Content
                                    && let Some(content_local) = hit.content_local
                                    && app
                                        .widgets
                                        .mouse_down(&canvas, hit.pane_id, content_local)
                                        .is_none()
                                {
                                    app.widgets.focus(None);
                                    app.widgets.clear_hover();
                                    canvas.set_cursor(None);
                                }
                            } else {
                                app.widgets.focus(None);
                                app.widgets.clear_hover();
                                canvas.set_cursor(None);
                            }
                        }

                        // Release widget and handle button click.
                        MouseEventKind::Up(MouseButton::Left) => {
                            if let Some(hit) = canvas.pane_at(pos)
                                && hit.element == PaneElement::Content
                                && let Some(content_local) = hit.content_local
                            {
                                if let Some(widget_hit) =
                                    app.widgets.mouse_up(&canvas, hit.pane_id, content_local)
                                    && widget_hit.widget_id == button_widget
                                {
                                    submit_input(&mut app, &mut canvas, text_pane, input_widget);
                                }
                            } else {
                                app.widgets.clear_hover();
                            }
                        }

                        _ => {}
                    }
                }

                Event::Key(key_event) => {
                    let focused_widget = app.widgets.focused();
                    let focused_is_input = focused_widget
                        .and_then(|widget_id| app.widgets.get(widget_id))
                        .is_some_and(|widget| matches!(widget, Widget::Input(_)));

                    match key_event.code {
                        // Quit unless typing in the input.
                        KeyCode::Char('q') | KeyCode::Char('Q') if !focused_is_input => break 'main,
                        _ => {}
                    }

                    if let Some(widget_id) = focused_widget {
                        match key_event.code {
                            KeyCode::Char(ch) => {
                                app.input_mut(widget_id, |input| input.insert_char(ch));
                            }
                            KeyCode::Backspace => {
                                app.input_mut(widget_id, |input| input.backspace());
                            }
                            // Enter submits the input.
                            KeyCode::Enter if widget_id == input_widget => {
                                submit_input(&mut app, &mut canvas, text_pane, input_widget);
                            }
                            KeyCode::Left => {
                                app.input_mut(widget_id, |input| input.move_left());
                            }
                            KeyCode::Right => {
                                app.input_mut(widget_id, |input| input.move_right());
                            }
                            _ => {}
                        }
                    }
                }

                _ => {}
            }
        }

        // Draw updated widget state.
        app.widgets.render_into(&mut canvas);
        canvas.render(&mut compositor, &mut renderer, &mut stdout)?;
        stdout.flush()?;

        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
