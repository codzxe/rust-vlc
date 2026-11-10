//! The stable wire form (card 5.L2 acceptance): cue → serialize →
//! parse → equal.
//!
//! ## Format
//!
//! A line-oriented document, LF-terminated (CRLF tolerated on parse —
//! dirty world, D-4), versioned by a magic line:
//!
//! ```text
//! LUMEN-CUES 1
//! C <id> <start_us> <end_us> <align> <line> <pos>
//! P <escaped-text>
//! ```
//!
//! for a plain payload, or
//!
//! ```text
//! LUMEN-CUES 1
//! C 7 1000000 2000000 mc n3 50
//! S 2
//! T 100 #ff0000ff red bold text
//! T 000 - plain
//! ```
//!
//! for a styled payload. Field by field: `<align>` is one of the nine
//! 2-char codes from [`Alignment::code`]; `<line>` is `-` (unset),
//! `p<pct>` (0..=100) or `n<lines>` (WebVTT line count, may be
//! negative); `<pos>` is `-` or a 0..=100 percentage; `T` flags are
//! three `0`/`1` characters (bold, italic, underline) followed by `-`
//! or `#rrggbbaa`. Text escapes: `\\`, `\n`, `\r`, `\t` — every record
//! stays one physical line.
//!
//! ## Engineering notes (speed + memory care)
//!
//! * **Copy header types** — a cue header parses into `RawHeader`, a
//!   fixed-size `Copy` stack value with every field already validated
//!   (percent, ordering, alignment); heap allocation starts only at
//!   the text payload, and cue assembly is infallible by construction.
//! * **Single-pass parser** — [`parse_document`] walks the input once
//!   via `str::split('\n')` (zero-copy `&str` slices, no line vector,
//!   no regex, no intermediate collections); each source byte is
//!   visited exactly once.
//! * **Lazy allocation** — the output `Vec` starts empty and grows
//!   with *completed* cues only (no speculative capacity scan); each
//!   text run allocates exactly one `String`, sized to the escaped
//!   slice length (an upper bound).
//! * **Stable** — serialization emits fields in a fixed order with no
//!   map iteration or float formatting, so re-serializing a parsed
//!   document reproduces the accepted bytes (pinned by the fuzz
//!   re-serialization check).
//!
//! [PROPOSAL: this is the model's canonical interchange form for tests
//! and debug dumps; 5.L1 owns the SRT/WebVTT *source-format* parsers
//! and the Lead unifies the two contracts at wave close.]

use std::fmt::Write as _;

use crate::model::cue::{Cue, CueId, CueText, StyledSegment};
use crate::model::error::WireError;
use crate::model::position::{
    Alignment, HorizontalAlign, LinePlacement, Percent, Positioning, VerticalAlign,
};
use crate::model::style::{Color, TextStyle};
use crate::model::time::CueInterval;

/// The magic + version line every document starts with.
const MAGIC: &str = "LUMEN-CUES 1"; // lumen-allow-literal: LUMEN-CUES wire-format magic — frozen format identifier, not display identity

/// The fixed-size `Copy` header state: everything before the text
/// payload, fully validated (interval order, percent domain, alignment
/// code) before any heap allocation happens for the cue.
#[derive(Debug, Clone, Copy)]
struct RawHeader {
    id: u64,
    start_us: i64,
    end_us: i64,
    vertical: VerticalAlign,
    horizontal: HorizontalAlign,
    line: Option<LinePlacement>,
    pos: Option<Percent>,
}

/// Serialize a document of cues into a fresh `String`.
///
/// The capacity hint (48 B/cue) covers a typical header line plus a
/// short payload without re-growth; oversized payloads grow amortized.
#[must_use]
pub fn serialize_document(cues: &[Cue]) -> String {
    let mut out = String::with_capacity(MAGIC.len() + 1 + cues.len() * 48);
    write_document(cues, &mut out);
    out
}

/// Serialize into a caller-owned buffer (no intermediate `String`).
pub fn write_document(cues: &[Cue], out: &mut String) {
    out.push_str(MAGIC);
    out.push('\n');
    for cue in cues {
        write_cue(cue, out);
    }
}

fn write_cue(cue: &Cue, out: &mut String) {
    let p = &cue.positioning;
    // Header: all Copy values, one formatted write into the buffer.
    let line_tok = match p.line {
        None => "-".to_string(),
        Some(LinePlacement::Percent(v)) => format!("p{}", v.get()),
        Some(LinePlacement::LineNumber(n)) => format!("n{n}"),
    };
    let pos_tok = match p.position {
        None => "-".to_string(),
        Some(v) => format!("{}", v.get()),
    };
    let _ = writeln!(
        out,
        "C {} {} {} {} {} {}",
        cue.id.get(),
        cue.interval.start_us(),
        cue.interval.end_us(),
        p.alignment.code(),
        line_tok,
        pos_tok,
    );
    match &cue.text {
        CueText::Plain(text) => {
            out.push_str("P ");
            escape_into(text, out);
            out.push('\n');
        }
        CueText::Styled(segments) => {
            let _ = writeln!(out, "S {}", segments.len());
            for seg in segments {
                out.push_str("T ");
                out.push(if seg.style.bold { '1' } else { '0' });
                out.push(if seg.style.italic { '1' } else { '0' });
                out.push(if seg.style.underline { '1' } else { '0' });
                out.push(' ');
                match seg.style.color {
                    None => out.push('-'),
                    Some(c) => c.write_hex(out),
                }
                out.push(' ');
                escape_into(&seg.text, out);
                out.push('\n');
            }
        }
    }
}

/// Escape `\`, LF, CR, TAB so every record stays one physical line.
fn escape_into(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
}

/// Inverse of [`escape_into`]; appends to `text` (pre-sized to the
/// escaped length, the exact upper bound).
fn unescape_into(escaped: &str, text: &mut String, line_no: usize) -> Result<(), WireError> {
    let mut rest = escaped;
    while let Some(pos) = rest.find('\\') {
        text.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let decoded = match after.chars().next() {
            Some('\\') => '\\',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            _ => return Err(WireError::BadEscape { line_no }),
        };
        text.push(decoded);
        rest = &after[decoded.len_utf8()..];
    }
    text.push_str(rest);
    Ok(())
}

/// Parse a wire document back into cues (grammar in the module docs).
/// Single pass, lazy allocation, `Copy` header state.
///
/// # Errors
/// A typed [`WireError`] (with the 1-based line number) for every
/// malformed input; never panics.
pub fn parse_document(src: &str) -> Result<Vec<Cue>, WireError> {
    let mut cues = Vec::new();
    let mut lines = src.split('\n').enumerate();

    let first = lines.next().map(|(_, l)| strip_cr(l));
    if first != Some(MAGIC) {
        return Err(WireError::MissingMagic);
    }

    let mut header: Option<RawHeader> = None;
    let mut styled: Option<StyledPending> = None;

    for (idx, raw_line) in lines {
        let line_no = idx + 1;
        let line = strip_cr(raw_line);
        if line.is_empty() {
            continue; // tolerate blank lines anywhere (dirty world)
        }
        let tag = line
            .as_bytes()
            .first()
            .copied()
            .ok_or(WireError::MissingField {
                line_no,
                field: "tag",
            })?;
        match tag {
            b'C' => {
                if header.is_some() || styled.is_some() {
                    return Err(WireError::MissingPayload {
                        line_no: line_no.saturating_sub(1),
                    });
                }
                header = Some(parse_header(line, line_no)?);
            }
            b'P' | b'S' => {
                reject_unterminated(&styled)?;
                let h = header.take().ok_or(WireError::MissingField {
                    line_no,
                    field: "C header before payload",
                })?;
                let body = payload_body(line, tag, line_no)?;
                if tag == b'P' {
                    let mut text = String::with_capacity(body.len());
                    unescape_into(body, &mut text, line_no)?;
                    cues.push(build_cue(h, CueText::Plain(text)));
                } else {
                    let count: usize = body.parse().map_err(|_| WireError::BadInt {
                        line_no,
                        field: "segment count",
                    })?;
                    if count == 0 {
                        // `S 0` is a complete empty styled payload.
                        cues.push(build_cue(h, CueText::Styled(Vec::new())));
                    } else {
                        styled = Some(StyledPending {
                            header: h,
                            segments: Vec::new(), // lazy: grows with T lines
                            remaining: count,
                            s_line: line_no,
                            expected: count,
                        });
                    }
                }
            }
            b'T' => {
                let seg = parse_segment(line, line_no)?;
                let Some(st) = styled.as_mut() else {
                    return Err(WireError::MissingField {
                        line_no,
                        field: "S line before T",
                    });
                };
                if st.remaining == 0 {
                    return Err(WireError::BadValue {
                        line_no,
                        field: "segment overflow",
                    });
                }
                st.segments.push(seg);
                st.remaining -= 1;
                if st.remaining == 0 {
                    let h = st.header; // Copy — read before the take below
                    match styled.take() {
                        Some(st) => cues.push(build_cue(h, CueText::Styled(st.segments))),
                        None => {
                            return Err(WireError::MissingField {
                                line_no,
                                field: "styled payload",
                            });
                        }
                    }
                }
            }
            _ => return Err(WireError::UnknownTag { line_no, tag }),
        }
    }

    reject_unterminated(&styled)?;
    if header.is_some() {
        return Err(WireError::MissingPayload {
            line_no: line_count(src),
        });
    }
    Ok(cues)
}

/// A styled payload still being accumulated.
struct StyledPending {
    header: RawHeader,
    segments: Vec<StyledSegment>,
    remaining: usize,
    s_line: usize,
    expected: usize,
}

/// If a styled payload is still open, surface its typed error (the
/// `S` line's number, declared vs. seen segment counts).
fn reject_unterminated(styled: &Option<StyledPending>) -> Result<(), WireError> {
    if let Some(st) = styled {
        return Err(WireError::UnterminatedStyled {
            line_no: st.s_line,
            expected: st.expected,
            seen: st.expected - st.remaining,
        });
    }
    Ok(())
}

fn strip_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

fn line_count(src: &str) -> usize {
    src.split('\n').count()
}

/// The body after `"<tag> "` — `""` when the line is exactly the tag.
fn payload_body(line: &str, tag: u8, line_no: usize) -> Result<&str, WireError> {
    let tag_ch = char::from(tag);
    if let Some(rest) = line.strip_prefix(tag_ch) {
        if let Some(rest) = rest.strip_prefix(' ') {
            return Ok(rest);
        }
        if rest.is_empty() {
            return Ok("");
        }
    }
    Err(WireError::MissingField {
        line_no,
        field: "payload body",
    })
}

/// Parse `C <id> <start_us> <end_us> <align> <line> <pos>` into a
/// fully-validated `RawHeader`. Every domain check (interval order,
/// percent range, alignment code) happens here, so downstream cue
/// assembly is infallible.
fn parse_header(line: &str, line_no: usize) -> Result<RawHeader, WireError> {
    let mut fields = line.split(' ');
    let tag = fields.next().ok_or(WireError::MissingField {
        line_no,
        field: "C tag",
    })?;
    if tag != "C" {
        let first_byte = tag.as_bytes().first().copied().unwrap_or(b'?');
        return Err(WireError::UnknownTag {
            line_no,
            tag: first_byte,
        });
    }
    let id: u64 = parse_field(&mut fields, line_no, "id")?;
    let start_us: i64 = parse_field(&mut fields, line_no, "start_us")?;
    let end_us: i64 = parse_field(&mut fields, line_no, "end_us")?;
    let align_tok = field_str(&mut fields, line_no, "align")?;
    let line_tok = field_str(&mut fields, line_no, "line")?;
    let pos_tok = field_str(&mut fields, line_no, "pos")?;
    if fields.next().is_some() {
        return Err(WireError::ExtraFields { line_no });
    }

    if end_us < start_us {
        return Err(WireError::EndBeforeStart {
            line_no,
            start_us,
            end_us,
        });
    }
    let alignment = Alignment::from_code(align_tok).ok_or(WireError::BadAlignment { line_no })?;
    let line = parse_line_spec(line_tok, line_no)?;
    let pos = parse_percent_opt(pos_tok, line_no)?;
    Ok(RawHeader {
        id,
        start_us,
        end_us,
        vertical: alignment.vertical,
        horizontal: alignment.horizontal,
        line,
        pos,
    })
}

fn parse_line_spec(tok: &str, line_no: usize) -> Result<Option<LinePlacement>, WireError> {
    if tok == "-" {
        return Ok(None);
    }
    if let Some(num) = tok.strip_prefix('p') {
        return Ok(Some(LinePlacement::Percent(parse_percent(
            num,
            line_no,
            "line percent",
        )?)));
    }
    if let Some(num) = tok.strip_prefix('n') {
        let v: i64 = num.parse().map_err(|_| WireError::BadInt {
            line_no,
            field: "line number",
        })?;
        return Ok(Some(LinePlacement::LineNumber(v)));
    }
    Err(WireError::BadValue {
        line_no,
        field: "line spec",
    })
}

/// `-` or a `0..=100` percentage.
fn parse_percent_opt(tok: &str, line_no: usize) -> Result<Option<Percent>, WireError> {
    if tok == "-" {
        return Ok(None);
    }
    Some(parse_percent(tok, line_no, "pos")).transpose()
}

fn parse_percent(tok: &str, line_no: usize, field: &'static str) -> Result<Percent, WireError> {
    let v: i32 = tok
        .parse()
        .map_err(|_| WireError::BadInt { line_no, field })?;
    if !(0..=100).contains(&v) {
        return Err(WireError::PercentOutOfRange { line_no, value: v });
    }
    match Percent::new(v) {
        Ok(p) => Ok(p),
        // Unreachable: the domain check above is identical; kept
        // panic-free rather than masked with a default.
        Err(_) => Err(WireError::BadValue { line_no, field }),
    }
}

fn parse_segment(line: &str, line_no: usize) -> Result<StyledSegment, WireError> {
    // "T <b><i><u> <color> <escaped text>" — text may contain spaces.
    let rest = payload_body(line, b'T', line_no)?;
    let (flags, rest) = split_first_token(rest, line_no, "style flags")?;
    let flag_bytes = flags.as_bytes();
    if flag_bytes.len() != 3 || !flag_bytes.iter().all(|&b| b == b'0' || b == b'1') {
        return Err(WireError::BadValue {
            line_no,
            field: "style flags",
        });
    }
    let (color_tok, escaped) = match rest.find(' ') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    let color = if color_tok == "-" {
        None
    } else {
        Some(Color::from_hex(color_tok).map_err(|_| WireError::BadColor { line_no })?)
    };
    let style = TextStyle::new(
        flag_bytes[0] == b'1',
        flag_bytes[1] == b'1',
        flag_bytes[2] == b'1',
        color,
    );
    let mut text = String::with_capacity(escaped.len());
    unescape_into(escaped, &mut text, line_no)?;
    Ok(StyledSegment { text, style })
}

fn split_first_token<'a>(
    rest: &'a str,
    line_no: usize,
    field: &'static str,
) -> Result<(&'a str, &'a str), WireError> {
    match rest.find(' ') {
        Some(i) => Ok((&rest[..i], &rest[i + 1..])),
        None => Err(WireError::MissingField { line_no, field }),
    }
}

fn field_str<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    line_no: usize,
    field: &'static str,
) -> Result<&'a str, WireError> {
    fields
        .next()
        .ok_or(WireError::MissingField { line_no, field })
}

fn parse_field<'a, T: std::str::FromStr>(
    fields: &mut impl Iterator<Item = &'a str>,
    line_no: usize,
    field: &'static str,
) -> Result<T, WireError> {
    let tok = field_str(fields, line_no, field)?;
    tok.parse()
        .map_err(|_| WireError::BadInt { line_no, field })
}

/// Assemble the typed [`Cue`] from a validated raw header — infallible
/// by construction: every domain check ran in [`parse_header`].
fn build_cue(header: RawHeader, text: CueText) -> Cue {
    let positioning = Positioning::new(
        Alignment::new(header.vertical, header.horizontal),
        header.line,
        header.pos,
    );
    // Infallible: parse_header rejected end < start; the Err arm is
    // structurally dead and stays panic-free (no masking default).
    let interval = match CueInterval::new(header.start_us, header.end_us) {
        Ok(i) => i,
        Err(_) => CueInterval::EMPTY,
    };
    Cue::new(CueId::new(header.id), interval, text, positioning)
}
