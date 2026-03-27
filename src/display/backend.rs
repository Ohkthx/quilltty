//! File: src/display/backend.rs
//!
//! Composes damaged pane data into `Frames` that are compared by the `Renderer`.

use std::io::{self, Write};

use crate::{
    Glyph, Rune, Style,
    display::{Pane, Point},
};

/// A span of damaged data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) start: usize,
    /// Start of the span.
    pub(crate) end: usize, // End of the span (exclusive).
}

impl Span {
    /// Creates a new span where end is exclusive.
    #[inline]
    pub(crate) fn new(start: usize, end: usize) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }
}

/// Represents an entire row that has damage.
#[derive(Debug, Clone, Default)]
pub(crate) enum DamagedRow {
    #[default]
    Clean,
    One(Span),
    Many(Vec<Span>),
}

impl DamagedRow {
    /// Returns `true` if the row contains any damaged span.
    #[inline]
    pub(crate) fn is_damaged(&self) -> bool {
        !matches!(self, Self::Clean)
    }

    /// Returns the spans for this row.
    #[inline]
    pub(crate) fn spans(&self) -> &[Span] {
        match self {
            Self::Clean => &[],
            Self::One(span) => std::slice::from_ref(span),
            Self::Many(spans) => spans.as_slice(),
        }
    }

    /// Clears all damage from this row.
    #[inline]
    pub(crate) fn clear(&mut self) {
        *self = Self::Clean;
    }

    /// Marks a single cell as damaged.
    #[inline]
    pub(crate) fn mark(&mut self, x: usize) {
        self.mark_range(x, x + 1);
    }

    /// Marks a span as damaged, where `end` is exclusive.
    pub(crate) fn mark_range(&mut self, start: usize, end: usize) {
        let Some(new_span) = Span::new(start, end) else {
            return;
        };

        match self {
            Self::Clean => {
                *self = Self::One(new_span);
            }

            Self::One(existing) => {
                // Disjoint with a real gap before.
                if new_span.end < existing.start {
                    *self = Self::Many(vec![new_span, *existing]);
                    return;
                }

                // Disjoint with a real gap after.
                if new_span.start > existing.end {
                    *self = Self::Many(vec![*existing, new_span]);
                    return;
                }

                // Overlapping or adjacent: merge.
                existing.start = existing.start.min(new_span.start);
                existing.end = existing.end.max(new_span.end);
            }

            Self::Many(spans) => {
                Self::insert_merged(spans, new_span);

                match spans.len() {
                    0 => *self = Self::Clean,
                    1 => *self = Self::One(spans[0]),
                    _ => {} // No change to structure.
                }
            }
        }
    }

    /// Inserts `new_span` into `spans`, keeping them sorted and merged.
    fn insert_merged(spans: &mut Vec<Span>, mut new_span: Span) {
        let mut i = 0;

        while i < spans.len() {
            let cur = spans[i];

            // New span is strictly before the current one, with a real gap.
            if new_span.end < cur.start {
                spans.insert(i, new_span);
                return;
            }

            // New span is strictly after the current one, with a real gap.
            if new_span.start > cur.end {
                i += 1;
                continue;
            }

            // Overlapping or adjacent: merge and continue consuming.
            new_span.start = new_span.start.min(cur.start);
            new_span.end = new_span.end.max(cur.end);
            spans.remove(i);
        }

        spans.push(new_span);
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
    #[inline]
    pub fn width(&self) -> usize {
        self.back.width
    }

    /// Height (in rows).
    #[inline]
    pub fn height(&self) -> usize {
        self.back.height
    }

    /// Applies the visible `Pane` data into the `back` buffer only where `spans`
    /// mark damage.
    pub(crate) fn flatten(&mut self, root: &Pane, panes: &[Pane], spans: &[DamagedRow]) {
        let (width, height) = (self.back.width, self.back.height);

        // Iterate each damaged row in screen space.
        for (y, row_damage) in spans.iter().enumerate().take(height) {
            // Process each damaged span within the row.
            for span in row_damage.spans() {
                let x0 = span.start.min(width);
                let x1 = span.end.min(width);

                if x0 >= x1 {
                    continue;
                }

                let row_start = y * width;

                // Seed the damaged slice from the root pane.
                self.back.data[row_start + x0..row_start + x1]
                    .copy_from_slice(&root.data[row_start + x0..row_start + x1]);

                // Overlay every visible pane that intersects this row/span.
                for pane in panes {
                    if !pane.visible {
                        continue;
                    }

                    // Pane vertical bounds in screen space: [py0, py1).
                    let py0 = pane.rect.y;
                    let py1 = py0.saturating_add(pane.height());
                    if y < py0 || y >= py1 {
                        continue;
                    }

                    // Pane horizontal bounds in screen space: [px0, px1).
                    let px0 = pane.rect.x;
                    let px1 = px0.saturating_add(pane.width());

                    // Intersect the damaged span [x0, x1) with the pane span [px0, px1).
                    let sx0 = x0.max(px0);
                    let sx1 = x1.min(px1).min(width);

                    if sx0 >= sx1 {
                        continue;
                    }

                    // Convert the overlapping screen-space slice into pane-local coordinates.
                    let src_y = y - py0;
                    let src_x = sx0 - px0;
                    let len = sx1 - sx0;

                    // Compute flat source/destination indices.
                    let src_idx = src_y * pane.width() + src_x;
                    let dst_idx = y * width + sx0;

                    // Overlay the pane slice onto the composed back buffer.
                    self.back.data[dst_idx..dst_idx + len]
                        .copy_from_slice(&pane.data[src_idx..src_idx + len]);
                }
            }
        }
    }
}

/// Compares frames by calculating differences, writes output to terminal.
pub struct Renderer {
    front: Frame,          // Last rendered frame, the current displayed.
    buf: AnsiBuffer,       // Output buffer that is written to terminal.
    cursor: Option<Point>, // Last rendered cursor position.
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
            cursor: None,
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
        spans: &[DamagedRow],
        cursor: Option<Point>,
        out: &mut W,
    ) -> io::Result<()> {
        let has_damage = spans.iter().any(|span| span.is_damaged());
        let cursor_changed = self.cursor != cursor;
        if !has_damage && !cursor_changed {
            return Ok(()); // Early exit with nothing to do.
        }

        self.buf.clear();

        if has_damage {
            let (front, buf) = (&mut self.front, &mut self.buf);
            Self::diff_damaged(front, &compositor.back, spans, buf);
        }

        self.buf.extend(AnsiBuffer::RESET_STYLE);

        match cursor {
            Some(Point { x, y }) => {
                self.buf.push_move_sequence(x, y);
                self.buf.extend(AnsiBuffer::SHOW_CURSOR);
            }
            None => {
                self.buf.extend(AnsiBuffer::HIDE_CURSOR);
            }
        }

        out.write_all(&self.buf.0)?;
        self.cursor = cursor;
        Ok(())
    }

    /// Compares only damaged spans between `front` and `back`, emits the minimal
    /// cursor/style/text updates, and updates `front` to match `back`.
    fn diff_damaged(front: &mut Frame, back: &Frame, spans: &[DamagedRow], out: &mut AnsiBuffer) {
        // Tracks the style currently active in the terminal so redundant style
        // sequences are avoided.
        let mut current_style = Style::default();

        let (width, height) = (front.width, front.height);
        if width == 0 || height == 0 {
            return;
        }

        debug_assert_eq!(width, back.width);
        debug_assert_eq!(height, back.height);

        // Iterate each damaged row in screen space.
        for (y, row_damage) in spans.iter().enumerate().take(height) {
            let row_start = y * width;

            // Borrow the aligned rows from each frame.
            let back_row = &back.data[row_start..row_start + width];
            let front_row = &mut front.data[row_start..row_start + width];

            // Compare only the damaged spans for this row.
            for span in row_damage.spans() {
                let mut x = span.start.min(width);
                let end = span.end.min(width);

                // Walk the damaged span from left to right.
                while x < end {
                    // Skip cells that are already identical.
                    if front_row[x] == back_row[x] {
                        x += 1;
                        continue;
                    }

                    // Find a contiguous run of differing cells.
                    let run_start = x;
                    while x < end && front_row[x] != back_row[x] {
                        x += 1;
                    }

                    // Move the cursor once to the start of the changed run.
                    out.push_move_sequence(run_start, y);

                    let mut dx = run_start;
                    while dx < x {
                        let style = back_row[dx].style;

                        // Emit a style sequence only when the style changes.
                        if style != current_style {
                            out.push_style(current_style, style);
                            current_style = style;
                        }

                        // Group adjacent cells that share the same style.
                        let style_start = dx;
                        while dx < x && back_row[dx].style == style {
                            dx += 1;
                        }

                        // Write the glyphs for this style run and mirror them into
                        // `front` so the front buffer stays in sync with the terminal.
                        for i in style_start..dx {
                            let cell = back_row[i];
                            out.push_rune(&cell.rune);
                            front_row[i] = cell;
                        }
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
