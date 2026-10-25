//! Card 5.L2 acceptance: **cue → stable serialize → parse → equal**,
//! as a 176-cue × 9-alignment matrix (1584 cues), plus per-cue
//! roundtrip, serialization determinism, and an honest perf smoke
//! (printed under `--nocapture`; numbers land in the worklog).

use std::time::Instant;

use subtitles::model::{
    Alignment, Color, Cue, CueId, CueInterval, CueText, LinePlacement, Percent, Positioning,
    StyledSegment, TextStyle, parse_document, serialize_document,
};

const N_SHAPES: usize = 176;

type Shape = (
    CueId,
    CueInterval,
    CueText,
    Option<LinePlacement>,
    Option<Percent>,
);

/// 176 base shapes: every timing edge (zero-duration, negative starts,
/// the `i64` extremes), every text payload flavor (escapes, unicode,
/// empty, plain/styled), every line/position mode.
fn shapes() -> Result<Vec<Shape>, String> {
    let starts = [0i64, -1_000_000, 1, 4_500_000_000, i64::MIN, i64::MAX - 5];
    let durations = [0i64, 0, 1, 997, 40_000, 3_600_000_000, i64::MAX];
    let lines: [Option<LinePlacement>; 7] = [
        None,
        Some(LinePlacement::Percent(Percent::ZERO)),
        Some(LinePlacement::Percent(Percent::HALF)),
        Some(LinePlacement::Percent(Percent::FULL)),
        Some(LinePlacement::LineNumber(0)),
        Some(LinePlacement::LineNumber(-2)),
        Some(LinePlacement::LineNumber(i64::MAX)),
    ];
    let positions = [
        None,
        Some(Percent::ZERO),
        Some(Percent::HALF),
        Some(Percent::FULL),
        None,
    ];

    let mut out = Vec::with_capacity(N_SHAPES);
    for i in 0..N_SHAPES {
        let start = starts[i % starts.len()];
        let dur = durations[(i / starts.len()) % durations.len()];
        // saturating keeps start <= end for every (start, dur >= 0).
        let end = start.saturating_add(dur).max(start);
        let interval = CueInterval::new(start, end).map_err(|e| format!("shape {i}: {e}"))?;
        let line = lines[(i / 3) % lines.len()];
        let position = positions[(i / 2) % positions.len()];
        let text = match i % 8 {
            0 => CueText::Plain(String::new()),
            1 => CueText::Plain("plain text".into()),
            2 => CueText::Plain("two\nlines\tand\\backslash\rand\r\nmix".into()),
            3 => CueText::Plain("こんにちは émoji 🎬 ñ".into()),
            4 => CueText::Styled(Vec::new()),
            5 => CueText::Styled(vec![StyledSegment {
                text: String::new(),
                style: TextStyle::PLAIN,
            }]),
            6 => CueText::Styled(vec![
                StyledSegment {
                    text: "bold ".into(),
                    style: TextStyle::new(true, false, false, None),
                },
                StyledSegment {
                    text: "italic underline red".into(),
                    style: TextStyle::new(
                        false,
                        true,
                        true,
                        Some(Color::new(0xff, 0x00, 0x00, 0xff)),
                    ),
                },
            ]),
            _ => CueText::Styled(vec![
                StyledSegment {
                    text: "all flags".into(),
                    style: TextStyle::new(
                        true,
                        true,
                        true,
                        Some(Color::new(0x0a, 0xbc, 0xde, 0x00)),
                    ),
                },
                StyledSegment {
                    text: " tail".into(),
                    style: TextStyle::new(false, false, false, None),
                },
                StyledSegment {
                    text: String::new(),
                    style: TextStyle::new(false, false, false, None),
                },
            ]),
        };
        out.push((CueId::new(i as u64 + 1), interval, text, line, position));
    }
    Ok(out)
}

/// The 9-alignment matrix: `Alignment::ALL ×` the 176 shapes.
fn matrix() -> Result<Vec<Vec<Cue>>, String> {
    let shapes = shapes()?;
    let mut docs = Vec::with_capacity(Alignment::ALL.len());
    for alignment in Alignment::ALL {
        let cues = shapes
            .iter()
            .map(|(id, interval, text, line, position)| {
                Cue::new(
                    *id,
                    *interval,
                    text.clone(),
                    Positioning::new(alignment, *line, *position),
                )
            })
            .collect();
        docs.push(cues);
    }
    Ok(docs)
}

#[test]
fn matrix_document_roundtrip() -> Result<(), String> {
    for (ai, cues) in matrix()?.iter().enumerate() {
        let wire = serialize_document(cues);
        let parsed = parse_document(&wire).map_err(|e| format!("alignment #{ai}: {e}"))?;
        assert_eq!(parsed.len(), cues.len(), "alignment #{ai} cue count");
        assert_eq!(
            &parsed,
            cues,
            "alignment #{ai} ({}) roundtrip equality",
            Alignment::ALL[ai].code()
        );
        // Stability: re-serializing the parse reproduces the bytes.
        assert_eq!(
            serialize_document(&parsed),
            wire,
            "alignment #{ai} byte stability"
        );
    }
    Ok(())
}

#[test]
fn per_cue_roundtrip() -> Result<(), String> {
    let mut count = 0usize;
    for cues in matrix()? {
        for cue in cues {
            let wire = serialize_document(std::slice::from_ref(&cue));
            let parsed = parse_document(&wire)
                .map_err(|e| format!("per-cue parse failed for {wire}: {e}"))?;
            assert_eq!(parsed.len(), 1);
            assert_eq!(parsed[0], cue, "per-cue roundtrip: {wire}");
            count += 1;
        }
    }
    assert_eq!(count, N_SHAPES * 9, "176 × 9 matrix size");
    Ok(())
}

#[test]
fn serialization_is_deterministic() -> Result<(), String> {
    let cues = &matrix()?[0];
    let a = serialize_document(cues);
    let mut b = String::new();
    subtitles::model::wire::write_document(cues, &mut b);
    assert_eq!(
        a, b,
        "serialize_document == write_document (same buffer path)"
    );
    Ok(())
}

#[test]
fn empty_document_roundtrips() {
    let wire = serialize_document(&[]);
    assert_eq!(wire, "LUMEN-CUES 1\n");
    assert_eq!(parse_document(&wire), Ok(Vec::new()));
}

/// Honest perf smoke (not a gate): 15,840 cues (176 × 9 × 10 timing
/// offsets), 3 iterations each way, min-of-3 reported. Numbers are
/// recorded in the worklog with host + profile.
#[test]
fn perf_smoke_15k_cues() -> Result<(), String> {
    let base = matrix()?.into_iter().flatten().collect::<Vec<Cue>>();
    let mut cues = Vec::with_capacity(base.len() * 10);
    for rep in 0..10u64 {
        for cue in &base {
            cues.push(Cue::new(
                CueId::new(cue.id.get() + rep * 10_000),
                cue.interval.shift_us((rep as i64) * 61_000_000),
                cue.text.clone(),
                cue.positioning,
            ));
        }
    }
    let n = cues.len();

    let mut best_ser = u128::MAX;
    let mut best_par = u128::MAX;
    let mut bytes = 0usize;
    for _ in 0..3 {
        let t0 = Instant::now();
        let wire = serialize_document(&cues);
        let t1 = Instant::now();
        let parsed = parse_document(&wire).map_err(|e| format!("perf parse: {e}"))?;
        let t2 = Instant::now();
        assert_eq!(parsed.len(), n);
        best_ser = best_ser.min(t1.duration_since(t0).as_micros());
        best_par = best_par.min(t2.duration_since(t1).as_micros());
        bytes = wire.len();
    }
    println!(
        "perf smoke: {n} cues, {bytes} wire bytes, serialize {} µs, parse {} µs (min of 3)",
        best_ser, best_par
    );
    Ok(())
}
