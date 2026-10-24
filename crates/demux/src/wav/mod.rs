//! The WAV demuxer (card 5.A2): RIFF/WAVE — plain PCM, IEEE float,
//! WAVE_FORMAT_EXTENSIBLE, `fact`, RF64/BW64 with `ds64`, LIST/INFO
//! metadata, odd-chunk word-align padding, IMA/MS ADPCM and
//! FLAC-in-WAV block framing.
//!
//! # What this is
//!
//! [`WavDemuxer`] implements the landed
//! [`plugin_api::Demuxer`](plugin_api::Demuxer) contract over any
//! seekable [`SourceStream`](plugin_api::SourceStream). The byte
//! layouts it reads are the ones ffmpeg's WAV/RF64 muxers emit (the
//! committed fixtures under `tests/fixtures/generated/` are ffmpeg
//! products; the differential suite pins the demuxed payload against
//! ffmpeg's raw dumps, and the corpus suite runs the twelve real
//! randomized WAVs).
//!
//! # Design (recorded for review)
//!
//! - **The `data` extent is the packet-extraction truth.** PCM has
//!   no packet structure: `next_event` frames the data extent into
//!   whole sample-frame packets of at most [`MAX_PACKET_BYTES`]
//!   (constant bit-rate, so the framing is pure arithmetic). ADPCM
//!   frames at block granularity (one `nBlockAlign` block per unit,
//!   `wSamplesPerBlock` frames each — the block is the codec's
//!   self-contained decode unit). FLAC-in-WAV frames at a byte
//!   quantum with **no** frame alignment `[PROPOSAL: frame-aligned
//!   FLAC-in-WAV packetization needs the FLAC frame-header grammar
//!   and belongs with the FLAC codec card (5.A1); the demuxer's
//!   packet contract there is the seam]`. The `fact` chunk's
//!   `dwSampleLength` is the *declared* frame count; the byte extent
//!   is authority — a declaration that disagrees with the bytes is
//!   damage-noted, the shorter extent wins.
//! - **Declared sizes are hints; the stream is authority** (the AVI
//!   precedent, D-3/D-9): every extent is clamped to its parent's
//!   and the stream's; a `data` declaration that runs past the file
//!   delivers its real available prefix (frame-floored) and then
//!   fails *typed* (`error::truncated`) after the packets — never a
//!   silent EOS-with-lost-tail. A `data` size sentinel
//!   (`0xFFFFFFFF`) outside RF64/BW64 means "to the end of the
//!   stream" — the streaming-writer convention — and is
//!   damage-noted, never read as 4 GiB. An RF64/BW64 head without
//!   its mandatory `ds64` is the one structural lie that refuses at
//!   `start` (the size truth never arrived).
//! - **One accepted-set with the decoder.** The PCM half of the
//!   `fmt ` parse mirrors `codecs::audio::pcm`'s blob rules exactly
//!   (tag/GUID resolution, width vocabulary, `nBlockAlign ==
//!   channels × width`), and the verbatim `fmt ` payload rides the
//!   engine's `DecoderConfig::extra_data`, where the PCM decoder
//!   re-parses it. ADPCM and FLAC-in-WAV enumerate tracks with
//!   their honest registry names (`adpcm_ima_wav`, `adpcm_ms`,
//!   `flac`) — codecs this engine does not carry yet; the engine
//!   reports the decode gap, the demuxer never fakes it.
//! - **Seeking is arithmetic** (CBR): PCM lands on the exact sample
//!   frame, ADPCM on the containing block boundary (blocks are the
//!   self-contained decode units), FLAC-in-WAV honestly refuses
//!   (variable-bitrate frames without a frame grammar — the typed
//!   `SeekSupport::None`, never a fake landing).
//! - **Bounded memory** (MP §12): payloads are read through the
//!   pooled [`PayloadSlots`](crate::avi::riff::PayloadSlots) arena
//!   (`2 × 64 KiB`, one copy per payload — source → arena → packet
//!   view), metadata reads are capped
//!   ([`format::MAX_FMT_BYTES`], [`MAX_INFO_BYTES`]), and every walk
//!   position only advances.
//!
//! # v0 scope (documented limits)
//!
//! - One audio track (the WAVE form's own shape); extra `data`
//!   chunks are damage-noted duplicates, first wins.
//! - No mid-file format changes: the first `fmt ` wins, duplicates
//!   are damage-noted (a second `fmt ` after `data` is the param-
//!   midchange corruption class — the container has no linkage for
//!   it; `[PROPOSAL`: engine-side `FormatChanged` is doc 03 §4.5(e)'s
//!   to own`]`).
//! - `wValidBitsPerSample < wBitsPerSample` (e.g. 20-in-24) and
//!   channel-mask→layout mapping are recorded gaps: typed refusals
//!   and pass-through facts respectively (doc 04's [VERIFY] row).

use std::collections::VecDeque;

use media_types::error::ErrorContext;
use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::header::DamageLog;
use crate::avi::riff::{FourCC, PayloadSlots, SourceReader, pad_for, u32le};

pub mod error;
pub mod format;
pub mod fuzz;
pub mod test_support;

#[cfg(test)]
mod tests;

/// Maximum packet payload trusted into memory per `next_event`
/// `[PROPOSAL: 64 KiB — 2× the input layer's read step; with the
/// largest accepted PCM frame (64 ch × 8 B = 512 B) this is still
/// ≥ 128 frames per packet, and the pool bound (2 × 64 KiB) stays
/// tiny]`.
pub const MAX_PACKET_BYTES: u64 = 64 * 1024;

/// Maximum LIST/INFO metadata trusted into memory per list
/// `[PROPOSAL: 64 KiB — INFO chunks are titles and credits; anything
/// larger is corruption and is skipped positionally]`.
pub const MAX_INFO_BYTES: u64 = 64 * 1024;

/// Maximum one INFO value may occupy
/// `[PROPOSAL: 1 KiB — a longer "title" is corruption]`.
pub const MAX_INFO_VALUE_BYTES: usize = 1024;

/// The RIFF `RIFF` magic.
const RIFF: FourCC = FourCC::tag(b"RIFF");
/// The RF64 magic (ITU-R BS.2088 / EBU Tech 3306).
const RF64: FourCC = FourCC::tag(b"RF64");
/// The BW64 magic (EBU Tech 3285 v2 — same `ds64` shape as RF64;
/// accepted identically, per the card's bw64 note).
const BW64: FourCC = FourCC::tag(b"BW64");
/// The WAVE form.
const FORM_WAVE: FourCC = FourCC::tag(b"WAVE");
/// The LIST chunk id.
const LIST: FourCC = FourCC::tag(b"LIST");
/// The INFO list form.
const FORM_INFO: FourCC = FourCC::tag(b"INFO");
/// The 32-bit size sentinel RF64 substitutes with `ds64` values (and
/// streaming writers use to mean "to the end of the stream").
const SIZE_SENTINEL: u32 = 0xFFFF_FFFF;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["wav", "wave", "rf64", "bw64"];

/// The probe: do the leading bytes look like RIFF/RF64/BW64 WAVE?
///
/// Recognizes the 12-byte head `RIFF/RF64/BW64 <size> WAVE`. Returns
/// `false` for anything shorter than the form field — a probe never
/// claims a format it cannot see.
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    if header.len() < 12 {
        return false;
    }
    let magic = FourCC::new([header[0], header[1], header[2], header[3]]);
    let form = FourCC::new([header[8], header[9], header[10], header[11]]);
    matches!(magic, RIFF | RF64 | BW64) && form == FORM_WAVE
}

/// One LIST/INFO metadata value (the demuxer-side record; the engine
/// maps ids to its metadata vocabulary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiffInfo {
    /// The INFO sub-chunk id (`INAM`, `IART`, `ISFT`, …).
    pub id: FourCC,
    /// The value as UTF-8 (lossy: metadata is advisory display text;
    /// invalid sequences render as replacement chars, never error).
    pub value: String,
}

/// The parsed `data` region, absolute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DataRegion {
    /// First payload byte (the chunk payload start).
    payload_start: u64,
    /// The **declared** payload end (chunk size, unclamped; the
    /// sentinel's "to stream end" resolves to the stream length).
    declared_end: u64,
    /// The effective end (declared ∩ RIFF extent ∩ stream).
    effective_end: u64,
}

/// The `ds64` chunk facts (RF64/BW64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Ds64 {
    riff_size: u64,
    data_size: u64,
    sample_count: u64,
}

/// A replayable error shape: stage + kind + context (the AVI
/// `StickyShape` precedent — the demuxer's own errors carry no
/// source chain and replay by shape).
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

/// The WAV demuxer.
///
/// Construct with [`WavDemuxer::new`] over a seekable source stream,
/// then drive it through the [`Demuxer`] trait: `start` parses the
/// RIFF walk (`fmt `/`fact`/`data`/`ds64`/LIST) and enumerates the
/// one audio track; `next_event` yields whole-sample-frame PCM
/// packets (plus `Recovered`/`EndOfStream`); `seek` maps time →
/// sample-frame position arithmetically (constant bit-rate — no
/// index needed).
///
/// # Example
///
/// ```
/// use demux::wav::WavDemuxer;
/// use plugin_api::{Demuxer, DemuxEvent};
///
/// # fn main() -> Result<(), media_types::EngineError> {
/// // A minimal valid PCM16 WAV (see the unit tests for the builder).
/// # let bytes = wav_demo_bytes();
/// let mut stream = demux::wav::test_support::mem_stream(bytes)?;
/// let mut wav = WavDemuxer::new(stream)?;
/// let tracks = wav.start()?;
/// assert_eq!(tracks.len(), 1);
/// assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
/// match wav.next_event()? {
///     DemuxEvent::Packet(p) => assert!(p.keyframe),
///     _ => unreachable!("a non-empty data chunk packets first"),
/// }
/// # Ok(())
/// # }
/// # fn wav_demo_bytes() -> std::sync::Arc<[u8]> {
/// #   demux::wav::test_support::minimal_pcm16_wav(&[0x34, 0x12], 44_100, 1)
/// # }
/// ```
pub struct WavDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    /// The parsed `fmt ` facts.
    fmt: Option<format::WaveFormat>,
    /// The verbatim `fmt ` payload (≤ [`format::MAX_FMT_BYTES`]) —
    /// the engine passes it as `DecoderConfig::extra_data`, where the
    /// PCM decoder re-parses it.
    fmt_bytes: Option<SharedBytes>,
    /// The `fact` chunk's `dwSampleLength` (declared frames), when
    /// present and non-zero.
    fact_frames: Option<u64>,
    /// The `ds64` facts (RF64/BW64).
    ds64: Option<Ds64>,
    /// Which container magic the file carried (observability).
    rf64_like: bool,
    data: Option<DataRegion>,
    info: Vec<RiffInfo>,
    damage: DamageLog,
    /// Next payload byte to read (absolute position).
    walk: u64,
    /// Sample frames already emitted (the PTS ladder, tick = frame;
    /// stays 0 for the frame-unknown FLAC payload).
    frames_emitted: u64,
    pending: VecDeque<DemuxEvent>,
    tracks: Option<Vec<Track>>,
    pool: PayloadSlots,
}

impl WavDemuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// No container I/O happens here (the trait's `start` does the
    /// parsing) — this only rejects sources that cannot carry a WAV:
    /// unknown length, unseekable, empty.
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for those three cases.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<WavDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(WavDemuxer {
            reader,
            started: false,
            eos: false,
            sticky: None,
            fmt: None,
            fmt_bytes: None,
            fact_frames: None,
            ds64: None,
            rf64_like: false,
            data: None,
            info: Vec::new(),
            damage: DamageLog::default(),
            walk: 0,
            frames_emitted: 0,
            pending: VecDeque::new(),
            tracks: None,
            pool: PayloadSlots::new(2, MAX_PACKET_BYTES as usize),
        })
    }

    /// The parsed `fmt ` facts — the decoder-config seam (start must
    /// have run; `None` before it or when the header parse failed).
    #[must_use]
    pub fn format(&self) -> Option<format::WaveFormat> {
        self.fmt
    }

    /// The verbatim `fmt ` chunk payload — the bytes the engine hands
    /// the decoder as `DecoderConfig::extra_data` (start must have
    /// run).
    #[must_use]
    pub fn fmt_chunk_bytes(&self) -> Option<&[u8]> {
        self.fmt_bytes.as_deref()
    }

    /// The LIST/INFO metadata collected by the header walk
    /// (`INAM`/`IART`/…, bounded per [`MAX_INFO_BYTES`]).
    #[must_use]
    pub fn info(&self) -> &[RiffInfo] {
        &self.info
    }

    /// Whether the file carried RF64/BW64 magic (observability; the
    /// `ds64` facts rode the parse either way).
    #[must_use]
    pub fn is_rf64_like(&self) -> bool {
        self.rf64_like
    }

    /// The `fact` chunk's declared sample count, when present.
    #[must_use]
    pub fn fact_sample_length(&self) -> Option<u64> {
        self.fact_frames
    }

    /// Total bytes skipped by bounded recovery so far (the
    /// `Recovered` events' sum; observable without driving the trait).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// The payload bytes resident in the packet pool (the D-1
    /// observable the memory audit consumes — bounded by
    /// `2 × MAX_PACKET_BYTES`, grown lazily).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// The effective sample-frame count of the data extent (start
    /// must have run). `None` for the frame-unknown FLAC payload —
    /// the container does not say without the frame grammar.
    #[must_use]
    pub fn frames(&self) -> Option<u64> {
        let region = self.data?;
        let fmt = self.fmt?;
        let unit = u64::from(fmt.unit_size());
        let units = region.effective_end.saturating_sub(region.payload_start) / unit;
        fmt.samples_per_unit()
            .map(|spu| units.saturating_mul(u64::from(spu)))
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

    /// Hand a walk event to the caller, keeping the
    /// damage-record-first order (the AVI `emit` contract).
    fn emit(&mut self, event: DemuxEvent) -> Option<DemuxEvent> {
        if self.pending.is_empty() {
            return Some(event);
        }
        self.pending.push_back(event);
        self.pending.pop_front()
    }

    /// The full header parse (the body of the trait's `start`).
    fn parse_container(&mut self) -> Result<Vec<Track>, EngineError> {
        // Top-of-file: `RIFF|RF64|BW64 <size> WAVE `. A file too
        // short to carry the head is classified by what it *does*
        // carry: a recognized magic prefix means the file was cut
        // inside the head (the honest flavor is truncation, not
        // "not a WAV" — D-9); anything else never promised WAV.
        let head = self.reader.read_prefix_at(12, 0)?;
        let recognized_magic = head.starts_with(RIFF.as_bytes())
            || head.starts_with(RF64.as_bytes())
            || head.starts_with(BW64.as_bytes());
        if head.len() < 12 {
            if recognized_magic {
                return Err(error::truncated(12, head.len() as u64));
            }
            return Err(error::not_wav(&head));
        }
        let magic = FourCC::new([head[0], head[1], head[2], head[3]]);
        let form = FourCC::new([head[8], head[9], head[10], head[11]]);
        if !matches!(magic, RIFF | RF64 | BW64) || form != FORM_WAVE {
            return Err(error::not_wav(&head));
        }
        self.rf64_like = matches!(magic, RF64 | BW64);

        let declared_size = u32le(&head, 4);
        // The 0xFFFFFFFF head sentinel: RF64/BW64 substitute the
        // `ds64` riffSize (mandatory); a plain-RIFF streaming writer
        // means "to the end of the stream" — undeclared, not 4 GiB.
        let sentinel = declared_size == SIZE_SENTINEL;
        if sentinel && !self.rf64_like {
            self.damage.note(0, "riff-size-sentinel");
        }
        let riff_declared_end = if sentinel {
            u64::MAX // resolved by ds64 (RF64/BW64) or the stream
        } else {
            8u64.saturating_add(u64::from(declared_size))
        };
        let mut extent_end = riff_declared_end.min(self.reader.length());
        if !sentinel && riff_declared_end > self.reader.length() {
            self.damage.note(
                riff_declared_end - self.reader.length(),
                "riff-extends-past-stream",
            );
        }

        // Top-level walk: `fmt `, `fact`, `data`, `ds64`, LIST/INFO;
        // everything else (JUNK, id3 , bext, cue ) skips positionally.
        let mut pos = 12u64;
        while pos + 8 <= extent_end {
            let Some(header_bytes) = self.reader.read_exact_or_end_at(8, pos)? else {
                return Err(error::truncated(pos + 8, self.reader.length()));
            };
            let mut raw = [0u8; 8];
            raw.copy_from_slice(&header_bytes);
            let id = FourCC::new([raw[0], raw[1], raw[2], raw[3]]);
            let payload_start = pos + 8;
            let mut size = u64::from(u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]));
            if size == u64::from(SIZE_SENTINEL) && id == FourCC::tag(b"data") {
                if let Some(ds64) = self.ds64 {
                    size = ds64.data_size;
                } else {
                    // The sentinel resolved to no ds64: in a plain
                    // RIFF this is the streaming-writer "to the end
                    // of the stream" — never a 4 GiB interpretation;
                    // in an RF64/BW64 the mandatory ds64 never
                    // arrived (the post-walk check refuses that one
                    // structurally).
                    self.damage.note(0, "data-sentinel-without-ds64");
                    size = self.reader.length().saturating_sub(payload_start);
                }
            }
            let next = payload_start
                .saturating_add(size)
                .saturating_add(pad_for(size));

            if id == FourCC::tag(b"fmt ") {
                self.parse_fmt_chunk(payload_start, size, extent_end)?;
            } else if id == FourCC::tag(b"fact") {
                if size < 4 {
                    self.damage.note(size, "fact-too-small");
                } else if let Some(bytes) = self.reader.read_exact_or_end_at(4, payload_start)? {
                    let raw = [bytes[0], bytes[1], bytes[2], bytes[3]];
                    let declared = u64::from(u32le(&raw, 0));
                    if declared > 0 {
                        self.fact_frames = Some(declared);
                    }
                }
            } else if id == FourCC::tag(b"ds64") {
                if !self.rf64_like {
                    self.damage.note(size, "ds64-outside-rf64");
                } else if let Some(ds64) = self.parse_ds64(payload_start, size, extent_end)? {
                    // The container-level size truth: recompute the
                    // walk extent from the ds64 riff size.
                    if sentinel {
                        let real_declared = 8u64.saturating_add(ds64.riff_size);
                        let real_end = real_declared.min(self.reader.length());
                        if real_declared > self.reader.length() {
                            self.damage.note(
                                real_declared - self.reader.length(),
                                "riff-extends-past-stream",
                            );
                        }
                        if real_end > extent_end {
                            extent_end = real_end;
                        }
                    }
                    self.ds64 = Some(ds64);
                }
            } else if id == FourCC::tag(b"data") {
                if self.data.is_some() {
                    self.damage.note(size, "duplicate-data");
                } else {
                    // `size` is already resolved (ds64 truth, or the
                    // plain-RIFF sentinel's to-stream-end).
                    self.data = Some(DataRegion {
                        payload_start,
                        declared_end: payload_start.saturating_add(size),
                        // effective_end recomputed after the walk (the
                        // extent may still grow via ds64).
                        effective_end: payload_start,
                    });
                }
            } else if id == LIST {
                if size < 4 {
                    // No room even for the form field — nothing to
                    // descend into.
                    self.damage.note(size, "list-too-small");
                } else {
                    let Some(form_bytes) = self.reader.read_exact_or_end_at(4, payload_start)?
                    else {
                        return Err(error::truncated(payload_start + 4, self.reader.length()));
                    };
                    if FourCC::new([form_bytes[0], form_bytes[1], form_bytes[2], form_bytes[3]])
                        == FORM_INFO
                    {
                        self.parse_info_list(
                            payload_start + 4,
                            size.saturating_sub(4),
                            extent_end,
                        )?;
                    }
                }
            }
            // JUNK / id3 / bext / cue / unknown: positional skip.

            // The live `data` chunk's over-declaration is attributed
            // once — by the extent clamp after the walk
            // (`data-extends-past-stream`) — so the same missing
            // bytes are never counted under two reasons.
            let is_live_data = id == FourCC::tag(b"data")
                && self
                    .data
                    .as_ref()
                    .is_some_and(|region| region.payload_start == payload_start);
            if next > extent_end {
                // The declared extent runs past the container: clamp,
                // note the swallowed bytes, end the walk (top-level
                // RIFF has no resync convention — everything after a
                // lying size is surrendered, counted, never silent).
                if !is_live_data {
                    self.damage
                        .note(next - extent_end, "chunk-extends-past-riff");
                }
                self.surface_damage();
                pos = extent_end;
            } else {
                pos = next;
            }
        }

        // RF64/BW64 carry their size truth in a mandatory `ds64` —
        // an RF64 head without it never completed its own
        // declaration (the structural-lie AC path).
        if self.rf64_like && self.ds64.is_none() {
            return Err(error::bad_structure(0));
        }

        // The declared container end the honest flavors key on: the
        // head size, resolved through ds64 for the RF64/BW64
        // sentinel. A stream that ends before it was *cut* — the
        // typed truncation is the honest verdict, not a missing-
        // chunk error (the AVI physically-short precedent). A plain-
        // RIFF sentinel declared nothing.
        let declared_end_final = match (sentinel, self.ds64) {
            (true, Some(ds64)) => 8u64.saturating_add(ds64.riff_size),
            (true, None) => {
                if self.rf64_like {
                    u64::MAX // unreachable: refused above
                } else {
                    self.reader.length() // the streaming-writer "to end"
                }
            }
            (false, _) => 8u64.saturating_add(u64::from(declared_size)),
        };
        let physically_short = declared_end_final > self.reader.length();
        let Some(fmt) = self.fmt else {
            if physically_short {
                return Err(error::truncated(declared_end_final, self.reader.length()));
            }
            return Err(error::no_fmt());
        };
        let Some(region) = self.data else {
            if physically_short {
                return Err(error::truncated(declared_end_final, self.reader.length()));
            }
            return Err(error::no_data());
        };

        // The data extent's effective end: declared ∩ container
        // extent ∩ stream.
        let effective_end = region
            .declared_end
            .min(extent_end)
            .min(self.reader.length());
        if region.declared_end > effective_end {
            self.damage.note(
                region.declared_end - effective_end,
                "data-extends-past-stream",
            );
        }
        let unit = u64::from(fmt.unit_size());
        let available = effective_end.saturating_sub(region.payload_start);
        let units = available.checked_div(unit).unwrap_or(0);
        if units == 0 {
            // The AC's zero-data path: a `data` chunk that is absent
            // or carries less than one whole packetization unit
            // demuxes nothing — the typed refusal, never an empty
            // stream.
            if physically_short {
                return Err(error::truncated(declared_end_final, self.reader.length()));
            }
            return Err(error::no_data());
        }

        // The fact declaration vs the bytes: bytes are authority; a
        // disagreement is counted, never guessed away — and an
        // *agreement* is not damage (0-byte notes would be noise).
        if let Some(declared) = self.fact_frames {
            let real = match fmt.samples_per_unit() {
                Some(spu) => units.saturating_mul(u64::from(spu)),
                None => 0, // FLAC: the frame grammar is not ours to run
            };
            if real > 0 && declared != real {
                let (bigger, smaller) = if declared > real {
                    (declared, real)
                } else {
                    (real, declared)
                };
                let per = u64::from(fmt.unit_size());
                self.damage.note(
                    (bigger - smaller).saturating_mul(per),
                    if declared > real {
                        "fact-frames-past-data"
                    } else {
                        "fact-frames-short-of-data"
                    },
                );
            }
        }

        self.data = Some(DataRegion {
            effective_end,
            ..region
        });

        // Track: one audio track, one tick = one sample frame (the
        // AVI PCM lane convention).
        let codec = CodecDescriptor::new(fmt.codec_descriptor())
            .map_err(|_| error::bad_format_params(0))?;
        let base =
            TimeBase::new(1, fmt.samples_per_sec).map_err(|_| error::bad_format_params(0))?;
        let span_frames = match fmt.samples_per_unit() {
            Some(spu) => units.saturating_mul(u64::from(spu)).min(i64::MAX as u64),
            // FLAC-in-WAV: the fact chunk's declared count is the
            // only container-level total (honest data when present).
            None => self.fact_frames.map_or(0, |f| f.min(i64::MAX as u64)),
        };
        let end = if fmt.samples_per_unit().is_none() && self.fact_frames.is_none() {
            Stamp::Missing
        } else {
            Stamp::Present(Timestamp::new(span_frames as i64, base))
        };
        let title = self
            .info
            .iter()
            .find(|tag| tag.id == FourCC::tag(b"INAM"))
            .map(|tag| tag.value.clone());
        let track = Track {
            id: TrackId::new(0),
            stream: StreamId::new(0),
            kind: MediaKind::Audio,
            language: None,
            codec,
            title,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Present(Timestamp::new(0, base)),
                    end,
                },
                base,
                gaps: Vec::new(),
            },
        };
        self.walk = region.payload_start;
        Ok(vec![track])
    }

    /// Parse the `fmt ` chunk payload (bounded by
    /// [`format::MAX_FMT_BYTES`]; duplicates damage-noted, first wins).
    fn parse_fmt_chunk(
        &mut self,
        payload_start: u64,
        size: u64,
        extent_end: u64,
    ) -> Result<(), EngineError> {
        if self.fmt.is_some() {
            self.damage.note(size, "duplicate-fmt");
            return Ok(());
        }
        if size < format::MIN_FMT_BYTES as u64 {
            return Err(error::bad_structure(payload_start - 8));
        }
        let want = size
            .min(format::MAX_FMT_BYTES as u64)
            .min(extent_end.saturating_sub(payload_start)) as usize;
        let bytes = self.reader.read_prefix_at(want, payload_start)?;
        if bytes.len() < format::MIN_FMT_BYTES {
            return Err(error::truncated(
                payload_start + format::MIN_FMT_BYTES as u64,
                payload_start + bytes.len() as u64,
            ));
        }
        let facts = format::parse_fmt(&bytes, payload_start - 8)?;
        self.fmt_bytes = Some(SharedBytes::from_vec(bytes));
        self.fmt = Some(facts);
        Ok(())
    }

    /// Parse the RF64/BW64 `ds64` chunk: `riffSize:u64`,
    /// `dataSize:u64`, `sampleCount:u64`, `tableLength:u32`, then
    /// `tableLength` × (id + size) — the table skips positionally
    /// (only the three sizes matter to the walk).
    fn parse_ds64(
        &mut self,
        payload_start: u64,
        size: u64,
        extent_end: u64,
    ) -> Result<Option<Ds64>, EngineError> {
        if size < 28 {
            self.damage.note(size, "ds64-too-small");
            return Ok(None);
        }
        let want = size.min(28).min(extent_end.saturating_sub(payload_start)) as usize;
        let bytes = self.reader.read_prefix_at(want, payload_start)?;
        if bytes.len() < 28 {
            return Err(error::truncated(
                payload_start + 28,
                payload_start + bytes.len() as u64,
            ));
        }
        let le_u64 = |at: usize| {
            u64::from_le_bytes([
                bytes[at],
                bytes[at + 1],
                bytes[at + 2],
                bytes[at + 3],
                bytes[at + 4],
                bytes[at + 5],
                bytes[at + 6],
                bytes[at + 7],
            ])
        };
        Ok(Some(Ds64 {
            riff_size: le_u64(0),
            data_size: le_u64(8),
            sample_count: le_u64(16),
        }))
    }

    /// Parse a `LIST INFO` payload: sub-chunks of (id, size, value),
    /// bounded by [`MAX_INFO_BYTES`] total and
    /// [`MAX_INFO_VALUE_BYTES`] per value. Word-align padding is
    /// honored; oversized values skip (damage-noted, the file still
    /// demuxes); text is lossy-decoded (advisory display data).
    fn parse_info_list(
        &mut self,
        payload_start: u64,
        size: u64,
        extent_end: u64,
    ) -> Result<(), EngineError> {
        let list_end = payload_start
            .saturating_add(size)
            .min(extent_end)
            .min(self.reader.length());
        if list_end.saturating_sub(payload_start) > MAX_INFO_BYTES {
            self.damage
                .note(list_end - payload_start, "info-list-over-ceiling");
            return Ok(());
        }
        let mut pos = payload_start;
        while pos + 8 <= list_end {
            let Some(header_bytes) = self.reader.read_exact_or_end_at(8, pos)? else {
                break; // the stream ended inside the list — the walk
                // bound was clamped already; stop reading here
            };
            let mut raw = [0u8; 8];
            raw.copy_from_slice(&header_bytes);
            let id = FourCC::new([raw[0], raw[1], raw[2], raw[3]]);
            let value_size = u64::from(u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]));
            let value_start = pos + 8;
            let next = value_start
                .saturating_add(value_size)
                .saturating_add(pad_for(value_size));

            if value_size > 0
                && value_size <= MAX_INFO_VALUE_BYTES as u64
                && value_start + value_size <= list_end
            {
                if let Some(value_bytes) = self
                    .reader
                    .read_exact_or_end_at(value_size as usize, value_start)?
                {
                    self.info.push(RiffInfo {
                        id,
                        value: String::from_utf8_lossy(&value_bytes)
                            .trim_end_matches('\0')
                            .to_string(),
                    });
                }
            } else if value_size > MAX_INFO_VALUE_BYTES as u64 {
                self.damage.note(value_size, "info-value-over-ceiling");
            }
            if next <= pos {
                break; // no progress (u64 saturation) — refuse to spin
            }
            pos = next;
        }
        Ok(())
    }

    /// The packetizer core (the `next_event` body): one packet per
    /// call, ≤ [`MAX_PACKET_BYTES`], whole packetization units only.
    fn step(&mut self) -> Result<Option<DemuxEvent>, EngineError> {
        let Some(region) = self.data else {
            return Err(self.fail(error::no_data()));
        };
        let Some(fmt) = self.fmt else {
            return Err(self.fail(error::no_fmt()));
        };
        let unit = u64::from(fmt.unit_size());
        let spu = fmt.samples_per_unit();
        // Structurally unreachable: the fmt parse enforces rate ≥ 1
        // and channels ≥ 1, so the base always constructs.
        let base = TimeBase::new(1, fmt.samples_per_sec).map_err(|_| error::bad_structure(0))?;

        if self.walk >= region.effective_end {
            if region.effective_end < region.declared_end {
                // The data was clamped (RIFF extent ∩ stream) but
                // declared more: the container is truncated, and the
                // walk says so instead of a silent empty EOF.
                return Err(self.fail(error::truncated(region.declared_end, region.effective_end)));
            }
            self.eos = true;
            return Ok(None);
        }

        let available = region.effective_end - self.walk;
        let whole_bytes = (available / unit) * unit;
        if whole_bytes == 0 {
            // A trailing sliver smaller than one packetization unit:
            // not a rounding choice — corrupt data, counted and
            // dropped (the ADPCM block / PCM sample-frame must be
            // whole for its decoder).
            self.damage.note(available, "data-not-frame-aligned");
            self.surface_damage();
            self.walk = region.effective_end;
            return Ok(None);
        }
        let want_units = (whole_bytes / unit).min(MAX_PACKET_BYTES / unit);
        let want_bytes = (want_units * unit) as usize;
        let read_from = self.walk;
        let payload = self.pool.fill(want_bytes, |out| {
            self.reader.read_prefix_into(out, read_from)
        });
        let payload = match payload {
            Ok(payload) => payload,
            Err(err) => return Err(self.fail(err)),
        };
        // Defensive containment: the region was clamped to the
        // stream, so a short read can only come from a source that
        // shrank mid-walk — a partial unit cannot be delivered (the
        // packet framing is whole units), so the loss is counted and
        // the walk ends at the clamp. The pool recycles the slot.
        let delivered = payload.len() as u64;
        if delivered < want_bytes as u64 {
            let whole = (delivered / unit) * unit;
            self.damage
                .note(want_bytes as u64 - whole, "payload-read-short");
            self.surface_damage();
            if whole > 0 {
                self.walk += whole;
                self.frames_emitted += spu.map_or(0, |s| (whole / unit) * u64::from(s));
            } else {
                self.walk = region.effective_end;
            }
            return Ok(None);
        }
        self.walk += want_bytes as u64;
        let packet_frames = spu.map_or(0, |s| want_units * u64::from(s));
        // PCM: whole packets start on the frame ladder; ADPCM blocks
        // too (units × samples-per-block). FLAC-in-WAV: no ladder —
        // the frame grammar is not ours, the stamps stay honestly
        // absent (a raw FLAC stream in a box).
        let (pts, duration) = if spu.is_some() {
            let pts_ticks = self.frames_emitted.min(i64::MAX as u64) as i64;
            let pts = Stamp::Present(Timestamp::new(pts_ticks, base));
            let duration = match Timestamp::new(packet_frames as i64, base).as_nanos() {
                Ok(Nanoseconds(nanos)) => Some(Duration::Known(Nanoseconds(nanos))),
                Err(_) => {
                    self.damage.note(0, "packet-duration-unrepresentable");
                    self.surface_damage();
                    None
                }
            };
            (pts, duration)
        } else {
            (Stamp::Missing, None)
        };
        self.frames_emitted += packet_frames;
        let packet = Packet {
            track: TrackId::new(0),
            pts,
            // Constant bit-rate payload: decode order is presentation
            // order; DTS mirrors PTS (the AVI PCM lane convention).
            dts: pts,
            duration,
            data: payload,
            // Every accepted payload is all-keyframe at the demux
            // grain (invariant D2's grammar clause: PCM frames and
            // ADPCM blocks are self-contained decode units).
            keyframe: true,
            discontinuity: false,
            // The engine's demux driver re-stamps the live generation
            // at push; the demuxer's stamp is authoritative for
            // nothing.
            generation: GenerationId::FIRST,
        };
        Ok(self.emit(DemuxEvent::Packet(packet)))
    }

    /// The shared seek computation (doc 03 §6 step 4's query half):
    /// validation + the sample-frame landing. **Never commits** —
    /// `reanchor` mutates only after this answers `Ok` (D4's
    /// compute-then-commit). Returns the landing as
    /// `(sample frame, absolute byte)`.
    fn locate(&self, target: TimeSpan) -> Result<(u64, u64), EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        if let Some(sticky) = &self.sticky {
            return Err(sticky.mint());
        }
        let Some(fmt) = self.fmt else {
            return Err(error::not_started());
        };
        let Some(region) = self.data else {
            return Err(error::seek_out_of_range());
        };
        let Some(spu) = fmt.samples_per_unit() else {
            // FLAC-in-WAV: variable-bitrate frames without a frame
            // grammar — the honest refusal (seekability() already
            // said None; this is the belt under it).
            return Err(error::seek_out_of_range());
        };
        let base =
            TimeBase::new(1, fmt.samples_per_sec).map_err(|_| error::bad_format_params(0))?;
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
        let unit = u64::from(fmt.unit_size());
        let total_units = region.effective_end.saturating_sub(region.payload_start) / unit;
        let total_frames = total_units.saturating_mul(u64::from(spu));
        // A target past the stream's end is the typed refusal (the
        // AVI seek contract — the house precedent): never a silent
        // clamp to the end. Landing exactly at the end is legal (the
        // next event is EOS).
        if total_frames > 0 && converted.ticks > total_frames as i64 {
            return Err(error::seek_out_of_range());
        }
        // The landing: PCM lands on the exact sample frame; ADPCM
        // floors to the containing block boundary (the block is the
        // self-contained decode unit — landing mid-block would hand
        // the decoder an undecodable prefix).
        let frame = (converted.ticks as u64).min(total_frames);
        let (land_frame, land_unit) = match fmt.codec {
            format::WavCodec::AdpcmImaWav { .. } | format::WavCodec::AdpcmMs { .. } => {
                let block = frame / u64::from(spu);
                (block * u64::from(spu), block)
            }
            format::WavCodec::Pcm { .. } => (frame, frame),
            // Unreachable: the `spu` binding above already refused
            // the FLAC payload; the match keeps the exhaustiveness
            // honest for future codecs.
            format::WavCodec::Flac { .. } => return Err(error::seek_out_of_range()),
        };
        Ok((land_frame, region.payload_start + land_unit * unit))
    }

    /// The reported landing span: where the stream resumes (the
    /// landed sample frame) and the declared stream end.
    fn landing_span(&self, frame: u64) -> Result<TimeSpan, EngineError> {
        let Some(fmt) = self.fmt else {
            return Err(error::not_started());
        };
        let base =
            TimeBase::new(1, fmt.samples_per_sec).map_err(|_| error::bad_format_params(0))?;
        let end = self.frames().map_or(Stamp::Missing, |frames| {
            Stamp::Present(Timestamp::new(frames.min(i64::MAX as u64) as i64, base))
        });
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(frame.min(i64::MAX as u64) as i64, base)),
            end,
        })
    }
}

impl Demuxer for WavDemuxer {
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
            // Fatal errors are sticky (the walk frozen at the damage
            // site); the first return carried the full error, replays
            // carry stage/kind/context.
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
                // The walk ended. Damage queued by the final step
                // surfaces BEFORE the terminal EndOfStream (the
                // damage-record-first contract).
                if let Some(event) = self.pending.pop_front() {
                    return Ok(event);
                }
                Ok(DemuxEvent::EndOfStream)
            }
            Err(err) => Err(err),
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Pure query: arithmetic on parsed facts, the source untouched.
        let (frame, _byte) = self.locate(target)?;
        self.landing_span(frame)
    }

    fn seekability(&self) -> SeekSupport {
        // Not started / broken / no data / frame-unknown payload: the
        // honest None.
        if !self.started || self.sticky.is_some() || self.data.is_none() || self.fmt.is_none() {
            return SeekSupport::None;
        }
        if self.fmt.is_none_or(|fmt| fmt.samples_per_unit().is_none()) {
            return SeekSupport::None;
        }
        // Constant bit-rate payload: time → byte position is
        // arithmetic on container-level truths — no index involved.
        SeekSupport::Fast { indexed: false }
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Compute (validation + the landing)…
        let (frame, byte) = self.locate(target)?;
        // …then commit. D4 holds by construction: every failure path
        // above left the demuxer untouched.
        self.walk = byte;
        self.frames_emitted = frame;
        // Pre-seek queued events (damage reports) die with the old
        // position — the demux-side stale purge.
        self.pending.clear();
        // A seek revives an exhausted stream.
        self.eos = false;
        self.landing_span(frame)
    }
}
