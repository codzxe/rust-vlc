//! FX1-A07 (FIX-PLAN F0.4j): the ASF header child walk's object-size
//! check must treat a crafted near-u64::MAX child size as the typed
//! `demux.asf.bad-object-size` refusal — W04-A10 F-001: the plain
//! `off as u64 + size` add (header.rs, the Header Object children
//! walk) overflowed on a child size ≥ 2^64 − offset (debug: "attempt
//! to add with overflow"; release: the sum wrapped past the
//! `end > header_size` guard into a `start > end` slice panic), a
//! direct violation of the module's no-panic house rule.
//!
//! The crafted blobs are the builder's minimal one-audio-stream ASF
//! with ONE size QWORD patched per case:
//!
//! * first child (File Properties Object) size := `u64::MAX` — the
//!   overflow class at the walk's first iteration;
//! * second child (Stream Properties) size := `u64::MAX` — the same
//!   class mid-walk, after the FPO already parsed;
//! * second child size := 1000 — a non-overflow overrun, pinning that
//!   the tightened check did not change the existing typed refusal;
//! * unpatched — the control: the walk still accepts every
//!   well-formed child and `start` returns the PCM track.

use std::sync::Arc;

use demux::asf::AsfDemuxer;
use demux::asf::test_support::{mem_stream, minimal_asf_audio, waveformatex};
use media_types::EngineError;
use plugin_api::Demuxer;

/// The Header Object head is 30 bytes; the first child (File
/// Properties, 24 + 80 bytes) frame starts at 30, its size QWORD at
/// +16; the second child (Stream Properties) frame starts at
/// 30 + 104.
const FIRST_CHILD_SIZE_AT: usize = 30 + 16;
const SECOND_CHILD_SIZE_AT: usize = 30 + 104 + 16;

/// The minimal one-audio-stream ASF with one child-size QWORD patched.
fn crafted(size_at: usize, size: u64) -> Arc<[u8]> {
    let wfx = waveformatex(1, 1, 11_025, 2, 16);
    let mut bytes = minimal_asf_audio(&wfx).to_vec();
    bytes[size_at..size_at + 8].copy_from_slice(&size.to_le_bytes());
    Arc::from(bytes.as_slice())
}

/// Hand crafted bytes to `start` and return its typed error — a
/// success is itself the failure under test.
fn start_refusal(bytes: Arc<[u8]>) -> EngineError {
    let stream = match mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem_stream: {err}"),
    };
    let mut asf = match AsfDemuxer::new(stream) {
        Ok(asf) => asf,
        Err(err) => unreachable!("new: {err}"),
    };
    match asf.start() {
        Ok(_) => unreachable!("crafted child size must be refused"),
        Err(err) => err,
    }
}

#[test]
fn fx1_a07_asf_child_size_overflow_is_typed_bad_object_size() {
    // First child, the walk's opening iteration: pre-fix this was the
    // overflowing `off + size` add (or, in release, the wrapped sum
    // slipping past the guard into the payload slice).
    let err = start_refusal(crafted(FIRST_CHILD_SIZE_AT, u64::MAX));
    assert_eq!(err.context.code, "demux.asf.bad-object-size");

    // Second child, mid-walk after the File Properties parsed: the
    // same crafted size on the Stream Properties frame.
    let err = start_refusal(crafted(SECOND_CHILD_SIZE_AT, u64::MAX));
    assert_eq!(err.context.code, "demux.asf.bad-object-size");
}

#[test]
fn fx1_a07_asf_child_overrun_without_overflow_still_typed() {
    // A 1000-byte child inside a 230-byte header: no u64 overflow, the
    // pre-existing `end > header_size` refusal — unchanged semantics.
    let err = start_refusal(crafted(SECOND_CHILD_SIZE_AT, 1000));
    assert_eq!(err.context.code, "demux.asf.bad-object-size");
}

#[test]
fn fx1_a07_asf_well_formed_children_still_walk() {
    // Control: the tightened check must not refuse a well-formed file.
    let wfx = waveformatex(1, 1, 11_025, 2, 16);
    let bytes = minimal_asf_audio(&wfx);
    let stream = match mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem_stream: {err}"),
    };
    let mut asf = match AsfDemuxer::new(stream) {
        Ok(asf) => asf,
        Err(err) => unreachable!("new: {err}"),
    };
    let tracks = match asf.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("well-formed minimal file must start: {err}"),
    };
    assert_eq!(tracks.len(), 1, "one audio track");
    assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
}
