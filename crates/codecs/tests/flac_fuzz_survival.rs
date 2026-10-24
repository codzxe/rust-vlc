//! Committed-fixture + seeded-mutation survival run for the
//! `fuzz_codec_flac` harness (card 5.A1 `+FB`): truncated, flipped,
//! spliced and fact-lied FLAC, plus the real corpus's dirty FLAC
//! mutants and garbage bins (D-11).
//!
//! # What runs here
//!
//! Three surfaces per mutant, in this order:
//!
//! 1. **The registered fuzz harness** — [`fuzz_codec_flac`], the very
//!    entry the proposed `fuzz/fuzz_targets/fuzz_codec_flac.rs` will
//!    call (its contract is absence-of-panic/hang/unbounded
//!    allocation, nothing else — damage is *expected* to surface as a
//!    typed error there and is swallowed).
//! 2. **The metadata walk** — [`parse_stream`]: every `Ok` must be a
//!    self-consistent layout (`frames_start` inside the data), every
//!    `Err` a typed `flac.*` code at stage `Decode` (`Unsupported` for
//!    the probe miss, `Corrupt` for structural damage).
//! 3. **The per-frame decode** — every frame the walker finds in a
//!    surviving layout is decoded through [`decode_frame`] with the
//!    stream's own facts: every failure is a typed `flac.*`
//!    `Corrupt`/`Unsupported` from the documented taxonomy, and every
//!    success is byte-length honest (`block_size × channels ×
//!    output width`).
//!
//! # The survival properties (D-3 dirty world, D-5 adversarial)
//!
//! - **(a) No panic anywhere** — a completing test is the assertion;
//!   the suite matches every `Result` (`testkit::Must` /
//!   `unreachable!` only on impossible-by-contract branches, the
//!   repo's test precedent).
//! - **(b) Every failure is typed** — asserted per mutant against the
//!   documented `flac.*` code set; a panic, an `unwrap`, or a bare
//!   `Err` without a stable code fails the suite.
//! - **(c) Per-frame isolation is exact** — for flip mutants whose
//!   damage lands entirely in the frame region, and for truncations,
//!   every frame whose own bytes were *not* touched must decode to
//!   the **identical bytes** as in the clean decode (FLAC frames are
//!   independent; the decoder is stateless between packets — pinned
//!   here, not just claimed). Splices and metadata-region flips have
//!   no clean baseline (offsets shift / facts lie) and only run the
//!   survival + typing surfaces.
//!
//! # Mutation classes (seeded, reproducible)
//!
//! Truncations (stratified + random prefix lengths), bit flips
//! (1..=8 XORs at random offsets, whole-file and metadata-only
//! strata — the facts-lie class), and garbage splices (8..=64 random
//! bytes at random offsets). Fixed seeds are recorded below; any
//! failure is exactly reproducible. The exhaustive single-byte flip
//! sweep over the mono fixture is the `#[ignore]` deep run.
//!
//! # Provenance (honest, D-5)
//!
//! The valid payloads are the card's own ffmpeg-authored fixtures
//! (`tests/fixtures/*.flac`, generation commands recorded in
//! `tests/data/flac_bitexact_reference.txt`). The external dirty
//! mutants are the corpus's two FLAC-named dirty files plus all
//! garbage bins — corpus-gated (SKIP with a message when the /tmp
//! corpus is absent, like every corpus-dependent suite).

mod flac;

use codecs::audio::flac::fuzz::fuzz_codec_flac;
use codecs::audio::flac::{decode_frame, parse_stream};
use flac::{corpus_dir, fixture_bytes, walk_frames};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The committed fixture set (small, mono, 24-bit, seektable carrier).
const FIXTURES: [&str; 4] = [
    "flac_s16_stereo_44100",
    "flac_mono_s16_44100",
    "flac_s24_stereo_48000",
    "flac_s16_stereo_44100_seektable",
];

/// Seeded mutations per class in the default gate run (measured:
/// ~13 s debug-profile for the whole suite — see the worklog entry;
/// the `#[ignore]` deep run carries the volume).
const DEFAULT_SEEDS: usize = 48;
/// Deep-run volume per class: `#[ignore]`-marked, executed on demand
/// (measured ~3 min debug-profile for the whole deep test).
const DEEP_SEEDS: usize = 512;
/// The exhaustive flip sweep drives the harness at this stride in the
/// deep run (one call is a full multi-pass over the payload).
const FLIP_STRIDE: usize = 16;

// Seed provenance (fixed, reproducible — ASCII "FLAC_TRUN",
// "FLAC_FLIP", "FLAC_SPLI", "FLAC_META"). The deep run reuses the
// same recorded seeds at 8× volume per class plus the exhaustive
// sweep — reproducibility over novelty.
const SEED_TRUNC: u64 = 0x464c_4143_5f54_5255;
const SEED_FLIP: u64 = 0x464c_4143_5f46_4c49;
const SEED_SPLICE: u64 = 0x464c_4143_5f53_504c;
const SEED_META: u64 = 0x464c_4143_5f4d_4554;

/// Every `flac.*` context code the parse/decode surfaces can produce
/// (the documented taxonomy of the module; a code outside this set in
/// a survival run is taxonomy drift and fails the suite).
const FLAC_CODES: [&str; 21] = [
    "flac.probe.magic",
    "flac.metadata.header_truncated",
    "flac.metadata.body_truncated",
    "flac.metadata.duplicate_streaminfo",
    "flac.metadata.seektable_length",
    "flac.metadata.block_type",
    "flac.metadata.streaminfo_missing",
    "flac.streaminfo.short",
    "flac.streaminfo.rate",
    "flac.streaminfo.bps",
    "flac.streaminfo.blocksize",
    "flac.frame.empty",
    "flac.frame.absent",
    "flac.frame.truncated",
    "flac.frame.crc",
    "flac.frame.structure",
    "flac.frame.feature",
    "flac.frame.channels",
    "flac.output.slot_size",
    "flac.output.fresh_slot_not_exclusive",
    "flac.streaminfo.missing",
];

/// The decoder's output width for a bit depth (the module's documented
/// left-justify mapping: ≤16 → S16, 17..=24 → S24, else S32).
fn output_width(bits_per_sample: u32) -> usize {
    match bits_per_sample {
        0..=16 => 2,
        17..=24 => 3,
        _ => 4,
    }
}

// ---- the seeded PRNG (the pcm suite's SplitMix64, verbatim) -------------------

/// SplitMix64 (public-domain reference generator): deterministic on
/// every platform, five lines, no dependencies. Fixed seeds above make
/// every mutant exactly reproducible.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> SplitMix64 {
        SplitMix64(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound` (`bound > 0`).
    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    /// One random byte.
    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

// ---- the outcome ledger (real numbers, D-11) ----------------------------------

/// Outcome ledger for one run: what every mutant did, as counts.
#[derive(Default)]
struct Ledger {
    /// Mutants run through the accounting surfaces.
    mutants: usize,
    /// Calls into the registered fuzz harness.
    harness_calls: usize,
    /// Layouts that parsed clean.
    layouts: usize,
    /// Frames decoded successfully (free surface).
    frames_ok: usize,
    /// Frame decodes that failed typed.
    frames_typed: usize,
    /// Walk-level typed rejections (parse).
    walk_typed: usize,
    /// Untouched frames byte-matched against the clean decode
    /// (property (c)).
    isolation_checks: usize,
}

impl Ledger {
    fn report(&self, label: &str) -> String {
        format!(
            "{label}: {mutants} accounted mutants, {harness} harness calls -> {layouts} \
             layouts, {ok} frames ok, {typed} frame errors (all typed), {walk} walk \
             rejections, {iso} isolation byte-matches",
            mutants = self.mutants,
            harness = self.harness_calls,
            layouts = self.layouts,
            ok = self.frames_ok,
            typed = self.frames_typed,
            walk = self.walk_typed,
            iso = self.isolation_checks
        )
    }
}

// ---- the accounting surfaces ---------------------------------------------------

/// Assert one `EngineError` from the FLAC surfaces is typed exactly:
/// stage `Decode`, a stable `flac.*` code from the recorded taxonomy,
/// kind `Corrupt` or `Unsupported`.
fn assert_typed_flac(err: &EngineError, context: &str) {
    assert_eq!(err.stage, PipelineStage::Decode, "{context}: {err}");
    assert!(
        err.context.code.starts_with("flac."),
        "{context}: stable code must be flac.*: {err}"
    );
    assert!(
        FLAC_CODES.contains(&err.context.code.as_str()),
        "{context}: unexpected code `{}`: {err}",
        err.context.code
    );
    assert!(
        matches!(
            err.kind,
            ErrorKind::Corrupt { .. } | ErrorKind::Unsupported { .. }
        ),
        "{context}: damage is Corrupt or Unsupported, never a bare Decoder fault: {err}"
    );
}

/// What one mutant did to the clean bytes — decides whether the
/// isolation oracle (property (c)) applies.
enum Damage {
    /// Exactly these byte offsets were XOR-flipped; nothing else
    /// changed (length preserved). The isolation oracle applies to
    /// every walked frame whose byte range avoids them all — *if* the
    /// flips never touched the metadata chain (the caller passes
    /// `None` for metadata-region flips, see [`Damage::NoBaseline`]).
    Flips(Vec<usize>),
    /// The mutant is a prefix of the clean stream (truncation): every
    /// walked frame that ends before the cut is untouched.
    Prefix,
    /// No clean baseline: splices (offsets shift) or metadata damage
    /// (the facts themselves may lie).
    NoBaseline,
}

/// The clean decode of one fixture: `(frames_start, per-frame
/// (offset, bytes) pairs)` — the isolation oracle, computed once per
/// fixture per run.
fn clean_frames(data: &[u8]) -> (u64, Vec<(u64, Vec<u8>)>) {
    let layout = match parse_stream(data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("clean fixture must parse: {err}"),
    };
    let walked = walk_frames(data, &layout.stream_info, layout.frames_start);
    let frames = walked
        .iter()
        .map(|frame| {
            let decoded = match decode_frame(&frame.packet, &layout.stream_info) {
                Ok(decoded) => decoded,
                Err(err) => unreachable!("clean fixture frame must decode: {err}"),
            };
            (frame.offset, decoded.samples)
        })
        .collect();
    (layout.frames_start, frames)
}

/// Run one mutant through the accounting surfaces (the harness pass is
/// driven separately by the caller). `clean` is the fixture's own
/// per-frame decode; `damage` says what changed.
fn account(
    mutant: &[u8],
    clean: &(u64, Vec<(u64, Vec<u8>)>),
    damage: &Damage,
    ledger: &mut Ledger,
) {
    ledger.mutants += 1;
    let layout = match parse_stream(mutant) {
        Ok(layout) => layout,
        Err(err) => {
            ledger.walk_typed += 1;
            assert_typed_flac(&err, "metadata walk");
            return;
        }
    };
    ledger.layouts += 1;
    assert!(
        layout.frames_start <= mutant.len() as u64,
        "frames_start must land inside the data"
    );
    let facts = &layout.stream_info;
    let walked = walk_frames(mutant, facts, layout.frames_start);
    for frame in &walked {
        // The isolation oracle: in a length-preserving flip mutant, a
        // walked frame whose whole packet range avoided every flip and
        // whose metadata chain was untouched must decode to exactly
        // the clean bytes; in a truncation, every frame that ends
        // before the cut must.
        let untouched: Option<&Vec<u8>> = match damage {
            Damage::Flips(offsets) => {
                let start = frame.offset as usize;
                let end = start + frame.packet.len();
                let clean_bytes = clean
                    .1
                    .iter()
                    .find(|(offset, _)| *offset == frame.offset)
                    .map(|(_, bytes)| bytes);
                clean_bytes.filter(|_| {
                    frame.offset >= clean.0 && offsets.iter().all(|at| *at < start || *at >= end)
                })
            }
            Damage::Prefix => {
                let end = (frame.offset + frame.packet.len() as u64) as usize;
                clean
                    .1
                    .iter()
                    .find(|(offset, _)| *offset == frame.offset)
                    .map(|(_, bytes)| bytes)
                    .filter(|_| end <= mutant.len())
            }
            Damage::NoBaseline => None,
        };
        match decode_frame(&frame.packet, facts) {
            Ok(decoded) => {
                ledger.frames_ok += 1;
                assert_eq!(
                    decoded.samples.len(),
                    decoded.block_size as usize
                        * facts.channels as usize
                        * output_width(facts.bits_per_sample),
                    "emitted bytes must be block_size x channels x output width"
                );
                if let Some(clean_bytes) = untouched {
                    ledger.isolation_checks += 1;
                    assert_eq!(
                        decoded.samples, *clean_bytes,
                        "an untouched frame must decode identically after neighbouring damage"
                    );
                }
            }
            Err(err) => {
                ledger.frames_typed += 1;
                assert_typed_flac(&err, "frame decode");
            }
        }
    }
}

// ---- the mutation engines -------------------------------------------------------

/// Truncations at stratified + random prefix lengths.
fn truncations(data: &[u8], count: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = SplitMix64::new(seed);
    let mut out = Vec::with_capacity(count + 4);
    out.push(Vec::new());
    out.push(data[..4.min(data.len())].to_vec());
    out.push(data[..data.len() / 2].to_vec());
    out.push(data[..data.len() - 1].to_vec());
    for _ in 0..count {
        let len = rng.below(data.len() + 1);
        out.push(data[..len].to_vec());
    }
    out
}

/// 1..=8 single-bit XORs at random offsets (the flip list is returned
/// alongside each mutant — the isolation oracle's input). When
/// `region` is given the flips land inside it only.
fn flips(
    data: &[u8],
    count: usize,
    seed: u64,
    region: Option<std::ops::Range<usize>>,
) -> Vec<(Vec<u8>, Vec<usize>)> {
    let mut rng = SplitMix64::new(seed);
    let (lo, hi) = match &region {
        Some(range) => (range.start.min(data.len()), range.end.min(data.len())),
        None => (0, data.len()),
    };
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let mut mutant = data.to_vec();
        let mut at = Vec::new();
        let n = 1 + rng.below(8);
        for _ in 0..n {
            let offset = lo + rng.below(hi.saturating_sub(lo).max(1));
            if offset < mutant.len() {
                mutant[offset] ^= 1 << rng.below(8);
                at.push(offset);
            }
        }
        out.push((mutant, at));
    }
    out
}

/// Random bytes spliced in at a random offset (8..=64 bytes).
fn splices(data: &[u8], count: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = SplitMix64::new(seed);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let at = rng.below(data.len() + 1);
        let len = 8 + rng.below(57);
        let mut mutant = Vec::with_capacity(data.len() + len);
        mutant.extend_from_slice(&data[..at]);
        for _ in 0..len {
            mutant.push(rng.byte());
        }
        mutant.extend_from_slice(&data[at..]);
        out.push(mutant);
    }
    out
}

// ---- the gate run ---------------------------------------------------------------

/// The full run over one fixture: all four mutation classes through
/// the accounting surfaces, the registered harness on a stride.
fn gate_fixture(name: &str, seeds: usize, harness_stride: usize, ledger: &mut Ledger) {
    let data = fixture_bytes(&format!("{name}.flac"));
    let clean = clean_frames(&data);
    let layout = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("{name}: {err}"),
    };

    for mutant in truncations(&data, seeds, SEED_TRUNC) {
        account(&mutant, &clean, &Damage::Prefix, ledger);
        fuzz_on_stride(&mutant, harness_stride, ledger);
    }
    for (mutant, at) in flips(&data, seeds, SEED_FLIP, None) {
        account(&mutant, &clean, &Damage::Flips(at), ledger);
        fuzz_on_stride(&mutant, harness_stride, ledger);
    }
    for mutant in splices(&data, seeds, SEED_SPLICE) {
        account(&mutant, &clean, &Damage::NoBaseline, ledger);
        fuzz_on_stride(&mutant, harness_stride, ledger);
    }
    // The facts-lie stratum: flips confined to the metadata chain. A
    // doctored STREAMINFO must turn frame decodes into typed errors
    // (channels/bps/blocksize lies) — never a panic.
    for (mutant, _) in flips(
        &data,
        seeds,
        SEED_META,
        Some(0..layout.frames_start as usize),
    ) {
        account(&mutant, &clean, &Damage::NoBaseline, ledger);
        fuzz_on_stride(&mutant, harness_stride, ledger);
    }
}

/// Drive the registered harness every `stride`-th mutant.
fn fuzz_on_stride(mutant: &[u8], stride: usize, ledger: &mut Ledger) {
    if stride <= 1 || ledger.mutants.is_multiple_of(stride) {
        fuzz_codec_flac(mutant);
        ledger.harness_calls += 1;
    }
}

#[test]
fn fixtures_survive_truncation_flips_splices_and_fact_lies() {
    let mut ledger = Ledger::default();
    for name in FIXTURES {
        gate_fixture(name, DEFAULT_SEEDS, 8, &mut ledger);
    }
    eprintln!("{}", ledger.report("fixtures gate run"));
    // Every class produced mutants and every one was accounted.
    assert!(ledger.mutants >= 4 * 4 * DEFAULT_SEEDS, "mutation volume");
    assert!(
        ledger.walk_typed > 0,
        "truncations must reach typed walk errors"
    );
    assert!(
        ledger.frames_typed > 0,
        "flips must reach typed frame errors"
    );
    assert!(ledger.frames_ok > 0, "untouched frames must still decode");
    assert!(
        ledger.isolation_checks > 0,
        "the isolation byte-match property must have been exercised"
    );
    assert!(
        ledger.harness_calls >= 4 * 4 * DEFAULT_SEEDS / 8,
        "harness volume"
    );
}

// The deep-run volume assertion is calibrated to the exhaustive sweep
// (every byte of the mono fixture) plus 512 seeds per class per
// fixture: 12 243 + 4 * 4 * (512 + 4) > 20 000 accounted mutants.

/// The exhaustive single-byte flip sweep over the smallest fixture
/// (every offset, MSB), plus the deep seed volume on all fixtures:
/// accounting on every mutant, the harness strided.
#[test]
#[ignore = "deep run: exhaustive flip sweep + 512 seeds per class per fixture"]
fn deep_exhaustive_flip_sweep_survives() {
    let mut ledger = Ledger::default();
    let data = fixture_bytes("flac_mono_s16_44100.flac");
    let clean = clean_frames(&data);
    for at in 0..data.len() {
        let mut mutant = data.clone();
        mutant[at] ^= 0x80;
        let damage = Damage::Flips(vec![at]);
        account(&mutant, &clean, &damage, &mut ledger);
        if at % FLIP_STRIDE == 0 {
            fuzz_codec_flac(&mutant);
            ledger.harness_calls += 1;
        }
    }
    for name in FIXTURES {
        gate_fixture(name, DEEP_SEEDS, 32, &mut ledger);
    }
    eprintln!("{}", ledger.report("deep run"));
    assert!(ledger.mutants > 20_000, "deep volume: {}", ledger.mutants);
}

/// The real corpus's dirty FLAC mutants and all garbage bins, through
/// the registered harness and the accounting surfaces (no clean
/// baseline exists for corpus files — survival + typing only). SKIPs
/// with a message when the corpus is absent.
#[test]
fn corpus_dirty_and_garbage_survive() {
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for dir in ["dirty", "garbage"] {
        let mut entries: Vec<(String, Vec<u8>)> = match std::fs::read_dir(root.join(dir)) {
            Ok(iter) => iter
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    let name = path.file_name()?.to_str()?.to_string();
                    let bytes = std::fs::read(&path).ok()?;
                    Some((name, bytes))
                })
                .collect(),
            Err(err) => unreachable!("corpus dir {dir} must read: {err}"),
        };
        entries.sort();
        for (name, bytes) in entries {
            inputs.push((format!("{dir}/{name}"), bytes));
        }
    }
    assert!(
        inputs.len() >= 20,
        "corpus-v4 carries 2 dirty FLACs + 22 garbage bins; found {}",
        inputs.len()
    );
    let mut ledger = Ledger::default();
    let no_baseline = (u64::MAX, Vec::new());
    for (name, bytes) in &inputs {
        // The registered harness on every file, dirty or garbage.
        fuzz_codec_flac(bytes);
        ledger.harness_calls += 1;
        // The accounting surfaces, typed-or-ok only.
        account(bytes, &no_baseline, &Damage::NoBaseline, &mut ledger);
        // The corpus's own FLAC-named dirty pair must be present and
        // accounted (the reference data names them explicitly).
        assert!(
            !name.ends_with(".flac")
                || name.contains("0046-audio-only")
                || name.contains("0071-cover-audio"),
            "unexpected FLAC-named dirty file: {name}"
        );
    }
    eprintln!(
        "{}",
        ledger.report(&format!("corpus dirty+garbage ({} files)", inputs.len()))
    );
    assert!(ledger.walk_typed > 0, "garbage must be typed-rejected");
}
