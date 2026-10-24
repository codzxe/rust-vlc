//! XSPF (XML Shareable Playlist Format) parsing (card 5.O1b) — a
//! hand-rolled XML-subset pull parser, no new dependencies.
//!
//! # Format model (subset of the XSPF v1 spec)
//!
//! * Root `<playlist version="1" xmlns="http://xspf.org/ns/0/">` containing
//!   one `<trackList>` of `<track>` elements. **Namespace-tolerant**: any
//!   (or no) namespace URI is accepted, and prefixed names
//!   (`<xspf:track>`) are matched by *local name*. A root that is not
//!   `playlist` is a damage note, not an error; a `version` attribute
//!   other than `1` likewise.
//! * Each `<track>` may carry `location`, `title`, `creator`,
//!   `annotation`, `duration` child elements (character data, entity-
//!   decoded). Extra occurrences of a field are damage (`DuplicateField`,
//!   first value wins). A track with no `location` survives with an empty
//!   location (`TrackWithoutLocation`).
//! * Playlist-level elements (`title`, `creator`, …) are legal XSPF and
//!   silently ignored — this crate's vocabulary has no playlist metadata
//!   and none is invented. `location`/`duration` *outside* a track are
//!   misplaced and noted. Unknown elements anywhere (extension content,
//!   foreign namespaces) are ignored, as the XSPF spec instructs
//!   consumers to do.
//! * Attributes are parsed (both quote styles, entity-decoded values) but
//!   only the playlist `version` is looked at — the rest are a tolerated
//!   subset.
//! * XML constructs: comments `<!--…-->`, processing instructions `<?…?>`,
//!   CDATA sections, and `<!…>` declarations (DOCTYPE etc., skipped
//!   naively to the next `>`).
//! * Entities: `&amp; &lt; &gt; &quot; &apos;` plus decimal `&#38;` and
//!   hex `&#x26;` numeric references. Unknown/malformed/out-of-range
//!   entities are kept **verbatim** with a `BadEntity` note (honest
//!   degradation: no invented characters).
//!
//! # Dirty-world behavior (directives D-1/D-2)
//!
//! * Never panics on any input (fuzz-pinned). Damage degrades to typed
//!   [`XspfWarning`]s: unclosed elements are **salvaged** — a `<track>`
//!   left open by EOF or by its parent's end tag still yields its entry;
//!   mismatched/stray end tags are noted and ignored; stray character
//!   data outside any field is noted and dropped.
//! * Bounded input (D-1): nesting deeper than [`MAX_XML_DEPTH`] elements
//!   is a fatal [`XspfError::TooDeep`], and any line over the crate's
//!   64 KiB line cap (counted on raw bytes between newlines) is a fatal
//!   [`XspfError::LineTooLong`]. Non-UTF-8 input is fatal via
//!   [`parse_bytes`].
//!
//! # Mapping onto the shared vocabulary
//!
//! `location`→[`Entry::location`], `title`→[`Entry::title`],
//! `duration` (milliseconds in XSPF)→[`Entry::duration`] in seconds.
//! `creator` and `annotation` have no dedicated fields in the shared
//! vocabulary; they ride the ordered directive pairs as
//! `("creator", …)` then `("annotation", …)` — no metadata invented, none
//! silently lost. [`Playlist::to_m3u_string`] serializes those pairs as
//! `#creator:`/`#annotation:` lines (kept in the file text; the M3U
//! parser's subset only re-attaches `#EXT…` directives, so the bridge is
//! lossless for location/title/duration and text-preserving, not
//! field-preserving, for creator/annotation — pinned by tests).

use crate::m3u::MAX_LINE_BYTES;
use crate::types::{Entry, HlsSegment, Playlist};

/// Maximum accepted element nesting depth (D-1 bounded input). XSPF needs
/// four (`playlist`/`trackList`/`track`/field); 64 allows generous
/// extension content while bounding stack growth on adversarial input.
pub const MAX_XML_DEPTH: usize = 64;

/// Fatal parse failures. Structural damage is otherwise salvaged into
/// warnings; only the D-1 bounds and non-UTF-8 input are fatal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XspfError {
    /// A single line (raw bytes between newlines) exceeds the crate's
    /// 64 KiB line cap. `line` is 1-based; `length` is the exact byte
    /// length of the offending line.
    #[error(
        "playlists: line {line} is {length} bytes, over the {}-byte line cap (D-1 bounded input)",
        MAX_LINE_BYTES
    )]
    LineTooLong {
        /// 1-based line number.
        line: usize,
        /// Byte length of the offending line.
        length: usize,
    },

    /// The byte input is not valid UTF-8.
    #[error(
        "playlists: input is not valid UTF-8 (XSPF is a text format; encoding is the caller's policy)"
    )]
    InvalidUtf8,

    /// Element nesting exceeded [`MAX_XML_DEPTH`] (D-1 bounded input).
    #[error(
        "playlists: line {line}: XML nesting depth {depth} exceeds the {}-element depth guard (D-1 bounded input)",
        MAX_XML_DEPTH
    )]
    TooDeep {
        /// 1-based line number of the offending start tag.
        line: usize,
        /// The offending depth (stack size including the new element).
        depth: usize,
    },
}

/// Non-fatal dirty-world findings (D-2). Line numbers are 1-based; no
/// element text, attribute values, locations or titles are ever embedded
/// (MP §12) — element/construct *kinds* only, matching the M3U warnings'
/// discipline of naming tags but never content.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XspfWarning {
    /// The root element's local name was not `playlist`. Parsed anyway.
    #[error("playlists: line {line}: root element is not 'playlist' (parsed anyway)")]
    UnexpectedRoot {
        /// 1-based line number.
        line: usize,
    },

    /// The playlist `version` attribute was present but not `1`.
    #[error("playlists: line {line}: playlist version attribute is not '1' (tolerated)")]
    UnexpectedVersion {
        /// 1-based line number.
        line: usize,
    },

    /// A `<!--` comment ran to EOF without `-->`.
    #[error("playlists: line {line}: unterminated comment (rest of input consumed)")]
    UnterminatedComment {
        /// 1-based line number.
        line: usize,
    },

    /// A CDATA section ran to EOF without `]]>`.
    #[error("playlists: line {line}: unterminated CDATA section (rest of input consumed as text)")]
    UnterminatedCdata {
        /// 1-based line number.
        line: usize,
    },

    /// A processing instruction ran to EOF without `?>`.
    #[error("playlists: line {line}: unterminated processing instruction (rest of input consumed)")]
    UnterminatedPi {
        /// 1-based line number.
        line: usize,
    },

    /// A `<!…>` declaration (e.g. DOCTYPE) ran to EOF without `>`.
    #[error("playlists: line {line}: unterminated declaration (rest of input consumed)")]
    UnterminatedDeclaration {
        /// 1-based line number.
        line: usize,
    },

    /// A start tag ran to EOF without `>`. The element is still delivered
    /// (salvaged with the attributes parsed so far).
    #[error("playlists: line {line}: unterminated start tag (element salvaged)")]
    UnterminatedTag {
        /// 1-based line number.
        line: usize,
    },

    /// `<` was not followed by a name-start character. Kept as literal
    /// text.
    #[error("playlists: line {line}: '<' not followed by a name (kept as literal text)")]
    BadTag {
        /// 1-based line number.
        line: usize,
    },

    /// An attribute was malformed (missing `=`, unquoted value, empty
    /// name). Skipped; the element survives.
    #[error("playlists: line {line}: malformed attribute (skipped)")]
    BadAttribute {
        /// 1-based line number.
        line: usize,
    },

    /// An end tag was malformed (`</` with no name, or junk before `>`).
    /// Resynchronized to the next `>`.
    #[error("playlists: line {line}: malformed end tag (resynchronized)")]
    MalformedEndTag {
        /// 1-based line number.
        line: usize,
    },

    /// An entity was unknown, malformed, or an out-of-range numeric
    /// reference. Kept verbatim as text.
    #[error("playlists: line {line}: malformed or unknown entity (kept literal)")]
    BadEntity {
        /// 1-based line number.
        line: usize,
    },

    /// An end tag matched no open element. Ignored.
    #[error("playlists: line {line}: end tag with no matching open element (ignored)")]
    UnexpectedEndTag {
        /// 1-based line number.
        line: usize,
    },

    /// An element was still open when an ancestor's end tag (or EOF)
    /// arrived. Its content is salvaged (fields applied, tracks emitted).
    #[error("playlists: line {line}: element left open by an ancestor end tag or EOF (salvaged)")]
    UnclosedElement {
        /// 1-based line number of the element's start tag.
        line: usize,
    },

    /// Non-whitespace character data appeared where no field element was
    /// open. Ignored.
    #[error("playlists: line {line}: character data outside any field element (ignored)")]
    StrayText {
        /// 1-based line number.
        line: usize,
    },

    /// A track field element occurred twice in one track. First value
    /// wins.
    #[error("playlists: line {line}: duplicate track field element (first value wins)")]
    DuplicateField {
        /// 1-based line number.
        line: usize,
    },

    /// A `location`/`duration` element appeared outside any track.
    /// Ignored.
    #[error("playlists: line {line}: location/duration element outside any track (ignored)")]
    MisplacedElement {
        /// 1-based line number.
        line: usize,
    },

    /// `duration` text was not a finite non-negative number. The entry
    /// survives with `duration: None`.
    #[error("playlists: line {line}: malformed duration (kept entry, duration dropped)")]
    BadDuration {
        /// 1-based line number.
        line: usize,
    },

    /// A track closed with no `location` element. The entry is kept with
    /// an empty location rather than dropped.
    #[error("playlists: line {line}: track with no location element (kept, empty location)")]
    TrackWithoutLocation {
        /// 1-based line number of the track's start tag.
        line: usize,
    },

    /// A `track` element started while another track was still open. The
    /// inner one is skipped entirely.
    #[error("playlists: line {line}: track nested inside a track (inner track ignored)")]
    NestedTrack {
        /// 1-based line number.
        line: usize,
    },
}

/// A successful XSPF parse: the playlist plus every dirty-world warning
/// seen on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct XspfParsed {
    /// The parsed playlist (tracks in document order).
    pub playlist: Playlist,
    /// Non-fatal findings, in document order.
    pub warnings: Vec<XspfWarning>,
}

/// Parses an XSPF playlist, discarding warnings.
pub fn parse(input: &str) -> Result<Playlist, XspfError> {
    Ok(parse_detailed(input)?.playlist)
}

/// Parses an XSPF playlist, reporting dirty-world warnings alongside it.
pub fn parse_detailed(input: &str) -> Result<XspfParsed, XspfError> {
    // BOM tolerance: exactly one leading U+FEFF.
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);
    // D-1 preflight: the same 64 KiB per-line cap the crate enforces for
    // M3U/PLS, counted on raw bytes between newlines.
    if let Some((line, length)) = line_over_cap(input) {
        return Err(XspfError::LineTooLong { line, length });
    }

    let mut scanner = Scanner::new(input);
    let mut playlist = Playlist::new();
    let mut warnings: Vec<XspfWarning> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut current_track: Option<TrackBuilder> = None;
    let mut skip_depth: usize = 0; // >0 inside a nested (inner) track
    let mut root_seen = false;

    loop {
        let tok = scanner.next_token();
        warnings.append(&mut scanner.warnings);
        let Some(tok) = tok else { break };
        match tok {
            Token::Start {
                line,
                name,
                attrs,
                self_closing,
            } => {
                let depth = stack.len() + 1;
                if depth > MAX_XML_DEPTH {
                    return Err(XspfError::TooDeep { line, depth });
                }
                if skip_depth > 0 {
                    if !self_closing {
                        skip_depth += 1;
                    }
                    continue;
                }
                if !root_seen {
                    root_seen = true;
                    if name != "playlist" {
                        warnings.push(XspfWarning::UnexpectedRoot { line });
                    } else if let Some((_, v)) = attrs.iter().find(|(k, _)| k == "version")
                        && v != "1"
                    {
                        warnings.push(XspfWarning::UnexpectedVersion { line });
                    }
                }

                let inside_field = stack
                    .last()
                    .is_some_and(|f| matches!(f.kind, FrameKind::Field { .. }));
                if inside_field {
                    // Nested elements inside a field element: their text is
                    // not the field's direct character data — ignored.
                    if !self_closing {
                        stack.push(Frame {
                            name,
                            line,
                            kind: FrameKind::Other,
                        });
                    }
                    continue;
                }

                if current_track.is_some() && name == "track" {
                    warnings.push(XspfWarning::NestedTrack { line });
                    if !self_closing {
                        skip_depth = 1;
                    }
                    continue;
                }

                if let Some(kind) = FieldKind::from_name(&name) {
                    if current_track.is_some() {
                        let dup = current_track.as_ref().is_some_and(|tb| tb.is_set(&kind));
                        if dup {
                            warnings.push(XspfWarning::DuplicateField { line });
                        }
                        if self_closing {
                            if !dup {
                                apply_field(kind, "", &mut current_track, &mut warnings, line);
                            }
                            continue;
                        }
                        stack.push(Frame {
                            name,
                            line,
                            kind: FrameKind::Field {
                                kind,
                                text: String::new(),
                                dup,
                            },
                        });
                        continue;
                    }
                    // Outside any track: title/creator/annotation are legal
                    // playlist-level children (ignored); location/duration
                    // are misplaced.
                    if matches!(kind, FieldKind::Location | FieldKind::Duration) {
                        warnings.push(XspfWarning::MisplacedElement { line });
                    }
                    if !self_closing {
                        stack.push(Frame {
                            name,
                            line,
                            kind: FrameKind::Other,
                        });
                    }
                    continue;
                }

                if name == "track" {
                    if self_closing {
                        warnings.push(XspfWarning::TrackWithoutLocation { line });
                        playlist.push(Entry::new(String::new()));
                        continue;
                    }
                    current_track = Some(TrackBuilder {
                        line,
                        ..TrackBuilder::default()
                    });
                    stack.push(Frame {
                        name,
                        line,
                        kind: FrameKind::Track,
                    });
                    continue;
                }

                // Everything else (playlist, trackList, unknown/extension
                // elements): an ignored container.
                if !self_closing {
                    stack.push(Frame {
                        name,
                        line,
                        kind: FrameKind::Other,
                    });
                }
            }
            Token::End { line, name } => {
                if skip_depth > 0 {
                    skip_depth -= 1;
                    continue;
                }
                if name.is_empty() {
                    continue; // degenerate `</>` — already noted by the scanner
                }
                match stack.iter().rposition(|f| f.name == name) {
                    None => warnings.push(XspfWarning::UnexpectedEndTag { line }),
                    Some(i) => {
                        while stack.len() > i {
                            let forced = stack.len() > i + 1;
                            let Some(frame) = stack.pop() else { break };
                            close_frame(
                                frame,
                                forced,
                                &mut playlist,
                                &mut current_track,
                                &mut warnings,
                            );
                        }
                    }
                }
            }
            Token::Text { line, content } => {
                if skip_depth > 0 {
                    continue;
                }
                match stack.last_mut().map(|f| &mut f.kind) {
                    Some(FrameKind::Field { text, dup, .. }) => {
                        if !*dup {
                            text.push_str(&content);
                        }
                    }
                    // Text of ignored/unknown elements (incl. legal
                    // playlist-level metadata): dropped, no warning.
                    Some(FrameKind::Other) => {}
                    // Track level or document level: only formatting
                    // whitespace is expected.
                    _ => {
                        if !content.trim().is_empty() {
                            warnings.push(XspfWarning::StrayText { line });
                        }
                    }
                }
            }
        }
    }

    // EOF: salvage every still-open element (innermost first).
    while let Some(frame) = stack.pop() {
        close_frame(
            frame,
            true,
            &mut playlist,
            &mut current_track,
            &mut warnings,
        );
    }

    Ok(XspfParsed { playlist, warnings })
}

/// Parses raw bytes as an XSPF playlist. Non-UTF-8 input is a typed error;
/// every byte input either returns or errors — never panics (fuzz-pinned).
pub fn parse_bytes(input: &[u8]) -> Result<XspfParsed, XspfError> {
    let text = std::str::from_utf8(input).map_err(|_| XspfError::InvalidUtf8)?;
    parse_detailed(text)
}

// ------------------------------------------------------------- pull scanner

/// One lexical event. Names are prefix-stripped local names; text runs are
/// entity-decoded (CDATA content is passed through raw by construction).
enum Token {
    Start {
        line: usize,
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    End {
        line: usize,
        name: String,
    },
    Text {
        line: usize,
        content: String,
    },
}

/// Byte-level pull scanner over the whole input. Tracks the current line
/// (1-based) for warnings; never panics: every malformed construct either
/// resynchronizes or consumes the remainder with a warning.
struct Scanner<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    warnings: Vec<XspfWarning>,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
            warnings: Vec::new(),
        }
    }

    /// Consumes `src[pos..end]`, counting newlines for line tracking.
    /// Callers only pass char-boundary ends.
    fn take(&mut self, end: usize) -> &'a str {
        let start = self.pos;
        let slice = &self.src[start..end];
        self.line += slice.bytes().filter(|&b| b == b'\n').count();
        self.pos = end;
        slice
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn advance_char(&mut self) {
        let step = self.peek_char().map_or(0, char::len_utf8);
        self.take(self.pos + step);
    }

    fn skip_ws(&mut self) {
        while self.peek_char().is_some_and(char::is_whitespace) {
            self.advance_char();
        }
    }

    fn rest_starts_with(&self, prefix: &str) -> bool {
        self.src[self.pos..].starts_with(prefix)
    }

    /// Advances past `marker` (or, with a warning, to EOF when the marker
    /// never arrives).
    fn skip_after(&mut self, marker: &str, eof_warning: XspfWarning) {
        match self.src[self.pos..].find(marker) {
            Some(i) => {
                self.take(self.pos + i + marker.len());
            }
            None => {
                self.warnings.push(eof_warning);
                self.take(self.src.len());
            }
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        loop {
            if self.pos >= self.src.len() {
                return None;
            }
            if !self.rest_starts_with("<") {
                // Character data run up to the next '<'.
                let end = self.src[self.pos..]
                    .find('<')
                    .map_or(self.src.len(), |i| self.pos + i);
                let line = self.line;
                let raw = self.take(end);
                return Some(Token::Text {
                    line,
                    content: decode_entities(raw, &mut self.warnings, line),
                });
            }
            if self.rest_starts_with("<!--") {
                let line = self.line;
                self.skip_after("-->", XspfWarning::UnterminatedComment { line });
                continue;
            }
            if self.rest_starts_with("<![CDATA[") {
                let line = self.line;
                self.take(self.pos + "<![CDATA[".len()); // the opener
                return match self.src[self.pos..].find("]]>") {
                    Some(i) => {
                        let end = self.pos + i;
                        let raw = self.take(end);
                        self.take(end + 3); // the "]]>" terminator
                        Some(Token::Text {
                            line,
                            content: raw.to_string(),
                        })
                    }
                    None => {
                        self.warnings.push(XspfWarning::UnterminatedCdata { line });
                        let raw = self.take(self.src.len());
                        Some(Token::Text {
                            line,
                            content: raw.to_string(),
                        })
                    }
                };
            }
            if self.rest_starts_with("<?") {
                let line = self.line;
                self.skip_after("?>", XspfWarning::UnterminatedPi { line });
                continue;
            }
            if self.rest_starts_with("<!") {
                // DOCTYPE and friends: naive skip to the next '>'.
                let line = self.line;
                self.skip_after(">", XspfWarning::UnterminatedDeclaration { line });
                continue;
            }
            if self.rest_starts_with("</") {
                return Some(self.scan_end_tag());
            }
            match self.src[self.pos + 1..].chars().next() {
                Some(c) if is_name_start(c) => return Some(self.scan_start_tag()),
                _ => {
                    // '<' with no name: keep it as literal text.
                    let line = self.line;
                    self.warnings.push(XspfWarning::BadTag { line });
                    let raw = self.take(self.pos + 1);
                    return Some(Token::Text {
                        line,
                        content: raw.to_string(),
                    });
                }
            }
        }
    }

    fn scan_end_tag(&mut self) -> Token {
        let start_line = self.line;
        self.take(self.pos + 2); // "</"
        self.skip_ws();
        let name_start = self.pos;
        while self.peek_char().is_some_and(is_name_char) {
            self.advance_char();
        }
        let raw_name = self.src[name_start..self.pos].to_string();
        self.skip_ws();
        if self.rest_starts_with(">") {
            self.take(self.pos + 1);
        } else {
            self.warnings
                .push(XspfWarning::MalformedEndTag { line: start_line });
            match self.src[self.pos..].find('>') {
                Some(i) => {
                    self.take(self.pos + i + 1);
                }
                None => {
                    self.take(self.src.len());
                }
            }
        }
        if raw_name.is_empty() {
            self.warnings
                .push(XspfWarning::MalformedEndTag { line: start_line });
        }
        Token::End {
            line: start_line,
            name: local_name(&raw_name),
        }
    }

    fn scan_start_tag(&mut self) -> Token {
        let start_line = self.line;
        self.take(self.pos + 1); // "<"
        let name_start = self.pos;
        while self.peek_char().is_some_and(is_name_char) {
            self.advance_char();
        }
        let raw_name = self.src[name_start..self.pos].to_string();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        let mut terminated = false;
        loop {
            self.skip_ws();
            if self.pos >= self.src.len() {
                break;
            }
            if self.rest_starts_with("/>") {
                self.take(self.pos + 2);
                self_closing = true;
                terminated = true;
                break;
            }
            if self.rest_starts_with(">") {
                self.take(self.pos + 1);
                terminated = true;
                break;
            }
            // One attribute: name '=' quoted-value, with damage resync.
            let attr_start = self.pos;
            while self.peek_char().is_some_and(is_name_char) {
                self.advance_char();
            }
            let attr_raw = self.src[attr_start..self.pos].to_string();
            self.skip_ws();
            if !self.rest_starts_with("=") {
                self.warnings
                    .push(XspfWarning::BadAttribute { line: self.line });
                self.resync_attr();
                continue;
            }
            self.take(self.pos + 1); // "="
            self.skip_ws();
            let Some(quote) = self.peek_char().filter(|&c| c == '"' || c == '\'') else {
                self.warnings
                    .push(XspfWarning::BadAttribute { line: self.line });
                self.resync_attr();
                continue;
            };
            let q = quote.to_string();
            self.advance_char();
            let v_start = self.pos;
            let v_end = self.src[v_start..]
                .find(&q)
                .map_or(self.src.len(), |i| v_start + i);
            let raw_value = self.take(v_end);
            let decoded = decode_entities(raw_value, &mut self.warnings, self.line);
            if self.rest_starts_with(&q) {
                self.take(self.pos + q.len()); // closing quote (absent at EOF)
            }
            if attr_raw.is_empty() {
                self.warnings
                    .push(XspfWarning::BadAttribute { line: self.line });
            } else {
                attrs.push((local_name(&attr_raw), decoded));
            }
        }
        if !terminated {
            self.warnings
                .push(XspfWarning::UnterminatedTag { line: start_line });
        }
        Token::Start {
            line: start_line,
            name: local_name(&raw_name),
            attrs,
            self_closing,
        }
    }

    /// Attribute-damage resync: skip to the next whitespace or `>`. A `/`
    /// is *consumed* — only `/` immediately before `>` is a terminator,
    /// and the main loop re-checks for it, so stopping at a lone `/`
    /// would spin (fuzz-caught: `<f/creator>`).
    fn resync_attr(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '>' {
                break;
            }
            self.advance_char();
        }
    }
}

// -------------------------------------------------------- tree construction

struct Frame {
    name: String,
    line: usize,
    kind: FrameKind,
}

enum FrameKind {
    Track,
    Field {
        kind: FieldKind,
        text: String,
        dup: bool,
    },
    Other,
}

enum FieldKind {
    Location,
    Title,
    Creator,
    Annotation,
    Duration,
}

impl FieldKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "location" => Some(Self::Location),
            "title" => Some(Self::Title),
            "creator" => Some(Self::Creator),
            "annotation" => Some(Self::Annotation),
            "duration" => Some(Self::Duration),
            _ => None,
        }
    }
}

/// Collects one open `<track>`'s fields.
#[derive(Default)]
struct TrackBuilder {
    line: usize,
    location: Option<String>,
    location_set: bool,
    title: Option<String>,
    title_set: bool,
    creator: Option<String>,
    creator_set: bool,
    annotation: Option<String>,
    annotation_set: bool,
    duration: Option<f64>,
    duration_set: bool,
}

impl TrackBuilder {
    fn is_set(&self, kind: &FieldKind) -> bool {
        match kind {
            FieldKind::Location => self.location_set,
            FieldKind::Title => self.title_set,
            FieldKind::Creator => self.creator_set,
            FieldKind::Annotation => self.annotation_set,
            FieldKind::Duration => self.duration_set,
        }
    }
}

/// Closes one stack frame. `forced` marks a frame popped by an ancestor's
/// end tag or EOF (i.e. it was left open) — salvaged with a warning.
fn close_frame(
    frame: Frame,
    forced: bool,
    playlist: &mut Playlist,
    current_track: &mut Option<TrackBuilder>,
    warnings: &mut Vec<XspfWarning>,
) {
    if forced {
        warnings.push(XspfWarning::UnclosedElement { line: frame.line });
    }
    match frame.kind {
        FrameKind::Field { kind, text, dup } => {
            if !dup {
                apply_field(kind, &text, current_track, warnings, frame.line);
            }
        }
        FrameKind::Track => {
            if let Some(tb) = current_track.take() {
                playlist.push(track_entry(tb, warnings));
            }
        }
        FrameKind::Other => {}
    }
}

/// Applies a finished field's text to the open track.
fn apply_field(
    kind: FieldKind,
    text: &str,
    current_track: &mut Option<TrackBuilder>,
    warnings: &mut Vec<XspfWarning>,
    line: usize,
) {
    let Some(tb) = current_track.as_mut() else {
        return;
    };
    let trimmed = text.trim();
    match kind {
        FieldKind::Location => {
            tb.location = Some(trimmed.to_string());
            tb.location_set = true;
        }
        FieldKind::Title => {
            tb.title = non_empty(trimmed);
            tb.title_set = true;
        }
        FieldKind::Creator => {
            tb.creator = non_empty(trimmed);
            tb.creator_set = true;
        }
        FieldKind::Annotation => {
            tb.annotation = non_empty(trimmed);
            tb.annotation_set = true;
        }
        FieldKind::Duration => {
            tb.duration_set = true;
            if trimmed.is_empty() {
                return; // <duration/> — the field is simply absent
            }
            match trimmed.parse::<f64>() {
                // XSPF durations are non-negative milliseconds.
                Ok(ms) if ms.is_finite() && ms >= 0.0 => tb.duration = Some(ms / 1000.0),
                _ => warnings.push(XspfWarning::BadDuration { line }),
            }
        }
    }
}

/// Builds the shared-vocabulary entry from a finished track.
fn track_entry(tb: TrackBuilder, warnings: &mut Vec<XspfWarning>) -> Entry {
    if !tb.location_set {
        warnings.push(XspfWarning::TrackWithoutLocation { line: tb.line });
    }
    let mut directives: Vec<(String, String)> = Vec::new();
    if let Some(c) = tb.creator {
        directives.push(("creator".to_string(), c));
    }
    if let Some(a) = tb.annotation {
        directives.push(("annotation".to_string(), a));
    }
    Entry {
        location: tb.location.unwrap_or_default(),
        duration: tb.duration,
        title: tb.title,
        directives,
        hls: HlsSegment::default(),
    }
}

// ----------------------------------------------------------------- helpers

fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// Local part of a (possibly prefixed) XML name: after the last `:`.
fn local_name(raw: &str) -> String {
    match raw.rfind(':') {
        Some(i) => raw[i + 1..].to_string(),
        None => raw.to_string(),
    }
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == ':' || (c as u32) > 127
}

fn is_name_char(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-' || c == '.'
}

/// First line (raw bytes between newlines) over the crate line cap, with
/// its exact length. D-1 preflight.
fn line_over_cap(input: &str) -> Option<(usize, usize)> {
    let mut line_no = 1usize;
    let mut run = 0usize;
    let mut violation: Option<(usize, usize)> = None;
    for &b in input.as_bytes() {
        if b == b'\n' {
            if run > MAX_LINE_BYTES && violation.is_none() {
                violation = Some((line_no, run));
            }
            run = 0;
            line_no += 1;
        } else {
            run += 1;
        }
    }
    if run > MAX_LINE_BYTES && violation.is_none() {
        violation = Some((line_no, run));
    }
    violation
}

/// Entity-decodes a character-data or attribute-value run. Unknown,
/// malformed and out-of-range entities are kept **verbatim** with a
/// `BadEntity` note at `line` (the run's start line).
fn decode_entities(raw: &str, warnings: &mut Vec<XspfWarning>, line: usize) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        // Bound the entity scan: legal references are short.
        match after.find(';') {
            Some(j) if j > 0 && j <= 32 => {
                let name = &after[..j];
                match decode_one(name) {
                    Some(c) => out.push(c),
                    None => {
                        warnings.push(XspfWarning::BadEntity { line });
                        out.push('&');
                        out.push_str(name);
                        out.push(';');
                    }
                }
                rest = &after[j + 1..];
            }
            _ => {
                warnings.push(XspfWarning::BadEntity { line });
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// One entity name (between `&` and `;`), or `None` when it is unknown,
/// malformed, or a numeric reference outside XML's character range.
fn decode_one(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let num = name.strip_prefix('#')?;
            let value = if let Some(hex) = strip_hex(num) {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                num.parse::<u32>().ok()?
            };
            match char::from_u32(value) {
                // XML forbids NUL even as a numeric reference.
                Some('\0') | None => None,
                Some(c) => Some(c),
            }
        }
    }
}

fn strip_hex(num: &str) -> Option<&str> {
    num.strip_prefix('x').or_else(|| num.strip_prefix('X'))
}
