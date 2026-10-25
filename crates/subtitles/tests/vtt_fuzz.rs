//! Fuzz-lite for the WebVTT parser (card 5.M3, +FB): deterministic
//! seeded mutations of a realistic document plus random buffers —
//! every input under 200 KB — must never panic, and the only typed
//! failure is the missing-header error. Anything accepted satisfies
//! the model invariants (valid intervals, in-range percentages) and
//! parsing is deterministic (same input → same document).

use subtitles::vtt::{VttError, parse};
use testkit::Must;

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

/// A realistic seed: header text, NOTE, STYLE, REGION, ids of both
/// kinds, every supported setting, tags, escapes, damage bait.
fn seed_document() -> String {
    let mut s = String::from(
        "WEBVTT - fuzz seed\n\nNOTE comment\n\nSTYLE\n::cue { color: gold; }\n\nREGION\nid:r width:50%\n\n",
    );
    for i in 0..64u32 {
        let id = match i % 3 {
            0 => format!("{i}\n"),
            1 => format!("cue-{i}\n"),
            _ => String::new(),
        };
        let settings = match i % 6 {
            0 => "line:0% position:50% align:start",
            1 => "line:-1,end",
            2 => "position:25%,left align:right",
            3 => "vertical:rl size:50%",
            4 => "line:2",
            _ => "align:middle position:200%",
        };
        let text = match i % 4 {
            0 => format!("<v Fred>hello &amp; goodbye <i>{i}</i>"),
            1 => "&#65;&#x42; &lt;tag&gt; plain".to_owned(),
            2 => format!("multi\nline\n<i>text</i> {i}"),
            _ => format!("unterminated <oops {i}"),
        };
        s.push_str(&format!(
            "{id}00:{:02}.{:03} --> 01:{:02}.{:03} {settings}\n{text}\n\n",
            i % 60,
            i * 7 % 1000,
            (i + 1) % 60,
            (i * 13) % 1000,
        ));
    }
    s
}

#[test]
fn mutated_documents_never_panic() {
    let seed = seed_document();
    assert!(seed.len() < 200_000, "seed must stay under 200 KB");
    let seed_bytes = seed.as_bytes().to_vec();
    let mut rng = Lcg(0x5eed_0c27_c0de_5eed);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for round in 0..4000u32 {
        let mut bytes = seed_bytes.clone();
        let mutations = 1 + rng.below(4);
        for _ in 0..mutations {
            if bytes.is_empty() {
                break;
            }
            match rng.below(4) {
                0 => {
                    // Flip a byte (biased toward the header region).
                    let idx = if rng.below(2) == 0 {
                        rng.below(bytes.len().min(8))
                    } else {
                        rng.below(bytes.len())
                    };
                    bytes[idx] = (rng.next_u64() & 0xff) as u8;
                }
                1 => bytes.truncate(rng.below(bytes.len() + 1)),
                2 => {
                    let idx = rng.below(bytes.len() + 1);
                    bytes.insert(idx, (rng.next_u64() & 0xff) as u8);
                }
                _ => {
                    // Splice a duplicated chunk elsewhere.
                    let len = 1 + rng.below(64.min(bytes.len()));
                    let from = rng.below(bytes.len() - len + 1);
                    let chunk = bytes[from..from + len].to_vec();
                    let at = rng.below(bytes.len() + 1);
                    let tail = bytes.split_off(at);
                    bytes.extend_from_slice(&chunk);
                    bytes.extend(tail);
                }
            }
            if bytes.len() > 200_000 {
                bytes.truncate(200_000); // keep the <200KB contract
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        match parse(&text) {
            Ok(doc) => {
                accepted += 1;
                check_invariants(&doc, round);
                // Determinism: same input parses identically.
                let again = parse(&text).must();
                assert_eq!(again, doc, "round {round}: non-deterministic parse");
            }
            Err(VttError::MissingHeader) => rejected += 1,
            Err(_) => unreachable!("VttError has no other variant today"),
        }
    }
    assert!(
        accepted > 100,
        "mutations never produced an accepted document"
    );
    assert!(rejected > 100, "mutations never produced a rejection");
}

#[test]
fn random_buffers_never_panic() {
    let mut rng = Lcg(0xbaad_f00d_dead_10cc);
    for round in 0..3000u32 {
        let len = rng.below(400);
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            // Half printable ASCII (headers/cues sometimes form), half
            // wild bytes (UTF-8 damage, NULs, high bytes).
            let b = if rng.below(2) == 0 {
                b' ' + (rng.below(95) as u8)
            } else {
                (rng.next_u64() & 0xff) as u8
            };
            bytes.push(b);
        }
        let text = String::from_utf8_lossy(&bytes);
        match parse(&text) {
            Ok(doc) => check_invariants(&doc, round),
            Err(VttError::MissingHeader) => {}
            Err(_) => unreachable!("VttError has no other variant today"),
        }
    }
}

#[test]
fn large_document_under_200kb_is_survivable() {
    // ~160 KB of cue blocks: bounded-time line walk, no blow-up.
    let mut s = String::from("WEBVTT\n\n");
    let cue = "00:01.000 --> 00:02.000 line:50%\nsome payload text here &amp; more\n\n";
    while s.len() < 160_000 {
        s.push_str(cue);
    }
    assert!(s.len() < 200_000);
    let doc = parse(&s).must();
    assert_eq!(doc.damage.len(), 0);
    assert!(!doc.cues.is_empty());
    check_invariants(&doc, u32::MAX);
    // A hostile 199 KB unterminated tag inside a cue must not hang,
    // panic, or balloon the payload.
    let hostile = format!(
        "WEBVTT\n\n00:01.000 --> 00:02.000\n{}\n",
        "<".repeat(199_000)
    );
    let doc = parse(&hostile).must();
    assert_eq!(doc.cues.len(), 1);
    assert_eq!(
        doc.cues[0].text.total_len(),
        0,
        "unterminated tag drops tail"
    );
}

/// Model invariants every accepted document must hold.
fn check_invariants(doc: &subtitles::vtt::VttDocument, round: u32) {
    for cue in &doc.cues {
        assert!(
            cue.interval.start_us() <= cue.interval.end_us(),
            "round {round}: reversed interval accepted"
        );
        if let Some(percent) = cue.positioning.position {
            assert!(percent.get() <= 100, "round {round}: percent out of range");
        }
        if let Some(subtitles::model::position::LinePlacement::Percent(p)) = cue.positioning.line {
            assert!(p.get() <= 100, "round {round}: line percent out of range");
        }
    }
    for event in &doc.damage {
        assert!(event.line_no >= 1, "round {round}: non-1-based line number");
    }
}
