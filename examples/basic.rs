use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode};
use quilltty::{Canvas, Color, Compositor, Glyph, Input, Rect, Renderer, Style, Terminal};

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let (terminal_width, terminal_height) = Terminal::size()?;

    let mut stdout = io::stdout().lock();

    let mut canvas = Canvas::new(terminal_width, terminal_height);
    let mut compositor = Compositor::new(terminal_width, terminal_height);
    let mut renderer = Renderer::new(terminal_width, terminal_height, true);

    let toggle_id = canvas
        .create_pane()
        .rect(
            Rect::default()
                .position(terminal_width / 10, terminal_height / 10)
                .width(terminal_width / 2)
                .height(terminal_height / 2),
        )
        .layer(2)
        .visible(false)
        .build();

    if let Some(pane) = canvas.pane_mut(toggle_id) {
        pane.fill(Glyph::default().with_style(Style::default().with_bg(Color::Blue)));
    }

    let pane_id = canvas
        .create_pane()
        .rect(
            Rect::default()
                .width((terminal_width / 2).max(12))
                .height((terminal_height / 2).max(6))
                .center_on(terminal_width / 2, terminal_height / 2),
        )
        .layer(1)
        .build();

    let root_style = Style::new().with_fg(Color::White);
    let pane_style = Style::new().with_fg(Color::Yellow);
    let pane_title = Style::new().with_fg(Color::Red).bold();
    let pane_fill_style = Style::new().with_fg(Color::Blue);

    // Root layer content.
    let root = canvas.root_mut();
    root.write_str(0, 0, "Press arrow keys to move pane.", root_style);
    root.write_str(0, 1, "Press 'q' to quit.", root_style);
    root.write_str(0, 2, "Press 't' to toggle hidden pane.", root_style);

    // Pane content.
    let pane = canvas.pane_mut(pane_id).expect("pane should exist");
    pane.fill(Glyph::from('·').with_style(pane_fill_style));
    pane.write_str(2, 0, "Pane layer", pane_title);
    pane.write_str(2, 1, "SOME TEXT TO BE DISPLAYED.", pane_style);
    pane.write_str(2, 2, "This pane has a higher z-layer.", pane_style);

    canvas.render(&mut compositor, &mut renderer, &mut stdout)?;
    stdout.flush()?;

    let input = Input::listen()?;

    'main: loop {
        for event in input.drain() {
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break 'main,
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        canvas.toggle_pane_visibility(toggle_id);
                    }
                    KeyCode::Left => {
                        if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                            rect.x = rect.x.saturating_sub(1);
                            canvas.move_pane(pane_id, rect.x, rect.y, true);
                        }
                    }
                    KeyCode::Right => {
                        if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                            rect.x = rect.x.saturating_add(1);
                            canvas.move_pane(pane_id, rect.x, rect.y, true);
                        }
                    }
                    KeyCode::Up => {
                        if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                            rect.y = rect.y.saturating_sub(1);
                            canvas.move_pane(pane_id, rect.x, rect.y, true);
                        }
                    }
                    KeyCode::Down => {
                        if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                            rect.y = rect.y.saturating_add(1);
                            canvas.move_pane(pane_id, rect.x, rect.y, true);
                        }
                    }
                    _ => {}
                }
            }
        }

        canvas.render(&mut compositor, &mut renderer, &mut stdout)?;
        stdout.flush()?;

        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
