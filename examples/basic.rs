use std::{io, thread, time::Duration};

use crossterm::{
    event::{Event, KeyCode},
    style::Stylize,
};
use quilltty::{Input, Terminal};

fn main() -> io::Result<()> {
    let _terminal = Terminal::new(true)?;
    println!("{}: {:?}", "Hello World".bold().red(), Terminal::size());

    let input = Input::listen()?;
    'main: loop {
        for event in input.drain() {
            if let Event::Key(key) = event {
                println!("[key: {}] ", key.code);
                if key.code == KeyCode::Char('q') {
                    break 'main;
                }
            }
        }

        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
