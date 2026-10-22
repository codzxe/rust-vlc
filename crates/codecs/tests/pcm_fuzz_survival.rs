//! Committed-corpus + seeded-mutation survival run for the
//! `fuzz_codec_pcm` harness (card 2.B5 `+FB`; ED-FIX-5 / audit M14 —
//! this file was cited by `crates/codecs/src/audio/pcm/fuzz.rs` and
//! `fuzz/fuzz_targets/fuzz_codec_pcm.rs` before it existed; it now
//! exists and delivers exactly what those citations claim).
//!
//! # What runs here
//!
//! Three surfaces per input, in this order:
//!
//! 1. **The registered fuzz harness** — [`fuzz_codec_pcm`], the very
//!    entry the cargo-fuzz target will call (its own contract is
//!    absence-of-panic/hang/unbounded-allocation, nothing else —
//!    damage is *expected* to surface as a concealed prefix or a
//!    skipped packet there).
//! 2. **The free [`decode_pcm`] entry** — layout analysis + byte-order
//!    normalization with no contract state.
//! 3. **The codec-api contract path** — `PcmDecoder`:
//!    configure → packet → sink → flush.
//!
//! # The survival properties (D-3 dirty world, D-5 adversarial)
//!
//! - **(a) No panic anywhere** — a completing test is the assertion;
//!   the suite itself matches every `Result` (no `unwrap`/`expect` on
//!   decoder output; `unreachable!` only on impossible-by-contract
//!   branches, the repo's test precedent).
//! - **(b) Every failure is typed** — the only `Err` the PCM decoder
//!   can produce is a configuration lie (zero channels/rate, malformed
//!   or lying WAVEFORMATEX, out-of-vocabulary descriptor): each is
//!   asserted to be `ErrorKind::Unsupported` at stage `Decode` with a
//!   stable `pcm.*` context code from the module's documented
//!   taxonomy. Payload damage *never* fails a call.
//! - **(c) Concealment accounting is honest** — the decoder's damage
//!   model (decoder.rs module docs) is asserted exactly, per mutant,
//!   against the injected damage: `dropped_tail_bytes == len %
//!   frame_bytes`, output == the whole-sample-frame prefix, and the
//!   outcome classification is exact (`NeedMoreInput` iff empty,
//!   `Recovered{skipped: 1, CorruptInput}` iff fewer bytes than one
//!   sample-frame, `Recovered{ConcealedFrame}` iff a non-empty prefix
//!   with a damaged tail, `Emitted{1}` iff whole). The emitted frame's
//!   duration is derived from the *decoded* instants at the configured
//!   rate, never from the damaged byte count.
//!
//! # Mutation classes (seeded, reproducible)
//!
//! Bit flips at random offsets (1..=8 single-bit XORs), truncations at
//! random lengths (stratified by seed index so every damage class is
//! guaranteed at any N), garbage splices (random bytes inserted at
//! random offsets), and format-header lies (random `PcmSource`
//! configurations including zero facts, corrupted/lying WAVEFORMATEX
//! blobs, out-of-vocabulary and wrong-layout descriptors). N per
//! class: [`DEFAULT_SEEDS`] in the gate run (measured well inside the
//! 10 s `cargo test -p codecs` budget), [`DEEP_SEEDS`] in the
//! `#[ignore]` deep run. Seeds are fixed constants recorded here —
//! any failure is exactly reproducible. The exhaustive boundary
//! sweeps (every prefix length, every single-byte flip) run the two
//! accounting surfaces on every mutant and the fuzz harness on a
//! recorded stride ([`HARNESS_STRIDE`]) — one harness call is several
//! full config-spread passes over the payload.
//!
//! # Provenance (honest, D-5)
//!
//! The valid payloads are the committed class-A parents' real audio
//! chunks (60 per container, CC0, `tests/fixtures/CORPUS.md`) —
//! generator-authored, extraction-pinned by the 2.B5 AC suite
//! (`pcm_fixtures.rs`). The adversarial value here is the mutation
//! engine, not external media; the external-oracle corpus is the
//! separate D-11 suite (`corpus.rs`, harness-gated).

mod common;

use codec_api::prelude::*;
use codecs::audio::pcm::fuzz::fuzz_codec_pcm;
use codecs::audio::pcm::{PcmDecoder, PcmFormat, PcmSource, decode_pcm};
use common::{
    extract_avi_audio_strf, extract_avi_pcm_chunks, extract_mp4_pcm_chunks, fixture_bytes,
};
use media_types::frame::AudioFrame;
use media_types::time::{Duration, Nanoseconds};
use media_types::{PipelineStage, Stamp, TrackId};

const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
const MP4: &str = "generated/valid/mp4_mjpeg_pcm.mp4";

/// One audio chunk per frame tick per parent (the 2.B5 AC's pinned
/// count).
const PARENT_CHUNKS: usize = 60;
/// The parents' true audio facts: s16le interleaved 44100 stereo.
const SOURCE: PcmSource = PcmSource {
    sample: PcmFormat::S16Le,
    planar: false,
    channels: 2,
};
const RATE: u32 = 44_100;
/// Bytes per sample-frame of the true config (2 channels × s16).
const FRAME_BYTES: usize = 4;

/// Seeded mutations per class in the default gate run (≥ 200 per the
/// audit-fix brief).
const DEFAULT_SEEDS: usize = 256;
/// Deep-run volume per class (≥ 2000): `#[ignore]`-marked, executed on
/// demand; the gate never pays for it.
const DEEP_SEEDS: usize = 2048;
/// The exhaustive sweeps run the *accounting* surfaces (free +
/// contract) on every mutant, but the fuzz harness only on this stride
/// (plus all boundary-adjacent mutants): one harness call decodes its
/// config spread — several full passes over the payload — and the
/// debug-profile gate budget is 10 s. The stride is recorded here so
/// the coverage claim stays honest: systematic, strided for the
/// harness, exhaustive for the assertions.
const HARNESS_STRIDE: usize = 32;

// Seed provenance (fixed, reproducible — ASCII "PCM_FLIP", "PCM_TRUN",
// "PCM_SPLI", "PCM_LIES", "PCM_DEEP").
const SEED_FLIP: u64 = 0x5043_4d5f_464c_4950;
const SEED_TRUNC: u64 = 0x5043_4d5f_5452_554e;
const SEED_SPLICE: u64 = 0x5043_4d5f_5350_4c49;
const SEED_LIES: u64 = 0x5043_4d5f_4c49_4553;
const SEED_DEEP: u64 = 0x5043_4d5f_4445_4550;

/// Out-of-vocabulary descriptor strings a lying container might emit
/// (dirty-world casing, wrong widths, foreign codecs, the bare `pcm`
/// fallback — D-9). All pass `CodecDescriptor::new` (non-empty, no
/// control bytes) so the lie reaches the decoder's own vocabulary
/// check.
const DESCRIPTOR_LIES: [&str; 10] = [
    "pcm",
    "pcm_f64le",
    "pcm_f64be",
    "pcm_s20le",
    "pcm_s16le_planarr",
    "PCM_S16LE",
    "s16le",
    "mjpeg",
    "adpcm",
    "xyzzy",
];

/// The typed codes a lying WAVEFORMATEX can produce (the parse
/// taxonomy of `pcm::decoder`; every code is `Unsupported` at stage
/// `Decode`).
const WAVEFORMAT_CODES: [&str; 6] = [
    "pcm.waveformat.length",
    "pcm.waveformat.channels",
    "pcm.waveformat.rate",
    "pcm.waveformat.tag",
    "pcm.waveformat.bits",
    "pcm.waveformat.block_align",
];

// ---- the seeded PRNG -------------------------------------------------------

/// SplitMix64 (public-domain reference generator): deterministic on
/// every platform, five lines, no dependencies — the seeded-mutation
/// engine. Fixed seeds above make every mutant exactly reproducible.
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

    /// A value in `0..bound` (`bound > 0`). Modulo reduction is
    /// deliberately simple: a 64-bit draw against the small bounds
    /// here makes the bias irrelevant for damage generation.
    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    /// One random byte.
    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

// ---- the outcome ledger (real numbers, D-11) ---------------------------------

/// Outcome ledger for one run: what every mutant did, as counts —
/// the numbers the deep run reports.
#[derive(Default)]
struct Ledger {
    mutants: usize,
    emitted: usize,
    concealed: usize,
    skipped: usize,
    empty: usize,
    typed_errors: usize,
    accepted_lies: usize,
}

impl Ledger {
    fn absorb(&mut self, other: &Ledger) {
        self.mutants += other.mutants;
        self.emitted += other.emitted;
        self.concealed += other.concealed;
        self.skipped += other.skipped;
        self.empty += other.empty;
        self.typed_errors += other.typed_errors;
        self.accepted_lies += other.accepted_lies;
    }

    fn report(&self, label: &str) -> String {
        format!(
            "{label}: {} mutants -> {} emitted, {} concealed, {} skipped, \
             {} empty, {} typed errors, {} accepted lies",
            self.mutants,
            self.emitted,
            self.concealed,
            self.skipped,
            self.empty,
            self.typed_errors,
            self.accepted_lies
        )
    }
}

// ---- corpus access ------------------------------------------------------------

/// The committed PCM chunks of both class-A parents (60 per container;
/// byte-identical streams, pinned by the 2.B5 AC suite) — the valid
/// payloads every mutation class derives from. Hygiene: the committed
/// chunks are whole sample-frames (352 800 = 88 200 × 4 bytes per
/// parent, the expectation's pinned stream length).
fn corpus_chunks() -> Vec<Vec<u8>> {
    let mut chunks = Vec::with_capacity(2 * PARENT_CHUNKS);
    for (fixture, extracted) in [
        (
            AVI,
            fixture_bytes(AVI).and_then(|bytes| extract_avi_pcm_chunks(&bytes)),
        ),
        (
            MP4,
            fixture_bytes(MP4).and_then(|bytes| extract_mp4_pcm_chunks(&bytes)),
        ),
    ] {
        let parent = match extracted {
            Ok(chunks) => chunks,
            Err(e) => unreachable!("{fixture}: {e}"),
        };
        assert_eq!(parent.len(), PARENT_CHUNKS, "{fixture}: 60 audio chunks");
        for (index, chunk) in parent.iter().enumerate() {
            assert!(
                !chunk.is_empty() && chunk.len().is_multiple_of(FRAME_BYTES),
                "{fixture} chunk {index}: committed chunks are whole sample-frames"
            );
        }
        chunks.extend(parent);
    }
    chunks
}

// ---- contract-path plumbing ----------------------------------------------------

/// A capturing sink: records every emitted audio frame (the decode
/// path's only emission target — the concealment-accounting oracle).
#[derive(Default)]
struct CaptureSink {
    frames: Vec<AudioFrame>,
}

impl FrameSink for CaptureSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            unreachable!("the PCM decoder emits audio frames only: {frame:?}");
        };
        self.frames.push(audio);
        Ok(())
    }
}

/// One payload packet for the contract path.
fn packet(bytes: Vec<u8>, generation: GenerationId) -> Packet {
    Packet {
        track: TrackId::new(1),
        pts: Stamp::Missing,
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(bytes),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// A decoder configured for the parents' true facts through the
/// `with_format` seam (descriptor + seeded facts, no `extra_data` —
/// the MP4-parent configuration path).
fn true_config_decoder() -> (PcmDecoder, GenerationId) {
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("generation: {e}"),
    };
    let mut decoder = match PcmDecoder::with_format(SOURCE.channels, RATE) {
        Ok(d) => d,
        Err(e) => unreachable!("true facts are valid: {e}"),
    };
    let descriptor = match CodecDescriptor::new(SOURCE.descriptor().as_str()) {
        Ok(d) => d,
        Err(e) => unreachable!("pcm_s16le is a valid descriptor: {e}"),
    };
    match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => (decoder, generation),
        Err(e) => unreachable!("true facts configure: {e}"),
    }
}

/// Assert an `EngineError` seen on a mutation run is a *typed PCM*
/// failure: stage `Decode`, a stable `pcm.*` code from the documented
/// taxonomy, and the `Unsupported` kind — a configuration lie, never a
/// panic, never media damage misreported as fatal.
fn assert_typed_pcm_error(err: &EngineError, allowed: &[&str], context: &str) {
    assert_eq!(err.stage, PipelineStage::Decode, "{context}: {err}");
    assert!(
        err.context.code.starts_with("pcm."),
        "{context}: stable code must be pcm.*: {err}"
    );
    assert!(
        allowed.contains(&err.context.code.as_str()),
        "{context}: unexpected code `{}` (allowed {allowed:?}): {err}",
        err.context.code
    );
    assert!(
        matches!(err.kind, ErrorKind::Unsupported { .. }),
        "{context}: a configuration lie is Unsupported, never Corrupt: {err}"
    );
}

/// The presentation duration of `instants` sample-frames at `rate` —
/// the decoder's documented formula, recomputed here independently
/// from the injected damage (floor division; saturating, like the
/// decoder).
fn duration_nanos(instants: u64, rate: u32) -> u64 {
    instants
        .saturating_mul(1_000_000_000)
        .checked_div(u64::from(rate))
        .unwrap_or(u64::MAX)
}

// ---- the two accounting surfaces ------------------------------------------------

/// Run one payload through the free surface with exact accounting:
/// `dropped_tail_bytes` must equal the bytes past the last whole
/// sample-frame, and the output must be exactly that prefix (a
/// width-preserving permutation — never longer, never fabricated).
fn free_surface(payload: &[u8], source: PcmSource, rate: u32, ledger: &mut Ledger) {
    ledger.mutants += 1;
    let decoded = match decode_pcm(payload, source, rate) {
        Ok(d) => d,
        Err(err) => {
            ledger.typed_errors += 1;
            assert_typed_pcm_error(
                &err,
                &["pcm.channels.zero", "pcm.rate.zero"],
                "free surface",
            );
            return;
        }
    };
    let frame_bytes = source.channels as usize * source.sample.bytes_per_sample() as usize;
    let tail = payload.len() % frame_bytes;
    assert_eq!(
        decoded.dropped_tail_bytes, tail,
        "dropped tail must count exactly the sub-frame remainder"
    );
    assert_eq!(
        decoded.samples.len(),
        payload.len() - tail,
        "output must be the whole-sample-frame prefix"
    );
    if payload.is_empty() {
        ledger.empty += 1;
    } else if payload.len() < frame_bytes {
        ledger.skipped += 1;
    } else if tail > 0 {
        ledger.concealed += 1;
    } else {
        ledger.emitted += 1;
    }
}

/// Run one payload through the contract path (a configured decoder)
/// with exact outcome classification and frame honesty. `frame_bytes`
/// and `rate` are what the *configuration in force* says — for the lie
/// classes that is the lie the decoder accepted.
fn contract_surface(
    decoder: &mut PcmDecoder,
    payload: &[u8],
    generation: GenerationId,
    frame_bytes: usize,
    rate: u32,
    ledger: &mut Ledger,
) {
    ledger.mutants += 1;
    let mut sink = CaptureSink::default();
    let outcome = match decoder.decode(packet(payload.to_vec(), generation), &mut sink) {
        Ok(outcome) => outcome,
        Err(err) => {
            ledger.typed_errors += 1;
            assert_typed_pcm_error(&err, &["pcm.not_configured"], "contract path");
            return;
        }
    };
    // The exact damage model, recomputed from the injected payload.
    let instants = payload.len() / frame_bytes;
    let tail = payload.len() % frame_bytes;
    let expected = if payload.is_empty() {
        DecodeOutcome::NeedMoreInput
    } else if instants == 0 {
        DecodeOutcome::Recovered {
            skipped: 1,
            reason: RecoverReason::CorruptInput,
        }
    } else if tail > 0 {
        DecodeOutcome::Recovered {
            skipped: 0,
            reason: RecoverReason::ConcealedFrame,
        }
    } else {
        DecodeOutcome::Emitted { frames: 1 }
    };
    assert_eq!(
        outcome, expected,
        "concealment accounting must match the injected damage exactly"
    );
    if instants == 0 {
        assert!(
            sink.frames.is_empty(),
            "nothing decodable means nothing emitted — silence is never fabricated"
        );
    } else {
        assert_eq!(sink.frames.len(), 1, "one frame per packet");
        let frame = &sink.frames[0];
        assert_eq!(
            frame.samples.len(),
            instants * frame_bytes,
            "the emitted frame is the whole-sample-frame prefix"
        );
        assert_eq!(
            frame.duration,
            Some(Duration::Known(Nanoseconds(duration_nanos(
                instants as u64,
                rate
            )))),
            "duration derives from decoded instants at the configured rate"
        );
    }
    match outcome {
        DecodeOutcome::Emitted { .. } => ledger.emitted += 1,
        DecodeOutcome::Recovered { skipped: 0, .. } => ledger.concealed += 1,
        DecodeOutcome::Recovered { .. } => ledger.skipped += 1,
        DecodeOutcome::NeedMoreInput => ledger.empty += 1,
        DecodeOutcome::Drained => unreachable!("decode never drains: {outcome:?}"),
    }
}

// ---- the mutation classes --------------------------------------------------------

/// The signature every seeded payload mutator shares (the run-class
/// loop's fn-pointer table).
type Mutator = fn(&[u8], usize, &mut SplitMix64) -> Vec<u8>;

/// Class 1 — bit flips at random offsets: 1..=8 single-bit XORs
/// anywhere in the payload. Length-preserving: sample *values* lie,
/// the frame structure cannot move.
fn mutate_bit_flips(base: &[u8], _index: usize, rng: &mut SplitMix64) -> Vec<u8> {
    let mut out = base.to_vec();
    let flips = 1 + rng.below(8);
    for _ in 0..flips {
        let at = rng.below(out.len());
        out[at] ^= 1 << rng.below(8);
    }
    out
}

/// Class 2 — truncations at random lengths, **stratified by seed
/// index** so every damage class is guaranteed at any N (with pure
/// uniform draws over a 5880-byte chunk the sub-frame cuts would be
/// ~0.07% each and coverage would be luck): every 64th seed group
/// pins the degenerate cuts 0..=3, each remainder class mod 4 gets a
/// quarter of the draws, and the lengths within a class stay random
/// (never re-entering the pinned region).
fn mutate_truncation(base: &[u8], index: usize, rng: &mut SplitMix64) -> Vec<u8> {
    let len = base.len();
    let frames = len / FRAME_BYTES;
    assert!(frames >= 2, "stratification needs two sample-frames");
    let cut = match index % 64 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        _ => match index % 4 {
            // Whole-frame cuts only: 4..=len.
            0 => FRAME_BYTES * (1 + rng.below(frames)),
            // Remainder-1/2/3 cuts: 5..=(len-1), never sub-frame.
            1 => FRAME_BYTES * (1 + rng.below(frames - 1)) + 1,
            2 => FRAME_BYTES * (1 + rng.below(frames - 1)) + 2,
            _ => FRAME_BYTES * (1 + rng.below(frames - 1)) + 3,
        },
    };
    base[..cut.min(len)].to_vec()
}

/// Class 3 — garbage splices: 1..=96 random bytes inserted at a random
/// offset (prepend and append included — the offset draw is uniform
/// over `0..=len`). The length moves, so the tail damage moves with
/// it.
fn mutate_garbage_splice(base: &[u8], _index: usize, rng: &mut SplitMix64) -> Vec<u8> {
    let at = rng.below(base.len() + 1);
    let span = 1 + rng.below(96);
    let mut out = Vec::with_capacity(base.len() + span);
    out.extend_from_slice(&base[..at]);
    for _ in 0..span {
        out.push(rng.byte());
    }
    out.extend_from_slice(&base[at..]);
    out
}

/// One seeded mutation-class pass: `seeds` mutants of random corpus
/// chunks through all three surfaces. The harness pass is pure
/// survival (its contract); the free + contract passes assert the
/// damage model exactly. Payload mutations under the true config must
/// never produce an `Err` — damage conceals, it does not fail.
fn run_mutation_class(
    label: &str,
    seed: u64,
    seeds: usize,
    mutate: Mutator,
    chunks: &[Vec<u8>],
    report: bool,
) -> Ledger {
    let mut rng = SplitMix64::new(seed);
    let mut ledger = Ledger::default();
    let (mut decoder, generation) = true_config_decoder();
    for index in 0..seeds {
        let base = &chunks[rng.below(chunks.len())];
        let mutant = mutate(base, index, &mut rng);
        // (a) survival: the registered harness, no panic anywhere.
        fuzz_codec_pcm(&mutant);
        // (b) + (c): typed errors and honest accounting.
        free_surface(&mutant, SOURCE, RATE, &mut ledger);
        contract_surface(
            &mut decoder,
            &mutant,
            generation,
            FRAME_BYTES,
            RATE,
            &mut ledger,
        );
    }
    assert_eq!(
        ledger.typed_errors, 0,
        "{label}: payload damage never fails a call — it conceals"
    );
    if report {
        println!("{}", ledger.report(label));
    }
    ledger
}

// ---- the format-header lie class --------------------------------------------------

/// A random (possibly lying) source configuration: any format, either
/// layout, 0..=8 channels (zero is the lie), and a rate pool that
/// includes zero.
fn random_source(rng: &mut SplitMix64) -> (PcmSource, u32) {
    const RATES: [u32; 8] = [0, 8_000, 11_025, 44_100, 48_000, 96_000, 192_000, 384_000];
    let source = PcmSource {
        sample: PcmFormat::ALL[rng.below(PcmFormat::ALL.len())],
        planar: rng.next_u64() & 1 == 1,
        channels: rng.below(9) as u32,
    };
    (source, RATES[rng.below(RATES.len())])
}

/// A `with_format`-seeded contract path under a lied source: zero
/// facts are typed errors at the seam; a coherent lie configures and
/// must then account honestly against the *lied* frame geometry.
fn contract_with_format_lie(payload: &[u8], source: PcmSource, rate: u32, ledger: &mut Ledger) {
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("generation: {e}"),
    };
    let mut decoder = match PcmDecoder::with_format(source.channels, rate) {
        Ok(decoder) => decoder,
        Err(err) => {
            ledger.typed_errors += 1;
            assert_typed_pcm_error(
                &err,
                &["pcm.channels.zero", "pcm.rate.zero"],
                "with_format lie",
            );
            return;
        }
    };
    let descriptor = match CodecDescriptor::new(source.descriptor().as_str()) {
        Ok(d) => d,
        Err(e) => unreachable!("in-vocabulary descriptor: {e}"),
    };
    if let Err(err) = decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        None,
    )) {
        ledger.typed_errors += 1;
        assert_typed_pcm_error(
            &err,
            &["pcm.channels.zero", "pcm.rate.zero"],
            "configure after with_format",
        );
        return;
    }
    let frame_bytes = source.channels as usize * source.sample.bytes_per_sample() as usize;
    contract_surface(&mut decoder, payload, generation, frame_bytes, rate, ledger);
}

/// A WAVEFORMATEX lie: the AVI parent's real 16-byte `strf` with
/// random bytes reassigned, or a lying length (truncation below 16 /
/// extension past it — 18 is the legal cbSize form, anything else is
/// not). A lie that is internally coherent is *accepted*: the decoder
/// then runs on the lied facts, and the accounting must be honest
/// against them.
fn waveformat_lie(payload: &[u8], strf: &[u8], rng: &mut SplitMix64, ledger: &mut Ledger) {
    let mut blob = strf.to_vec();
    if rng.below(6) == 0 {
        // Length lie.
        if rng.next_u64() & 1 == 0 {
            let cut = rng.below(blob.len());
            blob.truncate(cut);
        } else {
            let extra = 1 + rng.below(8);
            for _ in 0..extra {
                blob.push(rng.byte());
            }
        }
    } else {
        // Byte lie: 1..=4 offsets reassigned.
        let lies = 1 + rng.below(4);
        for _ in 0..lies {
            let at = rng.below(blob.len());
            blob[at] = rng.byte();
        }
    }
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("generation: {e}"),
    };
    let descriptor = match CodecDescriptor::new("pcm_s16le") {
        Ok(d) => d,
        Err(e) => unreachable!("pcm_s16le: {e}"),
    };
    let mut decoder = PcmDecoder::new();
    if let Err(err) = decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(blob.clone())),
    )) {
        ledger.typed_errors += 1;
        assert_typed_pcm_error(&err, &WAVEFORMAT_CODES, "waveformat lie");
        return;
    }
    // Accepted lie: the decoder now runs on the blob's own facts
    // (channels at [2..4], rate at [4..8] — valid because configure
    // succeeded; the descriptor pins s16, so 2 bytes per sample).
    ledger.accepted_lies += 1;
    let channels = u16::from_le_bytes([blob[2], blob[3]]) as u32;
    let rate = u32::from_le_bytes([blob[4], blob[5], blob[6], blob[7]]);
    contract_surface(
        &mut decoder,
        payload,
        generation,
        channels as usize * 2,
        rate,
        ledger,
    );
}

/// A descriptor lie, **stratified by seed index** so every outcome of
/// the descriptor-vs-facts negotiation is guaranteed at any N:
/// stratum 0 lies with s16 vocabulary (accepted — the strf's width),
/// stratum 1 with pure garbage from [`DESCRIPTOR_LIES`], stratum 2
/// with an integer width the strf contradicts, stratum 3 with a float
/// width the strf's integer tag contradicts. Each accepted descriptor
/// carries a random planar suffix — a *layout* lie against the
/// interleaved truth.
fn descriptor_lie(
    payload: &[u8],
    strf: &[u8],
    index: usize,
    rng: &mut SplitMix64,
    ledger: &mut Ledger,
) {
    let owned;
    let descriptor_string = match index % 4 {
        0 => {
            // s16 vocabulary against an s16 strf: accepted.
            let pick = rng.below(2);
            owned = format!(
                "{}{}",
                [PcmFormat::S16Le, PcmFormat::S16Be][pick].descriptor(),
                if rng.next_u64() & 1 == 1 {
                    "_planar"
                } else {
                    ""
                }
            );
            owned.as_str()
        }
        1 => DESCRIPTOR_LIES[rng.below(DESCRIPTOR_LIES.len())],
        2 => {
            // Integer widths ≠ 16: the bits cross-check fires.
            let pick = rng.below(3);
            owned = [PcmFormat::U8, PcmFormat::S24Le, PcmFormat::S32Be][pick]
                .descriptor()
                .to_string();
            owned.as_str()
        }
        _ => {
            // Float widths: the tag cross-check fires first.
            let pick = rng.below(2);
            owned = [PcmFormat::F32Le, PcmFormat::F32Be][pick]
                .descriptor()
                .to_string();
            owned.as_str()
        }
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("generation: {e}"),
    };
    let descriptor = match CodecDescriptor::new(descriptor_string) {
        Ok(d) => d,
        Err(e) => unreachable!("non-empty ASCII descriptor: {e}"),
    };
    let mut decoder = PcmDecoder::new();
    if let Err(err) = decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(strf.to_vec())),
    )) {
        ledger.typed_errors += 1;
        // In-vocabulary wrong-width descriptors fail a WAVEFORMATEX
        // cross-check (bits for integers, tag for floats — the parse
        // order); garbage fails the vocabulary itself.
        assert_typed_pcm_error(
            &err,
            &[
                "pcm.codec_descriptor",
                "pcm.waveformat.tag",
                "pcm.waveformat.bits",
            ],
            "descriptor lie",
        );
        return;
    }
    // Accepted: an s16 descriptor (± planar — same frame geometry)
    // against the strf's true facts.
    ledger.accepted_lies += 1;
    contract_surface(&mut decoder, payload, generation, FRAME_BYTES, RATE, ledger);
}

/// A self-consistent 16-byte PCMWAVEFORMAT declaring `(channels,
/// rate)` — the lying encoder that gets all its cross-checks right and
/// still disagrees with the truth.
fn self_consistent_waveformat(channels: u16, rate: u32) -> Vec<u8> {
    let mut blob = Vec::with_capacity(16);
    blob.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    blob.extend_from_slice(&channels.to_le_bytes());
    blob.extend_from_slice(&rate.to_le_bytes());
    blob.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    blob.extend_from_slice(&(channels * 2).to_le_bytes()); // block align
    blob.extend_from_slice(&16u16.to_le_bytes()); // bits
    blob
}

/// Configure a `with_format(seeded)` decoder with `blob` as
/// `extra_data` and assert the exact typed outcome: `Some(code)` must
/// fail with that code, `None` must configure.
fn expect_configure_code(seeded: (u32, u32), blob: &[u8], expected: Option<&str>, context: &str) {
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{context}: generation: {e}"),
    };
    let mut decoder = match PcmDecoder::with_format(seeded.0, seeded.1) {
        Ok(d) => d,
        Err(e) => unreachable!("{context}: seeded facts are valid: {e}"),
    };
    let descriptor = match CodecDescriptor::new("pcm_s16le") {
        Ok(d) => d,
        Err(e) => unreachable!("{context}: {e}"),
    };
    let result = decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(blob.to_vec())),
    ));
    match (result, expected) {
        (Err(err), Some(code)) => assert_typed_pcm_error(&err, &[code], context),
        (Ok(()), None) => {}
        (Err(err), None) => unreachable!("{context}: must configure: {err}"),
        (Ok(()), Some(code)) => unreachable!("{context}: must fail `{code}`, configured anyway"),
    }
}

// ---- the committed-corpus survival tests --------------------------------------------

/// The citation's core claim, literally: both class-A parents' *full
/// audio tracks* (each 60 chunks = 352 800 bytes = 2.000 s of audio)
/// through all three surfaces, whole and per chunk. A whole track in
/// one packet is the largest legal input this decoder can see.
#[test]
fn both_parents_full_audio_tracks_survive_and_account_exactly() {
    for (fixture, extracted) in [
        (
            AVI,
            fixture_bytes(AVI).and_then(|bytes| extract_avi_pcm_chunks(&bytes)),
        ),
        (
            MP4,
            fixture_bytes(MP4).and_then(|bytes| extract_mp4_pcm_chunks(&bytes)),
        ),
    ] {
        let chunks = match extracted {
            Ok(c) => c,
            Err(e) => unreachable!("{fixture}: {e}"),
        };
        assert_eq!(chunks.len(), PARENT_CHUNKS, "{fixture}");
        let full: Vec<u8> = chunks.concat();
        let full_len = full.len();

        let mut ledger = Ledger::default();
        // (a) the harness on the whole track and every chunk.
        fuzz_codec_pcm(&full);
        for chunk in &chunks {
            fuzz_codec_pcm(chunk);
        }
        // (b) + (c) the whole track through both accounting surfaces.
        free_surface(&full, SOURCE, RATE, &mut ledger);
        let (mut decoder, generation) = true_config_decoder();
        contract_surface(
            &mut decoder,
            &full,
            generation,
            FRAME_BYTES,
            RATE,
            &mut ledger,
        );
        assert_eq!(
            (
                ledger.emitted,
                ledger.concealed,
                ledger.skipped,
                ledger.empty,
                ledger.typed_errors
            ),
            (2, 0, 0, 0, 0),
            "{fixture}: a whole clean track is one honest Emitted frame per surface"
        );

        // Duration honesty, pinned directly on the emitted frame:
        // 88 200 stereo instants at 44 100 Hz = exactly 2.000 s.
        let mut sink = CaptureSink::default();
        let outcome = match decoder.decode(packet(full, generation), &mut sink) {
            Ok(outcome) => outcome,
            Err(e) => unreachable!("{fixture}: {e}"),
        };
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 }, "{fixture}");
        assert_eq!(sink.frames.len(), 1, "{fixture}");
        assert_eq!(
            sink.frames[0].duration,
            Some(Duration::Known(Nanoseconds(2_000_000_000))),
            "{fixture}: 88200 instants at 44100 Hz"
        );
        assert_eq!(sink.frames[0].samples.len(), full_len, "{fixture}");
    }
}

/// Systematic truncations of a real chunk at *every* prefix length
/// (0..=5880): every sample-frame boundary and every sub-sample
/// boundary, exhaustively through both accounting surfaces, with the
/// harness on a recorded stride (see [`HARNESS_STRIDE`]).
#[test]
fn systematic_prefix_truncations_of_a_real_chunk_survive_exactly() {
    let chunks = corpus_chunks();
    let chunk = &chunks[0];
    let len = chunk.len();
    let frames = len / FRAME_BYTES;
    let (mut decoder, generation) = true_config_decoder();
    let mut ledger = Ledger::default();
    for cut in 0..=len {
        let prefix = &chunk[..cut];
        let boundary = cut < 2 * FRAME_BYTES || cut > len - 2 * FRAME_BYTES;
        if cut.is_multiple_of(HARNESS_STRIDE) || boundary {
            fuzz_codec_pcm(prefix);
        }
        free_surface(prefix, SOURCE, RATE, &mut ledger);
        contract_surface(
            &mut decoder,
            prefix,
            generation,
            FRAME_BYTES,
            RATE,
            &mut ledger,
        );
    }
    assert_eq!(ledger.mutants, 2 * (len + 1), "two surfaces per prefix");
    assert_eq!(
        ledger.typed_errors, 0,
        "truncation damage never fails a call"
    );
    // Every damage class exercised, exhaustively and exactly counted.
    assert_eq!(ledger.empty, 2, "exactly one empty prefix, on each surface");
    assert_eq!(
        ledger.skipped,
        2 * (FRAME_BYTES - 1),
        "the sub-frame prefixes"
    );
    assert_eq!(
        ledger.concealed,
        2 * 3 * (frames - 1),
        "every non-empty prefix with a damaged tail, on each surface"
    );
    assert_eq!(
        ledger.emitted,
        2 * frames,
        "every whole-frame prefix, on each surface"
    );
}

/// Systematic single-byte flips of a real chunk: the MSB of *every*
/// byte, one mutant per offset (all sample and sub-sample boundary
/// positions included), through both accounting surfaces, with the
/// harness on a recorded stride (see [`HARNESS_STRIDE`]). A flip
/// preserves length and the chunk is whole-frame — so every mutant
/// must report a clean `Emitted` with a zero dropped tail: changed
/// sample *values* are not damage, and the decoder must never conceal
/// them.
#[test]
fn systematic_single_byte_flips_of_a_real_chunk_survive_exactly() {
    let chunks = corpus_chunks();
    let mut working = chunks[0].clone();
    let len = working.len();
    let (mut decoder, generation) = true_config_decoder();
    let mut ledger = Ledger::default();
    for at in 0..len {
        working[at] ^= 0x80;
        let boundary = at < FRAME_BYTES || at >= len - FRAME_BYTES;
        if at.is_multiple_of(HARNESS_STRIDE) || boundary {
            fuzz_codec_pcm(&working);
        }
        free_surface(&working, SOURCE, RATE, &mut ledger);
        contract_surface(
            &mut decoder,
            &working,
            generation,
            FRAME_BYTES,
            RATE,
            &mut ledger,
        );
        working[at] ^= 0x80;
    }
    assert_eq!(ledger.mutants, 2 * len);
    assert_eq!(
        (
            ledger.emitted,
            ledger.concealed,
            ledger.skipped,
            ledger.empty,
            ledger.typed_errors
        ),
        (2 * len, 0, 0, 0, 0),
        "bit flips are value lies, not structural damage"
    );
}

/// Every real chunk of both parents through boundary cuts (the first
/// and last sample-frame boundaries and every sub-sample offset around
/// them) and boundary flips (the first and last 4 bytes). The harness
/// rides the cuts of every fourth chunk; the accounting surfaces run
/// on every mutant.
#[test]
fn every_real_chunk_survives_boundary_cuts_and_flips() {
    let chunks = corpus_chunks();
    let (mut decoder, generation) = true_config_decoder();
    let mut ledger = Ledger::default();
    for (index, chunk) in chunks.iter().enumerate() {
        let len = chunk.len();
        for cut in [0, 1, 2, 3, 4, 8, len - 4, len - 3, len - 2, len - 1, len] {
            let prefix = &chunk[..cut];
            if index.is_multiple_of(4) {
                fuzz_codec_pcm(prefix);
            }
            free_surface(prefix, SOURCE, RATE, &mut ledger);
            contract_surface(
                &mut decoder,
                prefix,
                generation,
                FRAME_BYTES,
                RATE,
                &mut ledger,
            );
        }
        let mut mutant = chunk.clone();
        for at in [0, 1, 2, 3, len - 4, len - 3, len - 2, len - 1] {
            mutant[at] ^= 0x80;
            free_surface(&mutant, SOURCE, RATE, &mut ledger);
            contract_surface(
                &mut decoder,
                &mutant,
                generation,
                FRAME_BYTES,
                RATE,
                &mut ledger,
            );
            mutant[at] ^= 0x80;
        }
    }
    assert_eq!(ledger.typed_errors, 0);
}

// ---- the seeded mutation classes ------------------------------------------------------

#[test]
fn seeded_bit_flips_survive_with_honest_accounting() {
    let chunks = corpus_chunks();
    let ledger = run_mutation_class(
        "bit flips",
        SEED_FLIP,
        DEFAULT_SEEDS,
        mutate_bit_flips,
        &chunks,
        false,
    );
    // Flips preserve length and every committed chunk is whole-frame:
    // every outcome must be a clean emission.
    assert_eq!(
        (ledger.concealed, ledger.skipped, ledger.empty),
        (0, 0, 0),
        "flipped sample values are never concealed"
    );
    assert_eq!(ledger.emitted, 2 * DEFAULT_SEEDS);
}

#[test]
fn seeded_truncations_survive_with_honest_accounting() {
    let chunks = corpus_chunks();
    let ledger = run_mutation_class(
        "truncations",
        SEED_TRUNC,
        DEFAULT_SEEDS,
        mutate_truncation,
        &chunks,
        false,
    );
    // The stratification guarantees every damage class occurs, and
    // the pinned cuts are exactly countable: every 64th seed group
    // contributes one empty and three sub-frame cuts.
    let pins = DEFAULT_SEEDS.div_ceil(64);
    assert_eq!(ledger.empty, 2 * pins, "every 64th seed pins the empty cut");
    assert_eq!(ledger.skipped, 2 * 3 * pins, "the pinned sub-frame cuts");
    assert_eq!(
        ledger.emitted,
        2 * (DEFAULT_SEEDS / 4 - pins),
        "whole-frame draws"
    );
    assert_eq!(
        ledger.concealed,
        2 * 3 * (DEFAULT_SEEDS / 4 - pins),
        "damaged-tail draws"
    );
}

#[test]
fn seeded_garbage_splices_survive_with_honest_accounting() {
    let chunks = corpus_chunks();
    let ledger = run_mutation_class(
        "garbage splices",
        SEED_SPLICE,
        DEFAULT_SEEDS,
        mutate_garbage_splice,
        &chunks,
        false,
    );
    // Insertion only ever grows a whole-frame base: the empty and
    // skipped classes cannot occur (spans 1..=96 vs 5880-byte chunks).
    assert_eq!((ledger.empty, ledger.skipped), (0, 0));
    assert!(
        ledger.emitted >= 2 && ledger.concealed >= 2,
        "{}",
        ledger.report("garbage splices")
    );
}

/// The format-header lie class: random source lies (free surface +
/// `with_format` seam), corrupted/lying WAVEFORMATEX blobs, and
/// descriptor lies — every failure typed, every accepted lie honestly
/// accounted against the lie the decoder believed.
#[test]
fn seeded_format_header_lies_are_typed_or_honestly_decoded() {
    let chunks = corpus_chunks();
    let strf = match fixture_bytes(AVI).and_then(|bytes| extract_avi_audio_strf(&bytes)) {
        Ok(strf) => strf,
        Err(e) => unreachable!("{AVI}: {e}"),
    };
    assert_eq!(strf.len(), 16, "the writer's PCMWAVEFORMAT");
    let mut rng = SplitMix64::new(SEED_LIES);
    let mut ledger = Ledger::default();
    for index in 0..DEFAULT_SEEDS {
        let base = &chunks[rng.below(chunks.len())];
        let (source, rate) = random_source(&mut rng);
        free_surface(base, source, rate, &mut ledger);
        contract_with_format_lie(base, source, rate, &mut ledger);
        waveformat_lie(base, &strf, &mut rng, &mut ledger);
        descriptor_lie(base, &strf, index, &mut rng, &mut ledger);
    }
    // Coverage plumbing (not a damage claim): the stratifications
    // guarantee both directions of the lie class are populated — the
    // descriptor stratum 0 alone accepts N/4 lies.
    assert!(
        ledger.typed_errors >= DEFAULT_SEEDS,
        "{}",
        ledger.report("format lies")
    );
    assert!(
        ledger.accepted_lies >= DEFAULT_SEEDS / 4,
        "{}",
        ledger.report("format lies")
    );

    // Deterministic edge permutations: every typed code pinned exactly
    // (one lie per field, D-3's checklist). `None` = must configure.
    let mut short = strf.clone();
    short.truncate(15);
    let mut nineteen = strf.clone();
    nineteen.extend_from_slice(&[0u8; 3]);
    let mut eighteen = strf.clone();
    eighteen.extend_from_slice(&[0x7f, 0x7f]); // legal cbSize form
    let mut tag = strf.clone();
    tag[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes()); // extensible tag
    let mut zero_channels = strf.clone();
    zero_channels[2..4].copy_from_slice(&0u16.to_le_bytes());
    let mut zero_rate = strf.clone();
    zero_rate[4..8].copy_from_slice(&0u32.to_le_bytes());
    let mut bits = strf.clone();
    bits[14..16].copy_from_slice(&24u16.to_le_bytes()); // width lie
    let mut align = strf.clone();
    align[12..14].copy_from_slice(&8u16.to_le_bytes()); // align lie
    let cases: [(&[u8], Option<&str>); 8] = [
        (&tag, Some("pcm.waveformat.tag")),
        (&zero_channels, Some("pcm.waveformat.channels")),
        (&zero_rate, Some("pcm.waveformat.rate")),
        (&bits, Some("pcm.waveformat.bits")),
        (&align, Some("pcm.waveformat.block_align")),
        (&short, Some("pcm.waveformat.length")),
        (&nineteen, Some("pcm.waveformat.length")),
        (&eighteen, None), // the accepted 18-byte form
    ];
    for (blob, code) in cases {
        expect_configure_code((2, RATE), blob, code, "edge lie");
    }

    // Coherent-but-conflicting facts: a WAVEFORMATEX that passes every
    // internal cross-check yet disagrees with the seeded truth — a
    // typed conflict, never a silent pick.
    for lied_channels in [1u16, 3, 4, 6, 8] {
        expect_configure_code(
            (2, RATE),
            &self_consistent_waveformat(lied_channels, RATE),
            Some("pcm.stream_format_conflict"),
            "channel conflict",
        );
    }
    for lied_rate in [8_000u32, 48_000, 96_000] {
        expect_configure_code(
            (2, RATE),
            &self_consistent_waveformat(2, lied_rate),
            Some("pcm.stream_format_conflict"),
            "rate conflict",
        );
    }
    // And agreement is not a conflict.
    expect_configure_code(
        (2, RATE),
        &self_consistent_waveformat(2, RATE),
        None,
        "agreeing facts",
    );
}

// ---- the deep run -----------------------------------------------------------------------

/// The deep seeded-mutation run: [`DEEP_SEEDS`] mutants per payload
/// class (fresh seeds) plus the lie class at the same volume. Executed
/// on demand (`cargo test -p codecs --test pcm_fuzz_survival -- --
/// --ignored --nocapture`); the numbers it prints are the report's.
#[test]
#[ignore = "deep run: N=2048 per class, executed on demand for the audit report"]
fn deep_seeded_mutation_run() {
    let chunks = corpus_chunks();
    let strf = match fixture_bytes(AVI).and_then(|bytes| extract_avi_audio_strf(&bytes)) {
        Ok(strf) => strf,
        Err(e) => unreachable!("{AVI}: {e}"),
    };
    let mut total = Ledger::default();

    let payload_classes: [(&str, u64, Mutator); 3] = [
        ("deep bit flips", SEED_DEEP ^ 1, mutate_bit_flips),
        ("deep truncations", SEED_DEEP ^ 2, mutate_truncation),
        ("deep garbage splices", SEED_DEEP ^ 3, mutate_garbage_splice),
    ];
    for (label, seed, mutate) in payload_classes {
        let ledger = run_mutation_class(label, seed, DEEP_SEEDS, mutate, &chunks, true);
        total.absorb(&ledger);
    }

    // The lie class at deep volume.
    let mut rng = SplitMix64::new(SEED_DEEP ^ 4);
    let mut lies = Ledger::default();
    for index in 0..DEEP_SEEDS {
        let base = &chunks[rng.below(chunks.len())];
        let (source, rate) = random_source(&mut rng);
        free_surface(base, source, rate, &mut lies);
        contract_with_format_lie(base, source, rate, &mut lies);
        waveformat_lie(base, &strf, &mut rng, &mut lies);
        descriptor_lie(base, &strf, index, &mut rng, &mut lies);
    }
    println!("{}", lies.report("deep format lies"));
    total.absorb(&lies);
    println!("{}", total.report("DEEP TOTAL"));
}
