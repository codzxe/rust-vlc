//! CUE sheet parsing with album-oriented gapless semantics (card 5.O2).
//!
//! # Format model
//!
//! Line-based text (LF or CRLF; a leading UTF-8 BOM is tolerated and
//! skipped). Commands are case-insensitive; blank lines are ignored.
//!
//! * Session scope (before the first `TRACK`): `CATALOG`, `CDTEXTFILE`,
//!   `TITLE`, `PERFORMER`, `SONGWRITER`, `REM`.
//! * `FILE "<name>" <TYPE>` — opens a file context. `TYPE` is one of the
//!   CDRWIN set (`BINARY`, `MOTOROLA`, `AIFF`, `WAVE`, `MP3`) or a later
//!   community extension (`FLAC`, `APE`, …); unknown tokens are kept
//!   verbatim as [`CueFileKind::Other`]. An unquoted name is accepted by
//!   taking the *last* whitespace-delimited token as the type.
//! * `TRACK <n> <TYPE>` — `n` is 1..=99; `AUDIO` plus the data modes
//!   (`MODE1/2048`, …). Every `INDEX` belongs to the `FILE` context open
//!   at its own line (per spec: INDEX offsets are relative to the current
//!   FILE), and a track's own file is the file of its `INDEX 01`.
//! * `INDEX <nn> MM:SS:FF` — 75 fps Red Book frames (`FF` 0..=74,
//!   `SS` 0..=59, `MM` 0..=99). `INDEX 00` starts the (in-file) pregap,
//!   `INDEX 01` starts the track proper; higher index numbers are kept
//!   verbatim on the track.
//! * `PREGAP`/`POSTGAP MM:SS:FF` — gaps **not** stored in the file
//!   (silence a player inserts); `FLAGS DCP|4CH|PRE|SCMS`;
//!   `ISRC`; track-scoped `TITLE`/`PERFORMER`/`SONGWRITER`/`REM`.
//!
//! # Gapless window semantics (the reason this module exists)
//!
//! Each track's **play window** is `[INDEX 01, next INDEX 01)` — the
//! pregap is *excluded from the track's own duration* but exposed
//! separately, because in the album-oriented model the INDEX-00..INDEX-01
//! span is the tail of continuous playback (usually inside the previous
//! track's window):
//!
//! * `start_ns` — the track's `INDEX 01` offset into its FILE,
//!   nanoseconds.
//! * `duration_ns` — `next INDEX 01 − this INDEX 01`, but only when the
//!   next track shares the FILE (the single-FILE gapless album: windows
//!   tile the file exactly, so playback joins are zero-gap by
//!   construction). The last track of each FILE has `duration_ns: None` —
//!   the CUE does not know the file's length and this parser does not
//!   invent it.
//! * `pregap_ns` — `INDEX 01 − INDEX 00` when an in-file pregap exists
//!   (`pregap_synthetic == false`), otherwise the `PREGAP` command value
//!   (`pregap_synthetic == true`).
//! * `postgap_ns` — the `POSTGAP` command value.
//! * `gapless_join` — the previous track is in the same FILE and ends at
//!   or before this start (the zero-gap handoff).
//!
//! Frames are the source of truth (1/75 s); nanosecond fields are
//! round-to-nearest conversions ([`frames_to_ns`], error < 1 ns).
//!
//! # Dirty-world behavior (D-1/D-2, matching the M3U discipline)
//!
//! * Malformed `MM:SS:FF` (`MM > 99`, `SS > 59`, `FF > 74`, non-numeric,
//!   wrong part count) — typed [`CueWarning::BadTime`], line ignored.
//! * A track that never gets a valid `INDEX 01` — damage-note
//!   ([`CueWarning::MissingIndex01`]) **and the track is skipped**; a CUE
//!   track without a start point has no honest window.
//! * `TRACK` before any `FILE`, unknown commands, unknown `FLAGS` tokens,
//!   missing arguments, bad track/index numbers — typed warnings, parse
//!   continues.
//! * Backwards `INDEX 01` (next start before this start) —
//!   [`CueWarning::NonMonotonicIndex`], the earlier window is emptied to
//!   zero (`duration_ns = Some(0)`), `gapless_join` false.
//! * A line over [`MAX_LINE_BYTES`](crate::MAX_LINE_BYTES) is fatal
//!   (D-1); non-UTF-8 bytes are fatal via [`parse_cue_bytes`].
//! * No panics on any byte input — pinned by the fuzz-lite suite.
//!
//! # Example
//!
//! ```
//! use playlists::parse_cue;
//!
//! # fn main() -> Result<(), playlists::ParseError> {
//! let src = "\
//! FILE \"album.flac\" FLAC
//!   TRACK 01 AUDIO
//!     INDEX 01 00:00:00
//!   TRACK 02 AUDIO
//!     INDEX 00 03:00:00
//!     INDEX 01 03:02:25
//! ";
//! let sheet = parse_cue(src)?;
//! let w = sheet.track_windows();
//! assert_eq!(w.len(), 2);
//! // Track 1's window runs to track 2's INDEX 01 (13 675 frames).
//! assert_eq!(w[0].duration_ns, Some(182_333_333_333));
//! // Track 2's pregap (INDEX 00..01 = 175 frames) is excluded from
//! // track 1's *start* and exposed on track 2 instead.
//! assert_eq!(w[1].pregap_ns, Some(2_333_333_333));
//! assert!(!w[1].pregap_synthetic);
//! // Zero-gap join: window 1 ends exactly where window 2 begins.
//! assert_eq!(w[0].start_ns + w[0].duration_ns.unwrap_or(0), w[1].start_ns);
//! # Ok(())
//! # }
//! ```

use crate::error::ParseError;
use crate::m3u::MAX_LINE_BYTES;
use std::fmt;

/// Red Book frames per second — the CUE time base (`MM:SS:FF`).
pub const FRAMES_PER_SECOND: u64 = 75;

/// Nanoseconds per frame is not an integer (1e9/75); convert with
/// round-to-nearest so accumulated offsets stay within 1 ns of truth.
#[must_use]
pub fn frames_to_ns(frames: u64) -> u64 {
    // 1_000_000_000 % 75 == 25, so fractional parts are only ever
    // 0, 1/3 or 2/3 of a nanosecond — never exactly 1/2 — and `+ 37`
    // (floor(75/2)) is a correct round-to-nearest.
    (frames.saturating_mul(1_000_000_000) + 37) / FRAMES_PER_SECOND
}

/// The FILE a CUE points at, with its declared type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueFile {
    /// File name exactly as written (between the quotes, or the unquoted
    /// token run). Never resolved or normalized — caller's policy.
    pub name: String,
    /// Declared type. `Unspecified` when the sheet omitted it.
    pub kind: CueFileKind,
}

/// Known CUE FILE types; unknown tokens are kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueFileKind {
    /// `WAVE`
    Wave,
    /// `AIFF`
    Aiff,
    /// `MP3`
    Mp3,
    /// `FLAC` (community extension)
    Flac,
    /// `BINARY` (raw CD image)
    Binary,
    /// `MOTOROLA`
    Motorola,
    /// The sheet named no type (typed warning accompanies this).
    Unspecified,
    /// Any other token, verbatim (e.g. `APE`, `OGG`, `W64`).
    Other(String),
}

impl CueFileKind {
    fn from_token(token: &str) -> Self {
        match token.to_ascii_uppercase().as_str() {
            "WAVE" => Self::Wave,
            "AIFF" => Self::Aiff,
            "MP3" => Self::Mp3,
            "FLAC" => Self::Flac,
            "BINARY" => Self::Binary,
            "MOTOROLA" => Self::Motorola,
            _ => Self::Other(token.to_string()),
        }
    }
}

/// One INDEX point of a track: `(number, offset, owning file)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CueIndex {
    /// INDEX number as written (00 pregap start, 01 track start, higher
    /// numbers are extra index points, kept verbatim).
    pub number: u8,
    /// Offset from the start of `file`'s data, in 1/75 s frames.
    pub frames: u64,
    /// Index into [`CueSheet::files`] — the FILE context this INDEX was
    /// written under.
    pub file: usize,
}

/// TRACK datatype. Only `Audio` is playable by Lumen; data modes are kept
/// verbatim for round-tripping tools that care.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueTrackType {
    /// `AUDIO`
    Audio,
    /// `MODE1/2048`
    Mode1_2048,
    /// `MODE1/2352`
    Mode1_2352,
    /// `MODE2/2336`
    Mode2_2336,
    /// `MODE2/2352`
    Mode2_2352,
    /// `CDI/2336`
    Cdi2336,
    /// `CDI/2352`
    Cdi2352,
    /// The sheet named no datatype (typed warning accompanies this).
    Unspecified,
    /// Any other token, verbatim.
    Other(String),
}

impl CueTrackType {
    fn from_token(token: &str) -> Self {
        match token.to_ascii_uppercase().as_str() {
            "AUDIO" => Self::Audio,
            "MODE1/2048" => Self::Mode1_2048,
            "MODE1/2352" => Self::Mode1_2352,
            "MODE2/2336" => Self::Mode2_2336,
            "MODE2/2352" => Self::Mode2_2352,
            "CDI/2336" => Self::Cdi2336,
            "CDI/2352" => Self::Cdi2352,
            _ => Self::Other(token.to_string()),
        }
    }
}

/// Subcode FLAGS a track declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueFlag {
    /// `DCP` — digital copy permitted.
    Dcp,
    /// `4CH` — four-channel audio.
    FourCh,
    /// `PRE` — pre-emphasis.
    Pre,
    /// `SCMS` — serial copy management system.
    Scms,
}

impl CueFlag {
    fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "DCP" => Some(Self::Dcp),
            "4CH" => Some(Self::FourCh),
            "PRE" => Some(Self::Pre),
            "SCMS" => Some(Self::Scms),
            _ => None,
        }
    }
}

/// A parsed CUE track, verbatim fields plus the derived INDEX-01 binding.
/// Tracks that never obtain a valid `INDEX 01` are skipped at parse time
/// with a [`CueWarning::MissingIndex01`] damage-note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueTrack {
    /// Track number as written (1..=99).
    pub number: u32,
    /// Datatype as written.
    pub track_type: CueTrackType,
    /// Track-scoped `TITLE`.
    pub title: Option<String>,
    /// Track-scoped `PERFORMER`.
    pub performer: Option<String>,
    /// Track-scoped `SONGWRITER`.
    pub songwriter: Option<String>,
    /// Track-scoped `ISRC`.
    pub isrc: Option<String>,
    /// `FLAGS` in source order.
    pub flags: Vec<CueFlag>,
    /// Every INDEX point in source order (00 and 01 included).
    pub indexes: Vec<CueIndex>,
    /// `PREGAP MM:SS:FF` in frames — silence NOT stored in the file.
    pub pregap_frames: Option<u64>,
    /// `POSTGAP MM:SS:FF` in frames — silence NOT stored in the file.
    pub postgap_frames: Option<u64>,
    /// Track-scoped `REM` pairs in source order.
    pub rem: Vec<(String, String)>,
    /// Index into [`CueSheet::files`]: the FILE this track's `INDEX 01`
    /// was written under.
    pub file: usize,
    /// `INDEX 01` offset in frames (present by construction — tracks
    /// without it are skipped, not stored).
    pub start_frames: u64,
}

/// The album-oriented playback window of one track (see module docs for
/// the arithmetic and the pregap-exclusion rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackWindow {
    /// Index into [`CueSheet::tracks`] — `windows[i].track == i`.
    pub track: usize,
    /// Track number as written.
    pub number: u32,
    /// Index into [`CueSheet::files`] — the FILE this window addresses.
    pub file: usize,
    /// `INDEX 01` offset into the FILE, nanoseconds.
    pub start_ns: u64,
    /// Window length `[start_ns, next INDEX 01)` — the play duration with
    /// the pregap excluded. `None` when no following track shares the FILE
    /// (the CUE cannot know the file's own length).
    pub duration_ns: Option<u64>,
    /// Pregap length, nanoseconds, excluded from `duration_ns`. In-file
    /// (`INDEX 00..01`) when `pregap_synthetic` is false; the `PREGAP`
    /// command value (inserted silence) when true.
    pub pregap_ns: Option<u64>,
    /// Whether `pregap_ns` came from the `PREGAP` command rather than
    /// in-file INDEX arithmetic.
    pub pregap_synthetic: bool,
    /// `POSTGAP` command value, nanoseconds (inserted silence).
    pub postgap_ns: Option<u64>,
    /// The previous track is in the same FILE and does not overrun this
    /// start — the zero-gap handoff of album-oriented gapless playback.
    pub gapless_join: bool,
}

/// A parsed CUE sheet: verbatim session data, files, surviving tracks,
/// and the derived [`TrackWindow`] set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CueSheet {
    /// Session `CATALOG` (media catalogue number).
    pub catalog: Option<String>,
    /// Session `CDTEXTFILE`.
    pub cdtextfile: Option<String>,
    /// Session `TITLE` (album title).
    pub title: Option<String>,
    /// Session `PERFORMER` (album artist).
    pub performer: Option<String>,
    /// Session `SONGWRITER`.
    pub songwriter: Option<String>,
    /// Session-scoped `REM` pairs in source order.
    pub rem: Vec<(String, String)>,
    /// FILE contexts in source order.
    pub files: Vec<CueFile>,
    /// Tracks that obtained a valid `INDEX 01`, in source order.
    pub tracks: Vec<CueTrack>,
    /// One window per entry of `tracks` (same order).
    pub windows: Vec<TrackWindow>,
    /// Dirty-world findings in source order (arithmetic-stage findings
    /// appended after line-stage ones).
    pub warnings: Vec<CueWarning>,
}

impl CueSheet {
    /// The window set ([`Self::windows`], the album-oriented gapless
    /// handoff).
    #[must_use]
    pub fn track_windows(&self) -> &[TrackWindow] {
        &self.windows
    }

    /// The FILE a window/track's `file` index addresses.
    #[must_use]
    pub fn file_of(&self, index: usize) -> Option<&CueFile> {
        self.files.get(index)
    }
}

/// Which command a malformed `MM:SS:FF` belonged to (redaction-safe
/// warning context — no line text is embedded, MP §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeContext {
    /// An `INDEX` line.
    Index,
    /// A `PREGAP` line.
    Pregap,
    /// A `POSTGAP` line.
    Postgap,
}

impl fmt::Display for TimeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Index => "INDEX",
            Self::Pregap => "PREGAP",
            Self::Postgap => "POSTGAP",
        })
    }
}

/// Non-fatal dirty-world findings (D-2). Line numbers are 1-based; the
/// arithmetic-stage findings (`BadPregap`, `NonMonotonicIndex`) name the
/// track instead, since they are only decidable after the sheet closes.
/// No line content, names or paths are ever embedded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CueWarning {
    /// A line's first token matched no known CUE command.
    #[error("playlists: line {line}: unrecognized CUE command (line ignored)")]
    UnknownCommand {
        /// 1-based line number.
        line: usize,
    },

    /// A known command appeared with no argument at all.
    #[error("playlists: line {line}: command needs an argument (line ignored)")]
    MissingArgument {
        /// 1-based line number.
        line: usize,
    },

    /// `TRACK` number missing, non-numeric, or outside 1..=99.
    #[error("playlists: line {line}: malformed TRACK number (track skipped)")]
    BadTrackNumber {
        /// 1-based line number.
        line: usize,
    },

    /// `TRACK` named no datatype.
    #[error("playlists: line {line}: TRACK without a datatype (kept, datatype unspecified)")]
    TrackWithoutType {
        /// 1-based line number.
        line: usize,
    },

    /// `TRACK` appeared before any `FILE` opened a context.
    #[error("playlists: line {line}: TRACK with no open FILE (track skipped)")]
    TrackWithoutFile {
        /// 1-based line number.
        line: usize,
    },

    /// `FILE` named no type token.
    #[error("playlists: line {line}: FILE without a type (kept, type unspecified)")]
    FileWithoutType {
        /// 1-based line number.
        line: usize,
    },

    /// `INDEX` number missing, non-numeric, or above 99.
    #[error("playlists: line {line}: malformed INDEX number (line ignored)")]
    BadIndexNumber {
        /// 1-based line number.
        line: usize,
    },

    /// `MM:SS:FF` malformed: non-numeric part, wrong part count, or out
    /// of range (`MM > 99`, `SS > 59`, `FF > 74`).
    #[error("playlists: line {line}: malformed {context} time MM:SS:FF (line ignored)")]
    BadTime {
        /// 1-based line number.
        line: usize,
        /// The command the bad time belonged to.
        context: TimeContext,
    },

    /// A track closed (next TRACK or EOF) without any valid `INDEX 01`:
    /// the damage is noted and the track is skipped — a CUE track with
    /// no start point has no honest window.
    #[error(
        "playlists: line {line}: TRACK completed without INDEX 01 (damage noted, track skipped)"
    )]
    MissingIndex01 {
        /// 1-based line number of the TRACK line.
        line: usize,
    },

    /// A track-scoped command (`INDEX`, `PREGAP`, `POSTGAP`, `FLAGS`,
    /// `ISRC`) appeared outside any open track.
    #[error("playlists: line {line}: track-scoped command outside any TRACK (line ignored)")]
    CommandOutsideTrack {
        /// 1-based line number.
        line: usize,
    },

    /// A `FLAGS` token matched none of DCP/4CH/PRE/SCMS.
    #[error("playlists: line {line}: unknown FLAGS token (token ignored)")]
    UnknownFlag {
        /// 1-based line number.
        line: usize,
    },

    /// The track's INDEX 00 sits after its INDEX 01 or under a different
    /// FILE, so the in-file pregap is not computable; the pregap is
    /// dropped (the PREGAP command value, if any, remains on the track).
    #[error("playlists: track {track}: pregap unusable (INDEX 00/01 inconsistent, pregap dropped)")]
    BadPregap {
        /// Track number as written.
        track: u32,
    },

    /// The next track's INDEX 01 lies before this track's, so the window
    /// would be negative; it is emptied to zero instead.
    #[error("playlists: track {track}: INDEX 01 goes backwards (window emptied)")]
    NonMonotonicIndex {
        /// Track number as written (the offending track).
        track: u32,
    },
}

/// Parses a CUE sheet. Fatal only for the D-1 line-cap violation; every
/// other defect degrades to a typed [`CueWarning`].
pub fn parse_cue(input: &str) -> Result<CueSheet, ParseError> {
    // BOM tolerance: exactly one leading U+FEFF (matching the M3U rule).
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);

    let mut sheet = CueSheet::default();
    let mut cur_file: Option<usize> = None;
    let mut open: Option<TrackBuilder> = None;

    let mut rest: &str = input;
    let mut line_no: usize = 0;
    while !rest.is_empty() {
        line_no += 1;
        let (raw, tail): (&str, &str) = match rest.as_bytes().iter().position(|&b| b == b'\n') {
            Some(idx) => {
                let (line, after) = rest.split_at(idx);
                (line, &after[1..])
            }
            None => (rest, ""),
        };
        if raw.len() > MAX_LINE_BYTES {
            return Err(ParseError::LineTooLong {
                line: line_no,
                length: raw.len(),
            });
        }
        rest = tail;

        let line = raw.strip_suffix('\r').unwrap_or(raw).trim();
        if line.is_empty() {
            continue;
        }
        let (cmd, body) = split_first_token(line);
        let up = cmd.to_ascii_uppercase();
        match up.as_str() {
            "REM" => {
                let body = body.trim();
                let (key, value) = split_first_token(body);
                if key.is_empty() {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                }
                let pair = (key.to_string(), unquote(value).to_string());
                if let Some(t) = &mut open {
                    t.rem.push(pair);
                } else {
                    sheet.rem.push(pair);
                }
            }
            "CATALOG" => {
                let body = body.trim();
                if body.is_empty() {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                } else {
                    sheet.catalog = Some(unquote(body).to_string());
                }
            }
            "CDTEXTFILE" => {
                let body = body.trim();
                if body.is_empty() {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                } else {
                    sheet.cdtextfile = Some(unquote(body).to_string());
                }
            }
            "FILE" => {
                let body = body.trim();
                match parse_file_spec(body) {
                    Some((name, kind_tok)) => {
                        let kind = match kind_tok {
                            Some(tok) => CueFileKind::from_token(&tok),
                            None => {
                                sheet
                                    .warnings
                                    .push(CueWarning::FileWithoutType { line: line_no });
                                CueFileKind::Unspecified
                            }
                        };
                        cur_file = Some(sheet.files.len());
                        sheet.files.push(CueFile { name, kind });
                    }
                    None => {
                        sheet
                            .warnings
                            .push(CueWarning::MissingArgument { line: line_no });
                    }
                }
            }
            "TRACK" => {
                // The previous track closes the moment a new TRACK opens,
                // regardless of this line's own validity.
                finalize_track(&mut sheet, open.take());
                let body = body.trim();
                let mut tokens = body.split_whitespace();
                let Some(num_tok) = tokens.next() else {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                };
                let number = match num_tok.parse::<u32>() {
                    Ok(n) if (1..=99).contains(&n) => n,
                    _ => {
                        sheet
                            .warnings
                            .push(CueWarning::BadTrackNumber { line: line_no });
                        continue;
                    }
                };
                let track_type = match tokens.next() {
                    Some(tok) => CueTrackType::from_token(tok),
                    None => {
                        sheet
                            .warnings
                            .push(CueWarning::TrackWithoutType { line: line_no });
                        CueTrackType::Unspecified
                    }
                };
                match cur_file {
                    Some(_) => {
                        open = Some(TrackBuilder {
                            number,
                            track_type,
                            open_line: line_no,
                            title: None,
                            performer: None,
                            songwriter: None,
                            isrc: None,
                            flags: Vec::new(),
                            indexes: Vec::new(),
                            pregap: None,
                            postgap: None,
                            rem: Vec::new(),
                        });
                    }
                    None => {
                        sheet
                            .warnings
                            .push(CueWarning::TrackWithoutFile { line: line_no });
                    }
                }
            }
            "INDEX" => {
                let mut tokens = body.split_whitespace();
                let Some(num_tok) = tokens.next() else {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                };
                let number = match num_tok.parse::<u8>() {
                    Ok(n) if n <= 99 => n,
                    _ => {
                        sheet
                            .warnings
                            .push(CueWarning::BadIndexNumber { line: line_no });
                        continue;
                    }
                };
                let Some(time_tok) = tokens.next() else {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                };
                let Some(frames) = parse_time(time_tok) else {
                    sheet.warnings.push(CueWarning::BadTime {
                        line: line_no,
                        context: TimeContext::Index,
                    });
                    continue;
                };
                match (&mut open, cur_file) {
                    (Some(t), Some(file)) => t.indexes.push(CueIndex {
                        number,
                        frames,
                        file,
                    }),
                    _ => {
                        sheet
                            .warnings
                            .push(CueWarning::CommandOutsideTrack { line: line_no });
                    }
                }
            }
            "PREGAP" | "POSTGAP" => {
                let context = if up == "PREGAP" {
                    TimeContext::Pregap
                } else {
                    TimeContext::Postgap
                };
                let mut tokens = body.split_whitespace();
                let Some(time_tok) = tokens.next() else {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                };
                let Some(frames) = parse_time(time_tok) else {
                    sheet.warnings.push(CueWarning::BadTime {
                        line: line_no,
                        context,
                    });
                    continue;
                };
                match &mut open {
                    Some(t) => {
                        if context == TimeContext::Pregap {
                            t.pregap = Some(frames);
                        } else {
                            t.postgap = Some(frames);
                        }
                    }
                    None => {
                        sheet
                            .warnings
                            .push(CueWarning::CommandOutsideTrack { line: line_no });
                    }
                }
            }
            "FLAGS" => {
                let tokens: Vec<&str> = body.split_whitespace().collect();
                if tokens.is_empty() {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                }
                match &mut open {
                    Some(t) => {
                        for tok in tokens {
                            match CueFlag::from_token(tok) {
                                Some(flag) => t.flags.push(flag),
                                None => {
                                    sheet
                                        .warnings
                                        .push(CueWarning::UnknownFlag { line: line_no });
                                }
                            }
                        }
                    }
                    None => {
                        sheet
                            .warnings
                            .push(CueWarning::CommandOutsideTrack { line: line_no });
                    }
                }
            }
            "TITLE" | "PERFORMER" | "SONGWRITER" => {
                let body = body.trim();
                if body.is_empty() {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                }
                let value = unquote(body).to_string();
                match &mut open {
                    Some(t) => match up.as_str() {
                        "TITLE" => t.title = Some(value),
                        "PERFORMER" => t.performer = Some(value),
                        _ => t.songwriter = Some(value),
                    },
                    None => match up.as_str() {
                        "TITLE" => sheet.title = Some(value),
                        "PERFORMER" => sheet.performer = Some(value),
                        _ => sheet.songwriter = Some(value),
                    },
                }
            }
            "ISRC" => {
                let body = body.trim();
                if body.is_empty() {
                    sheet
                        .warnings
                        .push(CueWarning::MissingArgument { line: line_no });
                    continue;
                }
                match &mut open {
                    Some(t) => t.isrc = Some(body.to_string()),
                    None => {
                        sheet
                            .warnings
                            .push(CueWarning::CommandOutsideTrack { line: line_no });
                    }
                }
            }
            _ => {
                sheet
                    .warnings
                    .push(CueWarning::UnknownCommand { line: line_no });
            }
        }
    }
    finalize_track(&mut sheet, open.take());

    // Second pass: window arithmetic (gapless semantics). Findings here
    // are appended after every line-stage warning.
    let mut windows = Vec::with_capacity(sheet.tracks.len());
    let mut late: Vec<CueWarning> = Vec::new();
    for (i, tr) in sheet.tracks.iter().enumerate() {
        let file = tr.file;
        let start = tr.start_frames;

        // Pregap: in-file INDEX 00..01 arithmetic first, else the PREGAP
        // command (inserted silence). An INDEX 00 that cannot serve the
        // arithmetic is a typed finding, never a guess.
        let idx00 = tr.indexes.iter().find(|ix| ix.number == 0).copied();
        let (pregap_ns, pregap_synthetic) = match idx00 {
            Some(ix0) if ix0.file == file && ix0.frames <= start => {
                (Some(frames_to_ns(start - ix0.frames)), false)
            }
            Some(_) => {
                late.push(CueWarning::BadPregap { track: tr.number });
                (None, false)
            }
            None => match tr.pregap_frames {
                Some(g) => (Some(frames_to_ns(g)), true),
                None => (None, false),
            },
        };

        // Zero-gap join: the immediately preceding track lives in the
        // same FILE and does not overrun this start.
        let gapless_join =
            i > 0 && sheet.tracks[i - 1].file == file && sheet.tracks[i - 1].start_frames <= start;

        // Duration: the next track's INDEX 01 when it shares the FILE;
        // the last track of a FILE stays open-ended (no invented length).
        let duration_ns = match sheet.tracks.get(i + 1) {
            Some(next) if next.file == file => {
                if next.start_frames >= start {
                    Some(frames_to_ns(next.start_frames - start))
                } else {
                    late.push(CueWarning::NonMonotonicIndex { track: next.number });
                    Some(0)
                }
            }
            _ => None,
        };

        windows.push(TrackWindow {
            track: i,
            number: tr.number,
            file,
            start_ns: frames_to_ns(start),
            duration_ns,
            pregap_ns,
            pregap_synthetic,
            postgap_ns: tr.postgap_frames.map(frames_to_ns),
            gapless_join,
        });
    }
    sheet.warnings.append(&mut late);
    sheet.windows = windows;

    Ok(sheet)
}

/// Parses raw bytes as a CUE sheet. Non-UTF-8 input is a typed error;
/// every byte input either returns or errors — never panics (fuzz-pinned).
pub fn parse_cue_bytes(input: &[u8]) -> Result<CueSheet, ParseError> {
    let text = std::str::from_utf8(input).map_err(|_| ParseError::InvalidUtf8)?;
    parse_cue(text)
}

/// In-flight track under construction between its TRACK line and the next
/// TRACK/FILE-boundary... (a FILE line does NOT close a track: per spec,
/// later INDEX lines of the same track may reference the new file).
struct TrackBuilder {
    number: u32,
    track_type: CueTrackType,
    open_line: usize,
    title: Option<String>,
    performer: Option<String>,
    songwriter: Option<String>,
    isrc: Option<String>,
    flags: Vec<CueFlag>,
    indexes: Vec<CueIndex>,
    pregap: Option<u64>,
    postgap: Option<u64>,
    rem: Vec<(String, String)>,
}

/// Closes a track builder: pushes it onto the sheet when it has an
/// `INDEX 01` (first one wins), otherwise notes the damage and skips it.
fn finalize_track(sheet: &mut CueSheet, open: Option<TrackBuilder>) {
    let Some(t) = open else { return };
    let Some(start) = t.indexes.iter().copied().find(|ix| ix.number == 1) else {
        sheet
            .warnings
            .push(CueWarning::MissingIndex01 { line: t.open_line });
        return;
    };
    sheet.tracks.push(CueTrack {
        number: t.number,
        track_type: t.track_type,
        title: t.title,
        performer: t.performer,
        songwriter: t.songwriter,
        isrc: t.isrc,
        flags: t.flags,
        indexes: t.indexes,
        pregap_frames: t.pregap,
        postgap_frames: t.postgap,
        rem: t.rem,
        file: start.file,
        start_frames: start.frames,
    });
}

/// First whitespace-delimited token and the remainder (leading whitespace
/// of the remainder trimmed). Empty input yields `("", "")`.
fn split_first_token(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(i) => (&line[..i], line[i..].trim_start()),
        None => (line, ""),
    }
}

/// Strips one outer quote pair when the whole value is quoted (tools
/// write `\"` inside quoted strings; the spec has no escapes, so the
/// interior is kept verbatim). Anything else is returned verbatim
/// (after trim).
fn unquote(value: &str) -> &str {
    let t = value.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        return &t[1..t.len() - 1];
    }
    t
}

/// `FILE` body → `(name, type token?)`. Quoted names take everything up
/// to the closing quote and the first following token as the type; a
/// missing closing quote consumes the rest of the line with no type.
/// Unquoted bodies split from the right: the last token is the type when
/// at least two tokens exist.
fn parse_file_spec(body: &str) -> Option<(String, Option<String>)> {
    if let Some(after) = body.strip_prefix('"') {
        let (name, tail) = match after.find('"') {
            Some(i) => (&after[..i], &after[i + 1..]),
            None => (after, ""),
        };
        let kind = tail.split_whitespace().next().map(str::to_string);
        Some((name.to_string(), kind))
    } else {
        let tokens: Vec<&str> = body.split_whitespace().collect();
        match tokens.len() {
            0 => None,
            1 => Some((tokens[0].to_string(), None)),
            _ => {
                let kind = tokens[tokens.len() - 1].to_string();
                let name = tokens[..tokens.len() - 1].join(" ");
                Some((name, Some(kind)))
            }
        }
    }
}

/// `MM:SS:FF` → frames (`(MM·60 + SS)·75 + FF`). Rejects non-numeric
/// parts, wrong part counts, and the out-of-range cases `MM > 99`,
/// `SS > 59`, `FF > 74`.
fn parse_time(text: &str) -> Option<u64> {
    let mut parts = text.split(':');
    let mm = parts.next()?.parse::<u64>().ok()?;
    let ss = parts.next()?.parse::<u64>().ok()?;
    let ff = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() || mm > 99 || ss > 59 || ff > 74 {
        return None;
    }
    Some((mm * 60 + ss) * FRAMES_PER_SECOND + ff)
}
