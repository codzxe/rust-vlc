//! Card 5.M3 acceptance: three ffmpeg-generated WebVTT documents
//! (`ffmpeg -c:s webvtt`, 7.1.5) plus one handcrafted settings-heavy
//! document — cue counts, times, text and settings must equal the
//! hand-derived truth exactly.
//!
//! Fixture provenance (all committed, no test-time ffmpeg needed):
//!
//! * `basic.vtt` — from an SRT with entities, inline tags, a `<v>`
//!   speaker tag and multiline text; ffmpeg emits the short
//!   `MM:SS.mmm` form, keeps `&amp;`-family escapes and `<i>`/`<b>`
//!   tags for OUR stripper to handle, and drops `<v>` names itself.
//! * `long.vtt` — over-an-hour cues: the long `HH:MM:SS.mmm` form,
//!   UTF-8 payload, and ffmpeg's chronological re-order (lead-in cue
//!   first). `&quot;` stays literal — WebVTT defines exactly six
//!   named escapes and `quot` is not among them.
//! * `styled.vtt` — from an ASS with top/middle/bottom-left alignments
//!   and an `{\an4}` override: ffmpeg's webvtt encoder drops ALL
//!   positioning — plain cues, settings-free (recorded honesty gap of
//!   the ffmpeg route, not of this parser).
//! * `settings_heavy.vtt` — handcrafted: BOM + CRLF, NOTE/STYLE/REGION
//!   blocks, numeric + non-numeric cue ids, every supported setting
//!   form, `vertical`/`size` unsupported notes, a reversed interval
//!   and an out-of-range percentage.

use subtitles::model::cue::{Cue, CueText};
use subtitles::model::position::{HorizontalAlign, LinePlacement, Percent, VerticalAlign};
use subtitles::vtt::{DamageEvent, DamageKind, parse};
use testkit::Must;

const BASIC: &str = include_str!("fixtures/vtt/basic.vtt");
const LONG: &str = include_str!("fixtures/vtt/long.vtt");
const STYLED: &str = include_str!("fixtures/vtt/styled.vtt");
const SETTINGS_HEAVY: &str = include_str!("fixtures/vtt/settings_heavy.vtt");

/// (start µs, end µs, expected plain text) — everything a plain cue
/// carries; ids are checked separately where they matter. A styled
/// payload (never produced by this parser) fails every equality below.
fn plain(cue: &Cue) -> (i64, i64, String) {
    match &cue.text {
        CueText::Plain(text) => (cue.interval.start_us(), cue.interval.end_us(), text.clone()),
        CueText::Styled(_) => (i64::MIN, i64::MIN, "STYLED?".to_owned()),
    }
}

#[test]
fn ffmpeg_basic_cues_times_text_equal_truth() {
    let doc = parse(BASIC).must();
    assert_eq!(doc.damage, Vec::new(), "no damage expected");
    assert_eq!(doc.cues.len(), 3);
    assert_eq!(
        plain(&doc.cues[0]),
        (
            1_000_000,
            4_000_000,
            "Hello & world\nsecond line".to_owned()
        )
    );
    assert_eq!(
        plain(&doc.cues[1]),
        (5_500_000, 8_250_000, "Italic and bold text".to_owned())
    );
    assert_eq!(
        plain(&doc.cues[2]),
        (62_004_000, 63_000_000, "Speaker line".to_owned())
    );
    // ffmpeg wrote no id lines: sequential auto ids.
    assert_eq!(
        doc.cues.iter().map(|c| c.id.get()).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    // No settings emitted: classic bottom-center defaults.
    assert_eq!(doc.cues[0].positioning, Default::default());
}

#[test]
fn ffmpeg_long_form_hours_and_unicode_equal_truth() {
    let doc = parse(LONG).must();
    assert_eq!(doc.damage, Vec::new());
    assert_eq!(doc.cues.len(), 3);
    assert_eq!(
        plain(&doc.cues[0]),
        (500_000, 2_000_000, "Lead-in cue".to_owned())
    );
    assert_eq!(
        plain(&doc.cues[1]),
        (
            5_025_678_000,
            5_030_000_000,
            "Over an hour <tag> & &quot;quotes&quot;".to_owned()
        )
    );
    assert_eq!(
        plain(&doc.cues[2]),
        (
            5_031_000_000,
            5_042_500_000,
            "日本語 subtitle line".to_owned()
        )
    );
}

#[test]
fn ffmpeg_from_ass_drops_positioning_plain_cues() {
    let doc = parse(STYLED).must();
    assert_eq!(doc.damage, Vec::new());
    assert_eq!(doc.cues.len(), 4);
    let expect = [
        (1_000_000, 3_000_000, "Top anchored cue".to_owned()),
        (4_000_000, 6_500_000, "Middle cue".to_owned()),
        (7_000_000, 9_000_000, "Bottom-left cue".to_owned()),
        (10_000_000, 12_000_000, "Forced left-mid".to_owned()),
    ];
    for (cue, want) in doc.cues.iter().zip(expect) {
        assert_eq!(plain(cue), want);
        assert_eq!(cue.positioning, Default::default());
    }
}

#[test]
fn handcrafted_settings_heavy_equals_truth() {
    let doc = parse(SETTINGS_HEAVY).must();
    assert_eq!(doc.cues.len(), 6, "reversed cue dropped, rest kept");

    // Cue 1: numeric id, full settings subset.
    let cue = &doc.cues[0];
    assert_eq!(cue.id.get(), 1);
    assert_eq!(
        plain(cue),
        (1_000_000, 3_000_000, "Top-left anchored".to_owned())
    );
    assert_eq!(
        cue.positioning.line,
        Some(LinePlacement::Percent(Percent::ZERO))
    );
    assert_eq!(cue.positioning.position, Some(Percent::HALF));
    assert_eq!(cue.positioning.alignment.horizontal, HorizontalAlign::Left);
    assert_eq!(cue.positioning.alignment.vertical, VerticalAlign::Bottom);

    // Cue 2: line at 100%, center align, <i> stripped.
    let cue = &doc.cues[1];
    assert_eq!(cue.id.get(), 2);
    assert_eq!(
        plain(cue),
        (4_000_000, 6_000_000, "Bottom via line:100%".to_owned())
    );
    assert_eq!(
        cue.positioning.line,
        Some(LinePlacement::Percent(Percent::FULL))
    );
    assert_eq!(
        cue.positioning.alignment.horizontal,
        HorizontalAlign::Center
    );

    // Cue "three": non-numeric id → auto id 1; position suffix sets
    // horizontal; line-number + start suffix set vertical; escaped
    // tags survive as text.
    let cue = &doc.cues[2];
    assert_eq!(cue.id.get(), 1);
    assert_eq!(
        plain(cue),
        (
            7_000_000,
            9_000_000,
            "<v Kim>Left quarter & entity".to_owned()
        )
    );
    assert_eq!(cue.positioning.position, Some(Percent::new(25).must()));
    assert_eq!(cue.positioning.alignment.horizontal, HorizontalAlign::Left);
    assert_eq!(cue.positioning.line, Some(LinePlacement::LineNumber(2)));
    assert_eq!(cue.positioning.alignment.vertical, VerticalAlign::Top);

    // Cue 4: end-align; vertical/size are typed-unsupported notes.
    let cue = &doc.cues[3];
    assert_eq!(cue.id.get(), 4);
    assert_eq!(
        plain(cue),
        (
            10_000_000,
            12_500_000,
            "Right-aligned & unsupported".to_owned()
        )
    );
    assert_eq!(cue.positioning.alignment.horizontal, HorizontalAlign::Right);
    assert_eq!(cue.positioning.line, None);
    assert_eq!(cue.positioning.position, None);

    // Cue 5 (auto id 2): negative line number, from the bottom.
    let cue = &doc.cues[4];
    assert_eq!(cue.id.get(), 2);
    assert_eq!(
        plain(cue),
        (13_000_000, 15_000_000, "From the bottom".to_owned())
    );
    assert_eq!(cue.positioning.line, Some(LinePlacement::LineNumber(-1)));

    // Cue 6 (auto id 3): out-of-range position → cue kept, setting
    // dropped with a typed note.
    let cue = &doc.cues[5];
    assert_eq!(cue.id.get(), 3);
    assert_eq!(
        plain(cue),
        (17_500_000, 18_500_000, "Kept, position dropped".to_owned())
    );
    assert_eq!(cue.positioning.position, None);

    // The damage ledger: REGION block, vertical, size, reversed
    // interval, out-of-range percent — in encounter order, with the
    // 1-based line numbers of the handcrafted file.
    assert_eq!(
        doc.damage,
        vec![
            DamageEvent {
                line_no: 12,
                kind: DamageKind::UnsupportedBlock {
                    name: "REGION".to_owned()
                }
            },
            DamageEvent {
                line_no: 28,
                kind: DamageKind::UnsupportedSetting {
                    name: "vertical".to_owned()
                }
            },
            DamageEvent {
                line_no: 28,
                kind: DamageKind::UnsupportedSetting {
                    name: "size".to_owned()
                }
            },
            DamageEvent {
                line_no: 34,
                kind: DamageKind::ReversedInterval {
                    start_us: 16_000_000,
                    end_us: 15_000_000
                }
            },
            DamageEvent {
                line_no: 37,
                kind: DamageKind::BadSetting {
                    name: "position".to_owned()
                }
            },
        ]
    );

    // STYLE block collected, CSS deferred, ::cue rules counted.
    assert_eq!(doc.style_blocks.len(), 1);
    assert_eq!(doc.style_blocks[0].cue_rule_count, 2);
    assert!(doc.style_blocks[0].css.contains("background-color"));
}
