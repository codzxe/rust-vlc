//! EIA-608 caption decode (card 5.N3 step 2) — the FULL channel
//! state machine: parity, field/channel routing, the control-code
//! families (PAC / mid-row / misc / tab), the 15×32 double-buffered
//! character grid, and the three presentation modes (pop-on,
//! roll-up, paint-on).
//!
//! # Model
//!
//! Carriage hands the decoder `(field, pair)` tuples; a field keeps
//! an **active-channel latch** — every code with a channel bit
//! (control `0x10..=0x1F` first bytes and PAC `0x40..=0x5F` first
//! bytes carry it in bit 3, `0x08`) re-latches it, plain text rides
//! the latched channel. Per channel ([`CcChannel`]: CC1–CC4):
//!
//! * two 15×32 [`CcGrid`]s — **displayed** and **non-displayed**;
//! * a write cursor (row, column) and a pen (color, underline,
//!   italics) set by PACs and mid-row codes;
//! * the presentation [`CaptionMode`].
//!
//! ## Presentation modes (CEA-608 §? "Modes of Caption Presentation")
//!
//! * **Pop-on** (`0x14 0x20` resume caption loading): text builds in
//!   the non-displayed grid; `0x14 0x2E` erases it;
//!   `0x14 0x2F` end-caption **swaps** the grids and clears the new
//!   non-displayed one; `0x14 0x2C` erases the displayed grid.
//! * **Roll-up** (`0x14 0x25/0x26/0x27` for 2/3/4 rows): text writes
//!   the displayed grid directly at the cursor;
//!   `0x14 0x2D` carriage-return scrolls the window (anchored at the
//!   PAC row) one row up and homes the cursor. Entering roll-up from
//!   a different mode clears both grids (the ccextractor behavior;
//!   ffmpeg's keep is a known divergence — documented choice).
//! * **Paint-on** (`0x14 0x29` resume direct captioning): text
//!   writes the displayed grid directly, no buffering.
//! * **Text mode** (`0x14 0x2A/0x2B`, the T1–T4 services): recorded
//!   as its own mode but rendered with paint-on direct-write
//!   semantics — an approximation, see the module honesty table in
//!   [`super`].
//!
//! ## Honesty notes (D-10)
//!
//! * The PAC row map is the `index[]` table shared by the reference
//!   decoders (ffmpeg/ccextractor): row =
//!   `TABLE16[((first & 0x07) << 1) | ((second & 0x20) >> 5)]`,
//!   1-based, `0` invalid. Bit 4 of the first byte (the redundant
//!   field tag in the analog signal) and bit 3 (the channel tag) do
//!   not select rows; the field comes from the carriage (`cc_type`)
//!   and the channel from bit 3.
//! * PAC indents (`second ≥ 0x60`): indent = `(second & 0x0F) × 4`
//!   columns, clamped into the 32-column grid; those second bytes
//!   always carry the odd-row selector bit, so indent PACs select a
//!   row of the odd parity. [VERIFY exact published slot map]
//! * The special-character tables (French/Spanish, German/Danish,
//!   Nordic) follow the CEA-608 extended sets as implemented by
//!   ccextractor; the rarely-hit empty slots render as spaces.
//!   [VERIFY exact slots against CTA-608-E Table 13]
//! * Basic `0x7F` renders as U+25AE (solid block). No auto-wrap:
//!   writes past column 31 clamp and overwrite the last cell (the
//!   reference decoders' behavior — captioners send explicit CRs).
//! * XDS (field-2 pairs with first byte ≤ `0x0F`) is skipped by the
//!   caller and counted in the damage ledger — parsing it is out of
//!   this card's scope (honest table).
//!
//! # Never
//!
//! Panics, unsafe, unbounded work — every input is data (MP §8).

/// Caption grid rows (CEA-608: 15 displayable rows).
pub const ROWS: usize = 15;

/// Caption grid columns (CEA-608: 32 columns).
pub const COLS: usize = 32;

/// Set the MSB of `b` so the byte carries odd parity (the EIA-608
/// line convention; also how the SCTE-20/708-F1 forms are
/// re-synthesized).
#[must_use]
pub const fn with_odd_parity(b: u8) -> u8 {
    let data = b & 0x7F;
    // ones in the 7 data bits: odd -> parity 0, even -> parity 1.
    if (data.count_ones() & 1) == 1 {
        data
    } else {
        data | 0x80
    }
}

/// Odd-parity check over the full byte (MSB included).
#[must_use]
pub const fn parity_ok(b: u8) -> bool {
    b.count_ones() & 1 == 1
}

/// The 608 text colors (mid-row / PAC attribute sets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CcColor {
    /// White (the default pen).
    #[default]
    White,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Cyan.
    Cyan,
    /// Red.
    Red,
    /// Yellow.
    Yellow,
    /// Magenta.
    Magenta,
    /// Black (rare; rendered as opaque black — see module docs).
    Black,
}

impl CcColor {
    /// The straight-alpha RGB triple for rendering.
    #[must_use]
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            CcColor::White => (255, 255, 255),
            CcColor::Green => (0, 255, 0),
            CcColor::Blue => (0, 0, 255),
            CcColor::Cyan => (0, 255, 255),
            CcColor::Red => (255, 0, 0),
            CcColor::Yellow => (255, 255, 0),
            CcColor::Magenta => (255, 0, 255),
            CcColor::Black => (0, 0, 0),
        }
    }
}

/// One grid cell: a character plus its pen attributes. `ch == ' '`
/// is a blank cell (transparent when rendered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CcCell {
    /// The cell character (basic/extended 608 charset).
    pub ch: char,
    /// The pen color active when the character was written.
    pub color: CcColor,
    /// Underline (rendered as a bar under the cell).
    pub underline: bool,
    /// Italics (carried in the model; the solid-block render cannot
    /// slant — recorded honestly).
    pub italic: bool,
}

impl CcCell {
    /// A blank cell (transparent space).
    #[must_use]
    pub const fn blank() -> CcCell {
        CcCell {
            ch: ' ',
            color: CcColor::White,
            underline: false,
            italic: false,
        }
    }
}

/// The 15×32 caption grid (one display buffer of one channel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcGrid {
    /// `ROWS * COLS` cells, row-major from the top row.
    pub cells: Vec<CcCell>,
}

impl Default for CcGrid {
    fn default() -> Self {
        CcGrid::new()
    }
}

impl CcGrid {
    /// An all-blank grid.
    #[must_use]
    pub fn new() -> CcGrid {
        CcGrid {
            cells: vec![CcCell::blank(); ROWS * COLS],
        }
    }

    fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = CcCell::blank();
        }
    }

    /// The cell at 0-based `(row, col)` — blank for out-of-range
    /// coordinates (defensive; internal writers stay in range).
    #[must_use]
    pub fn cell(&self, row: usize, col: usize) -> CcCell {
        if row < ROWS && col < COLS {
            self.cells[row * COLS + col]
        } else {
            CcCell::blank()
        }
    }

    fn set(&mut self, row: usize, col: usize, cell: CcCell) {
        if row < ROWS && col < COLS {
            self.cells[row * COLS + col] = cell;
        }
    }

    /// The visible text of one row, right-trimmed (test vocabulary
    /// and the classic `1\r2\r3` shape checks).
    #[must_use]
    pub fn row_text(&self, row: usize) -> String {
        let mut s: String = self.cells[row * COLS..row * COLS + COLS]
            .iter()
            .map(|c| c.ch)
            .collect();
        while s.ends_with(' ') {
            s.pop();
        }
        s
    }

    /// True when every cell is blank (a cleared screen).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| c.ch == ' ')
    }
}

/// The presentation mode of one channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionMode {
    /// Pop-on: writes buffer non-displayed; `0x2F` swaps.
    PopOn,
    /// Roll-up with 2/3/4 rows.
    RollUp(u8),
    /// Paint-on: direct writes to displayed.
    PaintOn,
    /// Text mode (T1–T4): paint-on-like approximation (module docs).
    Text,
}

/// The four logical 608 channels (field × data channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CcChannel {
    /// Field 1, channel 1 (the common caption service).
    CC1,
    /// Field 1, channel 2.
    CC2,
    /// Field 2, channel 1.
    CC3,
    /// Field 2, channel 2.
    CC4,
}

impl CcChannel {
    /// From the carriage field (`cc_type` 0/1) and the pair's channel
    /// bit.
    #[must_use]
    pub const fn new(field: u8, channel2: bool) -> CcChannel {
        match (field & 1, channel2) {
            (0, false) => CcChannel::CC1,
            (0, true) => CcChannel::CC2,
            (1, false) => CcChannel::CC3,
            (_, _) => CcChannel::CC4,
        }
    }
}

/// PAC row table (module docs): 1-based row for the
/// `((first & 0x07) << 1) | ((second & 0x20) >> 5)` index; 0 invalid.
const PAC_ROW: [u8; 16] = [11, 0, 1, 2, 3, 4, 12, 13, 14, 15, 5, 6, 7, 8, 9, 10];

/// Mid-row / PAC attribute nibble → (color, italics). The 0x20-0x2F
/// mid-row codes and the 0x40-0x5F PAC second bytes share this low
/// nibble map.
const ATTR_NIBBLE: [(CcColor, bool); 16] = [
    (CcColor::White, false),   // 0x20 / 0x40
    (CcColor::White, false),   // 0x21 / 0x41 underline
    (CcColor::Green, false),   // 0x22 / 0x42
    (CcColor::Green, false),   // 0x23 / 0x43 underline
    (CcColor::Blue, false),    // 0x24 / 0x44
    (CcColor::Blue, false),    // 0x25 / 0x45 underline
    (CcColor::Cyan, false),    // 0x26 / 0x46
    (CcColor::Cyan, false),    // 0x27 / 0x47 underline
    (CcColor::Red, false),     // 0x28 / 0x48
    (CcColor::Red, false),     // 0x29 / 0x49 underline
    (CcColor::Yellow, false),  // 0x2A / 0x4A
    (CcColor::Yellow, false),  // 0x2B / 0x4B underline
    (CcColor::Magenta, false), // 0x2C / 0x4C
    (CcColor::Magenta, false), // 0x2D / 0x4D underline
    (CcColor::White, true),    // 0x2E / 0x4E italics
    (CcColor::White, true),    // 0x2F / 0x4F italics underline
];

/// Extended characters, French/Spanish set: prefixes `0x11/0x19` +
/// `0x30..=0x3F` (the `0x20..0x2F` half of those prefixes is the
/// mid-row family). Empty slots fall back to a space.
const EXT_FR_ES: [char; 16] = [
    '®', '°', '½', '¿', '™', '¢', '£', 'à', 'è', 'ì', 'ò', 'ù', 'á', 'é', 'í', 'ó',
];

/// Extended characters, German/Danish set: prefixes `0x12/0x1A` +
/// `0x20..=0x3F`.
const EXT_DE: [char; 32] = [
    'ú', 'ç', 'ñ', 'ã', 'Ã', 'õ', 'Õ', '{', '}', '\\', '^', '_', '|', '~', ' ', ' ', 'Ä', 'ä', 'Ö',
    'ö', 'Ü', 'ü', 'ß', '¥', '¤', '§', '¶', '¿', ' ', ' ', ' ', '♪',
];

/// Extended characters, Nordic set: prefixes `0x13/0x1B` +
/// `0x20..=0x2F` (the `0x30..0x3F` half is unused).
const EXT_NORDIC: [char; 16] = [
    'Á', 'á', 'É', 'é', 'Í', 'í', 'Ó', 'ó', 'Ú', 'ú', 'Ñ', 'ñ', '°', '£', '¡', '¿',
];

/// The basic character set, 0x20..=0x7F (ASCII plus the solid block).
#[must_use]
pub const fn basic_char(byte: u8) -> char {
    match byte {
        0x20..=0x7E => byte as char,
        // 0x7F: the solid block (module docs).
        _ => '\u{25AE}',
    }
}

/// Per-pair damage flags surfaced to the decoder layer (read after
/// [`Cc608Channel::decode_pair`], cleared on the next pair).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PairFlags {
    /// A reserved control prefix (`0x10/0x18/0x16/0x1E` family).
    pub reserved_control: bool,
    /// A PAC with the invalid row slot.
    pub invalid_pac: bool,
}

/// One channel's full 608 state.
#[derive(Debug, Clone)]
pub(crate) struct Cc608Channel {
    /// The presentation mode.
    pub mode: CaptionMode,
    /// The visible grid.
    pub displayed: CcGrid,
    /// The pop-on staging grid.
    pub hidden: CcGrid,
    /// Damage flags from the LAST decoded pair.
    pub flags: PairFlags,
    cursor_row: usize,
    cursor_col: usize,
    pen: CcCell,
    rollup_base: usize,
}

impl Default for Cc608Channel {
    fn default() -> Self {
        Cc608Channel::new()
    }
}

impl Cc608Channel {
    /// A fresh channel: pop-on mode, blank grids, cursor home at
    /// row 15 / column 0, white pen.
    #[must_use]
    pub fn new() -> Cc608Channel {
        Cc608Channel {
            mode: CaptionMode::PopOn,
            displayed: CcGrid::new(),
            hidden: CcGrid::new(),
            flags: PairFlags::default(),
            cursor_row: ROWS - 1,
            cursor_col: 0,
            pen: CcCell::blank(),
            rollup_base: ROWS - 1,
        }
    }

    /// The current cursor (0-based row, column) — test vocabulary.
    #[cfg(test)]
    pub(crate) fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Feed one parity-stripped pair. Returns `true` when the
    /// DISPLAYED grid changed (the event signal); per-pair damage is
    /// in [`Self::flags`].
    pub(crate) fn decode_pair(&mut self, hi: u8, lo: u8) -> bool {
        self.flags = PairFlags::default();
        if (0x10..=0x1F).contains(&hi) {
            return self.control_code(hi & 0x07, lo);
        }
        if (0x40..=0x5F).contains(&hi) && lo >= 0x40 {
            return self.pac(hi, lo);
        }
        // Plain text (or a malformed pair): both bytes as characters.
        let mut changed = false;
        if (0x20..=0x7F).contains(&hi) {
            changed |= self.put_char(basic_char(hi));
        }
        if (0x20..=0x7F).contains(&lo) {
            changed |= self.put_char(basic_char(lo));
        }
        changed
    }

    /// The grid text writes land in (non-displayed for pop-on).
    fn write_target(&mut self) -> (&mut CcGrid, bool) {
        match self.mode {
            CaptionMode::PopOn => (&mut self.hidden, false),
            _ => (&mut self.displayed, true),
        }
    }

    /// Write one character at the cursor and advance (clamp at the
    /// last column, module docs).
    fn put_char(&mut self, ch: char) -> bool {
        let cell = CcCell {
            ch,
            color: self.pen.color,
            underline: self.pen.underline,
            italic: self.pen.italic,
        };
        let (row, col) = (self.cursor_row, self.cursor_col);
        let (grid, changed) = self.write_target();
        grid.set(row, col, cell);
        self.cursor_col = (col + 1).min(COLS - 1);
        changed
    }

    /// The misc-control family (`0x14/0x15/0x1C/0x1D` + second byte).
    fn control_code(&mut self, class: u8, lo: u8) -> bool {
        match class {
            4 | 5 => self.misc_control(lo),
            1 => self.mid_row(lo),
            2 | 3 => self.extended_char(class, lo),
            7 => self.tab_offset(lo),
            // 0x10/0x18/0x16/0x1E: reserved prefixes.
            _ => {
                self.flags.reserved_control = true;
                false
            }
        }
    }

    fn misc_control(&mut self, lo: u8) -> bool {
        match lo {
            0x20 => {
                // Resume caption loading.
                self.mode = CaptionMode::PopOn;
                false
            }
            0x21 => {
                // Backspace: erase behind the cursor.
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
                let (row, col) = (self.cursor_row, self.cursor_col);
                let (grid, changed) = self.write_target();
                grid.set(row, col, CcCell::blank());
                changed
            }
            0x24 => {
                // Delete to end of row.
                let (row, from) = (self.cursor_row, self.cursor_col);
                let (grid, changed) = self.write_target();
                for col in from..COLS {
                    grid.set(row, col, CcCell::blank());
                }
                changed
            }
            0x25..=0x27 => {
                let rows = lo - 0x23; // 2..=4
                if self.mode == CaptionMode::RollUp(rows) {
                    return false; // re-send: window size only
                }
                if !matches!(self.mode, CaptionMode::RollUp(_)) {
                    self.displayed.clear();
                    self.hidden.clear();
                }
                self.rollup_base = self.cursor_row;
                self.mode = CaptionMode::RollUp(rows);
                true // the entry clear changed the screen
            }
            0x29 => {
                // Resume direct captioning.
                self.mode = CaptionMode::PaintOn;
                false
            }
            0x2A => {
                // Text restart: text mode, cleared, home at the base.
                self.mode = CaptionMode::Text;
                self.displayed.clear();
                self.hidden.clear();
                self.rollup_base = ROWS - 1;
                self.cursor_row = ROWS - 1;
                self.cursor_col = 0;
                true
            }
            0x2B => {
                // Resume text displaying.
                self.mode = CaptionMode::Text;
                false
            }
            0x2C => {
                // Erase displayed memory.
                self.displayed.clear();
                true
            }
            0x2D => self.carriage_return(),
            0x2E => {
                // Erase non-displayed memory.
                self.hidden.clear();
                self.cursor_col = 0;
                false
            }
            0x2F => {
                // End caption: swap the double buffer.
                std::mem::swap(&mut self.displayed, &mut self.hidden);
                self.hidden.clear();
                self.cursor_col = 0;
                true
            }
            // 0x22/0x23 alarms, 0x28 flash: display no-ops.
            _ => false,
        }
    }

    fn carriage_return(&mut self) -> bool {
        if let CaptionMode::RollUp(rows) = self.mode {
            let rows = rows as usize;
            let base = self.rollup_base;
            let top = base.saturating_sub(rows - 1);
            // Scroll the window up one row: each row takes the one
            // below it; the base row is cleared.
            for row in top..base {
                for col in 0..COLS {
                    let below = self.displayed.cell(row + 1, col);
                    self.displayed.set(row, col, below);
                }
            }
            for col in 0..COLS {
                self.displayed.set(base, col, CcCell::blank());
            }
            self.cursor_row = base;
            self.cursor_col = 0;
            true
        } else {
            // Pop/paint/text: home the column.
            self.cursor_col = 0;
            matches!(self.mode, CaptionMode::PaintOn | CaptionMode::Text)
        }
    }

    /// The mid-row family (`0x11/0x19` + `0x20..=0x2F`): pen
    /// attributes plus a one-column advance. `0x30..=0x3F` of the
    /// same prefixes is the French/Spanish extended set.
    fn mid_row(&mut self, lo: u8) -> bool {
        match lo {
            0x20..=0x2F => {
                let (color, italic) = ATTR_NIBBLE[(lo - 0x20) as usize];
                self.pen = CcCell {
                    ch: ' ',
                    color,
                    underline: lo & 0x01 != 0,
                    italic,
                };
                // A mid-row code occupies one column.
                self.cursor_col = (self.cursor_col + 1).min(COLS - 1);
                false
            }
            0x30..=0x3F => self.put_char(EXT_FR_ES[(lo - 0x30) as usize]),
            _ => false,
        }
    }

    /// The extended sets: `0x12/0x1A` (German/Danish) and
    /// `0x13/0x1B` (Nordic) + `0x20..=0x3F`. A hostile second byte
    /// outside that range renders a blank — never a panic (found by
    /// the fuzz-lite sweep).
    fn extended_char(&mut self, class: u8, lo: u8) -> bool {
        let ch = if (0x20..0x40).contains(&lo) {
            match class {
                2 => EXT_DE[(lo - 0x20) as usize],
                _ if lo <= 0x2F => EXT_NORDIC[(lo - 0x20) as usize],
                _ => ' ',
            }
        } else {
            ' '
        };
        self.put_char(ch)
    }

    /// Tab offsets (`0x17/0x1F` + `0x21..=0x23`): 1–3 columns.
    fn tab_offset(&mut self, lo: u8) -> bool {
        match lo {
            0x21..=0x23 => {
                self.cursor_col = (self.cursor_col + (lo - 0x20) as usize).min(COLS - 1);
                false
            }
            _ => false,
        }
    }

    /// Preamble address codes: row + pen + optional indent.
    fn pac(&mut self, hi: u8, lo: u8) -> bool {
        let index = (((hi & 0x07) as usize) << 1) | (usize::from(lo & 0x20 != 0));
        let row = PAC_ROW[index];
        if row == 0 {
            self.flags.invalid_pac = true;
            return false; // invalid slot
        }
        self.cursor_row = row as usize - 1;
        if lo >= 0x60 {
            // Indent PAC: white pen, indent in 4-column units,
            // clamped into the grid.
            let indent = ((lo & 0x0F) as usize * 4).min(COLS - 1);
            self.pen = CcCell::blank();
            self.cursor_col = indent;
        } else {
            let (color, italic) = ATTR_NIBBLE[(lo & 0x0F) as usize];
            self.pen = CcCell {
                ch: ' ',
                color,
                underline: lo & 0x01 != 0,
                italic,
            };
            self.cursor_col = 0;
        }
        // A PAC positions the cursor; the displayed grid only changes
        // for the roll-up base row (the window anchor).
        if matches!(self.mode, CaptionMode::RollUp(_)) {
            self.rollup_base = self.cursor_row;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_helpers_are_inverse() {
        for b in 0u8..=0x7F {
            let full = with_odd_parity(b);
            assert!(parity_ok(full), "byte {full:#04x}");
            assert_eq!(full & 0x7F, b);
        }
        assert!(parity_ok(0x80)); // data 0: one set bit (the parity)
        assert!(!parity_ok(0x00));
        assert!(!parity_ok(0xFF)); // eight bits — even
    }

    #[test]
    fn pac_row_table_matches_reference_layout() {
        let ch = Cc608Channel::new();
        // Row 15 (the roll-up base): first 0x44, second D5 set.
        let mut ch = ch;
        ch.pac(0x44, 0x60);
        assert_eq!(ch.cursor(), (14, 0));
        ch.pac(0x44, 0x40);
        assert_eq!(ch.cursor(), (13, 0)); // row 14
        ch.pac(0x41, 0x40);
        assert_eq!(ch.cursor(), (0, 0)); // row 1
        ch.pac(0x41, 0x60);
        assert_eq!(ch.cursor(), (1, 0)); // row 2
        ch.pac(0x47, 0x40);
        assert_eq!(ch.cursor(), (8, 0)); // row 9
        // Indent PAC: 0x68 -> indent 8*4 = 32 -> clamped to 31.
        ch.pac(0x44, 0x68);
        assert_eq!(ch.cursor(), (14, 31));
    }

    /// A fresh channel in paint-on mode (direct display writes) —
    /// the handiest test bed for the code families.
    fn paint() -> Cc608Channel {
        let mut ch = Cc608Channel::new();
        ch.decode_pair(0x14, 0x29);
        ch
    }

    #[test]
    fn basic_and_extended_charset_tables() {
        assert_eq!(basic_char(b'A'), 'A');
        assert_eq!(basic_char(b' '), ' ');
        assert_eq!(basic_char(0x7F), '\u{25AE}');
        // Spot entries pinned per the module-docs tables.
        let mut ch = paint();
        ch.decode_pair(0x11, 0x30);
        assert_eq!(ch.displayed.cell(14, 0).ch, '®');
        let mut ch = paint();
        ch.decode_pair(0x12, 0x20);
        assert_eq!(ch.displayed.cell(14, 0).ch, 'ú');
        let mut ch = paint();
        ch.decode_pair(0x12, 0x30);
        assert_eq!(ch.displayed.cell(14, 0).ch, 'Ä');
        let mut ch = paint();
        ch.decode_pair(0x13, 0x21);
        assert_eq!(ch.displayed.cell(14, 0).ch, 'á');
        let mut ch = paint();
        ch.decode_pair(0x12, 0x3F);
        assert_eq!(ch.displayed.cell(14, 0).ch, '♪');
    }

    #[test]
    fn mid_row_sets_pen_and_takes_a_column() {
        let mut ch = paint();
        assert!(!ch.decode_pair(0x11, 0x28)); // red pen
        ch.decode_pair(0x31, 0x32); // "12"
        // The mid-row code advanced the cursor: "12" lands at cols 1-2.
        let cell = ch.displayed.cell(14, 1);
        assert_eq!((cell.ch, cell.color), ('1', CcColor::Red));
        assert_eq!(ch.displayed.cell(14, 2).ch, '2');
        assert_eq!(ch.displayed.cell(14, 3).ch, ' ');
        // White-with-underline variant.
        assert!(!ch.decode_pair(0x11, 0x21));
        assert!(ch.pen.underline);
        assert_eq!(ch.pen.color, CcColor::White);
    }

    #[test]
    fn roll_up_scrolls_the_window() {
        let mut ch = Cc608Channel::new();
        ch.decode_pair(0x14, 0x25); // RU2, entry clear
        ch.pac(0x44, 0x60); // base row 15
        ch.decode_pair(0x31, 0x00); // "1"
        assert_eq!(ch.displayed.row_text(14), "1");
        ch.decode_pair(0x14, 0x2D); // CR
        ch.decode_pair(0x32, 0x00); // "2"
        ch.decode_pair(0x14, 0x2D); // CR
        ch.decode_pair(0x33, 0x00); // "3"
        assert_eq!(ch.displayed.row_text(13), "2");
        assert_eq!(ch.displayed.row_text(14), "3");
        assert!(ch.displayed.row_text(12).is_empty());
    }

    #[test]
    fn pop_on_double_buffer_swap_and_erase() {
        let mut ch = Cc608Channel::new();
        ch.decode_pair(0x14, 0x20); // resume caption loading
        ch.decode_pair(0x31, 0x32); // "12" -> hidden
        assert!(ch.displayed.is_empty());
        assert_eq!(ch.hidden.row_text(14), "12");
        ch.decode_pair(0x14, 0x2F); // end caption: swap
        assert_eq!(ch.displayed.row_text(14), "12");
        assert!(ch.hidden.is_empty());
        // Erase displayed clears the screen.
        assert!(ch.decode_pair(0x14, 0x2C));
        assert!(ch.displayed.is_empty());
    }

    #[test]
    fn backspace_and_delete_to_end_of_row() {
        let mut ch = Cc608Channel::new();
        ch.decode_pair(0x14, 0x29); // paint-on
        ch.decode_pair(0x31, 0x32); // "12"
        ch.decode_pair(0x14, 0x21); // backspace: erase "2"
        assert_eq!(ch.displayed.row_text(14), "1");
        ch.decode_pair(0x33, 0x34); // "34"
        ch.decode_pair(0x14, 0x24); // delete to end of row
        assert_eq!(ch.displayed.row_text(14), "134");
    }

    #[test]
    fn text_is_written_with_the_pac_pen() {
        let mut ch = paint();
        ch.pac(0x42, 0x42); // row 3, green pen
        ch.decode_pair(0x31, 0x32);
        assert_eq!(ch.cursor(), (2, 2));
        assert_eq!(ch.displayed.cell(2, 0).color, CcColor::Green);
    }
}
