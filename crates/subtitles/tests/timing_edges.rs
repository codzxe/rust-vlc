//! Card 5.L2 timing edge cases, EACH, at the wire boundary (the model
//! constructors are unit-pinned in `src/model/time.rs`; this suite
//! pins the same edges through serialize/parse and resolution):
//!
//! * zero-duration — legal, empty;
//! * `end < start` — typed invalid;
//! * overlap windows — touching ≠ overlap;
//! * out-of-range percents — typed invalid.

use std::error::Error;

use subtitles::model::{
    Alignment, Cue, CueId, CueInterval, CueText, HorizontalAlign, LinePlacement, Percent,
    Positioning, VerticalAlign, Viewport, WireError, parse_document, serialize_document,
};

#[test]
fn zero_duration_is_legal_at_the_wire() -> Result<(), Box<dyn Error>> {
    let cue = Cue::new(
        CueId::new(1),
        CueInterval::new(2_500, 2_500)?,
        CueText::Plain("flash".into()),
        Positioning::default(),
    );
    assert!(cue.interval.is_empty());
    let wire = serialize_document(std::slice::from_ref(&cue));
    let parsed = parse_document(&wire)?;
    assert_eq!(parsed, vec![cue]);
    Ok(())
}

#[test]
fn zero_duration_wire_bytes() -> Result<(), Box<dyn Error>> {
    let wire = "LUMEN-CUES 1\nC 1 2500 2500 tl - -\nP x\n";
    let parsed = parse_document(wire)?;
    assert_eq!(parsed.len(), 1);
    assert!(parsed[0].interval.is_empty());
    assert_eq!(parsed[0].interval.duration_us(), 0);
    Ok(())
}

#[test]
fn end_before_start_is_typed_invalid_at_the_wire() {
    let err = parse_document("LUMEN-CUES 1\nC 1 2000 1000 tl - -\nP x\n").err();
    assert_eq!(
        err,
        Some(WireError::EndBeforeStart {
            line_no: 2,
            start_us: 2000,
            end_us: 1000
        })
    );
}

#[test]
fn touching_is_not_overlap_at_the_wire() -> Result<(), Box<dyn Error>> {
    // Two cues that share the boundary instant 1000: both parse, and
    // the model reports touching (not overlap).
    let wire = "LUMEN-CUES 1\nC 1 0 1000 tl - -\nP a\nC 2 1000 2000 tl - -\nP b\n";
    let parsed = parse_document(wire)?;
    assert_eq!(parsed.len(), 2);
    assert!(!parsed[0].interval.overlaps(&parsed[1].interval));
    assert!(parsed[0].interval.touches(&parsed[1].interval));
    assert_eq!(parsed[0].interval.intersection(&parsed[1].interval), None);
    assert_eq!(
        parsed[0].interval.hull(&parsed[1].interval),
        CueInterval::new(0, 2000)?
    );
    Ok(())
}

#[test]
fn out_of_range_percent_is_typed_invalid_at_the_wire() {
    for (wire, want) in [
        (
            "LUMEN-CUES 1\nC 1 0 10 tl - 101\nP x\n",
            WireError::PercentOutOfRange {
                line_no: 2,
                value: 101,
            },
        ),
        (
            "LUMEN-CUES 1\nC 1 0 10 tl - -1\nP x\n",
            WireError::PercentOutOfRange {
                line_no: 2,
                value: -1,
            },
        ),
        (
            "LUMEN-CUES 1\nC 1 0 10 tl p101 -\nP x\n",
            WireError::PercentOutOfRange {
                line_no: 2,
                value: 101,
            },
        ),
        (
            "LUMEN-CUES 1\nC 1 0 10 tl p-1 -\nP x\n",
            WireError::PercentOutOfRange {
                line_no: 2,
                value: -1,
            },
        ),
    ] {
        let err = parse_document(wire).err();
        assert_eq!(err, Some(want), "wire: {wire}");
    }
}

#[test]
fn boundary_percents_are_legal_at_the_wire() -> Result<(), Box<dyn Error>> {
    let wire = "LUMEN-CUES 1\nC 1 0 10 bl p0 0\nP a\nC 2 0 10 tr p100 100\nP b\n";
    let parsed = parse_document(wire)?;
    assert_eq!(parsed.len(), 2);
    assert_eq!(
        parsed[0].positioning,
        Positioning::new(
            Alignment::new(VerticalAlign::Bottom, HorizontalAlign::Left),
            Some(LinePlacement::Percent(Percent::ZERO)),
            Some(Percent::ZERO),
        )
    );
    assert_eq!(
        parsed[1].positioning,
        Positioning::new(
            Alignment::new(VerticalAlign::Top, HorizontalAlign::Right),
            Some(LinePlacement::Percent(Percent::FULL)),
            Some(Percent::FULL),
        )
    );
    Ok(())
}

#[test]
fn crlf_documents_are_tolerated() -> Result<(), Box<dyn Error>> {
    let wire = "LUMEN-CUES 1\r\nC 1 0 1000 mc - -\r\nP crlf text\r\n";
    let parsed = parse_document(wire)?;
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].text, CueText::Plain("crlf text".into()));
    Ok(())
}

#[test]
fn every_alignment_resolves_within_the_viewport() -> Result<(), Box<dyn Error>> {
    let vp = Viewport::new(1920, 1080)?;
    for alignment in Alignment::ALL {
        for line in [
            None,
            Some(LinePlacement::Percent(Percent::ZERO)),
            Some(LinePlacement::Percent(Percent::HALF)),
            Some(LinePlacement::Percent(Percent::FULL)),
            Some(LinePlacement::LineNumber(0)),
            Some(LinePlacement::LineNumber(-3)),
            Some(LinePlacement::LineNumber(i64::MAX)),
            Some(LinePlacement::LineNumber(i64::MIN)),
        ] {
            for position in [None, Some(Percent::ZERO), Some(Percent::FULL)] {
                let p = Positioning::new(alignment, line, position);
                let r = p.resolve(vp, 40)?;
                assert!(
                    r.x_px <= vp.width_px,
                    "{} x out of viewport",
                    alignment.code()
                );
                assert!(
                    r.y_px <= vp.height_px,
                    "{} y out of viewport",
                    alignment.code()
                );
            }
        }
    }
    Ok(())
}
