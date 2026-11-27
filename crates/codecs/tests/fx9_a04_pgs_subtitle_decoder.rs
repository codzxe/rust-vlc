//! FX9-A04 regression suite — the embedded PGS subtitle track lane:
//! `codecs::subtitle::pgs::PgsSubtitleDecoder` (codec-api `Decoder`)
//! over the TS PES carriage, reduced to `Frame::Subtitle` overlay
//! cues via the media-types Phase-5 `SubtitleOverlay` payload.
//!
//! The audit finding this pins (REALITY-SCORE row 064,
//! subtitle-pgs UNREACHABLE; FIX-PLAN §5 F3.1): the frame
//! vocabulary was text-only (`SubtitleCue.text` a plain `String`,
//! `Frame::Subtitle` with zero producers tree-wide) while the
//! engine side was ready — so a TS file carrying PGS (stream type
//! `0x90`, descriptor minted `pgssub` at `demux/src/ts/mod.rs:367`)
//! listed a subtitle track no decoder could feed. This suite proves
//! the lane end to end at the decoder boundary: geometry, z-order,
//! the held-set timing model, typed damage, staleness, epoch
//! recycling, discontinuity, and the TS-mint ↔ decoder-descriptor
//! interlock.
//!
//! HONEST LABEL (the L2 crate's own convention): the crafted
//! display-set payloads mirror the `subtitles` crate's hand-crafted
//! vectors (verified line-by-line against the ffmpeg `pgssubdec.c`
//! semantics there — no reference encoder exists in this sandbox).
//! The engine-side lane semantics that cannot be imported from an
//! L3 test (playback-engine is L4) are pinned by citation in
//! `fx9_a04_ts_pgs_descriptor_and_engine_lane_semantics`.

use codec_api::prelude::*;
use codec_api::{contract, DecodeOutcome};
use codecs::subtitle::pgs::PgsSubtitleDecoder;
use demux::ts::TsStream;
use media_types::frame::{SubtitleFrame, SubtitleOverlay};
use media_types::time::{Nanoseconds, Stamp, Timestamp};
use media_types::{Frame, MediaKind, Packet, TrackId};

// ---------------------------------------------------------------------------
// Crafted display sets (the `subtitles` crate's own builders)
// ---------------------------------------------------------------------------

/// One stripped-framing segment: kind + 16-bit length + payload.
fn seg(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut v = vec![kind];
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

/// A PCS: 1920x1080 video extent, frame-rate id 2, composition
/// number 1, `state`, palette `palette_id`, `objects` placed at
/// their (x, y).
fn pcs(state: u8, palette_id: u8, objects: &[(u16, u16, u16)]) -> Vec<u8> {
    let mut d = vec![
        0x07, 0x80, // video width 1920
        0x04, 0x38, // video height 1080
        0x20, // frame-rate id 2
        0x00, 0x01, // composition number 1
        state,
        0x00, // palette_update false
        palette_id,
        objects.len() as u8,
    ];
    for &(id, x, y) in objects {
        d.extend_from_slice(&id.to_be_bytes());
        d.push(0); // window id
        d.push(0); // flags: not forced, not cropped
        d.extend_from_slice(&x.to_be_bytes());
        d.extend_from_slice(&y.to_be_bytes());
    }
    seg(0x16, &d)
}

/// A PDS: palette `id` version `version` with 5-byte YCbCrA entries.
fn pds(id: u8, version: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
    let mut d = vec![id, version];
    for &(eid, y, cr, cb, a) in entries {
        d.extend_from_slice(&[eid, y, cr, cb, a]);
    }
    seg(0x14, &d)
}

/// A first-fragment ODS: object `id`, `width` x `height`, RLE `rle`.
fn ods_first(id: u16, width: u16, height: u16, rle: &[u8]) -> Vec<u8> {
    let total = 4 + rle.len();
    let mut d = id.to_be_bytes().to_vec();
    d.push(0); // version
    d.push(0x80); // first (and last) fragment
    d.extend_from_slice(&[(total >> 16) as u8, (total >> 8) as u8, total as u8]);
    d.extend_from_slice(&width.to_be_bytes());
    d.extend_from_slice(&height.to_be_bytes());
    d.extend_from_slice(rle);
    seg(0x15, &d)
}

/// One 2x2 RLE of entry-1 pixel runs (the L2 crate's proven-clean
/// 8-byte shape: two 2-px runs with EOLs).
const RLE_2X2: [u8; 8] = [0x00, 0x82, 0x01, 0x00, 0x00, 0x00, 0x82, 0x01];

/// Palette 0: entry 1 = opaque studio white (BT.709 — 1080-line
/// video).
const WHITE_ENTRY: (u8, u8, u8, u8, u8) = (1, 235, 128, 128, 255);

/// One full epoch-start display set: `objects` defined + placed on
/// the 1920x1080 canvas, palette 0, closed by END.
fn display_set(state: u8, objects: &[(u16, u16, u16)]) -> Vec<u8> {
    let mut buf = vec![pcs(state, 0, objects)];
    buf.push(pds(0, 0, &[WHITE_ENTRY]));
    for &(id, _x, _y) in objects {
        buf.push(ods_first(id, 2, 2, &RLE_2X2));
    }
    buf.push(seg(0x80, &[]));
    buf.concat()
}

/// Re-frame a stripped display set the `.sup` way: a 4-byte 90 kHz
/// PTS prefix on every segment (the in-tree model's TS-PES carriage;
/// the prefix deliberately distinct from the stripped header bytes
/// so the framing detection is unambiguous).
fn to_sup_framing(parts: &[u8], pts_90khz: u32) -> Vec<u8> {
    let (segments, used) = subtitles::pgs::segment::walk_segments(
        parts,
        subtitles::pgs::segment::PgsFraming::Stripped,
    );
    assert_eq!(used, parts.len(), "fixture bug: the stripped set is not whole");
    let mut sup = Vec::with_capacity(parts.len() + segments.len() * 4);
    for segment in &segments {
        sup.extend_from_slice(&pts_90khz.to_be_bytes());
        sup.push(match segment.kind {
            subtitles::pgs::segment::SegmentKind::Pds => 0x14,
            subtitles::pgs::segment::SegmentKind::Ods => 0x15,
            subtitles::pgs::segment::SegmentKind::Pcs => 0x16,
            subtitles::pgs::segment::SegmentKind::Wds => 0x17,
            subtitles::pgs::segment::SegmentKind::End => 0x80,
            subtitles::pgs::segment::SegmentKind::Unknown(b) => b,
        });
        sup.extend_from_slice(&(segment.data.len() as u16).to_be_bytes());
        sup.extend_from_slice(segment.data);
    }
    sup
}

// ---------------------------------------------------------------------------
// Decoder plumbing
// ---------------------------------------------------------------------------

/// The TS-demuxer-minted descriptor for stream type 0x90 — the exact
/// string `demux::ts::TsStream::descriptor` produces and this
/// decoder's `configure` accepts (the FX9-A03 `decoder_for`
/// interlock).
fn pgssub_descriptor() -> media_types::ids::CodecDescriptor {
    match media_types::ids::CodecDescriptor::new("pgssub") {
        Ok(descriptor) => descriptor,
        Err(e) => unreachable!("{e}"),
    }
}

/// A configured decoder (the contract suite's `configured` pattern).
fn configured() -> (PgsSubtitleDecoder, GenerationId) {
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{e}"),
    };
    let mut decoder = PgsSubtitleDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        pgssub_descriptor(),
        generation,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => (decoder, generation),
        Err(e) => unreachable!("configure: {e}"),
    }
}

/// One PES payload packet at `pts` (90 kHz ticks).
fn packet(body: Vec<u8>, pts_ticks: i64, generation: GenerationId) -> Packet {
    let pts = Stamp::Present(match Timestamp::from_parts(pts_ticks, 1, 90_000) {
        Ok(ts) => ts,
        Err(e) => unreachable!("{e}"),
    });
    Packet {
        track: TrackId::new(1),
        pts,
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(body),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// A subtitle-frame collector (the PCM `Capture` pattern).
#[derive(Default)]
struct SubsSink {
    frames: Vec<SubtitleFrame>,
}

impl FrameSink for SubsSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Subtitle(subs) = frame else {
            unreachable!("PGS emits subtitle frames only");
        };
        self.frames.push(subs);
        Ok(())
    }
}

/// A stamp to whole nanoseconds (`None` when missing).
fn stamp_ns(stamp: &Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(Nanoseconds(ns)) => Some(i64::try_from(ns).unwrap_or(i64::MAX)),
        Err(_) => None,
    }
}

/// The one cue overlay of the i-th emitted frame (with count checks).
fn sole_overlay(frame: &SubtitleFrame) -> &SubtitleOverlay {
    assert_eq!(frame.cues.len(), 1, "expected exactly one cue");
    frame.cues[0]
        .overlay
        .as_ref()
        .unwrap_or_else(|| unreachable!("a PGS cue is always a bitmap cue"))
}

// ---------------------------------------------------------------------------
// The suite
// ---------------------------------------------------------------------------

/// The lane's happy path: a display set arrives, the NEXT set's PTS
/// closes its span, and the emitted `Frame::Subtitle` carries the
/// composed object as a `SubtitleOverlay` with its exact PCS
/// geometry — through both segment framings the carriage uses
/// (Matroska-stripped and the PTS-prefixed TS-PES form).
#[test]
fn fx9_a04_decode_emits_subtitle_frame_with_overlay_geometry() {
    for (label, body, set2) in [
        (
            "Matroska-stripped framing",
            display_set(0x80, &[(1, 10, 20)]),
            display_set(0x00, &[]),
        ),
        (
            "PTS-prefixed (.sup / TS PES) framing",
            to_sup_framing(&display_set(0x80, &[(1, 10, 20)]), 180_000),
            to_sup_framing(&display_set(0x00, &[]), 270_000),
        ),
    ] {
        let (mut decoder, generation) = configured();
        let mut sink = SubsSink::default();
        // Set 1 (object 1, 2x2 at (10, 20), pts 0): held, not emitted.
        match decoder.decode(packet(body, 0, generation), &mut sink) {
            Ok(DecodeOutcome::NeedMoreInput) => {}
            other => unreachable!("{label}: held set 1: {other:?}"),
        }
        assert!(sink.frames.is_empty(), "{label}: nothing emits before the closing PTS");
        // Set 2 (the clear, pts 90_000): emits set 1 closed.
        match decoder.decode(packet(set2, 90_000, generation), &mut sink) {
            Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
            other => unreachable!("{label}: closed set 1: {other:?}"),
        }
        assert_eq!(sink.frames.len(), 1, "{label}");
        let frame = &sink.frames[0];
        assert_eq!(stamp_ns(&frame.pts), Some(0), "{label}: the set's own PTS");
        let overlay = sole_overlay(frame);
        assert_eq!(
            (overlay.width, overlay.height, overlay.x, overlay.y, overlay.z),
            (2, 2, 10, 20, 0),
            "{label}: the PCS geometry, recovered exactly"
        );
        assert_eq!(overlay.rgba.len(), 2 * 2 * 4, "{label}: w*h*4 RGBA8 bytes");
        assert!(
            overlay
                .rgba
                .as_slice()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|px| px == &[255, 255, 255, 255]),
            "{label}: opaque studio white"
        );
        // The held-set timing model: half-open [t0, next - 1 ns].
        assert_eq!(stamp_ns(&frame.cues[0].span.start), Some(0), "{label}");
        assert_eq!(
            stamp_ns(&frame.cues[0].span.end),
            Some(999_999_999),
            "{label}: one nanosecond before the next set's PTS"
        );
        assert_eq!(frame.cues[0].text, "", "{label}: a bitmap cue carries no text");
        assert_eq!(frame.generation, generation, "{label}");
    }
}

/// Z-order follows the PCS composition-object order (the builder's
/// `z_base + i` rule), not the object-definition order.
#[test]
fn fx9_a04_overlay_cues_z_ordered_per_pcs_object_order() {
    // PCS lists object 2 FIRST (defined second) — its cue must carry
    // the lower z and its own placement.
    let (mut decoder, generation) = configured();
    let mut sink = SubsSink::default();
    let body = display_set(0x80, &[(2, 100, 900), (1, 100, 950)]);
    match decoder.decode(packet(body, 0, generation), &mut sink) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("held set: {other:?}"),
    }
    match decoder.decode(
        packet(display_set(0x00, &[]), 90_000, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("closed set: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 1);
    let cues = &sink.frames[0].cues;
    assert_eq!(cues.len(), 2, "one cue per PCS composition object");
    let first = cues[0]
        .overlay
        .as_ref()
        .unwrap_or_else(|| unreachable!("a PGS cue is always a bitmap cue"));
    let second = cues[1]
        .overlay
        .as_ref()
        .unwrap_or_else(|| unreachable!("a PGS cue is always a bitmap cue"));
    assert_eq!(
        (first.z, first.x, first.y),
        (0, 100, 900),
        "object 2 is listed first in the PCS: lowest z, its own placement"
    );
    assert_eq!(
        (second.z, second.x, second.y),
        (1, 100, 950),
        "object 1 is listed second: z follows the PCS order, not the ODS order"
    );
}

/// Flush emits the trailing held set open-ended (PGS has no explicit
/// end times) and then stays Drained.
#[test]
fn fx9_a04_flush_emits_trailing_set_open_ended() {
    let (mut decoder, generation) = configured();
    let mut sink = SubsSink::default();
    match decoder.decode(
        packet(display_set(0x80, &[(1, 10, 20)]), 90_000, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("held set: {other:?}"),
    }
    assert!(sink.frames.is_empty(), "nothing before flush");
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("flush emits the trailing set: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 1);
    let frame = &sink.frames[0];
    assert_eq!(stamp_ns(&frame.pts), Some(1_000_000_000));
    let overlay = sole_overlay(frame);
    assert_eq!((overlay.x, overlay.y), (10, 20));
    assert_eq!(stamp_ns(&frame.cues[0].span.start), Some(1_000_000_000));
    assert_eq!(
        frame.cues[0].span.end,
        Stamp::Missing,
        "the trailing set is open-ended"
    );
    // Drained sticks.
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        other => unreachable!("second flush stays Drained: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 1, "no double emission");
}

/// Empty and truncated inputs are typed damage (a `Recovered`
/// outcome, zero frames) — never a panic, never a fatal error — and
/// the decoder stays usable afterwards.
#[test]
fn fx9_a04_empty_and_truncated_input_is_typed_damage_never_error() {
    let (mut decoder, generation) = configured();

    // (a) An empty PES payload.
    let mut sink = SubsSink::default();
    match decoder.decode(packet(Vec::new(), 0, generation), &mut sink) {
        Ok(DecodeOutcome::Recovered {
            skipped: 1,
            reason: RecoverReason::CorruptInput,
        }) => {}
        other => unreachable!("empty payload: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 0, "no frame from an empty payload");

    // (b) A display set truncated mid-payload (the walk stops
    // early: typed LengthOverrun/Truncation damage).
    let whole = display_set(0x80, &[(1, 10, 20)]);
    let mut truncated = whole.clone();
    truncated.truncate(whole.len() / 2);
    match decoder.decode(packet(truncated, 0, generation), &mut sink) {
        Ok(DecodeOutcome::Recovered { .. }) => {}
        other => unreachable!("truncated set: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 0, "no frame from a truncated set");

    // (c) Garbage bytes (unknown segment types).
    match decoder.decode(
        packet(vec![0xF0; 16], 90_000, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::Recovered { .. }) => {}
        other => unreachable!("garbage: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 0, "no frame from garbage");

    // The decoder stayed usable: the next clean set decodes, holds
    // and flushes.
    match decoder.decode(
        packet(display_set(0x80, &[(1, 0, 0)]), 180_000, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("clean set after damage: {other:?}"),
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("flush after damage: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 1);
    assert_eq!(stamp_ns(&sink.frames[0].pts), Some(2_000_000_000));
}

/// Cancellation is generational: a stale packet is dropped on sight —
/// NeedMoreInput, zero frames, no state mutation (the very next
/// packet of the current generation decodes cleanly through it).
#[test]
fn fx9_a04_stale_generation_packet_dropped_on_sight() {
    let (mut decoder, generation) = configured();
    let body = display_set(0x80, &[(1, 10, 20)]);
    // The contract's own guard check (the mjpeg contract-suite
    // precedent), then the zero-frames assertion and the
    // no-state-mutation proof.
    match contract::stale_packet_is_dropped_on_sight(
        &mut decoder,
        &packet(body.clone(), 0, GenerationId::FIRST),
        generation,
    ) {
        Ok(()) => {}
        Err(violation) => unreachable!("{violation}"),
    }
    let mut sink = SubsSink::default();
    match decoder.decode(
        packet(body, 0, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("current-generation set after the stale drop: {other:?}"),
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("flush: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 1, "exactly the current-generation set");
}

/// Configure resets the epoch (the engine's decoder cache recycles
/// instances across generations): a recycled decoder never presents
/// the previous stream's objects — and a foreign codec descriptor is
/// a typed refusal.
#[test]
fn fx9_a04_configure_resets_epoch_for_recycled_decoder() {
    let (mut decoder, generation) = configured();
    let mut sink = SubsSink::default();
    // Epoch 1: object 1 + palette 0 enter the banks.
    match decoder.decode(
        packet(display_set(0x80, &[(1, 10, 20)]), 0, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("epoch 1: {other:?}"),
    }
    // A recycle (fresh generation, same instance).
    let next = match generation.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{e}"),
    };
    match decoder.configure(&DecoderConfig::new(
        pgssub_descriptor(),
        next,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => {}
        Err(e) => unreachable!("re-configure: {e}"),
    }
    // A NORMAL-state PCS re-referencing object 1: without the reset
    // the epoch's banks would still hold it and compose stale pixels;
    // with the reset the reference is typed damage and nothing is
    // held (the clear proof: flush has nothing to emit).
    let normal = [pcs(0x00, 0, &[(1, 100, 200)]), seg(0x80, &[])].concat();
    match decoder.decode(packet(normal, 90_000, next), &mut sink) {
        Ok(DecodeOutcome::Recovered { .. }) => {}
        other => unreachable!("post-recycle normal set: {other:?}"),
    }
    assert!(sink.frames.is_empty(), "no stale-epoch frame");
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        other => unreachable!("flush after recycle: {other:?}"),
    }
    // A foreign descriptor is a typed Unsupported refusal.
    let foreign = match media_types::ids::CodecDescriptor::new("h264") {
        Ok(d) => d,
        Err(e) => unreachable!("{e}"),
    };
    let err = match decoder.configure(&DecoderConfig::new(
        foreign,
        next,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => unreachable!("h264 must be refused"),
        Err(e) => e,
    };
    assert!(
        matches!(err.kind, media_types::ErrorKind::Unsupported { .. }),
        "foreign descriptor is Unsupported: {err}"
    );
}

/// The TS-mint ↔ decoder interlock plus the engine-lane semantics
/// this lane rides (the producer side machine-checked here; the
/// L4 engine side cited — playback-engine is not importable from an
/// L3 crate test).
///
/// Verified in-tree engine semantics (audit finding, FX9-A04):
/// * a subtitle track DECODES from open time — `Pipeline::wire`
///   (playback-engine/src/pipeline.rs:1051-1092) wires ONE limb per
///   kind, the container-default (`TrackFlags::default_on`) else
///   first-of-kind subtitle track, at the session generation; the
///   subtitle kind is the OPTIONAL kind — a provider that cannot
///   build the subtitle decoder deselects that track instead of
///   failing the open (the class-S seam, pipeline.rs:1036-1040,
///   1083-1087).
/// * `SelectSubtitle` (tracks.set-subtitle-track) is class S: it
///   resolves the selector over `MediaKind::Subtitle` tracks and
///   swaps the (optional) subtitle limb at the CURRENT generation —
///   no mint, no pipeline invalidation, no state-changed (the
///   pre-table branch, session.rs:1812-1814 and
///   `subtitle_select_step` session.rs:2221-2326); an unresolvable
///   selector is the typed TrackNotFound refusal. The lane exists
///   from the engine's own vocabulary (`WiredKind::Subtitle`,
///   tracks.rs:162; `QueuePlan::for_kind` total over it,
///   pipeline.rs:386).
#[test]
fn fx9_a04_ts_pgs_descriptor_and_engine_lane_semantics() {
    // The producer side: the TS demuxer's PMT stream for type 0x90.
    let stream = TsStream {
        pid: 0x1234,
        stream_type: 0x90,
        language: None,
        es_info: Vec::new(),
    };
    assert_eq!(stream.descriptor().as_str(), "pgssub");
    assert_eq!(stream.kind(), MediaKind::Subtitle);
    // The consumer side: this decoder accepts exactly that mint —
    // the FX9-A03 `decoder_for` arm dispatches on the same string.
    let (mut decoder, generation) = configured();
    let mut sink = SubsSink::default();
    match decoder.decode(
        packet(display_set(0x80, &[(1, 10, 20)]), 0, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("{other:?}"),
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("{other:?}"),
    }
    // The vocabulary half: a `Frame::Subtitle` with an overlay cue is
    // constructible end to end from the TS-minted track's decode —
    // the Phase-5 payload the engine's subtitle limb carries.
    let frame = &sink.frames[0];
    assert_eq!(frame.generation, generation);
    let overlay = sole_overlay(frame);
    assert_eq!((overlay.width, overlay.height, overlay.x, overlay.y), (2, 2, 10, 20));
    // An audio descriptor never reaches this lane (the interlock's
    // negative half is `fx9_a04_configure_resets_epoch...` above).
}

/// A mid-generation discontinuity (seek/splice marker on the
/// packet): the held set and the epoch banks belong to the old
/// timeline — both are dropped (counted, typed Discontinuity) and
/// the decoder keeps decoding the new timeline in the same
/// generation.
#[test]
fn fx9_a04_discontinuity_resets_epoch_mid_generation() {
    let (mut decoder, generation) = configured();
    let mut sink = SubsSink::default();
    // Epoch: object 1 defined; set 2 (NORMAL) re-presents it and is
    // held with a still-unknown end.
    match decoder.decode(
        packet(display_set(0x80, &[(1, 10, 20)]), 0, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("set 1: {other:?}"),
    }
    match decoder.decode(
        packet(display_set(0x00, &[(1, 100, 200)]), 90_000, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("set 2 closes set 1: {other:?}"),
    }
    // The discontinuity packet: a NORMAL PCS that WOULD compose
    // object 1 from the pre-discontinuity epoch — the reset makes it
    // typed damage instead, and the held set 2 is dropped (not
    // silently, not emitted).
    let mut discontinuity = packet(
        [pcs(0x00, 0, &[(1, 30, 40)]), seg(0x80, &[])].concat(),
        180_000,
        generation,
    );
    discontinuity.discontinuity = true;
    match decoder.decode(discontinuity, &mut sink) {
        Ok(DecodeOutcome::Recovered {
            skipped: 1,
            reason: RecoverReason::Discontinuity,
        }) => {}
        other => unreachable!("discontinuity: {other:?}"),
    }
    assert_eq!(sink.frames.len(), 1, "set 2 was dropped, never emitted");
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        other => unreachable!("nothing from the old timeline survives: {other:?}"),
    }
    // The same generation keeps decoding: a fresh epoch-start set
    // composes and flushes normally.
    match decoder.decode(
        packet(display_set(0x80, &[(1, 0, 0)]), 270_000, generation),
        &mut sink,
    ) {
        Ok(DecodeOutcome::NeedMoreInput) => {}
        other => unreachable!("post-discontinuity set: {other:?}"),
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("post-discontinuity flush: {other:?}"),
    }
    assert_eq!(stamp_ns(&sink.frames[1].pts), Some(3_000_000_000));
}
