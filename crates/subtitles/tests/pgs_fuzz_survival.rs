//! PGS fuzz-survival (card 5.N1 `+FB`, fuzz-lite): deterministic
//! seeded mutations + systematic truncation sweeps over hand-crafted
//! display sets.
//!
//! HONEST LABEL: the seed grammars are hand-crafted for this card
//! against the ffmpeg `pgssubdec.c` semantics (fetched 2026-10-25) —
//! ffmpeg cannot author PGS in this sandbox (refuses text→bitmap
//! transcoding), so there is NO reference-encoder differential
//! oracle; the survival contract here is absence-of-panic +
//! bounded invariants, per the dvb_vobsub precedent.
//!
//! Invariants pinned per pass:
//! * the damage ledger never exceeds
//!   `pgs::MAX_DAMAGE_EVENTS + 1` (the overflow marker);
//! * every composed image's `rgba.len() == w * h * 4`;
//! * the cue render never errors on a well-formed display set
//!   (errors are typed data, swallowed here).

use subtitles::pgs::segment::PgsFraming;
use subtitles::pgs::{self, MAX_DAMAGE_EVENTS, PgsCueBuilder, PgsDecoder};

/// A tiny deterministic LCG (the srt/dvb fuzz-survival precedent).
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            self.next_u32() as usize % n
        }
    }
}

fn seg(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut v = vec![kind];
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

fn pcs(state: u8, palette_id: u8, objects: &[(u16, u16, u16)]) -> Vec<u8> {
    let mut d = vec![
        0x07,
        0x80,
        0x04,
        0x38,
        0x20,
        0x00,
        0x01,
        state,
        0x00,
        palette_id,
        objects.len() as u8,
    ];
    for &(id, x, y) in objects {
        d.extend_from_slice(&id.to_be_bytes());
        d.push(0);
        d.push(0);
        d.extend_from_slice(&x.to_be_bytes());
        d.extend_from_slice(&y.to_be_bytes());
    }
    seg(0x16, &d)
}

fn pds(id: u8, version: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
    let mut d = vec![id, version];
    for &(eid, y, cr, cb, a) in entries {
        d.extend_from_slice(&[eid, y, cr, cb, a]);
    }
    seg(0x14, &d)
}

fn ods_first(id: u16, width: u16, height: u16, rle: &[u8]) -> Vec<u8> {
    let total = (4 + rle.len()) as u32;
    let mut d = id.to_be_bytes().to_vec();
    d.push(0);
    d.push(0x80);
    d.extend_from_slice(&total.to_be_bytes()[1..4]);
    d.extend_from_slice(&width.to_be_bytes());
    d.extend_from_slice(&height.to_be_bytes());
    d.extend_from_slice(rle);
    seg(0x15, &d)
}

/// Apply `.sup` PTS-prefix framing to a stripped-framed buffer.
fn to_sup_framing(parts: &[u8]) -> Vec<u8> {
    let (segs, used) = subtitles::pgs::segment::walk_segments(parts, PgsFraming::Stripped);
    assert_eq!(used, parts.len());
    let mut sup = Vec::new();
    for s in &segs {
        sup.extend_from_slice(&90_000u32.to_be_bytes());
        sup.push(match s.kind {
            subtitles::pgs::segment::SegmentKind::Pds => 0x14,
            subtitles::pgs::segment::SegmentKind::Ods => 0x15,
            subtitles::pgs::segment::SegmentKind::Pcs => 0x16,
            subtitles::pgs::segment::SegmentKind::Wds => 0x17,
            subtitles::pgs::segment::SegmentKind::End => 0x80,
            subtitles::pgs::segment::SegmentKind::Unknown(b) => b,
        });
        sup.extend_from_slice(&(s.data.len() as u16).to_be_bytes());
        sup.extend_from_slice(s.data);
    }
    sup
}

/// One full honest display set: epoch start, WDS, palette, two 8x4
/// objects with mixed RLE codes, END. `stripped` toggles the framing.
fn seed_display_set(stripped: bool) -> Vec<u8> {
    let rle = [
        0x00, 0xC0, 0x08, 0x01, 0x00, 0x00, // 8x entry 1 (16-bit run), EOL
        0x05, 0x00, 0x87, 0x02, 0x00, 0x00, // literal 5, 7x entry 2, EOL
        0x00, 0x81, 0x01, 0x00, 0x87, 0x02, 0x00, 0x00, // escape runs
        0x00, 0x40, 0x08, 0x00, 0x00, // 8x entry 0 (color-0 run), EOL
    ];
    let wds = seg(
        0x17,
        &[
            0x01, // one window
            0x00, 0x00, 0x64, // id 0, x = 100
            0x00, 0x00, // y = 0
            0x03, 0xC0, // width = 960
            0x01, 0x2C, // height = 300
        ],
    );
    let parts = [
        pcs(0x80, 0, &[(1, 100, 900), (2, 200, 950)]),
        wds,
        pds(
            0,
            0,
            &[
                (1, 235, 128, 128, 255),
                (2, 16, 128, 128, 255),
                (5, 128, 240, 110, 255),
            ],
        ),
        ods_first(1, 8, 4, &rle),
        ods_first(2, 8, 4, &rle),
        seg(0x80, &[]),
    ]
    .concat();
    if stripped {
        parts
    } else {
        to_sup_framing(&parts)
    }
}

/// The seed plus a second, Normal display set with an empty
/// composition (the clear) — for the mutation loop, whose
/// END-splitting then exercises the state machine across sets.
fn seed_two_display_sets(stripped: bool) -> Vec<u8> {
    let mut parts = seed_display_set(true);
    parts.extend(pcs(0x00, 0, &[]));
    parts.extend(seg(0x80, &[]));
    if stripped {
        parts
    } else {
        to_sup_framing(&parts)
    }
}

fn assert_invariants(set: &pgs::PgsDisplaySet) {
    assert!(
        set.damage.len() <= MAX_DAMAGE_EVENTS + 1,
        "damage ledger over cap: {}",
        set.damage.len()
    );
    for img in &set.objects {
        assert_eq!(
            img.rgba.len(),
            img.width as usize * img.height as usize * 4,
            "rgba stride"
        );
    }
}

#[test]
fn seeded_mutation_survival_stays_typed_and_bounded() {
    let seeds = [seed_two_display_sets(true), seed_two_display_sets(false)];
    let mut lcg = Lcg(0x5eed_1234_abcd_ef01);
    for round in 0..400 {
        let seed = &seeds[round % seeds.len()];
        let mut buf = seed.clone();
        // 1-8 mutations: flip, byte-set, splice, extend.
        let mutations = 1 + lcg.below(8);
        for _ in 0..mutations {
            match lcg.below(4) {
                0 if !buf.is_empty() => {
                    let i = lcg.below(buf.len());
                    buf[i] ^= 1 << (lcg.below(8));
                }
                1 if !buf.is_empty() => {
                    let i = lcg.below(buf.len());
                    buf[i] = lcg.next_u32() as u8;
                }
                2 => {
                    // Splice in 1-16 bytes of LCG noise.
                    let at = lcg.below(buf.len() + 1);
                    let n = 1 + lcg.below(16);
                    let noise: Vec<u8> = (0..n).map(|_| lcg.next_u32() as u8).collect();
                    let mut out = buf[..at].to_vec();
                    out.extend_from_slice(&noise);
                    out.extend_from_slice(&buf[at..]);
                    buf = out;
                }
                _ => {
                    // Truncate to a random shorter length.
                    if buf.len() > 1 {
                        buf.truncate(1 + lcg.below(buf.len()));
                    }
                }
            }
        }
        // Keep the fuzz-lite budget honest (< 200 KB per input).
        if buf.len() > 200_000 {
            buf.truncate(200_000);
        }
        subtitles::pgs::fuzz::fuzz_sub_pgs(&buf);
        let mut dec = PgsDecoder::new();
        let set = dec.decode_display_set(&buf, Some(90_000));
        assert_invariants(&set);
        let builder = PgsCueBuilder::new();
        let _ = builder.cues(&set, Some(90_000 * 2), 0);
    }
}

#[test]
fn truncation_sweep_never_panics_and_stays_typed() {
    for stripped in [true, false] {
        let seed = seed_display_set(stripped);
        for cut in 0..seed.len() {
            let mut dec = PgsDecoder::new();
            dec.set_framing(if stripped {
                PgsFraming::Stripped
            } else {
                PgsFraming::Sup
            });
            let set = dec.decode_display_set(&seed[..cut], Some(0));
            assert_invariants(&set);
            subtitles::pgs::fuzz::fuzz_sub_pgs(&seed[..cut]);
        }
        // The intact seed decodes cleanly in both framings.
        let mut dec = PgsDecoder::new();
        dec.set_framing(if stripped {
            PgsFraming::Stripped
        } else {
            PgsFraming::Sup
        });
        let set = dec.decode_display_set(&seed, Some(0));
        assert!(set.is_clean(), "intact seed damaged: {:?}", set.damage);
        // The first display set composes two objects.
        assert_eq!(set.objects.len(), 2);
        assert_eq!(set.objects[0].width, 8);
        assert_eq!(set.objects[0].rgba.len(), 8 * 4 * 4);
        let cues = PgsCueBuilder::new().cues(&set, Some(90_000), 0);
        assert!(cues.is_ok());
        assert_eq!(cues.as_ref().map(Vec::len), Ok(2));
    }
}
