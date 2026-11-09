//! # demux::asf — the ASF/WMV/WMA header tree (card 6.ASF-a)
//!
//! The Advanced Systems Format family (`.asf`/`.wmv`/`.wma`): a
//! GUID+size object tree whose Header Object carries the whole track
//! table. This unit lands the header walk — File Properties, Stream
//! Properties (WAVEFORMATEX / short-VIDEOINFOHEADER facts), Codec
//! List, Extended Content Description, a bounded Header Extension
//! child walk — behind [`AsfDemuxer`], plus the [`probe`] gate and
//! the media-probe signature. **Data Object packets are landed (card
//! 6.ASF-b)**: `next_event`
//! walks the packet grammar (EC flags, padding, send time, duration,
//! multi-payload headers, replicated data, fragment reassembly) and
//! yields real [`DemuxEvent::Packet`]s — one per completed media
//! object, preroll-subtracted stamps on the track's own base. Seeking
//! is card 6.ASF-c's scope: the Simple Index is located (see
//! `top_objects`) and `seek` still answers the typed
//! `demux.asf.seek.unsupported` refusal until that unit lands.
//!
//! ## What is verified where (the honest field map)
//!
//! The ASF Specification text was not acquirable this session (the
//! [VERIFY-P0] gate's verdict — see the worklog): every layout the
//! parser encodes is byte-verified against ffmpeg 7.1.5 output and
//! closes its objects exactly; unverified semantics (the SP+52 flag
//! bytes, the video TSD's `0x02` byte, the 2 reserved bytes in the
//! Header Object head) are carried raw and labeled in
//! [`header`](self::header)'s module docs — the plan's risk-register
//! discipline. See that module for the per-object field tables.
//!
//! ## House rules
//!
//! Std-only, zero external dependencies (C5); no `unsafe`; no
//! `unwrap`/`expect`/`panic!` in non-test code; bounded memory
//! (MP §12): the header read is ceiling-checked
//! ([`MAX_HEADER_BYTES`]) before it is trusted into an allocation,
//! metadata strings and codec-list fields are field-capped, and the
//! object walks are iteration-guarded.
//!
//! ## Error paths (typed, unit-pinned)
//!
//! Non-ASF bytes refuse at `start` (`demux.asf.not-asf`); a cut
//! header is `demux.asf.truncated`; an impossible object size is
//! `demux.asf.bad-object-size`; a header above the memory ceiling is
//! `demux.asf.header-too-large`; a broadcast file (FPO flags bit 0,
//! ffprobe itself refuses those — measured) is
//! `demux.asf.broadcast-unsupported`; a header without any audio/
//! video stream is `demux.asf.no-streams`; a broken WAVEFORMATEX /
//! video TSD is `demux.asf.bad-waveformatex` / `bad-video-info`;
//! trait-order violations are `demux.asf.not-started`; the two
//! next-unit refusals are `demux.asf.packets-not-implemented` (ASF-b)
//! and `demux.asf.seek.unsupported` (ASF-c).

use std::collections::VecDeque;

use media_types::ids::{GenerationId, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::riff::SourceReader;

pub mod data;
pub mod error;
pub mod fuzz;
pub mod guid;
pub mod header;
pub mod test_support;

#[cfg(test)]
mod tests;

/// Maximum Header Object size trusted into memory (MP §12; the
/// header walk's allocation bound — see `header::MAX_HEADER_BYTES`).
pub use header::MAX_HEADER_BYTES;

/// File extensions this demuxer claims (the asf muxer's own
/// extension list: `asf,wmv,wma` — the plan's §1.2 note; `nsc`
/// excluded per the plan's phase5 sketch).
pub const EXTENSIONS: &[&str] = &["asf", "wmv", "wma"];

/// The probe: do the leading bytes carry the ASF Header Object GUID?
///
/// Window-start-anchored like every builtin signature: the
/// broadcast-variant files carry a 12-byte MMS `$H`-family prefix
/// before the GUID (plan §2.5) and therefore **do not anchor** — the
/// honest probe-miss, matching the demuxer's own typed
/// broadcast-refusal lane for anything that opens them regardless.
#[must_use]
pub fn probe(prefix: &[u8]) -> bool {
    prefix.len() >= 16 && guid::Guid::read(prefix, 0) == guid::HEADER
}

/// The ASF demuxer (card 6.ASF-a: the header tree).
///
/// Construct with [`AsfDemuxer::new`] over a seekable source stream,
/// then drive it through the [`Demuxer`] trait: `start` runs the
/// GUID object walk and enumerates the audio/video track table (one
/// [`Track`] per Stream Properties object, codec descriptors per the
/// plan's §4.5 map); `next_event` refuses typed until ASF-b lands
/// the packet grammar (the module docs' honest not-yet).
///
/// The facts accessors (`file_properties`, `streams`, `codec_list`,
/// `metadata`, `data_object`, `top_objects`) are the host harvest
/// seam: the verbatim type-specific data per stream
/// ([`StreamRecord::tsd_bytes`](header::StreamRecord::tsd_bytes)) is
/// the decoder-config `extra_data` blob (the ADPCM WAVEFORMATEX
/// body), and the PCM lane's `pcm` fact is
/// `(channels, rate)` from the same record.
///
/// # Example
///
/// ```
/// use demux::asf::{self, AsfDemuxer};
/// use plugin_api::{Demuxer, DemuxEvent};
///
/// # fn main() -> Result<(), media_types::EngineError> {
/// # let bytes = asf_demo_bytes();
/// let mut stream = demux::asf::test_support::mem_stream(bytes)?;
/// let mut asf = AsfDemuxer::new(stream)?;
/// let tracks = asf.start()?;
/// assert_eq!(tracks.len(), 1);
/// assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
/// // The honest ASF-b-not-yet: packets refuse typed, exactly.
/// let err = asf.next_event().unwrap_err();
/// assert_eq!(err.context.code, "demux.asf.packets-not-implemented");
/// # Ok(())
/// # }
/// # fn asf_demo_bytes() -> std::sync::Arc<[u8]> {
/// #   demux::asf::test_support::minimal_asf_audio(&[
/// #       0x01, 0x00, 0x01, 0x00, 0x11, 0x2b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
/// #       0x02, 0x00, 0x10, 0x00, 0x00, 0x00,
/// #   ])
/// # }
/// ```
pub struct AsfDemuxer {
    reader: SourceReader,
    started: bool,
    header: Option<header::AsfHeader>,
    tracks: Option<Vec<Track>>,
    /// The packet walk (ASF-b): stream-number → track-slot join,
    /// fixed packet size, Data Object payload-area origin, the walk
    /// cursor, the per-stream fragment assembler and the completed
    /// objects waiting to be emitted.
    walk: Option<PacketWalk>,
}

/// The ASF-b packet-walk state (built once by `start`).
struct PacketWalk {
    /// Stream number (from each Stream Properties record) → track
    /// index; payload stream numbers join through it.
    slots: Vec<(u16, usize)>,
    /// Per-track stamp bases (audio: 1/sample-rate; video: 1/1000).
    bases: Vec<TimeBase>,
    /// The FPO's fixed packet size (min == max, enforced at start).
    packet_size: u64,
    /// Absolute offset of the first data packet (Data Object + 50).
    payload_offset: u64,
    /// Total packets (the Data Object's declared count, bounded by
    /// the object's own extent — never trusted past its bytes).
    packets_total: u64,
    /// The next packet index to read.
    next_packet: u64,
    /// The FPO preroll in ms (payload stamps are preroll-offset).
    preroll_ms: u64,
    /// Fragment reassembly (one bounded slot per stream).
    assembler: data::ObjectAssembler,
    /// Completed media objects pending emission.
    queue: VecDeque<Packet>,
    /// Nothing more will come (flush endpoint armed).
    eos: bool,
}

impl AsfDemuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// No container I/O happens here (the trait's `start` does the
    /// parsing) — this only rejects sources that cannot carry an ASF:
    /// unknown length, unseekable, empty (the shared `SourceReader`
    /// contract).
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for those three cases.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<AsfDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(AsfDemuxer {
            reader,
            started: false,
            header: None,
            tracks: None,
            walk: None,
        })
    }

    /// The full header-walk product (start must have run).
    #[must_use]
    pub fn header(&self) -> Option<&header::AsfHeader> {
        self.header.as_ref()
    }

    /// The File Properties facts (start must have run).
    #[must_use]
    pub fn file_properties(&self) -> Option<header::FileProperties> {
        self.header.as_ref().map(|h| h.properties)
    }

    /// The stream records — the track-table source and the
    /// decoder-config facts seam (start must have run).
    #[must_use]
    pub fn streams(&self) -> &[header::StreamRecord] {
        self.header.as_ref().map_or(&[], |h| &h.streams)
    }

    /// The stream record backing `track`, when `track` is one this
    /// demuxer enumerated (the host's per-track facts lookup).
    #[must_use]
    pub fn stream_record_for_track(&self, track: TrackId) -> Option<&header::StreamRecord> {
        let index = track.handle() as usize;
        self.header.as_ref()?.streams.get(index)
    }

    /// The Codec List entries (advisory; start must have run).
    #[must_use]
    pub fn codec_list(&self) -> &[header::CodecEntry] {
        self.header.as_ref().map_or(&[], |h| &h.codec_list)
    }

    /// The Extended Content Description string pairs (start must
    /// have run).
    #[must_use]
    pub fn metadata(&self) -> &[header::MetadataPair] {
        self.header.as_ref().map_or(&[], |h| &h.metadata)
    }

    /// The located Data Object (packets are ASF-b's; start must have
    /// run).
    #[must_use]
    pub fn data_object(&self) -> Option<header::DataObjectLocation> {
        self.header.as_ref().map(|h| h.data)
    }

    /// The top-level object map (start must have run).
    #[must_use]
    pub fn top_objects(&self) -> &[header::TopObject] {
        self.header.as_ref().map_or(&[], |h| &h.top_objects)
    }

    /// Total bytes/objects the bounded walk skipped or damage-noted
    /// (the observability sum of the header walk's `DamageLog`).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.header.as_ref().map_or(0, |h| h.damage.total)
    }

    /// The PCM lane's host fact (`(channels, rate)`) for `track` —
    /// the `pcm` harvest the open path seeds `PcmDecoder::with_format`
    /// with (the wav-lane seam; `None` for non-PCM tracks).
    #[must_use]
    pub fn pcm_fact(&self, track: TrackId) -> Option<(u16, u32)> {
        let record = self.stream_record_for_track(track)?;
        let wfx = record.wave_format?;
        record
            .codec
            .as_str()
            .starts_with("pcm_")
            .then_some((wfx.channels, wfx.samples_per_sec))
    }

    /// Build the L0 track table from the walk's stream records.
    fn build_tracks(records: &[header::StreamRecord]) -> Result<Vec<Track>, EngineError> {
        let mut tracks = Vec::with_capacity(records.len());
        for (index, record) in records.iter().enumerate() {
            // Timing: the plan's flagged decision — track spans come
            // from ASF-b's packet stamps, FPO durations are advisory
            // only (ffmpeg pads Play Duration: 4.164 s declared for a
            // 1.128 s file, measured). The base is the honest tick
            // each lane's stamps will ride: sample ticks for audio,
            // the verified millisecond preroll-offset clock for
            // video (plan §2.4). The span stays unknown until the
            // stamps exist.
            let base = if record.kind == media_types::ids::MediaKind::Audio {
                let rate = record.wave_format.map_or(0, |wfx| wfx.samples_per_sec);
                TimeBase::new(1, rate).map_err(|_| error::bad_waveformatex(0))?
            } else {
                TimeBase::new(1, 1000).map_err(|_| error::bad_object_size(0))?
            };
            tracks.push(Track {
                id: TrackId::new(index as u32),
                stream: StreamId::new(0),
                kind: record.kind,
                language: None,
                codec: record.codec.clone(),
                title: None,
                flags: TrackFlags::NONE,
                timing: TrackTiming {
                    span: TimeSpan::UNKNOWN,
                    base,
                    gaps: Vec::new(),
                },
            });
        }
        Ok(tracks)
    }
    /// Build the ASF-b packet walk from the parsed header (the
    /// fixed-packet-size and stream-join facts the walk rides).
    fn build_walk(
        parsed: &header::AsfHeader,
        tracks: &[Track],
    ) -> Result<PacketWalk, EngineError> {
        let properties = &parsed.properties;
        if properties.min_packet_size != properties.max_packet_size
            || properties.max_packet_size == 0
        {
            // The verified grammar has no per-packet length field;
            // variable-size files are an unverified shape.
            return Err(error::packet_variable_size());
        }
        let data = parsed.data;
        let payload_offset = data.offset + 50;
        let by_extent = if properties.max_packet_size > 0 {
            (data.size.saturating_sub(50)) / u64::from(properties.max_packet_size)
        } else {
            0
        };
        // The declared count, bounded by the object's own extent.
        let packets_total = data.packet_count.min(by_extent);
        let slots = parsed
            .streams
            .iter()
            .enumerate()
            .map(|(index, record)| (record.stream_number, index))
            .collect::<Vec<_>>();
        let bases = tracks.iter().map(|t| t.timing.base).collect::<Vec<_>>();
        Ok(PacketWalk {
            slots,
            bases,
            packet_size: u64::from(properties.max_packet_size),
            payload_offset,
            packets_total,
            next_packet: 0,
            preroll_ms: properties.preroll_ms,
            assembler: data::ObjectAssembler::new(parsed.streams.len()),
            queue: VecDeque::new(),
            eos: false,
        })
    }

    /// Read and parse packet `index`, feeding every payload through
    /// reassembly; completed objects queue up as `Packet`s.
    fn walk_one_packet(&mut self, index: u64) -> Result<(), EngineError> {
        let (packet_size, payload_offset) = self
            .walk
            .as_ref()
            .map_or((0, 0), |w| (w.packet_size, w.payload_offset));
        let at = payload_offset + index * packet_size;
        let bytes = self
            .reader
            .read_exact_or_end_at(packet_size as usize, at)?
            .ok_or_else(|| error::truncated(at + packet_size, at))?;
        let parsed_packet = data::parse_packet(&bytes)?;
        for payload in &parsed_packet.payloads {
            let slot = self.walk.as_ref().and_then(|w| {
                w.slots
                    .iter()
                    .position(|(number, _)| *number == payload.stream)
                    .map(|slot| (slot, w.bases.get(slot).copied()))
            });
            let Some((slot, base)) = slot else {
                // A payload for a stream the header never declared:
                // skipped, never fatal (the demux recovery seam).
                continue;
            };
            let data_bytes = &bytes[payload.data_range.0..payload.data_range.1];
            let finished = self.walk.as_mut().and_then(|w| {
                w.assembler
                    .feed(
                        slot,
                        payload.stream,
                        payload.media_object,
                        payload.offset_into_object,
                        payload.replicated,
                        payload.keyframe,
                        data_bytes,
                    )
                    .transpose()
                    .map(|r| r.map(|done| (done, w.preroll_ms)))
            });
            let Some(finished) = finished else {
                continue;
            };
            let ((presentation_time_ms, keyframe, buffer), preroll) = finished?;
            let Some(base) = base else {
                continue;
            };
            let Some(track) = self.tracks.as_ref().and_then(|t| t.get(slot)) else {
                continue;
            };
            // Preroll subtraction (measured: stamps ride preroll-offset
            // ms; ffprobe's zero-based pts is the arithmetic check).
            let preroll = u32::try_from(preroll).unwrap_or(u32::MAX);
            let pts_ms = presentation_time_ms.saturating_sub(preroll);
            let ticks = Self::ticks_for(track, base, pts_ms);
            if let Some(walk) = self.walk.as_mut() {
                walk.queue.push_back(Packet {
                    track: track.id,
                    pts: Stamp::Present(Timestamp::new(ticks, base)),
                    dts: Stamp::Missing,
                    duration: None,
                    data: SharedBytes::from_vec(buffer),
                    keyframe,
                    discontinuity: false,
                    generation: GenerationId::FIRST,
                });
            }
        }
        Ok(())
    }

    /// Convert a preroll-subtracted millisecond stamp onto `track`'s
    /// own base (audio rides sample ticks, video the ms clock — the
    /// ASF-a timing decision).
    fn ticks_for(track: &Track, base: TimeBase, pts_ms: u32) -> i64 {
        let _ = track;
        let ms = i64::from(pts_ms);
        let (num, den) = base.as_parts();
        if den == 1000 && num == 1 {
            // The millisecond clock is the base.
            ms
        } else {
            // Sample ticks: ms × rate / 1000 (checked; a base too
            // large for i64 clamps to the honest maximum).
            let rate = i64::from(den) / i64::from(num.max(1));
            ms.checked_mul(rate)
                .and_then(|t| t.checked_div(1000))
                .unwrap_or(i64::MAX)
        }
    }

}

impl Demuxer for AsfDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Ok(self.tracks.clone().unwrap_or_default());
        }
        let parsed = header::AsfHeader::parse(&mut self.reader)?;
        let tracks = Self::build_tracks(&parsed.streams)?;
        let walk = Self::build_walk(&parsed, &tracks)?;
        self.header = Some(parsed);
        self.tracks = Some(tracks.clone());
        self.walk = Some(walk);
        self.started = true;
        Ok(tracks)
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        loop {
            let (queued, exhausted) = {
                let Some(walk) = self.walk.as_ref() else {
                    return Err(error::not_started());
                };
                (
                    walk.queue.front().map(|p| p.track),
                    walk.next_packet >= walk.packets_total,
                )
            };
            if queued.is_some() {
                let packet = self
                    .walk
                    .as_mut()
                    .and_then(|w| w.queue.pop_front());
                if let Some(packet) = packet {
                    return Ok(DemuxEvent::Packet(packet));
                }
                continue;
            }
            if exhausted {
                if let Some(walk) = self.walk.as_mut() {
                    if walk.eos {
                        return Ok(DemuxEvent::EndOfStream);
                    }
                    walk.eos = true;
                }
                return Ok(DemuxEvent::EndOfStream);
            }
            let index = self
                .walk
                .as_ref()
                .map_or(0, |w| w.next_packet);
            self.walk_one_packet(index)?;
            if let Some(walk) = self.walk.as_mut() {
                walk.next_packet += 1;
            }
        }
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        // ASF-c's row: the Simple Index is located (see
        // `top_objects`), the seek table is not landed.
        Err(error::seek_unsupported())
    }

    fn seekability(&self) -> SeekSupport {
        // ASF-c owns the §4.3 policy (index → Fast{indexed}, no
        // index → SlowLinear); until then the honest default — the
        // typed refusals above, never fake successes.
        SeekSupport::None
    }
}
