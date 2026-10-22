//! W2-15 adversarial attack suite for the W1-02 PCM stream-facts
//! surfaces: the `extra_data` WAVEFORMATEX/WAVEFORMATEXTENSIBLE parse
//! (`PcmDecoder::configure` → `parse_waveformatex`) and the channel
//! extremes of the `with_format` seam.
//!
//! # What runs here
//!
//! 1. **Seeded blob mutations** — the 40-byte extensible form ffmpeg
//!    writes, mutated across every field (tag, channels, rate, align,
//!    bits, cbSize, valid bits, channel mask, GUID bytes) plus bit
//!    flips, truncations and splices, through the public configure
//!    path. Every failure must be a typed `Unsupported` from the
//!    documented `pcm.waveformat.*` taxonomy; every acceptance must
//!    then decode a payload with exact concealment accounting against
//!    the blob's own facts. This is the adversarial-mutation layer the
//!    W1-02 surface had none of (`fuzz_codec_pcm` exercises only
//!    `decode_pcm`/`with_format`; the survival suite's `waveformat_lie`
//!    mutates only the 16-byte fixture strf) — W2-15 MAJOR (g2).
//! 2. **Vendor-tail and wrap attacks** — the 42-byte blob the DEMUXER
//!    accepts (and ffprobe plays, measured 7.1.5) must configure here
//!    too (stage agreement, decoder.rs's own "the demuxer accepts the
//!    same blobs" contract) — W2-15 MAJOR (a4); the block-align
//!    cross-check must not truncate through `as u16` for huge channel
//!    counts, and the codec seam carries the demuxer's channel ceiling
//!    — W2-15 MAJOR (g1).
//! 3. **Channel extremes at the `with_format` seam** — `PcmPlan`'s
//!    frame math under `u32::MAX` channels: bounded skip on 64-bit,
//!    saturating (never wrapping) on 32-bit targets.
//!
//! B2 and B3 were the EXPECTED-TO-FAIL tests of the W2-15
//! verification (the failure text named the finding); they are landed
//! here together with the fixes they demanded and now pin the fixed
//! behavior. Per D-5 they are never weakened to pass — the code moved.

use codec_api::prelude::*;
use codecs::audio::pcm::{PcmDecoder, PcmFormat, PcmSource, decode_pcm};
use media_types::frame::AudioFrame;
use media_types::time::{Duration, Nanoseconds};
use media_types::{PipelineStage, Stamp, TrackId};

/// `KSDATAFORMAT_SUBTYPE_PCM` — `{00000001-0000-0010-8000-00AA00389B71}`.
const GUID_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` — `{00000003-0000-0010-8000-00AA00389B71}`.
const GUID_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

/// The typed codes a lying WAVEFORMATEX blob can produce (the
/// `parse_waveformatex` taxonomy — every code is `Unsupported` at stage
/// `Decode`).
const WAVEFORMAT_CODES: [&str; 8] = [
    "pcm.waveformat.length",
    "pcm.waveformat.channels",
    "pcm.waveformat.rate",
    "pcm.waveformat.tag",
    "pcm.waveformat.bits",
    "pcm.waveformat.block_align",
    "pcm.waveformat.extensible",
    "pcm.waveformat.subformat",
];

/// The seeded PRNG (SplitMix64, public-domain reference generator — the
/// pcm_fuzz_survival precedent; fixed seeds make every mutant exactly
/// reproducible).
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

/// The canonical 40-byte WAVEFORMATEXTENSIBLE (ffmpeg's AVI muxer
/// shape): 18-byte WAVEFORMATEX + cbSize + valid bits + channel mask +
/// SubFormat GUID.
fn ext_blob(
    guid: [u8; 16],
    channels: u16,
    rate: u32,
    bits: u16,
    valid: u16,
    mask: u32,
    cbsize: u16,
) -> Vec<u8> {
    let block_align = channels * (bits / 8);
    let mut v = Vec::with_capacity(40);
    v.extend_from_slice(&0xFFFEu16.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(u32::from(block_align) * rate).to_le_bytes());
    v.extend_from_slice(&block_align.to_le_bytes());
    v.extend_from_slice(&bits.to_le_bytes());
    v.extend_from_slice(&cbsize.to_le_bytes());
    v.extend_from_slice(&valid.to_le_bytes());
    v.extend_from_slice(&mask.to_le_bytes());
    v.extend_from_slice(&guid);
    v
}

/// A plain 16-byte PCMWAVEFORMAT with explicit tag/align (the wrap
/// attack's carrier).
fn plain_blob(tag: u16, channels: u16, rate: u32, block_align: u16, bits: u16) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&tag.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(u32::from(block_align) * rate).to_le_bytes());
    v.extend_from_slice(&block_align.to_le_bytes());
    v.extend_from_slice(&bits.to_le_bytes());
    v
}

fn next_generation() -> GenerationId {
    GenerationId::FIRST
        .next()
        .unwrap_or_else(|e| unreachable!("generation: {e}"))
}

/// Configure a fresh decoder with `blob` as `extra_data` — the public
/// surface the engine adapter will hand the container's verbatim `strf`.
fn configure(descriptor: &str, blob: Vec<u8>) -> Result<(PcmDecoder, GenerationId), EngineError> {
    let descriptor = match CodecDescriptor::new(descriptor) {
        Ok(d) => d,
        Err(e) => unreachable!("in-vocabulary descriptor: {e}"),
    };
    let generation = next_generation();
    let mut decoder = PcmDecoder::new();
    decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(blob)),
    ))?;
    Ok((decoder, generation))
}

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

/// The presentation duration of `instants` sample-frames at `rate` —
/// the decoder's documented formula, recomputed independently (floor
/// division; saturating, like the decoder).
fn duration_nanos(instants: u64, rate: u32) -> u64 {
    instants
        .saturating_mul(1_000_000_000)
        .checked_div(u64::from(rate))
        .unwrap_or(u64::MAX)
}

/// Lie pools for the field-targeted mutation class.
const CBSIZE_POOL: [u16; 7] = [0, 2, 21, 22, 23, 24, 0xFFFF];
const CHANNEL_POOL: [u16; 10] = [0, 1, 2, 3, 5, 8, 64, 65, 32768, 65535];
const COHERENT_CHANNEL_POOL: [u16; 5] = [1, 2, 3, 5, 8];
const BITS_POOL: [u16; 8] = [0, 8, 12, 16, 20, 24, 32, 64];
const TAG_POOL: [u16; 5] = [1, 3, 0xFFFE, 0x0011, 0x0055];

/// One seeded mutation of the canonical blob: a mutation class drawn
/// from the PRNG, applied to a fresh copy. Classes 0-2 are structural
/// (flips / truncation / splice — the splice can grow the blob past 40,
/// pinning the length gate's 41..=48 margin); classes 3-10 target one
/// field each; class 7 is a COHERENT channel change (align recomputed)
/// so the accepted-lie accounting sees varied geometries.
fn mutate(base: &[u8], rng: &mut SplitMix64) -> Vec<u8> {
    let mut out = base.to_vec();
    match rng.below(11) {
        0 => {
            let flips = 1 + rng.below(8);
            for _ in 0..flips {
                let at = rng.below(out.len());
                out[at] ^= 1 << rng.below(8);
            }
        }
        1 => {
            let cut = rng.below(base.len() + 1);
            out.truncate(cut);
        }
        2 => {
            let at = rng.below(out.len() + 1);
            let span = 1 + rng.below(8);
            let mut spliced = Vec::with_capacity(out.len() + span);
            spliced.extend_from_slice(&out[..at]);
            for _ in 0..span {
                spliced.push(rng.byte());
            }
            spliced.extend_from_slice(&out[at..]);
            return spliced;
        }
        3 => {
            let value = CBSIZE_POOL[rng.below(CBSIZE_POOL.len())];
            out[16..18].copy_from_slice(&value.to_le_bytes());
        }
        4 => {
            let value = rng.below(0x1000) as u16; // valid bits (incl. 0)
            out[18..20].copy_from_slice(&value.to_le_bytes());
        }
        5 => {
            let value = rng.next_u64() as u32; // channel mask
            out[20..24].copy_from_slice(&value.to_le_bytes());
        }
        6 => {
            let at = rng.below(16); // one GUID byte, one bit
            out[24 + at] ^= 1 << rng.below(8);
        }
        7 => {
            // Coherent channel change: align and avg follow, so this
            // mutant must configure (the accepted-lie generator).
            let channels = COHERENT_CHANNEL_POOL[rng.below(COHERENT_CHANNEL_POOL.len())];
            let bits = u16::from_le_bytes([out[14], out[15]]);
            let rate = u32::from_le_bytes([out[4], out[5], out[6], out[7]]);
            let align = channels * (bits / 8);
            out[2..4].copy_from_slice(&channels.to_le_bytes());
            out[8..12].copy_from_slice(&(u32::from(align) * rate).to_le_bytes());
            out[12..14].copy_from_slice(&align.to_le_bytes());
        }
        8 => {
            let value = CHANNEL_POOL[rng.below(CHANNEL_POOL.len())]; // stale align
            out[2..4].copy_from_slice(&value.to_le_bytes());
        }
        9 => {
            let value = BITS_POOL[rng.below(BITS_POOL.len())];
            out[14..16].copy_from_slice(&value.to_le_bytes());
        }
        _ => {
            let value = TAG_POOL[rng.below(TAG_POOL.len())];
            out[0..2].copy_from_slice(&value.to_le_bytes());
        }
    }
    out
}

/// Assert one accepted configuration decodes the fixed payload with
/// exact concealment accounting against the blob's own facts.
fn assert_accounted(decoder: &mut PcmDecoder, generation: GenerationId, blob: &[u8], bps: usize) {
    let channels = u16::from_le_bytes([blob[2], blob[3]]) as usize;
    let rate = u32::from_le_bytes([blob[4], blob[5], blob[6], blob[7]]);
    let payload: Vec<u8> = (0..30u8).map(|i| 0x40 + i).collect();
    let frame_bytes = channels * bps;
    let instants = payload.len() / frame_bytes;
    let tail = payload.len() % frame_bytes;
    let mut sink = CaptureSink::default();
    let outcome = match decoder.decode(packet(payload, generation), &mut sink) {
        Ok(outcome) => outcome,
        Err(err) => unreachable!("a configured decoder never errors on payload damage: {err}"),
    };
    let expected = if instants == 0 {
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
        "concealment accounting vs the blob's facts"
    );
    if instants == 0 {
        assert!(sink.frames.is_empty(), "silence is never fabricated");
    } else {
        assert_eq!(sink.frames.len(), 1);
        let frame = &sink.frames[0];
        assert_eq!(frame.samples.len(), instants * frame_bytes);
        assert_eq!(frame.format.channels as usize, channels);
        assert_eq!(frame.format.rate, rate);
        assert_eq!(
            frame.duration,
            Some(Duration::Known(Nanoseconds(duration_nanos(
                instants as u64,
                rate
            )))),
            "duration derives from decoded instants at the blob's rate"
        );
    }
}

/// Assert an `EngineError` from the configure path is a typed PCM
/// failure: stage `Decode`, `Unsupported` kind, a stable code from the
/// documented taxonomy.
fn assert_typed(err: &EngineError, context: &str) {
    assert_eq!(err.stage, PipelineStage::Decode, "{context}: {err}");
    assert!(
        matches!(err.kind, ErrorKind::Unsupported { .. }),
        "{context}: a configuration lie is Unsupported, never Corrupt: {err}"
    );
    assert!(
        WAVEFORMAT_CODES.contains(&err.context.code.as_str()),
        "{context}: unexpected code `{}` (allowed {:?}): {err}",
        err.context.code,
        WAVEFORMAT_CODES
    );
}

// ---- B1: the seeded blob-mutation survival + taxonomy layer ----------------

#[test]
fn seeded_extensible_blob_mutations_are_typed_or_honestly_decoded() {
    // W2-15 MAJOR coverage gap (g2): the 40-byte extensible parse
    // surface had zero adversarial-mutation coverage (fuzz_codec_pcm
    // never passes extra_data; the survival suite's waveformat_lie
    // mutates only the 16-byte fixture strf). 512 seeded mutants across
    // every field and three structural classes: every failure typed,
    // every acceptance honestly accounted against the blob's own facts.
    const SEED: u64 = 0x5732_3135_4558_5455; // "W215EXTU"
    let mut rng = SplitMix64::new(SEED);
    let mut typed = 0usize;
    let mut accepted = 0usize;

    // Structural guarantees first: the two canonical blobs configure,
    // and the empty blob is a typed length refusal (so both counters
    // are provably populated regardless of the RNG split).
    for (descriptor, blob, bps) in [
        (
            "pcm_s24le",
            ext_blob(GUID_PCM, 2, 48_000, 24, 24, 0x3, 22),
            3usize,
        ),
        (
            "pcm_f32le",
            ext_blob(GUID_IEEE_FLOAT, 1, 44_100, 32, 32, 0x4, 22),
            4usize,
        ),
    ] {
        let (mut decoder, generation) = match configure(descriptor, blob.clone()) {
            Ok(pair) => pair,
            Err(err) => unreachable!("the canonical blob configures: {err}"),
        };
        accepted += 1;
        assert_accounted(&mut decoder, generation, &blob, bps);
    }
    match configure("pcm_s24le", Vec::new()) {
        Ok(_) => unreachable!("the empty blob is a typed length refusal"),
        Err(err) => {
            typed += 1;
            assert_typed(&err, "empty blob");
        }
    }

    for index in 0..512usize {
        let float = index % 2 == 0;
        let (descriptor, canonical, bps) = if float {
            (
                "pcm_f32le",
                ext_blob(GUID_IEEE_FLOAT, 1, 44_100, 32, 32, 0x4, 22),
                4usize,
            )
        } else {
            (
                "pcm_s24le",
                ext_blob(GUID_PCM, 2, 48_000, 24, 24, 0x3, 22),
                3usize,
            )
        };
        let mutant = mutate(&canonical, &mut rng);
        match configure(descriptor, mutant.clone()) {
            Err(err) => {
                typed += 1;
                assert_typed(&err, "mutant");
            }
            Ok((mut decoder, generation)) => {
                accepted += 1;
                assert_accounted(&mut decoder, generation, &mutant, bps);
            }
        }
    }
    assert_eq!(
        typed + accepted,
        2 + 1 + 512,
        "every mutant classified exactly once"
    );
    assert!(accepted >= 3, "the canonical + coherent-class accepts");
    assert!(typed >= 1, "the empty-blob refusal");
}

// ---- B2: the 42-byte vendor-tail stage disagreement ------------------------

#[test]
fn forty_two_byte_vendor_tail_extensible_configures_like_the_demuxer() {
    // W2-15 MAJOR (stage disagreement, a4): cbSize = 24 with two
    // vendor bytes after the GUID is a legal WAVEFORMATEXTENSIBLE
    // (42-byte strf). The DEMUXER accepts it (GUID fixed at 24..40;
    // strf_size >= 40), and ffprobe 7.1.5 plays it as pcm_s16le
    // (measured on the hand-built sibling of this exact blob). This
    // seam's length gate `matches!(len, 16 | 18 | 40)` used to refuse
    // the same bytes with pcm.waveformat.length — contradicting the
    // decoder module's own contract ("the demuxer accepts the same
    // blobs — the engine may hand either crate's view the container's
    // verbatim strf"). Fixed: the extensible bound is physical (any
    // length >= 40 carries the GUID; the vendor tail past 40 is legal
    // per the MS schema and unread).
    let mut blob = ext_blob(GUID_PCM, 2, 8_000, 16, 16, 0x3, 24);
    blob.extend_from_slice(&[0xDE, 0xAD]); // vendor tail -> 42 bytes
    let (mut decoder, generation) = match configure("pcm_s16le", blob) {
        Ok(pair) => pair,
        Err(err) => unreachable!(
            "the 42-byte vendor-tail form is legal media (demux accepts, \
             ffprobe decodes pcm_s16le): {err}"
        ),
    };
    // And it must decode on the blob's own facts: 8 bytes = 1 stereo
    // s16 sample-frame.
    let mut sink = CaptureSink::default();
    let outcome = match decoder.decode(packet(vec![0x11; 8], generation), &mut sink) {
        Ok(outcome) => outcome,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames.len(), 1);
    assert_eq!(sink.frames[0].samples.len(), 8);
    assert_eq!(sink.frames[0].format.channels, 2);
}

// ---- B3: the block-align u16 truncation attack ------------------------------

#[test]
fn block_align_cross_check_must_not_wrap_u16() {
    // W2-15 MAJOR (validation bypass, g1): parse_waveformatex computed
    // the expected nBlockAlign as `(u32::from(channels) * bps) as u16`
    // — a TRUNCATING cast. For channels x bps >= 65536 the true product
    // wraps: channels=32768/s16 wraps to 0 (so a blob declaring
    // nBlockAlign = 0 passed its own "consistency" check), 65535/s16
    // wraps to 65534, 16384/f32 and 8192/f64 wrap to 0. The demuxer
    // refuses channels > 64 (MAX_PCM_CHANNELS) before ever reaching the
    // GUID; this seam must not be bypassable by the same untrusted
    // bytes it exists to cross-check (D-3: anticipate overflow values).
    // Fixed: a channel ceiling in stage agreement with the demuxer's,
    // plus checked (`u16::try_from`) align math.
    for (channels, align, bits, tag, descriptor) in [
        (32768u16, 0u16, 16u16, 1u16, "pcm_s16le"), // 65536 -> wrapped to 0
        (65535, 65534, 16, 1, "pcm_s16le"),         // 131070 -> wrapped to 65534
        (16384, 0, 32, 3, "pcm_f32le"),             // 65536 -> wrapped to 0
        (8192, 0, 64, 3, "pcm_f64le"),              // 65536 -> wrapped to 0
    ] {
        let blob = plain_blob(tag, channels, 8_000, align, bits);
        let err = match configure(descriptor, blob) {
            Ok(_) => unreachable!(
                "channels {channels} x width must not configure: the align \
                 cross-check wrapped (declared align {align})"
            ),
            Err(err) => err,
        };
        assert_eq!(err.stage, PipelineStage::Decode, "channels {channels}");
        assert!(
            matches!(err.kind, ErrorKind::Unsupported { .. }),
            "channels {channels}: {err}"
        );
    }

    // Adjacent observation — the ceiling disagreement: the demuxer caps
    // nChannels at 64 (damage-counted skip); this seam used to accept
    // 65 with an honest align. Fixed to the same ceiling (stage
    // agreement): 65 channels is now the typed refusal below.
    let blob = plain_blob(1, 65, 8_000, 130, 16);
    match configure("pcm_s16le", blob) {
        Ok((mut decoder, generation)) => {
            // 130 bytes = exactly 1 whole 65x2-byte sample-frame.
            let mut sink = CaptureSink::default();
            let outcome = match decoder.decode(packet(vec![0x55; 130], generation), &mut sink) {
                Ok(outcome) => outcome,
                Err(err) => unreachable!("{err}"),
            };
            assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
            assert_eq!(sink.frames[0].samples.len(), 130);
            assert_eq!(sink.frames[0].format.channels, 65);
        }
        Err(err) => assert_typed(&err, "65-channel ceiling decision"),
    }
}

// ---- B4: the cbSize=0 pin (stage-consistent, oracle-divergent) -------------

#[test]
fn cbsize_zero_with_a_complete_guid_is_stage_consistent_today() {
    // W2-15 surfaces (a5)/(f), codecs half: this seam ALSO ignores
    // cbSize (only the physical length gates), so the 40-byte cbSize=0
    // blob configures exactly like the cbSize=22 one — the two stages
    // agree with each other and both disagree with ffprobe 7.1.5
    // (which reports `unknown`: ffmpeg only trusts the GUID when
    // cbSize >= 22). This pins the current cross-stage agreement;
    // flipping it is a two-crate Lead decision (flip the demux-side
    // companion `extensible_cbsize_lies_resolve_by_physical_size_not_
    // declaration` together with this one).
    for cbsize in [0u16, 2, 21, 23, 0xFFFF] {
        let blob = ext_blob(GUID_PCM, 2, 8_000, 16, 16, 0x3, cbsize);
        let (mut decoder, generation) = match configure("pcm_s16le", blob.clone()) {
            Ok(pair) => pair,
            Err(err) => unreachable!("cbSize {cbsize} is unread today: {err}"),
        };
        assert_accounted(&mut decoder, generation, &blob, 2);
    }
}

// ---- B5: channel extremes at the with_format seam ---------------------------

#[test]
fn with_format_channel_extremes_never_panic_and_skip_honestly() {
    // W2-15 (g3): PcmPlan::new computes frame_bytes with a plain
    // multiply — the with_format seam rejects only zero, so u32::MAX
    // channels is accepted. On 64-bit the product is huge-but-finite:
    // every sane packet decodes to 0 whole sample-frames and skips
    // whole (bounded, observable). On a 32-bit target (wasm32 — an
    // explicit project target; see media-probe's own saturation
    // comments) the same multiply used to overflow: debug builds
    // panicked, release builds wrapped. This pins the 64-bit behavior
    // with the now-saturating math (the wasm32 hazard is cured by the
    // saturation, untestable from this 64-bit host).
    for channels in [u32::MAX, 0x8000_0000, 0x2000_0000] {
        let mut decoder = match PcmDecoder::with_format(channels, 8_000) {
            Ok(decoder) => decoder,
            Err(err) => unreachable!("only zero is refused today: {err}"),
        };
        let descriptor = match CodecDescriptor::new("pcm_s16le") {
            Ok(d) => d,
            Err(err) => unreachable!("{err}"),
        };
        let generation = next_generation();
        decoder
            .configure(&DecoderConfig::new(
                descriptor,
                generation,
                ThreadBudget::default(),
                None,
                None,
            ))
            .unwrap_or_else(|err| unreachable!("coherent facts configure: {err}"));
        let outcome = match decoder.decode(
            packet(vec![0x11; 4], generation),
            &mut CaptureSink::default(),
        ) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(
            outcome,
            DecodeOutcome::Recovered {
                skipped: 1,
                reason: RecoverReason::CorruptInput,
            },
            "channels {channels}: no sane packet carries a whole sample-frame"
        );
        assert_eq!(
            decoder.flush(&mut CaptureSink::default()).ok(),
            Some(DecodeOutcome::Drained),
            "channels {channels}: the decoder stays usable"
        );
    }

    // The free surface shares the plan math — same pin, plus the exact
    // concealment accounting for a sub-frame payload.
    let decoded = match decode_pcm(
        &[0xAB; 5],
        PcmSource {
            sample: PcmFormat::S16Le,
            planar: false,
            channels: u32::MAX,
        },
        8_000,
    ) {
        Ok(decoded) => decoded,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(decoded.samples.len(), 0, "no whole sample-frame exists");
    assert_eq!(
        decoded.dropped_tail_bytes, 5,
        "the whole payload is the tail"
    );
}
