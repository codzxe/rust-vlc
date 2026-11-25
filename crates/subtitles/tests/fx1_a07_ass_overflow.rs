//! FX1-A07 (FIX-PLAN F0.4i): the ASS parser must survive crafted
//! `PlayResX` + `\pos` magnitudes — W09-A01 F-001: the unchecked
//! `num + den / 2` i64 add in `to_percent` (parser.rs, the
//! `\pos`-to-percent math) panicked in debug and wrapped in release
//! on `PlayResX: 9223372036854775` + `{\pos(92233720368547,10)}` —
//! the audit's EXECUTED reproduction (probe exit 101, "attempt to add
//! with overflow").
//!
//! Post-fix contract (the crate's own policy, lib.rs "no panics on any
//! input"): every percent-math step is checked — an overflowing pair
//! counts into `AssStats::position_overrides_unmappable` (honest
//! refusal: the cue keeps its style placement, never a silent clamp),
//! while sane coordinates still map to exact percents. The crafted
//! numbers are syntactically valid ASS, so no damage is recorded.
//!
//! The scaffolding around the numbers is the in-crate suite's
//! ffmpeg-like minimal document (23-field style line, 10-field event
//! format).

use subtitles::ass;
use subtitles::model::cue::CueText;
use subtitles::model::position::{LinePlacement, Percent};

/// The overflow document. Cue 1 is the audit's exact reproduction:
/// `num + den / 2` ≈ 1.38e19 > i64::MAX (both operands individually in
/// range). Cue 2 is the sibling class — a `\pos` coordinate so large
/// the `milli * 100` numerator itself overflows.
const CRAFTED: &str = "\
[Script Info]
ScriptType: v4.00+
PlayResX: 9223372036854775
PlayResY: 288

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,16,&Hffffff,&Hffffff,&H0,&H0,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,{\\pos(92233720368547,10)}overflow x
Dialogue: 0,0:00:03.00,0:00:04.00,Default,,0,0,0,,{\\pos(9223372036854775,9223372036854775)}both huge
";

/// The control document: sane 384×288 script resolution with exact
/// half and quarter placements.
const SANE: &str = "\
[Script Info]
ScriptType: v4.00+
PlayResX: 384
PlayResY: 288

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,16,&Hffffff,&Hffffff,&H0,&H0,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,{\\pos(192,144)}center
Dialogue: 0,0:00:03.00,0:00:04.00,Default,,0,0,0,,{\\pos(96,72)}quarter
";

#[test]
fn fx1_a07_ass_pos_overflow_survives_and_counts_unmappable() {
    // The audit's panic reproduction must now parse to completion.
    let doc = ass::parse(CRAFTED);
    assert_eq!(doc.cues.len(), 2);
    assert_eq!(doc.stats.position_overrides_applied, 0);
    assert_eq!(doc.stats.position_overrides_unmappable, 2);
    // The \pos tags themselves parsed (applied to an override); the
    // percent vocabulary refused them.
    assert_eq!(doc.stats.applied_override_tags, 2);
    // Syntactically valid crafted numbers — no damage.
    assert!(doc.damage.is_empty(), "{:?}", doc.damage);
    for (i, cue) in doc.cues.iter().enumerate() {
        // Sane output: the override is not applied — the cue keeps
        // its style placement (alignment), no position, no line.
        assert!(cue.positioning.position.is_none(), "cue {i}");
        assert!(cue.positioning.line.is_none(), "cue {i}");
        assert_eq!(
            cue.positioning.alignment, doc.styles[0].alignment,
            "cue {i}"
        );
    }
    assert_eq!(doc.cues[0].text, CueText::Plain("overflow x".to_owned()));
    assert_eq!(doc.cues[1].text, CueText::Plain("both huge".to_owned()));
    assert_eq!(doc.cues[0].interval.start_us(), 1_000_000);
    assert_eq!(doc.cues[0].interval.end_us(), 2_000_000);
    assert_eq!(doc.cues[1].interval.start_us(), 3_000_000);
    assert_eq!(doc.cues[1].interval.end_us(), 4_000_000);
}

#[test]
fn fx1_a07_ass_sane_pos_still_applies_exact_percents() {
    // The tightened math must not refuse well-formed coordinates.
    let doc = ass::parse(SANE);
    assert_eq!(doc.cues.len(), 2);
    assert_eq!(doc.stats.position_overrides_applied, 2);
    assert_eq!(doc.stats.position_overrides_unmappable, 0);
    assert!(doc.damage.is_empty(), "{:?}", doc.damage);
    // 192/384 and 144/288 are exactly 50%.
    assert_eq!(doc.cues[0].positioning.position, Some(Percent::HALF));
    assert_eq!(
        doc.cues[0].positioning.line,
        Some(LinePlacement::Percent(Percent::HALF))
    );
    // 96/384 and 72/288 are exactly 25%.
    assert_eq!(doc.cues[1].positioning.position.map(Percent::get), Some(25));
    assert!(matches!(
        doc.cues[1].positioning.line,
        Some(LinePlacement::Percent(p)) if p.get() == 25
    ));
}
