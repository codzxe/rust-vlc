//! Card 5.N3 acceptance — CC 608/708 end-to-end over hand-crafted
//! `cc_data()` vectors.
//!
//! The vectors are hand-assembled per the ATSC A/53 Part 4 / CTA-708
//! grammars (the parity bits are computed by the crate's own
//! [`subtitles::cc::eia608::with_odd_parity`], which is itself
//! pinned by the unit suite). No reference corpus exists in the
//! restored sandbox (the N2 honesty-gap precedent: no ffmpeg
//! text→CC transcode path), so the pinning is: grammar round-trips,
//! the classic `"1\r2\r3"` roll-up shape, grid transitions, PAC
//! colors, pop-on double-buffering, typed error paths, and a
//! seeded-LCG fuzz-lite survival sweep (< 200 KB total input).

use subtitles::cc::cc_data::{
    ATSC_COUNTRY_CODE, ATSC_PROVIDER_CODE, CC_TYPE_CODE, CcDataError, CcDataTriplet, GA94,
    parse_ga94_sei, parse_scte20,
};
use subtitles::cc::eia608::{CcChannel, CcColor, with_odd_parity};
use subtitles::cc::{CcCueBuilder, CcDamage, CcDecoder, MAX_DAMAGE_EVENTS};
use testkit::Must;

/// Parity-complete a 608 byte.
fn p(b: u8) -> u8 {
    with_odd_parity(b)
}

/// A valid 608 field-1 triplet carrying the parity-complete pair.
fn t608(d1: u8, d2: u8) -> CcDataTriplet {
    CcDataTriplet::cc(0, [p(d1), p(d2)])
}

/// A GA94 SEI type-4 payload wrapping the given pairs (all field 1).
fn ga94(pairs: &[(u8, u8)]) -> Vec<u8> {
    let mut v = vec![ATSC_COUNTRY_CODE];
    v.extend_from_slice(&ATSC_PROVIDER_CODE.to_be_bytes());
    v.extend_from_slice(&GA94);
    v.push(CC_TYPE_CODE);
    v.push(0x40 | pairs.len() as u8); // process=1, zero=0, count
    for &(d1, d2) in pairs {
        v.push(0x04); // cc_valid=1, cc_type=0
        v.push(p(d1));
        v.push(p(d2));
    }
    v.push(0xFF);
    v
}

/// The classic roll-up caption sequence: RU2, PAC row 15, then
/// "1", CR, "2", CR, "3".
fn rollup_frames() -> Vec<Vec<CcDataTriplet>> {
    vec![
        vec![t608(0x14, 0x25), t608(0x44, 0x60), t608(0x31, 0x20)],
        vec![t608(0x14, 0x2D), t608(0x32, 0x20)],
        vec![t608(0x14, 0x2D), t608(0x33, 0x20)],
    ]
}

#[test]
fn classic_rollup_scrolls_through_the_full_stack() {
    let mut dec = CcDecoder::new();
    let frames = rollup_frames();
    let pts = [1_000_000_000i64, 2_000_000_000, 3_000_000_000];

    // Frame 1: RU2 entered (clear), PAC row 15, "1" written.
    let u1 = dec.decode(&frames[0], Some(pts[0]));
    assert_eq!(u1.events.len(), 1);
    assert_eq!(u1.events[0].channel, CcChannel::CC1);
    assert_eq!(u1.events[0].grid.row_text(14), "1");
    assert!(u1.damage.is_empty(), "damage: {:?}", u1.damage);

    // Frame 2: CR scrolls the window, "2" lands on the base row.
    let u2 = dec.decode(&frames[1], Some(pts[1]));
    assert_eq!(u2.events.len(), 1);
    assert_eq!(u2.events[0].grid.row_text(13), "1");
    assert_eq!(u2.events[0].grid.row_text(14), "2");

    // Frame 3: the classic end state — "2" over "3".
    let u3 = dec.decode(&frames[2], Some(pts[2]));
    assert_eq!(u3.events.len(), 1);
    assert_eq!(u3.events[0].grid.row_text(13), "2");
    assert_eq!(u3.events[0].grid.row_text(14), "3");
    assert!(u3.events[0].grid.row_text(12).is_empty());

    // Cues: the half-open [pts, next_pts-1] spans, the bounding-box
    // bitmap (1 row x 1 col = 22x32 px at the 704x480 reference;
    // frame 1 shows only "1" on the base row).
    let builder = CcCueBuilder::new();
    let c1 = builder.cues(&u1, Some(pts[1]), 0).must();
    assert_eq!(c1.len(), 1);
    assert_eq!((c1[0].bitmap.width, c1[0].bitmap.height), (22, 32));
    assert_eq!(c1[0].start, Some(pts[0]));
    assert_eq!(c1[0].end, Some(pts[1] - 1));
    assert!(c1[0].active_at(pts[0]));
    assert!(!c1[0].active_at(pts[1]));
    let c3 = builder.cues(&u3, None, 0).must();
    assert_eq!(c3.len(), 1);
    assert_eq!(c3[0].end, None); // open-ended: the player's clock closes it
    assert!(c3[0].active_at(10_000_000_000));
}

#[test]
fn ga94_sei_carriage_end_to_end() {
    // The same roll-up, but through the SEI type-4 payload + the
    // decode_ga94 convenience (the codecs SEI seam consume path).
    let mut dec = CcDecoder::new();
    let payload = ga94(&[(0x14, 0x25), (0x44, 0x60), (0x31, 0x20)]);
    let u = dec.decode_ga94(&payload, Some(5)).must();
    assert_eq!(u.events.len(), 1);
    assert_eq!(u.events[0].grid.row_text(14), "1");
}

#[test]
fn pac_colors_reach_the_bitmap() {
    // Paint-on + red mid-row + "A1" (a text pair: the second byte
    // below 0x40 — "AB" would be a PAC): both cells red.
    let mut dec = CcDecoder::new();
    let u = dec.decode(
        &[
            t608(0x14, 0x29), // resume direct captioning
            t608(0x11, 0x28), // mid-row: red (takes a column)
            t608(0x41, 0x31), // "A1"
        ],
        Some(1),
    );
    assert_eq!(u.events.len(), 1);
    let grid = &u.events[0].grid;
    assert_eq!(grid.row_text(14), " A1");
    assert_eq!(grid.cell(14, 1).color, CcColor::Red);
    assert_eq!(grid.cell(14, 2).color, CcColor::Red);
    // Render: red pixels across the first occupied cell's ascent rows.
    let cues = CcCueBuilder::new().cues(&u, Some(2), 0).must();
    assert_eq!(cues.len(), 1);
    let bmp = &cues[0].bitmap;
    // The occupied bbox starts at the 'A' (col 1): the blank mid-row
    // column falls OUTSIDE the bitmap (2 cells wide, 1 row).
    assert_eq!((bmp.width, bmp.height), (2 * 22, 32));
    let red = bmp.pixel(22, 5);
    assert_eq!((red.r, red.g, red.b, red.a), (255, 0, 0, 255));
    let a_cell = bmp.pixel(5, 5);
    assert_eq!((a_cell.r, a_cell.g, a_cell.b), (255, 0, 0));
    // The anchor pins the bitmap to the OCCUPIED box (col 1, row 15:
    // v = 1/15 above the frame bottom) — the blank mid-row column is
    // excluded, proving it renders nothing.
    assert_eq!(
        cues[0].layout.anchor.u,
        subtitles::compositing::Rational::new(1, 32).must()
    );
    assert_eq!(
        cues[0].layout.anchor.v,
        subtitles::compositing::Rational::new(1, 15).must()
    );
}

#[test]
fn pop_on_double_buffers_until_end_caption() {
    let mut dec = CcDecoder::new();
    // Resume caption loading + PAC row 5 + "H1" (a text pair) ->
    // hidden only.
    let u1 = dec.decode(
        &[t608(0x14, 0x20), t608(0x45, 0x40), t608(0x48, 0x31)],
        Some(1),
    );
    assert!(u1.events.is_empty(), "pop-on writes stay off-screen");
    // End caption: the swap displays it.
    let u2 = dec.decode(&[t608(0x14, 0x2F)], Some(2));
    assert_eq!(u2.events.len(), 1);
    assert_eq!(u2.events[0].grid.row_text(4), "H1");
    // Erase displayed clears the screen (an event with an empty grid
    // renders no cue).
    let u3 = dec.decode(&[t608(0x14, 0x2C)], Some(3));
    assert_eq!(u3.events.len(), 1);
    assert!(u3.events[0].grid.is_empty());
    let cues = CcCueBuilder::new().cues(&u3, Some(4), 0).must();
    assert!(cues.is_empty());
}

#[test]
fn parity_failure_is_typed_and_dropped() {
    let mut dec = CcDecoder::new();
    // 0x14 without its parity bit: the pair is dropped.
    let u = dec.decode(
        &[CcDataTriplet::cc(0, [0x14, p(0x29)]), t608(0x31, 0x32)],
        Some(1),
    );
    assert!(u.events.is_empty());
    assert!(matches!(
        u.damage.first(),
        Some(CcDamage::Parity {
            channel: CcChannel::CC1
        })
    ));
    // The channel state was untouched by the dropped pair: a
    // following good pair decodes. The first call's "12" went to
    // the pop-on HIDDEN buffer (the default mode), leaving the
    // cursor at column 2 — paint-on then writes at that cursor.
    let u = dec.decode(&[t608(0x14, 0x29), t608(0x31, 0x32)], Some(2));
    assert_eq!(u.events.len(), 1);
    assert_eq!(u.events[0].grid.row_text(14), "  12");
}

#[test]
fn xds_and_channel_latch() {
    let mut dec = CcDecoder::new();
    // Field-2 pair with a low first byte: XDS, skipped + counted.
    let u = dec.decode(&[CcDataTriplet::cc(1, [p(0x01), p(0x03)])], Some(1));
    assert!(u.events.is_empty());
    assert!(matches!(u.damage.first(), Some(CcDamage::XdsSkipped)));
    // A CC2 control code latches field 1's active channel; text that
    // follows rides CC2, not CC1.
    let u = dec.decode(
        &[
            t608(0x1C, 0x29), // paint-on via the channel-2 prefix
            t608(0x31, 0x32),
        ],
        Some(2),
    );
    assert_eq!(u.events.len(), 1);
    assert_eq!(u.events[0].channel, CcChannel::CC2);
    assert_eq!(u.events[0].grid.row_text(14), "12");
}

#[test]
fn cea708_service1_routes_to_608_and_native_is_recorded() {
    let mut dec = CcDecoder::new();
    // One DTVCC packet, 7 bytes: svc1 size6 payload
    // [paint-on, red mid-row, "AB"]. Reassembly: start code 3 ->
    // expected 2*3+1 = 7 bytes; the first packet byte is the svc1
    // block header (0x16), NOT a 608 code.
    let u = dec.decode(
        &[
            CcDataTriplet {
                cc_valid: true,
                cc_type: 2,
                data: [0x43, 0x16],
            },
            CcDataTriplet {
                cc_valid: true,
                cc_type: 3,
                data: [p(0x94), p(0x29)],
            },
            CcDataTriplet {
                cc_valid: true,
                cc_type: 3,
                data: [p(0x11), p(0x28)],
            },
            CcDataTriplet {
                cc_valid: true,
                cc_type: 3,
                data: [p(0x41), p(0x31)],
            },
        ],
        Some(1),
    );
    // The service-1 route painted "A1" in red on CC1 (the mid-row
    // took column 0; "A1" is a text pair — second byte below 0x40,
    // unlike "AB" which would read as a PAC).
    assert_eq!(u.events.len(), 1);
    assert_eq!(u.events[0].grid.row_text(14), " A1");
    assert_eq!(u.events[0].grid.cell(14, 1).color, CcColor::Red);

    // A native packet: svc5, one byte -> recorded, never
    // interpreted. (Start code 1 -> 3 bytes; the second continuation
    // arrives with no active packet -> typed damage.)
    let u = dec.decode(
        &[
            CcDataTriplet {
                cc_valid: true,
                cc_type: 2,
                data: [0x41, 0x51],
            },
            CcDataTriplet {
                cc_valid: true,
                cc_type: 3,
                data: [0x98, 0x01],
            },
            CcDataTriplet {
                cc_valid: true,
                cc_type: 3,
                data: [0x02, 0x03],
            },
        ],
        Some(2),
    );
    assert!(u.events.is_empty());
    assert_eq!(u.services_708.len(), 1);
    assert_eq!(u.services_708[0].service, 5);
    assert_eq!(u.services_708[0].bytes, 1);
    assert!(
        u.damage
            .iter()
            .any(|d| matches!(d, CcDamage::DtvContinuationWithoutStart))
    );
    assert!(
        u.services_708[0]
            .ops
            .contains(&subtitles::cc::cea708::Cc708Op::DefineWindow)
    );
}

#[test]
fn unknown_service_and_truncated_carriage_are_typed() {
    // Service 0 header: typed unknown-service damage, nothing
    // decoded. The packet completes on the continuation (start code
    // 1 -> 3 bytes) and its walk stops at the service-0 header.
    let mut dec = CcDecoder::new();
    let u = dec.decode(
        &[
            CcDataTriplet {
                cc_valid: true,
                cc_type: 2,
                data: [0x41, 0x01],
            },
            CcDataTriplet {
                cc_valid: true,
                cc_type: 3,
                data: [0xAA, 0xBB],
            },
        ],
        Some(1),
    );
    assert!(u.events.is_empty());
    assert!(matches!(
        u.damage.first(),
        Some(CcDamage::UnknownService708 { service: 0 })
    ));
    // The carriage grammar failures are typed (nothing decoded).
    let mut dec = CcDecoder::new();
    let full = ga94(&[(0x14, 0x29)]);
    let cut = &full[..full.len() - 2];
    assert!(matches!(
        dec.decode_ga94(cut, Some(1)),
        Err(CcDataError::Truncated { .. })
    ));
    let mut bad_marker = ga94(&[(0x31, 0x32)]);
    let n = bad_marker.len();
    bad_marker[n - 1] = 0x7E;
    assert!(matches!(
        dec.decode_ga94(&bad_marker, Some(1)),
        Err(CcDataError::BadMarker { got: 0x7E })
    ));
}

#[test]
fn scte20_carriage_end_to_end() {
    // Bit-pack an SCTE-20 payload: two pairs (paint-on + "OK").
    let mut bits: Vec<u8> = Vec::new();
    let mut bit = 0u32;
    let put = |bits: &mut Vec<u8>, bit: &mut u32, n: u32, v: u32| {
        for i in (0..n).rev() {
            let set = v & (1 << i) != 0;
            if (*bit).is_multiple_of(8) {
                bits.push(0);
            }
            if set {
                let idx = bits.len() - 1;
                bits[idx] |= 0x80 >> (*bit % 8);
            }
            *bit += 1;
        }
    };
    put(&mut bits, &mut bit, 1, 1); // process_cc_data_flag
    put(&mut bits, &mut bit, 1, 0); // zero_bit
    put(&mut bits, &mut bit, 5, 2); // cc_count = 2
    for (valid, ty, d1, d2) in [(true, 0u8, 0x14u8, 0x29u8), (true, 0, 0x6F, 0x6B)] {
        put(&mut bits, &mut bit, 1, u32::from(valid));
        put(&mut bits, &mut bit, 2, u32::from(ty));
        put(&mut bits, &mut bit, 7, u32::from(d1));
        put(&mut bits, &mut bit, 1, 0); // reserved
        put(&mut bits, &mut bit, 7, u32::from(d2));
        put(&mut bits, &mut bit, 1, 0); // reserved
    }
    let mut payload = vec![CC_TYPE_CODE];
    payload.extend_from_slice(&bits);
    payload.push(0xFF);

    let triplets = parse_scte20(&payload).must();
    assert_eq!(triplets.len(), 2);
    let mut dec = CcDecoder::new();
    let u = dec.decode(&triplets, Some(1));
    assert_eq!(u.events.len(), 1);
    assert_eq!(u.events[0].grid.row_text(14), "ok");
    assert!(u.damage.is_empty());
}

/// A seeded xorshift64 (the N2 fuzz-survival precedent).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn fuzz_lite_survival_seeded_mutations() {
    // Seeds: the hand-crafted grammars above. Mutations: byte
    // writes, truncations, insertions. Budget: < 200 KB total input.
    let seeds: Vec<Vec<u8>> = vec![
        ga94(&[(0x14, 0x25), (0x44, 0x60), (0x31, 0x20)]),
        ga94(&[(0x14, 0x20), (0x45, 0x40), (0x48, 0x49), (0x14, 0x2F)]),
        {
            // The SCTE-20 payload from the test above.
            let mut v = vec![CC_TYPE_CODE, 0x42];
            v.extend_from_slice(&[0b0100_0010, 0b0100_0000, 1, 0b0010_0101, 0]);
            v.extend_from_slice(&[0xFF]);
            v
        },
        // A DTVCC packet start + continuations (the 708 surface).
        vec![0x43, 0x14, 0x94, 0x29, 0x11, 0x28, 0x41, 0x42],
    ];
    let mut rng = Lcg(0x5EED_6087_08CA_FE00);
    let mut total = 0usize;
    const BUDGET: usize = 200 * 1024;
    let mut rounds = 0usize;
    while total < BUDGET && rounds < 16_000 {
        let seed = &seeds[rng.below(seeds.len())];
        let mut buf = seed.clone();
        let mutations = 1 + rng.below(8);
        for _ in 0..mutations {
            match rng.below(4) {
                0 if !buf.is_empty() => {
                    let i = rng.below(buf.len());
                    buf[i] = rng.next() as u8;
                }
                1 if !buf.is_empty() => {
                    buf.truncate(rng.below(buf.len() + 1));
                }
                2 => {
                    let i = rng.below(buf.len() + 1);
                    buf.insert(i, rng.next() as u8);
                }
                _ => {
                    // Bit-flip a run of bytes.
                    if !buf.is_empty() {
                        let i = rng.below(buf.len());
                        buf[i] ^= 1 << rng.below(8);
                    }
                }
            }
        }
        total += buf.len();
        // The harness contract: no panic, bounded damage. The
        // mutator feeds fuzz_sub_cc directly.
        subtitles::cc::fuzz::fuzz_sub_cc(&buf);
        // One direct decoder pass asserts the ledger cap invariants.
        let mut dec = CcDecoder::new();
        if let Ok(triplets) = parse_ga94_sei(&buf) {
            let u = dec.decode(&triplets, Some(1));
            assert!(u.damage.len() <= MAX_DAMAGE_EVENTS);
        }
        if let Ok(triplets) = parse_scte20(&buf) {
            let u = dec.decode(&triplets, Some(1));
            assert!(u.damage.len() <= MAX_DAMAGE_EVENTS);
        }
        rounds += 1;
    }
    assert!(total > 100 * 1024, "coverage budget spent: {total} bytes");
}

#[test]
fn systematic_truncation_sweep_survives() {
    // Every truncation of every seed through every surface — the
    // typed-truncation paths are the whole point.
    let seeds: Vec<Vec<u8>> = vec![
        ga94(&[(0x14, 0x25), (0x44, 0x60), (0x31, 0x20), (0x14, 0x2D)]),
        vec![
            0x03,
            0x42,
            0b0100_0010,
            0b0100_0000,
            1,
            0b0010_0101,
            0,
            0xFF,
        ],
    ];
    for seed in &seeds {
        for cut in 0..seed.len() {
            subtitles::cc::fuzz::fuzz_sub_cc(&seed[..cut]);
        }
        subtitles::cc::fuzz::fuzz_sub_cc(seed);
    }
}
