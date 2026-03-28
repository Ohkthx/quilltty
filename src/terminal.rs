//! File: src/terminal.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::{io, thread, time};

use crossterm::cursor::Show;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode};

use crate::geom::Point;

/// Enables / Disables the terminal state including mouse capturing.
pub struct Terminal {
    mouse: bool, // Flag for mouse capturing.
}

impl Terminal {
    /// Creates a new instance of the terminal.
    pub fn new(enable_mouse: bool) -> io::Result<Self> {
        enable_raw_mode()?;

        if enable_mouse {
            execute!(io::stdout(), EnableMouseCapture)?;
        }

        Ok(Self {
            mouse: enable_mouse,
        })
    }

    /// Outputs the size of the terminal.
    pub fn size() -> io::Result<Point> {
        terminal::size().map(|size| size.into())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = if self.mouse {
            execute!(io::stdout(), DisableMouseCapture, Show)
        } else {
            execute!(io::stdout(), Show)
        };

        let _ = disable_raw_mode();
    }
}

/// Background reader for terminal events.
pub struct Input {
    rx: mpsc::Receiver<Event>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<io::Result<()>>>,
}

impl Input {
    /// Starts background event reader polling for `interval_ms` milliseconds.
    pub fn listen(interval_ms: u64) -> io::Result<Self> {
        let (tx, rx) = mpsc::sync_channel(256);
        let stop = Arc::new(AtomicBool::new(false));
        let threaded_stop = Arc::clone(&stop);

        // Start a thread and poll for events.
        let handle = thread::Builder::new().name("quilltty-input".into()).spawn(
            move || -> io::Result<()> {
                while !threaded_stop.load(Ordering::Relaxed) {
                    if event::poll(time::Duration::from_millis(interval_ms))? {
                        let ev = event::read()?;
                        if tx.send(ev).is_err() {
                            break;
                        }
                    }
                }
                Ok(())
            },
        )?;

        Ok(Self {
            rx,
            stop,
            handle: Some(handle),
        })
    }

    /// Returns an iterator over all currently buffered events without blocking.
    pub fn drain(&self) -> impl Iterator<Item = Event> + '_ {
        self.rx.try_iter()
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);

        // Wait for stop signal to end event reading loop.
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
