//! File: src/terminal.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{io, thread};

use ::crossterm::cursor::Show;
use ::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use ::crossterm::execute;
use ::crossterm::terminal::{self, disable_raw_mode, enable_raw_mode};

use crate::{crossterm::event::Event, geom::Point};

/// Configuration for entering Quilltty's terminal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalOptions {
    /// Enables Crossterm mouse capture while the terminal handle is alive.
    pub mouse_capture: bool,
}

impl TerminalOptions {
    /// Creates default terminal options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables or disables mouse capture.
    #[must_use]
    pub fn with_mouse_capture(mut self, enabled: bool) -> Self {
        self.mouse_capture = enabled;
        self
    }
}

/// Enables / Disables the terminal state including mouse capturing.
pub struct Terminal {
    mouse_capture: bool, // Flag for mouse capturing.
}

impl Terminal {
    /// Creates a new instance of the terminal.
    pub fn new(enable_mouse: bool) -> io::Result<Self> {
        Self::with_options(TerminalOptions::new().with_mouse_capture(enable_mouse))
    }

    /// Creates a new instance of the terminal using explicit options.
    pub fn with_options(options: TerminalOptions) -> io::Result<Self> {
        enable_raw_mode()?;

        if options.mouse_capture {
            execute!(io::stdout(), EnableMouseCapture)?;
        }

        Ok(Self {
            mouse_capture: options.mouse_capture,
        })
    }

    /// Returns `true` when mouse capture is enabled.
    pub fn mouse_capture(&self) -> bool {
        self.mouse_capture
    }

    /// Outputs the size of the terminal.
    pub fn size() -> io::Result<Point> {
        terminal::size().map(|size| size.into())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = if self.mouse_capture {
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
    /// Default event polling interval in milliseconds.
    pub const DEFAULT_POLL_INTERVAL_MS: u64 = 16;

    /// Starts background event reader polling for `interval_ms` milliseconds.
    pub fn listen(interval_ms: u64) -> io::Result<Self> {
        Self::listen_with_interval(Duration::from_millis(interval_ms))
    }

    /// Starts background event reader polling at the provided interval.
    pub fn listen_with_interval(interval: Duration) -> io::Result<Self> {
        let (tx, rx) = mpsc::sync_channel(256);
        let stop = Arc::new(AtomicBool::new(false));
        let threaded_stop = Arc::clone(&stop);

        // Start a thread and poll for events.
        let handle = thread::Builder::new().name("quilltty-input".into()).spawn(
            move || -> io::Result<()> {
                while !threaded_stop.load(Ordering::Relaxed) {
                    if event::poll(interval)? {
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

    /// Returns one buffered event without blocking.
    pub fn try_read(&self) -> Option<Event> {
        self.rx.try_recv().ok()
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
