//! The MPEG-TS container host (card 6.OPW-ts — the registry's
//! seventh wired family, the ISO/IEC 13818-1 transport): construct
//! the demuxer, run the host pre-pass `start()` (the bounded PSI
//! scan — PAT/PMT/SDT + the program surface), harvest every
//! elementary stream's facts into [`TrackFact`]s, wire the PES
//! assembly seam, and wrap in the shared
//! [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! # The PES seam (this family's distinguishing wiring)
//!
//! [`TsDemuxer`] emits **no packets by itself**: its elementary
//! payloads flow through the
//! [`PesConsumer`](demux::ts::PesConsumer) seam (card 5.I1's split —
//! the transport/PSI half in the demuxer crate, the assembly half
//! behind the plug point). This host registers [`PesBridge`]: a
//! [`PesAssembler`](demux::ts::pes::PesAssembler) fed verbatim by
//! the seam, minting engine [`Packet`]s per completed PES into a
//! queue that [`TsHostDemuxer::next_event`] drains in the walk's
//! own order (the demuxer queued its events before the seam minted
//! packets for the same transport packet, so queue-first preserves
//! the internal sequence). The end-of-stream flush mints the
//! trailing unbounded (video) PES; a bounded PES short of its
//! declared length surfaces the assembler's typed truncation —
//! never a silent drop. Assembly damage and timing anomalies are
//! forwarded as `Recovered`/`Discontinuity` events (the assembler's
//! own ledgers — the narrow seam would otherwise swallow them).
//!
//! The harvest's decoder-config truth (the doc §4.3.3 rule; the
//! demuxer's [`programs`](demux::ts::TsDemuxer::programs) /
//! [`track_of_pid`](demux::ts::TsDemuxer::track_of_pid) /
//! [`TsStream::descriptor`](demux::ts::TsStream::descriptor)
//! accessors are the facts source — the OPW-0 doc's own citation):
//!
//! * **The codec descriptor is the PMT `stream_type`'s registry
//!   vocabulary** (`h264`, `hevc`, `aac`, `mp3`, `mpeg2video`, … —
//!   the track's own `codec`, minted by the demuxer from the same
//!   PMT walk; unknown types pass through as `ts-stream-0xNN`,
//!   honest unknowns). h264/hevc/mpeg2 video refuses decode typed
//!   at the provider (`cli.codec_unavailable` — no software
//!   decoder in this build); `aac` (0x0F/0x11, ADTS-framed in PES)
//!   dispatches the AAC-LC decoder, which builds its engine from
//!   the first frame's in-band header (the raw-ADTS precedent).
//! * **No `extra_data` for any wired codec**: AAC-in-TS is
//!   ADTS-framed (the config rides in every frame — a synthetic
//!   blob would misframe the decoder), h264/hevc-in-TS is Annex-B
//!   (SPS/PPS ride in-band; the `avcC`/`hvcC` records are the MP4
//!   family's seam), and the PMT's verbatim ES descriptor bytes
//!   stay demuxer-side (`TsStream::es_info`, kept for the decode
//!   seam — no decoder in this build consumes them).
//! * **No `pcm` pair**: the TS vocabulary has no PCM codec; the
//!   aac track's geometry is codec-domain (in-band), so the family
//!   opens limb-less exactly like raw ADTS (the recorded gap of
//!   that unit — same mechanism, same honesty).
//! * **No declared extent**: the PSI carries no duration — TS
//!   timing exists only post-walk (the track spans stay
//!   `TimeSpan::UNKNOWN`), so there is nothing to clamp
//!   `--start-time` against.
//! * **No `frame_ns`, no `dimensions`**: cadence and geometry are
//!   codec-domain in-band truths (the SPS/VPS payload — e.g.
//!   `0000-param-midchange.ts` re-declares its parameters there);
//!   a container-level guess would be a lie.
//! * **`--start-time`**: with no declared extent AND
//!   [`SeekSupport::None`], the demuxer's pure seek query answers
//!   the only honest thing it can — the typed
//!   `demux.ts.seek.unsupported` refusal — and that refusal IS the
//!   open path's answer (propagated verbatim). The skip watermark
//!   is never set on a family that cannot position: faking a
//!   mid-stream start by silently filtering packets would claim a
//!   placement nothing verified.
//! * **Seek**: `SeekSupport::None` (ts/mod.rs — the demuxer's own
//!   declaration, forwarded by the wrapper, never invented): the
//!   native-seek lane expects the typed refusal, exactly as the
//!   plan row pins.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use commands::CommandError;
use demux::adts::AdtsHeader;
use demux::ts::pes::{PesAssembler, PesPacket, pts_stamp};
use demux::ts::{PesConsumer, TsDemuxer};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeSpan};
use media_types::{EngineError, ErrorKind, Packet, Track, TrackId};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open an MPEG-TS source (the doc §4.2 per-container shape): the
/// probe already named `mpegts`; this constructs the demuxer, runs
/// the host pre-pass `start()` (the PSI scan), harvestes the
/// program surface's facts, wires the PES bridge and wraps the
/// demuxer + bridge in the shared selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): no PAT within the scan
/// window, an unrecognizable stream, `--start-time` on the
/// not-seekable family (the pure query's own refusal), and any
/// typed damage the walk raises.
pub(crate) fn open(
    stream: Box<dyn plugin_api::SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + program surface).
    let mut demuxer = TsDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&tracks);
    // 2. The start position: TS declares no extent (the PSI carries
    //    no duration) and no seekability — the pure seek query's
    //    typed refusal is the honest answer for any nonzero target
    //    (see the module docs: the watermark never fakes a place).
    let start_us = opts.start_time_us.unwrap_or(0).max(0);
    let mut skip_until_ns = None;
    if start_us > 0 {
        let target = TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(
                start_us.saturating_mul(1000),
                media_clock::NANO_BASE,
            )),
            end: Stamp::Missing,
        };
        demuxer.seek(target).map_err(|err| wire_error(&err))?;
        // Unreachable today (the query refuses), kept for the day
        // the demuxer grows seekability: the watermark is the
        // query's own promise then.
        skip_until_ns = Some(start_us.saturating_mul(1000));
    }
    // 3. The PES bridge: routing is the demuxer's own pid→track
    //    table (`track_of_pid` over every declared ES pid — the
    //    facts source the OPW-0 doc cites), the assembler the
    //    shared queue behind the seam. The AU-split set is the
    //    ADTS-framed aac family (PMT 0x0F/0x11): their PES
    //    payloads carry several ADTS frames each, and the decoder
    //    contract is one frame per packet (see `mint_adts_aus`).
    let mut routing: HashMap<u16, TrackId> = HashMap::new();
    let mut au_split: Vec<u16> = Vec::new();
    for program in demuxer.programs() {
        for stream in &program.streams {
            if let Some(track) = demuxer.track_of_pid(stream.pid) {
                routing.insert(stream.pid, track);
                if stream.descriptor().as_str() == "aac" {
                    au_split.push(stream.pid);
                }
            }
        }
    }
    let bridge = Arc::new(Mutex::new(PesBridge::new(routing, au_split)));
    demuxer.set_pes_consumer(Box::new(BridgeConsumer {
        bridge: Arc::clone(&bridge),
    }));
    let wrapper = SelectDemuxer::new(
        TsHostDemuxer {
            inner: demuxer,
            bridge,
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

/// Harvest the track facts from a started TS demuxer: one fact per
/// declared elementary stream, the codec from the PMT's own
/// registry vocabulary (the track's `codec`, minted by the
/// demuxer's `descriptor()`), the MPEG clock as the timing base —
/// and nothing the container does not declare (no PCM, no blob, no
/// extent, no cadence, no dimensions; see the module docs).
fn harvest_facts(tracks: &[Track]) -> MediaFacts {
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        // The demuxer's own build truth: every TS track rides the
        // 90 kHz MPEG clock (`mpeg_time_base`). Pinned as the
        // harvest's invariant — divergence would mean the track
        // and the PES stamps disagree about one clock, a demuxer
        // bug, never a wiring guess.
        debug_assert_eq!(
            track.timing.base.as_parts(),
            (1, 90_000),
            "the TS plumbing clock is 90 kHz"
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
        container: "mpegts".to_string(),
    }
}

// ---------------------------------------------------------------------------
// The PES bridge (the seam's assembly half — see the module docs)
// ---------------------------------------------------------------------------

/// The shared assembly state behind the seam: the assembler, the
/// demuxer's pid→track routing, the ADTS-AU-split pids, and the
/// queue of minted events the host demuxer drains. Shared because
/// the demuxer owns the consumer box while the wrapper owns the walk.
struct PesBridge {
    assembler: PesAssembler,
    routing: HashMap<u16, TrackId>,
    au_split: Vec<u16>,
    queue: VecDeque<DemuxEvent>,
}

impl PesBridge {
    fn new(routing: HashMap<u16, TrackId>, au_split: Vec<u16>) -> PesBridge {
        PesBridge {
            assembler: PesAssembler::new(),
            routing,
            au_split,
            queue: VecDeque::new(),
        }
    }

    /// One transport packet's PES payload (the seam's verbatim
    /// slice). The transport `discontinuity_indicator` is the
    /// demuxer's own `Discontinuity` event (it owns the
    /// adaptation-field walk) — the seam passes payload only, so
    /// the assembler hears `false` and no duplicate event is
    /// minted here.
    fn push(&mut self, pid: u16, pusi: bool, payload: &[u8]) -> Result<(), EngineError> {
        self.assembler.push_packet(pid, pusi, false, payload)?;
        self.drain()
    }

    /// The end-of-stream flush: the trailing unbounded (video) PES
    /// become final packets; a bounded PES short of its declared
    /// length is the assembler's typed truncation (raised verbatim
    /// — calling again re-raises it identically while the short
    /// buffer stays put: the naturally-sticky damage shape).
    fn finish(&mut self) -> Result<(), EngineError> {
        self.assembler.finish()?;
        self.drain()
    }

    /// Move everything the assembler completed into the event
    /// queue: packets first (in completion order), then the damage
    /// and timing ledgers — never silent, never invented.
    fn drain(&mut self) -> Result<(), EngineError> {
        while let Some(pes) = self.assembler.next_pes() {
            self.mint(pes)?;
        }
        for record in self.assembler.take_recovered() {
            self.queue.push_back(DemuxEvent::Recovered {
                skipped: record.skipped,
                reason: record.reason,
            });
        }
        // Timing anomalies (DTS regression/jump, PTS regression on
        // a no-DTS stream): honest container-level signals — one
        // `Discontinuity` each (arithmetic across them without a
        // clock reset is the engine's own rule to enforce).
        for _ in self.assembler.take_timing_events() {
            self.queue.push_back(DemuxEvent::Discontinuity);
        }
        Ok(())
    }

    /// Mint one completed PES into the queue: a whole packet for
    /// every family but ADTS-framed aac, which splits per AU (the
    /// decoder contract — see [`Self::mint_adts_aus`]).
    fn mint(&mut self, pes: PesPacket) -> Result<(), EngineError> {
        let Some(track) = self.routing.get(&pes.pid).copied() else {
            // A completed PES on a pid the PSI declared but the
            // routing missed: a demuxer/host disagreement —
            // counted as recovered damage, never silently dropped.
            self.queue.push_back(DemuxEvent::Recovered {
                skipped: pes.pes_len as u64,
                reason: "ts.pes.unrouted-pid",
            });
            return Ok(());
        };
        if self.au_split.contains(&pes.pid) {
            self.mint_adts_aus(pes, track)?;
        } else {
            self.queue
                .push_back(DemuxEvent::Packet(mint_packet(pes, track)?));
        }
        Ok(())
    }

    /// The ADTS audio PES → per-AU engine packets: a broadcast
    /// muxer packs several ADTS frames into ONE bounded PES (the
    /// demux suite's recorded multi-AU truth), while the AAC
    /// decoder's packet contract is **one complete ADTS frame per
    /// packet** (the raw-ADTS precedent — a multi-frame buffer
    /// would decode only its first frame). The walk is the demux
    /// crate's own published header grammar
    /// ([`AdtsHeader::parse`] — no local copy), gated on the
    /// payload actually starting with the ADTS syncword: a PES
    /// that does not (LATM, damage) keeps the whole-packet shape
    /// and the decoder's typed refusal — never a guess.
    ///
    /// Per-AU stamps are the **PES anchor + exact rational sample
    /// arithmetic** (`pts + round(samples_before × 90 000 /
    /// rate)`) — the same frame-arithmetic synthesis ffprobe
    /// performs for its per-AU rows (measured on 0091: last AU
    /// 304 030 == ffprobe exactly; intermediate AUs may differ by
    /// a tick — ffmpeg accumulates per-AU truncations, this wiring
    /// anchors each PES's AUs to the wire stamp).
    fn mint_adts_aus(&mut self, pes: PesPacket, track: TrackId) -> Result<(), EngineError> {
        let PesPacket {
            pid: _,
            stream_id: _,
            class: _,
            pts,
            dts,
            data_alignment: _,
            discontinuity,
            pes_len: _,
            payload,
        } = pes;
        // The first frame owns the rate the stamp arithmetic rides
        // on; an unresolvable index is the whole-packet shape (the
        // decoder's typed refusal path).
        let rate = AdtsHeader::parse(&payload).and_then(|head| head.sample_rate());
        if let Some(rate) = rate {
            let mut at = 0usize;
            let mut samples_before = 0u64;
            let mut aus = 0u64;
            while at + 7 <= payload.len() {
                let Some(header) = AdtsHeader::parse(&payload[at..]) else {
                    break;
                };
                let end = at + usize::from(header.frame_length);
                if end > payload.len() {
                    // A trailing partial AU: the ffmpeg-honest stop.
                    break;
                }
                let packet = Packet {
                    track,
                    pts: au_stamp(pts, samples_before, rate)?,
                    dts: au_stamp(dts, samples_before, rate)?,
                    duration: None,
                    data: SharedBytes::from_vec(payload[at..end].to_vec()),
                    keyframe: false,
                    discontinuity: discontinuity && aus == 0,
                    generation: media_types::GenerationId::FIRST,
                };
                self.queue.push_back(DemuxEvent::Packet(packet));
                samples_before = samples_before.saturating_add(header.samples());
                aus += 1;
                at = end;
            }
            if aus > 0 {
                if at < payload.len() {
                    // Non-ADTS tail bytes inside an ADTS PES:
                    // counted, never silently dropped.
                    self.queue.push_back(DemuxEvent::Recovered {
                        skipped: (payload.len() - at) as u64,
                        reason: "ts.pes.adts-tail",
                    });
                }
                return Ok(());
            }
        }
        // No ADTS framing found: the whole-PES shape, honestly.
        self.queue.push_back(DemuxEvent::Packet(Packet {
            track,
            pts: match pts {
                Some(ticks) => pts_stamp(ticks)?,
                None => Stamp::Missing,
            },
            dts: match dts {
                Some(ticks) => pts_stamp(ticks)?,
                None => Stamp::Missing,
            },
            duration: None,
            data: SharedBytes::from_vec(payload),
            keyframe: false,
            discontinuity,
            generation: media_types::GenerationId::FIRST,
        }));
        Ok(())
    }

    /// The next minted event, if any.
    fn next_event(&mut self) -> Option<DemuxEvent> {
        self.queue.pop_front()
    }
}

/// The seam's consumer box: locks the shared bridge, forwards the
/// payload verbatim.
struct BridgeConsumer {
    bridge: Arc<Mutex<PesBridge>>,
}

impl PesConsumer for BridgeConsumer {
    fn pes_payload(&mut self, pid: u16, pusi: bool, payload: &[u8]) -> Result<(), EngineError> {
        let mut bridge = self
            .bridge
            .lock()
            .map_err(|_| bridge_poisoned("the PES bridge lock was poisoned"))?;
        bridge.push(pid, pusi, payload)
    }
}

/// One completed PES packet → one engine packet: the 90 kHz stamps
/// in the track's own base, the elementary payload as shared bytes.
/// The container declares no keyframe bit (IVF's precedent) and no
/// per-PES duration.
fn mint_packet(pes: PesPacket, track: TrackId) -> Result<Packet, EngineError> {
    let pts = match pes.pts {
        Some(ticks) => pts_stamp(ticks)?,
        None => Stamp::Missing,
    };
    let dts = match pes.dts {
        Some(ticks) => pts_stamp(ticks)?,
        None => Stamp::Missing,
    };
    Ok(Packet {
        track,
        pts,
        dts,
        duration: None,
        data: SharedBytes::from_vec(pes.payload),
        keyframe: false,
        discontinuity: pes.discontinuity,
        generation: media_types::GenerationId::FIRST,
    })
}

/// A per-AU stamp: the PES anchor advanced by the exact rational
/// sample arithmetic (`round(samples × 90 000 / rate)` — u128
/// intermediate, saturating, never a float).
fn au_stamp(anchor: Option<u64>, samples_before: u64, rate: u32) -> Result<Stamp, EngineError> {
    let Some(ticks) = anchor else {
        return Ok(Stamp::Missing);
    };
    let rate = u64::from(rate.max(1));
    let add = u64::try_from(
        (u128::from(samples_before) * 90_000 + u128::from(rate / 2)) / u128::from(rate),
    )
    .unwrap_or(u64::MAX);
    pts_stamp(ticks.saturating_add(add))
}

/// The typed refusal for an unpoisonable lock going wrong anyway
/// (house shape: this build never panics, so this is unreachable —
/// but it stays a typed error, never an `unwrap`).
fn bridge_poisoned(what: &'static str) -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Demux,
        ErrorKind::Unsupported { format: None },
        media_types::error::ErrorContext::new("cli.ts.bridge-poisoned"),
    )
    .with_source(std::io::Error::other(what))
}

// ---------------------------------------------------------------------------
// The host demuxer (the walk + the bridge, one event stream)
// ---------------------------------------------------------------------------

/// The started TS demuxer plus its PES bridge behind the wrapper
/// seam: `start()` answers the pre-pass's cached list (the
/// demuxer's own idempotence), `next_event` interleaves the
/// demuxer's events with the bridge's minted packets, and seek
/// answers stay the demuxer's own (the typed `None`-family
/// refusal, forwarded — never invented).
struct TsHostDemuxer {
    inner: TsDemuxer,
    bridge: Arc<Mutex<PesBridge>>,
}

impl TsHostDemuxer {
    /// Run `f` on the shared bridge (typed poisoning refusal).
    fn with_bridge<T>(
        &self,
        f: impl FnOnce(&mut PesBridge) -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        let mut bridge = self
            .bridge
            .lock()
            .map_err(|_| bridge_poisoned("the PES bridge lock was poisoned"))?;
        f(&mut bridge)
    }
}

impl Demuxer for TsHostDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        // The demuxer's own idempotence answers the cached list
        // (the pre-pass's truth, re-served to the engine).
        self.inner.start()
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        // 1. The bridge's minted packets first: the transport walk
        //    queued its own events before the seam minted packets
        //    for the same transport packet, so this order preserves
        //    the demuxer's internal sequence.
        if let Some(event) = self.with_bridge(|bridge| Ok(bridge.next_event()))? {
            return Ok(event);
        }
        match self.inner.next_event()? {
            DemuxEvent::EndOfStream => {
                // 2. The flush: mint the trailing PES, then the
                //    honest end. A short bounded PES raises the
                //    typed truncation here (re-raised verbatim on
                //    every later call — see `finish`).
                self.with_bridge(PesBridge::finish)?;
                if let Some(event) = self.with_bridge(|bridge| Ok(bridge.next_event()))? {
                    return Ok(event);
                }
                Ok(DemuxEvent::EndOfStream)
            }
            event => Ok(event),
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        self.inner.seek(target)
    }

    fn seekability(&self) -> SeekSupport {
        self.inner.seekability()
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
    use commands::{ErrorCode, Stage};
    use media_types::{Frame, GenerationId, MediaKind};
    use playback_engine::DecoderProvider;
    use std::path::PathBuf;
    use testkit::Must;

    /// The audio sink (the adts.rs convention): counts decoded
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

    /// Open a source through the REAL CLI open path (canonicalize →
    /// access → probe → this container host).
    fn open(path: PathBuf, start_us: Option<i64>) -> OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// The real provider's decoder for one track (typed refusals
    /// RETURNED, never panicked — the lanes pin them by code).
    fn decoder_for(opened: &OpenedMedia, track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
        let facts = Arc::new(opened.facts.clone());
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

    /// A temp dir that removes itself (the probe_io convention; no
    /// committed .ts fixture exists in the tree, so the
    /// corpus-independent lane writes its own).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let dir =
                std::env::temp_dir().join(format!("lumen-opw-ts-{tag}-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
            std::fs::create_dir_all(&dir).must();
            TempDir(dir)
        }
        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
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

    /// Encode a 33-bit stamp into the 5 guarded PES bytes (the
    /// inverse of the assembler's `decode_ts5` — mirrored, with the
    /// marker bits the decoder requires).
    fn encode_ts5(value: u64, prefix: u8) -> [u8; 5] {
        let v = value & 0x1_FFFF_FFFF;
        [
            ((u64::from(prefix) << 4) | ((v >> 30) & 0x07) << 1 | 1) as u8,
            ((v >> 22) & 0xFF) as u8,
            ((((v >> 15) & 0x7F) << 1) | 1) as u8,
            ((v >> 7) & 0xFF) as u8,
            (((v & 0x7F) << 1) | 1) as u8,
        ]
    }

    /// One bounded PTS-only PES packet (test-local elementary
    /// bytes — the demux crate's test_support owns the PSI half).
    fn pes_bytes(stream_id: u8, pts: u64, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0x00, 0x00, 0x01, stream_id];
        // The length field counts everything after these 6 bytes:
        // flags1 + flags2 + data_length + the 5-byte PTS + payload.
        let after = 8 + payload.len();
        out.extend_from_slice(&u16::try_from(after).unwrap_or(u16::MAX).to_be_bytes());
        out.push(0x80); // flags1 (marker bits)
        out.push(0x80); // flags2: PTS only (0b10)
        out.push(5); // data_length
        out.extend_from_slice(&encode_ts5(pts, 0b0010));
        out.extend_from_slice(payload);
        out
    }

    /// One ADTS-shaped frame (85 bytes: 7-byte LC/44.1 kHz/stereo
    /// header + body, 1024 samples, VBR fullness) — framing only,
    /// nothing decodes it (the fields mirror `AdtsHeader::parse`).
    fn adts_frame_85(fill: u8) -> [u8; 85] {
        let mut frame = [fill; 85];
        frame[0] = 0xFF;
        frame[1] = 0xF1; // MPEG-4, layer 0, no CRC
        frame[2] = (1 << 6) | (4 << 2); // profile LC, sf_idx 4 (44100)
        frame[3] = 0x80; // channel_config high bits (2) + length hi
        frame[4] = (85 >> 3) as u8;
        frame[5] = ((85 & 0x07) << 5) | 0x1F; // length lo + VBR fullness
        frame[6] = 0xFC; // fullness lo + one raw block
        frame
    }

    /// A minimal single-program TS with REAL PES payloads (the
    /// demux crate's own builders): PAT/PMT/SDT, one bounded video
    /// PES on 0x100 (PTS 126 000), one bounded audio PES on 0x101
    /// (PTS 128 000) carrying TWO ADTS frames (the AU-split lane —
    /// every PES fills its transport packet's payload region
    /// exactly, the ffmpeg muxing convention the assembler's
    /// bounded-length checks assume).
    fn synthetic_ts() -> Vec<u8> {
        use demux::ts::test_support::{
            pat_body, pmt_body, psi_packets, psi_section, sdt_body, stream_of, ts_packet,
        };
        let pat = psi_section(0x00, 1, 0, &pat_body(&[(1, 0x1000)]));
        let pmt = psi_section(
            0x02,
            1,
            0,
            &pmt_body(0x100, &[], &[(0x1B, 0x100, &[]), (0x0F, 0x101, &[])]),
        );
        let sdt = psi_section(0x42, 1, 0, &sdt_body(1, &[(1, "FFmpeg", "Service01")]));
        let mut audio_aus = adts_frame_85(0xBB).to_vec();
        audio_aus.extend_from_slice(&adts_frame_85(0xBC));
        let mut packets = Vec::new();
        packets.extend(psi_packets(0x0011, 0, &[sdt]));
        packets.extend(psi_packets(0x0000, 0, &[pat]));
        packets.extend(psi_packets(0x1000, 0, &[pmt]));
        packets.push(ts_packet(
            0x100,
            true,
            0,
            None,
            &pes_bytes(0xE0, 126_000, &[0xAA; 170]),
        ));
        packets.push(ts_packet(
            0x101,
            true,
            0,
            None,
            &pes_bytes(0xC0, 128_000, &audio_aus),
        ));
        stream_of(&packets)
    }

    /// The corpus-independent lane: a synthetic transport stream
    /// opens through the REAL open path, carries both tracks' facts
    /// (the PMT registry vocabulary, the 90 kHz clock, nothing the
    /// container does not declare), demuxes its PES through the
    /// wired bridge, and answers seek with the typed
    /// `SeekSupport::None` refusal — never a fake position.
    #[test]
    fn synthetic_lane_opens_demuxes_and_refuses_seek_typed() {
        let dir = TempDir::new("synthetic");
        let path = dir.write("synthetic.ts", &synthetic_ts());
        let mut opened = open(path, None);
        assert_eq!(opened.facts.container, "mpegts");
        assert_eq!(opened.facts.tracks.len(), 2, "the PMT declares two ES");
        let (video, audio) = (&opened.facts.tracks[0], &opened.facts.tracks[1]);
        assert_eq!(video.kind, MediaKind::Video);
        assert_eq!(video.codec.as_str(), "h264", "stream_type 0x1B");
        assert_eq!(audio.kind, MediaKind::Audio);
        assert_eq!(audio.codec.as_str(), "aac", "stream_type 0x0F");
        for fact in &opened.facts.facts {
            assert_eq!(fact.base.as_parts(), (1, 90_000), "the MPEG clock");
            assert!(fact.pcm.is_none(), "TS has no PCM codec");
            assert!(
                fact.extra_data.is_none(),
                "no wired TS codec separates its config"
            );
            assert!(fact.span_end_ticks.is_none(), "the PSI declares no extent");
            assert!(fact.frame_ns.is_none(), "cadence is codec-domain");
            assert!(fact.dimensions.is_none(), "geometry is codec-domain");
            assert!(
                fact.audio_geometry().is_none(),
                "no geometry-bearing blob — limb-less, the ADTS gap"
            );
        }
        assert_eq!(opened.start_position_ns, 0);
        // The walk: the minted events — the video PES as ONE
        // packet, the audio PES AU-SPLIT into its two ADTS frames
        // (the second's stamp is the PES anchor +
        // round(1024 × 90 000 / 44 100) = 2 090 ticks) — then the
        // end.
        let mut pts_seen = Vec::new();
        let mut tracks_seen = Vec::new();
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    tracks_seen.push(packet.track);
                    match packet.pts {
                        Stamp::Present(stamp) => pts_seen.push(stamp.ticks),
                        Stamp::Missing => unreachable!("both PES carry PTS"),
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {
                    unreachable!("the synthetic lane is clean")
                }
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        assert_eq!(
            pts_seen,
            vec![126_000, 128_000, 130_090],
            "the encoded stamps + the AU arithmetic"
        );
        assert_eq!(
            tracks_seen,
            vec![video.id, audio.id, audio.id],
            "the routing + the AU split"
        );
        // The native-seek lane: SeekSupport::None, forwarded — the
        // typed refusal, never fake seekability.
        assert_eq!(opened.demuxer.seekability(), SeekSupport::None);
        match opened.demuxer.seek(TimeSpan::UNKNOWN) {
            Err(err) => assert_eq!(err.context.code.as_str(), "demux.ts.seek.unsupported"),
            Ok(_) => unreachable!("TS seek must refuse"),
        }
        // The codec truth: h264 has no software decoder in this
        // build — the typed decode-stage refusal; aac dispatches
        // the AAC-LC decoder (ADTS mode, in-band config).
        match decoder_for(&opened, video) {
            Err(err) => assert_eq!(err.context.code.as_str(), "cli.codec_unavailable", "{err}"),
            Ok(_) => unreachable!("no h264 decoder exists in this build"),
        }
        let _ = decoder_for(&opened, audio)
            .unwrap_or_else(|err| unreachable!("aac must dispatch: {err}"));
    }

    /// `--start-time` on the not-seekable family: the pure seek
    /// query's typed refusal IS the open answer (stage `demux`,
    /// code `demux.ts.seek.unsupported`) — no clamped watermark, no
    /// silently filtered "start".
    #[test]
    fn start_time_is_refused_typed_at_open() {
        let dir = TempDir::new("start-time");
        let path = dir.write("start-time.ts", &synthetic_ts());
        match open_source(&SourceSpec::File { path }, Some(1_000_000), false, false) {
            Err(err) => {
                assert_eq!(err.stage, Stage::Demux, "{err}");
                assert_eq!(err.code, ErrorCode::Unsupported, "{err}");
                assert_eq!(
                    err.details.get("code").map(String::as_str),
                    Some("demux.ts.seek.unsupported"),
                    "{err}"
                );
            }
            Ok(_) => unreachable!("--start-time must refuse on SeekSupport::None"),
        }
    }

    /// The corpus differential (the user's standing mandate): every
    /// `.ts` lane of corpus-v4 through the REAL open path — the
    /// four video lanes demux their full PES truth (counts and
    /// first/last PTS = the demux suite's ffprobe-measured pins)
    /// and refuse decode typed at the codec stage; 0091's aac track
    /// decodes the FULL stream exactly (measured this session:
    /// ffmpeg's decoded s16le output is 88 064 bytes = 44 032 mono
    /// sample frames). Skips loudly when the corpus is absent (the
    /// corpus_ogg.rs convention; no `LUMEN_CORPUS` override is set
    /// by this suite).
    #[test]
    fn corpus_ts_lanes_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
            eprintln!(
                // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
                "corpus(ts-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
                 — SKIP (the differential table is in the worklog)"
            );
            return;
        };
        // (file, codec, packets, first PTS, last PTS) per video lane
        // (PTS in 90 kHz ticks, the track's own base).
        let lanes: [(&str, &str, u64, i64, i64); 4] = [
            ("0000-param-midchange.ts", "h264", 81, 126_000, 526_000),
            ("0018-ts-video.ts", "h264", 234, 129_000, 477_000),
            ("0113-dynres-video.ts", "h264", 160, 126_000, 722_846),
            ("0134-ts-video.ts", "h264", 3, 126_000, 306_000),
        ];
        for (file, codec, packets, first_pts, last_pts) in lanes {
            let mut opened = open(dir.join(file), None);
            assert_eq!(opened.facts.container, "mpegts", "{file}");
            assert_eq!(opened.facts.tracks.len(), 1, "{file}");
            let fact = &opened.facts.facts[0];
            assert_eq!(fact.codec, codec, "{file}");
            assert_eq!(fact.base.as_parts(), (1, 90_000), "{file}");
            let mut seen = 0u64;
            let mut pts_series = Vec::new();
            loop {
                match opened.demuxer.next_event() {
                    Ok(DemuxEvent::Packet(packet)) => {
                        seen += 1;
                        match packet.pts {
                            Stamp::Present(stamp) => pts_series.push(stamp.ticks),
                            Stamp::Missing => {}
                        }
                    }
                    Ok(DemuxEvent::EndOfStream) => break,
                    // The clean lanes: no fabricated discontinuity,
                    // no recovered damage (the demux suite's
                    // measured zero-damage truth).
                    Ok(DemuxEvent::Recovered { skipped, .. }) => {
                        unreachable!("{file}: recovered {skipped} bytes on a clean lane")
                    }
                    Ok(DemuxEvent::Discontinuity) => {
                        unreachable!("{file}: discontinuity on a clean lane")
                    }
                    Err(err) => unreachable!("{file}: {err}"),
                }
            }
            assert_eq!(seen, packets, "{file}: PES packet count");
            assert_eq!(
                pts_series.first().copied(),
                Some(first_pts),
                "{file}: first PTS"
            );
            assert_eq!(
                pts_series.last().copied(),
                Some(last_pts),
                "{file}: last PTS"
            );
            // The midchange lane's container-level truth: the
            // parameter change rides IN-BAND (SPS payload,
            // codec-domain) — the demuxer neither fabricates a
            // discontinuity nor hides one; the cadence stays the
            // uniform 5 000-tick (18 fps) step end to end.
            if file.starts_with("0000-") {
                for pair in pts_series.windows(2) {
                    assert_eq!(
                        pair[1] - pair[0],
                        5_000,
                        "{file}: cadence must stay uniform"
                    );
                }
            }
            // The video codec truth: no software decoder — the
            // typed decode-stage refusal (h264 and mpeg2video alike;
            // every corpus video lane is h264).
            let track = opened.facts.tracks[0].clone();
            match decoder_for(&opened, &track) {
                Err(err) => {
                    assert_eq!(
                        err.context.code.as_str(),
                        "cli.codec_unavailable",
                        "{file}: {err}"
                    )
                }
                Ok(_) => unreachable!("{file}: no {codec} decoder exists in this build"),
            }
        }
        // 0091-bframe-storm: hevc video (53 PES, typed refusal) +
        // aac audio (6 multi-AU PES on the wire → 43 AU packets
        // after the split — ffprobe's own nb_read_packets is the
        // same 43; its per-AU pts rows are the same frame
        // arithmetic this wiring performs) decoding the FULL stream.
        let mut opened = open(dir.join("0091-bframe-storm.ts"), None);
        assert_eq!(opened.facts.container, "mpegts");
        assert_eq!(opened.facts.tracks.len(), 2);
        assert_eq!(opened.facts.facts[0].codec, "hevc");
        assert_eq!(opened.facts.facts[1].codec, "aac");
        let video = opened.facts.tracks[0].clone();
        let mut per_track = [0u64, 0u64];
        // (first PTS, last PTS) per track in EMISSION order — the
        // B-frame storm's video series is non-monotone by design,
        // so bounds would misreport the demux suite's end pins.
        let mut pts_ends = [Option::<(i64, i64)>::None, Option::<(i64, i64)>::None];
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let at = if packet.track == video.id { 0 } else { 1 };
                    per_track[at] += 1;
                    if let Stamp::Present(stamp) = packet.pts {
                        let slot = &mut pts_ends[at];
                        match slot {
                            None => *slot = Some((stamp.ticks, stamp.ticks)),
                            Some((_, last)) => *last = stamp.ticks,
                        }
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {
                    unreachable!("0091: discontinuity on a clean lane")
                }
                Err(err) => unreachable!("0091: {err}"),
            }
        }
        assert_eq!(per_track[0], 53, "0091 video PES count");
        assert_eq!(per_track[1], 43, "0091 audio AU count (== ffprobe)");
        assert_eq!(
            pts_ends[0],
            Some((132_667, 302_667)),
            "0091 video PTS ends (emission order)"
        );
        assert_eq!(
            pts_ends[1],
            Some((128_487, 304_030)),
            "0091 audio PTS ends (anchor + AU arithmetic; ffprobe's last row is the same 304 030)"
        );
        match decoder_for(&opened, &video) {
            Err(err) => assert_eq!(err.context.code.as_str(), "cli.codec_unavailable", "{err}"),
            Ok(_) => unreachable!("no hevc decoder exists in this build"),
        }
        // Fresh open for the decode pass (single-shot walk).
        let mut lane = open(dir.join("0091-bframe-storm.ts"), None);
        let facts = Arc::new(lane.facts.clone());
        let provider = HostDecoderProvider::new(Arc::clone(&facts));
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
            .unwrap_or_else(|err| unreachable!("0091 aac decoder_for: {err}"));
        let mut sink = CountingSink(0, 0);
        loop {
            match lane.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if packet.track == audio.id {
                        let _ = decoder
                            .decode(packet, &mut sink)
                            .unwrap_or_else(|err| unreachable!("0091 aac decode: {err}"));
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("0091 decode-pass demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("0091 aac flush: {err}"));
        // Measured this session: ffmpeg decodes 0091's audio to
        // 88 064 s16le bytes = 44 032 mono sample frames (43 AUs ×
        // 1024). The in-band ADTS config path through PES matches
        // it exactly.
        assert_eq!(sink.0, 44_032, "0091 aac decoded sample frames (== ffmpeg)");
        assert_eq!(sink.1, 1, "0091 aac mono output");
    }

    /// The corpus media dir (the corpus_ogg.rs convention).
    fn corpus_dir() -> Option<PathBuf> {
        // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
    }
}
