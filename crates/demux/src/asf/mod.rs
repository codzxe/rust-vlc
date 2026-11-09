//! # demux::asf — the ASF/WMV/WMA demuxer (cards 6.ASF-a/b/c)
//!
//! The Advanced Systems Format family (`.asf`/`.wmv`/`.wma`): a
//! GUID+size object tree whose Header Object carries the whole track
//! table. ASF-a landed the header walk — File Properties, Stream
//! Properties (WAVEFORMATEX / short-VIDEOINFOHEADER facts), Codec
//! List, Extended Content Description, a bounded Header Extension
//! child walk — plus the [`probe`] gate and the media-probe
//! signature. **ASF-b landed the Data Object packet walk**:
//! `next_event` walks the packet grammar (EC flags, padding, send
//! time, duration, multi-payload headers, replicated data, fragment
//! reassembly) and yields real [`DemuxEvent::Packet`]s — one per
//! completed media object, preroll-subtracted stamps on the track's
//! own base. **ASF-c landed seeking**: the Simple Index Object is
//! parsed on first seek (the ffmpeg-measured layout — see
//! [`data`]'s module docs on the [VERIFY-P0] gate) and maps time →
//! packet; indexless files get the bounded linear scan
//! ([`MAX_LINEAR_SEEK_PACKETS`]) with the honest `seek.too-slow`
//! refusal past the budget.
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
//! trait-order violations are `demux.asf.not-started`; packet
//! grammar refusals are `demux.asf.packet.bad-flags` /
//! `packet.bad-length` / `payload.bad-replication` /
//! `payload.oversized` / `packet.variable-size-unsupported`; seek
//! refusals are `demux.asf.seek.out-of-range` / `seek.too-slow` /
//! `index-too-large` (the `seek.unsupported` code stays minted for
//! the error-table history — no site mints it anymore).

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
/// // The Data Object carries no packets here: the walk ends at
/// // once, honestly (never a fake empty-stream shape).
/// assert_eq!(asf.next_event()?, DemuxEvent::EndOfStream);
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
    /// The parsed Simple Index (ASF-c), loaded lazily on first seek.
    simple_index: Option<Option<SimpleIndex>>,
}

/// The bounded linear-seek scan budget (indexless files): the
/// maximum data packets a no-index seek will walk header-by-header
/// before refusing `seek.too-slow` (the mkv precedent — the engine's
/// SlowLinear contract, MP §12 discipline applied to scan time).
pub const MAX_LINEAR_SEEK_PACKETS: u64 = 4096;

/// The largest Simple Index Object trusted into memory (MP §12):
/// 1 MiB is 174 762 one-second buckets — two full days of index.
pub const MAX_INDEX_BYTES: u64 = 1 << 20;

/// One Simple Index Object entry: the packet number its interval
/// bucket lands at (the ffmpeg-measured 6-byte entry form — DWORD
/// packet number, WORD count; the number is non-decreasing across
/// every measured file, the count is carried but unused by the
/// landing rule).
#[derive(Debug, Clone, Copy)]
struct IndexEntry {
    packet_number: u32,
    #[allow(dead_code)]
    packet_count: u16,
}

/// The parsed Simple Index Object (card 6.ASF-c).
///
/// Layout as measured against ffmpeg 7.1.5 muxer output (the
/// [VERIFY-P0] spec gate never opened — same discipline as the rest
/// of this crate): GUID(16) + size(8) + file id(16) + interval
/// QWORD (100 ns; ffmpeg writes 10 000 000 = 1 s) + one measured
/// DWORD (0 or 1 across the corpus, semantics unpinned — carried
/// raw) + entry count DWORD + `count` entries of (DWORD packet
/// number, WORD packet count). The object must close exactly: size
/// == 56 + 6 × count.
#[derive(Debug)]
struct SimpleIndex {
    /// The per-entry interval in 100 ns units (≥ 1 enforced).
    interval_100ns: u64,
    /// The entries, bucket order.
    entries: Vec<IndexEntry>,
}

impl SimpleIndex {
    /// Parse from the object's own bytes (GUID+size already included
    /// in `bytes`; the object was extent-validated by the header
    /// walk's top-object map).
    ///
    /// # Errors
    /// `demux.asf.bad-object-size` for a non-closing shape or a zero
    /// interval; `demux.asf.index-too-large` above the memory
    /// ceiling.
    fn parse(bytes: &[u8], offset: u64) -> Result<SimpleIndex, EngineError> {
        if bytes.len() < 56 || (bytes.len() - 56) % 6 != 0 {
            return Err(error::bad_object_size(offset));
        }
        let count = u32::from(bytes[52])
            | (u32::from(bytes[53]) << 8)
            | (u32::from(bytes[54]) << 16)
            | (u32::from(bytes[55]) << 24);
        if usize::try_from(count).unwrap_or(usize::MAX) != (bytes.len() - 56) / 6 {
            return Err(error::bad_object_size(offset));
        }
        let interval_100ns = u64::from(bytes[40])
            | (u64::from(bytes[41]) << 8)
            | (u64::from(bytes[42]) << 16)
            | (u64::from(bytes[43]) << 24)
            | (u64::from(bytes[44]) << 32)
            | (u64::from(bytes[45]) << 40)
            | (u64::from(bytes[46]) << 48)
            | (u64::from(bytes[47]) << 56);
        if interval_100ns == 0 {
            return Err(error::bad_object_size(offset));
        }
        let mut entries = Vec::with_capacity((bytes.len() - 56) / 6);
        let mut at = 56;
        while at + 6 <= bytes.len() {
            let number = u32::from(bytes[at])
                | (u32::from(bytes[at + 1]) << 8)
                | (u32::from(bytes[at + 2]) << 16)
                | (u32::from(bytes[at + 3]) << 24);
            let count = u16::from(bytes[at + 4]) | (u16::from(bytes[at + 5]) << 8);
            entries.push(IndexEntry {
                packet_number: number,
                packet_count: count,
            });
            at += 6;
        }
        Ok(SimpleIndex {
            interval_100ns,
            entries,
        })
    }

    /// The landing bucket for a zero-based target (the buckets ride
    /// the preroll-offset clock — measured: a 7 s file with 3.1 s
    /// preroll carries 11 one-second buckets, 7 + 3.1 rounded up).
    fn bucket_of(&self, target_ms: u64, preroll_ms: u64) -> usize {
        let interval_ms = self.interval_100ns / 10_000;
        let interval_ms = interval_ms.max(1);
        let index_ms = target_ms.saturating_add(preroll_ms);
        let bucket = (index_ms / interval_ms) as usize;
        bucket.min(self.entries.len().saturating_sub(1))
    }
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
            simple_index: None,
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
    fn build_walk(parsed: &header::AsfHeader, tracks: &[Track]) -> Result<PacketWalk, EngineError> {
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

    /// The walk-end half of the assembler's hold-and-flush contract:
    /// objects a first fragment completed stay held for the stream's
    /// next fragment (the proof the container moved on); when the
    /// walk ends without one, they release here instead of
    /// stranding — queued before the `EndOfStream`.
    fn drain_held(&mut self) {
        let Some(walk) = self.walk.as_mut() else {
            return;
        };
        let preroll_ms = walk.preroll_ms;
        for slot in 0..walk.bases.len() {
            let Some(base) = walk.bases.get(slot).copied() else {
                continue;
            };
            let Some(track) = self.tracks.as_ref().and_then(|t| t.get(slot)) else {
                continue;
            };
            let Some((presentation_time_ms, keyframe, buffer)) =
                walk.assembler.take_completed(slot)
            else {
                continue;
            };
            // Preroll subtraction (same arithmetic as the walk —
            // ffprobe's zero-based pts is the check).
            let preroll = u32::try_from(preroll_ms).unwrap_or(u32::MAX);
            let pts_ms = presentation_time_ms.saturating_sub(preroll);
            let ticks = Self::ticks_for(track, base, pts_ms);
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

    /// Locate + read + parse the Simple Index Object (once; cached as
    /// `None` for indexless files so audio-only seeks take the linear
    /// lane without re-scanning the top-object map).
    ///
    /// # Errors
    /// Propagates the reader's truncation and [`SimpleIndex::parse`]'s
    /// typed refusals; `demux.asf.index-too-large` above the ceiling.
    fn load_simple_index(&mut self) -> Result<Option<&SimpleIndex>, EngineError> {
        if self.simple_index.is_none() {
            let located = self.header.as_ref().and_then(|h| {
                h.top_objects
                    .iter()
                    .find(|o| o.guid == guid::SIMPLE_INDEX)
                    .map(|o| (o.offset, o.size))
            });
            let parsed = match located {
                Some((offset, size)) => {
                    if size > MAX_INDEX_BYTES {
                        return Err(error::index_too_large(MAX_INDEX_BYTES, size));
                    }
                    let len = usize::try_from(size).unwrap_or(usize::MAX);
                    let Some(bytes) = self.reader.read_exact_or_end_at(len, offset)? else {
                        return Err(error::truncated(offset + size, offset));
                    };
                    Some(SimpleIndex::parse(&bytes, offset)?)
                }
                None => None,
            };
            self.simple_index = Some(parsed);
        }
        Ok(self
            .simple_index
            .as_ref()
            .and_then(|cached| cached.as_ref()))
    }

    /// Re-anchor the packet walk at `packet` (the seek commit: drop
    /// half-delivered fragments and queued objects, restart the
    /// cursor, re-arm the flush endpoint).
    fn reanchor(&mut self, packet: u64) {
        if let Some(walk) = self.walk.as_mut() {
            walk.next_packet = packet.min(walk.packets_total);
            walk.assembler.clear();
            walk.queue.clear();
            walk.eos = false;
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
                let packet = self.walk.as_mut().and_then(|w| w.queue.pop_front());
                if let Some(packet) = packet {
                    return Ok(DemuxEvent::Packet(packet));
                }
                continue;
            }
            if exhausted {
                let flush_now = {
                    let Some(walk) = self.walk.as_mut() else {
                        return Ok(DemuxEvent::EndOfStream);
                    };
                    let first_end = !walk.eos;
                    walk.eos = true;
                    first_end
                };
                if flush_now {
                    // EOS drain (hold-and-flush): release objects a
                    // first fragment completed whose follow-up never
                    // came, then re-run the loop so the queue empties
                    // before the EndOfStream.
                    self.drain_held();
                    continue;
                }
                return Ok(DemuxEvent::EndOfStream);
            }
            let index = self.walk.as_ref().map_or(0, |w| w.next_packet);
            self.walk_one_packet(index)?;
            if let Some(walk) = self.walk.as_mut() {
                walk.next_packet += 1;
            }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        // The target clock: any base converts onto the millisecond
        // demux-clock (the index's own clock); a missing start is the
        // honest rewind-to-zero (the mkv `target_ticks` rule).
        let ms_base = TimeBase::new(1, 1000).map_err(|_| error::bad_object_size(0))?;
        let target_ms = match &target.start {
            Stamp::Present(ts) => ts
                .convert(&ms_base)
                .map(|t| t.ticks.max(0))
                .map_err(|_| error::seek_out_of_range())?,
            Stamp::Missing => 0,
        };
        let preroll = self.walk.as_ref().map_or(0, |w| w.preroll_ms);
        if let Some(index) = self.load_simple_index()? {
            // The indexed landing: bucket → entry packet number →
            // re-anchor. Targets beyond the last bucket clamp to it
            // (the ffmpeg/VLC contract — the walk's own extent bounds
            // us, never a fabricated position).
            let bucket = index.bucket_of(target_ms as u64, preroll);
            let interval_ms = (index.interval_100ns / 10_000).max(1);
            let packet = u64::from(index.entries.get(bucket).map_or(0, |e| e.packet_number));
            self.reanchor(packet);
            let landing_ms = (bucket as u64 * interval_ms).saturating_sub(preroll);
            return Ok(TimeSpan {
                start: Stamp::Present(Timestamp::new(landing_ms as i64, ms_base)),
                end: target.end,
            });
        }
        // No index: the bounded linear scan (header-only reads, send
        // times compared on the preroll-offset clock). Past the
        // budget the honest refusal — never an unbounded walk.
        let (packets_total, packet_size, payload_offset) =
            self.walk.as_ref().map_or((0, 0, 0), |w| {
                (w.packets_total, w.packet_size, w.payload_offset)
            });
        if packets_total > MAX_LINEAR_SEEK_PACKETS {
            return Err(error::seek_too_slow());
        }
        let mut landing_packet = packets_total;
        let mut landing_ms = 0_u64;
        for index in 0..packets_total {
            let at = payload_offset + index * packet_size;
            let bytes = self
                .reader
                .read_exact_or_end_at(packet_size as usize, at)?
                .ok_or_else(|| error::truncated(at + packet_size, at))?;
            let parsed = data::parse_packet(&bytes)?;
            if u64::from(parsed.send_time_ms) >= target_ms as u64 + preroll {
                landing_packet = index;
                landing_ms = u64::from(parsed.send_time_ms).saturating_sub(preroll);
                break;
            }
            landing_ms = u64::from(parsed.send_time_ms).saturating_sub(preroll);
        }
        self.reanchor(landing_packet);
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(landing_ms as i64, ms_base)),
            end: target.end,
        })
    }

    fn seekability(&self) -> SeekSupport {
        if !self.started {
            return SeekSupport::None;
        }
        // §4.3 policy: a located Simple Index is a real seek table →
        // Fast{indexed}; indexless files (audio-only, broadcast
        // survivors) ride the bounded linear lane → SlowLinear.
        let indexed = self
            .header
            .as_ref()
            .is_some_and(|h| h.top_objects.iter().any(|o| o.guid == guid::SIMPLE_INDEX));
        if indexed {
            SeekSupport::Fast { indexed: true }
        } else {
            SeekSupport::SlowLinear
        }
    }
}
