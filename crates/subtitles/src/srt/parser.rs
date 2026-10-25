//! The SRT block walker (card 5.M1): decoded text → model [`Cue`]s.
//!
//! ## Grammar (as accepted, dirty-world tolerant)
//!
//! A document is a sequence of **blocks** separated by one or more
//! blank (whitespace-only) lines. A well-formed block is
//!
//! ```text
//! <index>              optional, any unsigned integer, ignored as data
//! HH:MM:SS,mmm --> HH:MM:SS,mmm
//! <text line>…        one or more, verbatim (leading spaces kept)
//! ```
//!
//! Tolerances (each one a recorded damage event, never an abort):
//!
//! * **index line** — optional; non-numeric or `> u64` → damage +
//!   a synthetic id minted from `u64::MAX` downwards;
//! * **timing line** — the first block line containing `-->`;
//!   `.` is accepted for the `,` millisecond separator; the
//!   millisecond field may be 1–3 digits (decimal-fraction scaling:
//!   `,5` = 500 ms) or absent; whitespace around `-->` is optional;
//!   hours may exceed 23 (ffmpeg emits rolled 24 h+ hours);
//!   minutes/seconds > 59 roll up arithmetically; values beyond the
//!   i64 microsecond domain saturate with damage;
//! * **trailing junk** after the end timestamp is ignored (`X1:…`
//!   pixel-coordinate payloads get their own damage kind — R4: SRT
//!   positioning is not modeled);
//! * **garbage blocks** (no timing line), reversed intervals
//!   (`end < start`, typed-invalid in the model) and text-less blocks
//!   are skipped whole with damage — the file continues;
//! * **text lines** pass through the inline-markup mapper
//!   ([`crate::srt::tags`]).
//!
//! Everything is one pass over `str::split('\n')` slices (D-1/D-3:
//! zero intermediate line vectors beyond the current block, no
//! regex); the cue `Vec` grows only with completed cues.

use crate::model::cue::{Cue, CueId, CueText};
use crate::model::position::Positioning;
use crate::model::time::CueInterval;
use crate::srt::damage::{DamageSink, SrtDamageKind};
use crate::srt::encoding::{self, SourceEncoding};
use crate::srt::tags;

/// A parsed SRT document: the surviving cues plus the damage ledger.
///
/// Construction never fails and never panics — malformed content is
/// skipped-with-damage, not fatal (card 5.M1 AC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrtDocument {
    /// The cues that parsed, in document order.
    pub cues: Vec<Cue>,
    /// The (capped) damage ledger; see [`crate::srt::damage`].
    pub damages: Vec<crate::srt::damage::SrtDamage>,
    /// Damage events past the ledger cap (counted, not stored).
    pub dropped_damage_events: u32,
    /// The encoding the source was decoded as.
    pub encoding: SourceEncoding,
}

impl SrtDocument {
    /// True when no damage at all was recorded (clean parse).
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.damages.is_empty() && self.dropped_damage_events == 0
    }
}

/// Parse SRT bytes: BOM detection/decoding, line-ending
/// normalization, block walk. The entry point for the demux/engine
/// seam.
///
/// # Panics
/// Never — malformed input degrades into the damage ledger.
#[must_use]
pub fn parse(bytes: &[u8]) -> SrtDocument {
    let mut sink = DamageSink::new();
    let decoded = encoding::decode(bytes, &mut sink);
    let text = encoding::normalize_line_endings(&decoded.text);
    let cues = walk(&text, &mut sink);
    let (damages, dropped) = sink.into_parts();
    SrtDocument {
        cues,
        damages,
        dropped_damage_events: dropped,
        encoding: decoded.encoding,
    }
}

/// Parse an in-memory SRT string (already decoded by the caller);
/// line endings are normalized the same way.
#[must_use]
pub fn parse_str(text: &str) -> SrtDocument {
    let mut sink = DamageSink::new();
    let normalized = encoding::normalize_line_endings(text);
    let cues = walk(&normalized, &mut sink);
    let (damages, dropped) = sink.into_parts();
    SrtDocument {
        cues,
        damages,
        dropped_damage_events: dropped,
        encoding: SourceEncoding::Utf8,
    }
}

/// One block's worth of (1-based line number, raw line).
type Block<'a> = Vec<(u32, &'a str)>;

/// The single-pass block walk.
fn walk(text: &str, sink: &mut DamageSink) -> Vec<Cue> {
    let mut cues = Vec::new();
    let mut synthetic: u64 = 0;
    let mut block: Block<'_> = Vec::new();
    for (i, raw) in text.split('\n').enumerate() {
        if raw.trim().is_empty() {
            finish_block(&mut block, &mut cues, &mut synthetic, sink);
        } else {
            block.push((i as u32 + 1, raw));
        }
    }
    // A final block without a trailing blank line is still a block.
    finish_block(&mut block, &mut cues, &mut synthetic, sink);
    cues
}

/// Mint the next synthetic cue id (`u64::MAX` downwards — real SRT
/// indices are small and ascending, so collisions are pathological).
fn next_synthetic(synthetic: &mut u64) -> CueId {
    let id = CueId::new(u64::MAX - *synthetic);
    *synthetic = synthetic.saturating_add(1);
    id
}

/// Parse one accumulated block into cues (or damage), then clear it.
///
/// A block whose text region contains another *valid* timing line
/// (the classic "missing blank line" malformation) is split there
/// and the remainder re-parsed — resync, never abort. The loop is
/// explicit (not recursion): resync depth is bounded by the block's
/// line count without growing the stack (D-2).
fn finish_block<'a>(
    block: &mut Block<'a>,
    cues: &mut Vec<Cue>,
    synthetic: &mut u64,
    sink: &mut DamageSink,
) {
    let owned = std::mem::take(block);
    if owned.is_empty() {
        return;
    }
    let mut offset = 0usize;
    while offset < owned.len() {
        match parse_block(&owned[offset..], cues, synthetic, sink) {
            Resync::Done => break,
            Resync::At(at) => {
                sink.record(SrtDamageKind::StrayLinesBeforeTiming, owned[offset + at].0);
                offset += at;
            }
        }
    }
}

/// What `parse_block` wants the walker to do next.
enum Resync {
    /// The block is fully consumed.
    Done,
    /// A run-together next cue starts at this offset into the slice.
    At(usize),
}

/// Parse one block-slice: a timing line, its optional index, its
/// text — and where the next run-together cue starts, if any.
fn parse_block(
    lines: &[(u32, &str)],
    cues: &mut Vec<Cue>,
    synthetic: &mut u64,
    sink: &mut DamageSink,
) -> Resync {
    // The timing line: the first line carrying the arrow.
    let Some(tpos) = lines.iter().position(|(_, l)| l.contains("-->")) else {
        sink.record(SrtDamageKind::GarbageBlock, lines[0].0);
        return Resync::Done;
    };
    let (timing_line_no, timing_line) = lines[tpos];

    // The cue id: the block line before the timing line, if numeric.
    let id = if tpos == 0 {
        // No index line at all: optional in practice, minted silently.
        next_synthetic(synthetic)
    } else {
        if tpos > 1 {
            // Leading block lines besides the index slot.
            sink.record(SrtDamageKind::StrayLinesBeforeTiming, lines[0].0);
        }
        let (no, line) = lines[tpos - 1];
        match parse_index(line.trim()) {
            Some(n) => CueId::new(n),
            None => {
                let digits_only =
                    !line.trim().is_empty() && line.trim().bytes().all(|b| b.is_ascii_digit());
                sink.record(
                    if digits_only {
                        SrtDamageKind::IndexOverflow
                    } else {
                        SrtDamageKind::BadIndexLine
                    },
                    no,
                );
                next_synthetic(synthetic)
            }
        }
    };

    let timing = match parse_timing(timing_line.trim(), sink, timing_line_no) {
        Some(t) => t,
        None => {
            sink.record(SrtDamageKind::BadTimestamp, timing_line_no);
            return resync_after(lines, tpos).1;
        }
    };
    if timing.end_us < timing.start_us {
        sink.record(SrtDamageKind::ReversedInterval, timing_line_no);
        return resync_after(lines, tpos).1;
    }

    // Text: the lines after the timing line up to the next run-together
    // cue, verbatim, joined by LF. A numeric line directly before that
    // next timing line is its index, not text — and the remainder
    // restarts AT that index line.
    let (text_end, resync) = resync_after(lines, tpos);
    let (text_end, resync) = if let Resync::At(at) = resync {
        if at > tpos + 1 && is_index_like(lines[at - 1].1) {
            (at - 1, Resync::At(at - 1))
        } else {
            (at, Resync::At(at))
        }
    } else {
        (text_end, resync)
    };
    if text_end <= tpos + 1 {
        sink.record(SrtDamageKind::EmptyCueText, timing_line_no);
        return resync;
    }
    let joined = lines[tpos + 1..text_end]
        .iter()
        .map(|(_, l)| *l)
        .collect::<Vec<&str>>()
        .join("\n");
    let text: CueText = tags::parse_markup(&joined, sink, timing_line_no);

    // Infallible by construction: the reversal was rejected above.
    let interval = CueInterval::new(timing.start_us, timing.end_us).unwrap_or(CueInterval::EMPTY);
    cues.push(Cue::new(id, interval, text, Positioning::default()));
    resync
}

/// Where the next run-together cue starts: the next line after
/// `from` that both contains `-->` and parses as a valid timing line
/// (a literal `-->` in subtitle text that is not a timestamp stays
/// text). `text_end` is where the current cue's text ends.
fn resync_after(lines: &[(u32, &str)], from: usize) -> (usize, Resync) {
    for (q, (_, line)) in lines.iter().enumerate().skip(from + 1) {
        if line.contains("-->") && looks_like_timing(line.trim()) {
            return (q, Resync::At(q));
        }
    }
    (lines.len(), Resync::Done)
}

/// A probe that parses without recording damage (the real parse of
/// the same line does its own recording, exactly once).
fn looks_like_timing(line: &str) -> bool {
    parse_timing(line, &mut DamageSink::new(), 0).is_some()
}

/// `true` for a plain unsigned decimal (an index-line candidate).
fn is_index_like(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// `Some(n)` when the line is a plain unsigned decimal that fits u64.
fn parse_index(line: &str) -> Option<u64> {
    if line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    line.parse().ok()
}

/// A parsed timing line.
struct Timing {
    start_us: i64,
    end_us: i64,
}

/// Parse `HH:MM:SS,mmm --> HH:MM:SS,mmm` with the documented
/// tolerances. Damage for the tolerances is recorded here; `None`
/// means the line is unparseable (caller records [`SrtDamageKind::BadTimestamp`]).
fn parse_timing(line: &str, sink: &mut DamageSink, line_no: u32) -> Option<Timing> {
    let (start_side, end_side) = line.split_once("-->")?;
    // Start timestamp: the last whitespace-delimited token of the left
    // side (tolerates glued `-->` and stray prefixes).
    let s_trim = start_side.trim();
    let s_tok = s_trim.split_ascii_whitespace().next_back().unwrap_or("");
    if s_tok.len() < s_trim.len() {
        sink.record(SrtDamageKind::TrailingJunk, line_no);
    }
    let start_us = parse_time_token(s_tok, sink, line_no)?;
    // End timestamp: the first whitespace-delimited token of the right
    // side; everything after it is tolerated junk.
    let e_trim = end_side.trim_start();
    let e_tok = e_trim.split_ascii_whitespace().next().unwrap_or("");
    let end_us = parse_time_token(e_tok, sink, line_no)?;
    let trailing = e_trim.get(e_tok.len()..).unwrap_or("").trim();
    if !trailing.is_empty() {
        let positional = trailing
            .get(..2)
            .is_some_and(|p| p.eq_ignore_ascii_case("X1"))
            || trailing
                .get(..2)
                .is_some_and(|p| p.eq_ignore_ascii_case("Y1"));
        sink.record(
            if positional {
                SrtDamageKind::IgnoredPosition
            } else {
                SrtDamageKind::TrailingJunk
            },
            line_no,
        );
    }
    Some(Timing { start_us, end_us })
}

/// Parse one `H+:MM:SS` + optional `,`/`.`-separated millisecond
/// field into microseconds (saturating at the i64 domain).
fn parse_time_token(tok: &str, sink: &mut DamageSink, line_no: u32) -> Option<i64> {
    // The millisecond separator: the LAST ',' or '.' in the token
    // (a '.' inside the numeric fields makes the token unparseable
    // anyway, so the ambiguity never materializes).
    let (hms, frac) = match tok.rfind([',', '.']) {
        Some(p) => (&tok[..p], Some(&tok[p + 1..])),
        None => (tok, None),
    };
    let mut parts = hms.split(':');
    let (h, m, s) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || h.is_empty() || m.is_empty() || s.is_empty() {
        return None;
    }
    if !(h.bytes().all(|b| b.is_ascii_digit())
        && m.bytes().all(|b| b.is_ascii_digit())
        && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let h: u64 = h.parse().ok()?;
    let m: u64 = m.parse().ok()?;
    let s: u64 = s.parse().ok()?;
    // Field semantics, ffmpeg/VLC `sscanf("%3d")` style: the digits
    // ARE the milliseconds (`,5` = 5 ms), 1–3 digits accepted; more
    // than 3 truncate to the first 3 with damage.
    let ms = match frac {
        None | Some("") => 0,
        Some(d) => {
            if !d.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let eff = &d[..d.len().min(3)];
            if d.len() > 3 {
                sink.record(SrtDamageKind::LongMilliseconds, line_no);
            }
            eff.parse().ok()?
        }
    };
    let total_ms = h
        .checked_mul(3_600_000)?
        .checked_add(m.checked_mul(60_000)?)?
        .checked_add(s.checked_mul(1_000)?)?
        .checked_add(ms)?;
    match total_ms.checked_mul(1_000) {
        Some(us) if us <= i64::MAX as u64 => Some(us as i64),
        _ => {
            sink.record(SrtDamageKind::TimestampSaturated, line_no);
            Some(i64::MAX)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SrtDocument, parse, parse_str};
    use crate::model::cue::CueText;
    use crate::srt::damage::SrtDamageKind;
    use crate::srt::encoding::SourceEncoding;

    /// (kind, line) pairs for damage assertions.
    fn kinds(doc: &SrtDocument) -> Vec<(SrtDamageKind, u32)> {
        doc.damages.iter().map(|d| (d.kind, d.line_no)).collect()
    }

    #[test]
    fn canonical_two_cue_document_parses_clean() {
        let doc = parse_str(
            "1\n00:00:01,000 --> 00:00:04,000\nHello world\n\n2\n00:01:02,030 --> 00:01:02,050\nSecond\n",
        );
        assert!(doc.is_clean());
        assert_eq!(doc.cues.len(), 2);
        assert_eq!(doc.cues[0].id.get(), 1);
        assert_eq!(doc.cues[0].interval.start_us(), 1_000_000);
        assert_eq!(doc.cues[0].interval.end_us(), 4_000_000);
        assert_eq!(doc.cues[0].text, CueText::Plain("Hello world".into()));
        assert_eq!(doc.cues[1].id.get(), 2);
        assert_eq!(doc.cues[1].interval.start_us(), 62_030_000);
        assert_eq!(doc.cues[1].text, CueText::Plain("Second".into()));
    }

    #[test]
    fn crlf_and_blank_whitespace_lines_separate_blocks() {
        let doc = parse_str(
            "1\r\n00:00:00,000 --> 00:00:01,000\r\na\r\n \t \r\n2\r\n00:00:02,000 --> 00:00:03,000\r\nb\r\n",
        );
        assert!(doc.is_clean());
        assert_eq!(doc.cues.len(), 2);
        assert_eq!(doc.cues[1].text, CueText::Plain("b".into()));
        // A missing final newline still flushes the last block.
        let doc = parse_str("1\n00:00:00,000 --> 00:00:01,000\ntail");
        assert_eq!(doc.cues.len(), 1);
    }

    #[test]
    fn index_line_is_optional_and_multiline_text_is_verbatim() {
        let doc = parse_str("00:00:01,000 --> 00:00:02,000\nLine one\n  indented second\n");
        assert!(doc.is_clean());
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].id.get(), u64::MAX);
        assert_eq!(
            doc.cues[0].text,
            CueText::Plain("Line one\n  indented second".into())
        );
        // A second index-less cue mints the next synthetic id.
        let doc =
            parse_str("00:00:01,000 --> 00:00:02,000\na\n\n00:00:03,000 --> 00:00:04,000\nb\n");
        assert_eq!(doc.cues[1].id.get(), u64::MAX - 1);
    }

    #[test]
    fn timing_tolerances() {
        // '.' for ',', glued '-->', no space, 1-digit ms, missing ms.
        // Milliseconds are a FIELD (ffmpeg/VLC semantics): ',5' = 5 ms.
        let doc = parse_str("00:00:01.5 --> 00:00:02,25\nx\n");
        assert!(doc.is_clean());
        assert_eq!(doc.cues[0].interval.start_us(), 1_005_000);
        assert_eq!(doc.cues[0].interval.end_us(), 2_025_000);
        let doc = parse_str("00:00:01,000-->00:00:02,000\nx\n");
        assert!(doc.is_clean());
        assert_eq!(doc.cues[0].interval.start_us(), 1_000_000);
        let doc = parse_str("0:00:01 --> 0:00:02\nx\n");
        assert!(doc.is_clean());
        assert_eq!(doc.cues[0].interval.start_us(), 1_000_000);
        // Hours > 23 (ffmpeg's rolled 24 h+), minutes roll up.
        let doc = parse_str("24:00:00,100 --> 24:00:00,200\nx\n");
        assert!(doc.is_clean());
        assert_eq!(doc.cues[0].interval.start_us(), 86_400_100_000);
        let doc = parse_str("00:75:00,000 --> 00:76:00,000\nx\n");
        assert!(doc.is_clean());
        assert_eq!(doc.cues[0].interval.start_us(), 4_500_000_000);
    }

    #[test]
    fn trailing_junk_and_position_payloads_are_tolerated() {
        let doc = parse_str("1\n00:00:01,000 --> 00:00:02,000 X1:0 X2:100 Y1:5 Y2:50\nx\n");
        assert_eq!(kinds(&doc), vec![(SrtDamageKind::IgnoredPosition, 2)]);
        assert_eq!(doc.cues.len(), 1);
        let doc = parse_str("1\n00:00:01,000 --> 00:00:02,000 ??\nx\n");
        assert_eq!(kinds(&doc), vec![(SrtDamageKind::TrailingJunk, 2)]);
        assert_eq!(doc.cues.len(), 1);
    }

    #[test]
    fn malformed_blocks_are_skipped_without_aborting() {
        let doc = parse_str(
            "garbage block\nwith no timing\n\n1\n00:00:01,000 --> 00:00:02,000\nkeep me\n\n2\nnot a time\n3\n00:00:03,000 --> 00:00:04,000\nalso kept\n",
        );
        assert_eq!(doc.cues.len(), 2);
        assert_eq!(doc.cues[0].text, CueText::Plain("keep me".into()));
        assert_eq!(doc.cues[1].text, CueText::Plain("also kept".into()));
        assert!(
            doc.damages
                .iter()
                .any(|d| d.kind == SrtDamageKind::GarbageBlock)
        );
        // The second block's leading stray lines are recorded too.
        assert!(
            doc.damages
                .iter()
                .any(|d| d.kind == SrtDamageKind::StrayLinesBeforeTiming)
        );
    }

    #[test]
    fn reversed_and_empty_and_bad_indexes() {
        let doc = parse_str("1\n00:00:05,000 --> 00:00:02,000\nreversed\n");
        assert_eq!(doc.cues.len(), 0);
        assert_eq!(kinds(&doc), vec![(SrtDamageKind::ReversedInterval, 2)]);
        let doc = parse_str("1\n00:00:01,000 --> 00:00:02,000\n\n");
        assert_eq!(kinds(&doc), vec![(SrtDamageKind::EmptyCueText, 2)]);
        let doc = parse_str("1a\n00:00:01,000 --> 00:00:02,000\nx\n");
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].id.get(), u64::MAX);
        assert_eq!(kinds(&doc), vec![(SrtDamageKind::BadIndexLine, 1)]);
        let doc = parse_str("99999999999999999999999\n00:00:01,000 --> 00:00:02,000\nx\n");
        assert_eq!(kinds(&doc), vec![(SrtDamageKind::IndexOverflow, 1)]);
    }

    #[test]
    fn saturation_and_long_milliseconds() {
        let doc = parse_str("1\n4000000000:00:00,000 --> 4000000000:00:00,001\nbig\n");
        // 4e9 h saturates the µs domain; the cue survives clamped.
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].interval.start_us(), i64::MAX);
        assert_eq!(doc.cues[0].interval.end_us(), i64::MAX);
        assert!(
            doc.damages
                .iter()
                .any(|d| d.kind == SrtDamageKind::TimestampSaturated)
        );
        let doc = parse_str("1\n00:00:01,12345 --> 00:00:02,000\nx\n");
        assert_eq!(doc.cues[0].interval.start_us(), 1_123_000);
        assert_eq!(kinds(&doc), vec![(SrtDamageKind::LongMilliseconds, 2)]);
    }

    #[test]
    fn byte_entry_decodes_utf16_and_reports_encoding() {
        let mut le = vec![0xFF, 0xFE];
        for unit in "1\n00:00:01,000 --> 00:00:02,000\r\nhéllo\n".encode_utf16() {
            le.extend_from_slice(&unit.to_le_bytes());
        }
        let doc = parse(&le);
        assert!(doc.is_clean());
        assert_eq!(doc.encoding, SourceEncoding::Utf16Le);
        assert_eq!(doc.cues[0].text, CueText::Plain("héllo".into()));
        assert_eq!(parse(b"").cues.len(), 0);
        assert_eq!(parse(b"\n\n \n").cues.len(), 0);
    }

    #[test]
    fn stray_text_before_timing_is_kept_as_text() {
        // The classic "missing blank line" malformation: the previous
        // cue's text runs into the next block — resync at the next
        // valid timing line, its numeric index line is adopted.
        let doc = parse_str(
            "1\n00:00:01,000 --> 00:00:02,000\nfirst\n2\n00:00:03,000 --> 00:00:04,000\nsecond\n",
        );
        assert_eq!(doc.cues.len(), 2);
        assert_eq!(doc.cues[0].text, CueText::Plain("first".into()));
        assert_eq!(doc.cues[1].id.get(), 2);
        assert_eq!(doc.cues[1].text, CueText::Plain("second".into()));
        assert!(
            doc.damages
                .iter()
                .any(|d| d.kind == SrtDamageKind::StrayLinesBeforeTiming)
        );
        // A literal '-->' in text that is not a timestamp stays text.
        let doc = parse_str("1\n00:00:01,000 --> 00:00:02,000\nsee --> here\n");
        assert!(doc.is_clean());
        assert_eq!(doc.cues[0].text, CueText::Plain("see --> here".into()));
    }
}
