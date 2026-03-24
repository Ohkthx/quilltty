//! File: src/display/backend.rs
//!
//! Composes damaged pane data into `Frames` that are compared by the `Renderer`.

use std::io::{self, Write};

use crate::{Glyph, Rune, Style, display::Pane};

/// Convert from XY Coordinate system to index.
#[inline]
pub(crate) fn to_index(x: usize, y: usize, width: usize) -> usize {
    y * width + x
}

/// A span that has been marked as changed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DamagedSpan {
    pub(crate) dirty: bool,  // Marks if the span should be checked.
    pub(crate) start: usize, // Starting index of change.
    pub(crate) end: usize,   // Ending index of change.
}

impl DamagedSpan {
    /// Marks a single index as dirty.
    pub(crate) fn mark(&mut self, x: usize) {
        self.mark_range(x, x);
    }

    /// Marks an inclusive range as dirty.
    pub(crate) fn mark_range(&mut self, start: usize, end: usize) {
        if start > end {
            return;
        }

        if !self.dirty {
            self.start = start;
            self.end = end;
            self.dirty = true;
        } else {
            self.start = self.start.min(start);
            self.end = self.end.max(end);
        }
    }

    /// Clears the span, resetting it to not be dirty.
    pub(crate) fn clear(&mut self) {
        self.start = usize::MAX;
        self.end = 0;
        self.dirty = false;
    }
}

impl Default for DamagedSpan {
    fn default() -> Self {
        Self {
            dirty: false,
            start: usize::MAX,
            end: 0,
        }
    }
}

/// Snapshot a full output.
pub struct Frame {
    width: usize,     // Width (# of columns)
    height: usize,    // Height (# of rows)
    data: Vec<Glyph>, // Flattened output.
}

impl Frame {
    /// Initializes a new `Frame` with default data.
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![Glyph::default(); width * height],
        }
    }
}

/// Composes `Panes` onto a `Frame`.
pub struct Compositor {
    back: Frame, // Snapshot of panes to be displayed.
}

impl Compositor {
    /// Initializes a new `Compositor` for `Pane` compositions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            back: Frame::new(width, height),
        }
    }

    /// Width (in columns).
    pub fn width(&self) -> usize {
        self.back.width
    }

    /// Height (in rows).
    pub fn height(&self) -> usize {
        self.back.height
    }

    /// Applies the `Pane` data into the `back` buffer where `spans` are marked as dirty.
    pub(crate) fn flatten(&mut self, root: &Pane, panes: &[Pane], spans: &[DamagedSpan]) {
        let (width, height) = (self.back.width, self.back.height);

        for (y, span) in spans.iter().copied().enumerate().take(height) {
            if !span.dirty {
                continue;
            }

            // Clamp the damaged span to the screen width.
            // `x0` is inclusive, `x1` is exclusive for slice indexing.
            let x0 = span.start.min(width - 1);
            let x1 = span.end.min(width - 1) + 1;

            // Flat index where this screen row begins.
            let row_start = y * width;

            // Seed the damaged span in `back` from the root pane.
            self.back.data[row_start + x0..row_start + x1]
                .copy_from_slice(&root.data[row_start + x0..row_start + x1]);

            // Iterate only panes that can be rendered.
            for pane in panes {
                if !pane.visible {
                    continue;
                }

                // Pane vertical bounds in screen space: [py0, py1).
                let py0 = pane.rect.y;
                let py1 = py0.saturating_add(pane.height());

                // Skip panes that do not intersect this screen row.
                if y < py0 || y >= py1 {
                    continue;
                }

                // Pane horizontal bounds in screen space: [px0, px1).
                let px0 = pane.rect.x;
                let px1 = px0.saturating_add(pane.width());

                // Intersect the damaged span [x0, x1) with the pane span [px0, px1).
                let sx0 = x0.max(px0);
                let sx1 = x1.min(px1).min(width);

                // No horizontal overlap on this row.
                if sx0 >= sx1 {
                    continue;
                }

                // Convert the overlapping screen-space slice into pane-local coordinates.
                let src_y = y - py0;
                let src_x = sx0 - px0;
                let len = sx1 - sx0;

                // Compute flat source/destination indices.
                let src_idx = to_index(src_x, src_y, pane.width());
                let dst_idx = to_index(sx0, y, width);

                // Overlay the pane slice onto the composed back buffer.
                self.back.data[dst_idx..dst_idx + len]
                    .copy_from_slice(&pane.data[src_idx..src_idx + len]);
            }
        }
    }
}

/// Compares frames by calculating differences, writes output to terminal.
pub struct Renderer {
    front: Frame,    // Last rendered frame, the current displayed.
    buf: AnsiBuffer, // Output buffer that is written to terminal.
}

impl Renderer {
    /// Initializes a new `Renderer` calculates differences between frames and outputs the results.
    pub fn new(width: usize, height: usize, clear: bool) -> Self {
        let mut front = Frame::new(width, height);
        if clear {
            front.data.fill(Glyph::from('\0'));
        }

        Self {
            front,
            buf: AnsiBuffer::new(width * height * 12),
        }
    }

    /// Width (in columns).
    pub fn width(&self) -> usize {
        self.front.width
    }

    /// Height (in rows).
    pub fn height(&self) -> usize {
        self.front.height
    }

    /// Diffs the compositor back buffer against the current front buffer and
    /// writes only the damaged output to `out`.
    pub(crate) fn render<W: Write>(
        &mut self,
        compositor: &Compositor,
        spans: &[DamagedSpan],
        out: &mut W,
    ) -> io::Result<()> {
        debug_assert_eq!(self.width(), compositor.width());
        debug_assert_eq!(self.height(), compositor.height());

        self.buf.clear();

        let (front, buf) = (&mut self.front, &mut self.buf);
        Self::diff_damaged(front, &compositor.back, spans, buf);

        self.buf.extend(AnsiBuffer::RESET_STYLE);
        out.write_all(&self.buf.0)
    }

    /// Invalidates the cached front buffer so the next render rewrites everything.
    pub(crate) fn invalidate(&mut self) {
        self.front.data.fill(Glyph::default());
    }

    /// Compares only dirty spans between `front` and `back`, emits the minimal
    /// cursor/style/text updates, and updates `front` to match `back`.
    fn diff_damaged(front: &mut Frame, back: &Frame, spans: &[DamagedSpan], out: &mut AnsiBuffer) {
        let mut current_style = Style::default();

        let (width, height) = (front.width, front.height);
        if width == 0 || height == 0 {
            return;
        }

        debug_assert_eq!(width, back.width);
        debug_assert_eq!(height, back.height);

        for (y, span) in spans.iter().copied().enumerate().take(height) {
            if !span.dirty {
                continue;
            }

            // Starting x-coordinate for the row as in index.
            let row_start = y * width;

            let back_row = &back.data[row_start..row_start + width];
            let front_row = &mut front.data[row_start..row_start + width];

            // Starting and ending index for the span.
            let mut x = span.start.min(width - 1);
            let end = span.end.min(width - 1) + 1;

            while x < end {
                if front_row[x] == back_row[x] {
                    // Don't modify matching cells between front and back buffer.
                    x += 1;
                    continue;
                }

                // Cell does not match, get the length of the non-matching span.
                let run_start = x;
                while x < end && front_row[x] != back_row[x] {
                    x += 1;
                }

                // Move the cursor to the start before writing changes.
                out.push_move_sequence(run_start, y);

                let mut dx = run_start;
                while dx < x {
                    let style = back_row[dx].style;
                    if style != current_style {
                        // Update the style.
                        out.push_style(current_style, style);
                        current_style = style;
                    }

                    // Mark the current style and collect the span that remains the same.
                    let style_start = dx;
                    while dx < x && back_row[dx].style == style {
                        dx += 1;
                    }

                    // Push the span of same style & glyph into the write buffer.
                    for i in style_start..dx {
                        let cell = back_row[i];
                        out.push_rune(&cell.rune);
                        front_row[i] = cell; // Replicate to front buffer.
                    }
                }
            }
        }
    }
}

struct AnsiBuffer(Vec<u8>);

impl AnsiBuffer {
    const CSI: &[u8] = b"\x1b[";
    const CSI_MOVE_BASE_COST: usize = 4;
    const RESET_STYLE: &[u8] = b"\x1b[0m";
    const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
    const SHOW_CURSOR: &[u8] = b"\x1b[?25h";

    fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn extend(&mut self, slice: &[u8]) {
        self.0.extend_from_slice(slice);
    }

    /// Appends a decimal `usize` without allocation, using a fast two-digit table.
    fn push_usize(&mut self, mut n: usize) {
        const DIGITS_00_99: &[u8; 200] = b"00010203040506070809\
            10111213141516171819\
            20212223242526272829\
            30313233343536373839\
            40414243444546474849\
            50515253545556575859\
            60616263646566676869\
            70717273747576777879\
            80818283848586878889\
            90919293949596979899";

        if n < 10 {
            self.0.push(b'0' + n as u8);
            return;
        }

        let mut tmp = [0u8; 20];
        let mut i = 20;

        while n >= 100 {
            let rem = n % 100;
            n /= 100;

            i -= 2;
            tmp[i] = DIGITS_00_99[rem * 2];
            tmp[i + 1] = DIGITS_00_99[rem * 2 + 1];
        }

        if n < 10 {
            i -= 1;
            tmp[i] = b'0' + n as u8;
        } else {
            i -= 2;
            tmp[i] = DIGITS_00_99[n * 2];
            tmp[i + 1] = DIGITS_00_99[n * 2 + 1];
        }

        self.0.extend_from_slice(&tmp[i..]);
    }

    /// Emits the minimal SGR sequence needed to transition from `prev` to `new`.
    fn push_style(&mut self, prev: Style, new: Style) {
        if prev == new {
            return;
        }

        let mut first = true;

        let prev_flags = prev.flags();
        let new_flags = new.flags();

        let prev_fg = prev.fg();
        let prev_bg = prev.bg();

        let new_fg = new.fg();
        let new_bg = new.bg();

        self.0.extend_from_slice(Self::CSI);

        macro_rules! emit_num {
            ($num:expr) => {{
                if !first {
                    self.0.push(b';');
                }
                self.push_usize($num);
                first = false;
            }};
        }

        // Disable flags.

        if (prev_flags & (Style::FLAG_BOLD | Style::FLAG_DIM)) != 0
            && (new_flags & (Style::FLAG_BOLD | Style::FLAG_DIM)) == 0
        {
            emit_num!(22);
        }

        if (prev_flags & Style::FLAG_ITALIC != 0) && (new_flags & Style::FLAG_ITALIC == 0) {
            emit_num!(23);
        }

        if (prev_flags & Style::FLAG_UNDERLINE != 0) && (new_flags & Style::FLAG_UNDERLINE == 0) {
            emit_num!(24);
        }

        if (prev_flags & Style::FLAG_BLINK != 0) && (new_flags & Style::FLAG_BLINK == 0) {
            emit_num!(25);
        }

        if (prev_flags & Style::FLAG_STRIKE != 0) && (new_flags & Style::FLAG_STRIKE == 0) {
            emit_num!(29);
        }

        // Enable flags.

        if (new_flags & Style::FLAG_BOLD != 0) && (prev_flags & Style::FLAG_BOLD == 0) {
            emit_num!(1);
        }

        if (new_flags & Style::FLAG_DIM != 0) && (prev_flags & Style::FLAG_DIM == 0) {
            emit_num!(2);
        }

        if (new_flags & Style::FLAG_ITALIC != 0) && (prev_flags & Style::FLAG_ITALIC == 0) {
            emit_num!(3);
        }

        if (new_flags & Style::FLAG_UNDERLINE != 0) && (prev_flags & Style::FLAG_UNDERLINE == 0) {
            emit_num!(4);
        }

        if (new_flags & Style::FLAG_BLINK != 0) && (prev_flags & Style::FLAG_BLINK == 0) {
            emit_num!(5);
        }

        if (new_flags & Style::FLAG_STRIKE != 0) && (prev_flags & Style::FLAG_STRIKE == 0) {
            emit_num!(9);
        }

        // Colors.

        if prev_fg != new_fg {
            emit_num!(new_fg.fg_code() as usize);
        }

        if prev_bg != new_bg {
            emit_num!(new_bg.bg_code() as usize);
        }

        if !first {
            self.0.push(b'm');
        }
    }

    fn push_rune(&mut self, rune: &Rune) {
        self.0.extend_from_slice(&rune.bytes[..rune.len as usize]);
    }

    /// Moves the cursor to a 1-based terminal position.
    fn push_move_sequence(&mut self, x: usize, y: usize) {
        self.0.extend_from_slice(Self::CSI);
        self.push_usize(y + 1);
        self.0.push(b';');
        self.push_usize(x + 1);
        self.0.push(b'H');
    }
}
