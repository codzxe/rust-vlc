//! The AIFF demuxer (card 5.A2): FORM/AIFF + AIFC — COMM (including
//! the 80-bit extended-float sample rate), SSND with its
//! offset/blockSize pair, and MARK/INST/COMT chunks skipped
//! positionally (skip-tolerant).
//!
//! # What this is
//!
//! [`AiffDemuxer`] implements the landed
//! [`plugin_api::Demuxer`](plugin_api::Demuxer) contract over any
//! seekable [`SourceStream`](plugin_api::SourceStream). The byte
//! layouts it reads are the ones ffmpeg's AIFF muxer emits (the
//! committed fixtures are ffmpeg products; the differential suite
//! pins the demuxed PCM against ffmpeg's raw dumps).
//!
//! # Design (recorded for review — the WAV twin's shape)
//!
//! - **AIFF is IFF: big-endian chunk headers, even payloads.** The
//!   byte plumbing is shared with the RIFF family
//!   ([`SourceReader`](crate::avi::riff::SourceReader) is
//!   endian-agnostic); only the header parse and the PCM byte order
//!   differ (see [`chunks`]).
//! - **The SSND sound extent is the packet-extraction truth.** Sound
//!   data starts `offset` bytes past SSND's 8-byte header pair;
//!   `next_event` frames it into whole sample-frame packets of at
//!   most [`MAX_PACKET_BYTES`] (the WAV packetizer's arithmetic,
//!   big-endian payload). COMM's `numSampleFrames` is the *declared*
//!   count; the byte extent is authority — a disagreement is
//!   damage-noted.
//! - **Declared sizes are hints; the stream is authority** (D-3/D-9):
//!   extents clamp to the FORM extent ∩ stream; an SSND cut by the
//!   end of the file delivers its whole-frame prefix and then fails
//!   typed (`error::truncated`).
//! - **The rate is an 80-bit extended float, self-built**
//!   ([`chunks::extended80_to_f64`]): only normalized, finite,
//!   positive integral values are accepted — denormals, the Apple
//!   pseudo-zero, infinities, NaNs, unnormals and fractional rates
//!   are typed structural refusals (no guessed rates, ever).
//! - **AIFC compression vocabulary**: `NONE` (big-endian PCM),
//!   `sowt` (little-endian PCM — doc 04's [VERIFY] corner, resolved
//!   as pcm_s16/24/32le), `fl32`/`fl64` (big-endian float), `raw `
//!   (unsigned 8-bit — the ffmpeg AIFC mapping for `pcm_u8`).
//!   Classic AIFF's 8-bit samples are *signed* — no registry
//!   vocabulary, the typed refusal. `ulaw`/`alaw`/`ima4` and friends:
//!   typed `unsupported-compression` (the codecs do not exist in this
//!   engine; pretending to demux them helps nobody).
//! - **Seeking is arithmetic** (CBR): the landing is the exact
//!   sample frame.
//! - **Bounded memory** (MP §12): payloads ride the pooled
//!   [`PayloadSlots`](crate::avi::riff::PayloadSlots) arena
//!   (`2 × 64 KiB`, one copy per payload), COMM reads are capped
//!   ([`chunks::MAX_COMM_BYTES`]), and every walk position only
//!   advances.

use std::collections::VecDeque;

use media_types::error::ErrorContext;
use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::header::DamageLog;
use crate::avi::riff::{FourCC, PayloadSlots, SourceReader, pad_for};

pub mod chunks;
pub mod error;
pub mod fuzz;
pub mod test_support;

#[cfg(test)]
mod tests;

/// Maximum packet payload trusted into memory per `next_event`
/// (the WAV demuxer's twin constant — same rationale).
pub const MAX_PACKET_BYTES: u64 = 64 * 1024;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["aiff", "aif", "aifc"];

/// The probe: do the leading bytes look like FORM/AIFF or FORM/AIFC?
///
/// Recognizes the 12-byte head `FORM <size> AIFF|AIFC`. Returns
/// `false` for anything shorter — a probe never claims a format it
/// cannot see.
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    if header.len() < 12 {
        return false;
    }
    let magic = FourCC::new([header[0], header[1], header[2], header[3]]);
    let form = FourCC::new([header[8], header[9], header[10], header[11]]);
    magic == chunks::FORM && (form == chunks::FORM_AIFF || form == chunks::FORM_AIFC)
}

/// The parsed sound region, absolute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SoundRegion {
    /// The SSND chunk's payload start (the walk's identity anchor
    /// for the live SSND).
    payload_start: u64,
    /// First sound byte (SSND payload + 8 + offset).
    data_start: u64,
    /// The **declared** end (SSND chunk end, unclamped).
    declared_end: u64,
    /// The effective end (declared ∩ FORM extent ∩ stream).
    effective_end: u64,
}

/// A replayable error shape (the WAV twin — see its docs).
#[derive(Debug, Clone, PartialEq, Eq)]
struct StickyShape {
    stage: media_types::PipelineStage,
    kind: media_types::ErrorKind,
    context: ErrorContext,
}

impl StickyShape {
    fn of(err: &EngineError) -> StickyShape {
        StickyShape {
            stage: err.stage,
            kind: err.kind.clone(),
            context: err.context.clone(),
        }
    }

    fn mint(&self) -> EngineError {
        EngineError::new(self.stage, self.kind.clone(), self.context.clone())
    }
}

/// The AIFF demuxer.
///
/// Construct with [`AiffDemuxer::new`] over a seekable source stream,
/// then drive it through the [`Demuxer`] trait: `start` parses the
/// FORM walk (COMM/SSND + skip-tolerant chunks) and enumerates the
/// one audio track; `next_event` yields whole-sample-frame PCM
/// packets; `seek` maps time → sample-frame arithmetically.
///
/// # Example
///
/// ```
/// use demux::aiff::AiffDemuxer;
/// use plugin_api::{Demuxer, DemuxEvent};
///
/// # fn main() -> Result<(), media_types::EngineError> {
/// # let bytes = aiff_demo_bytes();
/// let mut stream = demux::avi::test_support::mem_stream(bytes)?;
/// let mut aiff = AiffDemuxer::new(stream)?;
/// let tracks = aiff.start()?;
/// assert_eq!(tracks.len(), 1);
/// assert_eq!(tracks[0].codec.as_str(), "pcm_s16be");
/// match aiff.next_event()? {
///     DemuxEvent::Packet(p) => assert!(p.keyframe),
///     _ => unreachable!("a non-empty SSND packets first"),
/// }
/// # Ok(())
/// # }
/// # fn aiff_demo_bytes() -> std::sync::Arc<[u8]> {
/// #   demux::aiff::test_support::minimal_aiff(1, 44_100, 16, &[0x12, 0x34])
/// # }
/// ```
pub struct AiffDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    /// The parsed COMM facts.
    comm: Option<chunks::CommChunk>,
    /// The verbatim COMM payload (≤ [`chunks::MAX_COMM_BYTES`]) —
    /// the engine passes it as `DecoderConfig::extra_data` (the WAV
    /// `fmt ` seam's twin).
    comm_bytes: Option<SharedBytes>,
    sound: Option<SoundRegion>,
    /// The SSND offset/blockSize pair (observability).
    ssnd_header: Option<(u32, u32)>,
    damage: DamageLog,
    /// Next sound byte to read (absolute position).
    walk: u64,
    /// Sample frames already emitted (the PTS ladder).
    frames_emitted: u64,
    pending: VecDeque<DemuxEvent>,
    tracks: Option<Vec<Track>>,
    pool: PayloadSlots,
}

impl AiffDemuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// No container I/O happens here — this only rejects sources that
    /// cannot carry an AIFF: unknown length, unseekable, empty.
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for those three cases.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<AiffDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(AiffDemuxer {
            reader,
            started: false,
            eos: false,
            sticky: None,
            comm: None,
            comm_bytes: None,
            sound: None,
            ssnd_header: None,
            damage: DamageLog::default(),
            walk: 0,
            frames_emitted: 0,
            pending: VecDeque::new(),
            tracks: None,
            pool: PayloadSlots::new(2, MAX_PACKET_BYTES as usize),
        })
    }

    /// The parsed COMM facts — the decoder-config seam (start must
    /// have run).
    #[must_use]
    pub fn comm(&self) -> Option<chunks::CommChunk> {
        self.comm
    }

    /// The verbatim COMM chunk payload — the bytes the engine hands
    /// the decoder as `DecoderConfig::extra_data`.
    #[must_use]
    pub fn comm_chunk_bytes(&self) -> Option<&[u8]> {
        self.comm_bytes.as_deref()
    }

    /// The SSND offset/blockSize pair, when an SSND was parsed.
    #[must_use]
    pub fn ssnd_offset_block(&self) -> Option<(u32, u32)> {
        self.ssnd_header
    }

    /// Total bytes skipped by bounded recovery so far (the
    /// `Recovered` events' sum).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// The payload bytes resident in the packet pool (the D-1
    /// observable — bounded by `2 × MAX_PACKET_BYTES`).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// The effective sample-frame count of the sound extent.
    #[must_use]
    pub fn frames(&self) -> Option<u64> {
        let region = self.sound?;
        let comm = self.comm?;
        let frame = u64::from(frame_size(&comm));
        Some((region.effective_end.saturating_sub(region.data_start)) / frame)
    }

    /// Record a fatal error: sticky + returned.
    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    /// Queue the damage log's events for `next_event` to surface.
    fn surface_damage(&mut self) {
        for (skipped, reason) in std::mem::take(&mut self.damage.events) {
            self.pending
                .push_back(DemuxEvent::Recovered { skipped, reason });
        }
    }

    /// Hand a walk event to the caller, damage-record-first (the AVI
    /// `emit` contract).
    fn emit(&mut self, event: DemuxEvent) -> Option<DemuxEvent> {
        if self.pending.is_empty() {
            return Some(event);
        }
        self.pending.push_back(event);
        self.pending.pop_front()
    }

    /// The full header parse (the body of the trait's `start`).
    fn parse_container(&mut self) -> Result<Vec<Track>, EngineError> {
        // `FORM <size> AIFF|AIFC`. A recognized-magic cut inside the
        // head is truncation; anything else never promised AIFF.
        let head = self.reader.read_prefix_at(12, 0)?;
        let recognized_magic = head.starts_with(chunks::FORM.as_bytes());
        if head.len() < 12 {
            if recognized_magic {
                return Err(error::truncated(12, head.len() as u64));
            }
            return Err(error::not_aiff(&head));
        }
        let form = FourCC::new([head[8], head[9], head[10], head[11]]);
        let aifc = match form {
            f if f == chunks::FORM_AIFF => false,
            f if f == chunks::FORM_AIFC => true,
            _ => return Err(error::not_aiff(&head)),
        };

        let declared_size = u64::from(chunks::u32be(&head, 4));
        let declared_end = 8u64.saturating_add(declared_size);
        let extent_end = declared_end.min(self.reader.length());
        if declared_end > self.reader.length() {
            self.damage.note(
                declared_end - self.reader.length(),
                "form-extends-past-stream",
            );
        }

        // Chunk walk: COMM and SSND parsed; everything else (MARK,
        // INST, COMT, NAME, AUTH, ANNO, `(c) `, APPL, JUNK …) skips
        // positionally — skip-tolerant by design.
        let mut pos = 12u64;
        while pos + 8 <= extent_end {
            let Some(header_bytes) = self.reader.read_exact_or_end_at(8, pos)? else {
                return Err(error::truncated(pos + 8, self.reader.length()));
            };
            let mut raw = [0u8; 8];
            raw.copy_from_slice(&header_bytes);
            let id = FourCC::new([raw[0], raw[1], raw[2], raw[3]]);
            let size = u64::from(chunks::u32be(&raw, 4));
            let payload_start = pos + 8;
            let next = payload_start
                .saturating_add(size)
                .saturating_add(pad_for(size));

            if id == chunks::COMM {
                if self.comm.is_some() {
                    self.damage.note(size, "duplicate-comm");
                } else if size < chunks::COMM_MIN_BYTES as u64 {
                    return Err(error::bad_structure(pos));
                } else {
                    let want = (size as usize)
                        .min(chunks::MAX_COMM_BYTES)
                        .min((extent_end.saturating_sub(payload_start)) as usize);
                    let bytes = self.reader.read_prefix_at(want, payload_start)?;
                    let floor = if aifc {
                        chunks::COMM_AIFC_MIN_BYTES
                    } else {
                        chunks::COMM_MIN_BYTES
                    };
                    if bytes.len() < floor {
                        return Err(error::truncated(
                            payload_start + floor as u64,
                            payload_start + bytes.len() as u64,
                        ));
                    }
                    let comm = chunks::parse_comm(&bytes, aifc, pos)?;
                    self.comm_bytes = Some(SharedBytes::from_vec(bytes));
                    self.comm = Some(comm);
                }
            } else if id == chunks::SSND {
                if self.sound.is_some() {
                    self.damage.note(size, "duplicate-ssnd");
                } else {
                    let want = (size.min(8) as usize)
                        .min((extent_end.saturating_sub(payload_start)) as usize);
                    let bytes = self.reader.read_prefix_at(want, payload_start)?;
                    if bytes.len() < 8 {
                        return Err(error::truncated(
                            payload_start + 8,
                            payload_start + bytes.len() as u64,
                        ));
                    }
                    let (offset, block_size) = chunks::parse_ssnd(&bytes, pos)?;
                    self.ssnd_header = Some((offset, block_size));
                    let data_start = payload_start + 8 + u64::from(offset);
                    let declared_end = payload_start.saturating_add(size);
                    self.sound = Some(SoundRegion {
                        payload_start,
                        data_start,
                        declared_end,
                        effective_end: data_start,
                    });
                }
            }
            // MARK / INST / COMT / NAME / AUTH / ANNO / (c) / APPL /
            // unknown: positional skip (the skip-tolerant contract —
            // instrument and marker chunks never gate the sound).

            // The live SSND's over-declaration is attributed once — by
            // the sound-extent clamp after the walk
            // (`ssnd-extends-past-stream`) — never under two reasons
            // for the same missing bytes.
            let is_live_ssnd = id == chunks::SSND
                && self
                    .sound
                    .as_ref()
                    .is_some_and(|region| region.payload_start == payload_start);
            if next > extent_end {
                if !is_live_ssnd {
                    self.damage
                        .note(next - extent_end, "chunk-extends-past-form");
                }
                self.surface_damage();
                pos = extent_end;
            } else {
                pos = next;
            }
        }

        let physically_short = declared_end > self.reader.length();
        let Some(comm) = self.comm else {
            if physically_short {
                return Err(error::truncated(declared_end, self.reader.length()));
            }
            return Err(error::no_comm());
        };
        let Some(region) = self.sound else {
            if physically_short {
                return Err(error::truncated(declared_end, self.reader.length()));
            }
            return Err(error::no_ssnd());
        };

        // The sound extent: declared ∩ FORM extent ∩ stream, floored
        // at the offset-shifted data start.
        let effective_end = region
            .declared_end
            .min(extent_end)
            .min(self.reader.length())
            .max(region.data_start);
        if region.declared_end > effective_end {
            self.damage.note(
                region.declared_end - effective_end,
                "ssnd-extends-past-stream",
            );
        }
        let frame = u64::from(frame_size(&comm));
        let available = effective_end.saturating_sub(region.data_start);
        let frames = available.checked_div(frame).unwrap_or(0);
        if frames == 0 {
            if physically_short {
                return Err(error::truncated(declared_end, self.reader.length()));
            }
            return Err(error::no_ssnd());
        }

        // COMM's declared frame count vs the bytes: bytes are
        // authority; a disagreement is counted.
        if u64::from(comm.num_frames) != frames {
            let (bigger, smaller) = if u64::from(comm.num_frames) > frames {
                (u64::from(comm.num_frames), frames)
            } else {
                (frames, u64::from(comm.num_frames))
            };
            self.damage.note(
                (bigger - smaller).saturating_mul(frame),
                if u64::from(comm.num_frames) > frames {
                    "comm-frames-past-ssnd"
                } else {
                    "comm-frames-short-of-ssnd"
                },
            );
        }

        self.sound = Some(SoundRegion {
            effective_end,
            ..region
        });

        let codec = CodecDescriptor::new(codec_of(&comm)).map_err(|_| error::bad_comm(0))?;
        let base = TimeBase::new(1, comm.sample_rate).map_err(|_| error::bad_comm(0))?;
        let track = Track {
            id: TrackId::new(0),
            stream: StreamId::new(0),
            kind: MediaKind::Audio,
            language: None,
            codec,
            title: None,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Present(Timestamp::new(0, base)),
                    end: Stamp::Present(Timestamp::new(frames.min(i64::MAX as u64) as i64, base)),
                },
                base,
                gaps: Vec::new(),
            },
        };
        self.walk = region.data_start;
        Ok(vec![track])
    }

    /// The PCM packetizer core (the `next_event` body): one
    /// whole-sample-frame packet per call, ≤ [`MAX_PACKET_BYTES`].
    fn step(&mut self) -> Result<Option<DemuxEvent>, EngineError> {
        let Some(region) = self.sound else {
            return Err(self.fail(error::no_ssnd()));
        };
        let Some(comm) = self.comm else {
            return Err(self.fail(error::no_comm()));
        };
        let frame = u64::from(frame_size(&comm));
        // Structurally unreachable: the COMM parse enforces rate ≥ 1.
        let base = TimeBase::new(1, comm.sample_rate).map_err(|_| error::bad_structure(0))?;

        if self.walk >= region.effective_end {
            if region.effective_end < region.declared_end {
                return Err(self.fail(error::truncated(region.declared_end, region.effective_end)));
            }
            self.eos = true;
            return Ok(None);
        }

        let available = region.effective_end - self.walk;
        let whole_bytes = (available / frame) * frame;
        if whole_bytes == 0 {
            self.damage.note(available, "ssnd-not-frame-aligned");
            self.surface_damage();
            self.walk = region.effective_end;
            return Ok(None);
        }
        let want_frames = (whole_bytes / frame).min(MAX_PACKET_BYTES / frame);
        let want_bytes = (want_frames * frame) as usize;
        let read_from = self.walk;
        let payload = self.pool.fill(want_bytes, |out| {
            self.reader.read_prefix_into(out, read_from)
        });
        let payload = match payload {
            Ok(payload) => payload,
            Err(err) => return Err(self.fail(err)),
        };
        let delivered = payload.len() as u64;
        if delivered < want_bytes as u64 {
            let whole = (delivered / frame) * frame;
            self.damage
                .note(want_bytes as u64 - whole, "payload-read-short");
            self.surface_damage();
            if whole > 0 {
                self.walk += whole;
                self.frames_emitted += whole / frame;
            } else {
                self.walk = region.effective_end;
            }
            return Ok(None);
        }
        self.walk += want_bytes as u64;
        let pts_ticks = self.frames_emitted.min(i64::MAX as u64) as i64;
        let pts = Stamp::Present(Timestamp::new(pts_ticks, base));
        let duration = match Timestamp::new(want_frames as i64, base).as_nanos() {
            Ok(Nanoseconds(nanos)) => Some(Duration::Known(Nanoseconds(nanos))),
            Err(_) => {
                self.damage.note(0, "packet-duration-unrepresentable");
                self.surface_damage();
                None
            }
        };
        self.frames_emitted += want_frames;
        let packet = Packet {
            track: TrackId::new(0),
            pts,
            dts: pts,
            duration,
            data: payload,
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
        Ok(self.emit(DemuxEvent::Packet(packet)))
    }

    /// The shared seek computation (the WAV twin): validation + the
    /// sample-frame landing, never committing.
    fn locate(&self, target: TimeSpan) -> Result<(u64, u64), EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        if let Some(sticky) = &self.sticky {
            return Err(sticky.mint());
        }
        let Some(comm) = self.comm else {
            return Err(error::not_started());
        };
        let Some(region) = self.sound else {
            return Err(error::seek_out_of_range());
        };
        let base = TimeBase::new(1, comm.sample_rate).map_err(|_| error::bad_comm(0))?;
        let target_ts = target
            .start
            .timestamp()
            .map_err(|_| error::seek_out_of_range())?;
        if target_ts.ticks < 0 {
            return Err(error::seek_out_of_range());
        }
        let converted = target_ts
            .convert(&base)
            .map_err(|_| error::seek_out_of_range())?;
        let frame = u64::from(frame_size(&comm));
        let total_frames = region.effective_end.saturating_sub(region.data_start) / frame;
        // A target past the stream's end is the typed refusal (the
        // AVI seek contract — the house precedent); landing exactly
        // at the end is legal (the next event is EOS).
        if total_frames > 0 && converted.ticks > total_frames as i64 {
            return Err(error::seek_out_of_range());
        }
        let land = (converted.ticks as u64).min(total_frames);
        Ok((land, region.data_start + land * frame))
    }

    /// The reported landing span.
    fn landing_span(&self, frame: u64) -> Result<TimeSpan, EngineError> {
        let Some(comm) = self.comm else {
            return Err(error::not_started());
        };
        let base = TimeBase::new(1, comm.sample_rate).map_err(|_| error::bad_comm(0))?;
        let end = self.frames().map_or(Stamp::Missing, |frames| {
            Stamp::Present(Timestamp::new(frames.min(i64::MAX as u64) as i64, base))
        });
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(frame.min(i64::MAX as u64) as i64, base)),
            end,
        })
    }
}

/// Bytes per sample frame from the COMM facts (channels × sample
/// width of the resolved codec).
fn frame_size(comm: &chunks::CommChunk) -> u32 {
    let bytes = match comm.compression {
        Some((_, codec)) => u32::from(codec.bytes_per_sample()),
        None => u32::from(comm.sample_size.div_ceil(8)),
    };
    u32::from(comm.channels).saturating_mul(bytes)
}

/// The registry descriptor for the COMM facts.
fn codec_of(comm: &chunks::CommChunk) -> &'static str {
    match comm.compression {
        Some((_, codec)) => codec.descriptor(),
        None => match comm.sample_size.div_ceil(8) {
            2 => "pcm_s16be",
            3 => "pcm_s24be",
            _ => "pcm_s32be",
        },
    }
}

impl Demuxer for AiffDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if let Some(tracks) = &self.tracks {
            return Ok(tracks.clone());
        }
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        match self.parse_container() {
            Ok(tracks) => {
                self.started = true;
                self.tracks = Some(tracks.clone());
                self.surface_damage();
                Ok(tracks)
            }
            Err(err) => {
                self.sticky = Some(StickyShape::of(&err));
                Err(err)
            }
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        if self.eos {
            return Ok(DemuxEvent::EndOfStream);
        }
        match self.step() {
            Ok(Some(event)) => Ok(event),
            Ok(None) => {
                if let Some(event) = self.pending.pop_front() {
                    return Ok(event);
                }
                Ok(DemuxEvent::EndOfStream)
            }
            Err(err) => Err(err),
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        let (frame, _byte) = self.locate(target)?;
        self.landing_span(frame)
    }

    fn seekability(&self) -> SeekSupport {
        if !self.started || self.sticky.is_some() || self.sound.is_none() || self.comm.is_none() {
            return SeekSupport::None;
        }
        SeekSupport::Fast { indexed: false }
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        let (frame, byte) = self.locate(target)?;
        self.walk = byte;
        self.frames_emitted = frame;
        self.pending.clear();
        self.eos = false;
        self.landing_span(frame)
    }
}
