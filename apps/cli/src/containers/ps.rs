//! The MPEG-PS/VOB container host (card 6.OPW-ps — the registry's
//! FINAL wired family, the ISO/IEC 13818-1 program stream: `.mpg` /
//! `.mpeg` / `.ps` / `.vob`): construct the demuxer, run the host
//! pre-pass `start()` (the header-only pack/PES scan), harvest every
//! discovered elementary stream's facts into [`TrackFact`]s, and wrap
//! in the shared [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! # No bridge (the family's distinguishing simplification)
//!
//! Unlike the TS sibling, [`PsDemuxer`] emits its packets ITSELF —
//! the PES parse lives inside the demuxer (no
//! [`PesConsumer`](demux::ts::PesConsumer) seam to bridge). The host
//! wraps it in [`PsHostDemuxer`] for ONE decode-contract reason
//! below, forwarding every trait answer verbatim.
//!
//! # The MPEG-audio streaming frame assembler (the decode truth)
//!
//! The demuxer's container truth is the PES packet ("one PES = one
//! demuxed packet" — the 5.I2 recorded design; ffmpeg re-packetizes
//! to codec access units, so ffprobe's counts legitimately differ).
//! But the MP2 decoder's packet contract is **one frame per packet**
//! (the raw-ES precedent — a multi-frame buffer decodes only its
//! first frame), and MEASURED on the committed fixture the muxer
//! cuts audio PES at ~2 KiB boundaries **without frame alignment**:
//! `ps-mp2-video-audio.mpg`'s 5 audio PES carry 8 150 payload bytes
//! = exactly 39 Layer II frames, and **4 of the 39 frames span a PES
//! boundary** (the trailing 14-byte unstamped PES exists only to
//! complete frame 39). A per-PES split is therefore impossible —
//! [`MpegAudioAssembler`] is a STREAMING frame walk over the
//! concatenated payload stream: leftover bytes from PES *n* prepend
//! to PES *n+1*, and a frame is minted only once complete.
//!
//! Per-AU stamps are the **anchor + exact rational sample
//! arithmetic** the TS unit established (`anchor +
//! round(samples_before × 90 000 / rate)`, u128 intermediate): each
//! PES's PTS anchors the frames whose first byte rides in it, and a
//! frame spanning a boundary belongs to the PES that started it
//! (MEASURED: the fixture's last AU stamps 136 957 == ffprobe's last
//! synthesized row exactly; intermediate AUs may differ by a tick —
//! ffmpeg accumulates per-AU truncations, this wiring anchors each
//! PES's AUs to the wire stamp). Damage is bounded and never silent:
//! an unparseable walk position re-syncs to the next judgeable frame
//! header (a `Recovered` note for the skipped prefix), a buffer with
//! no sync candidate drains whole (one note), and end-of-stream
//! leftovers surface as a tail note — the hold is capped at one
//! partial frame + one PES payload (a legal hold can never exceed
//! it; beyond that the drain note fires).
//!
//! The split set is exactly the decodable MPEG-audio descriptors
//! (`mp1`/`mp2`/`mp3` — the demuxer's own peek classification).
//! ac3/dts/pcm_dvd keep the whole-PES shape: no decoder exists in
//! this build, so the container's PES truth is the honest packet and
//! the provider's typed refusal is the honest answer.
//!
//! # The harvest's decoder-config truth (the doc §4.3.3 rule; the
//! demuxer's [`stream_descriptors`](PsDemuxer::stream_descriptors)
//! accessor is the facts source the OPW-0 doc cites)
//!
//! * **The codec descriptor is the pre-scan's own classification**
//!   (`mpeg2video`/`mpeg1video` by the sequence-extension peek,
//!   `mp2`/`mp3`/`mp1` by the frame-header layer bits, `ac3`/`dts`/
//!   `pcm_dvd`/`dvd_subtitle` by the private-1 substream byte,
//!   `dvd_nav_packet` for private-2). The track list carries the
//!   same registry vocabulary — pinned equal by a debug assert (a
//!   divergence would be a demuxer bug, never a wiring guess).
//! * **No `extra_data` for any track**: mp2's config rides in-band
//!   in every frame header; mpeg2video's SPS rides in-band; the
//!   AC-3/DTS syncframes are self-describing; the DVD LPCM geometry
//!   rides the 6-byte header the demuxer strips and does not
//!   re-expose (the demuxer's own recorded v0 scope note — owed to
//!   a future PCM-DVD codec card). Seeding any blob here would be
//!   an invention.
//! * **No `pcm` pair** (same LPCM note): the mp2 track's geometry
//!   is codec-domain in-band, so the audio session opens limb-less
//!   exactly like raw ADTS/MP3 (the recorded family gap — the same
//!   mechanism, the same honesty).
//! * **A declared extent** (unlike TS): the pre-scan's per-stream
//!   first/last PTS ride the track spans, so the family HAS a
//!   `duration_us` to clamp `--start-time` against (the AVI shape).
//! * **No `frame_ns`, no `dimensions`**: cadence and geometry are
//!   in-band sequence-header truths; a container-level guess would
//!   be a lie.
//! * **`--start-time`**: the AVI shape — clamp against the declared
//!   extent, validate through the demuxer's pure `seek` query (it
//!   clamps to the pre-scanned SCR extent — the container's own
//!   clock), then the wrapper's skip watermark drops every packet
//!   that predates the target (unjudgeable unstamped packets pass —
//!   never dropped on a guess). No reanchor walk runs at open: the
//!   watermark is the bounded-budget behavior the plan's row names.
//! * **Seek**: [`SeekSupport::SlowLinear`] (no index — the demuxer's
//!   own declaration, forwarded by the wrapper, never invented):
//!   the native-seek lane asserts the seekability and the real
//!   skip, the contrast to the TS unit's typed `None` refusal.

use std::collections::VecDeque;

use commands::CommandError;
use demux::es::mp3::parse_head;
use demux::ps::PsDemuxer;
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeSpan, Timestamp};
use media_types::{EngineError, Packet, Track, TrackId};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// The MPEG-PS payload ceiling (the demuxer's own `MAX_PACKET_BYTES`
/// — a PES length is a u16, so no legal packet exceeds it). The
/// assembler's damage-hold cap rides the same bound: a legitimate
/// hold is one partial frame + one PES payload, so two ceilings of
/// slack can never clip a legal stream and always bound a damaged
/// one.
const HOLD_CAP_BYTES: usize = 2 * demux::ps::MAX_PACKET_BYTES as usize;

/// The damage-note reason for bytes skipped re-synchronizing the
/// MPEG-audio walk to the next frame header.
const REASON_RESYNC: &str = "ps.pes.mpeg-audio-resync";
/// The damage-note reason for a buffer with no judgeable frame
/// header at all (drained whole, bounded).
const REASON_DRAIN: &str = "ps.pes.mpeg-audio-drain";
/// The damage-note reason for end-of-stream leftovers that never
/// completed a frame.
const REASON_TAIL: &str = "ps.pes.mpeg-audio-tail";

/// Open an MPEG-PS/VOB source (the doc §4.2 per-container shape):
/// the probe already named `mpegps`; this constructs the demuxer,
/// runs the host pre-pass `start()` (the header-only structure
/// scan), harvests the discovered streams' facts, resolves
/// `--start-time` through the pure seek query (the AVI shape — the
/// family declares an extent) and wraps the demuxer + the
/// MPEG-audio assemblers in the shared selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): a non-PS stream the
/// probe misnamed, structural damage the pre-scan raises, and any
/// typed damage the walk raises later.
pub(crate) fn open(
    stream: Box<dyn plugin_api::SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + stream registry).
    let mut demuxer = PsDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    // 2. Clamp the start position against the declared extent (the
    //    pre-scan's per-stream PTS spans), then validate through the
    //    demuxer's pure seek query — it clamps to the pre-scanned
    //    SCR extent, the container's own clock. The wrapper's skip
    //    watermark (not a reanchor walk) does the positioning: the
    //    bounded-budget behavior the plan's row pins.
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
    let mut skip_until_ns = None;
    if start_us > 0 {
        let target = TimeSpan {
            start: Stamp::Present(Timestamp::new(
                start_us.saturating_mul(1000),
                media_clock::NANO_BASE,
            )),
            end: Stamp::Missing,
        };
        demuxer.seek(target).map_err(|err| wire_error(&err))?;
        skip_until_ns = Some(start_us.saturating_mul(1000));
    }
    // 3. The MPEG-audio assemblers: one per decodable MPEG-audio
    //    track (the demuxer's own peek classification — the split
    //    set is exactly the descriptors with wired decoders; the
    //    undecodable families keep the whole-PES container truth).
    let assemblers: Vec<(TrackId, MpegAudioAssembler)> = facts
        .facts
        .iter()
        .filter(|fact| is_mpeg_audio(fact.codec.as_str()))
        .map(|fact| (fact.id, MpegAudioAssembler::default()))
        .collect();
    let wrapper = SelectDemuxer::new(
        PsHostDemuxer {
            inner: demuxer,
            assemblers,
            queue: VecDeque::new(),
        },
        skip_until_ns,
        facts
            .facts
            .iter()
            .map(|fact| (fact.id, fact.base))
            .collect(),
        opts.no_video,
        opts.no_audio,
    );
    Ok(OpenedMedia {
        demuxer: Box::new(wrapper),
        facts,
        start_position_ns: start_us.saturating_mul(1000),
        uri: opts.uri.clone(),
    })
}

/// The decodable MPEG-audio descriptors (the split set): the
/// family's one-frame-per-packet decoder contract is the reason the
/// split exists; everything else keeps the PES shape.
fn is_mpeg_audio(codec: &str) -> bool {
    matches!(codec, "mp1" | "mp2" | "mp3")
}

/// Harvest the track facts from a started PS demuxer: one fact per
/// discovered stream, the codec from the pre-scan's own registry
/// vocabulary (the track's `codec` — pinned against the
/// `stream_descriptors` accessor the OPW-0 doc cites), the 90 kHz
/// MPEG clock as the timing base — and nothing the container does
/// not declare (no PCM pair, no blob, no cadence, no dimensions;
/// the LPCM geometry note lives in the module docs).
fn harvest_facts(demuxer: &PsDemuxer, tracks: &[Track]) -> MediaFacts {
    // The doc's cited accessor and the track list are the SAME
    // truth (both mint from the pre-scan's stream registry) — pinned
    // as the harvest's invariant; divergence would be a demuxer bug.
    let descriptors = demuxer.stream_descriptors();
    debug_assert_eq!(
        descriptors.len(),
        tracks.len(),
        "stream_descriptors parallels the track list"
    );
    for (track, (codec, kind)) in tracks.iter().zip(descriptors) {
        debug_assert_eq!(track.codec.as_str(), codec, "descriptor vocabulary parity");
        debug_assert_eq!(track.kind, kind, "track kind parity");
    }
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        // The demuxer's own build truth: every PS track rides the
        // 90 kHz MPEG clock. Pinned as the harvest's invariant —
        // divergence would mean the track spans and the PES stamps
        // disagree about one clock, a demuxer bug, never a wiring
        // guess.
        debug_assert_eq!(
            track.timing.base.as_parts(),
            (1, 90_000),
            "the PS plumbing clock is 90 kHz"
        );
        let span_end_ticks = track
            .timing
            .span
            .end
            .timestamp()
            .ok()
            .map(|stamp| stamp.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec: track.codec.as_str().to_string(),
            base: track.timing.base,
            span_end_ticks,
            pcm: None,
            extra_data: None,
            frame_ns: None,
            dimensions: None,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "mpegps".to_string(),
    }
}

// ---------------------------------------------------------------------------
// The MPEG-audio streaming frame assembler (see the module docs)
// ---------------------------------------------------------------------------

/// The streaming Layer I/II/III frame walk over one MPEG-audio
/// track's concatenated PES payload stream: holds the unconsumed
/// bytes (which always begin at a frame boundary or an unjudgeable
/// short tail), the stamp anchors of the PES that contributed the
/// byte at the walk position, and the sample count since that
/// anchor (the per-AU stamp arithmetic).
#[derive(Default)]
struct MpegAudioAssembler {
    /// Unconsumed payload bytes (front = the next frame boundary).
    buffer: Vec<u8>,
    /// The PTS of the PES contributing `buffer`'s walk position.
    anchor_pts: Option<Timestamp>,
    /// The DTS of the same PES (the same sample ladder).
    anchor_dts: Option<Timestamp>,
    /// Samples minted since the anchor (the stamp arithmetic rides
    /// on the first parsed frame's rate).
    samples: u64,
    /// A PES-marked discontinuity waits for the first AU minted
    /// after it (the boundary belongs to that AU).
    pending_discontinuity: bool,
}

/// One PES packet's payload into the walk: append, mint every frame
/// that completes, note damage bounded-and-never-silently. See the
/// module docs for the spanning-frame and stamp-anchor truths.
impl MpegAudioAssembler {
    fn push(&mut self, packet: Packet, queue: &mut VecDeque<DemuxEvent>) {
        let Packet {
            track,
            pts,
            dts,
            duration: _,
            data,
            keyframe,
            discontinuity,
            generation,
        } = packet;
        // The crossing rule: a frame belongs to the PES whose bytes
        // carry its FIRST byte. An arrival into an empty buffer
        // anchors immediately; otherwise the anchors switch once the
        // walk consumes past the pre-arrival bytes (`old_len`).
        let old_len = self.buffer.len();
        let mut crossed = old_len == 0;
        if crossed {
            self.anchor_pts = timestamp_of(pts);
            self.anchor_dts = timestamp_of(dts);
            self.samples = 0;
        }
        if discontinuity {
            self.pending_discontinuity = true;
        }
        self.buffer.extend_from_slice(data.as_slice());
        let mut at = 0usize;
        loop {
            let tail = &self.buffer[at..];
            if tail.len() < 4 {
                // A short tail cannot be judged — it may complete
                // with the next PES. Held (bounded by the cap below).
                break;
            }
            let Ok(head) = parse_head(tail) else {
                // Garbage at the walk position: the bounded resync.
                match resync_from(tail) {
                    Resync::Found(next) => {
                        queue.push_back(DemuxEvent::Recovered {
                            skipped: next as u64,
                            reason: REASON_RESYNC,
                        });
                        at += next;
                        continue;
                    }
                    // A sync pair too close to the end to judge: the
                    // whole tail holds until more bytes arrive (the
                    // cap below bounds the pathological case).
                    Resync::MaybeTail if self.buffer.len() - at <= HOLD_CAP_BYTES => break,
                    Resync::MaybeTail | Resync::Drain => {
                        queue.push_back(DemuxEvent::Recovered {
                            skipped: (self.buffer.len() - at) as u64,
                            reason: REASON_DRAIN,
                        });
                        self.buffer.truncate(at);
                        break;
                    }
                }
            };
            let end = at.saturating_add(usize::try_from(head.frame_size).unwrap_or(usize::MAX));
            if end > self.buffer.len() {
                // A parsing but incomplete frame: the spanning hold.
                break;
            }
            // Mint: the anchor + exact rational sample arithmetic.
            let rate = head.sample_rate.max(1);
            let add = ticks_for(self.samples, rate);
            let add = i64::try_from(add).unwrap_or(i64::MAX);
            let au_pts = self
                .anchor_pts
                .map(|anchor| {
                    Stamp::Present(Timestamp::new(
                        anchor.ticks.saturating_add(add),
                        anchor.base,
                    ))
                })
                .unwrap_or(Stamp::Missing);
            let au_dts = self
                .anchor_dts
                .map(|anchor| {
                    Stamp::Present(Timestamp::new(
                        anchor.ticks.saturating_add(add),
                        anchor.base,
                    ))
                })
                .unwrap_or(Stamp::Missing);
            let duration_ns = u64::try_from(
                u128::from(head.samples_per_frame) * 1_000_000_000 / u128::from(rate),
            )
            .unwrap_or(u64::MAX);
            let discontinuity = self.pending_discontinuity;
            self.pending_discontinuity = false;
            queue.push_back(DemuxEvent::Packet(Packet {
                track,
                pts: au_pts,
                dts: au_dts,
                duration: Some(Duration::Known(Nanoseconds(duration_ns))),
                data: SharedBytes::from_vec(self.buffer[at..end].to_vec()),
                keyframe,
                discontinuity,
                generation,
            }));
            self.samples = self
                .samples
                .saturating_add(u64::from(head.samples_per_frame));
            at = end;
            if !crossed && at >= old_len {
                // The walk entered this PES's own bytes: it anchors
                // everything from here.
                self.anchor_pts = timestamp_of(pts);
                self.anchor_dts = timestamp_of(dts);
                self.samples = 0;
                crossed = true;
            }
        }
        self.buffer.drain(..at);
    }

    /// The end-of-stream flush: leftovers that never completed a
    /// frame are one tail note — counted, never silently dropped.
    fn flush(&mut self, queue: &mut VecDeque<DemuxEvent>) {
        if !self.buffer.is_empty() {
            queue.push_back(DemuxEvent::Recovered {
                skipped: self.buffer.len() as u64,
                reason: REASON_TAIL,
            });
            self.buffer.clear();
        }
    }
}

/// A stamp's timestamp, when present.
fn timestamp_of(stamp: Stamp) -> Option<Timestamp> {
    match stamp {
        Stamp::Present(ts) => Some(ts),
        Stamp::Missing => None,
    }
}

/// The per-AU tick advance: `round(samples × 90 000 / rate)` — u128
/// intermediate, saturating, never a float (the TS unit's pinned
/// arithmetic).
fn ticks_for(samples: u64, rate: u32) -> u64 {
    let rate = u64::from(rate.max(1));
    u64::try_from((u128::from(samples) * 90_000 + u128::from(rate / 2)) / u128::from(rate))
        .unwrap_or(u64::MAX)
}

/// The bounded re-synchronization scan for the garbage case: the
/// next judgeable frame header in the buffer, a sync pair too close
/// to the end to judge (hold), or nothing (drain).
enum Resync {
    /// A frame header at this offset — the skipped prefix is damage.
    Found(usize),
    /// A sync pair inside the last 3 bytes: cannot judge yet.
    MaybeTail,
    /// No candidate at all.
    Drain,
}

fn resync_from(buffer: &[u8]) -> Resync {
    let len = buffer.len();
    let mut i = 1;
    while i + 1 < len {
        if buffer[i] == 0xFF && buffer[i + 1] & 0xE0 == 0xE0 {
            if i + 4 <= len {
                if parse_head(&buffer[i..]).is_ok() {
                    return Resync::Found(i);
                }
            } else {
                return Resync::MaybeTail;
            }
        }
        i += 1;
    }
    Resync::Drain
}

// ---------------------------------------------------------------------------
// The host demuxer (the walk + the assemblers, one event stream)
// ---------------------------------------------------------------------------

/// The started PS demuxer plus its MPEG-audio assemblers behind the
/// wrapper seam: `start()` answers the pre-pass's cached list (the
/// demuxer's own idempotence), `next_event` interleaves the
/// demuxer's events with the assemblers' minted AU packets
/// (queue-first — the demuxer queued its own events before the
/// assembler minted packets for the same PES, so the internal
/// sequence is preserved), and seek answers stay the demuxer's own
/// (the clamping pure query, `SlowLinear`, the compute-then-commit
/// reanchor — forwarded, never invented).
struct PsHostDemuxer {
    inner: PsDemuxer,
    assemblers: Vec<(TrackId, MpegAudioAssembler)>,
    queue: VecDeque<DemuxEvent>,
}

impl Demuxer for PsHostDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        // The demuxer's own idempotence answers the cached list
        // (the pre-pass's truth, re-served to the engine).
        self.inner.start()
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        loop {
            // 1. The assemblers' minted packets first (the internal
            //    order — see the struct docs).
            if let Some(event) = self.queue.pop_front() {
                return Ok(event);
            }
            match self.inner.next_event()? {
                DemuxEvent::Packet(packet) => {
                    let track = packet.track;
                    match self.assemblers.iter_mut().find(|(id, _)| *id == track) {
                        Some((_, assembler)) => {
                            assembler.push(packet, &mut self.queue);
                        }
                        // Every non-MPEG-audio family keeps the
                        // whole-PES container truth.
                        None => return Ok(DemuxEvent::Packet(packet)),
                    }
                }
                DemuxEvent::EndOfStream => {
                    // 2. The flush: leftovers become tail notes
                    //    before the honest end.
                    for (_, assembler) in &mut self.assemblers {
                        assembler.flush(&mut self.queue);
                    }
                    return Ok(DemuxEvent::EndOfStream);
                }
                event => return Ok(event),
            }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        self.inner.seek(target)
    }

    fn seekability(&self) -> SeekSupport {
        self.inner.seekability()
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The demuxer's compute-then-commit walk (contract D4) —
        // the assemblers' streaming state stays intact across it:
        // a reanchor marks the next packet discontinuous and the
        // walk resumes at a pack boundary, so the next PES pushes
        // into a still-valid buffer (a mid-frame cut at the landing
        // holds exactly like a PES-boundary cut — the assembler
        // neither knows nor needs to know).
        self.inner.reanchor(target)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demux_host::open_source;
    use crate::pipeline_host::HostDecoderProvider;
    use codec_api::{Decoder, DecoderConfig, FrameSink, ThreadBudget};
    use commands::catalog::types::SourceSpec;
    use media_types::{Frame, GenerationId, MediaKind};
    use playback_engine::DecoderProvider;
    use testkit::Must;

    /// The audio sink (the adts.rs/ts.rs convention): counts decoded
    /// sample frames and remembers the negotiated channel count.
    struct CountingSink(u64, u32);
    impl FrameSink for CountingSink {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            if let Frame::Audio(audio) = frame
                && let Some(frames) = audio.format.frames_in(audio.samples.len())
            {
                self.0 += frames;
                self.1 = audio.format.channels;
            }
            Ok(())
        }
    }

    /// A committed demux-crate PS fixture (the family has NO corpus
    /// lanes — the census truth; these four files are the lanes).
    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/demux/fixtures/ps")
            .join(name)
    }

    /// Open a source through the REAL CLI open path (canonicalize →
    /// access → probe → this container host).
    fn open(path: std::path::PathBuf, start_us: Option<i64>) -> OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// The real provider's decoder for one track (typed refusals
    /// RETURNED, never panicked — the lanes pin them by code).
    fn decoder_for(opened: &OpenedMedia, track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
        let facts = std::sync::Arc::new(opened.facts.clone());
        let provider = HostDecoderProvider::new(facts);
        let config = DecoderConfig::new(
            track.codec.clone(),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        );
        provider.decoder_for(track, &config)
    }

    /// One 208-byte Layer II frame SHAPE (MPEG-1, 64 kbps, 44.1 kHz,
    /// no padding — `144 × 64 000 / 44 100 = 208`, 1 152 samples):
    /// framing only, nothing decodes it (the fields mirror
    /// `parse_head`'s own grammar — the demux crate's test-support
    /// convention).
    fn mp2_frame_208(fill: u8) -> Vec<u8> {
        let mut frame = vec![fill; 208];
        frame[0] = 0xFF;
        frame[1] = 0xFD; // MPEG-1, Layer II, no CRC
        frame[2] = 0x40; // bitrate idx 4 (64 kbps), rate idx 0 (44100), no pad
        frame[3] = 0xC4; // mono, mode extension, no copyright/original emphasis
        frame
    }

    /// A temp dir that removes itself (the probe_io/ts.rs
    /// convention).
    struct TempDir(std::path::PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let dir =
                std::env::temp_dir().join(format!("lumen-opw-ps-{tag}-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
            std::fs::create_dir_all(&dir).must();
            TempDir(dir)
        }
        fn write(&self, name: &str, bytes: &[u8]) -> std::path::PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).must();
            path
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The synthetic lane: a crafted audio-only PS whose SECOND
    /// frame SPANS the PES boundary (the measured muxer behavior the
    /// assembler exists for) with the second PES UNSTAMPED (the
    /// anchor truth: the spanning frame belongs to the PES that
    /// started it), a garbage tail after the last frame (the bounded
    /// drain), and the SlowLinear seek truth.
    #[test]
    fn synthetic_lane_splits_spanning_frames_and_drains_typed() {
        use demux::ps::test_support::{mpeg1_pack, mpeg1_pes, program_end};
        let frame1 = mp2_frame_208(0x11);
        let frame2 = mp2_frame_208(0x22);
        // PES A: frame 1 whole + the first 100 bytes of frame 2
        // (frame 2 spans into PES B).
        let mut pes_a_payload = frame1.clone();
        pes_a_payload.extend_from_slice(&frame2[..100]);
        // PES B (unstamped): the remaining 108 bytes of frame 2 + a
        // 30-byte garbage tail (no sync candidate — the drain).
        let mut pes_b_payload = frame2[100..].to_vec();
        pes_b_payload.extend_from_slice(&[0x00u8; 30]);
        let mut stream = Vec::new();
        stream.extend_from_slice(&mpeg1_pack(0, 250_000));
        stream.extend_from_slice(&mpeg1_pack(90_000, 250_000));
        stream.extend_from_slice(&mpeg1_pes(0xC0, &pes_a_payload, Some(9_000)));
        stream.extend_from_slice(&mpeg1_pes(0xC0, &pes_b_payload, None));
        stream.extend_from_slice(&program_end());
        let dir = TempDir::new("synthetic");
        let path = dir.write("synthetic.mpg", &stream);
        let mut opened = open(path, None);
        // Facts: one mp2 audio track, the 90 kHz clock, the stamped
        // PES's span, nothing the container does not declare.
        assert_eq!(opened.facts.container, "mpegps");
        assert_eq!(opened.facts.tracks.len(), 1);
        let track = opened.facts.tracks[0].clone();
        assert_eq!(track.kind, MediaKind::Audio);
        assert_eq!(track.codec.as_str(), "mp2", "the peek classification");
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.base.as_parts(), (1, 90_000), "the MPEG clock");
        assert_eq!(fact.span_end_ticks, Some(9_000), "the one stamped PES");
        assert!(fact.pcm.is_none(), "no PCM pair — the LPCM note's mirror");
        assert!(fact.extra_data.is_none(), "mp2's config rides in-band");
        assert!(fact.frame_ns.is_none(), "cadence is codec-domain");
        assert!(fact.dimensions.is_none(), "geometry is codec-domain");
        assert!(
            fact.audio_geometry().is_none(),
            "no geometry-bearing blob — limb-less, the raw-MP3 family gap"
        );
        assert_eq!(opened.start_position_ns, 0);
        // The walk: TWO AU packets (the spanning frame completed from
        // PES B under PES A's anchor — the second stamp is the
        // anchor + round(1 152 × 90 000 / 44 100) = +2 351), then
        // the 30-byte garbage drain note, then the end.
        let mut events = Vec::new();
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let pts = match packet.pts {
                        Stamp::Present(stamp) => stamp.ticks,
                        Stamp::Missing => unreachable!("both AUs carry the anchor arithmetic"),
                    };
                    events.push(format!("packet {}B pts {}", packet.data.len(), pts));
                    assert_eq!(packet.data.len(), 208, "one whole frame per AU");
                    assert_eq!(packet.track, track.id);
                    assert_eq!(
                        packet.duration,
                        Some(Duration::Known(Nanoseconds(26_122_448))),
                        "1152 samples @ 44.1 kHz (floored ns)"
                    );
                }
                Ok(DemuxEvent::Recovered { skipped, reason }) => {
                    events.push(format!("recovered {skipped} {reason}"));
                }
                Ok(DemuxEvent::Discontinuity) => events.push("discontinuity".to_string()),
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        assert_eq!(
            events,
            vec![
                "packet 208B pts 9000".to_string(),
                "packet 208B pts 11351".to_string(),
                format!("recovered 30 {REASON_DRAIN}"),
            ],
            "the spanning split + the bounded drain"
        );
        // The seek truth: SlowLinear (forwarded), the pure query
        // clamps to the pre-scanned SCR extent (two packs, SCRs 0
        // and 90 000 — a past-the-clock target lands on the last).
        assert_eq!(opened.demuxer.seekability(), SeekSupport::SlowLinear);
        let asked = TimeSpan {
            start: Stamp::Present(Timestamp::new(2_000_000_000, media_clock::NANO_BASE)),
            end: Stamp::Missing,
        };
        match opened.demuxer.seek(asked) {
            Ok(landed) => match landed.start {
                Stamp::Present(stamp) => {
                    assert_eq!(stamp.ticks, 90_000, "clamped to the last SCR");
                    assert_eq!(stamp.base.as_parts(), (1, 90_000), "the PS clock");
                }
                Stamp::Missing => unreachable!("the clamp answers a position"),
            },
            Err(err) => unreachable!("the pure query clamps, never refuses here: {err}"),
        }
    }

    /// The MPG lane (the family's decode truth): the committed
    /// fixture opens through the REAL open path with the measured
    /// facts; the mp2 track's 5 PES (8 150 bytes, 4 of the 39 frames
    /// SPANNING boundaries) re-packetize to 39 AU packets whose
    /// stamp ladder ends 136 957 == ffprobe's last synthesized row;
    /// the FULL mp2 stream decodes 44 928 sample frames == ffmpeg
    /// (measured this session: 89 856 s16le bytes); mpeg2video
    /// refuses decode typed at the provider.
    #[test]
    fn mpg_lane_demuxes_splits_and_decodes_mp2_exactly() {
        let mut opened = open(fixture("ps-mp2-video-audio.mpg"), None);
        assert_eq!(opened.facts.container, "mpegps");
        assert_eq!(opened.facts.tracks.len(), 2);
        let (video, audio) = (
            opened.facts.tracks[0].clone(),
            opened.facts.tracks[1].clone(),
        );
        assert_eq!(video.kind, MediaKind::Video);
        assert_eq!(video.codec.as_str(), "mpeg2video");
        assert_eq!(audio.kind, MediaKind::Audio);
        assert_eq!(audio.codec.as_str(), "mp2");
        let (video_fact, audio_fact) = (&opened.facts.facts[0], &opened.facts.facts[1]);
        assert_eq!(
            video_fact.span_end_ticks,
            Some(135_000),
            "the demux suite's pin"
        );
        assert_eq!(
            audio_fact.span_end_ticks,
            Some(118_149),
            "the last STAMPED PES"
        );
        for fact in &[video_fact, audio_fact] {
            assert_eq!(fact.base.as_parts(), (1, 90_000));
            assert!(fact.pcm.is_none() && fact.extra_data.is_none());
            assert!(fact.frame_ns.is_none() && fact.dimensions.is_none());
        }
        // The walk: 32 whole video PES (container truth — the
        // demuxer's recorded design) + 39 mp2 AUs (the split; ==
        // ffprobe's nb_read_packets for the same stream) + the two
        // padding-stream damage notes (never silent).
        let mut per_track = [0u64, 0u64];
        let mut audio_stamps = Vec::new();
        let mut notes = Vec::new();
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let at = if packet.track == video.id { 0 } else { 1 };
                    per_track[at] += 1;
                    if at == 1
                        && let Stamp::Present(stamp) = packet.pts
                    {
                        audio_stamps.push(stamp.ticks);
                    }
                }
                Ok(DemuxEvent::Recovered { skipped, reason }) => notes.push((skipped, reason)),
                Ok(DemuxEvent::Discontinuity) => {
                    unreachable!("the clean fixture carries no discontinuity")
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        assert_eq!(per_track[0], 32, "video PES (the container truth)");
        assert_eq!(per_track[1], 39, "mp2 AUs (== ffprobe's packet count)");
        assert_eq!(
            notes,
            vec![(1114u64, "padding-stream"), (2009, "padding-stream")],
            "the two padding PES, damage-noted"
        );
        // The stamp ladder (measured): the anchor + arithmetic
        // series — 47 618 … 136 957, the PES anchors 71 129 /
        // 94 639 / 118 149 riding ON the ladder (the wire values,
        // each ±1 of the pure 2 351.02 ladder — the honest
        // container-truth wobble; ffprobe's own rows are the same
        // arithmetic accumulated per AU: first and last match
        // exactly, the recorded ±1-tick intermediate divergence).
        assert_eq!(
            audio_stamps,
            vec![
                47_618, 49_969, 52_320, 54_671, 57_022, 59_373, 61_724, 64_075, 66_426, 68_777,
                71_129, 73_480, 75_831, 78_182, 80_533, 82_884, 85_235, 87_586, 89_937, 92_288,
                94_639, 96_990, 99_341, 101_692, 104_043, 106_394, 108_745, 111_096, 113_447,
                115_798, 118_149, 120_500, 122_851, 125_202, 127_553, 129_904, 132_255, 134_606,
                136_957
            ],
            "the anchor + exact rational sample arithmetic"
        );
        // The codec truth: mpeg2video has no software decoder in
        // this build — the typed decode-stage refusal.
        match decoder_for(&opened, &video) {
            Err(err) => {
                assert_eq!(err.context.code.as_str(), "cli.codec_unavailable", "{err}")
            }
            Ok(_) => unreachable!("no mpeg2video decoder exists in this build"),
        }
        // The decode pass: the FULL mp2 stream through the real
        // provider (fresh single-shot walk).
        let mut lane = open(fixture("ps-mp2-video-audio.mpg"), None);
        let facts = std::sync::Arc::new(lane.facts.clone());
        let provider = HostDecoderProvider::new(std::sync::Arc::clone(&facts));
        let audio = facts.tracks[1].clone();
        let config = DecoderConfig::new(
            audio.codec.clone(),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        );
        let mut decoder = provider
            .decoder_for(&audio, &config)
            .unwrap_or_else(|err| unreachable!("mp2 decoder_for: {err}"));
        let mut sink = CountingSink(0, 0);
        loop {
            match lane.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if packet.track == audio.id {
                        let _ = decoder
                            .decode(packet, &mut sink)
                            .unwrap_or_else(|err| unreachable!("mp2 decode: {err}"));
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("decode-pass demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("mp2 flush: {err}"));
        // Measured this session: ffmpeg decodes the same stream to
        // 89 856 s16le bytes = 44 928 mono sample frames (39 AUs ×
        // 1 152). The streaming split matches it exactly.
        assert_eq!(sink.0, 44_928, "mp2 decoded sample frames (== ffmpeg)");
        assert_eq!(sink.1, 1, "mp2 mono output");
    }

    /// The three VOB lanes (the undecodable family truth): each
    /// opens with the nav Data track deselected from the engine's
    /// view (packets still flow — the engine drops them at the
    /// unwired-track boundary), the video/audio PES counts match the
    /// demux suite's pins, and every track's codec refuses decode
    /// typed (`cli.codec_unavailable`) — mpeg2video, ac3, dts and
    /// pcm_dvd have no decoder in this build (the LPCM geometry
    /// rides the stripped DVD header: the demuxer's own recorded
    /// v0 scope note — the refusal is the honest answer, not a
    /// misclassification).
    #[test]
    fn vob_lanes_demux_and_refuse_undecodable_codecs_typed() {
        // (file, audio descriptor, audio PES count, audio last PTS
        // in 90 kHz ticks — the demux suite's measured pins).
        let lanes: [(&str, &str, u64, i64); 3] = [
            ("ps-vob-ac3.vob", "ac3", 7, 126_034),
            ("ps-vob-dts.vob", "dts", 24, 136_114),
            ("ps-vob-lpcm.vob", "pcm_dvd", 48, 138_583),
        ];
        for (file, audio_codec, audio_packets, audio_last) in lanes {
            let mut opened = open(fixture(file), None);
            assert_eq!(opened.facts.container, "mpegps", "{file}");
            assert_eq!(opened.facts.tracks.len(), 3, "{file}");
            let (nav, video, audio) = (
                opened.facts.tracks[0].clone(),
                opened.facts.tracks[1].clone(),
                opened.facts.tracks[2].clone(),
            );
            assert_eq!(nav.kind, MediaKind::Data, "{file}: the DVD nav track");
            assert_eq!(nav.codec.as_str(), "dvd_nav_packet", "{file}");
            assert_eq!(video.codec.as_str(), "mpeg2video", "{file}");
            assert_eq!(audio.codec.as_str(), audio_codec, "{file}");
            // The engine's view: the nav Data track deselects at
            // open (the subtitle/data convention — §3).
            let engine_tracks = opened
                .demuxer
                .start()
                .unwrap_or_else(|err| unreachable!("{file}: wrapper start: {err}"));
            assert_eq!(engine_tracks.len(), 2, "{file}: the nav track deselects");
            // The walk: the demux suite's pinned PES counts + the
            // three padding-stream damage notes (measured).
            let mut per_track = [0u64; 3];
            let mut notes = 0u64;
            loop {
                match opened.demuxer.next_event() {
                    Ok(DemuxEvent::Packet(packet)) => {
                        let at = if packet.track == nav.id {
                            0
                        } else if packet.track == video.id {
                            1
                        } else {
                            2
                        };
                        per_track[at] += 1;
                    }
                    Ok(DemuxEvent::Recovered { reason, .. }) => {
                        assert_eq!(reason, "padding-stream", "{file}: only padding notes");
                        notes += 1;
                    }
                    Ok(DemuxEvent::Discontinuity) => {
                        unreachable!("{file}: no discontinuity on a clean lane")
                    }
                    Ok(DemuxEvent::EndOfStream) => break,
                    Err(err) => unreachable!("{file}: {err}"),
                }
            }
            assert_eq!(per_track[0], 4, "{file}: nav PES (container truth)");
            assert_eq!(per_track[1], 68, "{file}: video PES");
            assert_eq!(
                per_track[2], audio_packets,
                "{file}: audio PES (whole — no decoder)"
            );
            assert_eq!(notes, 3, "{file}: the three padding notes");
            // The span pins (the pre-scan's own truth).
            assert_eq!(
                opened.facts.facts[1].span_end_ticks,
                Some(135_090),
                "{file}: video span end"
            );
            assert_eq!(
                opened.facts.facts[2].span_end_ticks,
                Some(audio_last),
                "{file}: audio span end (the last STAMPED PES)"
            );
            assert_eq!(opened.facts.facts[2].base.as_parts(), (1, 90_000), "{file}");
            // Every track's codec refuses decode typed — the honest
            // initial state (the mpeg2/ac3/dts cards are future work).
            // pcm_dvd lands in the provider's PCM arm, whose own
            // honest answer for a track with no PCM facts is
            // `cli.pcm_facts_absent`: the DVD LPCM geometry rides the
            // 6-byte header the demuxer strips (its recorded v0
            // scope note — owed to a future PCM-DVD codec card), so
            // the harvest carries no pair and the arm refuses typed.
            for track in [video.clone(), audio.clone()] {
                match decoder_for(&opened, &track) {
                    Err(err) => assert_eq!(
                        err.context.code.as_str(),
                        if track.codec.as_str() == "pcm_dvd" {
                            "cli.pcm_facts_absent"
                        } else {
                            "cli.codec_unavailable"
                        },
                        "{file}: {err}"
                    ),
                    Ok(_) => {
                        unreachable!("{file}: no {} decoder exists in this build", track.codec)
                    }
                }
            }
        }
    }

    /// `--start-time` on the SlowLinear family (the TS unit's
    /// contrast): the family IS seekable and declares an extent, so
    /// the target validates through the pure seek query and the
    /// wrapper's skip watermark does a REAL skip — every surviving
    /// stamped packet lands at/after the target (measured on the
    /// MPG lane at 0.6 s = 54 000 ticks: the 2 early-stamped video
    /// PES and the 3 early AUs drop; the 12 unstamped video PES
    /// pass — unjudgeable stamps are never dropped on a guess).
    #[test]
    fn start_time_skips_packets_with_slow_linear_seekability() {
        let mut opened = open(fixture("ps-mp2-video-audio.mpg"), Some(600_000));
        assert_eq!(opened.start_position_ns, 600_000_000);
        assert_eq!(
            opened.demuxer.seekability(),
            SeekSupport::SlowLinear,
            "no index — the demuxer's own declaration, forwarded"
        );
        let video = opened.facts.tracks[0].clone();
        let mut per_track = [0u64, 0u64];
        let mut first_seen = [Option::<i64>::None, Option::<i64>::None];
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let at = if packet.track == video.id { 0 } else { 1 };
                    per_track[at] += 1;
                    if let Stamp::Present(stamp) = packet.pts {
                        // The watermark: every judged stamp is at or
                        // after the 0.6 s target (54 000 ticks).
                        assert!(stamp.ticks >= 54_000, "early packet: {} ticks", stamp.ticks);
                        if first_seen[at].is_none() {
                            first_seen[at] = Some(stamp.ticks);
                        }
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) => {}
                Ok(DemuxEvent::Discontinuity) => {
                    unreachable!("no discontinuity on the clean lane")
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        // Measured: 32 video PES − 2 early-stamped = 30 kept (the
        // 12 unstamped B-frame PES pass); 39 AUs − 3 early = 36
        // kept. The first survivors: video 55 800, AU 54 671
        // (the ladder 47 618 + 3 × 2 351).
        assert_eq!(per_track[0], 30, "video kept (2 stamped-early dropped)");
        assert_eq!(per_track[1], 36, "audio kept (3 early AUs dropped)");
        assert_eq!(first_seen[0], Some(55_800), "first kept video stamp");
        assert_eq!(first_seen[1], Some(54_671), "first kept AU stamp");
    }
}
