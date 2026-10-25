//! No-panic fuzz sweeps (card 5.L2, fuzz-lite pattern per 5.J2a):
//! deterministic seeded mutations of a real serialized document plus
//! random buffers, constructors and resolution — every outcome is
//! Ok-or-typed, and anything the parser accepts re-serializes
//! stably (roundtrip idempotence under mutation).

use subtitles::model::{
    Alignment, Color, Cue, CueId, CueInterval, CueText, LinePlacement, Percent, Positioning,
    StyledSegment, TextStyle, Viewport, parse_document, serialize_document,
};

/// Deterministic xorshift64* — no external RNG, reproducible runs.
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

fn seed_document() -> String {
    let mut cues = Vec::new();
    for i in 0..48u64 {
        let text = if i % 3 == 0 {
            CueText::Plain(format!("line {i}\nwith\\escapes\tand 日本語"))
        } else {
            CueText::Styled(vec![
                StyledSegment {
                    text: format!("run {i} "),
                    style: TextStyle::new(true, i % 2 == 0, false, None),
                },
                StyledSegment {
                    text: "colored".into(),
                    style: TextStyle::new(
                        false,
                        false,
                        true,
                        Some(Color::new(0x11, 0x22, 0x33, 0x44)),
                    ),
                },
            ])
        };
        let line = match i % 4 {
            0 => None,
            1 => Some(LinePlacement::Percent(Percent::HALF)),
            2 => Some(LinePlacement::LineNumber(i as i64 - 24)),
            _ => Some(LinePlacement::LineNumber(-3)),
        };
        let cue = CueInterval::new(i as i64 * 250_000, i as i64 * 250_000 + 900_000)
            .map(|interval| {
                Cue::new(
                    CueId::new(i + 1),
                    interval,
                    text,
                    Positioning::new(Alignment::ALL[(i as usize) % 9], line, Some(Percent::HALF)),
                )
            })
            .ok();
        if let Some(cue) = cue {
            cues.push(cue);
        }
    }
    serialize_document(&cues)
}

#[test]
fn mutated_documents_never_panic() -> Result<(), String> {
    let seed = seed_document();
    let seed_bytes = seed.as_bytes().to_vec();
    let mut rng = Lcg(0x5eed_c0e5_eed0_c0e5);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for round in 0..4000u32 {
        let mut bytes = seed_bytes.clone();
        let mutations = 1 + rng.below(3);
        for _ in 0..mutations {
            if bytes.is_empty() {
                break;
            }
            match rng.below(4) {
                0 => {
                    // Flip a byte to a wild value (biased to the header region).
                    let idx = if rng.below(2) == 0 {
                        rng.below(bytes.len().min(3))
                    } else {
                        rng.below(bytes.len())
                    };
                    bytes[idx] = (rng.next_u64() & 0xff) as u8;
                }
                1 => {
                    // Truncate at a random point.
                    bytes.truncate(rng.below(bytes.len() + 1));
                }
                2 => {
                    // Insert a wild byte.
                    let idx = rng.below(bytes.len() + 1);
                    bytes.insert(idx, (rng.next_u64() & 0xff) as u8);
                }
                _ => {
                    // Splice a duplicated chunk elsewhere.
                    let len = 1 + rng.below(24.min(bytes.len()));
                    let from = rng.below(bytes.len() - len + 1);
                    let chunk = bytes[from..from + len].to_vec();
                    let at = rng.below(bytes.len() + 1);
                    let tail = bytes.split_off(at);
                    bytes.extend_from_slice(&chunk);
                    bytes.extend(tail);
                }
            }
        }
        // Mutations may destroy UTF-8 validity — lossy like a real
        // dirty-world reader would see.
        let text = String::from_utf8_lossy(&bytes);
        match parse_document(&text) {
            Ok(parsed) => {
                accepted += 1;
                // Idempotence: accepted documents re-serialize stably.
                let re = serialize_document(&parsed);
                let again = parse_document(&re).map_err(|e| {
                    format!("round {round}: re-serialize produced unparseable doc: {e}")
                })?;
                assert_eq!(again, parsed, "round {round}: re-parse drift");
            }
            Err(_) => rejected += 1,
        }
    }
    // The mutations must actually bite in both directions.
    assert!(
        accepted > 100,
        "mutations never produced an accepted document"
    );
    assert!(rejected > 100, "mutations never produced a rejection");
    Ok(())
}

#[test]
fn random_buffers_never_panic() {
    let mut rng = Lcg(0xbaad_f00d_dead_10cc);
    for _ in 0..2000 {
        let len = rng.below(300);
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            // Bias half the bytes into printable ASCII so headers
            // sometimes form; the rest are wild.
            let b = if rng.below(2) == 0 {
                b' ' + (rng.below(95) as u8)
            } else {
                (rng.next_u64() & 0xff) as u8
            };
            bytes.push(b);
        }
        let text = String::from_utf8_lossy(&bytes);
        let _ = parse_document(&text); // Ok-or-typed; must not panic
    }
}

#[test]
fn constructor_sweep_is_ok_or_typed() {
    let mut rng = Lcg(0x1234_5678_9abc_def0);
    for _ in 0..20_000 {
        let a = rng.next_u64() as i64;
        let b = rng.next_u64() as i64;
        match CueInterval::new(a, b) {
            Ok(iv) => {
                assert!(iv.start_us() <= iv.end_us());
                assert!(iv.duration_us() >= 0);
                assert_eq!(iv.contains(iv.start_us()), !iv.is_empty());
            }
            Err(e) => assert!(
                matches!(e, subtitles::model::ModelError::EndBeforeStart { .. }),
                "unexpected error {e:?}"
            ),
        }
        let p = rng.next_u64() as i32;
        match Percent::new(p) {
            Ok(pc) => assert!(pc.get() <= 100),
            Err(e) => assert!(
                matches!(e, subtitles::model::ModelError::PercentOutOfRange { .. }),
                "unexpected error {e:?}"
            ),
        }
        let w = rng.below(4096) as u32;
        let h = rng.below(4096) as u32;
        match Viewport::new(w, h) {
            Ok(vp) => assert!(vp.width_px > 0 && vp.height_px > 0),
            Err(e) => assert!(
                matches!(e, subtitles::model::ModelError::EmptyViewport),
                "unexpected error {e:?}"
            ),
        }
    }
}

#[test]
fn resolve_sweep_never_panics() {
    let mut rng = Lcg(0xfeed_face_cafe_babe);
    for _ in 0..20_000 {
        let vp = match Viewport::new(rng.below(4096) as u32, rng.below(4096) as u32) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let percent = Percent::new(rng.next_u64() as i32 % 101).ok();
        let line = match rng.below(3) {
            0 => None,
            1 => Percent::new(rng.next_u64() as i32 % 101)
                .ok()
                .map(LinePlacement::Percent),
            _ => Some(LinePlacement::LineNumber(rng.next_u64() as i64)),
        };
        let p = Positioning::new(Alignment::ALL[rng.below(9)], line, percent);
        if let Ok(r) = p.resolve(vp, rng.below(200) as u32) {
            assert!(r.x_px <= vp.width_px);
            assert!(r.y_px <= vp.height_px);
        }
    }
}
