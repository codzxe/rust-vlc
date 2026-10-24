//! The WebVTT parser (card 5.M3, doc 04 §5 row 2): cue blocks with the
//! positioning-settings subset, NOTE blocks skipped, STYLE blocks
//! collected tolerantly (CSS is NOT applied — styling is deferred onto
//! the model's [`TextStyle`] by a later card, the same staged honesty
//! as the ASS route, doc 06 §7.8).
//!
//! # Document shape
//!
//! ```text
//! WEBVTT[ optional header text…]
//! [blank line]
//! [NOTE … skipped block]
//! [STYLE … CSS body, collected, deferred]
//! [REGION … skipped block, typed note]
//! [cue-id line — optional]
//! HH:MM:SS.mmm --> HH:MM:SS.mmm [settings…]
//! cue text lines…
//! [blank line separates blocks]
//! ```
//!
//! Both timestamp spellings (`HH:MM:SS.mmm` and `MM:SS.mmm`) parse;
//! whitespace around `-->` is optional per spec. Cue text is stripped
//! to plain ([`CueText::Plain`]): `<v Speaker>`/`<i>`-family tags
//! removed, `&amp;`-family and numeric escapes decoded — see
//! [`text`](self) module docs for the exactly-once decode order.
//!
//! # Tolerance contract (never panic)
//!
//! * A missing `WEBVTT` header is the one **fatal**, typed error
//!   ([`VttError::MissingHeader`]) — everything else is survivable.
//! * Malformed blocks (bad timestamps, reversed intervals, orphan
//!   text, `-->` beyond the second line) are skipped and reported as
//!   [`DamageEvent`]s with 1-based line numbers.
//! * `vertical:`/`size:` settings and REGION blocks are known WebVTT
//!   this parser does not apply — typed notes, never silent
//!   misrendering.
//! * BOM is skipped, CRLF is normalized, whitespace-only lines count
//!   as blank.
//!
//! # Cue identity
//!
//! Numeric id lines become [`CueId`]s verbatim; non-numeric ids
//! (legal WebVTT) and id-less cues get sequential auto ids starting
//! at 1 in document order. Ids are opaque — uniqueness beyond the
//! auto sequence is the file's responsibility.
//!
//! # Examples
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use subtitles::model::cue::CueText;
//! use subtitles::model::position::{LinePlacement, Percent};
//! use subtitles::vtt::parse;
//!
//! let doc = parse(
//!     "WEBVTT\n\nintro\n00:01.000 --> 00:04.000 line:0% align:start\n<v Fred>Hello &amp; welcome!\n",
//! )?;
//! assert_eq!(doc.cues.len(), 1);
//! let cue = &doc.cues[0];
//! assert_eq!(cue.interval.start_us(), 1_000_000);
//! assert_eq!(cue.interval.end_us(), 4_000_000);
//! assert_eq!(cue.text, CueText::Plain("Hello & welcome!".into()));
//! assert_eq!(cue.positioning.line, Some(LinePlacement::Percent(Percent::ZERO)));
//! assert!(doc.damage.is_empty());
//! # Ok(())
//! # }
//! ```

use crate::model::cue::{Cue, CueId, CueText};
use crate::model::time::CueInterval;

mod damage;
mod settings;
mod text;
mod timestamp;

pub use damage::{DamageEvent, DamageKind};

/// A STYLE block, collected but not applied: the raw CSS plus the
/// count of `::cue` rules seen in it (the "CSS subset noted" part of
/// the card — applying it is deferred, like ASS styling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleBlock {
    /// The block's CSS text (lines joined with `\n`), verbatim.
    pub css: String,
    /// How many selectors/rules in `css` mention `::cue`.
    pub cue_rule_count: usize,
}

/// A parsed WebVTT document: cues in document order plus the damage
/// ledger (every tolerated malformation, never fatal past the header).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VttDocument {
    /// Cues in document order.
    pub cues: Vec<Cue>,
    /// Tolerated malformations/degradations, in encounter order.
    pub damage: Vec<DamageEvent>,
    /// STYLE blocks (CSS collected, styling deferred).
    pub style_blocks: Vec<StyleBlock>,
}

/// The one fatal WebVTT failure: the document does not begin with the
/// `WEBVTT` header line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VttError {
    /// No `WEBVTT` magic at the start of the document (a leading U+FEFF
    /// BOM is tolerated and skipped).
    MissingHeader,
}

impl core::fmt::Display for VttError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingHeader => {
                write!(f, "document does not start with the WEBVTT header line")
            }
        }
    }
}

impl std::error::Error for VttError {}

/// Parse a WebVTT (`.vtt`) document into the shared cue model.
///
/// # Errors
/// [`VttError::MissingHeader`] — and only that — when the first line
/// does not start with `WEBVTT`. Every other malformation is a
/// [`DamageEvent`] on the returned document.
pub fn parse(source: &str) -> Result<VttDocument, VttError> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    // Line split with CRLF normalization; whitespace-only = blank.
    let lines: Vec<&str> = source
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    if lines.is_empty() || !lines[0].starts_with("WEBVTT") {
        return Err(VttError::MissingHeader);
    }
    let mut doc = VttDocument::default();
    let mut auto_id: u64 = 0;

    // Walk blocks: header first, then blank-line-separated blocks.
    let mut idx = 1usize; // past the magic line
    // Skip the rest of the header block (header text) up to a blank.
    while idx < lines.len() && !is_blank(lines[idx]) {
        idx += 1;
    }
    while idx < lines.len() {
        if is_blank(lines[idx]) {
            idx += 1;
            continue;
        }
        // Collect the block's line range [start, end).
        let start = idx;
        while idx < lines.len() && !is_blank(lines[idx]) {
            idx += 1;
        }
        let block = &lines[start..idx];
        parse_block(block, start + 1, &mut doc, &mut auto_id);
    }
    Ok(doc)
}

/// Parse one non-blank block (line numbers are absolute, 1-based:
/// `block_base` is the 1-based number of `block[0]`). `auto_id` is the
/// running auto-id counter (0 = none assigned yet).
fn parse_block(block: &[&str], block_base: usize, doc: &mut VttDocument, auto_id: &mut u64) {
    let first = block[0];
    let line_no = |i: usize| (block_base + i) as u32;
    if first.starts_with("NOTE") {
        return; // comment block: skipped whole
    }
    if first.starts_with("STYLE") {
        let css = block[1..].join("\n");
        let cue_rule_count = css.matches("::cue").count();
        doc.style_blocks.push(StyleBlock {
            css,
            cue_rule_count,
        });
        return;
    }
    if first.starts_with("REGION") {
        doc.damage
            .push(DamageEvent::unsupported_block("REGION", line_no(0)));
        return;
    }
    // Cue candidate: find the line carrying the separator.
    let timing_idx = block.iter().position(|l| l.contains("-->"));
    let Some(timing_idx) = timing_idx else {
        doc.damage.push(DamageEvent {
            line_no: line_no(0),
            kind: DamageKind::OrphanBlock,
        });
        return;
    };
    if timing_idx > 1 {
        // A cue id is at most ONE line before the timing line.
        doc.damage.push(DamageEvent {
            line_no: line_no(timing_idx),
            kind: DamageKind::TimingLineNotSecond,
        });
        return;
    }
    // Id line: present iff the separator is on the second line.
    let id = if timing_idx == 1 {
        Some(block[0])
    } else {
        None
    };
    let timing_line = block[timing_idx];
    let Some((start_us, end_us, settings_str)) = split_timing(timing_line) else {
        doc.damage.push(DamageEvent {
            line_no: line_no(timing_idx),
            kind: DamageKind::BadTimestamp,
        });
        return;
    };
    let outcome = settings::parse_settings(settings_str, line_no(timing_idx), &mut doc.damage);
    let Ok(interval) = CueInterval::new(start_us, end_us) else {
        doc.damage.push(DamageEvent {
            line_no: line_no(timing_idx),
            kind: DamageKind::ReversedInterval { start_us, end_us },
        });
        return;
    };
    let cue_id = match id {
        Some(raw) => match raw.parse::<u64>() {
            Ok(n) => CueId::new(n),
            Err(_) if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) => {
                // All digits but not a u64: overflow.
                doc.damage.push(DamageEvent {
                    line_no: line_no(0),
                    kind: DamageKind::IdOverflow,
                });
                *auto_id = auto_id.saturating_add(1);
                CueId::new(*auto_id)
            }
            Err(_) => {
                // Legal non-numeric id: auto-assigned.
                *auto_id = auto_id.saturating_add(1);
                CueId::new(*auto_id)
            }
        },
        None => {
            *auto_id = auto_id.saturating_add(1);
            CueId::new(*auto_id)
        }
    };
    let text = text::decode_cue_text(&block[timing_idx + 1..].join("\n"));
    let positioning = settings::to_positioning(&outcome);
    doc.cues.push(Cue::new(
        cue_id,
        interval,
        CueText::Plain(text),
        positioning,
    ));
}

/// Split `lhs --> rhs` into (start µs, end µs, remaining settings).
/// Whitespace around `-->` is optional per spec; the second timestamp
/// is parsed greedily so settings ride behind it on the same line.
fn split_timing(line: &str) -> Option<(i64, i64, &str)> {
    let (lhs, rhs) = line.split_once("-->")?;
    let start_us = timestamp::parse_timestamp(lhs)?;
    let rhs = rhs.trim_start();
    let (end_us, rest) = timestamp::parse_timestamp_prefix(rhs)?;
    // Per spec settings must be whitespace-separated; tolerate glued
    // junk by treating everything after the timestamp as settings —
    // unknown tokens there are typed notes, not fatal.
    Some((start_us, end_us, rest))
}

fn is_blank(line: &str) -> bool {
    line.bytes().all(|b| b == b' ' || b == b'\t')
}

#[cfg(test)]
mod tests {
    use super::{VttError, parse};
    use crate::model::cue::CueText;
    use crate::model::position::{HorizontalAlign, LinePlacement, Percent, VerticalAlign};
    use crate::vtt::damage::DamageKind;
    use testkit::Must;

    #[test]
    fn header_forms_and_empty_documents() {
        assert_eq!(parse("WEBVTT").must().cues.len(), 0);
        let doc = parse("WEBVTT\n").must();
        assert_eq!(doc.cues.len(), 0);
        let doc = parse("WEBVTT - this file has header text\n\n").must();
        assert_eq!(doc.cues.len(), 0);
        // BOM + CRLF tolerated.
        let doc = parse("\u{feff}WEBVTT\r\n\r\n00:01.000 --> 00:02.000\r\nhi\r\n").must();
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(parse(""), Err(VttError::MissingHeader));
        assert_eq!(parse("SRT-ish\n"), Err(VttError::MissingHeader));
        assert_eq!(parse("WEB VTT\n"), Err(VttError::MissingHeader));
    }

    #[test]
    fn basic_cue_with_id_and_settings() {
        let src = "WEBVTT\n\n7\n00:00:01.000 --> 00:00:04.000 line:0% position:50% align:start\n<v Fred>Hello &amp; welcome!\n";
        let doc = parse(src).must();
        assert_eq!(doc.damage.len(), 0);
        assert_eq!(doc.cues.len(), 1);
        let cue = &doc.cues[0];
        assert_eq!(cue.id.get(), 7);
        assert_eq!(cue.interval.start_us(), 1_000_000);
        assert_eq!(cue.interval.end_us(), 4_000_000);
        assert_eq!(cue.text, CueText::Plain("Hello & welcome!".into()));
        assert_eq!(
            cue.positioning.line,
            Some(LinePlacement::Percent(Percent::ZERO))
        );
        assert_eq!(cue.positioning.position, Some(Percent::HALF));
        assert_eq!(cue.positioning.alignment.horizontal, HorizontalAlign::Left);
        assert_eq!(cue.positioning.alignment.vertical, VerticalAlign::Bottom);
    }

    #[test]
    fn ids_numeric_nonnumeric_and_absent() {
        let src = "WEBVTT\n\n1\n00:01.000 --> 00:02.000\na\n\nnamed-cue\n00:03.000 --> 00:04.000\nb\n\n00:05.000 --> 00:06.000\nc\n";
        let doc = parse(src).must();
        assert_eq!(doc.cues.len(), 3);
        assert_eq!(doc.cues[0].id.get(), 1);
        // Non-numeric then absent: auto ids 1, 2 (numeric "1" kept as-is).
        assert_eq!(doc.cues[1].id.get(), 1);
        assert_eq!(doc.cues[2].id.get(), 2);
    }

    #[test]
    fn note_style_region_blocks() {
        let src = concat!(
            "WEBVTT\n\n",
            "NOTE this is a comment\nspanning lines\n\n",
            "STYLE\n::cue {\n  color: gold;\n}\n::cue(b) { font-weight: bold; }\n\n",
            "REGION\nid:fred width:40%\n\n",
            "00:01.000 --> 00:02.000\nvisible\n"
        );
        let doc = parse(src).must();
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].text, CueText::Plain("visible".into()));
        assert_eq!(doc.style_blocks.len(), 1);
        assert_eq!(doc.style_blocks[0].cue_rule_count, 2);
        assert!(
            doc.style_blocks[0]
                .css
                .contains("::cue(b) { font-weight: bold; }")
        );
        assert_eq!(doc.damage.len(), 1); // REGION
        assert!(matches!(
            doc.damage[0].kind,
            DamageKind::UnsupportedBlock { .. }
        ));
    }

    #[test]
    fn malformed_blocks_are_damages_not_panics() {
        let src = concat!(
            "WEBVTT\n\n",
            "00:xx.000 --> 00:02.000\nbad stamp\n\n", // BadTimestamp
            "00:05.000 --> 00:04.000\nreversed\n\n",  // ReversedInterval
            "junk line\nanother junk\n\n",            // OrphanBlock
            "id line\nfiller\n00:07.000 --> 00:08.000\nlate\n\n", // TimingLineNotSecond
            "00:09.000 --> 00:10.000 size:50% vertical:lr\nok\n\n", // unsupported notes
            "99999999999999999999999\n00:11.000 --> 00:12.000\nbig id\n"  // IdOverflow → auto
        );
        let doc = parse(src).must();
        assert_eq!(doc.cues.len(), 2);
        assert_eq!(doc.cues[0].text, CueText::Plain("ok".into()));
        assert_eq!(doc.cues[1].text, CueText::Plain("big id".into()));
        assert!(matches!(doc.damage[0].kind, DamageKind::BadTimestamp));
        assert!(matches!(
            doc.damage[1].kind,
            DamageKind::ReversedInterval { .. }
        ));
        assert!(matches!(doc.damage[2].kind, DamageKind::OrphanBlock));
        assert!(matches!(
            doc.damage[3].kind,
            DamageKind::TimingLineNotSecond
        ));
        assert!(matches!(
            doc.damage[4].kind,
            DamageKind::UnsupportedSetting { .. }
        ));
        assert!(matches!(
            doc.damage[5].kind,
            DamageKind::UnsupportedSetting { .. }
        ));
        assert!(matches!(doc.damage[6].kind, DamageKind::IdOverflow));
        assert_eq!(doc.damage[6].line_no, 20);
        // The overflowing-id cue survived with an auto id.
        assert_eq!(doc.cues[1].id.get(), 2);
    }

    #[test]
    fn glued_separator_and_empty_cue_text() {
        // No spaces around `-->`, and a cue whose payload is empty.
        let doc = parse("WEBVTT\n\n00:01.500-->00:02.750\n").must();
        assert_eq!(doc.damage.len(), 0);
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].interval.start_us(), 1_500_000);
        assert_eq!(doc.cues[0].interval.end_us(), 2_750_000);
        assert_eq!(doc.cues[0].text, CueText::Plain(String::new()));
    }

    #[test]
    fn multiline_text_and_line_numbers() {
        let src = "WEBVTT\n\n00:01.000 --> 00:02.000\nfirst &lt;b&gt;line\nsecond <i>line</i>\n\n99:00.000 --> 100:00.000\nlater\n";
        let doc = parse(src).must();
        assert_eq!(doc.cues.len(), 2);
        assert_eq!(
            doc.cues[0].text,
            CueText::Plain("first <b>line\nsecond line".into())
        );
        // 99 minutes and 100 minutes are legal hours-less forms.
        assert_eq!(doc.cues[1].interval.start_us(), 5_940_000_000);
        assert_eq!(doc.cues[1].interval.end_us(), 6_000_000_000);
    }
}
