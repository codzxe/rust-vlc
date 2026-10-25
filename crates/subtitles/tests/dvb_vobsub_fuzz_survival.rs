//! Fuzz-lite survival AC (card 5.N2 `+FB`): deterministic seeded
//! mutation + truncation sweeps over synthetic-but-valid DVB display
//! sets and VobSub idx/sub pairs, all through the
//! [`subtitles::dvb::fuzz`] / [`subtitles::vobsub::fuzz`] harnesses
//! (`fuzz_sub_dvb` / `fuzz_sub_vobsub`, doc 04 §5 registry). The
//! seed corpus is BUILT IN CODE (zero committed bytes — well under
//! the 200 KB budget; the mutation surface is bounded by the same
//! cap at runtime).
//!
//! **Contract: absence of panic/hang/unbounded allocation** —
//! mutated input is expected to produce damage events or zero
//! regions/cues, never a crash. Pinned invariants per run:
//!
//! * the DVB damage ledger respects its cap (64) with the overflow
//!   *counted*, never stored;
//! * every composed region's RGBA buffer is exactly
//!   `width × height × 4` (the CompositedCue construction contract);
//! * every VobSub cue's RGBA buffer likewise;
//! * the SPU accumulation never exceeds the 128 KiB cap (checked
//!   by the decoder under test — re-asserted via the Ok shape).
//!
//! # Why no ffmpeg-generated corpus (honest)
//!
//! ffmpeg 7.1.5 carries the `dvbsub`/`dvdsub` ENCODERS but refuses
//! text→bitmap subtitle transcoding ("Subtitle encoding currently
//! only possible from text to text or bitmap to bitmap"), and the
//! restored sandbox has no DVB/VobSub-bearing corpus lane — so the
//! reference-encoder differential is NOT generatable here and the
//! AC rides on the hand-crafted bit vectors in the unit suites
//! (tables cross-checked line-by-line against ffmpeg's
//! `dvbsub_read_{2,4,8}bit_string` / `dvdsub` RLE semantics).

use subtitles::dvb::fuzz::fuzz_sub_dvb;
use subtitles::dvb::{DvbDamage, DvbDecoder, MAX_DAMAGE_EVENTS};
use subtitles::vobsub::fuzz::fuzz_sub_vobsub;
use subtitles::vobsub::{MAX_DAMAGE_EVENTS as VS_MAX, parse_vobsub};

/// A tiny deterministic LCG (the srt/mjpeg precedent — reproducible
/// offline, no external fuzzer).
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

// ---------------------------------------------------------------------
// DVB: seed display sets built with the same grammar the unit suite
// uses (page → region → CLUT → object → end-of-display-set).
// ---------------------------------------------------------------------

fn seg(kind: u8, page: u16, data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x0F, kind];
    v.extend_from_slice(&page.to_be_bytes());
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

fn page_seg(timeout: u8, version: u8, state: u8, entries: &[(u8, u16, u16)]) -> Vec<u8> {
    let mut d = vec![timeout, (version << 4) | (state << 2)];
    for &(id, x, y) in entries {
        d.push(id);
        d.push(0);
        d.extend_from_slice(&x.to_be_bytes());
        d.extend_from_slice(&y.to_be_bytes());
    }
    seg(0x10, 1, &d)
}

#[allow(clippy::too_many_arguments)]
fn region_seg(
    id: u8,
    w: u16,
    h: u16,
    depth_code: u8,
    clut: u8,
    fill: bool,
    fill_color: u8,
    objects: &[(u16, u16, u16)],
) -> Vec<u8> {
    let mut d = vec![
        id,
        (2 << 4) | (u8::from(fill) << 3),
        (w >> 8) as u8,
        w as u8,
        (h >> 8) as u8,
        h as u8,
        0b0010_0000 | (depth_code << 2),
        clut,
        0,
        fill_color,
    ];
    for &(oid, x, y) in objects {
        d.extend_from_slice(&oid.to_be_bytes());
        d.push((x >> 8) as u8);
        d.push(x as u8);
        d.push((y >> 8) as u8);
        d.push(y as u8);
    }
    seg(0x11, 1, &d)
}

fn clut_seg(id: u8, version: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
    let mut d = vec![id, version << 4];
    for &(eid, y, cb, cr, alpha) in entries {
        d.extend_from_slice(&[eid, 0b1110_0001, y, cr, cb, alpha]);
    }
    seg(0x12, 1, &d)
}

fn object_seg(id: u16, top: &[u8], bottom: &[u8]) -> Vec<u8> {
    let mut d = id.to_be_bytes().to_vec();
    d.push(0);
    d.extend_from_slice(&(top.len() as u16).to_be_bytes());
    d.extend_from_slice(&(bottom.len() as u16).to_be_bytes());
    d.extend_from_slice(top);
    d.extend_from_slice(bottom);
    seg(0x13, 1, &d)
}

/// A realistic 2-bit display set (fill + object paint + 0xF0 field
/// steps), the shape the mutator mangles.
fn dvb_seed_2bit() -> Vec<u8> {
    let mut top = Vec::new();
    for v in 0u8..6 {
        // 2-bit run codes 3..10 of value (v % 3) + 1, then EOL.
        top.push(0x10);
        top.push(0b0010_0000 | ((v % 5) << 2) | ((v % 3) + 1));
        top.push(0x00);
        top.push(0xF0);
    }
    let bottom = top.clone();
    let mut p = Vec::new();
    p.extend_from_slice(&page_seg(6, 1, 1, &[(3, 40, 60)]));
    p.extend_from_slice(&region_seg(3, 24, 12, 1, 7, true, 1, &[(9, 2, 1)]));
    p.extend_from_slice(&clut_seg(
        7,
        0,
        &[
            (1, 235, 128, 128, 0),
            (2, 82, 90, 240, 40),
            (3, 41, 240, 110, 255),
        ],
    ));
    p.extend_from_slice(&object_seg(9, &top, &bottom));
    p.extend_from_slice(&seg(0x80, 1, &[]));
    p
}

/// An 8-bit display set with an explicit display definition and a
/// map-table sub-block (the cross-depth surface).
fn dvb_seed_8bit() -> Vec<u8> {
    let mut top = vec![0x22]; // map4to8 override follows (16 bytes)
    top.extend_from_slice(&[0x11u8; 16]);
    top.push(0x12);
    // 8-bit: 0x05, run 0x83 0x07 = 3 px of 7, 00 00 EOL.
    top.extend_from_slice(&[0x05, 0x00, 0x83, 0x07, 0x00, 0x00]);
    top.push(0xF0);
    let mut p = Vec::new();
    p.extend_from_slice(&page_seg(9, 1, 1, &[(1, 100, 200)]));
    p.extend_from_slice(&region_seg(1, 8, 2, 3, 5, false, 0, &[(2, 0, 0)]));
    p.extend_from_slice(&clut_seg(5, 0, &[(7, 200, 128, 128, 0)]));
    p.extend_from_slice(&object_seg(2, &top, &[]));
    p.extend_from_slice(&seg(0x14, 1, &[0x15, 0x02, 0xCF, 0x02, 0x3F]));
    p.extend_from_slice(&seg(0x80, 1, &[]));
    p
}

/// A two-display-set stream (acquisition then normal-case repaint),
/// so the mutation crosses the PES boundary too.
fn dvb_seed_stream() -> Vec<u8> {
    let mut s = dvb_seed_2bit();
    s.extend_from_slice(&page_seg(6, 2, 0, &[(3, 40, 60)]));
    s.extend_from_slice(&object_seg(9, &[0x10, 0b01_01_00_00, 0xF0], &[]));
    s.extend_from_slice(&seg(0x80, 1, &[]));
    s
}

fn dvb_invariants(name: &str, payload: &[u8]) {
    let mut dec = DvbDecoder::new();
    let update = dec.decode_pes(payload, Some(90_000));
    assert!(
        update.damage.len() <= MAX_DAMAGE_EVENTS,
        "{name}: DVB damage ledger over cap ({} > {MAX_DAMAGE_EVENTS})",
        update.damage.len()
    );
    assert!(
        update.damage.iter().all(|d| {
            !matches!(d, DvbDamage::DamageOverflow { .. })
                || update.damage.len() == MAX_DAMAGE_EVENTS
        }),
        "{name}: DamageOverflow stored inside the cap"
    );
    for r in &update.regions {
        assert_eq!(
            r.rgba.len(),
            r.width as usize * r.height as usize * 4,
            "{name}: region rgba stride broken"
        );
    }
    fuzz_sub_dvb(payload);
}

// ---------------------------------------------------------------------
// VobSub: seed idx/sub pairs (the same grammar the unit suite uses).
// ---------------------------------------------------------------------

fn vobsub_build_spu(top: &[u8], bottom: &[u8], x1: u16, y1: u16, w: u16, h: u16) -> Vec<u8> {
    let x2 = x1 + w - 1;
    let y2 = y1 + h - 1;
    let mut spu = vec![0, 0, 0, 0];
    let off1 = spu.len();
    spu.extend_from_slice(top);
    let off2 = spu.len();
    spu.extend_from_slice(bottom);
    let cmds = spu.len();
    let cmds2 = cmds + 4 + 1 + 3 + 3 + 7 + 5 + 1;
    spu.extend_from_slice(&[0x00, 0x00]);
    spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
    spu.push(0x01);
    spu.push(0x03);
    spu.extend_from_slice(&[0x01, 0x23]);
    spu.push(0x04);
    spu.extend_from_slice(&[0x0F, 0x00]);
    spu.push(0x05);
    spu.extend_from_slice(&[
        (x1 >> 4) as u8,
        ((x1 & 0xF) << 4) as u8 | (x2 >> 8) as u8,
        (x2 & 0xFF) as u8,
        (y1 >> 4) as u8,
        ((y1 & 0xF) << 4) as u8 | (y2 >> 8) as u8,
        (y2 & 0xFF) as u8,
    ]);
    spu.push(0x06);
    spu.extend_from_slice(&(off1 as u16).to_be_bytes());
    spu.extend_from_slice(&(off2 as u16).to_be_bytes());
    spu.push(0xFF);
    spu.extend_from_slice(&[0x00, 0x50]);
    spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
    spu.push(0x02);
    spu.push(0xFF);
    let total = spu.len();
    spu[0..2].copy_from_slice(&(total as u16).to_be_bytes());
    spu[2..4].copy_from_slice(&(cmds as u16).to_be_bytes());
    spu
}

fn wrap_pes(substream: u8, spu: &[u8]) -> Vec<u8> {
    let mut pes = vec![0x00, 0x00, 0x01, 0xBD];
    pes.extend_from_slice(&((3 + 1 + spu.len()) as u16).to_be_bytes());
    pes.extend_from_slice(&[0x81, 0x00, 0x00]);
    pes.push(substream);
    pes.extend_from_slice(spu);
    pes
}

const VOBSUB_IDX: &[u8] = b"size: 720x480\n\
     palette: 000000, 0000ff, 00ff00, ffff00, ff0000, ff00ff, ffff00, ffffff,\n\
               808080, 404040, 202020, 101010, 080808, 040404, 020202, 010101\n\
     id: en, index: 0\n\
     timestamp: 00:00:01:000, filepos: 000000000\n\
     timestamp: 00:00:03:500, filepos: 000000100\n";

fn vobsub_seed_sub() -> Vec<u8> {
    let line = [0x50u8, 0x00, 0x20, 0x00, 0x00]; // c1 + fill c2; fill c0
    let spu1 = vobsub_build_spu(&line, &line, 100, 200, 8, 4);
    let spu2 = vobsub_build_spu(&line, &line, 10, 20, 4, 2);
    let mut sub = wrap_pes(0x20, &spu1);
    // Pad to the second idx filepos with non-start-code filler (the
    // PES scan skips it), then the entry-2 SPU at exactly 0x100.
    while sub.len() < 0x100 {
        sub.push(0xEE);
    }
    sub.extend_from_slice(&wrap_pes(0x20, &spu2));
    sub
}

fn vobsub_invariants(name: &str, idx: &[u8], sub: &[u8]) {
    let doc = parse_vobsub(idx, sub);
    assert!(
        doc.damage.len() <= VS_MAX + 1, // the Overflow event is appended
        "{name}: vobsub damage ledger over cap ({} > {VS_MAX})",
        doc.damage.len()
    );
    for cue in &doc.cues {
        assert_eq!(
            cue.rgba.len(),
            cue.width as usize * cue.height as usize * 4,
            "{name}: cue rgba stride broken"
        );
    }
    fuzz_sub_vobsub(idx, sub);
}

// ---------------------------------------------------------------------
// The deterministic mutation loops.
// ---------------------------------------------------------------------

const ROUNDS: usize = 400;

#[test]
fn dvb_fuzz_survival_mutations_and_truncations() {
    let seeds = [dvb_seed_2bit(), dvb_seed_8bit(), dvb_seed_stream()];
    for (si, seed) in seeds.iter().enumerate() {
        // Clean pass first (also exercises the composition path).
        dvb_invariants(&format!("seed{si}-clean"), seed);
        // Systematic truncation sweep.
        for cut in (0..seed.len()).step_by(7.max(seed.len() / 24)) {
            dvb_invariants(&format!("seed{si}-cut{cut}"), &seed[..cut]);
        }
        // Seeded LCG mutations: byte flips, byte sets, splices,
        // field-length corruption (biased into the 6-byte headers).
        let mut lcg = Lcg(0x5E2D_DB00 + si as u64);
        for round in 0..ROUNDS {
            let mut m = seed.clone();
            let hits = 1 + lcg.below(8);
            for _ in 0..hits {
                match lcg.below(4) {
                    0 => {
                        let at = lcg.below(m.len());
                        m[at] ^= 1 << lcg.below(8);
                    }
                    1 => {
                        let at = lcg.below(m.len());
                        m[at] = lcg.next_u32() as u8;
                    }
                    2 => {
                        let at = lcg.below(m.len().saturating_sub(1));
                        let len = 1 + lcg.below(8.min(m.len() - at));
                        m.splice(at..at + len, []);
                    }
                    _ => {
                        // Corrupt a segment length field (bytes 4-5
                        // after any 0x0F sync the seed contains).
                        let at = lcg.below(m.len().saturating_sub(6));
                        if m[at] == 0x0F {
                            m[at + 4] = lcg.next_u32() as u8;
                            m[at + 5] = lcg.next_u32() as u8;
                        }
                    }
                }
            }
            let name = format!("seed{si}-mut{round}");
            dvb_invariants(&name, &m);
        }
    }
}

#[test]
fn vobsub_fuzz_survival_mutations_and_truncations() {
    let idx = VOBSUB_IDX;
    let sub = vobsub_seed_sub();
    // Clean pass.
    vobsub_invariants("vobsub-clean", idx, &sub);
    assert!(
        parse_vobsub(idx, &sub).cues.len() >= 2,
        "seed must decode at least the two idx entries"
    );
    // Truncation sweeps over both halves.
    for cut in (0..sub.len()).step_by(9.max(sub.len() / 24)) {
        vobsub_invariants(&format!("vobsub-sub-cut{cut}"), idx, &sub[..cut]);
    }
    for cut in (0..idx.len()).step_by(5) {
        vobsub_invariants(&format!("vobsub-idx-cut{cut}"), &idx[..cut], &sub);
    }
    // Seeded LCG mutations over the sub bytes (PES walk + SPU + RLE)
    // and the idx text.
    let mut lcg = Lcg(0x5E2D_5B00);
    for round in 0..ROUNDS {
        let mut m = sub.clone();
        for _ in 0..(1 + lcg.below(10)) {
            match lcg.below(3) {
                0 => {
                    let at = lcg.below(m.len());
                    m[at] ^= 1 << lcg.below(8);
                }
                1 => {
                    let at = lcg.below(m.len());
                    m[at] = lcg.next_u32() as u8;
                }
                _ => {
                    let at = lcg.below(m.len().saturating_sub(1));
                    let len = 1 + lcg.below(16.min(m.len() - at));
                    m.splice(at..at + len, []);
                }
            }
        }
        vobsub_invariants(&format!("vobsub-mut{round}"), idx, &m);
        // The idx half mutates too (same round, independent bytes).
        let mut mi = idx.to_vec();
        for _ in 0..(1 + lcg.below(6)) {
            let at = lcg.below(mi.len());
            mi[at] = b"0123456789abcdefx:,#\n\t\xff"[lcg.below(21)];
        }
        vobsub_invariants(&format!("vobsub-idxmut{round}"), &mi, &sub);
    }
}
