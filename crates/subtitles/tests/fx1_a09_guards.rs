//! FX1-A09 regression guards — the wave-09 audit findings for the
//! subtitles crate that each shipped as one typed guard:
//!
//! * **CC PAC range (W09-A07, S0)** — EIA-608 preamble address codes
//!   are a CONTROL-family first byte (`0x10..=0x1F`) plus a text-range
//!   second byte (`0x40..=0x7F`); the pre-fix dialect keyed PACs on
//!   text first bytes `0x40..=0x5F`, so the audit's real-spec pop-on
//!   "HELLO WORLD!" rendered as "D!" and paint-on "AB" vanished. The
//!   test feeds the audit's exact byte sequence and asserts the full
//!   text on the PAC's row.
//! * **PGS composition amplification (W09-A04, S1)** — a PCS may
//!   re-reference one canvas up to 255 times; pre-fix `compose()`
//!   re-decoded + re-copied the canvas per reference (~4 GiB retained
//!   from a ~2 KiB display set). The guard caps retained RGBA
//!   ([`subtitles::pgs::MAX_COMPOSED_RGBA_BYTES`]) with one typed
//!   [`subtitles::pgs::PgsDamage::CompositionRetained`] event.
//! * **DVB stale-CLUT index (W09-A02/A08, S1)** — redefining a region
//!   at a shallower depth keeps its painted pixel buffer; `bank[index]`
//!   read out of bounds on the stale 8-bit indices. The guard turns
//!   the first stale index into a typed
//!   [`subtitles::dvb::DvbDamage::StaleClutIndex`] and skips the
//!   region.

use subtitles::cc::cc_data::CcDataTriplet;
use subtitles::cc::eia608::{CcChannel, with_odd_parity};
use subtitles::cc::{CcDamage, CcDecoder};
use subtitles::dvb::{DvbDamage, DvbDecoder};
use subtitles::pgs::{self, PgsDecoder};

// ---------------------------------------------------------------------------
// CC PAC range — the audit's exact "HELLO WORLD!" reproduction.
// ---------------------------------------------------------------------------

/// Parity-complete one 608 byte.
fn p(b: u8) -> u8 {
    with_odd_parity(b)
}

/// A valid 608 field-1 triplet carrying the parity-complete pair.
fn t608(d1: u8, d2: u8) -> CcDataTriplet {
    CcDataTriplet::cc(0, [p(d1), p(d2)])
}

/// The audit's pop-on reproduction, verbatim: RCL + PAC row 5
/// (`0x15 0x40` — control-range first byte) + the six text pairs of
/// "HELLO WORLD!" + EOC. Pre-fix, five of the six pairs were eaten as
/// "PACs" and the real PAC was dropped ("D!" survived).
fn hello_world_triplets() -> Vec<CcDataTriplet> {
    [
        (0x14u8, 0x20u8), // RCL — resume caption loading
        (0x15, 0x40),     // PAC: row 5, white pen
        (0x48, 0x45),     // "HE"
        (0x4C, 0x4C),     // "LL"
        (0x4F, 0x20),     // "O "
        (0x57, 0x4F),     // "WO"
        (0x52, 0x4C),     // "RL"
        (0x44, 0x21),     // "D!"
        (0x14, 0x2F),     // EOC — end of caption (swap)
    ]
    .map(|(d1, d2)| t608(d1, d2))
    .to_vec()
}

/// The audit's exact "HELLO WORLD!" byte sequence decodes to the full
/// text on the PAC's row — every text pair is text, the real PAC
/// positions the cursor, no damage.
#[test]
fn fx1_a09_cc_pac_range_renders_hello_world() {
    let mut dec = CcDecoder::new();
    let u = dec.decode(&hello_world_triplets(), Some(90_000));
    assert!(u.damage.is_empty(), "damage: {:?}", u.damage);
    assert_eq!(u.events.len(), 1, "the EOC swap displays once");
    assert_eq!(u.events[0].channel, CcChannel::CC1);
    assert_eq!(
        u.events[0].grid.row_text(4),
        "HELLO WORLD!",
        "the full caption on row 5 (the PAC's row), not the pre-fix \"D!\""
    );
}

/// The audit's second executed proof: paint-on "AB" (a text pair
/// whose first byte is 0x41 and second 0x42 — the pre-fix dialect ate
/// it as a PAC) renders as text.
#[test]
fn fx1_a09_cc_text_pair_ab_survives_paint_on() {
    let mut dec = CcDecoder::new();
    let u = dec.decode(
        &[t608(0x14, 0x29), t608(0x41, 0x42)], // RDC, "AB"
        Some(90_000),
    );
    assert!(u.damage.is_empty(), "damage: {:?}", u.damage);
    assert_eq!(u.events.len(), 1);
    assert_eq!(u.events[0].grid.row_text(14), "AB", "\"AB\" is text now");
    // The damage taxonomy stays reachable: a truly invalid PAC row
    // slot (first 0x10 + second with bit 5, the 0x60-0x7F indent
    // window on the invalid row-11 slot) is still typed.
    let mut dec = CcDecoder::new();
    let u = dec.decode(&[t608(0x10, 0x60)], Some(1));
    assert!(
        u.damage
            .iter()
            .any(|d| matches!(d, CcDamage::InvalidPac { .. })),
        "invalid PAC slot still typed: {:?}",
        u.damage
    );
}

// ---------------------------------------------------------------------------
// PGS composition retention cap.
// ---------------------------------------------------------------------------

/// One stripped-framing PGS segment: kind + 16-bit length + payload.
fn seg(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut v = vec![kind];
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

/// A PCS (video 1920x1080) referencing `objects.len()` composition
/// objects (all id 1, at 0,0).
fn pcs(state: u8, palette_id: u8, objects: usize) -> Vec<u8> {
    let mut d = vec![
        0x07,
        0x80, // width 1920
        0x04,
        0x38, // height 1080
        0x20, // frame-rate id 2
        0x00,
        0x01, // composition number 1
        state,
        0x00, // palette_update false
        palette_id,
        objects as u8,
    ];
    for _ in 0..objects {
        // object id 1, window 0, flags 0, x=0, y=0.
        d.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }
    seg(0x16, &d)
}

/// One 2048x2048 object (the `rle::MAX_OBJECT_PIXELS` worst case)
/// whose RLE fills the canvas exactly with index-0 long runs — 256
/// three-byte 16383-px runs + one 256-px run, a clean decode.
fn max_canvas_ods() -> Vec<u8> {
    let mut rle = Vec::with_capacity(256 * 3 + 3);
    for _ in 0..256 {
        rle.extend_from_slice(&[0x00, 0x7F, 0xFF]);
    }
    rle.extend_from_slice(&[0x00, 0x41, 0x00]);
    let total = 4 + rle.len();
    let mut d = vec![0x00, 0x01, 0x00, 0x80]; // id 1, version 0, first
    // The declared total is a 3-byte BE field covering width+height
    // too (the crate's own `ods_first_declared` helper). `total` is a
    // usize, so `to_be_bytes()[1..4]` is zero bytes on a 64-bit host:
    // the ODS then declares a 0-byte RLE, the fragment is dropped as
    // FragmentOverrun and the decode is truncated — spurious RLE
    // damage riding along with the retention cap.
    d.extend_from_slice(&[(total >> 16) as u8, (total >> 8) as u8, total as u8]);
    d.extend_from_slice(&2048u16.to_be_bytes());
    d.extend_from_slice(&2048u16.to_be_bytes());
    d.extend_from_slice(&rle);
    seg(0x15, &d)
}

/// The audit's amplification shape, scaled to the cap: a ~2 KiB
/// display set (PCS listing one max-canvas object five times + PDS +
/// ODS + END). Pre-fix, compose() re-decoded + retained five
/// 16 MiB copies; the guard trips on the fifth and keeps exactly the
/// first four (64 MiB, [`pgs::MAX_COMPOSED_RGBA_BYTES`]).
#[test]
fn fx1_a09_pgs_retention_cap_trips_typed_not_unbounded() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&pcs(0x80, 0, 5));
    buf.extend_from_slice(&seg(0x14, &[0x00, 0x00])); // PDS id 0, no entries
    buf.extend_from_slice(&max_canvas_ods());
    buf.extend_from_slice(&seg(0x80, &[])); // END
    let mut dec = PgsDecoder::new();
    let set = dec.decode_display_set(&buf, Some(90_000));
    // The cap fires once, typed: the fifth reference is dropped.
    let hits = set
        .damage
        .iter()
        .filter(|d| matches!(d, pgs::PgsDamage::CompositionRetained { .. }))
        .count();
    assert_eq!(hits, 1, "one typed retention event: {:?}", set.damage);
    assert!(
        set.damage
            .iter()
            .any(|d| matches!(d, pgs::PgsDamage::CompositionRetained { bytes } if *bytes == 5 * 16_777_216)),
        "the tripping total is five canvases: {:?}",
        set.damage
    );
    // Exactly the first four references are retained — the retained
    // bytes equal the cap, never the pre-fix 5x/255x amplification.
    assert_eq!(set.objects.len(), 4);
    let retained: usize = set.objects.iter().map(|o| o.rgba.len()).sum();
    assert_eq!(retained, pgs::MAX_COMPOSED_RGBA_BYTES);
    for o in &set.objects {
        assert_eq!(
            o.rgba.len(),
            2048 * 2048 * 4,
            "each composed image is one full canvas"
        );
    }
    // The RLE itself decodes clean — only the retention event fires.
    assert!(
        !set.damage
            .iter()
            .any(|d| matches!(d, pgs::PgsDamage::RleOverrun { .. })),
        "no spurious RLE damage: {:?}",
        set.damage
    );
}

// ---------------------------------------------------------------------------
// DVB stale-CLUT index.
// ---------------------------------------------------------------------------

/// One DVB segment: 6-byte header (sync 0x0F, kind, page id, length).
fn dvb_seg(kind: u8, page: u16, data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x0F, kind];
    v.extend_from_slice(&page.to_be_bytes());
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

/// The audit's deterministic stale-CLUT attack, one PES payload: (1)
/// page composition listing region 1; (2) region 1, 4x2, depth_code 3
/// (8-bit), fill=0, object 1 placed at (0,0); (3) object data: an
/// 8-bit block painting pixel value 200 (dirty=true); (4) region 1
/// REDEFINED at the same 4x2 but depth_code 1 (2-bit), fill=0 — the
/// same-size redefinition mutates the depth and keeps the painted
/// pixel buffer; (5) end-of-display-set → compose. Pre-fix,
/// `bank4[200]` read out of bounds (a release-profile panic).
fn stale_clut_payload() -> Vec<u8> {
    let mut buf = Vec::new();
    // Page composition: region 1 at (0,0), normal state.
    buf.extend_from_slice(&dvb_seg(
        0x10,
        1,
        &[0x0A, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
    ));
    // Region 1: 4x2, 8-bit (depth_code 3), clut 0 (never defined),
    // no fill, object 1 at (0,0).
    buf.extend_from_slice(&dvb_seg(
        0x11,
        1,
        &[
            0x01, 0x00, 0x00, 0x04, 0x00, 0x02, 0x2C, 0x00, 0x00, 0x00, //
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ],
    ));
    // Object 1: one top-field line whose 8-bit block paints value 200.
    buf.extend_from_slice(&dvb_seg(
        0x13,
        1,
        &[0x00, 0x01, 0x00, 0x00, 0x02, 0x00, 0x00, 0x12, 0xC8],
    ));
    // Region 1 redefined: same 4x2, depth_code 1 (2-bit), no fill.
    buf.extend_from_slice(&dvb_seg(
        0x11,
        1,
        &[0x01, 0x00, 0x00, 0x04, 0x00, 0x02, 0x24, 0x00, 0x00, 0x00],
    ));
    buf.extend_from_slice(&dvb_seg(0x80, 1, &[]));
    buf
}

/// The stale-CLUT index is typed and the region is skipped — no panic,
/// no wrong-palette render.
#[test]
fn fx1_a09_dvb_stale_clut_index_is_typed_not_a_panic() {
    let mut dec = DvbDecoder::new();
    let up = dec.decode_pes(&stale_clut_payload(), Some(90_000));
    assert!(up.regions.is_empty(), "the stale region is skipped");
    assert!(
        up.damage.iter().any(|d| matches!(
            d,
            DvbDamage::StaleClutIndex {
                region_id: 1,
                index: 200
            }
        )),
        "the first stale index (200) is typed: {:?}",
        up.damage
    );
    assert!(
        up.damage.iter().any(|d| matches!(
            d,
            DvbDamage::BadClutId {
                clut_id: 0,
                region_id: 1
            }
        )),
        "the undefined CLUT keeps its typed fallback: {:?}",
        up.damage
    );
}

/// The mirror: a region whose indices all fit its depth still renders
/// (the guard skips only the stale-buffer case) — the same shape with
/// a 2-bit paint (index 3) composes through the 4-entry bank.
#[test]
fn fx1_a09_dvb_fresh_clut_indices_still_render() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&dvb_seg(
        0x10,
        1,
        &[0x0A, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
    ));
    buf.extend_from_slice(&dvb_seg(
        0x11,
        1,
        &[
            0x01, 0x00, 0x00, 0x04, 0x00, 0x02, 0x24, 0x00, 0x00, 0x00, //
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ],
    ));
    // Object 1: 2-bit block painting indices 1 and 3 (the top
    // 4-entry bank) then EOL.
    buf.extend_from_slice(&dvb_seg(
        0x13,
        1,
        &[
            0x00,
            0x01,
            0x00,
            0x00,
            0x02,
            0x00,
            0x00,
            0x10,
            0b01_11_00_00,
        ],
    ));
    buf.extend_from_slice(&dvb_seg(0x80, 1, &[]));
    let mut dec = DvbDecoder::new();
    let up = dec.decode_pes(&buf, Some(90_000));
    assert_eq!(up.regions.len(), 1, "the region composes: {:?}", up.damage);
    assert!(
        !up.damage
            .iter()
            .any(|d| matches!(d, DvbDamage::StaleClutIndex { .. })),
        "in-bounds indices are not stale: {:?}",
        up.damage
    );
    assert_eq!(up.regions[0].rgba.len(), 4 * 2 * 4);
}
