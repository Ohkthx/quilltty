use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use quilltty::{
    BorderKind, Canvas, Color, Compositor, Glyph, Input, PaneElement, PaneHit, PaneId, Point, Rect,
    Renderer, Style, Terminal,
};

enum DragMode {
    Move { grab_offset: Point },
    Resize,
}

struct DragState {
    pane_id: PaneId,
    mode: DragMode,
}

fn key_event(key: &KeyEvent, canvas: &mut Canvas, pane_id: PaneId, toggle_id: PaneId) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('t') | KeyCode::Char('T') => {
            canvas.set_pane_title(Canvas::ROOT_ID, Some("NEW TITLE".into()));
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            canvas.toggle_pane_visibility(toggle_id);
        }
        KeyCode::Left => {
            if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                rect.x = rect.x.saturating_sub(1);
                canvas.move_pane(pane_id, rect.origin(), true);
            }
        }
        KeyCode::Right => {
            if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                rect.x = rect.x.saturating_add(1);
                canvas.move_pane(pane_id, rect.origin(), true);
            }
        }
        KeyCode::Up => {
            if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                rect.y = rect.y.saturating_sub(1);
                canvas.move_pane(pane_id, rect.origin(), true);
            }
        }
        KeyCode::Down => {
            if let Some(mut rect) = canvas.pane(pane_id).map(|p| p.rect()) {
                rect.y = rect.y.saturating_add(1);
                canvas.move_pane(pane_id, rect.origin(), true);
            }
        }
        _ => {}
    }

    false
}

fn mouse_event(
    mouse: &crossterm::event::MouseEvent,
    canvas: &mut Canvas,
    drag: &mut Option<DragState>,
    last_hit: &Option<PaneHit>,
) -> bool {
    let position: Point = (mouse.column as usize, mouse.row as usize).into();

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(hit) = last_hit {
                canvas.focus_pane(hit.pane_id);

                *drag = match hit.element {
                    PaneElement::Title | PaneElement::Border => Some(DragState {
                        pane_id: hit.pane_id,
                        mode: DragMode::Move {
                            grab_offset: hit.local,
                        },
                    }),
                    PaneElement::Resize => Some(DragState {
                        pane_id: hit.pane_id,
                        mode: DragMode::Resize,
                    }),
                    PaneElement::Content => None,
                };
            }
        }

        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(state) = drag.as_ref() {
                match state.mode {
                    DragMode::Move { grab_offset } => {
                        let pos = position.saturating_sub(grab_offset);
                        canvas.move_pane(state.pane_id, pos, true);
                    }
                    DragMode::Resize => {
                        if let Some(rect) = canvas.pane(state.pane_id).map(|p| p.rect()) {
                            let width = position.x.saturating_sub(rect.x).saturating_add(1);
                            let height = position.y.saturating_sub(rect.y).saturating_add(1);
                            canvas.resize_pane(state.pane_id, width, height);
                        }
                    }
                }
            }
        }

        MouseEventKind::Up(MouseButton::Left) => {
            *drag = None;
        }

        _ => {}
    }

    false
}

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    let (width, height) = Terminal::size()?.into();
    let border = Some(BorderKind::Double);

    let mut stdout = io::stdout().lock();

    let mut canvas = Canvas::new(width, height, border);
    let mut compositor = Compositor::new(width, height);
    let mut renderer = Renderer::new(width, height, true);

    let mut drag: Option<DragState> = None;

    let toggle_id = canvas
        .create_pane()
        .rect(
            Rect::default()
                .position(width / 10, height / 10)
                .width(width / 2)
                .height(height / 2),
        )
        .layer(2)
        .visible(false)
        .border(border)
        .build();

    if let Some(pane) = canvas.pane_mut(toggle_id) {
        pane.fill(Glyph::default().with_style(Style::default().with_bg(Color::Blue)));
    }

    let pane_id = canvas
        .create_pane()
        .rect(
            Rect::default()
                .width((width / 2).max(12))
                .height((height / 2).max(6))
                .center_on(width / 2, height / 2),
        )
        .layer(1)
        .movable(true)
        .border(border)
        .border_style(Style::default())
        .title("Created Pane")
        .build();

    let root_style = Style::new().with_fg(Color::White);
    let pane_style = Style::new().with_fg(Color::Yellow);
    let pane_fill_style = Style::new().with_fg(Color::Blue).blink();

    // Root layer content.
    let root = canvas.root_mut();
    root.write_str(0, 0, "Press arrow keys to move pane.", root_style);
    root.write_str(0, 1, "Press 'q' to quit.", root_style);
    root.write_str(0, 2, "Press 's' to toggle hidden pane.", root_style);
    canvas.set_pane_title(Canvas::ROOT_ID, Some("quilltty - root".into()));

    // Pane content.
    let pane = canvas.pane_mut(pane_id).expect("pane should exist");
    pane.fill(Glyph::from('·').with_style(pane_fill_style));
    pane.write_str(2, 0, "SOME TEXT TO BE DISPLAYED.", pane_style);
    pane.write_str(2, 1, "This pane has a higher z-layer.", pane_style);

    canvas.render(&mut compositor, &mut renderer, &mut stdout)?;
    stdout.flush()?;

    let mut last_hit: Option<PaneHit>;
    let input = Input::listen(25)?;

    'main: loop {
        for event in input.drain() {
            let exit = match event {
                Event::Key(key) => key_event(&key, &mut canvas, pane_id, toggle_id),
                Event::Mouse(mouse) => {
                    let position = (mouse.column as usize, mouse.row as usize).into();
                    last_hit = canvas.pane_at(position);

                    mouse_event(&mouse, &mut canvas, &mut drag, &last_hit)
                }
                _ => false,
            };

            if exit {
                break 'main;
            }
        }

        canvas.render(&mut compositor, &mut renderer, &mut stdout)?;
        stdout.flush()?;

        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
